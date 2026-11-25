use serde::{Deserialize, Serialize};

use crate::ids::{EpisodeKey, EpisodeNumber, SeriesId, StreamProfile};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OnlineStreamKind {
    Hevc,
    Main,
    Preview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackOrigin {
    OfflineLocal { profile: StreamProfile },
    OnlineStream { kind: OnlineStreamKind },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaybackSource {
    pub key: EpisodeKey,
    pub playable_uri: String,
    pub origin: PlaybackOrigin,
    pub duration_secs: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlaybackMode {
    ShortDrama,
    Cinema,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExternalPlayerKind {
    Mpv,
    Iina,
    MovistPro,
    PotPlayer,
    SystemDefault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchProgress {
    pub key: EpisodeKey,
    pub position_secs: u32,
    pub duration_secs: u32,
    pub completed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchHistoryEntry {
    pub series_id: SeriesId,
    pub title: String,
    pub cover_url: String,
    pub last_episode: EpisodeNumber,
    pub position_secs: u32,
    pub duration_secs: u32,
    pub episode_completed: bool,
    pub total_episodes: u32,
    pub watched_at_epoch_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeriesBookmark {
    pub series_id: SeriesId,
    pub title: String,
    pub cover_url: String,
    pub episode_count: u32,
    pub is_finished: bool,
    pub score: f32,
    pub bookmarked_at_epoch_secs: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum AppAppearance {
    System,
    Light,
    #[default]
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaybackPreference {
    pub preferred_profile: StreamProfile,
    pub playback_cache_limit_mb: u32,
    pub default_rate_hundredths: u16,
    pub appearance: AppAppearance,
    pub sidebar_collapsed: bool,
    pub sidebar_width: u16,
}

impl Default for PlaybackPreference {
    fn default() -> Self {
        Self {
            preferred_profile: StreamProfile::Hevc,
            playback_cache_limit_mb: 512,
            default_rate_hundredths: 100,
            appearance: AppAppearance::Dark,
            sidebar_collapsed: false,
            sidebar_width: 208,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchHistoryEntry {
    pub keyword: String,
    pub searched_at_epoch_secs: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageOverview {
    pub offline_library_bytes: u64,
    pub playback_cache_bytes: u64,
    pub catalog_cache_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportWatchProgressRequest {
    pub key: EpisodeKey,
    pub position_secs: u32,
    pub duration_secs: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportWatchProgressResult {
    pub progress: WatchProgress,
    pub next_episode: Option<EpisodeNumber>,
}
