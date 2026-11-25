use std::sync::Arc;

use hg_backend_contracts::catalog::{
    CatalogMetadataPort, CatalogPlaybackTicketPort, EpisodeSnapshot, PlaybackTicket, SeriesSnapshot,
};
use hg_backend_contracts::error::{PortError, PortFuture};
use hg_backend_contracts::offline::OfflineAssetQueryPort as _;
use hg_backend_contracts::viewing::PlaybackHaltPort;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId, StreamProfile};
use hg_infra::clock::FixedClock;
use hg_infra::db::LocalDatabase;
use hg_infra::storage::StorageLayout;
use tempfile::TempDir;

use crate::OfflineDeps;
use crate::fetcher::OfflineSegmentFetcher;

use super::handler::offline_library;
use super::port::OfflineAssetQueryAdapter;

struct NoopPorts;

impl CatalogMetadataPort for NoopPorts {
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

impl CatalogPlaybackTicketPort for NoopPorts {
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

impl PlaybackHaltPort for NoopPorts {
    fn halt_episode(&self, _key: EpisodeKey) -> PortFuture<'_, Result<(), PortError>> {
        Box::pin(async { Ok(()) })
    }

    fn halt_series(&self, _series_id: SeriesId) -> PortFuture<'_, Result<(), PortError>> {
        Box::pin(async { Ok(()) })
    }
}

impl OfflineSegmentFetcher for NoopPorts {
    fn fetch_bytes<'a>(
        &'a self,
        url: &'a str,
        _cdn: &'a hg_backend_contracts::catalog::CdnAuthHeaders,
    ) -> PortFuture<'a, Result<Vec<u8>, PortError>> {
        Box::pin(async move {
            Err(PortError::NotFound {
                resource: url.to_owned(),
            })
        })
    }
}

async fn make_deps() -> (TempDir, OfflineDeps) {
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
    let noop = Arc::new(NoopPorts);
    let deps = OfflineDeps {
        db,
        storage,
        clock: Arc::new(FixedClock::new(1_700_000_000)),
        network: Arc::new(hg_infra::network::FixedNetworkMonitor::new(
            hg_infra::network::NetworkKind::Unmetered,
        )),
        catalog_metadata: Arc::clone(&noop) as _,
        catalog_tickets: Arc::clone(&noop) as _,
        playback_halt: Arc::clone(&noop) as _,
        segment_fetcher: noop,
    };
    (temp_dir, deps)
}

#[tokio::test]
async fn offline_library_aggregates_episodes_orders_series_and_skips_empty() {
    let (_temp, deps) = make_deps().await;

    sqlx::query(
        "INSERT INTO offline_series (series_id, title, cover_local_path, total_episodes, updated_at_epoch_secs)
         VALUES
            (10, '旧剧', 's10/cover.png', 2, 100),
            (20, '新剧', 's20/cover.png', 1, 200),
            (30, '空剧', 's30/cover.png', 0, 300)",
    )
    .execute(deps.pool())
    .await
    .expect("seed series");

    sqlx::query(
        "INSERT INTO offline_episodes (series_id, ep_num, title, duration_secs, profile, playlist_rel_path, size_bytes, downloaded_at_epoch_secs)
         VALUES
            (10, 2, '第2集', 120, 'main', 's10/e2/index.m3u8', 2000, 102),
            (10, 1, '第1集', 90, 'hevc', 's10/e1/index.m3u8', 1000, 101),
            (20, 1, '首集', 60, 'hevc', 's20/e1/index.m3u8', 500, 201)",
    )
    .execute(deps.pool())
    .await
    .expect("seed episodes");

    let library = offline_library(&deps).await.expect("query library");
    assert_eq!(library.len(), 2);

    let first = library.first().expect("first series");
    assert_eq!(first.series_id.get(), 20);
    assert_eq!(first.total_size_bytes, 500);
    assert!(first.cover_local_uri.starts_with("file:///"));
    assert!(first.cover_local_uri.ends_with("/s20/cover.png"));
    assert_eq!(first.episodes.len(), 1);

    let second = library.get(1).expect("second series");
    assert_eq!(second.series_id.get(), 10);
    assert_eq!(second.total_size_bytes, 3000);
    assert_eq!(second.episodes.len(), 2);
    assert_eq!(second.episodes.first().expect("ep1").ep_num.get(), 1);
    assert_eq!(
        second.episodes.first().expect("ep1").profile,
        StreamProfile::Hevc
    );
    assert_eq!(second.episodes.get(1).expect("ep2").ep_num.get(), 2);
    assert_eq!(
        second.episodes.get(1).expect("ep2").profile,
        StreamProfile::Main
    );

    let port = OfflineAssetQueryAdapter::new(deps);
    let ep10_2 = EpisodeKey::new(
        SeriesId::try_from(10).expect("s10"),
        EpisodeNumber::try_from(2).expect("e2"),
    );
    let found = port
        .find_committed_episode(ep10_2)
        .await
        .expect("find ep")
        .expect("should exist");
    assert_eq!(found.profile, StreamProfile::Main);
    assert_eq!(found.playlist_rel_path, "s10/e2/index.m3u8");
    assert_eq!(found.duration_secs, 120);
    assert_eq!(found.size_bytes, 2000);

    let missing = EpisodeKey::new(
        SeriesId::try_from(99).expect("s99"),
        EpisodeNumber::try_from(1).expect("e1"),
    );
    assert!(
        port.find_committed_episode(missing)
            .await
            .expect("find missing")
            .is_none()
    );
    assert_eq!(
        port.total_offline_library_bytes().await.expect("bytes"),
        3500
    );
}
