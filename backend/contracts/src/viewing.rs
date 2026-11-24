use hg_contracts::ids::{EpisodeKey, SeriesId};

use crate::error::{PortError, PortFuture};

pub trait PlaybackHaltPort: Send + Sync {
    fn halt_episode(&self, key: EpisodeKey) -> PortFuture<'_, Result<(), PortError>>;

    fn halt_series(&self, series_id: SeriesId) -> PortFuture<'_, Result<(), PortError>>;
}
