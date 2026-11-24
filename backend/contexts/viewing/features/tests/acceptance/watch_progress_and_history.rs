use std::collections::HashMap;
use std::sync::Arc;

use hg_backend_contracts::catalog::{CatalogMetadataPort, EpisodeSnapshot, SeriesSnapshot};
use hg_backend_contracts::error::{PortError, PortFuture};
use hg_contracts::error::ContractError;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId};
use hg_contracts::viewing::ReportWatchProgressRequest;
use hg_infra::clock::FixedClock;
use hg_infra::db::LocalDatabase;
use hg_viewing_features::{ViewingDeps, cmd_record_watch_progress, qry_watch_history};

struct StubCatalogMetadata {
    series: HashMap<SeriesId, SeriesSnapshot>,
}

impl StubCatalogMetadata {
    fn with_series(items: impl IntoIterator<Item = SeriesSnapshot>) -> Self {
        let series = items
            .into_iter()
            .map(|item| (item.series_id, item))
            .collect();
        Self { series }
    }
}

impl CatalogMetadataPort for StubCatalogMetadata {
    fn fetch_series_snapshot(
        &self,
        series_id: SeriesId,
    ) -> PortFuture<'_, Result<SeriesSnapshot, PortError>> {
        let result = self
            .series
            .get(&series_id)
            .cloned()
            .ok_or_else(|| PortError::NotFound {
                resource: format!("series:{series_id}"),
            });
        Box::pin(async move { result })
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

fn sample_series(id: u64, title: &str, total_episodes: u32) -> SeriesSnapshot {
    SeriesSnapshot {
        series_id: SeriesId::try_from(id).expect("valid series id"),
        title: title.to_owned(),
        cover_url: format!("https://cdn.example.com/covers/{id}.jpg"),
        total_episodes,
        is_finished: true,
        score: 9.2,
    }
}

#[tokio::test]
async fn record_watch_progress_persists_deduplicates_and_orders_history() {
    let db = LocalDatabase::open_in_memory()
        .await
        .expect("in-memory sqlite");
    let clock = Arc::new(FixedClock::new(1_700_000_000));
    let catalog = Arc::new(StubCatalogMetadata::with_series([
        sample_series(101, "夜雨长灯", 20),
        sample_series(202, "长安旧梦", 10),
    ]));
    let deps = ViewingDeps {
        db: db.clone(),
        clock: Arc::clone(&clock) as _,
        catalog_metadata: catalog,
    };

    let s101 = SeriesId::try_from(101).expect("series 101");
    let s202 = SeriesId::try_from(202).expect("series 202");
    let ep1 = EpisodeNumber::try_from(1).expect("ep 1");
    let ep2 = EpisodeNumber::try_from(2).expect("ep 2");
    let ep10 = EpisodeNumber::try_from(10).expect("ep 10");

    let mid = cmd_record_watch_progress::execute(
        &deps,
        ReportWatchProgressRequest {
            key: EpisodeKey::new(s101, ep1),
            position_secs: 45,
            duration_secs: 120,
        },
    )
    .await
    .expect("mid-episode report should succeed");
    assert!(!mid.progress.completed);
    assert_eq!(mid.next_episode, None);

    clock.advance_secs(30);
    let tail = cmd_record_watch_progress::execute(
        &deps,
        ReportWatchProgressRequest {
            key: EpisodeKey::new(s101, ep1),
            position_secs: 118,
            duration_secs: 120,
        },
    )
    .await
    .expect("tail-episode report should succeed");
    assert!(tail.progress.completed);
    assert_eq!(tail.next_episode, Some(ep2));

    clock.advance_secs(60);
    let finale = cmd_record_watch_progress::execute(
        &deps,
        ReportWatchProgressRequest {
            key: EpisodeKey::new(s202, ep10),
            position_secs: 130,
            duration_secs: 120,
        },
    )
    .await
    .expect("finale report should clamp and complete");
    assert_eq!(finale.progress.position_secs, 120);
    assert!(finale.progress.completed);
    assert_eq!(finale.next_episode, None);

    let history = qry_watch_history::execute(&deps)
        .await
        .expect("watch history query should succeed");
    assert_eq!(history.len(), 2);

    let first = history.first().expect("most recent history entry");
    assert_eq!(first.series_id, s202);
    assert_eq!(first.title, "长安旧梦");
    assert_eq!(first.last_episode, ep10);
    assert_eq!(first.position_secs, 120);
    assert!(first.episode_completed);
    assert_eq!(first.watched_at_epoch_secs, 1_700_000_090);

    let second = history.get(1).expect("older history entry");
    assert_eq!(second.series_id, s101);
    assert_eq!(second.title, "夜雨长灯");
    assert_eq!(second.last_episode, ep1);
    assert_eq!(second.position_secs, 118);
    assert!(second.episode_completed);
    assert_eq!(second.watched_at_epoch_secs, 1_700_000_030);

    db.close().await;
}

#[tokio::test]
async fn record_watch_progress_rejects_unknown_series_without_persisting_dirty_row() {
    let db = LocalDatabase::open_in_memory()
        .await
        .expect("in-memory sqlite");
    let clock = Arc::new(FixedClock::new(1_700_000_500));
    let catalog = Arc::new(StubCatalogMetadata::with_series([]));
    let deps = ViewingDeps {
        db: db.clone(),
        clock,
        catalog_metadata: catalog,
    };

    let missing = SeriesId::try_from(999).expect("series 999");
    let ep1 = EpisodeNumber::try_from(1).expect("ep 1");

    let err = cmd_record_watch_progress::execute(
        &deps,
        ReportWatchProgressRequest {
            key: EpisodeKey::new(missing, ep1),
            position_secs: 10,
            duration_secs: 90,
        },
    )
    .await
    .expect_err("unknown series should fail");

    assert!(matches!(err, ContractError::NotFound { .. }));

    let history = qry_watch_history::execute(&deps)
        .await
        .expect("empty history should succeed");
    assert!(history.is_empty());

    db.close().await;
}
