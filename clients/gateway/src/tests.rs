use std::future::Future;
use std::num::{NonZeroU32, NonZeroU64};
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId};
use hg_contracts::offline::EnqueueEpisodeDownloadsRequest;
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
