use std::num::{NonZeroU32, NonZeroU64};

use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId, StreamProfile};
use hg_contracts::offline::{
    DownloadFailureReason, DownloadTaskState, DownloadTaskView, PauseReason,
};

use super::query::{
    TaskStatusTone, is_active_task, progress_detail_text, progress_ratio, queue_summary_text,
    sort_queue_tasks, state_badge,
};

fn sample_task(id: u64, ep: u32, state: DownloadTaskState) -> DownloadTaskView {
    DownloadTaskView {
        key: EpisodeKey::new(
            SeriesId::new(NonZeroU64::new(id).unwrap_or(NonZeroU64::MIN)),
            EpisodeNumber::new(NonZeroU32::new(ep).unwrap_or(NonZeroU32::MIN)),
        ),
        series_title: format!("剧目 {id}"),
        episode_title: format!("第 {ep} 集"),
        cover_url: "poster://amber-noir".to_owned(),
        profile: StreamProfile::Hevc,
        state,
    }
}

#[test]
fn active_task_filter_and_sort_prioritize_running_tasks() {
    let mut tasks = vec![
        sample_task(
            1002,
            2,
            DownloadTaskState::Paused {
                reason: PauseReason::UserRequested,
                completed_segments: 6,
                total_segments: 22,
            },
        ),
        sample_task(1003, 1, DownloadTaskState::Completed),
        sample_task(1001, 5, DownloadTaskState::Queued),
        sample_task(
            1001,
            4,
            DownloadTaskState::Downloading {
                completed_segments: 14,
                total_segments: 24,
                downloaded_bytes: 11_400_000,
            },
        ),
    ];

    tasks.retain(|t| is_active_task(t.state));
    sort_queue_tasks(&mut tasks);

    let ordered_eps: Vec<u32> = tasks.iter().map(|t| t.key.ep_num.get()).collect();
    assert_eq!(ordered_eps, vec![4, 5, 2]);
}

#[test]
fn progress_ratio_and_detail_cover_all_active_states() {
    let downloading = DownloadTaskState::Downloading {
        completed_segments: 12,
        total_segments: 24,
        downloaded_bytes: 10_485_760,
    };
    assert!((progress_ratio(downloading) - 0.5).abs() < 0.001);
    assert_eq!(state_badge(downloading), ("下载中", TaskStatusTone::Brand));
    assert_eq!(
        progress_detail_text(downloading, StreamProfile::Hevc),
        "12/24 分片 · 10.0 MB · HEVC"
    );

    let paused = DownloadTaskState::Paused {
        reason: PauseReason::WifiOnlyBlocked,
        completed_segments: 3,
        total_segments: 12,
    };
    assert!((progress_ratio(paused) - 0.25).abs() < 0.001);
    assert_eq!(state_badge(paused), ("已暂停", TaskStatusTone::Warning));
    assert_eq!(
        progress_detail_text(paused, StreamProfile::Main),
        "3/12 分片 · 等待 Wi-Fi · 标准"
    );

    let failed = DownloadTaskState::Failed {
        reason: DownloadFailureReason::NetworkError,
        completed_segments: 0,
        total_segments: 0,
    };
    assert!(progress_ratio(failed).abs() < f32::EPSILON);
    assert_eq!(state_badge(failed), ("失败", TaskStatusTone::Warning));
    assert_eq!(
        progress_detail_text(failed, StreamProfile::Hevc),
        "0/0 分片 · 网络异常 · HEVC"
    );

    let queued = DownloadTaskState::Queued;
    assert_eq!(state_badge(queued), ("排队中", TaskStatusTone::Neutral));
}

#[test]
fn queue_summary_counts_active_states() {
    let tasks = vec![
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
        sample_task(
            1003,
            1,
            DownloadTaskState::Failed {
                reason: DownloadFailureReason::TicketExpired,
                completed_segments: 2,
                total_segments: 20,
            },
        ),
    ];

    assert_eq!(
        queue_summary_text(&tasks),
        "共 3 项 · 1 下载中 · 1 已暂停 · 1 失败"
    );
}
