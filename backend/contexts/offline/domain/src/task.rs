use std::collections::HashSet;

use hg_backend_contracts::catalog::StreamEndpoints;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId, StreamProfile};
use hg_contracts::offline::{DownloadFailureReason, DownloadTaskState, PauseReason};

use crate::error::OfflineError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedDownloadStream {
    pub profile: StreamProfile,
    pub playlist_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnqueuePlan {
    pub to_queue: Vec<EpisodeKey>,
    pub skipped_already_present: Vec<EpisodeKey>,
}

pub fn select_download_stream(
    streams: &StreamEndpoints,
    preferred: StreamProfile,
) -> Result<SelectedDownloadStream, OfflineError> {
    let ordered = match preferred {
        StreamProfile::Hevc => [
            (StreamProfile::Hevc, streams.hevc.as_deref()),
            (StreamProfile::Main, streams.main.as_deref()),
        ],
        StreamProfile::Main => [
            (StreamProfile::Main, streams.main.as_deref()),
            (StreamProfile::Hevc, streams.hevc.as_deref()),
        ],
    };

    for (profile, candidate) in ordered {
        if let Some(url) = candidate.map(str::trim).filter(|u| !u.is_empty()) {
            return Ok(SelectedDownloadStream {
                profile,
                playlist_url: url.to_owned(),
            });
        }
    }

    Err(OfflineError::PreviewOnlyNotDownloadable)
}

pub fn plan_episode_enqueue(
    series_id: SeriesId,
    requested: &[EpisodeNumber],
    existing_present: &HashSet<EpisodeKey>,
) -> Result<EnqueuePlan, OfflineError> {
    if requested.is_empty() {
        return Err(OfflineError::EmptyEpisodeSelection);
    }

    let mut seen = HashSet::new();
    let mut to_queue = Vec::new();
    let mut skipped_already_present = Vec::new();

    for &ep_num in requested {
        let key = EpisodeKey::new(series_id, ep_num);
        if !seen.insert(key) {
            continue;
        }
        if existing_present.contains(&key) {
            skipped_already_present.push(key);
        } else {
            to_queue.push(key);
        }
    }

    Ok(EnqueuePlan {
        to_queue,
        skipped_already_present,
    })
}

pub const fn state_label(state: DownloadTaskState) -> &'static str {
    match state {
        DownloadTaskState::Queued => "queued",
        DownloadTaskState::FetchingSource => "fetching_source",
        DownloadTaskState::Downloading { .. } => "downloading",
        DownloadTaskState::Paused { .. } => "paused",
        DownloadTaskState::Failed { .. } => "failed",
        DownloadTaskState::Completed => "completed",
        DownloadTaskState::Cancelled => "cancelled",
    }
}

pub fn transition_to_fetching(state: DownloadTaskState) -> Result<DownloadTaskState, OfflineError> {
    match state {
        DownloadTaskState::Queued => Ok(DownloadTaskState::FetchingSource),
        DownloadTaskState::FetchingSource
        | DownloadTaskState::Downloading { .. }
        | DownloadTaskState::Paused { .. }
        | DownloadTaskState::Failed { .. }
        | DownloadTaskState::Completed
        | DownloadTaskState::Cancelled => Err(OfflineError::InvalidTransition {
            from: state_label(state),
            action: "begin_fetching",
        }),
    }
}

pub fn transition_to_downloading(
    state: DownloadTaskState,
    completed_segments: u32,
    total_segments: u32,
    downloaded_bytes: u64,
) -> Result<DownloadTaskState, OfflineError> {
    match state {
        DownloadTaskState::FetchingSource | DownloadTaskState::Downloading { .. } => {
            Ok(DownloadTaskState::Downloading {
                completed_segments: completed_segments.min(total_segments),
                total_segments,
                downloaded_bytes,
            })
        }
        DownloadTaskState::Queued
        | DownloadTaskState::Paused { .. }
        | DownloadTaskState::Failed { .. }
        | DownloadTaskState::Completed
        | DownloadTaskState::Cancelled => Err(OfflineError::InvalidTransition {
            from: state_label(state),
            action: "downloading",
        }),
    }
}

pub fn transition_to_paused(
    state: DownloadTaskState,
    reason: PauseReason,
) -> Result<DownloadTaskState, OfflineError> {
    match state {
        DownloadTaskState::Queued | DownloadTaskState::FetchingSource => {
            Ok(DownloadTaskState::Paused {
                reason,
                completed_segments: 0,
                total_segments: 0,
            })
        }
        DownloadTaskState::Downloading {
            completed_segments,
            total_segments,
            ..
        } => Ok(DownloadTaskState::Paused {
            reason,
            completed_segments,
            total_segments,
        }),
        DownloadTaskState::Paused { .. }
        | DownloadTaskState::Failed { .. }
        | DownloadTaskState::Completed
        | DownloadTaskState::Cancelled => Err(OfflineError::InvalidTransition {
            from: state_label(state),
            action: "pause",
        }),
    }
}

pub fn transition_to_resumed(state: DownloadTaskState) -> Result<DownloadTaskState, OfflineError> {
    match state {
        DownloadTaskState::Paused { .. } => Ok(DownloadTaskState::Queued),
        DownloadTaskState::Queued
        | DownloadTaskState::FetchingSource
        | DownloadTaskState::Downloading { .. }
        | DownloadTaskState::Failed { .. }
        | DownloadTaskState::Completed
        | DownloadTaskState::Cancelled => Err(OfflineError::InvalidTransition {
            from: state_label(state),
            action: "resume",
        }),
    }
}

pub fn transition_to_failed(
    state: DownloadTaskState,
    reason: DownloadFailureReason,
) -> Result<DownloadTaskState, OfflineError> {
    match state {
        DownloadTaskState::FetchingSource => Ok(DownloadTaskState::Failed {
            reason,
            completed_segments: 0,
            total_segments: 0,
        }),
        DownloadTaskState::Downloading {
            completed_segments,
            total_segments,
            ..
        } => Ok(DownloadTaskState::Failed {
            reason,
            completed_segments,
            total_segments,
        }),
        DownloadTaskState::Queued
        | DownloadTaskState::Paused { .. }
        | DownloadTaskState::Failed { .. }
        | DownloadTaskState::Completed
        | DownloadTaskState::Cancelled => Err(OfflineError::InvalidTransition {
            from: state_label(state),
            action: "fail",
        }),
    }
}

pub fn transition_to_retried(state: DownloadTaskState) -> Result<DownloadTaskState, OfflineError> {
    match state {
        DownloadTaskState::Failed { .. } => Ok(DownloadTaskState::Queued),
        DownloadTaskState::Queued
        | DownloadTaskState::FetchingSource
        | DownloadTaskState::Downloading { .. }
        | DownloadTaskState::Paused { .. }
        | DownloadTaskState::Completed
        | DownloadTaskState::Cancelled => Err(OfflineError::InvalidTransition {
            from: state_label(state),
            action: "retry",
        }),
    }
}

pub fn transition_to_completed(
    state: DownloadTaskState,
) -> Result<DownloadTaskState, OfflineError> {
    match state {
        DownloadTaskState::Downloading { .. } => Ok(DownloadTaskState::Completed),
        DownloadTaskState::Queued
        | DownloadTaskState::FetchingSource
        | DownloadTaskState::Paused { .. }
        | DownloadTaskState::Failed { .. }
        | DownloadTaskState::Completed
        | DownloadTaskState::Cancelled => Err(OfflineError::InvalidTransition {
            from: state_label(state),
            action: "complete",
        }),
    }
}

pub fn transition_to_cancelled(
    state: DownloadTaskState,
) -> Result<DownloadTaskState, OfflineError> {
    match state {
        DownloadTaskState::Queued
        | DownloadTaskState::FetchingSource
        | DownloadTaskState::Downloading { .. }
        | DownloadTaskState::Paused { .. }
        | DownloadTaskState::Failed { .. } => Ok(DownloadTaskState::Cancelled),
        DownloadTaskState::Completed | DownloadTaskState::Cancelled => {
            Err(OfflineError::InvalidTransition {
                from: state_label(state),
                action: "cancel",
            })
        }
    }
}
