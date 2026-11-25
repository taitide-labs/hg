use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId};
use hg_contracts::offline::OfflineSeriesView;

pub(super) fn remove_series_from_library(
    library: &mut Vec<OfflineSeriesView>,
    series_id: SeriesId,
) {
    library.retain(|series| series.series_id != series_id);
}

pub(super) fn remove_episode_from_library(library: &mut Vec<OfflineSeriesView>, key: EpisodeKey) {
    for series in library.iter_mut() {
        if series.series_id == key.series_id {
            series.episodes.retain(|ep| ep.ep_num != key.ep_num);
            series.total_size_bytes = series.episodes.iter().map(|ep| ep.size_bytes).sum();
        }
    }
    library.retain(|series| !series.episodes.is_empty());
}

#[must_use]
pub(super) fn delete_series_a11y_label(title: &str) -> String {
    format!("删除 {title} 离线剧目")
}

#[must_use]
pub(super) fn manage_episodes_a11y_label(title: &str) -> String {
    format!("管理 {title} 离线分集")
}

#[must_use]
pub(super) fn delete_episode_a11y_label(title: &str, ep_num: EpisodeNumber) -> String {
    format!("删除 {title} 第 {} 集离线分集", ep_num.get())
}

#[must_use]
pub(super) fn play_episode_a11y_label(title: &str, ep_num: EpisodeNumber) -> String {
    format!("播放 {title} 第 {} 集", ep_num.get())
}

#[must_use]
pub(super) fn undownloaded_episode_a11y_label(title: &str, ep_num: u32) -> String {
    format!("{title} 第 {ep_num} 集 未下载")
}

#[must_use]
pub(super) fn episode_segment_ranges(total_episodes: u32, segment_size: u32) -> Vec<(u32, u32)> {
    if total_episodes == 0 || segment_size == 0 {
        return Vec::new();
    }
    let mut ranges = Vec::new();
    let mut start = 1_u32;
    while start <= total_episodes {
        let end = start
            .saturating_add(segment_size.saturating_sub(1))
            .min(total_episodes);
        ranges.push((start, end));
        start = end.saturating_add(1);
    }
    ranges
}
