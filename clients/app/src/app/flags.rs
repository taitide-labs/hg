use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FeatureFlags {
    pub watchlist_history: bool,
    pub watchlist_bookmarks: bool,
    pub offline_library: bool,
    pub offline_queue: bool,
    pub settings_preferences: bool,
    pub settings_storage: bool,
    pub player_comment_replies: bool,
    pub player_batch_download: bool,
}

static WATCHLIST_HISTORY: AtomicBool = AtomicBool::new(false);
static WATCHLIST_BOOKMARKS: AtomicBool = AtomicBool::new(false);
static OFFLINE_LIBRARY: AtomicBool = AtomicBool::new(false);
static OFFLINE_QUEUE: AtomicBool = AtomicBool::new(false);
static SETTINGS_PREFERENCES: AtomicBool = AtomicBool::new(false);
static SETTINGS_STORAGE: AtomicBool = AtomicBool::new(false);
static PLAYER_COMMENT_REPLIES: AtomicBool = AtomicBool::new(false);
static PLAYER_BATCH_DOWNLOAD: AtomicBool = AtomicBool::new(false);

#[must_use]
pub fn current() -> FeatureFlags {
    FeatureFlags {
        watchlist_history: WATCHLIST_HISTORY.load(Ordering::Relaxed),
        watchlist_bookmarks: WATCHLIST_BOOKMARKS.load(Ordering::Relaxed),
        offline_library: OFFLINE_LIBRARY.load(Ordering::Relaxed),
        offline_queue: OFFLINE_QUEUE.load(Ordering::Relaxed),
        settings_preferences: SETTINGS_PREFERENCES.load(Ordering::Relaxed),
        settings_storage: SETTINGS_STORAGE.load(Ordering::Relaxed),
        player_comment_replies: PLAYER_COMMENT_REPLIES.load(Ordering::Relaxed),
        player_batch_download: PLAYER_BATCH_DOWNLOAD.load(Ordering::Relaxed),
    }
}

#[cfg(test)]
pub fn set_for_test(flags: FeatureFlags) {
    WATCHLIST_HISTORY.store(flags.watchlist_history, Ordering::Relaxed);
    WATCHLIST_BOOKMARKS.store(flags.watchlist_bookmarks, Ordering::Relaxed);
    OFFLINE_LIBRARY.store(flags.offline_library, Ordering::Relaxed);
    OFFLINE_QUEUE.store(flags.offline_queue, Ordering::Relaxed);
    SETTINGS_PREFERENCES.store(flags.settings_preferences, Ordering::Relaxed);
    SETTINGS_STORAGE.store(flags.settings_storage, Ordering::Relaxed);
    PLAYER_COMMENT_REPLIES.store(flags.player_comment_replies, Ordering::Relaxed);
    PLAYER_BATCH_DOWNLOAD.store(flags.player_batch_download, Ordering::Relaxed);
}
