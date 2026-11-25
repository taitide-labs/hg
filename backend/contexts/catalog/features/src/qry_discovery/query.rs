use hg_contracts::catalog::{
    ListRankingRequest, ListSeriesRequest, RankingKind, SearchSeriesRequest, SeriesFilter,
    SeriesSort,
};
use hg_contracts::error::ContractError;
use hg_infra::edge_crypto::EdgeChannelScope;

pub const FEATURE_NAME: &str = "catalog.discovery";
pub const STATIC_META_CACHE_TTL_SECS: u64 = 1_800;
pub const CATALOG_LIST_CACHE_TTL_SECS: u64 = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpstreamRoute {
    pub path: String,
    pub scope: EdgeChannelScope,
    pub ttl_secs: Option<u64>,
}

#[must_use]
pub fn home_feed_route() -> UpstreamRoute {
    UpstreamRoute {
        path: "/api/home".to_owned(),
        scope: EdgeChannelScope::Dynamic,
        ttl_secs: Some(CATALOG_LIST_CACHE_TTL_SECS),
    }
}

#[must_use]
pub fn home_latest_route() -> UpstreamRoute {
    UpstreamRoute {
        path: "/api/home/latest".to_owned(),
        scope: EdgeChannelScope::Public,
        ttl_secs: Some(CATALOG_LIST_CACHE_TTL_SECS),
    }
}

#[must_use]
pub fn categories_route() -> UpstreamRoute {
    UpstreamRoute {
        path: "/api/site/categories/video".to_owned(),
        scope: EdgeChannelScope::Public,
        ttl_secs: Some(STATIC_META_CACHE_TTL_SECS),
    }
}

#[must_use]
pub fn tag_groups_route() -> UpstreamRoute {
    UpstreamRoute {
        path: "/api/tags".to_owned(),
        scope: EdgeChannelScope::Public,
        ttl_secs: Some(STATIC_META_CACHE_TTL_SECS),
    }
}

pub fn list_series_route(request: &ListSeriesRequest) -> Result<UpstreamRoute, ContractError> {
    let sort_value = match request.sort {
        SeriesSort::New => "new",
        SeriesSort::Hot => "hot",
        SeriesSort::HotWindow => "hot_window",
        SeriesSort::Random => "random",
    };

    let page = request.page.page.get();
    let size = request.page.size.get();

    let path = match &request.filter {
        SeriesFilter::All { category_id: None } => {
            format!("/api/videos?page={page}&size={size}&sort={sort_value}")
        }
        SeriesFilter::All {
            category_id: Some(cate_id),
        } => {
            format!("/api/videos?cate_id={cate_id}&page={page}&size={size}&sort={sort_value}")
        }
        SeriesFilter::CategoryRoute { route } => {
            let trimmed = route.trim();
            if trimmed.is_empty() {
                return Err(ContractError::InvalidInput {
                    message: "频道分类路由不能为空".to_owned(),
                });
            }
            format!(
                "/api/videos/category/{}?page={page}&size={size}&sort={sort_value}",
                percent_encode(trimmed)
            )
        }
        SeriesFilter::TagSlug { slug } => {
            let trimmed = slug.trim();
            if trimmed.is_empty() {
                return Err(ContractError::InvalidInput {
                    message: "标签标识不能为空".to_owned(),
                });
            }
            format!(
                "/api/videos/tag/{}?page={page}&size={size}&sort={sort_value}",
                percent_encode(trimmed)
            )
        }
    };

    let (scope, ttl_secs) = if matches!(request.sort, SeriesSort::Random) {
        (EdgeChannelScope::Dynamic, None)
    } else {
        (EdgeChannelScope::Public, Some(CATALOG_LIST_CACHE_TTL_SECS))
    };

    Ok(UpstreamRoute {
        path,
        scope,
        ttl_secs,
    })
}

#[must_use]
pub fn list_ranking_route(request: ListRankingRequest) -> UpstreamRoute {
    let kind = match request.kind {
        RankingKind::Hot => "hot",
        RankingKind::Potential => "potential",
        RankingKind::Recommend => "recommend",
    };
    let page = request.page.page.get();
    let size = request.page.size.get();
    UpstreamRoute {
        path: format!("/api/ranks/{kind}?page={page}&size={size}"),
        scope: EdgeChannelScope::Public,
        ttl_secs: Some(CATALOG_LIST_CACHE_TTL_SECS),
    }
}

pub fn search_series_route(request: &SearchSeriesRequest) -> Result<UpstreamRoute, ContractError> {
    let trimmed = request.keyword.trim();
    if trimmed.is_empty() {
        return Err(ContractError::InvalidInput {
            message: "搜索关键词不能为空".to_owned(),
        });
    }

    let page = request.page.page.get();
    let size = request.page.size.get();
    Ok(UpstreamRoute {
        path: format!(
            "/api/search?page={page}&q={}&size={size}",
            percent_encode(trimmed)
        ),
        scope: EdgeChannelScope::Dynamic,
        ttl_secs: None,
    })
}

#[must_use]
pub fn search_suggestions_route(keyword: &str) -> Option<UpstreamRoute> {
    let trimmed = keyword.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(UpstreamRoute {
        path: format!(
            "/api/search/suggest?q={}&type=video",
            percent_encode(trimmed)
        ),
        scope: EdgeChannelScope::Public,
        ttl_secs: None,
    })
}

#[must_use]
pub fn hot_keywords_route() -> UpstreamRoute {
    UpstreamRoute {
        path: "/api/search/hot".to_owned(),
        scope: EdgeChannelScope::Public,
        ttl_secs: None,
    }
}

#[must_use]
pub fn hot_tags_route() -> UpstreamRoute {
    UpstreamRoute {
        path: "/api/search/tags/hot".to_owned(),
        scope: EdgeChannelScope::Public,
        ttl_secs: None,
    }
}

#[must_use]
pub fn percent_encode(input: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(input.len());
    for byte in input.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(*byte));
        } else {
            out.push('%');
            let hi = usize::from(byte >> 4);
            let lo = usize::from(byte & 0x0F);
            out.push(char::from(HEX.get(hi).copied().unwrap_or(b'0')));
            out.push(char::from(HEX.get(lo).copied().unwrap_or(b'0')));
        }
    }
    out
}
