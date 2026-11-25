use std::collections::HashMap;
use std::fmt::{self, Write as _};
use std::io::{Cursor, ErrorKind};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{GenericImageView as _, ImageFormat};
use parking_lot::Mutex;
use sha2::{Digest as _, Sha256};
use sqlx::FromRow;

use crate::clock::Clock;
use crate::db::LocalDatabase;
use crate::error::InfraError;
use crate::storage::StorageLayout;

pub const DEFAULT_COVER_CACHE_LIMIT_BYTES: u64 = 150 * 1024 * 1024;
const COVER_CACHE_SUBDIR: &str = "covers";
const MAX_COVER_EDGE_PX: u32 = 600;
const COVER_JPEG_QUALITY: u8 = 82;
const ACCESS_TOUCH_INTERVAL_SECS: i64 = 3600;
const EVICT_CHECK_BYTES_THRESHOLD: u64 = 4 * 1024 * 1024;
const TOUCHED_MEMORY_CACHE_MAX: usize = 2048;

#[derive(Clone)]
pub struct CoverCacheStore {
    db: LocalDatabase,
    storage: Arc<StorageLayout>,
    clock: Arc<dyn Clock>,
    limit_override_bytes: Arc<AtomicU64>,
    pending_evict_bytes: Arc<AtomicU64>,
    touched_at: Arc<Mutex<HashMap<String, i64>>>,
}

impl fmt::Debug for CoverCacheStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CoverCacheStore")
            .field("limit_override_bytes", &self.limit_override_bytes)
            .finish()
    }
}

#[derive(FromRow)]
struct CachedCoverRow {
    cover_url: String,
    local_rel_path: String,
    size_bytes: i64,
}

impl CoverCacheStore {
    #[must_use]
    pub fn new(db: LocalDatabase, storage: Arc<StorageLayout>, clock: Arc<dyn Clock>) -> Self {
        Self {
            db,
            storage,
            clock,
            limit_override_bytes: Arc::new(AtomicU64::new(0)),
            pending_evict_bytes: Arc::new(AtomicU64::new(0)),
            touched_at: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn set_limit_override(&self, limit_bytes: Option<u64>) {
        self.limit_override_bytes
            .store(limit_bytes.unwrap_or(0), Ordering::SeqCst);
    }

    pub async fn read(&self, cover_url: &str) -> Result<Option<Vec<u8>>, InfraError> {
        let rel_path = build_cover_rel_path(cover_url);
        let Ok(clean_rel) = sanitize_relative_path(&rel_path) else {
            return Ok(None);
        };
        let full_path = self.storage.catalog_cache_dir().join(clean_rel);

        let bytes = match tokio::fs::read(&full_path).await {
            Ok(bytes) if !bytes.is_empty() => bytes,
            Ok(_) => return Ok(None),
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(InfraError::Io(err)),
        };

        let now = i64::try_from(self.clock.now_epoch_secs()).unwrap_or(i64::MAX);
        if self.should_touch_db(cover_url, now) {
            drop(
                sqlx::query(
                    "UPDATE catalog_cover_cache
                     SET accessed_at_epoch_secs = ?1
                     WHERE cover_url = ?2",
                )
                .bind(now)
                .bind(cover_url)
                .execute(self.db.pool())
                .await,
            );
        }
        Ok(Some(bytes))
    }

    fn should_touch_db(&self, cover_url: &str, now: i64) -> bool {
        let mut guard = self.touched_at.lock();
        if let Some(&prev) = guard.get(cover_url)
            && now.saturating_sub(prev) < ACCESS_TOUCH_INTERVAL_SECS
        {
            return false;
        }
        if guard.len() >= TOUCHED_MEMORY_CACHE_MAX {
            guard.clear();
        }
        guard.insert(cover_url.to_owned(), now);
        true
    }

    pub async fn store_decrypted(
        &self,
        cover_url: &str,
        decrypted: &[u8],
    ) -> Result<Vec<u8>, InfraError> {
        let raw = decrypted.to_vec();
        let optimized = tokio::task::spawn_blocking(move || optimize_cover_bytes(&raw))
            .await
            .unwrap_or_else(|_| decrypted.to_vec());

        let rel_path = build_cover_rel_path(cover_url);
        self.write_cover_file(&rel_path, &optimized).await?;
        let now = i64::try_from(self.clock.now_epoch_secs()).unwrap_or(i64::MAX);
        {
            let mut guard = self.touched_at.lock();
            if guard.len() >= TOUCHED_MEMORY_CACHE_MAX {
                guard.clear();
            }
            guard.insert(cover_url.to_owned(), now);
        }
        self.upsert_cover_index(cover_url, &rel_path, optimized.len())
            .await?;

        let has_override = self.limit_override_bytes.load(Ordering::Relaxed) > 0;
        let written = u64::try_from(optimized.len()).unwrap_or(0);
        let accumulated = self
            .pending_evict_bytes
            .fetch_add(written, Ordering::Relaxed)
            .saturating_add(written);
        if has_override || accumulated >= EVICT_CHECK_BYTES_THRESHOLD {
            self.pending_evict_bytes.store(0, Ordering::Relaxed);
            self.evict_lru_over_limit(cover_url).await;
        }
        Ok(optimized)
    }

    async fn write_cover_file(&self, rel_path: &str, bytes: &[u8]) -> Result<(), InfraError> {
        let clean_rel = sanitize_relative_path(rel_path)?;
        let target = self.storage.catalog_cache_dir().join(clean_rel);
        if let Some(parent) = target.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let tmp_path = target.with_extension("tmp");
        tokio::fs::write(&tmp_path, bytes).await?;
        tokio::fs::rename(&tmp_path, &target).await?;
        Ok(())
    }

    async fn upsert_cover_index(
        &self,
        cover_url: &str,
        rel_path: &str,
        size_bytes: usize,
    ) -> Result<(), InfraError> {
        let now = i64::try_from(self.clock.now_epoch_secs()).unwrap_or(i64::MAX);
        let size = i64::try_from(size_bytes).unwrap_or(i64::MAX);

        sqlx::query(
            "INSERT INTO catalog_cover_cache (
                cover_url,
                local_rel_path,
                size_bytes,
                accessed_at_epoch_secs
            ) VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(cover_url) DO UPDATE SET
                local_rel_path = excluded.local_rel_path,
                size_bytes = excluded.size_bytes,
                accessed_at_epoch_secs = excluded.accessed_at_epoch_secs",
        )
        .bind(cover_url)
        .bind(rel_path)
        .bind(size)
        .bind(now)
        .execute(self.db.pool())
        .await?;
        Ok(())
    }

    async fn evict_lru_over_limit(&self, protected_url: &str) {
        let limit_bytes = self.resolve_limit_bytes();
        let Ok(raw_total) = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(SUM(size_bytes), 0) FROM catalog_cover_cache",
        )
        .fetch_one(self.db.pool())
        .await
        else {
            return;
        };

        let mut total_bytes = u64::try_from(raw_total).unwrap_or(0);
        if total_bytes <= limit_bytes {
            return;
        }

        let cache_dir = self.storage.catalog_cache_dir();
        loop {
            if total_bytes <= limit_bytes {
                break;
            }
            let Ok(rows) = sqlx::query_as::<_, CachedCoverRow>(
                "SELECT cover_url, local_rel_path, size_bytes
                 FROM catalog_cover_cache
                 WHERE cover_url != ?1
                 ORDER BY accessed_at_epoch_secs ASC, cover_url ASC
                 LIMIT 32",
            )
            .bind(protected_url)
            .fetch_all(self.db.pool())
            .await
            else {
                break;
            };
            if rows.is_empty() {
                break;
            }
            for row in rows {
                if total_bytes <= limit_bytes {
                    break;
                }
                let size = u64::try_from(row.size_bytes).unwrap_or(0);
                total_bytes = total_bytes.saturating_sub(size);
                drop(
                    sqlx::query("DELETE FROM catalog_cover_cache WHERE cover_url = ?1")
                        .bind(&row.cover_url)
                        .execute(self.db.pool())
                        .await,
                );
                if let Ok(clean_rel) = sanitize_relative_path(&row.local_rel_path) {
                    drop(tokio::fs::remove_file(cache_dir.join(clean_rel)).await);
                }
            }
        }
    }

    fn resolve_limit_bytes(&self) -> u64 {
        let override_bytes = self.limit_override_bytes.load(Ordering::SeqCst);
        if override_bytes > 0 {
            override_bytes
        } else {
            DEFAULT_COVER_CACHE_LIMIT_BYTES
        }
    }
}

#[must_use]
pub fn build_cover_rel_path(cover_url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(cover_url.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in digest {
        let _written = write!(&mut hex, "{byte:02x}");
    }
    format!("{COVER_CACHE_SUBDIR}/{hex}.bin")
}

#[must_use]
pub fn optimize_cover_bytes(decrypted: &[u8]) -> Vec<u8> {
    let Ok(decoded) = image::load_from_memory(decrypted) else {
        return decrypted.to_vec();
    };
    let (width, height) = decoded.dimensions();
    let resized = width > MAX_COVER_EDGE_PX || height > MAX_COVER_EDGE_PX;
    let processed = if resized {
        decoded.resize(MAX_COVER_EDGE_PX, MAX_COVER_EDGE_PX, FilterType::Triangle)
    } else {
        decoded
    };

    let mut out = Vec::new();
    let encoded_ok = if processed.color().has_alpha() {
        processed
            .write_to(&mut Cursor::new(&mut out), ImageFormat::WebP)
            .is_ok()
    } else {
        let rgb = processed.to_rgb8();
        let encoder = JpegEncoder::new_with_quality(&mut out, COVER_JPEG_QUALITY);
        rgb.write_with_encoder(encoder).is_ok()
    };

    if !encoded_ok || out.is_empty() || (!resized && out.len() >= decrypted.len()) {
        return decrypted.to_vec();
    }
    out
}

fn sanitize_relative_path(raw: &str) -> Result<PathBuf, InfraError> {
    if raw.is_empty() || raw.contains('\\') {
        return Err(InfraError::InvalidPath {
            detail: format!("非法的封面缓存相对路径: {raw}"),
        });
    }

    let mut clean = PathBuf::new();
    for component in Path::new(raw).components() {
        match component {
            Component::Normal(part) => clean.push(part),
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(InfraError::InvalidPath {
                    detail: format!("封面缓存路径包含非法遍历段: {raw}"),
                });
            }
        }
    }

    if clean.as_os_str().is_empty() {
        return Err(InfraError::InvalidPath {
            detail: "封面缓存相对路径不能为空".to_owned(),
        });
    }
    Ok(clean)
}
