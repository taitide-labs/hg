use hg_contracts::error::ContractError;
use hg_contracts::ids::SeriesId;
use hg_contracts::viewing::{
    DislikedSeriesEntry, PlaybackPreference, SearchHistoryEntry, SeriesBookmark, StorageOverview,
};
use hg_infra::storage::StorageLayout;

use crate::ViewingDeps;

use super::query::{
    fetch_bookmarks, fetch_disliked_series, fetch_is_bookmarked, fetch_is_disliked,
    fetch_playback_preference, fetch_search_history,
};

pub async fn bookmarks(deps: &ViewingDeps) -> Result<Vec<SeriesBookmark>, ContractError> {
    fetch_bookmarks(deps.db.pool()).await
}

pub async fn is_bookmarked(deps: &ViewingDeps, series_id: SeriesId) -> Result<bool, ContractError> {
    fetch_is_bookmarked(deps.db.pool(), series_id).await
}

pub async fn disliked_series(
    deps: &ViewingDeps,
) -> Result<Vec<DislikedSeriesEntry>, ContractError> {
    fetch_disliked_series(deps.db.pool()).await
}

pub async fn is_disliked(deps: &ViewingDeps, series_id: SeriesId) -> Result<bool, ContractError> {
    fetch_is_disliked(deps.db.pool(), series_id).await
}

pub async fn playback_preference(deps: &ViewingDeps) -> Result<PlaybackPreference, ContractError> {
    fetch_playback_preference(deps.db.pool()).await
}

pub async fn search_history(deps: &ViewingDeps) -> Result<Vec<SearchHistoryEntry>, ContractError> {
    fetch_search_history(deps.db.pool()).await
}

pub async fn storage_overview(deps: &ViewingDeps) -> Result<StorageOverview, ContractError> {
    let offline_library_bytes = deps
        .offline_assets
        .total_offline_library_bytes()
        .await
        .map_err(ContractError::from)?;
    let playback_cache_bytes = StorageLayout::dir_size_bytes(deps.storage.playback_cache_dir())
        .await
        .map_err(|err| ContractError::Internal {
            detail: format!("统计播放缓存占用失败: {err}"),
        })?;
    let catalog_cache_bytes = StorageLayout::dir_size_bytes(deps.storage.catalog_cache_dir())
        .await
        .map_err(|err| ContractError::Internal {
            detail: format!("统计目录缓存占用失败: {err}"),
        })?;

    Ok(StorageOverview {
        offline_library_bytes,
        playback_cache_bytes,
        catalog_cache_bytes,
    })
}
