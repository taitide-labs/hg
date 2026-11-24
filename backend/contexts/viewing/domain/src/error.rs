use hg_contracts::error::ContractError;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ViewingError {
    #[error("播放偏好配置无效: {message}")]
    InvalidPreference { message: String },
    #[error("当前分集没有可用播放流")]
    NoPlayableStream,
}

impl From<ViewingError> for ContractError {
    fn from(value: ViewingError) -> Self {
        match value {
            ViewingError::InvalidPreference { message } => Self::InvalidInput { message },
            ViewingError::NoPlayableStream => Self::NotFound {
                resource: "playable_stream".to_owned(),
            },
        }
    }
}
