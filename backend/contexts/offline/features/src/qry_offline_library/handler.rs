use hg_contracts::error::ContractError;
use hg_contracts::offline::OfflineSeriesView;

use crate::OfflineDeps;

use super::query::{FEATURE_NAME, ensure_enabled};

pub async fn offline_library(deps: &OfflineDeps) -> Result<Vec<OfflineSeriesView>, ContractError> {
    ensure_enabled(deps.flags.library_and_outbox)?;
    Err(ContractError::Internal {
        detail: format!("{FEATURE_NAME} 切片尚未实现"),
    })
}
