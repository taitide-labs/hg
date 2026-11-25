use hg_backend_contracts::error::PortError;
use hg_contracts::catalog::{
    CommentReply, EpisodeComment, ListCommentRepliesRequest, ListEpisodeCommentsRequest,
    PagedResult, SeriesDetail,
};
use hg_contracts::ids::SeriesId;
use hg_infra::edge_crypto::EdgeChannelScope;

use crate::CatalogDeps;

mod cover_cache;
mod parser;

#[cfg(test)]
mod tests;

pub const FEATURE_NAME: &str = "catalog.detail_and_comments";
const SERIES_DETAIL_CACHE_TTL_SECS: u64 = 300;

pub async fn fetch_series_detail(
    deps: &CatalogDeps,
    series_id: SeriesId,
) -> Result<SeriesDetail, PortError> {
    let cache_key = format!("series_detail:v2:{series_id}");
    let now = deps.clock.now_epoch_secs();

    if let Some(cached) = deps.cache.read_json::<SeriesDetail>(&cache_key, now).await {
        return Ok(cached);
    }

    let path = format!("/api/videos/{series_id}");
    let payload = deps
        .upstream
        .request_json(EdgeChannelScope::Dynamic, "GET", &path, None)
        .await?;
    let detail = parser::parse_series_detail(series_id, &payload)?;
    deps.cache
        .write_json(&cache_key, &detail, now, SERIES_DETAIL_CACHE_TTL_SECS)
        .await
        .map_err(|err| PortError::StorageFailure {
            detail: format!("序列化剧目详情缓存失败: {err}"),
        })?;
    Ok(detail)
}

pub async fn fetch_episode_comments(
    deps: &CatalogDeps,
    request: ListEpisodeCommentsRequest,
) -> Result<PagedResult<EpisodeComment>, PortError> {
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
    cover_cache::fetch_or_cache_cover(deps, cover_url).await
}
