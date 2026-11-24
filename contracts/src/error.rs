use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractError {
    NotFound { resource: String },
    InvalidInput { message: String },
    NetworkUnavailable { detail: String },
    UpstreamRejected { detail: String },
    PreviewOnlyUnavailableForDownload,
    StorageFull,
    Internal { detail: String },
}

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { resource } => write!(f, "资源不存在: {resource}"),
            Self::InvalidInput { message } => write!(f, "无效参数: {message}"),
            Self::NetworkUnavailable { detail } => write!(f, "网络不可用: {detail}"),
            Self::UpstreamRejected { detail } => write!(f, "上游服务拒绝请求: {detail}"),
            Self::PreviewOnlyUnavailableForDownload => {
                write!(f, "当前分集仅有预览流，不支持离线下载")
            }
            Self::StorageFull => write!(f, "设备存储空间不足"),
            Self::Internal { detail } => write!(f, "内部错误: {detail}"),
        }
    }
}

impl std::error::Error for ContractError {}
