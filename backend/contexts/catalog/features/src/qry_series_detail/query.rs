use hg_backend_contracts::error::PortError;
use hg_contracts::catalog::{
    CommentReply, EpisodeComment, ListCommentRepliesRequest, ListEpisodeCommentsRequest,
    PagedResult, SeriesDetail,
};
use hg_contracts::error::ContractError;
use hg_contracts::ids::SeriesId;
use hg_infra::edge_crypto::EdgeChannelScope;

use crate::CatalogDeps;

mod cover_cache;
mod parser;

#[cfg(test)]
mod tests;

pub const FEATURE_NAME: &str = "catalog.detail_and_comments";
const SERIES_DETAIL_CACHE_TTL_SECS: u64 = 300;

pub fn ensure_enabled(enabled: bool) -> Result<(), ContractError> {
    if enabled {
        Ok(())
    } else {
        Err(ContractError::Internal {
            detail: format!("{FEATURE_NAME} 特性开关未开启"),
        })
    }
}

pub fn ensure_enabled_port(enabled: bool) -> Result<(), PortError> {
    if enabled {
        Ok(())
    } else {
        Err(PortError::InvalidState {
            detail: format!("{FEATURE_NAME} 特性开关未开启"),
        })
    }
}

pub async fn fetch_series_detail(
    deps: &CatalogDeps,
    series_id: SeriesId,
) -> Result<SeriesDetail, PortError> {
    ensure_enabled_port(deps.flags.detail_and_comments)?;

    let cache_key = format!("series_detail:{series_id}");
    let now =
        i64::try_from(deps.clock.now_epoch_secs()).map_err(|err| PortError::InvalidState {
            detail: format!("当前时间戳超出 SQLite 整数范围: {err}"),
        })?;

    if let Some(cached) = try_read_cached_series_detail(deps, &cache_key, now).await? {
        return Ok(cached);
    }

    let path = format!("/api/videos/{series_id}");
    let payload = deps
        .upstream
        .request_json(EdgeChannelScope::Dynamic, "GET", &path, None)
        .await?;
    let detail = parser::parse_series_detail(series_id, &payload)?;
    store_cached_series_detail(deps, &cache_key, &detail, now).await?;
    Ok(detail)
}

pub async fn fetch_episode_comments(
    deps: &CatalogDeps,
    request: ListEpisodeCommentsRequest,
) -> Result<PagedResult<EpisodeComment>, PortError> {
    ensure_enabled_port(deps.flags.detail_and_comments)?;

    let path = format!(
        "/api/videos/{}/comments?ep={}&page={}&size={}",
        request.series_id.get(),
        request.ep_num.get(),
        request.page.page.get(),
        request.page.size.get(),
    );
    let payload = deps
        .upstream
        .request_json(EdgeChannelScope::Dynamic, "GET", &path, None)
        .await?;
    parser::parse_episode_comments(&payload, request.page)
}

pub async fn fetch_comment_replies(
    deps: &CatalogDeps,
    request: ListCommentRepliesRequest,
) -> Result<PagedResult<CommentReply>, PortError> {
    ensure_enabled_port(deps.flags.detail_and_comments)?;

    let path = format!(
        "/api/videos/{}/comments/{}/replies?page={}&size={}",
        request.series_id.get(),
        request.comment_id.get(),
        request.page.page.get(),
        request.page.size.get(),
    );
    let payload = deps
        .upstream
        .request_json(EdgeChannelScope::Dynamic, "GET", &path, None)
        .await?;
    parser::parse_comment_replies(&payload, request.page)
}

pub async fn fetch_decrypted_cover(
    deps: &CatalogDeps,
    cover_url: &str,
) -> Result<Vec<u8>, PortError> {
    ensure_enabled_port(deps.flags.detail_and_comments)?;
    cover_cache::fetch_or_cache_cover(deps, cover_url).await
}

async fn try_read_cached_series_detail(
    deps: &CatalogDeps,
    cache_key: &str,
    now_epoch_secs: i64,
) -> Result<Option<SeriesDetail>, PortError> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT payload_json
         FROM catalog_cache_entries
         WHERE cache_key = ?1 AND expires_at_epoch_secs > ?2",
    )
    .bind(cache_key)
    .bind(now_epoch_secs)
    .fetch_optional(deps.pool())
    .await
    .map_err(|err| PortError::StorageFailure {
        detail: format!("读取剧目详情缓存失败: {err}"),
    })?;

    let Some((payload_json,)) = row else {
        return Ok(None);
    };

    Ok(serde_json::from_str::<SeriesDetail>(&payload_json).ok())
}

async fn store_cached_series_detail(
    deps: &CatalogDeps,
    cache_key: &str,
    detail: &SeriesDetail,
    now_epoch_secs: i64,
) -> Result<(), PortError> {
    let payload_json = serde_json::to_string(detail).map_err(|err| PortError::StorageFailure {
        detail: format!("序列化剧目详情缓存失败: {err}"),
    })?;
    let size_bytes =
        i64::try_from(payload_json.len()).map_err(|err| PortError::StorageFailure {
            detail: format!("剧目详情缓存体积超出范围: {err}"),
        })?;
    let ttl = i64::try_from(SERIES_DETAIL_CACHE_TTL_SECS).unwrap_or(300);
    let expires_at = now_epoch_secs.saturating_add(ttl);

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
    .bind(now_epoch_secs)
    .execute(deps.pool())
    .await
    .map_err(|err| PortError::StorageFailure {
        detail: format!("写入剧目详情缓存失败: {err}"),
    })?;
    Ok(())
}
