pub mod handler;
pub mod query;

#[cfg(test)]
mod tests;

pub use handler::{bookmarks, is_bookmarked, playback_preference, storage_overview};
