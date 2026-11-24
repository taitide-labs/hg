use hg_contracts::error::ContractError;
use hg_contracts::offline::OfflineSeriesView;

use crate::OfflineDeps;

use super::query::fetch_offline_library;

pub async fn offline_library(deps: &OfflineDeps) -> Result<Vec<OfflineSeriesView>, ContractError> {
    fetch_offline_library(deps.pool(), &deps.storage).await
}
