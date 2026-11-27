use std::collections::HashSet;

use hg_backend_contracts::catalog::StreamEndpoints;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId, StreamProfile};
use hg_contracts::offline::{
    DownloadFailureReason, DownloadPolicy, DownloadTaskState, PauseReason,
};

use crate::error::OfflineError;
use crate::policy::{default_download_policy, validate_download_policy};
use crate::task::{
    plan_episode_enqueue, select_download_stream, state_label, transition_to_cancelled,
    transition_to_completed, transition_to_downloading, transition_to_failed,
    transition_to_fetching, transition_to_paused, transition_to_resumed, transition_to_retried,
};

#[test]
fn rejects_preview_only_streams_for_offline_download() {
    let preview_only = StreamEndpoints {
        hevc: None,
        main: Some("   ".to_owned()),
        preview: Some("https://cdn.example.com/preview.m3u8".to_owned()),
    };

    assert_eq!(
        select_download_stream(&preview_only, StreamProfile::Hevc),
        Err(OfflineError::PreviewOnlyNotDownloadable)
    );
}

#[test]
fn falls_back_between_hevc_and_main_per_preference() {
    let main_only = StreamEndpoints {
        hevc: None,
        main: Some("https://cdn.example.com/main.m3u8".to_owned()),
        preview: Some("https://cdn.example.com/preview.m3u8".to_owned()),
    };

    let selected = select_download_stream(&main_only, StreamProfile::Hevc).expect("should select");
    assert_eq!(selected.profile, StreamProfile::Main);
    assert_eq!(selected.playlist_url, "https://cdn.example.com/main.m3u8");
}

#[test]
fn deduplicates_and_skips_already_present_episodes() {
    let series_id = SeriesId::try_from(14288).expect("valid series id");
    let ep1 = EpisodeNumber::try_from(1).expect("ep1");
    let ep2 = EpisodeNumber::try_from(2).expect("ep2");
    let ep3 = EpisodeNumber::try_from(3).expect("ep3");

    let existing = HashSet::from([EpisodeKey::new(series_id, ep1)]);
    let plan = plan_episode_enqueue(series_id, &[ep1, ep2, ep2, ep3], &existing)
        .expect("should build plan");

    assert_eq!(
        plan.skipped_already_present,
        vec![EpisodeKey::new(series_id, ep1)]
    );
    assert_eq!(
        plan.to_queue,
        vec![
            EpisodeKey::new(series_id, ep2),
            EpisodeKey::new(series_id, ep3)
        ]
    );
}

#[test]
fn enforces_seven_state_download_lifecycle() {
    let s = DownloadTaskState::Queued;
    let s = transition_to_fetching(s).expect("queued -> fetching");
    let s = transition_to_downloading(s, 3, 10, 4096).expect("fetching -> downloading");
    let s = transition_to_paused(s, PauseReason::UserRequested).expect("downloading -> paused");
    let s = transition_to_resumed(s).expect("paused -> queued");
    let s = transition_to_fetching(s).expect("queued -> fetching");
    let s =
        transition_to_failed(s, DownloadFailureReason::NetworkError).expect("fetching -> failed");
    let s = transition_to_retried(s).expect("failed -> queued");
    let s = transition_to_fetching(s).expect("queued -> fetching");
    let s = transition_to_downloading(s, 10, 10, 10240).expect("fetching -> downloading");
    let s = transition_to_completed(s).expect("downloading -> completed");

    assert_eq!(s, DownloadTaskState::Completed);
    assert!(transition_to_cancelled(s).is_err());

    assert_eq!(state_label(DownloadTaskState::Queued), "queued");
    assert_eq!(
        state_label(DownloadTaskState::FetchingSource),
        "fetching_source"
    );
    assert_eq!(
        state_label(DownloadTaskState::Downloading {
            completed_segments: 1,
            total_segments: 2,
            downloaded_bytes: 100,
        }),
        "downloading"
    );
    assert_eq!(
        state_label(DownloadTaskState::Paused {
            reason: PauseReason::UserRequested,
            completed_segments: 1,
            total_segments: 2,
        }),
        "paused"
    );
    assert_eq!(
        state_label(DownloadTaskState::Failed {
            reason: DownloadFailureReason::NetworkError,
            completed_segments: 1,
            total_segments: 2,
        }),
        "failed"
    );
    assert_eq!(state_label(DownloadTaskState::Completed), "completed");
    assert_eq!(state_label(DownloadTaskState::Cancelled), "cancelled");
}

#[test]
fn validates_download_policy_bounds() {
    assert!(validate_download_policy(&default_download_policy()).is_ok());

    let invalid = DownloadPolicy {
        preferred_profile: StreamProfile::Main,
        wifi_only: false,
        max_concurrent_tasks: 0,
        storage_directory: None,
    };
    assert!(matches!(
        validate_download_policy(&invalid),
        Err(OfflineError::InvalidPolicy { .. })
    ));
}
