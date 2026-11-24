use hg_contracts::ids::StreamProfile;
use hg_contracts::offline::{
    DownloadFailureReason, DownloadPolicy, DownloadTaskState, DownloadTaskView, PauseReason,
};

use super::query::{DownloadPolicyRow, DownloadTaskRow};

#[test]
fn maps_policy_row_to_contract() {
    let row = DownloadPolicyRow {
        preferred_profile: "main".to_owned(),
        wifi_only: 0,
        max_concurrent_tasks: 4,
        storage_directory: Some("/custom/offline".to_owned()),
    };

    let policy = DownloadPolicy::try_from(row).expect("valid policy row");
    assert_eq!(policy.preferred_profile, StreamProfile::Main);
    assert!(!policy.wifi_only);
    assert_eq!(policy.max_concurrent_tasks, 4);
    assert_eq!(policy.storage_directory.as_deref(), Some("/custom/offline"));
}

#[test]
fn maps_all_download_task_states_and_rejects_missing_reasons() {
    let base =
        |state: &str, pause_reason: Option<&str>, failure_reason: Option<&str>| DownloadTaskRow {
            series_id: 101,
            ep_num: 2,
            series_title: "夜雨长灯".to_owned(),
            episode_title: "第2集".to_owned(),
            cover_url: "/covers/101.jpg".to_owned(),
            profile: "hevc".to_owned(),
            state: state.to_owned(),
            pause_reason: pause_reason.map(str::to_owned),
            failure_reason: failure_reason.map(str::to_owned),
            completed_segments: 3,
            total_segments: 10,
            downloaded_bytes: 2048,
        };

    let paused = DownloadTaskView::try_from(base("paused", Some("wifi_only_blocked"), None))
        .expect("paused");
    assert_eq!(
        paused.state,
        DownloadTaskState::Paused {
            reason: PauseReason::WifiOnlyBlocked,
            completed_segments: 3,
            total_segments: 10,
        }
    );

    let failed = DownloadTaskView::try_from(base("failed", None, Some("preview_stream_only")))
        .expect("failed");
    assert_eq!(
        failed.state,
        DownloadTaskState::Failed {
            reason: DownloadFailureReason::PreviewStreamOnly,
            completed_segments: 3,
            total_segments: 10,
        }
    );

    let downloading =
        DownloadTaskView::try_from(base("downloading", None, None)).expect("downloading");
    assert_eq!(
        downloading.state,
        DownloadTaskState::Downloading {
            completed_segments: 3,
            total_segments: 10,
            downloaded_bytes: 2048,
        }
    );

    assert!(DownloadTaskView::try_from(base("paused", None, None)).is_err());
    assert!(DownloadTaskView::try_from(base("failed", None, None)).is_err());
    assert!(DownloadTaskView::try_from(base("unknown", None, None)).is_err());
}
