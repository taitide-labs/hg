use std::cell::RefCell;
use std::num::{NonZeroU16, NonZeroU32};
use std::rc::Rc;
use std::sync::Arc;

use hg_contracts::catalog::{
    Category, HomeFeed, ListRankingRequest, ListSeriesRequest, RankingEntry, RankingKind,
    SeriesFilter, SeriesSort,
};
use hg_contracts::ids::PageRequest;
use nsh_core::prelude::*;

use super::super::parts::{dedup_append, sync_poster_preferences, sync_watched_series};
use super::super::store;
use crate::gateway;

const HOME_CANDIDATE_FETCH_SIZE: u16 = 20;
const HOME_RANKING_FETCH_SIZE: u16 = 20;
const HOME_SHELF_MIN_ITEMS: usize = 10;
const HOME_TOPUP_FETCH_SIZE: u16 = 10;
const HOME_REFILL_VISIBLE_TARGET: usize = 8;

fn first_page(size: u16) -> PageRequest {
    PageRequest::new(
        NonZeroU32::new(1).unwrap_or(NonZeroU32::MIN),
        NonZeroU16::new(size).unwrap_or(NonZeroU16::MIN),
    )
}

#[derive(Default)]
struct HomeLoadBatch {
    feed: Option<Result<HomeFeed, String>>,
    categories: Option<Result<Vec<Category>, String>>,
    rankings: Option<Result<Vec<RankingEntry>, String>>,
    hydrated_from_cache: bool,
    committed: bool,
}

fn apply_loaded_home(feed: HomeFeed, categories: Vec<Category>, rankings: Vec<RankingEntry>) {
    let needs_latest_topup = feed.latest.len() < HOME_SHELF_MIN_ITEMS;
    let short_categories: Vec<u32> = feed
        .category_recommendations
        .iter()
        .filter(|rec| rec.items.len() < HOME_SHELF_MIN_ITEMS)
        .map(|rec| rec.category_id)
        .collect();
    let categories_for_topup = categories.clone();
    store::set_home_loaded(feed, rankings, categories);
    let gw = gateway::current();
    if needs_latest_topup {
        spawn_latest_topup(&gw, HOME_TOPUP_FETCH_SIZE);
    }
    for cat_id in short_categories {
        spawn_category_topup(&gw, cat_id, &categories_for_topup, HOME_TOPUP_FETCH_SIZE);
    }
}

fn try_commit_home_batch(batch_cell: &Rc<RefCell<HomeLoadBatch>>) {
    let (ready, hydrated_from_cache) = {
        let mut batch = batch_cell.borrow_mut();
        if batch.committed
            || batch.feed.is_none()
            || batch.categories.is_none()
            || batch.rankings.is_none()
        {
            return;
        }
        batch.committed = true;
        let hydrated = batch.hydrated_from_cache;
        let res = match (
            batch.feed.take(),
            batch.categories.take(),
            batch.rankings.take(),
        ) {
            (Some(Ok(feed)), Some(Ok(categories)), Some(Ok(rankings))) => {
                Ok((feed, categories, rankings))
            }
            (Some(Err(err)), _, _) | (_, Some(Err(err)), _) | (_, _, Some(Err(err))) => Err(err),
            _ => return,
        };
        (res, hydrated)
    };

    match ready {
        Ok((feed, categories, rankings)) => apply_loaded_home(feed, categories, rankings),
        Err(err) => {
            if !hydrated_from_cache {
                store::set_home_error(err);
            }
        }
    }
}

fn spawn_latest_topup(gw: &Arc<dyn hg_gateway::contract::AppGateway>, fetch_size: u16) {
    let gw = Arc::clone(gw);
    spawn(async move {
        if let Ok(extra) = gw
            .list_series(ListSeriesRequest {
                filter: SeriesFilter::All { category_id: None },
                sort: SeriesSort::New,
                page: first_page(fetch_size),
            })
            .await
        {
            store::home_state().update(|s| {
                if let Some(current_feed) = &mut s.feed {
                    dedup_append(&mut current_feed.latest, extra.items, |item| item.id);
                }
            });
        }
    });
}

fn spawn_category_topup(
    gw: &Arc<dyn hg_gateway::contract::AppGateway>,
    cat_id: u32,
    categories: &[Category],
    fetch_size: u16,
) {
    let filter = categories
        .iter()
        .find(|c| c.id == cat_id && !c.route.is_empty())
        .map_or(
            SeriesFilter::All {
                category_id: Some(cat_id),
            },
            |c| SeriesFilter::CategoryRoute {
                route: c.route.clone(),
            },
        );
    let gw_cat = Arc::clone(gw);
    spawn(async move {
        if let Ok(extra) = gw_cat
            .list_series(ListSeriesRequest {
                filter,
                sort: SeriesSort::Hot,
                page: first_page(fetch_size),
            })
            .await
        {
            store::home_state().update(|s| {
                if let Some(current_feed) = &mut s.feed
                    && let Some(target_rec) = current_feed
                        .category_recommendations
                        .iter_mut()
                        .find(|r| r.category_id == cat_id)
                {
                    dedup_append(&mut target_rec.items, extra.items, |item| item.id);
                }
            });
        }
    });
}

pub fn refill_disliked_home_shelves() {
    let (feed, categories) =
        store::home_state().with_untracked(|s| (s.feed.clone(), s.categories.clone()));
    let Some(feed) = feed else {
        return;
    };
    let gw = gateway::current();
    let max_candidates = usize::from(HOME_CANDIDATE_FETCH_SIZE);
    let visible_latest = feed
        .latest
        .iter()
        .filter(|item| !store::is_disliked_untracked(item.id))
        .count();
    if visible_latest < HOME_REFILL_VISIBLE_TARGET && feed.latest.len() < max_candidates {
        spawn_latest_topup(&gw, HOME_CANDIDATE_FETCH_SIZE);
    }

    for rec in &feed.category_recommendations {
        let visible_cat = rec
            .items
            .iter()
            .filter(|item| !store::is_disliked_untracked(item.id))
            .count();
        if visible_cat >= HOME_REFILL_VISIBLE_TARGET || rec.items.len() >= max_candidates {
            continue;
        }
        spawn_category_topup(&gw, rec.category_id, &categories, HOME_CANDIDATE_FETCH_SIZE);
    }
}

pub fn load_home() {
    store::set_home_loading();
    spawn(sync_watched_series());
    spawn(sync_poster_preferences());

    let gw = gateway::current();
    let rank_req = ListRankingRequest {
        kind: RankingKind::Hot,
        page: first_page(HOME_RANKING_FETCH_SIZE),
    };
    let batch = Rc::new(RefCell::new(HomeLoadBatch::default()));

    let gw_cache = Arc::clone(&gw);
    let batch_cache = Rc::clone(&batch);
    spawn(async move {
        if let Ok(Some(snapshot)) = gw_cache.cached_home_snapshot().await {
            let mut guard = batch_cache.borrow_mut();
            if !guard.committed {
                guard.hydrated_from_cache = true;
                drop(guard);
                store::set_home_loaded(snapshot.feed, snapshot.rankings, snapshot.categories);
            }
        }
    });

    let gw_feed = Arc::clone(&gw);
    let batch_feed = Rc::clone(&batch);
    spawn(async move {
        let res = gw_feed.home_feed().await.map_err(|e| e.to_string());
        batch_feed.borrow_mut().feed = Some(res);
        try_commit_home_batch(&batch_feed);
    });

    let gw_cats = Arc::clone(&gw);
    let batch_cats = Rc::clone(&batch);
    spawn(async move {
        let res = gw_cats.categories().await.map_err(|e| e.to_string());
        batch_cats.borrow_mut().categories = Some(res);
        try_commit_home_batch(&batch_cats);
    });

    let batch_rank = Rc::clone(&batch);
    spawn(async move {
        let res = gw
            .list_ranking(rank_req)
            .await
            .map(|paged| paged.items)
            .map_err(|e| e.to_string());
        batch_rank.borrow_mut().rankings = Some(res);
        try_commit_home_batch(&batch_rank);
    });
}
