use hg_contracts::ids::SeriesId;
use hg_contracts::offline::{DownloadTaskState, DownloadTaskView, OfflineSeriesView};
use nsh_core::prelude::*;

#[derive(Debug, Clone, Default)]
pub struct OfflineState {
    pub library: Vec<OfflineSeriesView>,
    pub tasks: Vec<DownloadTaskView>,
    pub loading_library: bool,
    pub loading_queue: bool,
    pub has_loaded_library: bool,
    pub has_loaded_queue: bool,
    pub queue_error: Option<String>,
    pub library_error: Option<String>,
    pub episodes_modal_series: Option<SeriesId>,
    pub episodes_modal_delete_mode: bool,
    pub episodes_modal_segment: usize,
}

#[must_use]
pub fn state() -> Signal<OfflineState> {
    global::<OfflineState>()
}

pub(crate) fn sort_library_recent_first(series_list: &mut [OfflineSeriesView]) {
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
pub(crate) const fn is_active_task(state: DownloadTaskState) -> bool {
    !matches!(
        state,
        DownloadTaskState::Completed | DownloadTaskState::Cancelled
    )
}

const fn state_sort_rank(state: DownloadTaskState) -> u8 {
    match state {
        DownloadTaskState::Downloading { .. } => 0,
        DownloadTaskState::FetchingSource => 1,
        DownloadTaskState::Queued => 2,
        DownloadTaskState::Paused { .. } => 3,
        DownloadTaskState::Failed { .. } => 4,
        DownloadTaskState::Completed => 5,
        DownloadTaskState::Cancelled => 6,
    }
}

pub(crate) fn sort_queue_tasks(tasks: &mut [DownloadTaskView]) {
    tasks.sort_by(|a, b| {
        state_sort_rank(a.state)
            .cmp(&state_sort_rank(b.state))
            .then_with(|| a.key.series_id.get().cmp(&b.key.series_id.get()))
            .then_with(|| a.key.ep_num.get().cmp(&b.key.ep_num.get()))
    });
}

pub(crate) fn set_library_loaded(mut library: Vec<OfflineSeriesView>) {
    sort_library_recent_first(&mut library);
    let unchanged = state().with_untracked(|s| {
        s.has_loaded_library
            && !s.loading_library
            && s.library_error.is_none()
            && s.library == library
    });
    if unchanged {
        return;
    }
    state().update(|s| {
        if s.library != library {
            s.library = library;
        }
        s.loading_library = false;
        s.has_loaded_library = true;
        s.library_error = None;
    });
}

pub(crate) fn set_queue_loaded(mut tasks: Vec<DownloadTaskView>) {
    tasks.retain(|task| is_active_task(task.state));
    sort_queue_tasks(&mut tasks);
    let unchanged = state().with_untracked(|s| {
        s.has_loaded_queue && !s.loading_queue && s.queue_error.is_none() && s.tasks == tasks
    });
    if unchanged {
        return;
    }
    state().update(|s| {
        if s.tasks != tasks {
            s.tasks = tasks;
        }
        s.loading_queue = false;
        s.has_loaded_queue = true;
        s.queue_error = None;
    });
}

#[must_use]
pub(crate) fn has_running_tasks() -> bool {
    state().with(|s| {
        s.tasks.iter().any(|task| {
            matches!(
                task.state,
                DownloadTaskState::Queued
                    | DownloadTaskState::FetchingSource
                    | DownloadTaskState::Downloading { .. }
            )
        })
    })
}

pub(crate) fn open_episodes_modal(series_id: SeriesId) {
    state().update(|s| {
        s.episodes_modal_series = Some(series_id);
        s.episodes_modal_delete_mode = false;
        s.episodes_modal_segment = 0;
    });
}

pub(crate) fn close_episodes_modal() {
    state().update(|s| {
        s.episodes_modal_series = None;
        s.episodes_modal_delete_mode = false;
        s.episodes_modal_segment = 0;
    });
}

pub(crate) fn toggle_episodes_modal_delete_mode() {
    state().update(|s| {
        s.episodes_modal_delete_mode = !s.episodes_modal_delete_mode;
    });
}

pub(crate) fn set_episodes_modal_segment(index: usize) {
    state().update(|s| {
        s.episodes_modal_segment = index;
    });
}
