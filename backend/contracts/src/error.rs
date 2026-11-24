use std::future::Future;
use std::pin::Pin;

use hg_contracts::error::ContractError;
use thiserror::Error;

pub type PortFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PortError {
    #[error("资源不存在: {resource}")]
    NotFound { resource: String },
    #[error("网络不可用: {detail}")]
    NetworkUnavailable { detail: String },
    #[error("上游拒绝请求: {detail}")]
    UpstreamRejected { detail: String },
    #[error("存储读写异常: {detail}")]
    StorageFailure { detail: String },
    #[error("非法状态: {detail}")]
    InvalidState { detail: String },
}

impl From<PortError> for ContractError {
    fn from(value: PortError) -> Self {
        match value {
            PortError::NotFound { resource } => Self::NotFound { resource },
            PortError::NetworkUnavailable { detail } => Self::NetworkUnavailable { detail },
            PortError::UpstreamRejected { detail } => Self::UpstreamRejected { detail },
            PortError::StorageFailure { detail } | PortError::InvalidState { detail } => {
                Self::Internal { detail }
            }
        }
    }
}
