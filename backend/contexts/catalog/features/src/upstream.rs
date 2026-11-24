use hg_backend_contracts::error::{PortError, PortFuture};
use hg_infra::edge_crypto::{EdgeChannelScope, EdgeHttpClient};
use serde_json::Value;

#[cfg(test)]
mod tests;

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

pub fn normalize_upstream_response(
    http_status: u16,
    body: Value,
    path_with_query: &str,
) -> Result<Value, PortError> {
    if http_status >= 400 {
        return Err(map_http_error(http_status, &body, path_with_query));
    }

    if let Some(biz_status) = body.get("status").and_then(Value::as_i64)
        && biz_status != 1
    {
        let message = extract_error_message(&body)
            .unwrap_or_else(|| format!("上游业务拒绝，状态码 {biz_status}"));
        if is_not_found_message(&message) {
            return Err(PortError::NotFound {
                resource: format!("{path_with_query}: {message}"),
            });
        }
        return Err(PortError::UpstreamRejected { detail: message });
    }

    Ok(body)
}

fn map_http_error(http_status: u16, body: &Value, path_with_query: &str) -> PortError {
    let effective_status = body
        .get("status")
        .and_then(Value::as_u64)
        .and_then(|s| u16::try_from(s).ok())
        .filter(|&s| s >= 400)
        .unwrap_or(http_status);

    let detail = extract_error_message(body)
        .unwrap_or_else(|| format!("上游接口 {path_with_query} 返回状态码 {effective_status}"));

    if effective_status == 404 || is_not_found_message(&detail) {
        PortError::NotFound {
            resource: format!("{path_with_query}: {detail}"),
        }
    } else {
        PortError::UpstreamRejected { detail }
    }
}

fn extract_error_message(body: &Value) -> Option<String> {
    for key in ["detail", "msg", "error", "title"] {
        if let Some(text) = body
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            return Some(text.to_owned());
        }
    }
    None
}

fn is_not_found_message(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    message.contains("不存在") || message.contains("未找到") || lower.contains("not found")
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
            normalize_upstream_response(response.status, response.body, path_with_query)
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
