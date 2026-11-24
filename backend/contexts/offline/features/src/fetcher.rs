use std::time::Duration;

use hg_backend_contracts::catalog::CdnAuthHeaders;
use hg_backend_contracts::error::{PortError, PortFuture};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, REFERER, USER_AGENT};

pub trait OfflineSegmentFetcher: Send + Sync {
    fn fetch_bytes<'a>(
        &'a self,
        url: &'a str,
        cdn: &'a CdnAuthHeaders,
    ) -> PortFuture<'a, Result<Vec<u8>, PortError>>;
}

#[derive(Clone)]
pub struct HttpSegmentFetcher {
    client: reqwest::Client,
}

impl HttpSegmentFetcher {
    pub fn new() -> Result<Self, PortError> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|err| PortError::NetworkUnavailable {
                detail: format!("创建分片下载客户端失败: {err}"),
            })?;
        Ok(Self { client })
    }
}

impl OfflineSegmentFetcher for HttpSegmentFetcher {
    fn fetch_bytes<'a>(
        &'a self,
        url: &'a str,
        cdn: &'a CdnAuthHeaders,
    ) -> PortFuture<'a, Result<Vec<u8>, PortError>> {
        Box::pin(async move {
            let headers = build_cdn_headers(cdn)?;
            let resp = self
                .client
                .get(url)
                .headers(headers)
                .send()
                .await
                .map_err(|err| PortError::NetworkUnavailable {
                    detail: format!("下载分片失败: {err}"),
                })?;
            let status = resp.status();
            if !status.is_success() {
                return Err(PortError::UpstreamRejected {
                    detail: format!("下载分片 {url} 返回状态码 {status}"),
                });
            }
            resp.bytes()
                .await
                .map(|b| b.to_vec())
                .map_err(|err| PortError::NetworkUnavailable {
                    detail: format!("读取分片响应失败: {err}"),
                })
        })
    }
}

fn build_cdn_headers(cdn: &CdnAuthHeaders) -> Result<HeaderMap, PortError> {
    let mut headers = HeaderMap::new();
    if !cdn.referer.is_empty()
        && let Ok(val) = HeaderValue::from_str(&cdn.referer)
    {
        headers.insert(REFERER, val);
    }
    if !cdn.user_agent.is_empty()
        && let Ok(val) = HeaderValue::from_str(&cdn.user_agent)
    {
        headers.insert(USER_AGENT, val);
    }
    if let (Some(name), Some(value)) = (&cdn.auth_header_name, &cdn.auth_header_value) {
        let header_name =
            HeaderName::from_bytes(name.as_bytes()).map_err(|err| PortError::InvalidState {
                detail: format!("无效的 CDN 鉴权请求头 {name}: {err}"),
            })?;
        let header_value = HeaderValue::from_str(value).map_err(|err| PortError::InvalidState {
            detail: format!("无效的 CDN 鉴权头值: {err}"),
        })?;
        headers.insert(header_name, header_value);
    }
    if let Some(app) = &cdn.cdn_app
        && let Ok(val) = HeaderValue::from_str(app)
    {
        headers.insert(HeaderName::from_static("x-cdn-app"), val);
    }
    Ok(headers)
}
