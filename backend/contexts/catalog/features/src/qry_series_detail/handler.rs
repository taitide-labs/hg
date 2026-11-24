use hg_contracts::catalog::{
    CommentReply, EpisodeComment, ListCommentRepliesRequest, ListEpisodeCommentsRequest,
    PagedResult, SeriesDetail,
};
use hg_contracts::error::ContractError;
use hg_contracts::ids::SeriesId;

use crate::CatalogDeps;

use super::query::{fetch_comment_replies, fetch_episode_comments, fetch_series_detail};

pub async fn series_detail(
    deps: &CatalogDeps,
    series_id: SeriesId,
) -> Result<SeriesDetail, ContractError> {
    fetch_series_detail(deps, series_id)
        .await
        .map_err(ContractError::from)
}

pub async fn list_episode_comments(
    deps: &CatalogDeps,
    request: ListEpisodeCommentsRequest,
) -> Result<PagedResult<EpisodeComment>, ContractError> {
    fetch_episode_comments(deps, request)
        .await
        .map_err(ContractError::from)
}

pub async fn list_comment_replies(
    deps: &CatalogDeps,
    request: ListCommentRepliesRequest,
) -> Result<PagedResult<CommentReply>, ContractError> {
    fetch_comment_replies(deps, request)
        .await
        .map_err(ContractError::from)
}
