use hg_contracts::error::ContractError;
use hg_contracts::ids::SeriesId;
use hg_contracts::viewing::{PlaybackPreference, SeriesBookmark, StorageOverview};

use crate::ViewingDeps;

use super::query::{FEATURE_NAME, ensure_enabled};

fn unimplemented_stub() -> ContractError {
    ContractError::Internal {
        detail: format!("{FEATURE_NAME} 切片尚未实现"),
    }
}

pub async fn bookmarks(deps: &ViewingDeps) -> Result<Vec<SeriesBookmark>, ContractError> {
    ensure_enabled(deps.flags.watchlist_and_settings)?;
    Err(unimplemented_stub())
}

pub async fn is_bookmarked(
    deps: &ViewingDeps,
    _series_id: SeriesId,
) -> Result<bool, ContractError> {
    ensure_enabled(deps.flags.watchlist_and_settings)?;
    Err(unimplemented_stub())
}

pub async fn playback_preference(deps: &ViewingDeps) -> Result<PlaybackPreference, ContractError> {
    ensure_enabled(deps.flags.watchlist_and_settings)?;
    Err(unimplemented_stub())
}

pub async fn storage_overview(deps: &ViewingDeps) -> Result<StorageOverview, ContractError> {
    ensure_enabled(deps.flags.watchlist_and_settings)?;
    Err(unimplemented_stub())
}
