use hg_contracts::catalog::{
    CommentReply, EpisodeComment, ListCommentRepliesRequest, ListEpisodeCommentsRequest,
    PagedResult, SeriesDetail,
};
use hg_contracts::error::ContractError;
use hg_contracts::ids::SeriesId;

use crate::CatalogDeps;

use super::query::{FEATURE_NAME, ensure_enabled};

fn unimplemented_stub() -> ContractError {
    ContractError::Internal {
        detail: format!("{FEATURE_NAME} 切片尚未实现"),
    }
}

pub async fn series_detail(
    deps: &CatalogDeps,
    _series_id: SeriesId,
) -> Result<SeriesDetail, ContractError> {
    ensure_enabled(deps.flags.detail_and_comments)?;
    Err(unimplemented_stub())
}

pub async fn list_episode_comments(
    deps: &CatalogDeps,
    _request: ListEpisodeCommentsRequest,
) -> Result<PagedResult<EpisodeComment>, ContractError> {
    ensure_enabled(deps.flags.detail_and_comments)?;
    Err(unimplemented_stub())
}

pub async fn list_comment_replies(
    deps: &CatalogDeps,
    _request: ListCommentRepliesRequest,
) -> Result<PagedResult<CommentReply>, ContractError> {
    ensure_enabled(deps.flags.detail_and_comments)?;
    Err(unimplemented_stub())
}
