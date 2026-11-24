use std::collections::{BTreeSet, HashMap};

use hg_contracts::ids::SeriesId;
use nsh_core::prelude::*;

use super::super::store;
use super::command::{
    BATCH_SEGMENT_SIZE, EpisodeAvailability, build_enqueue_request, classify_existing_episodes,
    episode_segment_ranges, format_enqueue_feedback, prune_unavailable_selection,
    selectable_episodes_in_range, toggle_episode_selection, toggle_select_segment,
};
use crate::gateway;

#[derive(Debug, Clone, Default)]
pub struct BatchDownloadState {
    pub open: bool,
    pub series_id: Option<SeriesId>,
    pub segment_index: usize,
    pub selected: BTreeSet<u32>,
    pub existing: HashMap<u32, EpisodeAvailability>,
    pub loading_status: bool,
    pub submitting: bool,
    pub error: Option<String>,
    pub request_seq: u64,
}

fn state() -> Signal<BatchDownloadState> {
    global::<BatchDownloadState>()
}

#[must_use]
pub fn snapshot() -> BatchDownloadState {
    state().get()
}

#[must_use]
pub fn is_open() -> bool {
    state().with(|s| s.open)
}

pub fn reset_state() {
    state().update(|s| {
        let next_seq = s.request_seq.wrapping_add(1);
        *s = BatchDownloadState {
            request_seq: next_seq,
            ..BatchDownloadState::default()
        };
    });
}

pub fn close() {
    reset_state();
    store::set_batch_download_open(false);
}

pub fn sync_from_player_store() {
    let snap = store::snapshot();
    let should_open = snap.batch_download_open;
    let series_id = snap.detail.as_ref().map(|d| d.summary.id);
    let was_open = state().with_untracked(|s| s.open);

    if should_open && !was_open {
        let Some(sid) = series_id else {
            store::set_batch_download_open(false);
            return;
        };
        let initial_seg = snap.current_episode.map_or(0, |ep| {
            ((ep.get().saturating_sub(1)) / BATCH_SEGMENT_SIZE) as usize
        });
        open_and_load_status(sid, initial_seg);
    } else if !should_open && was_open {
        reset_state();
    }
}

pub fn set_segment(index: usize) {
    state().update(|s| s.segment_index = index);
}

pub fn toggle_episode(ep_num: u32) {
    state().update(|s| {
        s.error = None;
        toggle_episode_selection(&mut s.selected, ep_num);
    });
}

pub fn toggle_select_current_segment() {
    let Some(detail) = store::snapshot().detail else {
        return;
    };
    let ranges = episode_segment_ranges(detail.total_episodes, BATCH_SEGMENT_SIZE);
    state().update(|s| {
        let active_idx = s.segment_index.min(ranges.len().saturating_sub(1));
        let range = ranges
            .get(active_idx)
            .copied()
            .unwrap_or((1, detail.total_episodes));
        let selectable = selectable_episodes_in_range(&detail.episodes, range, &s.existing);
        s.error = None;
        toggle_select_segment(&mut s.selected, &selectable);
    });
}

pub fn submit_downloads() {
    let (series_id, selected, already_submitting) =
        state().with_untracked(|s| (s.series_id, s.selected.clone(), s.submitting));
    if already_submitting {
        return;
    }
    let Some(sid) = series_id else {
        return;
    };
    let Some(req) = build_enqueue_request(sid, &selected) else {
        state().update(|s| s.error = Some("请先选择要离线下载的分集".to_owned()));
        return;
    };

    let mut seq = 0;
    state().update(|s| {
        s.request_seq = s.request_seq.wrapping_add(1);
        seq = s.request_seq;
        s.submitting = true;
        s.error = None;
    });

    let gw = gateway::current();
    spawn(async move {
        match gw.enqueue_episode_downloads(req).await {
            Ok(result) => {
                let message = format_enqueue_feedback(&result);
                let still_matching = state().with_untracked(|s| s.request_seq == seq);
                if still_matching {
                    store::set_hint(message);
                    close();
                }
            }
            Err(err) => {
                state().update(|s| {
                    if s.request_seq == seq {
                        s.submitting = false;
                        s.error = Some(err.to_string());
                    }
                });
            }
        }
    });
}

fn open_and_load_status(series_id: SeriesId, segment_index: usize) {
    let mut seq = 0;
    state().update(|s| {
        seq = s.request_seq.wrapping_add(1);
        *s = BatchDownloadState {
            open: true,
            series_id: Some(series_id),
            segment_index,
            selected: BTreeSet::new(),
            existing: HashMap::new(),
            loading_status: true,
            submitting: false,
            error: None,
            request_seq: seq,
        };
    });

    let gw = gateway::current();
    spawn(async move {
        let library = gw.offline_library().await.unwrap_or_default();
        let tasks = gw.download_tasks().await.unwrap_or_default();
        let existing = classify_existing_episodes(series_id, &library, &tasks);
        state().update(|s| {
            if s.request_seq != seq {
                return;
            }
            prune_unavailable_selection(&mut s.selected, &existing);
            s.existing = existing;
            s.loading_status = false;
        });
    });
}
