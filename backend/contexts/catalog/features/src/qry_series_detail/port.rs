use hg_backend_contracts::catalog::{CatalogMetadataPort, EpisodeSnapshot, SeriesSnapshot};
use hg_backend_contracts::error::{PortError, PortFuture};
use hg_contracts::ids::{EpisodeKey, SeriesId};

use crate::CatalogDeps;

use super::query::FEATURE_NAME;

#[derive(Clone)]
pub struct CatalogMetadataAdapter {
    deps: CatalogDeps,
}

impl CatalogMetadataAdapter {
    #[must_use]
    pub const fn new(deps: CatalogDeps) -> Self {
        Self { deps }
    }

    fn disabled_or_stub(&self) -> PortError {
        if self.deps.flags.detail_and_comments {
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

impl CatalogMetadataPort for CatalogMetadataAdapter {
    fn fetch_series_snapshot(
        &self,
        _series_id: SeriesId,
    ) -> PortFuture<'_, Result<SeriesSnapshot, PortError>> {
        let err = self.disabled_or_stub();
        Box::pin(async move { Err(err) })
    }

    fn fetch_episode_snapshot(
        &self,
        _key: EpisodeKey,
    ) -> PortFuture<'_, Result<EpisodeSnapshot, PortError>> {
        let err = self.disabled_or_stub();
        Box::pin(async move { Err(err) })
    }

    fn fetch_decrypted_cover<'a>(
        &'a self,
        _cover_url: &'a str,
    ) -> PortFuture<'a, Result<Vec<u8>, PortError>> {
        let err = self.disabled_or_stub();
        Box::pin(async move { Err(err) })
    }
}
