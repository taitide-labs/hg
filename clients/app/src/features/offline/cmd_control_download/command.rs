use hg_contracts::ids::EpisodeKey;
use hg_contracts::offline::{
    ControlAllDownloadsAction, ControlDownloadAction, DownloadTaskState, DownloadTaskView,
    PauseReason,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TaskPrimaryAction {
    pub label: &'static str,
    pub action: ControlDownloadAction,
    pub emphasized: bool,
}

#[must_use]
pub(super) const fn primary_action_for_state(
    state: DownloadTaskState,
) -> Option<TaskPrimaryAction> {
    match state {
        DownloadTaskState::Queued
        | DownloadTaskState::FetchingSource
        | DownloadTaskState::Downloading { .. } => Some(TaskPrimaryAction {
            label: "暂停",
            action: ControlDownloadAction::Pause,
            emphasized: false,
        }),
        DownloadTaskState::Paused { .. } => Some(TaskPrimaryAction {
            label: "继续",
            action: ControlDownloadAction::Resume,
            emphasized: true,
        }),
        DownloadTaskState::Failed { .. } => Some(TaskPrimaryAction {
            label: "重试",
            action: ControlDownloadAction::Retry,
            emphasized: true,
        }),
        DownloadTaskState::Completed | DownloadTaskState::Cancelled => None,
    }
}

#[must_use]
pub(super) const fn can_cancel_state(state: DownloadTaskState) -> bool {
    !matches!(
        state,
        DownloadTaskState::Completed | DownloadTaskState::Cancelled
    )
}

#[must_use]
pub(super) fn cancel_a11y_label(key: EpisodeKey) -> String {
    format!(
        "取消 剧目 {} 第 {} 集下载任务",
        key.series_id.get(),
        key.ep_num.get()
    )
}

pub(super) fn apply_single_control(
    tasks: &mut Vec<DownloadTaskView>,
    key: EpisodeKey,
    action: ControlDownloadAction,
) {
    for task in tasks.iter_mut() {
        if task.key != key {
            continue;
        }
        match action {
            ControlDownloadAction::Pause => {
                let (completed_segments, total_segments) = match task.state {
                    DownloadTaskState::Downloading {
                        completed_segments,
                        total_segments,
                        ..
                    }
                    | DownloadTaskState::Paused {
                        completed_segments,
                        total_segments,
                        ..
                    }
                    | DownloadTaskState::Failed {
                        completed_segments,
                        total_segments,
                        ..
                    } => (completed_segments, total_segments),
                    DownloadTaskState::Queued
                    | DownloadTaskState::FetchingSource
                    | DownloadTaskState::Completed
                    | DownloadTaskState::Cancelled => (0, 20),
                };
                task.state = DownloadTaskState::Paused {
                    reason: PauseReason::UserRequested,
                    completed_segments,
                    total_segments,
                };
            }
            ControlDownloadAction::Resume | ControlDownloadAction::Retry => {
                task.state = DownloadTaskState::Queued;
            }
            ControlDownloadAction::Cancel => {
                task.state = DownloadTaskState::Cancelled;
            }
        }
    }
    tasks.retain(|t| {
        !matches!(
            t.state,
            DownloadTaskState::Completed | DownloadTaskState::Cancelled
        )
    });
}

pub(super) fn apply_bulk_control(
    tasks: &mut [DownloadTaskView],
    action: ControlAllDownloadsAction,
) {
    for task in tasks.iter_mut() {
        match action {
            ControlAllDownloadsAction::PauseAll => match task.state {
                DownloadTaskState::Downloading {
                    completed_segments,
                    total_segments,
                    ..
                } => {
                    task.state = DownloadTaskState::Paused {
                        reason: PauseReason::UserRequested,
                        completed_segments,
                        total_segments,
                    };
                }
                DownloadTaskState::Queued | DownloadTaskState::FetchingSource => {
                    task.state = DownloadTaskState::Paused {
                        reason: PauseReason::UserRequested,
                        completed_segments: 0,
                        total_segments: 20,
                    };
                }
                DownloadTaskState::Paused { .. }
                | DownloadTaskState::Failed { .. }
                | DownloadTaskState::Completed
                | DownloadTaskState::Cancelled => {}
            },
            ControlAllDownloadsAction::ResumeAll => {
                if matches!(task.state, DownloadTaskState::Paused { .. }) {
                    task.state = DownloadTaskState::Queued;
                }
            }
        }
    }
}
