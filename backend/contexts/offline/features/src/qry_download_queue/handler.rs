use hg_contracts::error::ContractError;
use hg_contracts::offline::{DownloadPolicy, DownloadTaskView};

use crate::OfflineDeps;

use super::query::{FEATURE_NAME, ensure_enabled};

fn unimplemented_stub() -> ContractError {
    ContractError::Internal {
        detail: format!("{FEATURE_NAME} 切片尚未实现"),
    }
}

pub async fn download_tasks(deps: &OfflineDeps) -> Result<Vec<DownloadTaskView>, ContractError> {
    ensure_enabled(deps.flags.download_queue)?;
    Err(unimplemented_stub())
}

pub async fn download_policy(deps: &OfflineDeps) -> Result<DownloadPolicy, ContractError> {
    ensure_enabled(deps.flags.download_queue)?;
    Err(unimplemented_stub())
}
