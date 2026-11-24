use hg_contracts::error::ContractError;
use hg_contracts::ids::{EpisodeKey, SeriesId};
use hg_contracts::viewing::PlaybackSource;

use crate::ViewingDeps;

use super::command::{FEATURE_NAME, ensure_enabled};

fn unimplemented_stub() -> ContractError {
    ContractError::Internal {
        detail: format!("{FEATURE_NAME} 切片尚未实现"),
    }
}

pub async fn prepare_episode_playback(
    deps: &ViewingDeps,
    _key: EpisodeKey,
) -> Result<PlaybackSource, ContractError> {
    ensure_enabled(deps.flags.playback_resolution)?;
    tokio::task::yield_now().await;
    Err(unimplemented_stub())
}

pub async fn refresh_online_playback(
    deps: &ViewingDeps,
    _key: EpisodeKey,
) -> Result<PlaybackSource, ContractError> {
    ensure_enabled(deps.flags.playback_resolution)?;
    Err(unimplemented_stub())
}

pub async fn stop_deleted_offline_episode(
    deps: &ViewingDeps,
    _key: EpisodeKey,
) -> Result<(), ContractError> {
    ensure_enabled(deps.flags.playback_resolution)?;
    Err(unimplemented_stub())
}

pub async fn stop_deleted_offline_series(
    deps: &ViewingDeps,
    _series_id: SeriesId,
) -> Result<(), ContractError> {
    ensure_enabled(deps.flags.playback_resolution)?;
    Err(unimplemented_stub())
}
