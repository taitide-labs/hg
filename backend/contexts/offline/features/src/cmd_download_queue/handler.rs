use hg_contracts::error::ContractError;
use hg_contracts::ids::EpisodeKey;
use hg_contracts::offline::{
    ControlAllDownloadsAction, ControlDownloadRequest, DownloadPolicy,
    EnqueueEpisodeDownloadsRequest, EnqueueEpisodeDownloadsResult,
};

use crate::OfflineDeps;

use super::command::{FEATURE_NAME, ensure_enabled};
use super::workflow::check_download_policy;

fn unimplemented_stub() -> ContractError {
    ContractError::Internal {
        detail: format!("{FEATURE_NAME} 切片尚未实现"),
    }
}

pub async fn enqueue_episode_downloads(
    deps: &OfflineDeps,
    _request: EnqueueEpisodeDownloadsRequest,
) -> Result<EnqueueEpisodeDownloadsResult, ContractError> {
    ensure_enabled(deps.flags.download_queue)?;
    Err(unimplemented_stub())
}

pub async fn control_download(
    deps: &OfflineDeps,
    _request: ControlDownloadRequest,
) -> Result<(), ContractError> {
    ensure_enabled(deps.flags.download_queue)?;
    Err(unimplemented_stub())
}

pub async fn control_all_downloads(
    deps: &OfflineDeps,
    _action: ControlAllDownloadsAction,
) -> Result<(), ContractError> {
    ensure_enabled(deps.flags.download_queue)?;
    Err(unimplemented_stub())
}

pub async fn update_download_policy(
    deps: &OfflineDeps,
    policy: DownloadPolicy,
) -> Result<(), ContractError> {
    ensure_enabled(deps.flags.download_queue)?;
    check_download_policy(&policy)?;
    Err(unimplemented_stub())
}

pub async fn execute_next_queued_task(
    deps: &OfflineDeps,
) -> Result<Option<EpisodeKey>, ContractError> {
    ensure_enabled(deps.flags.download_queue)?;
    tokio::task::yield_now().await;
    Err(unimplemented_stub())
}
