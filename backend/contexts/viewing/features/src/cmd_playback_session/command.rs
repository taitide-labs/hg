use hg_contracts::error::ContractError;

pub const FEATURE_NAME: &str = "viewing.playback_resolution";

pub fn ensure_enabled(enabled: bool) -> Result<(), ContractError> {
    if enabled {
        Ok(())
    } else {
        Err(ContractError::Internal {
            detail: format!("{FEATURE_NAME} 特性开关未开启"),
        })
    }
}
