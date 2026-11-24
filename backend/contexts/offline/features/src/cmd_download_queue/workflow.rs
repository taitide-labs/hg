use hg_contracts::error::ContractError;
use hg_contracts::offline::DownloadPolicy;
use hg_offline_domain::policy::validate_download_policy;

pub fn check_download_policy(policy: &DownloadPolicy) -> Result<(), ContractError> {
    validate_download_policy(policy).map_err(ContractError::from)
}
