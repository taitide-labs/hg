use hg_backend_contracts::catalog::{CatalogPlaybackTicketPort, PlaybackTicket};
use hg_backend_contracts::error::{PortError, PortFuture};
use hg_contracts::ids::EpisodeKey;

use crate::CatalogDeps;

use super::command::ensure_port_enabled;
use super::handler;

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
        key: EpisodeKey,
    ) -> PortFuture<'_, Result<PlaybackTicket, PortError>> {
        Box::pin(async move {
            ensure_port_enabled(self.deps.flags.playback_and_cache)?;
            handler::fetch_playback_ticket(&self.deps, key).await
        })
    }
}
