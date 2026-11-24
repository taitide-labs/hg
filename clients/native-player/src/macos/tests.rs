use std::cell::Cell;
use std::rc::Rc;

use super::avfoundation::{
    CMTime, MediaSourceKind, classify_source, cmtime_to_secs_u32, video_gravity_for_aspect,
};
use super::pool::{DEFAULT_POOL_CAPACITY, PlayerPool};
use crate::protocol::VideoAspect;

#[derive(Debug)]
struct StubPlayer {
    id: u32,
    reset_count: Rc<Cell<u32>>,
    dropped_count: Rc<Cell<u32>>,
}

impl Drop for StubPlayer {
    fn drop(&mut self) {
        self.dropped_count.set(self.dropped_count.get() + 1);
    }
}

#[test]
fn pool_borrows_returns_reuses_lifo_and_evicts_over_capacity() {
    let mut pool = PlayerPool::with_default_capacity();
    assert_eq!(pool.capacity(), DEFAULT_POOL_CAPACITY);
    assert!(pool.acquire().is_none());

    let reset_count = Rc::new(Cell::new(0));
    let dropped_count = Rc::new(Cell::new(0));

    let make = |id: u32| StubPlayer {
        id,
        reset_count: Rc::clone(&reset_count),
        dropped_count: Rc::clone(&dropped_count),
    };
    let reset = |item: &StubPlayer| {
        item.reset_count.set(item.reset_count.get() + 1);
    };

    assert!(pool.release(make(1), reset));
    assert!(pool.release(make(2), reset));
    assert!(pool.release(make(3), reset));
    assert_eq!(pool.len(), 3);
    assert_eq!(reset_count.get(), 3);
    assert_eq!(dropped_count.get(), 0);

    assert!(!pool.release(make(4), reset));
    assert_eq!(pool.len(), 3);
    assert_eq!(reset_count.get(), 4);
    assert_eq!(dropped_count.get(), 1);

    let reused_third = pool.acquire();
    assert_eq!(reused_third.as_ref().map(|p| p.id), Some(3));
    let reused_second = pool.acquire();
    assert_eq!(reused_second.as_ref().map(|p| p.id), Some(2));
    assert_eq!(pool.len(), 1);

    drop(reused_third);
    drop(reused_second);
    drop(pool);
    assert_eq!(dropped_count.get(), 4);
}

#[test]
fn pool_with_zero_capacity_drops_every_returned_player() {
    let mut pool = PlayerPool::new(0);
    let dropped_count = Rc::new(Cell::new(0));
    let reset_count = Rc::new(Cell::new(0));
    let stub = StubPlayer {
        id: 99,
        reset_count: Rc::clone(&reset_count),
        dropped_count: Rc::clone(&dropped_count),
    };

    assert!(!pool.release(stub, |p| p.reset_count.set(p.reset_count.get() + 1)));
    assert_eq!(pool.len(), 0);
    assert_eq!(reset_count.get(), 1);
    assert_eq!(dropped_count.get(), 1);
}

#[test]
fn maps_video_aspect_to_av_layer_video_gravity() {
    assert_eq!(
        video_gravity_for_aspect(VideoAspect::Original),
        "AVLayerVideoGravityResizeAspect"
    );
    assert_eq!(
        video_gravity_for_aspect(VideoAspect::Cinema16x9),
        "AVLayerVideoGravityResizeAspect"
    );
    assert_eq!(
        video_gravity_for_aspect(VideoAspect::Portrait9x16),
        "AVLayerVideoGravityResizeAspect"
    );
    assert_eq!(
        video_gravity_for_aspect(VideoAspect::Fill),
        "AVLayerVideoGravityResizeAspectFill"
    );
}

#[test]
fn converts_cmtime_precisely_and_rejects_invalid_or_indefinite_values() {
    assert_eq!(CMTime::zero().seconds(), Some(0.0));
    let half = CMTime::from_seconds(0.5);
    assert_eq!(half.value, 300);
    assert_eq!(half.timescale, 600);
    assert_eq!(cmtime_to_secs_u32(CMTime::from_seconds(42.8)), 42);

    let invalid = CMTime {
        value: 600,
        timescale: 600,
        flags: 0,
        epoch: 0,
    };
    assert_eq!(invalid.seconds(), None);
    assert_eq!(cmtime_to_secs_u32(invalid), 0);

    let indefinite = CMTime {
        value: 0,
        timescale: 600,
        flags: 0x11,
        epoch: 0,
    };
    assert_eq!(indefinite.seconds(), None);
    assert_eq!(cmtime_to_secs_u32(indefinite), 0);
}

#[test]
fn classifies_loopback_hls_urls_and_local_paths() {
    assert_eq!(
        classify_source("http://127.0.0.1:49200/online/1/index.m3u8"),
        Some(MediaSourceKind::Url)
    );
    assert_eq!(
        classify_source("file:///tmp/ep01/index.m3u8"),
        Some(MediaSourceKind::Url)
    );
    assert_eq!(
        classify_source("/tmp/ep01/index.m3u8"),
        Some(MediaSourceKind::FilePath)
    );
    assert_eq!(classify_source("   "), None);
}
