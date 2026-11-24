use hg_backend_contracts::catalog::{CatalogPlaybackTicketPort, PlaybackTicket};
use hg_backend_contracts::error::{PortError, PortFuture};
use hg_contracts::ids::EpisodeKey;

use crate::CatalogDeps;

use super::command::FEATURE_NAME;

#[derive(Clone)]
pub struct CatalogPlaybackTicketAdapter {
    deps: CatalogDeps,
}

impl CatalogPlaybackTicketAdapter {
    #[must_use]
    pub const fn new(deps: CatalogDeps) -> Self {
        Self { deps }
    }
}

impl CatalogPlaybackTicketPort for CatalogPlaybackTicketAdapter {
    fn fetch_playback_ticket(
        &self,
        _key: EpisodeKey,
    ) -> PortFuture<'_, Result<PlaybackTicket, PortError>> {
        let err = if self.deps.flags.playback_and_cache {
            PortError::InvalidState {
                detail: format!("{FEATURE_NAME} 切片尚未实现"),
            }
        } else {
            PortError::InvalidState {
                detail: format!("{FEATURE_NAME} 特性开关未开启"),
            }
        };
        Box::pin(async move { Err(err) })
    }
}
