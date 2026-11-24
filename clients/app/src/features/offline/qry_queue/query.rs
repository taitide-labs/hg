use hg_contracts::ids::StreamProfile;
use hg_contracts::offline::{
    DownloadFailureReason, DownloadTaskState, DownloadTaskView, PauseReason,
};
use nsh_core::prelude::*;

use super::super::store;
use crate::gateway;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TaskStatusTone {
    Brand,
    Neutral,
    Warning,
    Success,
}

pub fn load_queue() {
    store::state().update(|s| {
        s.loading_queue = true;
        s.error = None;
    });

    let gw = gateway::current();
    spawn(async move {
        match gw.download_tasks().await {
            Ok(mut tasks) => {
                tasks.retain(|task| is_active_task(task.state));
                sort_queue_tasks(&mut tasks);
                store::state().update(|s| {
                    s.tasks = tasks;
                    s.loading_queue = false;
                    s.error = None;
                });
            }
            Err(err) => {
                store::state().update(|s| {
                    s.loading_queue = false;
                    s.error = Some(err.to_string());
                });
            }
        }
    });
}

#[must_use]
pub(super) const fn is_active_task(state: DownloadTaskState) -> bool {
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

pub(super) fn sort_queue_tasks(tasks: &mut [DownloadTaskView]) {
    tasks.sort_by(|a, b| {
        state_sort_rank(a.state)
            .cmp(&state_sort_rank(b.state))
            .then_with(|| a.key.series_id.get().cmp(&b.key.series_id.get()))
            .then_with(|| a.key.ep_num.get().cmp(&b.key.ep_num.get()))
    });
}

#[must_use]
pub(super) const fn state_badge(state: DownloadTaskState) -> (&'static str, TaskStatusTone) {
    match state {
        DownloadTaskState::Queued => ("排队中", TaskStatusTone::Neutral),
        DownloadTaskState::FetchingSource => ("解析中", TaskStatusTone::Brand),
        DownloadTaskState::Downloading { .. } => ("下载中", TaskStatusTone::Brand),
        DownloadTaskState::Paused { .. } => ("已暂停", TaskStatusTone::Warning),
        DownloadTaskState::Failed { .. } => ("失败", TaskStatusTone::Warning),
        DownloadTaskState::Completed => ("已完成", TaskStatusTone::Success),
        DownloadTaskState::Cancelled => ("已取消", TaskStatusTone::Neutral),
    }
}

#[must_use]
pub(super) fn progress_ratio(state: DownloadTaskState) -> f32 {
    match state {
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
        } => {
            if total_segments == 0 {
                0.0
            } else {
                (completed_segments as f32 / total_segments as f32).clamp(0.0, 1.0)
            }
        }
        DownloadTaskState::Completed => 1.0,
        DownloadTaskState::Queued
        | DownloadTaskState::FetchingSource
        | DownloadTaskState::Cancelled => 0.0,
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    let val = bytes as f64;
    if bytes < 1024 {
        format!("{bytes} B")
    } else if val < MB {
        format!("{:.1} KB", val / KB)
    } else {
        format!("{:.1} MB", val / MB)
    }
}

const fn profile_label(profile: StreamProfile) -> &'static str {
    match profile {
        StreamProfile::Hevc => "HEVC",
        StreamProfile::Main => "标准",
    }
}

const fn pause_reason_label(reason: PauseReason) -> &'static str {
    match reason {
        PauseReason::UserRequested => "用户暂停",
        PauseReason::WifiOnlyBlocked => "等待 Wi-Fi",
        PauseReason::AppInterrupted => "应用中断",
    }
}

const fn failure_reason_label(reason: DownloadFailureReason) -> &'static str {
    match reason {
        DownloadFailureReason::PreviewStreamOnly => "仅预览流不可下载",
        DownloadFailureReason::NetworkError => "网络异常",
        DownloadFailureReason::StorageFull => "存储空间不足",
        DownloadFailureReason::TicketExpired => "凭据已过期",
        DownloadFailureReason::CorruptedSegment => "分片校验失败",
    }
}

#[must_use]
pub(super) fn progress_detail_text(state: DownloadTaskState, profile: StreamProfile) -> String {
    let quality = profile_label(profile);
    match state {
        DownloadTaskState::Queued => format!("等待调度 · {quality}"),
        DownloadTaskState::FetchingSource => format!("正在获取播放源 · {quality}"),
        DownloadTaskState::Downloading {
            completed_segments,
            total_segments,
            downloaded_bytes,
        } => format!(
            "{completed_segments}/{total_segments} 分片 · {} · {quality}",
            format_bytes(downloaded_bytes)
        ),
        DownloadTaskState::Paused {
            reason,
            completed_segments,
            total_segments,
        } => format!(
            "{completed_segments}/{total_segments} 分片 · {} · {quality}",
            pause_reason_label(reason)
        ),
        DownloadTaskState::Failed {
            reason,
            completed_segments,
            total_segments,
        } => format!(
            "{completed_segments}/{total_segments} 分片 · {} · {quality}",
            failure_reason_label(reason)
        ),
        DownloadTaskState::Completed => format!("已完成 · {quality}"),
        DownloadTaskState::Cancelled => format!("已取消 · {quality}"),
    }
}

#[must_use]
pub(super) fn queue_summary_text(tasks: &[DownloadTaskView]) -> String {
    let mut downloading = 0usize;
    let mut paused = 0usize;
    let mut failed = 0usize;
    for task in tasks {
        match task.state {
            DownloadTaskState::Downloading { .. } | DownloadTaskState::FetchingSource => {
                downloading += 1;
            }
            DownloadTaskState::Paused { .. } => paused += 1,
            DownloadTaskState::Failed { .. } => failed += 1,
            DownloadTaskState::Queued
            | DownloadTaskState::Completed
            | DownloadTaskState::Cancelled => {}
        }
    }
    format!(
        "共 {} 项 · {} 下载中 · {} 已暂停{}",
        tasks.len(),
        downloading,
        paused,
        if failed > 0 {
            format!(" · {failed} 失败")
        } else {
            String::new()
        }
    )
}
