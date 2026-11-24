use hg_backend_contracts::error::PortError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InfraError {
    #[error("数据库操作失败: {0}")]
    Database(#[from] sqlx::Error),
    #[error("数据库迁移失败: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("本地文件读写失败: {0}")]
    Io(#[from] std::io::Error),
    #[error("上游网络请求失败: {0}")]
    Http(#[from] reqwest::Error),
    #[error("加解密失败: {detail}")]
    Crypto { detail: String },
    #[error("非法存储路径: {detail}")]
    InvalidPath { detail: String },
    #[error("上游协议异常: {detail}")]
    Protocol { detail: String },
    #[error("本地回环代理异常: {detail}")]
    Loopback { detail: String },
}

impl From<InfraError> for PortError {
    fn from(value: InfraError) -> Self {
        match value {
            InfraError::Database(err) => Self::StorageFailure {
                detail: err.to_string(),
            },
            InfraError::Migration(err) => Self::StorageFailure {
                detail: err.to_string(),
            },
            InfraError::Io(err) => Self::StorageFailure {
                detail: err.to_string(),
            },
            InfraError::Http(err) => Self::NetworkUnavailable {
                detail: err.to_string(),
            },
            InfraError::Crypto { detail } | InfraError::Protocol { detail } => {
                Self::UpstreamRejected { detail }
            }
            InfraError::InvalidPath { detail } | InfraError::Loopback { detail } => {
                Self::InvalidState { detail }
            }
        }
    }
}
