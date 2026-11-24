use std::fmt::Write as _;
use std::path::PathBuf;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL_SAFE_NO_PAD;
use bytes::Bytes;
use http_body_util::Full;
use hyper::header::{CACHE_CONTROL, CONTENT_TYPE};
use hyper::{Method, Response, StatusCode};
use sha2::{Digest as _, Sha256};

use crate::edge_crypto::{DEFAULT_USER_AGENT, decrypt_cover_bytes, resolve_cover_url};
use crate::storage::StorageLayout;

pub(super) fn build_cover_proxy_url(base_url: &str, raw_cover_url: &str) -> String {
    let trimmed = raw_cover_url.trim();
    if trimmed.is_empty() || trimmed.starts_with("file://") || trimmed.starts_with(base_url) {
        return trimmed.to_owned();
    }
    let resolved = resolve_cover_url(trimmed);
    let encoded = BASE64_URL_SAFE_NO_PAD.encode(resolved.as_bytes());
    format!("{base_url}/cover/{encoded}")
}

pub(super) async fn proxy_cover_request(
    http_client: &reqwest::Client,
    storage: &StorageLayout,
    method: &Method,
    encoded: &str,
) -> Response<Full<Bytes>> {
    let Some(remote_url) = decode_remote_url(encoded) else {
        return empty_status(StatusCode::BAD_REQUEST);
    };

    let cache_path = cover_cache_path(storage, &remote_url);
    if let Ok(cached) = tokio::fs::read(&cache_path).await
        && !cached.is_empty()
    {
        return image_response(method, cached);
    }

    let Ok(resp) = http_client
        .get(&remote_url)
        .header(reqwest::header::USER_AGENT, DEFAULT_USER_AGENT)
        .send()
        .await
    else {
        return empty_status(StatusCode::BAD_GATEWAY);
    };
    if !resp.status().is_success() {
        return empty_status(StatusCode::BAD_GATEWAY);
    }
    let Ok(raw_bytes) = resp.bytes().await else {
        return empty_status(StatusCode::BAD_GATEWAY);
    };
    let Ok(decrypted) = decrypt_cover_bytes(&raw_bytes) else {
        return empty_status(StatusCode::BAD_GATEWAY);
    };

    persist_cover_cache(&cache_path, &decrypted).await;
    image_response(method, decrypted)
}

fn decode_remote_url(encoded: &str) -> Option<String> {
    let raw = BASE64_URL_SAFE_NO_PAD.decode(encoded).ok()?;
    let url = String::from_utf8(raw).ok()?;
    if url.starts_with("http://") || url.starts_with("https://") {
        Some(url)
    } else {
        None
    }
}

fn cover_cache_path(storage: &StorageLayout, remote_url: &str) -> PathBuf {
    let mut hasher = Sha256::new();
    hasher.update(remote_url.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in digest {
        let _written = write!(&mut hex, "{byte:02x}");
    }
    storage
        .catalog_cache_dir()
        .join("covers")
        .join(format!("{hex}.bin"))
}

async fn persist_cover_cache(cache_path: &PathBuf, bytes: &[u8]) {
    let Some(parent) = cache_path.parent() else {
        return;
    };
    if tokio::fs::create_dir_all(parent).await.is_err() {
        return;
    }
    let tmp_path = cache_path.with_extension("tmp");
    if tokio::fs::write(&tmp_path, bytes).await.is_ok() {
        drop(tokio::fs::rename(&tmp_path, cache_path).await);
    }
}

fn detect_image_content_type(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "image/jpeg"
    } else if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        "image/png"
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        "image/webp"
    } else {
        "application/octet-stream"
    }
}

fn image_response(method: &Method, bytes: Vec<u8>) -> Response<Full<Bytes>> {
    let content_type = detect_image_content_type(&bytes);
    let body = if *method == Method::HEAD {
        Bytes::new()
    } else {
        Bytes::from(bytes)
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, content_type)
        .header(CACHE_CONTROL, "public, max-age=86400")
        .body(Full::new(body))
        .unwrap_or_else(|_| empty_status(StatusCode::INTERNAL_SERVER_ERROR))
}

fn empty_status(status: StatusCode) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .body(Full::new(Bytes::new()))
        .unwrap_or_else(|_| Response::new(Full::new(Bytes::new())))
}
