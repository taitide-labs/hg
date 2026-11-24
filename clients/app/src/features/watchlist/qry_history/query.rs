use hg_contracts::viewing::WatchHistoryEntry;
use nsh_core::prelude::*;

use super::super::store;
use crate::gateway;

pub fn load_history() {
    store::state().update(|s| {
        s.loading_history = true;
        s.error = None;
    });

    let gw = gateway::current();
    spawn(async move {
        match gw.watch_history().await {
            Ok(mut entries) => {
                sort_recent_first(&mut entries);
                store::state().update(|s| {
                    s.history = entries;
                    s.loading_history = false;
                    s.error = None;
                });
            }
            Err(err) => {
                store::state().update(|s| {
                    s.loading_history = false;
                    s.error = Some(err.to_string());
                });
            }
        }
    });
}

pub(super) fn sort_recent_first(entries: &mut [WatchHistoryEntry]) {
    entries.sort_by_key(|a| std::cmp::Reverse(a.watched_at_epoch_secs));
}

#[must_use]
pub(super) fn format_duration(seconds: u32) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{mins:02}:{secs:02}")
}

#[must_use]
pub(super) fn progress_ratio(entry: &WatchHistoryEntry) -> f32 {
    if entry.episode_completed {
        return 1.0;
    }
    if entry.duration_secs == 0 {
        return 0.0;
    }
    (entry.position_secs as f32 / entry.duration_secs as f32).clamp(0.0, 1.0)
}

#[must_use]
pub(super) fn progress_summary(entry: &WatchHistoryEntry) -> String {
    if entry.episode_completed {
        format!("已看完 · 共 {} 集", entry.total_episodes)
    } else {
        format!(
            "已看 {} / {} · 共 {} 集",
            format_duration(entry.position_secs),
            format_duration(entry.duration_secs),
            entry.total_episodes
        )
    }
}
