use hg_contracts::viewing::SeriesBookmark;
use nsh_core::prelude::*;

use super::super::store;
use crate::gateway;

pub fn load_bookmarks() {
    store::state().update(|s| {
        s.loading_bookmarks = true;
        s.error = None;
    });

    let gw = gateway::current();
    spawn(async move {
        match gw.bookmarks().await {
            Ok(mut entries) => {
                sort_recent_first(&mut entries);
                store::state().update(|s| {
                    s.bookmarks = entries;
                    s.loading_bookmarks = false;
                    s.error = None;
                });
            }
            Err(err) => {
                store::state().update(|s| {
                    s.loading_bookmarks = false;
                    s.error = Some(err.to_string());
                });
            }
        }
    });
}

pub(super) fn sort_recent_first(entries: &mut [SeriesBookmark]) {
    entries.sort_by_key(|a| std::cmp::Reverse(a.bookmarked_at_epoch_secs));
}

#[must_use]
pub(super) fn format_score(score: f32) -> String {
    format!("{score:.1} 分")
}

#[must_use]
pub(super) fn episode_status(entry: &SeriesBookmark) -> String {
    if entry.is_finished {
        format!("全 {} 集", entry.episode_count)
    } else {
        format!("更新至 {} 集", entry.episode_count)
    }
}
