use hg_contracts::error::ContractError;
use hg_contracts::offline::OfflineSeriesView;

use crate::OfflineDeps;

use super::query::{ensure_enabled, fetch_offline_library};

pub async fn offline_library(deps: &OfflineDeps) -> Result<Vec<OfflineSeriesView>, ContractError> {
    ensure_enabled(deps.flags.library_and_outbox)?;
    fetch_offline_library(deps.pool(), &deps.storage).await
}
