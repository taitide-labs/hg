pub mod handler;
pub mod query;

#[cfg(test)]
mod tests;

pub use handler::{
    bookmarks, disliked_series, is_bookmarked, is_disliked, playback_preference, search_history,
    storage_overview,
};
