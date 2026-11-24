use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CatalogError {
    #[error("查询参数无效: {message}")]
    InvalidQuery { message: String },
    #[error("播放凭据未返回任何可用流地址")]
    EmptyStreamList,
    #[error("上游响应格式无效: {detail}")]
    UpstreamPayloadMalformed { detail: String },
}
