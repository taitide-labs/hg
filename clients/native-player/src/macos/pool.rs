use std::cell::RefCell;

use nsh_platform_macos::objc2::msg_send;
use nsh_platform_macos::objc2::rc::Retained;
use nsh_platform_macos::objc2::runtime::{AnyObject, Bool};

use super::avfoundation::class;

pub(super) const DEFAULT_POOL_CAPACITY: usize = 3;

pub(super) struct PlayerPool<T> {
    capacity: usize,
    idle: Vec<T>,
}

impl<T> PlayerPool<T> {
    pub(super) const fn new(capacity: usize) -> Self {
        Self {
            capacity,
            idle: Vec::new(),
        }
    }

    pub(super) const fn with_default_capacity() -> Self {
        Self::new(DEFAULT_POOL_CAPACITY)
    }

    pub(super) fn acquire(&mut self) -> Option<T> {
        self.idle.pop()
    }

    pub(super) fn release(&mut self, item: T, reset: impl FnOnce(&T)) -> bool {
        reset(&item);
        if self.idle.len() < self.capacity {
            self.idle.push(item);
            true
        } else {
            drop(item);
            false
        }
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.idle.len()
    }

    #[cfg(test)]
    pub(super) const fn capacity(&self) -> usize {
        self.capacity
    }
}

thread_local! {
    static AV_PLAYER_POOL: RefCell<PlayerPool<Retained<AnyObject>>> =
        const { RefCell::new(PlayerPool::with_default_capacity()) };
}

pub(super) fn acquire_av_player() -> Option<Retained<AnyObject>> {
    if let Ok(Some(player)) = AV_PLAYER_POOL.try_with(|pool| pool.borrow_mut().acquire()) {
        return Some(player);
    }
    let cls = class(c"AVPlayer")?;
    #[expect(unsafe_code, reason = "通过 Objective-C 运行时创建新的 AVPlayer 实例")]
    let player: Option<Retained<AnyObject>> = unsafe { msg_send![cls, new] };
    player
}

pub(super) fn release_av_player(player: Retained<AnyObject>) {
    reset_av_player(&player);
    if let Ok(kept) = AV_PLAYER_POOL.try_with(|pool| pool.borrow_mut().release(player, |_| {})) {
        let _ = kept;
    }
}

fn reset_av_player(player: &AnyObject) {
    #[expect(
        unsafe_code,
        reason = "归还或销毁前调用 AVPlayer 官方 API 暂停并清空当前媒体项"
    )]
    unsafe {
        let _: () = msg_send![player, pause];
        let _: () = msg_send![
            player,
            replaceCurrentItemWithPlayerItem: std::ptr::null::<AnyObject>()
        ];
        let _: () = msg_send![player, setMuted: Bool::NO];
        let _: () = msg_send![player, setVolume: 0.8_f32];
    }
}
