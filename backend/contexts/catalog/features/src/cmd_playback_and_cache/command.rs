use hg_backend_contracts::error::PortError;
use hg_contracts::error::ContractError;

pub const FEATURE_NAME: &str = "catalog.playback_and_cache";

pub fn ensure_enabled(enabled: bool) -> Result<(), ContractError> {
    if enabled {
        Ok(())
    } else {
        Err(ContractError::Internal {
            detail: format!("{FEATURE_NAME} 特性开关未开启"),
        })
    }
}

pub fn ensure_port_enabled(enabled: bool) -> Result<(), PortError> {
    if enabled {
        Ok(())
    } else {
        Err(PortError::InvalidState {
            detail: format!("{FEATURE_NAME} 特性开关未开启"),
        })
    }
}
