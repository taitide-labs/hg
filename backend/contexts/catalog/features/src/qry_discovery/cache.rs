use hg_contracts::error::ContractError;
use serde::Serialize;
use serde::de::DeserializeOwned;
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, FromRow)]
struct CacheEntryRow {
    payload_json: String,
    expires_at_epoch_secs: i64,
}

pub async fn read_cached_json<T: DeserializeOwned>(
    pool: &SqlitePool,
    cache_key: &str,
    now_epoch_secs: u64,
) -> Result<Option<T>, ContractError> {
    let row = sqlx::query_as::<_, CacheEntryRow>(
        "SELECT payload_json, expires_at_epoch_secs
         FROM catalog_cache_entries
         WHERE cache_key = ?1",
    )
    .bind(cache_key)
    .fetch_optional(pool)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("读取目录缓存失败: {err}"),
    })?;

    let Some(entry) = row else {
        return Ok(None);
    };

    let now_i64 = i64::try_from(now_epoch_secs).map_err(|err| ContractError::Internal {
        detail: format!("当前时间戳超出 SQLite 整数范围: {err}"),
    })?;
    if entry.expires_at_epoch_secs <= now_i64 {
        return Ok(None);
    }

    Ok(serde_json::from_str::<T>(&entry.payload_json).ok())
}

pub async fn write_cached_json<T: Serialize>(
    pool: &SqlitePool,
    cache_key: &str,
    value: &T,
    now_epoch_secs: u64,
    ttl_secs: u64,
) -> Result<(), ContractError> {
    let payload_json = serde_json::to_string(value).map_err(|err| ContractError::Internal {
        detail: format!("序列化目录缓存失败: {err}"),
    })?;
    let size_bytes = i64::try_from(payload_json.len()).map_err(|err| ContractError::Internal {
        detail: format!("目录缓存字节数超出 SQLite 整数范围: {err}"),
    })?;
    let updated_at = i64::try_from(now_epoch_secs).map_err(|err| ContractError::Internal {
        detail: format!("更新时间戳超出 SQLite 整数范围: {err}"),
    })?;
    let expires_at = i64::try_from(now_epoch_secs.saturating_add(ttl_secs)).map_err(|err| {
        ContractError::Internal {
            detail: format!("过期时间戳超出 SQLite 整数范围: {err}"),
        }
    })?;

    sqlx::query("DELETE FROM catalog_cache_entries WHERE expires_at_epoch_secs <= ?1")
        .bind(updated_at)
        .execute(pool)
        .await
        .map_err(|err| ContractError::Internal {
            detail: format!("清理过期目录缓存失败: {err}"),
        })?;

    sqlx::query(
        "INSERT INTO catalog_cache_entries (
            cache_key,
            payload_json,
            size_bytes,
            expires_at_epoch_secs,
            updated_at_epoch_secs
        ) VALUES (?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(cache_key) DO UPDATE SET
            payload_json = excluded.payload_json,
            size_bytes = excluded.size_bytes,
            expires_at_epoch_secs = excluded.expires_at_epoch_secs,
            updated_at_epoch_secs = excluded.updated_at_epoch_secs",
    )
    .bind(cache_key)
    .bind(&payload_json)
    .bind(size_bytes)
    .bind(expires_at)
    .bind(updated_at)
    .execute(pool)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("写入目录缓存失败: {err}"),
    })?;

    Ok(())
}
