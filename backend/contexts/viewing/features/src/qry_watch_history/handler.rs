use hg_contracts::error::ContractError;
use hg_contracts::viewing::WatchHistoryEntry;

use crate::ViewingDeps;

use super::query::fetch_watch_history;

pub async fn execute(deps: &ViewingDeps) -> Result<Vec<WatchHistoryEntry>, ContractError> {
    fetch_watch_history(deps.db.pool()).await
}
