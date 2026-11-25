use std::collections::HashMap;
use std::sync::Arc;

use hg_backend_contracts::catalog::{
    CatalogMetadataPort, CatalogPlaybackTicketPort, CdnAuthHeaders, EpisodeSnapshot,
    PlaybackTicket, SeriesSnapshot, StreamEndpoints,
};
use hg_backend_contracts::error::{PortError, PortFuture};
use hg_backend_contracts::viewing::PlaybackHaltPort;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId, StreamProfile};
use hg_contracts::offline::{
    ControlAllDownloadsAction, ControlDownloadAction, ControlDownloadRequest,
    DownloadFailureReason, DownloadPolicy, DownloadTaskState, EnqueueEpisodeDownloadsRequest,
    PauseReason,
};
use hg_infra::clock::FixedClock;
use hg_infra::db::LocalDatabase;
use hg_infra::network::{FixedNetworkMonitor, NetworkKind};
use hg_infra::storage::StorageLayout;

use crate::OfflineDeps;
use crate::fetcher::OfflineSegmentFetcher;

use super::handler::{
    control_all_downloads, control_download, enqueue_episode_downloads, execute_next_queued_task,
    update_download_policy,
};
use super::store::load_existing_episode_keys;
use super::workflow::{
    apply_batch_control_action, apply_control_action, rewrite_media_playlist,
    select_master_playlist_variant,
};

#[derive(Default)]
struct TestCatalog {
    series: HashMap<SeriesId, SeriesSnapshot>,
    episodes: HashMap<EpisodeKey, EpisodeSnapshot>,
    covers: HashMap<String, Vec<u8>>,
    tickets: HashMap<EpisodeKey, PlaybackTicket>,
}

impl CatalogMetadataPort for TestCatalog {
    fn fetch_series_snapshot(
        &self,
        series_id: SeriesId,
    ) -> PortFuture<'_, Result<SeriesSnapshot, PortError>> {
        let found = self.series.get(&series_id).cloned();
        Box::pin(async move {
            found.ok_or_else(|| PortError::NotFound {
                resource: format!("series:{series_id}"),
            })
        })
    }

    fn fetch_episode_snapshot(
        &self,
        key: EpisodeKey,
    ) -> PortFuture<'_, Result<EpisodeSnapshot, PortError>> {
        let found = self.episodes.get(&key).cloned();
        Box::pin(async move {
            found.ok_or_else(|| PortError::NotFound {
                resource: format!("ep:{}", key.ep_num),
            })
        })
    }

    fn fetch_decrypted_cover<'a>(
        &'a self,
        cover_url: &'a str,
    ) -> PortFuture<'a, Result<Vec<u8>, PortError>> {
        let found = self.covers.get(cover_url).cloned();
        Box::pin(async move {
            found.ok_or_else(|| PortError::NotFound {
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
        let found = self.tickets.get(&key).cloned();
        Box::pin(async move {
            found.ok_or_else(|| PortError::NotFound {
                resource: format!("ticket:{}", key.ep_num),
            })
        })
    }
}

struct NoopHalt;

impl PlaybackHaltPort for NoopHalt {
    fn halt_episode(&self, _key: EpisodeKey) -> PortFuture<'_, Result<(), PortError>> {
        Box::pin(async { Ok(()) })
    }

    fn halt_series(&self, _series_id: SeriesId) -> PortFuture<'_, Result<(), PortError>> {
        Box::pin(async { Ok(()) })
    }
}

#[derive(Default)]
struct TestFetcher {
    files: HashMap<String, Vec<u8>>,
}

impl OfflineSegmentFetcher for TestFetcher {
    fn fetch_bytes<'a>(
        &'a self,
        url: &'a str,
        _cdn: &'a CdnAuthHeaders,
    ) -> PortFuture<'a, Result<Vec<u8>, PortError>> {
        let found = self.files.get(url).cloned();
        Box::pin(async move {
            found.ok_or_else(|| PortError::NotFound {
                resource: url.to_owned(),
            })
        })
    }
}

async fn build_deps(
    catalog: TestCatalog,
    fetcher: TestFetcher,
) -> (tempfile::TempDir, OfflineDeps) {
    let (temp, _net, deps) =
        build_deps_with_network(catalog, fetcher, NetworkKind::Unmetered).await;
    (temp, deps)
}

async fn build_deps_with_network(
    catalog: TestCatalog,
    fetcher: TestFetcher,
    initial_network: NetworkKind,
) -> (tempfile::TempDir, Arc<FixedNetworkMonitor>, OfflineDeps) {
    let temp = tempfile::tempdir().expect("tempdir");
    let storage = Arc::new(
        StorageLayout::new(&temp.path().join("data"), &temp.path().join("cache")).expect("storage"),
    );
    storage.ensure_dirs().await.expect("ensure dirs");
    let db = LocalDatabase::open_in_memory().await.expect("db");
    let catalog = Arc::new(catalog);
    let network = Arc::new(FixedNetworkMonitor::new(initial_network));
    let deps = OfflineDeps {
        db,
        storage,
        clock: Arc::new(FixedClock::new(1_700_000_000)),
        network: Arc::clone(&network) as _,
        catalog_metadata: Arc::clone(&catalog) as _,
        catalog_tickets: catalog,
        playback_halt: Arc::new(NoopHalt),
        segment_fetcher: Arc::new(fetcher),
    };
    (temp, network, deps)
}

#[test]
fn rewrites_media_playlist_segments_and_aux_uris() {
    let playlist = concat!(
        "#EXTM3U\n",
        "#EXT-X-VERSION:3\n",
        "#EXT-X-KEY:METHOD=AES-128,URI=\"https://cdn.example.com/keys/1.key?tok=a\"\n",
        "#EXT-X-MAP:URI=\"init.mp4?tok=b\"\n",
        "#EXTINF:5.0,\n",
        "seg0.ts?tok=c\n",
        "#EXTINF:5.0,\n",
        "/other/seg1.m4s\n",
        "#EXT-X-ENDLIST\n"
    );

    let rewritten = rewrite_media_playlist(playlist, "https://cdn.example.com/s1/e1/index.m3u8")
        .expect("rewrite");

    assert_eq!(rewritten.segment_count, 2);
    assert_eq!(rewritten.assets.len(), 4);
    assert!(rewritten.rewritten_m3u8.contains("URI=\"key_00000.key\""));
    assert!(rewritten.rewritten_m3u8.contains("URI=\"map_00001.mp4\""));
    assert!(rewritten.rewritten_m3u8.contains("\nseg_00000.ts\n"));
    assert!(rewritten.rewritten_m3u8.contains("\nseg_00001.m4s\n"));
}

#[test]
fn selects_highest_bandwidth_master_variant() {
    let master = concat!(
        "#EXTM3U\n",
        "#EXT-X-STREAM-INF:BANDWIDTH=800000,RESOLUTION=720x1280\n",
        "low/index.m3u8\n",
        "#EXT-X-STREAM-INF:BANDWIDTH=2400000,RESOLUTION=1080x1920\n",
        "high/index.m3u8\n",
    );

    let selected = select_master_playlist_variant(master, "https://cdn.example.com/v/master.m3u8")
        .expect("master")
        .expect("variant");
    assert_eq!(selected, "https://cdn.example.com/v/high/index.m3u8");
}

#[test]
fn applies_single_and_batch_control_transitions() {
    let paused = apply_control_action(
        DownloadTaskState::Downloading {
            completed_segments: 2,
            total_segments: 5,
            downloaded_bytes: 1024,
        },
        ControlDownloadAction::Pause,
    )
    .expect("pause");
    assert_eq!(
        paused,
        DownloadTaskState::Paused {
            reason: PauseReason::UserRequested,
            completed_segments: 2,
            total_segments: 5,
        }
    );

    let resumed = apply_batch_control_action(paused, ControlAllDownloadsAction::ResumeAll);
    assert_eq!(resumed, Some(DownloadTaskState::Queued));

    let retried = apply_control_action(
        DownloadTaskState::Failed {
            reason: DownloadFailureReason::NetworkError,
            completed_segments: 1,
            total_segments: 5,
        },
        ControlDownloadAction::Retry,
    )
    .expect("retry");
    assert_eq!(retried, DownloadTaskState::Queued);

    let unchanged = apply_batch_control_action(
        DownloadTaskState::Completed,
        ControlAllDownloadsAction::PauseAll,
    );
    assert_eq!(unchanged, None);
}

fn build_main_fallback_fixture(
    series_id: SeriesId,
    key1: EpisodeKey,
    key2: EpisodeKey,
) -> (TestCatalog, TestFetcher) {
    let mut catalog = TestCatalog::default();
    catalog.series.insert(
        series_id,
        SeriesSnapshot {
            series_id,
            title: "山河故人".to_owned(),
            cover_url: "/covers/202.png".to_owned(),
            total_episodes: 2,
            is_finished: true,
            score: 9.0,
        },
    );
    for (key, title, duration_secs) in [(key1, "第一集", 120), (key2, "第二集", 130)] {
        catalog.episodes.insert(
            key,
            EpisodeSnapshot {
                key,
                title: title.to_owned(),
                duration_secs,
                playable: true,
            },
        );
    }
    catalog
        .covers
        .insert("/covers/202.png".to_owned(), b"\x89PNGdata".to_vec());
    catalog.tickets.insert(
        key1,
        PlaybackTicket {
            key: key1,
            streams: StreamEndpoints {
                hevc: None,
                main: Some("https://cdn.example.com/202/1/main.m3u8".to_owned()),
                preview: None,
            },
            cdn: CdnAuthHeaders {
                referer: "https://example.com".to_owned(),
                user_agent: "test".to_owned(),
                auth_header_name: None,
                auth_header_value: None,
                cdn_app: None,
            },
            issued_at_epoch_secs: 1_700_000_000,
            expires_at_epoch_secs: 1_700_001_000,
        },
    );

    let mut fetcher = TestFetcher::default();
    fetcher.files.insert(
        "https://cdn.example.com/202/1/main.m3u8".to_owned(),
        b"#EXTM3U\n#EXTINF:5.0,\nseg0.ts\n#EXT-X-ENDLIST\n".to_vec(),
    );
    fetcher.files.insert(
        "https://cdn.example.com/202/1/seg0.ts".to_owned(),
        b"video-bytes".to_vec(),
    );
    (catalog, fetcher)
}

#[tokio::test]
async fn falls_back_hevc_to_main_persists_library_and_skips_completed_on_reenqueue() {
    let series_id = SeriesId::try_from(202).expect("series");
    let ep1 = EpisodeNumber::try_from(1).expect("ep1");
    let ep2 = EpisodeNumber::try_from(2).expect("ep2");
    let key1 = EpisodeKey::new(series_id, ep1);
    let key2 = EpisodeKey::new(series_id, ep2);

    let (catalog, fetcher) = build_main_fallback_fixture(series_id, key1, key2);
    let (_temp, deps) = build_deps(catalog, fetcher).await;
    update_download_policy(
        &deps,
        DownloadPolicy {
            preferred_profile: StreamProfile::Hevc,
            wifi_only: false,
            max_concurrent_tasks: 2,
            storage_directory: None,
        },
    )
    .await
    .expect("policy");

    let enqueued = enqueue_episode_downloads(
        &deps,
        EnqueueEpisodeDownloadsRequest {
            series_id,
            episodes: vec![ep1, ep2],
        },
    )
    .await
    .expect("enqueue");
    assert_eq!(enqueued.queued, vec![key1, key2]);

    control_all_downloads(&deps, ControlAllDownloadsAction::PauseAll)
        .await
        .expect("pause all");
    assert_eq!(
        execute_next_queued_task(&deps).await.expect("empty queue"),
        None
    );

    control_download(
        &deps,
        ControlDownloadRequest {
            key: key2,
            action: ControlDownloadAction::Cancel,
        },
    )
    .await
    .expect("cancel ep2");
    control_all_downloads(&deps, ControlAllDownloadsAction::ResumeAll)
        .await
        .expect("resume all");

    assert_eq!(
        execute_next_queued_task(&deps).await.expect("run ep1"),
        Some(key1)
    );
    assert!(!deps.storage.staging_episode_dir(key1).exists());

    let ep_dir = deps.storage.library_episode_dir(key1);
    let m3u8 = tokio::fs::read_to_string(ep_dir.join("index.m3u8"))
        .await
        .expect("read rewritten playlist");
    assert!(m3u8.contains("seg_00000.ts"));
    assert!(ep_dir.join("seg_00000.ts").exists());
    assert!(
        deps.storage
            .library_series_dir(series_id)
            .join("cover.png")
            .exists()
    );

    let saved_profile: (String,) =
        sqlx::query_as("SELECT profile FROM offline_episodes WHERE series_id = ?1 AND ep_num = ?2")
            .bind(202_i64)
            .bind(1_i64)
            .fetch_one(deps.pool())
            .await
            .expect("offline_episodes row");
    assert_eq!(saved_profile.0, "main");

    let existing = load_existing_episode_keys(deps.pool(), series_id)
        .await
        .expect("existing");
    assert!(existing.contains(&key1));
    assert!(!existing.contains(&key2));

    let re_enqueued = enqueue_episode_downloads(
        &deps,
        EnqueueEpisodeDownloadsRequest {
            series_id,
            episodes: vec![ep1, ep2],
        },
    )
    .await
    .expect("re-enqueue");
    assert_eq!(re_enqueued.queued, vec![key2]);
    assert_eq!(re_enqueued.skipped_already_present, vec![key1]);
}

#[tokio::test]
async fn wifi_only_policy_pauses_tasks_when_network_is_metered_and_resumes_on_wifi() {
    let series_id = SeriesId::try_from(202).expect("series");
    let ep1 = EpisodeNumber::try_from(1).expect("ep1");
    let ep2 = EpisodeNumber::try_from(2).expect("ep2");
    let key1 = EpisodeKey::new(series_id, ep1);
    let key2 = EpisodeKey::new(series_id, ep2);
    let (catalog, fetcher) = build_main_fallback_fixture(series_id, key1, key2);
    let (_temp, network, deps) =
        build_deps_with_network(catalog, fetcher, NetworkKind::Metered).await;
    update_download_policy(
        &deps,
        DownloadPolicy {
            preferred_profile: StreamProfile::Hevc,
            wifi_only: true,
            max_concurrent_tasks: 2,
            storage_directory: None,
        },
    )
    .await
    .expect("enable wifi_only");
    enqueue_episode_downloads(
        &deps,
        EnqueueEpisodeDownloadsRequest {
            series_id,
            episodes: vec![ep1],
        },
    )
    .await
    .expect("enqueue");

    let ran = execute_next_queued_task(&deps).await.expect("execute");
    assert_eq!(ran, None);
    let state = super::store::read_task_state(deps.pool(), key1)
        .await
        .expect("read state");
    assert_eq!(
        state,
        Some(DownloadTaskState::Paused {
            reason: PauseReason::WifiOnlyBlocked,
            completed_segments: 0,
            total_segments: 0,
        })
    );

    network.set(NetworkKind::Unmetered);
    let resumed = execute_next_queued_task(&deps)
        .await
        .expect("resume on unmetered wifi");
    assert_eq!(resumed, Some(key1));
}
