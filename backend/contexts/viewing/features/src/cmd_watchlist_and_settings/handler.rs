use hg_contracts::error::ContractError;
use hg_contracts::ids::SeriesId;
use hg_contracts::viewing::PlaybackPreference;

use crate::ViewingDeps;

use super::command::{FEATURE_NAME, ensure_enabled};
use super::workflow::check_playback_preference;

fn unimplemented_stub() -> ContractError {
    ContractError::Internal {
        detail: format!("{FEATURE_NAME} 切片尚未实现"),
    }
}

pub async fn remove_watch_history(
    deps: &ViewingDeps,
    _series_id: SeriesId,
) -> Result<(), ContractError> {
    ensure_enabled(deps.flags.watchlist_and_settings)?;
    Err(unimplemented_stub())
}

pub async fn clear_watch_history(deps: &ViewingDeps) -> Result<(), ContractError> {
    ensure_enabled(deps.flags.watchlist_and_settings)?;
    Err(unimplemented_stub())
}

pub async fn bookmark_series(
    deps: &ViewingDeps,
    _series_id: SeriesId,
) -> Result<(), ContractError> {
    ensure_enabled(deps.flags.watchlist_and_settings)?;
    Err(unimplemented_stub())
}

pub async fn unbookmark_series(
    deps: &ViewingDeps,
    _series_id: SeriesId,
) -> Result<(), ContractError> {
    ensure_enabled(deps.flags.watchlist_and_settings)?;
    Err(unimplemented_stub())
}

pub async fn update_playback_preference(
    deps: &ViewingDeps,
    preference: PlaybackPreference,
) -> Result<(), ContractError> {
    ensure_enabled(deps.flags.watchlist_and_settings)?;
    check_playback_preference(&preference)?;
    Err(unimplemented_stub())
}

pub async fn clear_playback_cache(deps: &ViewingDeps) -> Result<(), ContractError> {
    ensure_enabled(deps.flags.watchlist_and_settings)?;
    Err(unimplemented_stub())
}
