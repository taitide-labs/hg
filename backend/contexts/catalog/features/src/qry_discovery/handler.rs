use hg_contracts::catalog::{
    Category, HomeFeed, HotKeyword, HotTag, ListRankingRequest, ListSeriesRequest, PagedResult,
    RankingEntry, SearchSeriesRequest, SearchSuggestion, SeriesSummary, TagGroup,
};
use hg_contracts::error::ContractError;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::CatalogDeps;

use super::cache::{read_cached_json, write_cached_json};
use super::codec::{
    extract_data, parse_categories, parse_home_feed, parse_hot_keywords, parse_hot_tags,
    parse_paged_series, parse_ranking, parse_search_suggestions, parse_tag_groups,
};
use super::query::{
    UpstreamRoute, categories_route, home_feed_route, hot_keywords_route, hot_tags_route,
    list_ranking_route, list_series_route, search_series_route, search_suggestions_route,
    tag_groups_route,
};

async fn execute_route<T, F>(
    deps: &CatalogDeps,
    route: &UpstreamRoute,
    context: &str,
    parser: F,
) -> Result<T, ContractError>
where
    T: Serialize + DeserializeOwned,
    F: FnOnce(Value) -> Result<T, ContractError>,
{
    let cache_ttl = route.ttl_secs;

    if cache_ttl.is_some() {
        let now = deps.clock.now_epoch_secs();
        if let Some(cached) = read_cached_json::<T>(deps.pool(), &route.path, now).await? {
            return Ok(cached);
        }
    }

    let envelope = deps
        .upstream
        .request_json(route.scope, "GET", &route.path, None)
        .await
        .map_err(ContractError::from)?;
    let data = extract_data(envelope, context)?;
    let parsed = parser(data)?;

    if let Some(ttl_secs) = cache_ttl {
        let now = deps.clock.now_epoch_secs();
        write_cached_json(deps.pool(), &route.path, &parsed, now, ttl_secs).await?;
    }

    Ok(parsed)
}

pub async fn home_feed(deps: &CatalogDeps) -> Result<HomeFeed, ContractError> {
    let route = home_feed_route();
    execute_route(deps, &route, "首页聚合", parse_home_feed).await
}

pub async fn categories(deps: &CatalogDeps) -> Result<Vec<Category>, ContractError> {
    let route = categories_route();
    execute_route(deps, &route, "频道分类", parse_categories).await
}

pub async fn tag_groups(deps: &CatalogDeps) -> Result<Vec<TagGroup>, ContractError> {
    let route = tag_groups_route();
    execute_route(deps, &route, "标签分组", parse_tag_groups).await
}

pub async fn list_series(
    deps: &CatalogDeps,
    request: ListSeriesRequest,
) -> Result<PagedResult<SeriesSummary>, ContractError> {
    let page_req = request.page;
    let route = list_series_route(&request)?;
    execute_route(deps, &route, "剧目列表", move |data| {
        parse_paged_series(data, page_req, "剧目列表")
    })
    .await
}

pub async fn list_ranking(
    deps: &CatalogDeps,
    request: ListRankingRequest,
) -> Result<PagedResult<RankingEntry>, ContractError> {
    let kind = request.kind;
    let page_req = request.page;
    let route = list_ranking_route(request);
    execute_route(deps, &route, "排行榜", move |data| {
        parse_ranking(data, kind, page_req)
    })
    .await
}

pub async fn search_series(
    deps: &CatalogDeps,
    request: SearchSeriesRequest,
) -> Result<PagedResult<SeriesSummary>, ContractError> {
    let page_req = request.page;
    let route = search_series_route(&request)?;
    execute_route(deps, &route, "全文检索", move |data| {
        parse_paged_series(data, page_req, "全文检索")
    })
    .await
}

pub async fn search_suggestions(
    deps: &CatalogDeps,
    keyword: String,
) -> Result<SearchSuggestion, ContractError> {
    let Some(route) = search_suggestions_route(&keyword) else {
        return Ok(SearchSuggestion {
            series: Vec::new(),
            phrases: Vec::new(),
            tags: Vec::new(),
        });
    };
    execute_route(deps, &route, "搜索建议", parse_search_suggestions).await
}

pub async fn hot_keywords(deps: &CatalogDeps) -> Result<Vec<HotKeyword>, ContractError> {
    let route = hot_keywords_route();
    execute_route(deps, &route, "热搜词", parse_hot_keywords).await
}

pub async fn hot_tags(deps: &CatalogDeps) -> Result<Vec<HotTag>, ContractError> {
    let route = hot_tags_route();
    execute_route(deps, &route, "热门标签", parse_hot_tags).await
}
