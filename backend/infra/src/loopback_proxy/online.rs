use std::fmt::{self, Write as _};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use bytes::Bytes;
use http_body_util::Full;
use hyper::header::CONTENT_TYPE;
use hyper::{Method, Response, StatusCode};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use sha2::{Digest as _, Sha256};
use sqlx::FromRow;
use url::Url;

use crate::clock::Clock;
use crate::db::LocalDatabase;
use crate::error::InfraError;
use crate::storage::StorageLayout;

const DEFAULT_CACHE_LIMIT_BYTES: u64 = 512 * 1024 * 1024;
const BYTES_PER_MB: u64 = 1024 * 1024;

#[derive(Clone)]
pub(super) struct PlaybackCacheStore {
    db: LocalDatabase,
    storage: Arc<StorageLayout>,
    clock: Arc<dyn Clock>,
    limit_override_bytes: Arc<AtomicU64>,
}

impl fmt::Debug for PlaybackCacheStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlaybackCacheStore")
            .field("limit_override_bytes", &self.limit_override_bytes)
            .finish()
    }
}

#[derive(FromRow)]
struct CachedSegmentRow {
    segment_key: String,
    local_rel_path: String,
    size_bytes: i64,
}

impl PlaybackCacheStore {
    pub(super) fn new(
        db: LocalDatabase,
        storage: Arc<StorageLayout>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            db,
            storage,
            clock,
            limit_override_bytes: Arc::new(AtomicU64::new(0)),
        }
    }

    pub(super) fn set_limit_override(&self, limit_bytes: Option<u64>) {
        self.limit_override_bytes
            .store(limit_bytes.unwrap_or(0), Ordering::SeqCst);
    }

    async fn read_cached_segment(&self, key: &str) -> Option<Bytes> {
        let rel_path = format!("{key}.seg");
        let full_path = self.storage.playback_cache_dir().join(&rel_path);
        let bytes = tokio::fs::read(&full_path).await.ok()?;
        if bytes.is_empty() {
            return None;
        }
        let now = i64::try_from(self.clock.now_epoch_secs()).unwrap_or(i64::MAX);
        drop(
            sqlx::query(
                "UPDATE viewing_playback_cache_segments
                 SET accessed_at_epoch_secs = ?2
                 WHERE segment_key = ?1",
            )
            .bind(key)
            .bind(now)
            .execute(self.db.pool())
            .await,
        );
        Some(Bytes::from(bytes))
    }

    async fn write_and_evict_segment(&self, key: &str, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        let rel_path = format!("{key}.seg");
        let dir = self.storage.playback_cache_dir();
        if tokio::fs::create_dir_all(&dir).await.is_err() {
            return;
        }
        let full_path = dir.join(&rel_path);
        let tmp_path = full_path.with_extension("tmp");
        if tokio::fs::write(&tmp_path, bytes).await.is_err() {
            return;
        }
        if tokio::fs::rename(&tmp_path, &full_path).await.is_err() {
            drop(tokio::fs::remove_file(&tmp_path).await);
            return;
        }

        let now = i64::try_from(self.clock.now_epoch_secs()).unwrap_or(i64::MAX);
        let size = i64::try_from(bytes.len()).unwrap_or(i64::MAX);
        drop(
            sqlx::query(
                "INSERT INTO viewing_playback_cache_segments (
                    segment_key, local_rel_path, size_bytes, accessed_at_epoch_secs
                ) VALUES (?1, ?2, ?3, ?4)
                ON CONFLICT(segment_key) DO UPDATE SET
                    local_rel_path = excluded.local_rel_path,
                    size_bytes = excluded.size_bytes,
                    accessed_at_epoch_secs = excluded.accessed_at_epoch_secs",
            )
            .bind(key)
            .bind(&rel_path)
            .bind(size)
            .bind(now)
            .execute(self.db.pool())
            .await,
        );

        self.evict_lru_over_limit(key).await;
    }

    async fn resolve_limit_bytes(&self) -> u64 {
        let override_bytes = self.limit_override_bytes.load(Ordering::SeqCst);
        if override_bytes > 0 {
            return override_bytes;
        }
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT playback_cache_limit_mb FROM viewing_playback_preference WHERE id = 1",
        )
        .fetch_optional(self.db.pool())
        .await
        .ok()
        .flatten();
        row.and_then(|(mb,)| u64::try_from(mb).ok())
            .map_or(DEFAULT_CACHE_LIMIT_BYTES, |mb| {
                mb.saturating_mul(BYTES_PER_MB)
            })
    }

    async fn evict_lru_over_limit(&self, protected_key: &str) {
        let limit_bytes = self.resolve_limit_bytes().await;
        let Ok(rows) = sqlx::query_as::<_, CachedSegmentRow>(
            "SELECT segment_key, local_rel_path, size_bytes
             FROM viewing_playback_cache_segments
             ORDER BY accessed_at_epoch_secs ASC, segment_key ASC",
        )
        .fetch_all(self.db.pool())
        .await
        else {
            return;
        };

        let mut total_bytes = rows
            .iter()
            .map(|r| u64::try_from(r.size_bytes).unwrap_or(0))
            .fold(0_u64, u64::saturating_add);
        if total_bytes <= limit_bytes {
            return;
        }

        let cache_dir = self.storage.playback_cache_dir();
        for row in rows {
            if total_bytes <= limit_bytes {
                break;
            }
            if row.segment_key == protected_key {
                continue;
            }
            let size = u64::try_from(row.size_bytes).unwrap_or(0);
            total_bytes = total_bytes.saturating_sub(size);
            drop(
                sqlx::query("DELETE FROM viewing_playback_cache_segments WHERE segment_key = ?1")
                    .bind(&row.segment_key)
                    .execute(self.db.pool())
                    .await,
            );
            drop(tokio::fs::remove_file(cache_dir.join(&row.local_rel_path)).await);
        }
    }
}

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
    cache: Option<&PlaybackCacheStore>,
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

    let url_looks_like_playlist = is_hls_playlist(&target_url, None);
    let cache_key = (!url_looks_like_playlist && *method == Method::GET)
        .then(|| segment_cache_key(&target_url));

    if let (Some(store), Some(key)) = (cache, cache_key.as_deref())
        && let Some(cached) = store.read_cached_segment(key).await
    {
        return ok_response(content_type_for_url(&target_url), cached);
    }

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
        upstream_ct.unwrap_or_else(|| content_type_for_url(&target_url).to_owned())
    };

    if *method == Method::HEAD {
        return ok_response(&content_type, Bytes::new());
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
        if let (Some(store), Some(key)) = (cache, cache_key.as_deref()) {
            store.write_and_evict_segment(key, &raw_bytes).await;
        }
        raw_bytes
    };

    ok_response(&content_type, body)
}

fn segment_cache_key(url: &Url) -> String {
    let mut hasher = Sha256::new();
    if let Some(host) = url.host_str() {
        hasher.update(host.as_bytes());
    }
    hasher.update(url.path().as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in digest {
        let _written = write!(&mut hex, "{byte:02x}");
    }
    hex
}

fn content_type_for_url(url: &Url) -> &'static str {
    if url.path().ends_with(".ts") {
        "video/mp2t"
    } else if url.path().ends_with(".m4s") || url.path().ends_with(".mp4") {
        "video/mp4"
    } else {
        "application/octet-stream"
    }
}

fn ok_response(content_type: &str, body: Bytes) -> Response<Full<Bytes>> {
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
