use std::num::{NonZeroU32, NonZeroU64};

use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId, StreamProfile};
use hg_contracts::offline::{
    ControlAllDownloadsAction, ControlDownloadAction, DownloadFailureReason, DownloadTaskState,
    DownloadTaskView, PauseReason,
};

use super::command::{
    apply_bulk_control, apply_single_control, can_cancel_state, cancel_a11y_label,
    primary_action_for_state,
};

fn key(series: u64, ep: u32) -> EpisodeKey {
    EpisodeKey::new(
        SeriesId::new(NonZeroU64::new(series).unwrap_or(NonZeroU64::MIN)),
        EpisodeNumber::new(NonZeroU32::new(ep).unwrap_or(NonZeroU32::MIN)),
    )
}

fn sample_task(series: u64, ep: u32, state: DownloadTaskState) -> DownloadTaskView {
    DownloadTaskView {
        key: key(series, ep),
        series_title: format!("剧目 {series}"),
        episode_title: format!("第 {ep} 集"),
        cover_url: "poster://amber-noir".to_owned(),
        profile: StreamProfile::Hevc,
        state,
    }
}

#[test]
fn primary_action_matches_each_task_state() {
    let downloading = DownloadTaskState::Downloading {
        completed_segments: 4,
        total_segments: 20,
        downloaded_bytes: 1_000_000,
    };
    let paused = DownloadTaskState::Paused {
        reason: PauseReason::UserRequested,
        completed_segments: 4,
        total_segments: 20,
    };
    let failed = DownloadTaskState::Failed {
        reason: DownloadFailureReason::NetworkError,
        completed_segments: 4,
        total_segments: 20,
    };

    assert_eq!(
        primary_action_for_state(downloading).map(|a| (a.label, a.action)),
        Some(("暂停", ControlDownloadAction::Pause))
    );
    assert_eq!(
        primary_action_for_state(DownloadTaskState::Queued).map(|a| (a.label, a.action)),
        Some(("暂停", ControlDownloadAction::Pause))
    );
    assert_eq!(
        primary_action_for_state(paused).map(|a| (a.label, a.action)),
        Some(("继续", ControlDownloadAction::Resume))
    );
    assert_eq!(
        primary_action_for_state(failed).map(|a| (a.label, a.action)),
        Some(("重试", ControlDownloadAction::Retry))
    );
    assert!(primary_action_for_state(DownloadTaskState::Completed).is_none());
    assert!(can_cancel_state(failed));
    assert!(!can_cancel_state(DownloadTaskState::Completed));
    assert_eq!(
        cancel_a11y_label(key(1001, 4)),
        "取消 剧目 1001 第 4 集下载任务"
    );
}

#[test]
fn apply_bulk_control_pauses_active_and_resumes_paused_tasks() {
    let mut tasks = vec![
        sample_task(
            1001,
            4,
            DownloadTaskState::Downloading {
                completed_segments: 14,
                total_segments: 24,
                downloaded_bytes: 11_400_000,
            },
        ),
        sample_task(
            1002,
            2,
            DownloadTaskState::Paused {
                reason: PauseReason::UserRequested,
                completed_segments: 6,
                total_segments: 22,
            },
        ),
    ];

    apply_bulk_control(&mut tasks, ControlAllDownloadsAction::PauseAll);
    assert!(matches!(
        tasks.first().map(|t| t.state),
        Some(DownloadTaskState::Paused {
            completed_segments: 14,
            total_segments: 24,
            ..
        })
    ));

    apply_bulk_control(&mut tasks, ControlAllDownloadsAction::ResumeAll);
    assert!(
        tasks
            .iter()
            .all(|t| matches!(t.state, DownloadTaskState::Queued))
    );
}

#[test]
fn apply_single_control_supports_pause_resume_retry_and_cancel() {
    let k1 = key(1001, 4);
    let k2 = key(1002, 2);
    let mut tasks = vec![
        sample_task(
            1001,
            4,
            DownloadTaskState::Downloading {
                completed_segments: 10,
                total_segments: 20,
                downloaded_bytes: 5_000_000,
            },
        ),
        sample_task(
            1002,
            2,
            DownloadTaskState::Failed {
                reason: DownloadFailureReason::NetworkError,
                completed_segments: 3,
                total_segments: 20,
            },
        ),
    ];

    apply_single_control(&mut tasks, k1, ControlDownloadAction::Pause);
    assert!(matches!(
        tasks.first().map(|t| t.state),
        Some(DownloadTaskState::Paused { .. })
    ));

    apply_single_control(&mut tasks, k2, ControlDownloadAction::Retry);
    assert!(matches!(
        tasks.get(1).map(|t| t.state),
        Some(DownloadTaskState::Queued)
    ));

    apply_single_control(&mut tasks, k1, ControlDownloadAction::Resume);
    assert!(matches!(
        tasks.first().map(|t| t.state),
        Some(DownloadTaskState::Queued)
    ));

    apply_single_control(&mut tasks, k1, ControlDownloadAction::Cancel);
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks.first().map(|t| t.key), Some(k2));
}
