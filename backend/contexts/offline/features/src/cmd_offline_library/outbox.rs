use hg_backend_contracts::offline::OfflineDomainEvent;
use hg_contracts::error::ContractError;

use crate::OfflineDeps;

use super::command::{FEATURE_NAME, ensure_enabled};

pub fn decode_outbox_event(payload_json: &str) -> Result<OfflineDomainEvent, ContractError> {
    serde_json::from_str(payload_json).map_err(|err| ContractError::Internal {
        detail: format!("反序列化离线 Outbox 事件失败: {err}"),
    })
}

pub async fn dispatch_pending_outbox(deps: &OfflineDeps) -> Result<usize, ContractError> {
    ensure_enabled(deps.flags.library_and_outbox)?;
    Err(ContractError::Internal {
        detail: format!("{FEATURE_NAME} 切片尚未实现"),
    })
}
