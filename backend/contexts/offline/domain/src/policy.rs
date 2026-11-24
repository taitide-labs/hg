use hg_contracts::ids::StreamProfile;
use hg_contracts::offline::DownloadPolicy;

use crate::error::OfflineError;

pub const MIN_CONCURRENT_TASKS: u8 = 1;
pub const MAX_CONCURRENT_TASKS: u8 = 5;

#[must_use]
pub const fn default_download_policy() -> DownloadPolicy {
    DownloadPolicy {
        preferred_profile: StreamProfile::Hevc,
        wifi_only: true,
        max_concurrent_tasks: 2,
        storage_directory: None,
    }
}

pub fn validate_download_policy(policy: &DownloadPolicy) -> Result<(), OfflineError> {
    if !(MIN_CONCURRENT_TASKS..=MAX_CONCURRENT_TASKS).contains(&policy.max_concurrent_tasks) {
        return Err(OfflineError::InvalidPolicy {
            message: format!(
                "并发任务上限必须在 {MIN_CONCURRENT_TASKS}..={MAX_CONCURRENT_TASKS} 之间"
            ),
        });
    }
    if let Some(dir) = &policy.storage_directory
        && dir.trim().is_empty()
    {
        return Err(OfflineError::InvalidPolicy {
            message: "自定义存储目录不能为空白".to_owned(),
        });
    }
    Ok(())
}
