use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use hg_backend_contracts::catalog::{
    CatalogMetadataPort, CatalogPlaybackTicketPort, CdnAuthHeaders, EpisodeSnapshot,
    PlaybackTicket, SeriesSnapshot,
};
use hg_backend_contracts::error::{PortError, PortFuture};
use hg_backend_contracts::offline::OfflineDomainEvent;
use hg_backend_contracts::viewing::PlaybackHaltPort;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId};
use hg_infra::clock::FixedClock;
use hg_infra::db::LocalDatabase;
use hg_infra::storage::StorageLayout;
use parking_lot::Mutex;
use tempfile::TempDir;

use crate::OfflineDeps;
use crate::fetcher::OfflineSegmentFetcher;

use super::handler::{delete_offline_episode, delete_offline_series};
use super::outbox::{decode_outbox_event, dispatch_pending_outbox};
use super::workflow::{build_episode_removed_event, build_series_removed_event, outbox_event_type};

struct NoopCatalogAndFetcher;

impl CatalogMetadataPort for NoopCatalogAndFetcher {
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

impl CatalogPlaybackTicketPort for NoopCatalogAndFetcher {
    fn fetch_playback_ticket(
        &self,
        key: EpisodeKey,
    ) -> PortFuture<'_, Result<PlaybackTicket, PortError>> {
        Box::pin(async move {
            Err(PortError::NotFound {
                resource: format!("ticket:{}:{}", key.series_id, key.ep_num),
            })
        })
    }
}

impl OfflineSegmentFetcher for NoopCatalogAndFetcher {
    fn fetch_bytes<'a>(
        &'a self,
        url: &'a str,
        _cdn: &'a CdnAuthHeaders,
    ) -> PortFuture<'a, Result<Vec<u8>, PortError>> {
        Box::pin(async move {
            Err(PortError::NotFound {
                resource: url.to_owned(),
            })
        })
    }
}

struct RecordingHaltPort {
    storage: Arc<StorageLayout>,
    should_fail: AtomicBool,
    events: Mutex<Vec<(String, bool)>>,
}

impl RecordingHaltPort {
    fn new(storage: Arc<StorageLayout>) -> Self {
        Self {
            storage,
            should_fail: AtomicBool::new(false),
            events: Mutex::new(Vec::new()),
        }
    }

    fn set_fail(&self, fail: bool) {
        self.should_fail.store(fail, Ordering::SeqCst);
    }

    fn take_events(&self) -> Vec<(String, bool)> {
        std::mem::take(&mut *self.events.lock())
    }
}

impl PlaybackHaltPort for RecordingHaltPort {
    fn halt_episode(&self, key: EpisodeKey) -> PortFuture<'_, Result<(), PortError>> {
        let dir_existed = self.storage.library_episode_dir(key).exists();
        self.events
            .lock()
            .push((format!("ep:{}:{}", key.series_id, key.ep_num), dir_existed));
        let fail = self.should_fail.load(Ordering::SeqCst);
        Box::pin(async move {
            if fail {
                Err(PortError::InvalidState {
                    detail: "模拟停播失败".to_owned(),
                })
            } else {
                Ok(())
            }
        })
    }

    fn halt_series(&self, series_id: SeriesId) -> PortFuture<'_, Result<(), PortError>> {
        let dir_existed = self.storage.library_series_dir(series_id).exists();
        self.events
            .lock()
            .push((format!("series:{series_id}"), dir_existed));
        let fail = self.should_fail.load(Ordering::SeqCst);
        Box::pin(async move {
            if fail {
                Err(PortError::InvalidState {
                    detail: "模拟整剧停播失败".to_owned(),
                })
            } else {
                Ok(())
            }
        })
    }
}

async fn make_deps() -> (TempDir, Arc<RecordingHaltPort>, OfflineDeps) {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let storage = Arc::new(
        StorageLayout::new(
            &temp_dir.path().join("data"),
            &temp_dir.path().join("cache"),
        )
        .expect("storage"),
    );
    storage.ensure_dirs().await.expect("ensure dirs");
    let db = LocalDatabase::open_in_memory().await.expect("sqlite");
    let noop = Arc::new(NoopCatalogAndFetcher);
    let halt = Arc::new(RecordingHaltPort::new(Arc::clone(&storage)));
    let deps = OfflineDeps {
        db,
        storage,
        clock: Arc::new(FixedClock::new(1_700_000_000)),
        network: Arc::new(hg_infra::network::FixedNetworkMonitor::new(
            hg_infra::network::NetworkKind::Unmetered,
        )),
        catalog_metadata: Arc::clone(&noop) as _,
        catalog_tickets: Arc::clone(&noop) as _,
        playback_halt: Arc::clone(&halt) as _,
        segment_fetcher: noop,
    };
    (temp_dir, halt, deps)
}

async fn seed_series_with_two_episodes(
    deps: &OfflineDeps,
    series_id: SeriesId,
) -> (EpisodeKey, EpisodeKey) {
    let ep1 = EpisodeKey::new(series_id, EpisodeNumber::try_from(1).expect("e1"));
    let ep2 = EpisodeKey::new(series_id, EpisodeNumber::try_from(2).expect("e2"));

    for key in [ep1, ep2] {
        let dir = deps.storage.library_episode_dir(key);
        tokio::fs::create_dir_all(&dir).await.expect("mkdir");
        tokio::fs::write(dir.join("index.m3u8"), b"#EXTM3U\n")
            .await
            .expect("write playlist");
    }

    sqlx::query(
        "INSERT INTO offline_series (series_id, title, cover_local_path, total_episodes, updated_at_epoch_secs)
         VALUES (?1, '剧目', 's55/cover.png', 2, 1700000000)",
    )
    .bind(i64::try_from(series_id.get()).expect("i64"))
    .execute(deps.pool())
    .await
    .expect("seed series");

    for ep in [1_i64, 2_i64] {
        sqlx::query(
            "INSERT INTO offline_episodes (series_id, ep_num, title, duration_secs, profile, playlist_rel_path, size_bytes, downloaded_at_epoch_secs)
             VALUES (?1, ?2, '分集', 90, 'hevc', ?3, 512, 1700000000)",
        )
        .bind(i64::try_from(series_id.get()).expect("i64"))
        .bind(ep)
        .bind(format!("s{}/e{ep}/index.m3u8", series_id.get()))
        .execute(deps.pool())
        .await
        .expect("seed ep");
    }

    (ep1, ep2)
}

#[test]
fn workflow_builds_valid_outbox_events_and_labels() {
    let series_id = SeriesId::try_from(42).expect("s42");
    let key = EpisodeKey::new(series_id, EpisodeNumber::try_from(3).expect("e3"));

    let ep_event = build_episode_removed_event(key);
    assert_eq!(outbox_event_type(&ep_event), "offline_episode_removed");
    let ep_json = serde_json::to_string(&ep_event).expect("serialize ep");
    let decoded_ep = decode_outbox_event(&ep_json).expect("decode ep");
    assert_eq!(
        decoded_ep,
        OfflineDomainEvent::OfflineEpisodeRemoved {
            key,
            asset_rel_dir: "s42/e3".to_owned(),
        }
    );

    let series_event = build_series_removed_event(series_id);
    assert_eq!(outbox_event_type(&series_event), "offline_series_removed");
    let series_json = serde_json::to_string(&series_event).expect("serialize series");
    let decoded_series = decode_outbox_event(&series_json).expect("decode series");
    assert_eq!(
        decoded_series,
        OfflineDomainEvent::OfflineSeriesRemoved {
            series_id,
            asset_rel_dir: "s42".to_owned(),
        }
    );
}

#[tokio::test]
async fn dispatch_halts_playback_before_removing_directory_and_cleans_empty_series() {
    let (_temp, halt, deps) = make_deps().await;
    let series_id = SeriesId::try_from(55).expect("s55");
    let (ep1, ep2) = seed_series_with_two_episodes(&deps, series_id).await;

    delete_offline_episode(&deps, ep1)
        .await
        .expect("delete ep1");
    assert_eq!(halt.take_events(), vec![("ep:55:1".to_owned(), true)]);
    assert!(!deps.storage.library_episode_dir(ep1).exists());
    assert!(deps.storage.library_episode_dir(ep2).exists());

    delete_offline_episode(&deps, ep2)
        .await
        .expect("delete ep2");
    assert_eq!(halt.take_events(), vec![("ep:55:2".to_owned(), true)]);
    assert!(!deps.storage.library_episode_dir(ep2).exists());

    let series_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM offline_series")
        .fetch_one(deps.pool())
        .await
        .expect("count series");
    assert_eq!(series_count, 0);
}

#[tokio::test]
async fn delete_missing_assets_is_idempotent_without_spurious_halting() {
    let (_temp, halt, deps) = make_deps().await;
    let series_id = SeriesId::try_from(77).expect("s77");
    let key = EpisodeKey::new(series_id, EpisodeNumber::try_from(1).expect("e1"));

    delete_offline_episode(&deps, key)
        .await
        .expect("idempotent ep delete");
    delete_offline_series(&deps, series_id)
        .await
        .expect("idempotent series delete");

    assert!(halt.take_events().is_empty());
    let outbox_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM offline_outbox_events")
        .fetch_one(deps.pool())
        .await
        .expect("count outbox");
    assert_eq!(outbox_count, 0);
}

#[tokio::test]
async fn dispatch_keeps_event_pending_when_halt_fails_and_retries_cleanly() {
    let (_temp, halt, deps) = make_deps().await;
    let series_id = SeriesId::try_from(88).expect("s88");
    let (ep1, _ep2) = seed_series_with_two_episodes(&deps, series_id).await;

    halt.set_fail(true);
    let err = delete_offline_episode(&deps, ep1)
        .await
        .expect_err("halt failure should propagate");
    assert!(err.to_string().contains("模拟停播失败"));
    assert!(deps.storage.library_episode_dir(ep1).exists());

    let pending: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM offline_outbox_events WHERE dispatched_at_epoch_secs IS NULL",
    )
    .fetch_one(deps.pool())
    .await
    .expect("count pending");
    assert_eq!(pending, 1);

    halt.set_fail(false);
    let dispatched = dispatch_pending_outbox(&deps)
        .await
        .expect("retry dispatch");
    assert_eq!(dispatched, 1);
    assert!(!deps.storage.library_episode_dir(ep1).exists());

    let pending_after: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM offline_outbox_events WHERE dispatched_at_epoch_secs IS NULL",
    )
    .fetch_one(deps.pool())
    .await
    .expect("count pending after");
    assert_eq!(pending_after, 0);
}
