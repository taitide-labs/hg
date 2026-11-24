use std::collections::HashMap;
use std::sync::Arc;

use hg_backend_contracts::catalog::{
    CatalogMetadataPort, CatalogPlaybackTicketPort, EpisodeSnapshot, PlaybackTicket, SeriesSnapshot,
};
use hg_backend_contracts::error::{PortError, PortFuture};
use hg_backend_contracts::offline::{CommittedOfflineAsset, OfflineAssetQueryPort};
use hg_contracts::error::ContractError;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId, StreamProfile};
use hg_contracts::viewing::PlaybackPreference;
use hg_infra::clock::FixedClock;
use hg_infra::db::LocalDatabase;
use hg_infra::loopback_proxy::LoopbackProxyHandle;
use hg_infra::storage::StorageLayout;
use tempfile::TempDir;

use crate::ViewingDeps;

use super::handler::{
    bookmark_series, clear_playback_cache, clear_watch_history, remove_watch_history,
    update_playback_preference,
};
use super::workflow::{check_playback_preference, plan_bookmark_series, profile_as_str};

#[derive(Default)]
struct StubCatalog {
    series: HashMap<SeriesId, SeriesSnapshot>,
}

impl StubCatalog {
    fn with_series(mut self, series: SeriesSnapshot) -> Self {
        self.series.insert(series.series_id, series);
        self
    }
}

impl CatalogMetadataPort for StubCatalog {
    fn fetch_series_snapshot(
        &self,
        series_id: SeriesId,
    ) -> PortFuture<'_, Result<SeriesSnapshot, PortError>> {
        let found = self
            .series
            .get(&series_id)
            .cloned()
            .ok_or_else(|| PortError::NotFound {
                resource: format!("series:{series_id}"),
            });
        Box::pin(async move { found })
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

impl CatalogPlaybackTicketPort for StubCatalog {
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

#[derive(Default)]
struct StubOffline;

impl OfflineAssetQueryPort for StubOffline {
    fn find_committed_episode(
        &self,
        _key: EpisodeKey,
    ) -> PortFuture<'_, Result<Option<CommittedOfflineAsset>, PortError>> {
        Box::pin(async move { Ok(None) })
    }

    fn total_offline_library_bytes(&self) -> PortFuture<'_, Result<u64, PortError>> {
        Box::pin(async move { Ok(0) })
    }
}

async fn make_deps(catalog: StubCatalog) -> (TempDir, ViewingDeps) {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let storage = Arc::new(
        StorageLayout::new(
            &temp_dir.path().join("data"),
            &temp_dir.path().join("cache"),
        )
        .expect("storage layout"),
    );
    storage.ensure_dirs().await.expect("ensure dirs");
    let db = LocalDatabase::open_in_memory().await.expect("sqlite");
    let clock = Arc::new(FixedClock::new(1_700_000_123));
    let loopback = LoopbackProxyHandle::start(Arc::clone(&storage))
        .await
        .expect("loopback");
    let deps = ViewingDeps {
        db,
        storage,
        clock,
        catalog_metadata: Arc::new(catalog),
        catalog_tickets: Arc::new(StubCatalog::default()),
        offline_assets: Arc::new(StubOffline),
        loopback,
    };
    (temp_dir, deps)
}

#[test]
fn workflow_plans_bookmark_and_maps_profile() {
    let series_id = SeriesId::try_from(77).expect("series 77");
    let snapshot = SeriesSnapshot {
        series_id,
        title: "月下逢".to_owned(),
        cover_url: "https://cdn.example.com/77.jpg".to_owned(),
        total_episodes: 36,
        is_finished: false,
        score: 9.1,
    };

    let planned = plan_bookmark_series(&snapshot, 1_700_000_999);
    assert_eq!(planned.series_id, series_id);
    assert_eq!(planned.title, "月下逢");
    assert_eq!(planned.episode_count, 36);
    assert!(!planned.is_finished);
    assert_eq!(planned.bookmarked_at_epoch_secs, 1_700_000_999);

    assert_eq!(profile_as_str(StreamProfile::Hevc), "hevc");
    assert_eq!(profile_as_str(StreamProfile::Main), "main");

    let invalid = PlaybackPreference {
        preferred_profile: StreamProfile::Main,
        playback_cache_limit_mb: 16,
        default_rate_hundredths: 100,
    };
    assert!(matches!(
        check_playback_preference(&invalid),
        Err(ContractError::InvalidInput { .. })
    ));
}

#[tokio::test]
async fn bookmark_unknown_series_returns_not_found_without_persisting_row() {
    let (_temp, deps) = make_deps(StubCatalog::default()).await;
    let missing = SeriesId::try_from(404).expect("series 404");

    let err = bookmark_series(&deps, missing)
        .await
        .expect_err("missing series should fail");
    assert!(matches!(err, ContractError::NotFound { .. }));

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM viewing_bookmarks")
        .fetch_one(deps.pool())
        .await
        .expect("count bookmarks");
    assert_eq!(count.0, 0);

    deps.loopback.shutdown();
    deps.db.close().await;
}

#[tokio::test]
async fn bookmark_upserts_and_clear_watch_history_deletes_all_rows() {
    let s101 = SeriesId::try_from(101).expect("series 101");
    let catalog = StubCatalog::default().with_series(SeriesSnapshot {
        series_id: s101,
        title: "春风渡".to_owned(),
        cover_url: "https://cdn.example.com/101.jpg".to_owned(),
        total_episodes: 24,
        is_finished: true,
        score: 9.4,
    });
    let (_temp, deps) = make_deps(catalog).await;

    bookmark_series(&deps, s101).await.expect("first bookmark");
    bookmark_series(&deps, s101)
        .await
        .expect("idempotent bookmark upsert");
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM viewing_bookmarks")
        .fetch_one(deps.pool())
        .await
        .expect("count bookmarks");
    assert_eq!(count.0, 1);

    for sid in [101_i64, 202_i64] {
        sqlx::query(
            "INSERT INTO viewing_watch_history (
                series_id, title, cover_url, last_episode, position_secs,
                duration_secs, episode_completed, total_episodes, watched_at_epoch_secs
            ) VALUES (?1, 't', 'c', 1, 10, 90, 0, 10, 1700000000)",
        )
        .bind(sid)
        .execute(deps.pool())
        .await
        .expect("seed history");
    }

    remove_watch_history(&deps, s101)
        .await
        .expect("remove single history");
    let after_single_remove: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM viewing_watch_history")
        .fetch_one(deps.pool())
        .await
        .expect("count history after single remove");
    assert_eq!(after_single_remove.0, 1);

    clear_watch_history(&deps)
        .await
        .expect("clear watch history");
    let remaining: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM viewing_watch_history")
        .fetch_one(deps.pool())
        .await
        .expect("count history");
    assert_eq!(remaining.0, 0);

    deps.loopback.shutdown();
    deps.db.close().await;
}

#[tokio::test]
async fn update_playback_preference_rejects_invalid_and_persists_valid() {
    let (_temp, deps) = make_deps(StubCatalog::default()).await;

    let bad_rate = PlaybackPreference {
        preferred_profile: StreamProfile::Main,
        playback_cache_limit_mb: 256,
        default_rate_hundredths: 400,
    };
    let err = update_playback_preference(&deps, bad_rate)
        .await
        .expect_err("invalid rate should fail");
    assert!(matches!(err, ContractError::InvalidInput { .. }));

    let valid = PlaybackPreference {
        preferred_profile: StreamProfile::Main,
        playback_cache_limit_mb: 1024,
        default_rate_hundredths: 200,
    };
    update_playback_preference(&deps, valid)
        .await
        .expect("valid preference should persist");

    let row: (String, i64, i64) = sqlx::query_as(
        "SELECT preferred_profile, playback_cache_limit_mb, default_rate_hundredths
         FROM viewing_playback_preference WHERE id = 1",
    )
    .fetch_one(deps.pool())
    .await
    .expect("read saved preference");
    assert_eq!(row, ("main".to_owned(), 1024, 200));

    deps.loopback.shutdown();
    deps.db.close().await;
}

#[tokio::test]
async fn clear_playback_cache_wipes_db_segments_and_files_preserving_offline() {
    let (_temp, deps) = make_deps(StubCatalog::default()).await;

    sqlx::query(
        "INSERT INTO viewing_playback_cache_segments (
            segment_key, local_rel_path, size_bytes, accessed_at_epoch_secs
        ) VALUES ('s1_e1_0', 's1_e1_0.ts', 512, 1700000000)",
    )
    .execute(deps.pool())
    .await
    .expect("seed cache segment");

    let cache_file = deps.storage.playback_cache_dir().join("s1_e1_0.ts");
    tokio::fs::write(&cache_file, b"segment-bytes")
        .await
        .expect("write cache segment");

    let ep = EpisodeKey::new(
        SeriesId::try_from(1).expect("series 1"),
        EpisodeNumber::try_from(1).expect("ep 1"),
    );
    let offline_dir = deps.storage.library_episode_dir(ep);
    tokio::fs::create_dir_all(&offline_dir)
        .await
        .expect("create offline dir");
    let offline_file = offline_dir.join("index.m3u8");
    tokio::fs::write(&offline_file, b"#EXTM3U\n")
        .await
        .expect("write offline file");

    clear_playback_cache(&deps)
        .await
        .expect("clear playback cache");

    let remaining_segments: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM viewing_playback_cache_segments")
            .fetch_one(deps.pool())
            .await
            .expect("count segments");
    assert_eq!(remaining_segments.0, 0);
    assert!(!cache_file.exists());
    assert!(deps.storage.playback_cache_dir().exists());
    assert!(offline_file.exists());

    deps.loopback.shutdown();
    deps.db.close().await;
}
