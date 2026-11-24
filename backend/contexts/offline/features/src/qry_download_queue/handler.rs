use hg_contracts::error::ContractError;
use hg_contracts::offline::{DownloadPolicy, DownloadTaskView};

use crate::OfflineDeps;

use super::query::{ensure_enabled, fetch_download_policy, fetch_download_tasks};

pub async fn download_tasks(deps: &OfflineDeps) -> Result<Vec<DownloadTaskView>, ContractError> {
    ensure_enabled(deps.flags.download_queue)?;
    fetch_download_tasks(deps.pool()).await
}

pub async fn download_policy(deps: &OfflineDeps) -> Result<DownloadPolicy, ContractError> {
    ensure_enabled(deps.flags.download_queue)?;
    fetch_download_policy(deps.pool()).await
}
