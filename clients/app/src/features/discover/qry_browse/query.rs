use std::num::{NonZeroU16, NonZeroU32};

use hg_contracts::catalog::{
    ListRankingRequest, ListSeriesRequest, RankingKind, SearchSeriesRequest, SeriesFilter,
    SeriesSort,
};
use hg_contracts::ids::PageRequest;
use nsh_core::prelude::*;

use super::super::store::{self, BrowseKind};
use crate::gateway;

fn default_page() -> PageRequest {
    PageRequest::new(
        NonZeroU32::new(1).unwrap_or(NonZeroU32::MIN),
        NonZeroU16::new(24).unwrap_or(NonZeroU16::MIN),
    )
}

pub fn load_meta() {
    let gw = gateway::current();
    spawn(async move {
        let groups = gw.tag_groups().await.unwrap_or_default();
        let keywords = gw.hot_keywords().await.unwrap_or_default();
        store::set_browse_meta(groups, keywords);
    });
}

pub fn load_browse(kind: BrowseKind) {
    if let BrowseKind::Search { ref query } = kind {
        store::search_input_signal().set(query.clone());
    }
    store::set_browse_loading(kind.clone());
    let gw = gateway::current();
    let page = default_page();

    spawn(async move {
        match kind {
            BrowseKind::Featured => {
                let req = ListSeriesRequest {
                    filter: SeriesFilter::All { category_id: None },
                    sort: SeriesSort::Hot,
                    page,
                };
                match gw.list_series(req).await {
                    Ok(paged) => store::set_browse_series(paged.items),
                    Err(err) => store::set_browse_error(err.to_string()),
                }
            }
            BrowseKind::Ranking(rk) => {
                let req = ListRankingRequest { kind: rk, page };
                match gw.list_ranking(req).await {
                    Ok(paged) => store::set_browse_rankings(paged.items),
                    Err(err) => store::set_browse_error(err.to_string()),
                }
            }
            BrowseKind::Category { route, .. } => {
                let req = ListSeriesRequest {
                    filter: SeriesFilter::CategoryRoute { route },
                    sort: SeriesSort::Hot,
                    page,
                };
                match gw.list_series(req).await {
                    Ok(paged) => store::set_browse_series(paged.items),
                    Err(err) => store::set_browse_error(err.to_string()),
                }
            }
            BrowseKind::Tag { slug, .. } => {
                let req = ListSeriesRequest {
                    filter: SeriesFilter::TagSlug { slug },
                    sort: SeriesSort::Hot,
                    page,
                };
                match gw.list_series(req).await {
                    Ok(paged) => store::set_browse_series(paged.items),
                    Err(err) => store::set_browse_error(err.to_string()),
                }
            }
            BrowseKind::Search { query } => {
                let trimmed = query.trim().to_owned();
                if trimmed.is_empty() {
                    let req = ListSeriesRequest {
                        filter: SeriesFilter::All { category_id: None },
                        sort: SeriesSort::Hot,
                        page,
                    };
                    match gw.list_series(req).await {
                        Ok(paged) => store::set_browse_series(paged.items),
                        Err(err) => store::set_browse_error(err.to_string()),
                    }
                } else {
                    let req = SearchSeriesRequest {
                        keyword: trimmed,
                        page,
                    };
                    match gw.search_series(req).await {
                        Ok(paged) => store::set_browse_series(paged.items),
                        Err(err) => store::set_browse_error(err.to_string()),
                    }
                }
            }
        }
    });
}

pub fn switch_ranking(kind: RankingKind) {
    load_browse(BrowseKind::Ranking(kind));
}
