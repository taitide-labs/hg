use hg_contracts::error::ContractError;

use crate::CatalogDeps;

use super::command::{FEATURE_NAME, ensure_enabled};

pub async fn clear_catalog_cache(deps: &CatalogDeps) -> Result<(), ContractError> {
    ensure_enabled(deps.flags.playback_and_cache)?;
    tokio::task::yield_now().await;
    Err(ContractError::Internal {
        detail: format!("{FEATURE_NAME} 切片尚未实现"),
    })
}
