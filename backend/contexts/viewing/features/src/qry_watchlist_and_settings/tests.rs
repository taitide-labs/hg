use std::sync::Arc;

use hg_backend_contracts::catalog::{
    CatalogMetadataPort, CatalogPlaybackTicketPort, EpisodeSnapshot, PlaybackTicket, SeriesSnapshot,
};
use hg_backend_contracts::error::{PortError, PortFuture};
use hg_backend_contracts::offline::{CommittedOfflineAsset, OfflineAssetQueryPort};
use hg_contracts::ids::{EpisodeKey, SeriesId};
use hg_contracts::viewing::PlaybackPreference;
use hg_infra::clock::FixedClock;
use hg_infra::db::LocalDatabase;
use hg_infra::loopback_proxy::LoopbackProxyHandle;
use hg_infra::storage::StorageLayout;
use tempfile::TempDir;

use crate::ViewingDeps;

use super::handler::{bookmarks, is_bookmarked, playback_preference, storage_overview};

struct NoopCatalog;

impl CatalogMetadataPort for NoopCatalog {
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

impl CatalogPlaybackTicketPort for NoopCatalog {
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

struct StubOfflineBytes(u64);

impl OfflineAssetQueryPort for StubOfflineBytes {
    fn find_committed_episode(
        &self,
        _key: EpisodeKey,
    ) -> PortFuture<'_, Result<Option<CommittedOfflineAsset>, PortError>> {
        Box::pin(async move { Ok(None) })
    }

    fn total_offline_library_bytes(&self) -> PortFuture<'_, Result<u64, PortError>> {
        let bytes = self.0;
        Box::pin(async move { Ok(bytes) })
    }
}

async fn make_deps(offline_bytes: u64) -> (TempDir, ViewingDeps) {
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
    let clock = Arc::new(FixedClock::new(1_700_000_000));
    let loopback = LoopbackProxyHandle::start(Arc::clone(&storage))
        .await
        .expect("loopback");
    let catalog = Arc::new(NoopCatalog);
    let deps = ViewingDeps {
        db,
        storage,
        clock,
        catalog_metadata: Arc::clone(&catalog) as _,
        catalog_tickets: catalog,
        offline_assets: Arc::new(StubOfflineBytes(offline_bytes)),
        loopback,
    };
    (temp_dir, deps)
}

#[tokio::test]
async fn bookmarks_orders_recent_first_and_maps_fields() {
    let (_temp, deps) = make_deps(0).await;
    let s10 = SeriesId::try_from(10).expect("series 10");
    let s20 = SeriesId::try_from(20).expect("series 20");
    let s30 = SeriesId::try_from(30).expect("series 30");

    sqlx::query(
        "INSERT INTO viewing_bookmarks (
            series_id, title, cover_url, episode_count, is_finished, score, bookmarked_at_epoch_secs
        ) VALUES
            (10, '旧剧', 'https://cdn.example.com/10.jpg', 12, 0, 8.5, 1700000010),
            (20, '新剧', 'https://cdn.example.com/20.jpg', 24, 1, 9.5, 1700000090)",
    )
    .execute(deps.pool())
    .await
    .expect("seed bookmarks");

    let list = bookmarks(&deps).await.expect("query bookmarks");
    assert_eq!(list.len(), 2);
    let first = list.first().expect("first");
    assert_eq!(first.series_id, s20);
    assert_eq!(first.title, "新剧");
    assert_eq!(first.episode_count, 24);
    assert!(first.is_finished);
    assert!((first.score - 9.5).abs() < 0.01);
    assert_eq!(first.bookmarked_at_epoch_secs, 1_700_000_090);

    let second = list.get(1).expect("second");
    assert_eq!(second.series_id, s10);
    assert!(!second.is_finished);

    assert!(is_bookmarked(&deps, s10).await.expect("s10 bookmarked"));
    assert!(!is_bookmarked(&deps, s30).await.expect("s30 not bookmarked"));

    deps.loopback.shutdown();
    deps.db.close().await;
}

#[tokio::test]
async fn playback_preference_reads_seed_default_and_handles_missing_row() {
    let (_temp, deps) = make_deps(0).await;
    let expected_default = PlaybackPreference::default();

    assert_eq!(
        playback_preference(&deps)
            .await
            .expect("read seeded preference"),
        expected_default
    );

    sqlx::query("DELETE FROM viewing_playback_preference WHERE id = 1")
        .execute(deps.pool())
        .await
        .expect("delete seed row");
    assert_eq!(
        playback_preference(&deps)
            .await
            .expect("fallback default preference"),
        expected_default
    );

    deps.loopback.shutdown();
    deps.db.close().await;
}

#[tokio::test]
async fn storage_overview_aggregates_offline_playback_and_catalog_bytes() {
    let (_temp, deps) = make_deps(8192).await;

    tokio::fs::write(
        deps.storage.playback_cache_dir().join("seg.ts"),
        b"1234567890",
    )
    .await
    .expect("write playback cache");
    tokio::fs::write(
        deps.storage.catalog_cache_dir().join("cover.bin"),
        b"abcdef",
    )
    .await
    .expect("write catalog cache");

    let overview = storage_overview(&deps)
        .await
        .expect("query storage overview");
    assert_eq!(overview.offline_library_bytes, 8192);
    assert_eq!(overview.playback_cache_bytes, 10);
    assert_eq!(overview.catalog_cache_bytes, 6);

    deps.loopback.shutdown();
    deps.db.close().await;
}
