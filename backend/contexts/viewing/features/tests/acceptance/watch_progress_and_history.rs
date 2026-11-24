use hg_contracts::error::ContractError;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId};
use hg_contracts::viewing::ReportWatchProgressRequest;
use hg_viewing_features::{cmd_record_watch_progress, qry_watch_history};

use super::support::{StubOfflineAssets, StubViewingCatalog, ViewingTestHarness, sample_series};

#[tokio::test]
async fn record_watch_progress_persists_deduplicates_and_orders_history() {
    let catalog = StubViewingCatalog::default().with_series([
        sample_series(101, "夜雨长灯", 20),
        sample_series(202, "长安旧梦", 10),
    ]);
    let harness = ViewingTestHarness::new(catalog, StubOfflineAssets::default()).await;

    let s101 = SeriesId::try_from(101).expect("series 101");
    let s202 = SeriesId::try_from(202).expect("series 202");
    let ep1 = EpisodeNumber::try_from(1).expect("ep 1");
    let ep2 = EpisodeNumber::try_from(2).expect("ep 2");
    let ep10 = EpisodeNumber::try_from(10).expect("ep 10");

    let mid = cmd_record_watch_progress::execute(
        &harness.deps,
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

    harness.clock.advance_secs(30);
    let tail = cmd_record_watch_progress::execute(
        &harness.deps,
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

    harness.clock.advance_secs(60);
    let finale = cmd_record_watch_progress::execute(
        &harness.deps,
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

    let history = qry_watch_history::execute(&harness.deps)
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

    harness.shutdown().await;
}

#[tokio::test]
async fn record_watch_progress_rejects_unknown_series_without_persisting_dirty_row() {
    let harness =
        ViewingTestHarness::new(StubViewingCatalog::default(), StubOfflineAssets::default()).await;

    let missing = SeriesId::try_from(999).expect("series 999");
    let ep1 = EpisodeNumber::try_from(1).expect("ep 1");

    let err = cmd_record_watch_progress::execute(
        &harness.deps,
        ReportWatchProgressRequest {
            key: EpisodeKey::new(missing, ep1),
            position_secs: 10,
            duration_secs: 90,
        },
    )
    .await
    .expect_err("unknown series should fail");

    assert!(matches!(err, ContractError::NotFound { .. }));

    let history = qry_watch_history::execute(&harness.deps)
        .await
        .expect("empty history should succeed");
    assert!(history.is_empty());

    harness.shutdown().await;
}
