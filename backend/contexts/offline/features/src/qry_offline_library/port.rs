use hg_backend_contracts::error::{PortError, PortFuture};
use hg_backend_contracts::offline::{CommittedOfflineAsset, OfflineAssetQueryPort};
use hg_contracts::ids::EpisodeKey;

use crate::OfflineDeps;

use super::query::{FEATURE_NAME, find_committed_episode, total_offline_library_bytes};

#[derive(Clone)]
pub struct OfflineAssetQueryAdapter {
    deps: OfflineDeps,
}

impl OfflineAssetQueryAdapter {
    #[must_use]
    pub const fn new(deps: OfflineDeps) -> Self {
        Self { deps }
    }

    fn ensure_port_enabled(&self) -> Result<(), PortError> {
        if self.deps.flags.library_and_outbox {
            Ok(())
        } else {
            Err(PortError::InvalidState {
                detail: format!("{FEATURE_NAME} 特性开关未开启"),
            })
        }
    }
}

impl OfflineAssetQueryPort for OfflineAssetQueryAdapter {
    fn find_committed_episode(
        &self,
        key: EpisodeKey,
    ) -> PortFuture<'_, Result<Option<CommittedOfflineAsset>, PortError>> {
        Box::pin(async move {
            self.ensure_port_enabled()?;
            find_committed_episode(self.deps.pool(), &self.deps.storage, key).await
        })
    }

    fn total_offline_library_bytes(&self) -> PortFuture<'_, Result<u64, PortError>> {
        Box::pin(async move {
            self.ensure_port_enabled()?;
            total_offline_library_bytes(self.deps.pool()).await
        })
    }
}
