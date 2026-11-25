use std::future::Future;
use std::num::{NonZeroU16, NonZeroU32, NonZeroU64};
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use hg_contracts::catalog::{ListSeriesRequest, SeriesFilter, SeriesSort};
use hg_contracts::error::ContractError;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, PageRequest, SeriesId};
use hg_contracts::offline::{DownloadTaskState, EnqueueEpisodeDownloadsRequest};
use hg_contracts::viewing::{PlaybackOrigin, ReportWatchProgressRequest};

use crate::contract::AppGateway;
use crate::mock::MockGateway;

fn block_ready<F: Future>(future: F) -> F::Output {
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    match pin!(future).poll(&mut cx) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("mock gateway future should resolve immediately"),
    }
}

fn sid(raw: u64) -> SeriesId {
    SeriesId::new(NonZeroU64::new(raw).unwrap())
}

fn ep(raw: u32) -> EpisodeNumber {
    EpisodeNumber::new(NonZeroU32::new(raw).unwrap())
}

#[test]
fn playback_prefers_offline_episode_over_online_stream() {
    let gw = MockGateway::new();
    let offline_src =
        block_ready(gw.prepare_episode_playback(EpisodeKey::new(sid(1001), ep(1)))).unwrap();
    assert!(matches!(
        offline_src.origin,
        PlaybackOrigin::OfflineLocal { .. }
    ));

    let online_src =
        block_ready(gw.prepare_episode_playback(EpisodeKey::new(sid(1001), ep(5)))).unwrap();
    assert!(matches!(
        online_src.origin,
        PlaybackOrigin::OnlineStream { .. }
    ));
}

#[test]
fn clearing_caches_never_touches_offline_library() {
    let gw = MockGateway::new();
    let before = block_ready(gw.storage_overview()).unwrap();
    assert!(before.offline_library_bytes > 0);
    assert!(before.playback_cache_bytes > 0);
    assert!(before.catalog_cache_bytes > 0);

    block_ready(gw.clear_playback_cache()).unwrap();
    block_ready(gw.clear_catalog_cache()).unwrap();

    let after = block_ready(gw.storage_overview()).unwrap();
    assert_eq!(after.offline_library_bytes, before.offline_library_bytes);
    assert_eq!(after.playback_cache_bytes, 0);
    assert_eq!(after.catalog_cache_bytes, 0);
}

#[test]
fn watch_progress_marks_episode_completed_near_end_and_suggests_next() {
    let gw = MockGateway::new();
    let res = block_ready(gw.report_watch_progress(ReportWatchProgressRequest {
        key: EpisodeKey::new(sid(1001), ep(4)),
        position_secs: 138,
        duration_secs: 140,
    }))
    .unwrap();
    assert!(res.progress.completed);
    assert_eq!(res.next_episode, Some(ep(5)));

    let history = block_ready(gw.watch_history()).unwrap();
    assert_eq!(history.first().map(|h| h.series_id), Some(sid(1001)));
    assert_eq!(history.first().map(|h| h.last_episode), Some(ep(4)));
}

#[test]
fn enqueue_downloads_deduplicates_offline_and_queued_episodes() {
    let gw = MockGateway::new();
    let res = block_ready(
        gw.enqueue_episode_downloads(EnqueueEpisodeDownloadsRequest {
            series_id: sid(1001),
            episodes: vec![ep(1), ep(4), ep(5), ep(6)],
        }),
    )
    .unwrap();
    assert_eq!(res.skipped_already_present.len(), 2);
    assert_eq!(res.queued.len(), 2);
}

#[test]
fn mock_gateway_supports_method_error_paged_catalog_failed_task_and_gates() {
    let gw = MockGateway::new()
        .with_paged_catalog(30)
        .with_failed_download_task();
    assert!(
        gw.snapshot_download_tasks()
            .iter()
            .any(|t| matches!(t.state, DownloadTaskState::Failed { .. }))
    );

    let page1 = block_ready(gw.list_series(ListSeriesRequest {
        filter: SeriesFilter::All { category_id: None },
        sort: SeriesSort::Hot,
        page: PageRequest::new(NonZeroU32::MIN, NonZeroU16::new(24).unwrap()),
    }))
    .unwrap();
    assert!(page1.pagination.pages >= 2);
    assert_eq!(page1.pagination.total, 36);

    gw.inject_method_error(
        "watch_history",
        Some(ContractError::NetworkUnavailable {
            detail: "targeted fault".to_owned(),
        }),
    );
    assert!(block_ready(gw.watch_history()).is_err());
    assert!(block_ready(gw.bookmarks()).is_ok());

    let gate = gw.queue_method_gate("bookmarks");
    let mut fut = gw.bookmarks();
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    assert!(matches!(fut.as_mut().poll(&mut cx), Poll::Pending));
    gate.release();
    assert!(matches!(fut.as_mut().poll(&mut cx), Poll::Ready(Ok(_))));
}
