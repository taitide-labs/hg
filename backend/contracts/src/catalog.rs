use hg_contracts::ids::{EpisodeKey, SeriesId};
use serde::{Deserialize, Serialize};

use crate::error::{PortError, PortFuture};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeriesSnapshot {
    pub series_id: SeriesId,
    pub title: String,
    pub cover_url: String,
    pub total_episodes: u32,
    pub is_finished: bool,
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpisodeSnapshot {
    pub key: EpisodeKey,
    pub title: String,
    pub duration_secs: u32,
    pub playable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamEndpoints {
    pub hevc: Option<String>,
    pub main: Option<String>,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CdnAuthHeaders {
    pub referer: String,
    pub user_agent: String,
    pub auth_header_name: Option<String>,
    pub auth_header_value: Option<String>,
    pub cdn_app: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaybackTicket {
    pub key: EpisodeKey,
    pub streams: StreamEndpoints,
    pub cdn: CdnAuthHeaders,
    pub issued_at_epoch_secs: u64,
    pub expires_at_epoch_secs: u64,
}

pub trait CatalogMetadataPort: Send + Sync {
    fn fetch_series_snapshot(
        &self,
        series_id: SeriesId,
    ) -> PortFuture<'_, Result<SeriesSnapshot, PortError>>;

    fn fetch_episode_snapshot(
        &self,
        key: EpisodeKey,
    ) -> PortFuture<'_, Result<EpisodeSnapshot, PortError>>;

    fn fetch_decrypted_cover<'a>(
        &'a self,
        cover_url: &'a str,
    ) -> PortFuture<'a, Result<Vec<u8>, PortError>>;
}

pub trait CatalogPlaybackTicketPort: Send + Sync {
    fn fetch_playback_ticket(
        &self,
        key: EpisodeKey,
    ) -> PortFuture<'_, Result<PlaybackTicket, PortError>>;
}
