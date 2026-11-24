use hg_backend_contracts::catalog::{CatalogMetadataPort, EpisodeSnapshot, SeriesSnapshot};
use hg_backend_contracts::error::{PortError, PortFuture};
use hg_contracts::ids::{EpisodeKey, SeriesId};

use crate::CatalogDeps;

use super::query::{fetch_decrypted_cover, fetch_series_detail};

#[derive(Clone)]
pub struct CatalogMetadataAdapter {
    deps: CatalogDeps,
}

impl CatalogMetadataAdapter {
    #[must_use]
    pub const fn new(deps: CatalogDeps) -> Self {
        Self { deps }
    }
}

impl CatalogMetadataPort for CatalogMetadataAdapter {
    fn fetch_series_snapshot(
        &self,
        series_id: SeriesId,
    ) -> PortFuture<'_, Result<SeriesSnapshot, PortError>> {
        Box::pin(async move {
            let detail = fetch_series_detail(&self.deps, series_id).await?;
            Ok(SeriesSnapshot {
                series_id: detail.summary.id,
                title: detail.summary.title,
                cover_url: detail.summary.cover_url,
                total_episodes: detail.total_episodes,
                is_finished: detail.summary.is_finished,
                score: detail.summary.score,
            })
        })
    }

    fn fetch_episode_snapshot(
        &self,
        key: EpisodeKey,
    ) -> PortFuture<'_, Result<EpisodeSnapshot, PortError>> {
        Box::pin(async move {
            let detail = fetch_series_detail(&self.deps, key.series_id).await?;
            let episode = detail
                .episodes
                .into_iter()
                .find(|ep| ep.ep_num == key.ep_num)
                .ok_or_else(|| PortError::NotFound {
                    resource: format!("series/{}/episodes/{}", key.series_id, key.ep_num),
                })?;
            Ok(EpisodeSnapshot {
                key,
                title: episode.title,
                duration_secs: episode.duration_secs,
                playable: episode.playable,
            })
        })
    }

    fn fetch_decrypted_cover<'a>(
        &'a self,
        cover_url: &'a str,
    ) -> PortFuture<'a, Result<Vec<u8>, PortError>> {
        Box::pin(async move { fetch_decrypted_cover(&self.deps, cover_url).await })
    }
}
