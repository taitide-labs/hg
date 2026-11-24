use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use bytes::Bytes;
use http_body_util::Full;
use hyper::header::CONTENT_TYPE;
use hyper::{Method, Response, StatusCode};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use url::Url;

use crate::error::InfraError;

#[derive(Debug, Clone)]
pub(super) struct OnlineStreamSession {
    pub(super) manifest_url: Url,
    pub(super) headers: HeaderMap,
}

pub(super) fn build_online_session(
    stream_url: &str,
    raw_headers: &[(String, String)],
) -> Result<OnlineStreamSession, InfraError> {
    let manifest_url = Url::parse(stream_url).map_err(|err| InfraError::InvalidPath {
        detail: format!("在线播放地址非法: {err}"),
    })?;
    if !matches!(manifest_url.scheme(), "http" | "https") {
        return Err(InfraError::InvalidPath {
            detail: format!("在线播放地址协议不受支持: {}", manifest_url.scheme()),
        });
    }

    let mut headers = HeaderMap::new();
    for (name, value) in raw_headers {
        let Ok(header_name) = HeaderName::from_bytes(name.trim().as_bytes()) else {
            continue;
        };
        let Ok(header_value) = HeaderValue::from_str(value.trim()) else {
            continue;
        };
        headers.insert(header_name, header_value);
    }

    Ok(OnlineStreamSession {
        manifest_url,
        headers,
    })
}

pub(super) async fn proxy_online_request(
    client: &reqwest::Client,
    method: &Method,
    token: &str,
    subpath: &str,
    query: Option<&str>,
    session: &OnlineStreamSession,
) -> Response<Full<Bytes>> {
    let target_url = match subpath {
        "manifest.m3u8" => session.manifest_url.clone(),
        "asset" => match decode_asset_url(query) {
            Some(url) => url,
            None => return empty_response(StatusCode::BAD_REQUEST),
        },
        _ => return empty_response(StatusCode::NOT_FOUND),
    };

    let req_builder = if *method == Method::HEAD {
        client.head(target_url.clone())
    } else {
        client.get(target_url.clone())
    };
    let upstream_resp = match req_builder.headers(session.headers.clone()).send().await {
        Ok(resp) => resp,
        Err(_) => return empty_response(StatusCode::BAD_GATEWAY),
    };

    let status =
        StatusCode::from_u16(upstream_resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let upstream_ct = upstream_resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(ToOwned::to_owned);

    if !status.is_success() {
        return empty_response(status);
    }

    let is_playlist = is_hls_playlist(&target_url, upstream_ct.as_deref());
    let content_type = if is_playlist {
        "application/vnd.apple.mpegurl".to_owned()
    } else {
        upstream_ct.unwrap_or_else(|| "application/octet-stream".to_owned())
    };

    if *method == Method::HEAD {
        return Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, content_type)
            .body(Full::new(Bytes::new()))
            .unwrap_or_else(|_| empty_response(StatusCode::INTERNAL_SERVER_ERROR));
    }

    let Ok(raw_bytes) = upstream_resp.bytes().await else {
        return empty_response(StatusCode::BAD_GATEWAY);
    };
    let body = if is_playlist {
        match std::str::from_utf8(&raw_bytes) {
            Ok(text) => Bytes::from(rewrite_hls_playlist(text, &target_url, token)),
            Err(_) => raw_bytes,
        }
    } else {
        raw_bytes
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, content_type)
        .body(Full::new(body))
        .unwrap_or_else(|_| empty_response(StatusCode::INTERNAL_SERVER_ERROR))
}

pub(super) fn rewrite_hls_playlist(manifest: &str, base_url: &Url, token: &str) -> String {
    let mut out = String::with_capacity(manifest.len().saturating_add(128));
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            out.push('\n');
            continue;
        }
        if trimmed.starts_with('#') {
            out.push_str(&rewrite_tag_uri(line, base_url, token));
            out.push('\n');
            continue;
        }
        if let Some(proxied) = encode_proxied_uri(trimmed, base_url, token) {
            out.push_str(&proxied);
        } else {
            out.push_str(trimmed);
        }
        out.push('\n');
    }
    out
}

fn rewrite_tag_uri(line: &str, base_url: &Url, token: &str) -> String {
    let Some((prefix, rest)) = line.split_once("URI=\"") else {
        return line.to_owned();
    };
    let Some((raw_uri, suffix)) = rest.split_once('"') else {
        return line.to_owned();
    };
    let Some(proxied) = encode_proxied_uri(raw_uri, base_url, token) else {
        return line.to_owned();
    };
    format!("{prefix}URI=\"{proxied}\"{suffix}")
}

fn encode_proxied_uri(raw: &str, base_url: &Url, token: &str) -> Option<String> {
    let resolved = base_url.join(raw).ok()?;
    if !matches!(resolved.scheme(), "http" | "https") {
        return None;
    }
    let encoded = URL_SAFE_NO_PAD.encode(resolved.as_str().as_bytes());
    Some(format!("/online/{token}/asset?u={encoded}"))
}

fn decode_asset_url(query: Option<&str>) -> Option<Url> {
    let query_str = query?;
    let encoded = query_str
        .split('&')
        .find_map(|pair| pair.strip_prefix("u="))?;
    let bytes = URL_SAFE_NO_PAD.decode(encoded).ok()?;
    let raw = String::from_utf8(bytes).ok()?;
    let parsed = Url::parse(&raw).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }
    Some(parsed)
}

fn is_hls_playlist(url: &Url, content_type: Option<&str>) -> bool {
    if url.path().ends_with(".m3u8") {
        return true;
    }
    content_type.is_some_and(|ct| {
        let lower = ct.to_ascii_lowercase();
        lower.contains("mpegurl") || lower.contains("vnd.apple.mpegurl")
    })
}

fn empty_response(status: StatusCode) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .body(Full::new(Bytes::new()))
        .unwrap_or_else(|_| Response::new(Full::new(Bytes::new())))
}
