use std::num::{NonZeroU16, NonZeroU32};

use hg_contracts::catalog::{
    ListSeriesRequest, PagedResult, SearchSeriesRequest, SeriesFilter, SeriesSort, SeriesSummary,
};
use hg_contracts::error::ContractError;
use hg_contracts::ids::PageRequest;
use nsh_core::prelude::*;

use super::super::parts::{
    adaptive_page_size, dedup_append, min_items_to_fill_viewport, sync_watched_series,
};
use super::super::store::{self, SearchTarget};
use crate::gateway;

fn page_request(page: u32) -> PageRequest {
    let (w, h) = store::viewport_size_untracked();
    let size = adaptive_page_size(w, h, false);
    PageRequest::new(
        NonZeroU32::new(page.max(1)).unwrap_or(NonZeroU32::MIN),
        NonZeroU16::new(size).unwrap_or(NonZeroU16::MIN),
    )
}

pub fn ensure_viewport_filled() {
    let (w, h) = store::viewport_size_untracked();
    let needs_more = store::search_state().with_untracked(|s| {
        if s.loading
            || s.loading_more
            || s.error.is_some()
            || s.load_more_error.is_some()
            || s.target.is_none()
        {
            return false;
        }
        let Some(page_info) = s.pagination else {
            return false;
        };
        if page_info.page >= page_info.pages {
            return false;
        }
        s.items.len() < min_items_to_fill_viewport(w, h, false)
    });
    if needs_more {
        load_next_search_page();
    }
}

pub fn init_search_page() {
    let gw = gateway::current();
    spawn(async move {
        sync_watched_series().await;
        let groups = gw.tag_groups().await.unwrap_or_default();
        let keywords = gw.hot_keywords().await.unwrap_or_default();
        store::set_browse_meta(groups, keywords);
        if let Ok(history) = gw.search_history().await {
            store::search_state().update(|s| {
                s.history = history;
            });
        }
    });
}

pub fn clear_results() {
    store::search_state().update(|s| {
        s.request_seq = s.request_seq.wrapping_add(1);
        s.target = None;
        s.items.clear();
        s.pagination = None;
        s.loading = false;
        s.loading_more = false;
        s.error = None;
        s.load_more_error = None;
    });
}

pub fn submit_keyword(raw: &str) {
    let trimmed = raw.trim().to_owned();
    store::search_input_signal().set(trimmed.clone());
    if trimmed.is_empty() {
        clear_results();
        return;
    }

    let mut seq = 0;
    store::search_state().update(|s| {
        s.request_seq = s.request_seq.wrapping_add(1);
        seq = s.request_seq;
        s.target = Some(SearchTarget::Keyword(trimmed.clone()));
        s.items.clear();
        s.pagination = None;
        s.loading = true;
        s.loading_more = false;
        s.error = None;
        s.load_more_error = None;
    });

    let gw = gateway::current();
    let keyword_for_history = trimmed.clone();
    spawn(async move {
        if gw.record_search_history(keyword_for_history).await.is_ok()
            && let Ok(history) = gw.search_history().await
        {
            store::search_state().update(|s| {
                s.history = history;
            });
        }
    });

    fetch_search_page(
        SearchTarget::Keyword(trimmed),
        SeriesSort::Hot,
        1,
        seq,
        true,
    );
}

pub fn toggle_tag(slug: String, name: String) {
    let is_already_active = store::search_state().with_untracked(
        |s| matches!(&s.target, Some(SearchTarget::Tag { name: current, .. }) if *current == name),
    );
    if is_already_active {
        clear_results();
        return;
    }

    select_tag_internal(slug, name);
}

pub fn activate_tag(name: String) {
    store::search_input_signal().set(String::new());
    select_tag_internal(name.clone(), name);
}

fn select_tag_internal(slug: String, name: String) {
    let mut seq = 0;
    let mut sort = SeriesSort::Hot;
    store::search_state().update(|s| {
        s.request_seq = s.request_seq.wrapping_add(1);
        seq = s.request_seq;
        s.target = Some(SearchTarget::Tag {
            slug: slug.clone(),
            name: name.clone(),
        });
        sort = s.sort;
        s.items.clear();
        s.pagination = None;
        s.loading = true;
        s.loading_more = false;
        s.error = None;
        s.load_more_error = None;
    });

    fetch_search_page(SearchTarget::Tag { slug, name }, sort, 1, seq, true);
}

pub fn switch_sort(sort: SeriesSort) {
    let tag_target = store::search_state().with_untracked(|s| match &s.target {
        Some(SearchTarget::Tag { slug, name }) => Some((slug.clone(), name.clone())),
        _ => None,
    });
    let Some((slug, name)) = tag_target else {
        return;
    };

    let mut seq = 0;
    store::search_state().update(|s| {
        s.request_seq = s.request_seq.wrapping_add(1);
        seq = s.request_seq;
        s.sort = sort;
        s.items.clear();
        s.pagination = None;
        s.loading = true;
        s.loading_more = false;
        s.error = None;
        s.load_more_error = None;
    });

    fetch_search_page(SearchTarget::Tag { slug, name }, sort, 1, seq, true);
}

pub fn load_next_search_page() {
    let params = store::search_state().with_untracked(|s| {
        if s.loading || s.loading_more {
            return None;
        }
        let target = s.target.clone()?;
        let page_info = s.pagination?;
        if page_info.page >= page_info.pages {
            return None;
        }
        Some((
            target,
            s.sort,
            page_info.page.saturating_add(1),
            s.request_seq,
        ))
    });

    let Some((target, sort, next_page, seq)) = params else {
        return;
    };

    store::search_state().update(|s| {
        if s.request_seq == seq {
            s.loading_more = true;
            s.load_more_error = None;
        }
    });
    fetch_search_page(target, sort, next_page, seq, false);
}

pub fn remove_history(keyword: String) {
    store::search_state().update(|s| {
        s.history.retain(|entry| entry.keyword != keyword);
    });
    let gw = gateway::current();
    spawn(async move {
        if gw.remove_search_history(keyword).await.is_ok()
            && let Ok(history) = gw.search_history().await
        {
            store::search_state().update(|s| {
                s.history = history;
            });
        }
    });
}

pub fn clear_history() {
    store::search_state().update(|s| {
        s.history.clear();
    });
    let gw = gateway::current();
    spawn(async move {
        if gw.clear_search_history().await.is_ok()
            && let Ok(history) = gw.search_history().await
        {
            store::search_state().update(|s| {
                s.history = history;
            });
        }
    });
}

fn fetch_search_page(
    target: SearchTarget,
    sort: SeriesSort,
    page_num: u32,
    seq: u64,
    initial: bool,
) {
    let gw = gateway::current();
    let page = page_request(page_num);

    spawn(async move {
        let res = match target {
            SearchTarget::Keyword(keyword) => {
                gw.search_series(SearchSeriesRequest { keyword, page })
                    .await
            }
            SearchTarget::Tag { name, .. } => {
                gw.list_series(ListSeriesRequest {
                    filter: SeriesFilter::TagSlug { slug: name },
                    sort,
                    page,
                })
                .await
            }
        };
        apply_search_result(seq, initial, res);
    });
}

fn apply_search_result(
    seq: u64,
    initial: bool,
    result: Result<PagedResult<SeriesSummary>, ContractError>,
) {
    match result {
        Ok(paged) => {
            store::search_state().update(|s| {
                if s.request_seq != seq {
                    return;
                }
                if initial {
                    s.items = paged.items;
                    s.loading = false;
                } else {
                    dedup_append(&mut s.items, paged.items, |item| item.id);
                    s.loading_more = false;
                }
                s.pagination = Some(paged.pagination);
                s.error = None;
                s.load_more_error = None;
            });
            ensure_viewport_filled();
        }
        Err(err) => {
            store::search_state().update(|s| {
                if s.request_seq != seq {
                    return;
                }
                if initial {
                    s.loading = false;
                    s.error = Some(err.to_string());
                } else {
                    s.loading_more = false;
                    s.load_more_error = Some(err.to_string());
                }
            });
        }
    }
}
