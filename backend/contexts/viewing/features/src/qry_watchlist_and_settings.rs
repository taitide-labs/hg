pub mod handler;
pub mod query;

#[cfg(test)]
mod tests;

pub use handler::{
    bookmarks, is_bookmarked, playback_preference, search_history, storage_overview,
};
