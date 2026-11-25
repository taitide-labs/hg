use std::num::{NonZeroU16, NonZeroU32};

use hg_contracts::catalog::{
    ListRankingRequest, ListSeriesRequest, RankingKind, SearchSeriesRequest, SeriesFilter,
    SeriesSort,
};
use hg_contracts::ids::PageRequest;
use nsh_core::prelude::*;

use super::super::parts::{dedup_append, sync_watched_series};
use super::super::store::{self, BrowseKind, RankingTab};
use crate::gateway;

const BROWSE_PAGE_SIZE: u16 = 24;

fn page_request(page: u32) -> PageRequest {
    PageRequest::new(
        NonZeroU32::new(page.max(1)).unwrap_or(NonZeroU32::MIN),
        NonZeroU16::new(BROWSE_PAGE_SIZE).unwrap_or(NonZeroU16::MIN),
    )
}

pub fn load_meta() {
    let gw = gateway::current();
    spawn(async move {
        sync_watched_series().await;
        let groups = gw.tag_groups().await.unwrap_or_default();
        let keywords = gw.hot_keywords().await.unwrap_or_default();
        store::set_browse_meta(groups, keywords);
    });
}

pub fn load_browse(kind: BrowseKind) {
    if let BrowseKind::Search { ref query } = kind {
        store::search_input_signal().set(query.clone());
    }
    spawn(sync_watched_series());
    load_browse_with_sort(kind, SeriesSort::Hot);
}

pub fn switch_sort(sort: SeriesSort) {
    let kind = store::browse_state().with_untracked(|s| s.kind.clone());
    load_browse_with_sort(kind, sort);
}

pub fn switch_ranking(kind: RankingKind) {
    load_browse(BrowseKind::Ranking(kind));
}

fn load_browse_with_sort(kind: BrowseKind, sort: SeriesSort) {
    if let BrowseKind::Ranking(rk) = &kind {
        store::ranking_tab_signal().set(RankingTab::from(*rk));
    }
    let mut seq = 0;
    store::browse_state().update(|s| {
        s.request_seq = s.request_seq.wrapping_add(1);
        seq = s.request_seq;
        s.kind = kind.clone();
        s.sort = sort;
        s.items.clear();
        s.ranking_items.clear();
        s.pagination = None;
        s.loading = true;
        s.loading_more = false;
        s.error = None;
        s.load_more_error = None;
    });
    fetch_browse_page(kind, sort, 1, seq, true);
}

pub fn load_next_browse_page() {
    let params = store::browse_state().with_untracked(|s| {
        if s.loading || s.loading_more {
            return None;
        }
        let page_info = s.pagination?;
        if page_info.page >= page_info.pages {
            return None;
        }
        Some((
            s.kind.clone(),
            s.sort,
            page_info.page.saturating_add(1),
            s.request_seq,
        ))
    });

    let Some((kind, sort, next_page, seq)) = params else {
        return;
    };

    store::browse_state().update(|s| {
        if s.request_seq == seq {
            s.loading_more = true;
            s.load_more_error = None;
        }
    });
    fetch_browse_page(kind, sort, next_page, seq, false);
}

fn fetch_browse_page(kind: BrowseKind, sort: SeriesSort, page_num: u32, seq: u64, initial: bool) {
    let gw = gateway::current();
    let page = page_request(page_num);

    spawn(async move {
        match kind {
            BrowseKind::Featured => {
                let req = ListSeriesRequest {
                    filter: SeriesFilter::All { category_id: None },
                    sort,
                    page,
                };
                apply_series_result(seq, initial, gw.list_series(req).await);
            }
            BrowseKind::Ranking(rk) => {
                let req = ListRankingRequest { kind: rk, page };
                match gw.list_ranking(req).await {
                    Ok(paged) => {
                        store::browse_state().update(|s| {
                            if s.request_seq != seq {
                                return;
                            }
                            if initial {
                                s.ranking_items = paged.items;
                                s.items.clear();
                                s.loading = false;
                            } else {
                                dedup_append(&mut s.ranking_items, paged.items, |r| r.series.id);
                                s.loading_more = false;
                            }
                            s.pagination = Some(paged.pagination);
                            s.error = None;
                            s.load_more_error = None;
                        });
                    }
                    Err(err) => apply_error(seq, initial, err.to_string()),
                }
            }
            BrowseKind::Category { route, .. } => {
                let req = ListSeriesRequest {
                    filter: SeriesFilter::CategoryRoute { route },
                    sort,
                    page,
                };
                apply_series_result(seq, initial, gw.list_series(req).await);
            }
            BrowseKind::Tag { slug, .. } => {
                let req = ListSeriesRequest {
                    filter: SeriesFilter::TagSlug { slug },
                    sort,
                    page,
                };
                apply_series_result(seq, initial, gw.list_series(req).await);
            }
            BrowseKind::Search { query } => {
                let trimmed = query.trim().to_owned();
                if trimmed.is_empty() {
                    let req = ListSeriesRequest {
                        filter: SeriesFilter::All { category_id: None },
                        sort,
                        page,
                    };
                    apply_series_result(seq, initial, gw.list_series(req).await);
                } else {
                    let req = SearchSeriesRequest {
                        keyword: trimmed,
                        page,
                    };
                    apply_series_result(seq, initial, gw.search_series(req).await);
                }
            }
        }
    });
}

fn apply_series_result(
    seq: u64,
    initial: bool,
    result: Result<
        hg_contracts::catalog::PagedResult<hg_contracts::catalog::SeriesSummary>,
        hg_contracts::error::ContractError,
    >,
) {
    match result {
        Ok(paged) => {
            store::browse_state().update(|s| {
                if s.request_seq != seq {
                    return;
                }
                if initial {
                    s.items = paged.items;
                    s.ranking_items.clear();
                    s.loading = false;
                } else {
                    dedup_append(&mut s.items, paged.items, |item| item.id);
                    s.loading_more = false;
                }
                s.pagination = Some(paged.pagination);
                s.error = None;
                s.load_more_error = None;
            });
        }
        Err(err) => apply_error(seq, initial, err.to_string()),
    }
}

fn apply_error(seq: u64, initial: bool, message: String) {
    store::browse_state().update(|s| {
        if s.request_seq != seq {
            return;
        }
        if initial {
            s.loading = false;
            s.error = Some(message);
        } else {
            s.loading_more = false;
            s.load_more_error = Some(message);
        }
    });
}
