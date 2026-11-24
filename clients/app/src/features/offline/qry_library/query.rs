use hg_contracts::ids::EpisodeNumber;
use hg_contracts::offline::{OfflineEpisodeView, OfflineSeriesView};
use nsh_core::prelude::*;

use super::super::store;
use crate::gateway;

pub fn load_library() {
    store::state().update(|s| {
        s.loading_library = true;
        s.error = None;
    });

    let gw = gateway::current();
    spawn(async move {
        match gw.offline_library().await {
            Ok(mut series_list) => {
                sort_library_recent_first(&mut series_list);
                store::state().update(|s| {
                    s.library = series_list;
                    s.loading_library = false;
                    s.error = None;
                });
            }
            Err(err) => {
                store::state().update(|s| {
                    s.loading_library = false;
                    s.error = Some(err.to_string());
                });
            }
        }
    });
}

pub(super) fn sort_library_recent_first(series_list: &mut [OfflineSeriesView]) {
    for series in series_list.iter_mut() {
        series.episodes.sort_by_key(|ep| ep.ep_num.get());
    }
    series_list.sort_by(|a, b| {
        latest_downloaded_epoch(b)
            .cmp(&latest_downloaded_epoch(a))
            .then_with(|| a.series_id.get().cmp(&b.series_id.get()))
    });
}

fn latest_downloaded_epoch(series: &OfflineSeriesView) -> u64 {
    series
        .episodes
        .iter()
        .map(|ep| ep.downloaded_at_epoch_secs)
        .max()
        .unwrap_or(0)
}

#[must_use]
pub(super) fn first_downloaded_episode(series: &OfflineSeriesView) -> Option<EpisodeNumber> {
    series
        .episodes
        .iter()
        .map(|ep| ep.ep_num)
        .min_by_key(|ep| ep.get())
}

#[must_use]
pub(super) fn format_size_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;

    let val = bytes as f64;
    if bytes < 1024 {
        format!("{bytes} B")
    } else if val < MB {
        format!("{:.1} KB", val / KB)
    } else if val < GB {
        format!("{:.1} MB", val / MB)
    } else {
        format!("{:.2} GB", val / GB)
    }
}

#[must_use]
pub(super) fn library_totals(series_list: &[OfflineSeriesView]) -> (usize, usize, u64) {
    let episode_count = series_list.iter().map(|s| s.episodes.len()).sum();
    let total_bytes = series_list.iter().map(|s| s.total_size_bytes).sum();
    (series_list.len(), episode_count, total_bytes)
}

#[must_use]
pub(super) fn series_stats_text(series: &OfflineSeriesView) -> String {
    format!(
        "已下载 {} 集 · {}",
        series.episodes.len(),
        format_size_bytes(series.total_size_bytes)
    )
}

#[must_use]
pub(super) fn episodes_preview_text(episodes: &[OfflineEpisodeView]) -> String {
    if episodes.is_empty() {
        return "暂无可用分集".to_owned();
    }
    let labels: Vec<String> = episodes
        .iter()
        .take(4)
        .map(|ep| format!("第 {} 集", ep.ep_num.get()))
        .collect();
    if episodes.len() > 4 {
        format!("已缓存 {} 等 {} 集", labels.join(" · "), episodes.len())
    } else {
        format!("已缓存 {}", labels.join(" · "))
    }
}
