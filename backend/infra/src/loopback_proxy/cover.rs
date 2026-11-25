use std::collections::HashSet;
use std::sync::Arc;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL_SAFE_NO_PAD;
use bytes::Bytes;
use http_body_util::Full;
use hyper::header::{CACHE_CONTROL, CONTENT_TYPE};
use hyper::{Method, Response, StatusCode};
use parking_lot::Mutex;
use tokio::sync::Semaphore;

use crate::cover_cache::{CoverCacheStore, build_cover_rel_path, optimize_cover_bytes};
use crate::edge_crypto::{DEFAULT_USER_AGENT, decrypt_cover_bytes, resolve_cover_url};
use crate::storage::StorageLayout;

pub(super) struct CoverProxyContext<'a> {
    pub(super) http_client: &'a reqwest::Client,
    pub(super) storage: &'a Arc<StorageLayout>,
    pub(super) cover_cache: Option<&'a CoverCacheStore>,
    pub(super) store_limiter: &'a Arc<Semaphore>,
    pub(super) in_flight: &'a Arc<Mutex<HashSet<String>>>,
}

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
    ctx: CoverProxyContext<'_>,
    method: &Method,
    encoded: &str,
) -> Response<Full<Bytes>> {
    let Some(remote_url) = decode_remote_url(encoded) else {
        return empty_status(StatusCode::BAD_REQUEST);
    };

    if let Some(cache) = ctx.cover_cache {
        if let Ok(Some(cached)) = cache.read(&remote_url).await {
            return image_response(method, cached);
        }
    } else {
        let fallback_path = ctx
            .storage
            .catalog_cache_dir()
            .join(build_cover_rel_path(&remote_url));
        if let Ok(cached) = tokio::fs::read(&fallback_path).await
            && !cached.is_empty()
        {
            return image_response(method, cached);
        }
    }

    let Ok(resp) = ctx
        .http_client
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

    spawn_background_cover_store(&ctx, remote_url, decrypted.clone());
    image_response(method, decrypted)
}

fn spawn_background_cover_store(
    ctx: &CoverProxyContext<'_>,
    remote_url: String,
    decrypted: Vec<u8>,
) {
    if !ctx.in_flight.lock().insert(remote_url.clone()) {
        return;
    }
    let cover_cache = ctx.cover_cache.cloned();
    let storage = Arc::clone(ctx.storage);
    let limiter = Arc::clone(ctx.store_limiter);
    let in_flight = Arc::clone(ctx.in_flight);

    tokio::spawn(async move {
        let _permit = limiter.acquire().await.ok();
        if let Some(cache) = cover_cache {
            drop(cache.store_decrypted(&remote_url, &decrypted).await);
        } else {
            let raw = decrypted.clone();
            let optimized = tokio::task::spawn_blocking(move || optimize_cover_bytes(&raw))
                .await
                .unwrap_or(decrypted);
            let fallback_path = storage
                .catalog_cache_dir()
                .join(build_cover_rel_path(&remote_url));
            if let Some(parent) = fallback_path.parent() {
                drop(tokio::fs::create_dir_all(parent).await);
            }
            drop(tokio::fs::write(&fallback_path, &optimized).await);
        }
        in_flight.lock().remove(&remote_url);
    });
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
        .header(CACHE_CONTROL, "public, max-age=604800")
        .body(Full::new(body))
        .unwrap_or_else(|_| empty_status(StatusCode::INTERNAL_SERVER_ERROR))
}

fn empty_status(status: StatusCode) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .body(Full::new(Bytes::new()))
        .unwrap_or_else(|_| Response::new(Full::new(Bytes::new())))
}
