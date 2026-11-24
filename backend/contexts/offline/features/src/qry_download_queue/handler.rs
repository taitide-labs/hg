use hg_contracts::error::ContractError;
use hg_contracts::offline::{DownloadPolicy, DownloadTaskView};

use crate::OfflineDeps;

use super::query::{fetch_download_policy, fetch_download_tasks};

pub async fn download_tasks(deps: &OfflineDeps) -> Result<Vec<DownloadTaskView>, ContractError> {
    fetch_download_tasks(deps.pool()).await
}

pub async fn download_policy(deps: &OfflineDeps) -> Result<DownloadPolicy, ContractError> {
    fetch_download_policy(deps.pool()).await
}
