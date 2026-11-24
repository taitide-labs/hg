use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use hg_backend_contracts::error::PortError;
use hg_infra::edge_crypto::decrypt_cover_bytes;

use crate::CatalogDeps;

const COVER_CACHE_SUBDIR: &str = "covers";

pub async fn fetch_or_cache_cover(
    deps: &CatalogDeps,
    cover_url: &str,
) -> Result<Vec<u8>, PortError> {
    let trimmed_url = cover_url.trim();
    if trimmed_url.is_empty() {
        return Err(PortError::InvalidState {
            detail: "封面地址不能为空".to_owned(),
        });
    }

    let now =
        i64::try_from(deps.clock.now_epoch_secs()).map_err(|err| PortError::InvalidState {
            detail: format!("当前时间戳超出 SQLite 整数范围: {err}"),
        })?;

    if let Some(cached) = try_read_cached_cover(deps, trimmed_url, now).await? {
        return Ok(cached);
    }

    let raw_bytes = deps.upstream.fetch_raw_cover(trimmed_url).await?;
    let decrypted = decrypt_cover_bytes(&raw_bytes).map_err(PortError::from)?;
    let rel_path = build_cover_rel_path(trimmed_url);
    write_cover_file(deps.storage.catalog_cache_dir(), &rel_path, &decrypted).await?;
    upsert_cover_index(deps, trimmed_url, &rel_path, decrypted.len(), now).await?;

    Ok(decrypted)
}

async fn try_read_cached_cover(
    deps: &CatalogDeps,
    cover_url: &str,
    now_epoch_secs: i64,
) -> Result<Option<Vec<u8>>, PortError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT local_rel_path FROM catalog_cover_cache WHERE cover_url = ?1")
            .bind(cover_url)
            .fetch_optional(deps.pool())
            .await
            .map_err(|err| PortError::StorageFailure {
                detail: format!("查询封面缓存索引失败: {err}"),
            })?;

    let Some((rel_path,)) = row else {
        return Ok(None);
    };

    let Ok(clean_rel) = sanitize_relative_path(&rel_path) else {
        return Ok(None);
    };
    let full_path = deps.storage.catalog_cache_dir().join(clean_rel);

    match tokio::fs::read(&full_path).await {
        Ok(bytes) if !bytes.is_empty() => {
            touch_cover_accessed_at(deps, cover_url, now_epoch_secs).await?;
            Ok(Some(bytes))
        }
        Ok(_) => Ok(None),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(err) => Err(PortError::StorageFailure {
            detail: format!("读取本地封面缓存失败: {err}"),
        }),
    }
}

async fn touch_cover_accessed_at(
    deps: &CatalogDeps,
    cover_url: &str,
    now_epoch_secs: i64,
) -> Result<(), PortError> {
    sqlx::query("UPDATE catalog_cover_cache SET accessed_at_epoch_secs = ?1 WHERE cover_url = ?2")
        .bind(now_epoch_secs)
        .bind(cover_url)
        .execute(deps.pool())
        .await
        .map_err(|err| PortError::StorageFailure {
            detail: format!("更新封面缓存访问时间失败: {err}"),
        })?;
    Ok(())
}

async fn write_cover_file(cache_dir: &Path, rel_path: &str, bytes: &[u8]) -> Result<(), PortError> {
    let clean_rel = sanitize_relative_path(rel_path)?;
    let target = cache_dir.join(clean_rel);
    if let Some(parent) = target.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|err| PortError::StorageFailure {
                detail: format!("创建封面缓存目录失败: {err}"),
            })?;
    }

    let tmp_path = target.with_extension("tmp");
    tokio::fs::write(&tmp_path, bytes)
        .await
        .map_err(|err| PortError::StorageFailure {
            detail: format!("写入临时封面缓存失败: {err}"),
        })?;
    tokio::fs::rename(&tmp_path, &target)
        .await
        .map_err(|err| PortError::StorageFailure {
            detail: format!("提交封面缓存文件失败: {err}"),
        })?;
    Ok(())
}

async fn upsert_cover_index(
    deps: &CatalogDeps,
    cover_url: &str,
    rel_path: &str,
    size_bytes: usize,
    now_epoch_secs: i64,
) -> Result<(), PortError> {
    let size = i64::try_from(size_bytes).map_err(|err| PortError::StorageFailure {
        detail: format!("封面字节大小超出 SQLite 整数范围: {err}"),
    })?;

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
    .bind(now_epoch_secs)
    .execute(deps.pool())
    .await
    .map_err(|err| PortError::StorageFailure {
        detail: format!("登记封面缓存索引失败: {err}"),
    })?;
    Ok(())
}

pub fn build_cover_rel_path(cover_url: &str) -> String {
    let digest = fnv1a_128(cover_url.as_bytes());
    format!("{COVER_CACHE_SUBDIR}/{digest:032x}.bin")
}

fn fnv1a_128(bytes: &[u8]) -> u128 {
    const OFFSET_BASIS: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
    const PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013b;

    let mut hash = OFFSET_BASIS;
    for &byte in bytes {
        hash ^= u128::from(byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

fn sanitize_relative_path(raw: &str) -> Result<PathBuf, PortError> {
    if raw.is_empty() || raw.contains('\\') {
        return Err(PortError::StorageFailure {
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
                return Err(PortError::StorageFailure {
                    detail: format!("封面缓存路径包含非法遍历段: {raw}"),
                });
            }
        }
    }

    if clean.as_os_str().is_empty() {
        return Err(PortError::StorageFailure {
            detail: "封面缓存相对路径不能为空".to_owned(),
        });
    }
    Ok(clean)
}
