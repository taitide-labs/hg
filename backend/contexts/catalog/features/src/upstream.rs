use hg_backend_contracts::error::{PortError, PortFuture};
use hg_infra::edge_crypto::{EdgeChannelScope, EdgeHttpClient};
use serde_json::Value;

pub trait CatalogUpstream: Send + Sync {
    fn request_json<'a>(
        &'a self,
        scope: EdgeChannelScope,
        method: &'a str,
        path_with_query: &'a str,
        body: Option<&'a Value>,
    ) -> PortFuture<'a, Result<Value, PortError>>;

    fn fetch_raw_cover<'a>(
        &'a self,
        cover_url: &'a str,
    ) -> PortFuture<'a, Result<Vec<u8>, PortError>>;
}

pub struct EdgeCatalogUpstream {
    client: EdgeHttpClient,
}

impl EdgeCatalogUpstream {
    #[must_use]
    pub const fn new(client: EdgeHttpClient) -> Self {
        Self { client }
    }
}

impl CatalogUpstream for EdgeCatalogUpstream {
    fn request_json<'a>(
        &'a self,
        scope: EdgeChannelScope,
        method: &'a str,
        path_with_query: &'a str,
        body: Option<&'a Value>,
    ) -> PortFuture<'a, Result<Value, PortError>> {
        Box::pin(async move {
            let response = self
                .client
                .request_json(scope, method, path_with_query, body)
                .await
                .map_err(PortError::from)?;
            if response.status >= 400 {
                return Err(PortError::UpstreamRejected {
                    detail: format!("上游接口 {path_with_query} 返回状态码 {}", response.status),
                });
            }
            Ok(response.body)
        })
    }

    fn fetch_raw_cover<'a>(
        &'a self,
        cover_url: &'a str,
    ) -> PortFuture<'a, Result<Vec<u8>, PortError>> {
        Box::pin(async move {
            let resolved = hg_infra::edge_crypto::resolve_cover_url(cover_url);
            self.client
                .fetch_raw_bytes(&resolved, None)
                .await
                .map_err(PortError::from)
        })
    }
}
