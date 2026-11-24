use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FeatureFlags {
    pub watchlist_history: bool,
    pub watchlist_remove_history: bool,
    pub watchlist_bookmarks: bool,
    pub watchlist_unbookmark: bool,
    pub offline_library: bool,
    pub offline_delete: bool,
    pub offline_queue: bool,
    pub offline_control_download: bool,
    pub settings_playback_pref: bool,
    pub settings_download_policy: bool,
    pub settings_storage: bool,
    pub settings_clear_cache: bool,
    pub player_comment_replies: bool,
    pub player_batch_download: bool,
}

static WATCHLIST_HISTORY: AtomicBool = AtomicBool::new(false);
static WATCHLIST_REMOVE_HISTORY: AtomicBool = AtomicBool::new(false);
static WATCHLIST_BOOKMARKS: AtomicBool = AtomicBool::new(false);
static WATCHLIST_UNBOOKMARK: AtomicBool = AtomicBool::new(false);
static OFFLINE_LIBRARY: AtomicBool = AtomicBool::new(false);
static OFFLINE_DELETE: AtomicBool = AtomicBool::new(false);
static OFFLINE_QUEUE: AtomicBool = AtomicBool::new(false);
static OFFLINE_CONTROL_DOWNLOAD: AtomicBool = AtomicBool::new(false);
static SETTINGS_PLAYBACK_PREF: AtomicBool = AtomicBool::new(false);
static SETTINGS_DOWNLOAD_POLICY: AtomicBool = AtomicBool::new(false);
static SETTINGS_STORAGE: AtomicBool = AtomicBool::new(false);
static SETTINGS_CLEAR_CACHE: AtomicBool = AtomicBool::new(false);
static PLAYER_COMMENT_REPLIES: AtomicBool = AtomicBool::new(false);
static PLAYER_BATCH_DOWNLOAD: AtomicBool = AtomicBool::new(false);

#[must_use]
pub fn current() -> FeatureFlags {
    FeatureFlags {
        watchlist_history: WATCHLIST_HISTORY.load(Ordering::Relaxed),
        watchlist_remove_history: WATCHLIST_REMOVE_HISTORY.load(Ordering::Relaxed),
        watchlist_bookmarks: WATCHLIST_BOOKMARKS.load(Ordering::Relaxed),
        watchlist_unbookmark: WATCHLIST_UNBOOKMARK.load(Ordering::Relaxed),
        offline_library: OFFLINE_LIBRARY.load(Ordering::Relaxed),
        offline_delete: OFFLINE_DELETE.load(Ordering::Relaxed),
        offline_queue: OFFLINE_QUEUE.load(Ordering::Relaxed),
        offline_control_download: OFFLINE_CONTROL_DOWNLOAD.load(Ordering::Relaxed),
        settings_playback_pref: SETTINGS_PLAYBACK_PREF.load(Ordering::Relaxed),
        settings_download_policy: SETTINGS_DOWNLOAD_POLICY.load(Ordering::Relaxed),
        settings_storage: SETTINGS_STORAGE.load(Ordering::Relaxed),
        settings_clear_cache: SETTINGS_CLEAR_CACHE.load(Ordering::Relaxed),
        player_comment_replies: PLAYER_COMMENT_REPLIES.load(Ordering::Relaxed),
        player_batch_download: PLAYER_BATCH_DOWNLOAD.load(Ordering::Relaxed),
    }
}

#[cfg(test)]
pub fn set_for_test(flags: FeatureFlags) {
    WATCHLIST_HISTORY.store(flags.watchlist_history, Ordering::Relaxed);
    WATCHLIST_REMOVE_HISTORY.store(flags.watchlist_remove_history, Ordering::Relaxed);
    WATCHLIST_BOOKMARKS.store(flags.watchlist_bookmarks, Ordering::Relaxed);
    WATCHLIST_UNBOOKMARK.store(flags.watchlist_unbookmark, Ordering::Relaxed);
    OFFLINE_LIBRARY.store(flags.offline_library, Ordering::Relaxed);
    OFFLINE_DELETE.store(flags.offline_delete, Ordering::Relaxed);
    OFFLINE_QUEUE.store(flags.offline_queue, Ordering::Relaxed);
    OFFLINE_CONTROL_DOWNLOAD.store(flags.offline_control_download, Ordering::Relaxed);
    SETTINGS_PLAYBACK_PREF.store(flags.settings_playback_pref, Ordering::Relaxed);
    SETTINGS_DOWNLOAD_POLICY.store(flags.settings_download_policy, Ordering::Relaxed);
    SETTINGS_STORAGE.store(flags.settings_storage, Ordering::Relaxed);
    SETTINGS_CLEAR_CACHE.store(flags.settings_clear_cache, Ordering::Relaxed);
    PLAYER_COMMENT_REPLIES.store(flags.player_comment_replies, Ordering::Relaxed);
    PLAYER_BATCH_DOWNLOAD.store(flags.player_batch_download, Ordering::Relaxed);
}
