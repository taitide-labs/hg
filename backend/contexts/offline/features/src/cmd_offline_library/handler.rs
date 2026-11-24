use hg_contracts::error::ContractError;
use hg_contracts::ids::{EpisodeKey, SeriesId};

use crate::OfflineDeps;

use super::command::{FEATURE_NAME, ensure_enabled};

fn unimplemented_stub() -> ContractError {
    ContractError::Internal {
        detail: format!("{FEATURE_NAME} 切片尚未实现"),
    }
}

pub async fn delete_offline_episode(
    deps: &OfflineDeps,
    _key: EpisodeKey,
) -> Result<(), ContractError> {
    ensure_enabled(deps.flags.library_and_outbox)?;
    Err(unimplemented_stub())
}

pub async fn delete_offline_series(
    deps: &OfflineDeps,
    _series_id: SeriesId,
) -> Result<(), ContractError> {
    ensure_enabled(deps.flags.library_and_outbox)?;
    Err(unimplemented_stub())
}
