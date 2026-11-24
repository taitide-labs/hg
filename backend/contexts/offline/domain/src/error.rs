use hg_contracts::error::ContractError;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OfflineError {
    #[error("下载任务状态流转非法: {from} -> {action}")]
    InvalidTransition {
        from: &'static str,
        action: &'static str,
    },
    #[error("当前分集仅有预览流，禁止离线下载")]
    PreviewOnlyNotDownloadable,
    #[error("下载策略配置无效: {message}")]
    InvalidPolicy { message: String },
    #[error("下载分集列表不能为空")]
    EmptyEpisodeSelection,
}

impl From<OfflineError> for ContractError {
    fn from(value: OfflineError) -> Self {
        match value {
            OfflineError::PreviewOnlyNotDownloadable => Self::PreviewOnlyUnavailableForDownload,
            OfflineError::InvalidPolicy { message } => Self::InvalidInput { message },
            OfflineError::EmptyEpisodeSelection => Self::InvalidInput {
                message: "至少选择 1 个分集".to_owned(),
            },
            OfflineError::InvalidTransition { from, action } => Self::InvalidInput {
                message: format!("状态 {from} 不允许执行 {action}"),
            },
        }
    }
}
