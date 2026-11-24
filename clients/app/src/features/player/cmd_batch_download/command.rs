use std::collections::{BTreeSet, HashMap};
use std::num::NonZeroU32;

use hg_contracts::catalog::EpisodeSummary;
use hg_contracts::ids::{EpisodeNumber, SeriesId};
use hg_contracts::offline::{
    DownloadTaskState, DownloadTaskView, EnqueueEpisodeDownloadsRequest,
    EnqueueEpisodeDownloadsResult, OfflineSeriesView,
};

pub const BATCH_SEGMENT_SIZE: u32 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpisodeAvailability {
    Available,
    AlreadyOffline,
    AlreadyQueued,
}

#[must_use]
pub fn episode_segment_ranges(total: u32, chunk: u32) -> Vec<(u32, u32)> {
    if total == 0 || chunk == 0 {
        return Vec::new();
    }
    let mut ranges = Vec::new();
    let mut start = 1;
    while start <= total {
        let end = (start + chunk - 1).min(total);
        ranges.push((start, end));
        start = end.saturating_add(1);
    }
    ranges
}

#[must_use]
pub fn classify_existing_episodes(
    series_id: SeriesId,
    offline_library: &[OfflineSeriesView],
    download_tasks: &[DownloadTaskView],
) -> HashMap<u32, EpisodeAvailability> {
    let mut map = HashMap::new();

    for series in offline_library {
        if series.series_id == series_id {
            for ep in &series.episodes {
                map.insert(ep.ep_num.get(), EpisodeAvailability::AlreadyOffline);
            }
        }
    }

    for task in download_tasks {
        if task.key.series_id != series_id {
            continue;
        }
        let num = task.key.ep_num.get();
        match task.state {
            DownloadTaskState::Completed => {
                map.insert(num, EpisodeAvailability::AlreadyOffline);
            }
            DownloadTaskState::Cancelled => {}
            DownloadTaskState::Queued
            | DownloadTaskState::FetchingSource
            | DownloadTaskState::Downloading { .. }
            | DownloadTaskState::Paused { .. }
            | DownloadTaskState::Failed { .. } => {
                map.entry(num).or_insert(EpisodeAvailability::AlreadyQueued);
            }
        }
    }

    map
}

#[must_use]
pub fn availability_for(
    ep_num: u32,
    existing: &HashMap<u32, EpisodeAvailability>,
) -> EpisodeAvailability {
    existing
        .get(&ep_num)
        .copied()
        .unwrap_or(EpisodeAvailability::Available)
}

#[must_use]
pub fn selectable_episodes_in_range(
    episodes: &[EpisodeSummary],
    range: (u32, u32),
    existing: &HashMap<u32, EpisodeAvailability>,
) -> Vec<u32> {
    episodes
        .iter()
        .filter(|ep| {
            let num = ep.ep_num.get();
            num >= range.0
                && num <= range.1
                && ep.playable
                && availability_for(num, existing) == EpisodeAvailability::Available
        })
        .map(|ep| ep.ep_num.get())
        .collect()
}

#[must_use]
pub fn is_segment_fully_selected(selected: &BTreeSet<u32>, segment_selectable: &[u32]) -> bool {
    !segment_selectable.is_empty() && segment_selectable.iter().all(|num| selected.contains(num))
}

pub fn toggle_episode_selection(selected: &mut BTreeSet<u32>, ep_num: u32) {
    if !selected.remove(&ep_num) {
        selected.insert(ep_num);
    }
}

pub fn toggle_select_segment(selected: &mut BTreeSet<u32>, segment_selectable: &[u32]) {
    if is_segment_fully_selected(selected, segment_selectable) {
        for num in segment_selectable {
            selected.remove(num);
        }
    } else {
        for &num in segment_selectable {
            selected.insert(num);
        }
    }
}

pub fn prune_unavailable_selection(
    selected: &mut BTreeSet<u32>,
    existing: &HashMap<u32, EpisodeAvailability>,
) {
    selected.retain(|num| availability_for(*num, existing) == EpisodeAvailability::Available);
}

#[must_use]
pub fn build_enqueue_request(
    series_id: SeriesId,
    selected: &BTreeSet<u32>,
) -> Option<EnqueueEpisodeDownloadsRequest> {
    let episodes: Vec<EpisodeNumber> = selected
        .iter()
        .filter_map(|&num| NonZeroU32::new(num).map(EpisodeNumber::new))
        .collect();
    if episodes.is_empty() {
        None
    } else {
        Some(EnqueueEpisodeDownloadsRequest {
            series_id,
            episodes,
        })
    }
}

#[must_use]
pub fn format_enqueue_feedback(result: &EnqueueEpisodeDownloadsResult) -> String {
    let queued = result.queued.len();
    let skipped = result.skipped_already_present.len();
    match (queued, skipped) {
        (0, _) => "所选分集已在离线片库或下载队列中".to_owned(),
        (q, 0) => format!("已加入离线下载（{q} 集）"),
        (q, s) => format!("已加入离线下载（{q} 集，已跳过 {s} 集）"),
    }
}
