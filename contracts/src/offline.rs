use serde::{Deserialize, Serialize};

use crate::ids::{EpisodeKey, EpisodeNumber, SeriesId, StreamProfile};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PauseReason {
    UserRequested,
    WifiOnlyBlocked,
    AppInterrupted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DownloadFailureReason {
    PreviewStreamOnly,
    NetworkError,
    StorageFull,
    TicketExpired,
    CorruptedSegment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DownloadTaskState {
    Queued,
    FetchingSource,
    Downloading {
        completed_segments: u32,
        total_segments: u32,
        downloaded_bytes: u64,
    },
    Paused {
        reason: PauseReason,
        completed_segments: u32,
        total_segments: u32,
    },
    Failed {
        reason: DownloadFailureReason,
        completed_segments: u32,
        total_segments: u32,
    },
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadTaskView {
    pub key: EpisodeKey,
    pub series_title: String,
    pub episode_title: String,
    pub cover_url: String,
    pub profile: StreamProfile,
    pub state: DownloadTaskState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfflineEpisodeView {
    pub ep_num: EpisodeNumber,
    pub title: String,
    pub duration_secs: u32,
    pub profile: StreamProfile,
    pub size_bytes: u64,
    pub downloaded_at_epoch_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfflineSeriesView {
    pub series_id: SeriesId,
    pub title: String,
    pub cover_local_uri: String,
    pub total_episodes: u32,
    pub total_size_bytes: u64,
    pub episodes: Vec<OfflineEpisodeView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadPolicy {
    pub preferred_profile: StreamProfile,
    pub wifi_only: bool,
    pub max_concurrent_tasks: u8,
    pub storage_directory: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnqueueEpisodeDownloadsRequest {
    pub series_id: SeriesId,
    pub episodes: Vec<EpisodeNumber>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnqueueEpisodeDownloadsResult {
    pub queued: Vec<EpisodeKey>,
    pub skipped_already_present: Vec<EpisodeKey>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlDownloadAction {
    Pause,
    Resume,
    Retry,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlDownloadRequest {
    pub key: EpisodeKey,
    pub action: ControlDownloadAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlAllDownloadsAction {
    PauseAll,
    ResumeAll,
}
