use hg_contracts::ids::{EpisodeKey, SeriesId, StreamProfile};
use serde::{Deserialize, Serialize};

use crate::error::{PortError, PortFuture};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommittedOfflineAsset {
    pub key: EpisodeKey,
    pub profile: StreamProfile,
    pub playlist_rel_path: String,
    pub duration_secs: u32,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum OfflineDomainEvent {
    OfflineEpisodeRemoved {
        key: EpisodeKey,
        asset_rel_dir: String,
    },
    OfflineSeriesRemoved {
        series_id: SeriesId,
        asset_rel_dir: String,
    },
}

pub trait OfflineAssetQueryPort: Send + Sync {
    fn find_committed_episode(
        &self,
        key: EpisodeKey,
    ) -> PortFuture<'_, Result<Option<CommittedOfflineAsset>, PortError>>;

    fn total_offline_library_bytes(&self) -> PortFuture<'_, Result<u64, PortError>>;
}
