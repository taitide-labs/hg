use hg_contracts::viewing::{SeriesBookmark, WatchHistoryEntry};
use nsh_core::prelude::*;

#[derive(Debug, Clone, Default)]
pub struct WatchlistState {
    pub history: Vec<WatchHistoryEntry>,
    pub bookmarks: Vec<SeriesBookmark>,
    pub loading_history: bool,
    pub loading_bookmarks: bool,
    pub error: Option<String>,
}

#[must_use]
pub fn state() -> Signal<WatchlistState> {
    global::<WatchlistState>()
}
