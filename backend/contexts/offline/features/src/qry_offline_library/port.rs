use hg_backend_contracts::error::{PortError, PortFuture};
use hg_backend_contracts::offline::{CommittedOfflineAsset, OfflineAssetQueryPort};
use hg_contracts::ids::EpisodeKey;

use crate::OfflineDeps;

use super::query::FEATURE_NAME;

#[derive(Clone)]
pub struct OfflineAssetQueryAdapter {
    deps: OfflineDeps,
}

impl OfflineAssetQueryAdapter {
    #[must_use]
    pub const fn new(deps: OfflineDeps) -> Self {
        Self { deps }
    }

    fn disabled_or_stub(&self) -> PortError {
        if self.deps.flags.library_and_outbox {
            PortError::InvalidState {
                detail: format!("{FEATURE_NAME} 切片尚未实现"),
            }
        } else {
            PortError::InvalidState {
                detail: format!("{FEATURE_NAME} 特性开关未开启"),
            }
        }
    }
}

impl OfflineAssetQueryPort for OfflineAssetQueryAdapter {
    fn find_committed_episode(
        &self,
        _key: EpisodeKey,
    ) -> PortFuture<'_, Result<Option<CommittedOfflineAsset>, PortError>> {
        let err = self.disabled_or_stub();
        Box::pin(async move { Err(err) })
    }

    fn total_offline_library_bytes(&self) -> PortFuture<'_, Result<u64, PortError>> {
        let err = self.disabled_or_stub();
        Box::pin(async move { Err(err) })
    }
}
