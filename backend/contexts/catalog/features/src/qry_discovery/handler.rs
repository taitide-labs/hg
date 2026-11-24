use hg_contracts::catalog::{
    Category, HomeFeed, HotKeyword, HotTag, ListRankingRequest, ListSeriesRequest, PagedResult,
    RankingEntry, SearchSeriesRequest, SearchSuggestion, SeriesSummary, TagGroup,
};
use hg_contracts::error::ContractError;

use crate::CatalogDeps;

use super::query::{FEATURE_NAME, ensure_enabled};

fn unimplemented_stub() -> ContractError {
    ContractError::Internal {
        detail: format!("{FEATURE_NAME} 切片尚未实现"),
    }
}

pub async fn home_feed(deps: &CatalogDeps) -> Result<HomeFeed, ContractError> {
    ensure_enabled(deps.flags.discovery)?;
    Err(unimplemented_stub())
}

pub async fn categories(deps: &CatalogDeps) -> Result<Vec<Category>, ContractError> {
    ensure_enabled(deps.flags.discovery)?;
    Err(unimplemented_stub())
}

pub async fn tag_groups(deps: &CatalogDeps) -> Result<Vec<TagGroup>, ContractError> {
    ensure_enabled(deps.flags.discovery)?;
    Err(unimplemented_stub())
}

pub async fn list_series(
    deps: &CatalogDeps,
    _request: ListSeriesRequest,
) -> Result<PagedResult<SeriesSummary>, ContractError> {
    ensure_enabled(deps.flags.discovery)?;
    Err(unimplemented_stub())
}

pub async fn list_ranking(
    deps: &CatalogDeps,
    _request: ListRankingRequest,
) -> Result<PagedResult<RankingEntry>, ContractError> {
    ensure_enabled(deps.flags.discovery)?;
    Err(unimplemented_stub())
}

pub async fn search_series(
    deps: &CatalogDeps,
    _request: SearchSeriesRequest,
) -> Result<PagedResult<SeriesSummary>, ContractError> {
    ensure_enabled(deps.flags.discovery)?;
    Err(unimplemented_stub())
}

pub async fn search_suggestions(
    deps: &CatalogDeps,
    _keyword: String,
) -> Result<SearchSuggestion, ContractError> {
    ensure_enabled(deps.flags.discovery)?;
    Err(unimplemented_stub())
}

pub async fn hot_keywords(deps: &CatalogDeps) -> Result<Vec<HotKeyword>, ContractError> {
    ensure_enabled(deps.flags.discovery)?;
    Err(unimplemented_stub())
}

pub async fn hot_tags(deps: &CatalogDeps) -> Result<Vec<HotTag>, ContractError> {
    ensure_enabled(deps.flags.discovery)?;
    Err(unimplemented_stub())
}
