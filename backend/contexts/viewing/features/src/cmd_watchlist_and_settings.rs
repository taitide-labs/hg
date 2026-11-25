pub mod command;
pub mod handler;
pub mod workflow;

#[cfg(test)]
mod tests;

pub use handler::{
    bookmark_series, clear_disliked_series, clear_playback_cache, clear_search_history,
    clear_watch_history, dislike_series, record_search_history, remove_search_history,
    remove_watch_history, unbookmark_series, undislike_series, update_playback_preference,
};
