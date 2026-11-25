use hg_contracts::error::ContractError;
use hg_contracts::ids::SeriesId;
use hg_contracts::viewing::{WatchHistoryEntry, WatchProgress};

use crate::ViewingDeps;

use super::query::{fetch_episode_progress, fetch_watch_history};

pub async fn execute(deps: &ViewingDeps) -> Result<Vec<WatchHistoryEntry>, ContractError> {
    fetch_watch_history(deps.db.pool()).await
}

pub async fn episode_progress(
    deps: &ViewingDeps,
    series_id: SeriesId,
) -> Result<Vec<WatchProgress>, ContractError> {
    fetch_episode_progress(deps.db.pool(), series_id).await
}
