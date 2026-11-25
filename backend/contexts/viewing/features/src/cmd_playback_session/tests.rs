use std::collections::HashMap;
use std::sync::Arc;

use hg_backend_contracts::catalog::{
    CatalogMetadataPort, CatalogPlaybackTicketPort, CdnAuthHeaders, EpisodeSnapshot,
    PlaybackTicket, SeriesSnapshot, StreamEndpoints,
};
use hg_backend_contracts::error::{PortError, PortFuture};
use hg_backend_contracts::offline::{CommittedOfflineAsset, OfflineAssetQueryPort};
use hg_contracts::error::ContractError;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId, StreamProfile};
use hg_contracts::viewing::{OnlineStreamKind, PlaybackOrigin};
use hg_infra::clock::FixedClock;
use hg_infra::db::LocalDatabase;
use hg_infra::loopback_proxy::LoopbackProxyHandle;
use hg_infra::storage::StorageLayout;
use parking_lot::Mutex;

use crate::ViewingDeps;

use super::handler::{
    prepare_episode_playback, refresh_online_playback, stop_deleted_offline_episode,
    stop_deleted_offline_series,
};
use super::workflow::{parse_preferred_profile, plan_offline_registration, plan_online_source};

#[derive(Default)]
struct TestCatalog {
    tickets: Mutex<Vec<PlaybackTicket>>,
    calls: Mutex<Vec<EpisodeKey>>,
}

impl TestCatalog {
    fn with_tickets(tickets: Vec<PlaybackTicket>) -> Self {
        Self {
            tickets: Mutex::new(tickets),
            calls: Mutex::new(Vec::new()),
        }
    }

    fn calls(&self) -> Vec<EpisodeKey> {
        self.calls.lock().clone()
    }
}

impl CatalogMetadataPort for TestCatalog {
    fn fetch_series_snapshot(
        &self,
        series_id: SeriesId,
    ) -> PortFuture<'_, Result<SeriesSnapshot, PortError>> {
        Box::pin(async move {
            Err(PortError::NotFound {
                resource: format!("series:{series_id}"),
            })
        })
    }

    fn fetch_episode_snapshot(
        &self,
        key: EpisodeKey,
    ) -> PortFuture<'_, Result<EpisodeSnapshot, PortError>> {
        Box::pin(async move {
            Err(PortError::NotFound {
                resource: format!("episode:{}:{}", key.series_id, key.ep_num),
            })
        })
    }

    fn fetch_decrypted_cover<'a>(
        &'a self,
        cover_url: &'a str,
    ) -> PortFuture<'a, Result<Vec<u8>, PortError>> {
        Box::pin(async move {
            Err(PortError::NotFound {
                resource: cover_url.to_owned(),
            })
        })
    }
}

impl CatalogPlaybackTicketPort for TestCatalog {
    fn fetch_playback_ticket(
        &self,
        key: EpisodeKey,
    ) -> PortFuture<'_, Result<PlaybackTicket, PortError>> {
        self.calls.lock().push(key);
        let mut queue = self.tickets.lock();
        let next = queue
            .iter()
            .position(|ticket| ticket.key == key)
            .map(|idx| queue.remove(idx));
        Box::pin(async move {
            next.ok_or_else(|| PortError::NotFound {
                resource: format!("ticket:{}:{}", key.series_id, key.ep_num),
            })
        })
    }
}

#[derive(Default)]
struct TestOfflineAssets {
    assets: HashMap<EpisodeKey, CommittedOfflineAsset>,
}

impl OfflineAssetQueryPort for TestOfflineAssets {
    fn find_committed_episode(
        &self,
        key: EpisodeKey,
    ) -> PortFuture<'_, Result<Option<CommittedOfflineAsset>, PortError>> {
        let found = self.assets.get(&key).cloned();
        Box::pin(async move { Ok(found) })
    }

    fn total_offline_library_bytes(&self) -> PortFuture<'_, Result<u64, PortError>> {
        Box::pin(async { Ok(0) })
    }
}

fn sample_key(series: u64, ep: u32) -> EpisodeKey {
    EpisodeKey::new(
        SeriesId::try_from(series).expect("valid series"),
        EpisodeNumber::try_from(ep).expect("valid episode"),
    )
}

fn sample_ticket(key: EpisodeKey, suffix: &str) -> PlaybackTicket {
    PlaybackTicket {
        key,
        streams: StreamEndpoints {
            hevc: Some(format!("https://cdn.example.com/{suffix}/hevc.m3u8")),
            main: Some(format!("https://cdn.example.com/{suffix}/main.m3u8")),
            preview: Some(format!("https://cdn.example.com/{suffix}/preview.m3u8")),
        },
        cdn: CdnAuthHeaders {
            referer: "https://example.com/".to_owned(),
            user_agent: "hg-test/1.0".to_owned(),
            auth_header_name: None,
            auth_header_value: None,
            cdn_app: None,
        },
        issued_at_epoch_secs: 1_700_000_000,
        expires_at_epoch_secs: 1_700_001_800,
    }
}

#[test]
fn parses_preferred_profile_and_rejects_unknown_value() {
    assert_eq!(
        parse_preferred_profile("hevc").expect("hevc"),
        StreamProfile::Hevc
    );
    assert_eq!(
        parse_preferred_profile("main").expect("main"),
        StreamProfile::Main
    );
    assert!(matches!(
        parse_preferred_profile("4k"),
        Err(ContractError::Internal { .. })
    ));
}

#[test]
fn workflow_plans_offline_registration_and_online_preference_fallback() {
    let key = sample_key(12, 1);
    let asset = CommittedOfflineAsset {
        key,
        profile: StreamProfile::Main,
        playlist_rel_path: "s12/e1/index.m3u8".to_owned(),
        duration_secs: 105,
        size_bytes: 1024,
    };
    let offline = plan_offline_registration(&asset).expect("offline plan");
    assert_eq!(offline.playlist_rel_path, "s12/e1/index.m3u8");
    assert_eq!(
        offline.origin,
        PlaybackOrigin::OfflineLocal {
            profile: StreamProfile::Main
        }
    );
    assert_eq!(offline.duration_secs, 105);

    let ticket = sample_ticket(key, "v1");
    let main_source = plan_online_source(key, &ticket, StreamProfile::Main).expect("main stream");
    assert_eq!(
        main_source.origin,
        PlaybackOrigin::OnlineStream {
            kind: OnlineStreamKind::Main
        }
    );
    assert_eq!(
        main_source.playable_uri,
        "https://cdn.example.com/v1/main.m3u8"
    );

    let empty_ticket = PlaybackTicket {
        streams: StreamEndpoints {
            hevc: None,
            main: None,
            preview: None,
        },
        ..ticket
    };
    assert!(matches!(
        plan_online_source(key, &empty_ticket, StreamProfile::Hevc),
        Err(ContractError::NotFound { .. })
    ));
}

#[tokio::test]
async fn refreshes_online_playback_using_stored_preference_and_halts_series_sessions() {
    let temp = tempfile::tempdir().expect("tempdir");
    let storage = Arc::new(
        StorageLayout::new(&temp.path().join("data"), &temp.path().join("cache")).expect("storage"),
    );
    storage.ensure_dirs().await.expect("ensure dirs");
    let db = LocalDatabase::open_in_memory().await.expect("sqlite");
    let loopback = LoopbackProxyHandle::start(Arc::clone(&storage))
        .await
        .expect("loopback");

    let s10 = SeriesId::try_from(10).expect("series 10");
    let ep1 = EpisodeKey::new(s10, EpisodeNumber::try_from(1).expect("ep 1"));
    let ep2 = EpisodeKey::new(s10, EpisodeNumber::try_from(2).expect("ep 2"));
    let ep3 = EpisodeKey::new(s10, EpisodeNumber::try_from(3).expect("ep 3"));

    let ep_dir = storage.library_episode_dir(ep1);
    tokio::fs::create_dir_all(&ep_dir).await.expect("ep dir");
    tokio::fs::write(ep_dir.join("index.m3u8"), b"#EXTM3U\n")
        .await
        .expect("write m3u8");

    let mut offline_map = HashMap::new();
    offline_map.insert(
        ep1,
        CommittedOfflineAsset {
            key: ep1,
            profile: StreamProfile::Hevc,
            playlist_rel_path: "s10/e1/index.m3u8".to_owned(),
            duration_secs: 60,
            size_bytes: 512,
        },
    );
    let catalog = Arc::new(TestCatalog::with_tickets(vec![
        sample_ticket(ep2, "first"),
        sample_ticket(ep2, "refreshed"),
        sample_ticket(ep3, "third"),
    ]));

    sqlx::query("UPDATE viewing_playback_preference SET preferred_profile = 'main' WHERE id = 1")
        .execute(db.pool())
        .await
        .expect("update preference");

    let deps = ViewingDeps {
        db: db.clone(),
        storage,
        clock: Arc::new(FixedClock::new(1_700_000_000)),
        catalog_metadata: Arc::clone(&catalog) as _,
        catalog_tickets: Arc::clone(&catalog) as _,
        offline_assets: Arc::new(TestOfflineAssets {
            assets: offline_map,
        }),
        loopback: loopback.clone(),
    };

    let offline_src = prepare_episode_playback(&deps, ep1)
        .await
        .expect("offline prepare");
    assert_eq!(loopback.active_session_count(), 1);
    assert!(catalog.calls().is_empty());

    let initial_online = prepare_episode_playback(&deps, ep2)
        .await
        .expect("online prepare");
    assert_eq!(
        initial_online.origin,
        PlaybackOrigin::OnlineStream {
            kind: OnlineStreamKind::Main
        }
    );
    assert!(
        initial_online
            .playable_uri
            .starts_with(&loopback.base_url())
    );
    assert!(initial_online.playable_uri.contains("/online/"));

    let refreshed_online = refresh_online_playback(&deps, ep2)
        .await
        .expect("online refresh");
    assert!(
        refreshed_online
            .playable_uri
            .starts_with(&loopback.base_url())
    );
    assert_ne!(refreshed_online.playable_uri, initial_online.playable_uri);
    assert_eq!(catalog.calls(), vec![ep2, ep2]);

    stop_deleted_offline_episode(&deps, ep3)
        .await
        .expect("halt unrelated episode");
    assert_eq!(loopback.active_session_count(), 3);

    stop_deleted_offline_series(&deps, s10)
        .await
        .expect("halt series");
    assert_eq!(loopback.active_session_count(), 0);

    let client = reqwest::Client::new();
    let resp = client
        .get(&offline_src.playable_uri)
        .send()
        .await
        .expect("request halted session");
    assert_eq!(resp.status(), reqwest::StatusCode::GONE);

    loopback.shutdown();
    db.close().await;
}

#[tokio::test]
#[ignore = "BUG: 在线回环代理直通流式转发，尚未写入 viewing_playback_cache_segments 或按 PlaybackPreference.playback_cache_limit_mb 自动淘汰"]
async fn online_loopback_playback_caches_segments_within_configured_limit() {
    let temp = tempfile::tempdir().expect("tempdir");
    let storage = Arc::new(
        StorageLayout::new(&temp.path().join("data"), &temp.path().join("cache")).expect("storage"),
    );
    storage.ensure_dirs().await.expect("ensure dirs");
    let db = LocalDatabase::open_in_memory().await.expect("sqlite");
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM viewing_playback_cache_segments")
        .fetch_one(db.pool())
        .await
        .expect("count playback cache segments");
    assert!(
        count.0 > 0,
        "online playback segments must populate viewing_playback_cache_segments and honor cache limit"
    );
    db.close().await;
}
