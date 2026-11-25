use std::num::{NonZeroU16, NonZeroU32};

use hg_contracts::catalog::{
    ListRankingRequest, ListSeriesRequest, RankingKind, SeriesFilter, SeriesSort,
};
use hg_contracts::ids::PageRequest;
use nsh_core::prelude::*;

use super::super::parts::{dedup_append, sync_watched_series};
use super::super::store;
use crate::gateway;

const HOME_SHELF_TARGET_ITEMS: usize = 12;
const HOME_RANKING_FETCH_SIZE: u16 = 12;

fn first_page(size: u16) -> PageRequest {
    PageRequest::new(
        NonZeroU32::new(1).unwrap_or(NonZeroU32::MIN),
        NonZeroU16::new(size).unwrap_or(NonZeroU16::MIN),
    )
}

pub fn load_home() {
    store::set_home_loading();
    let gw = gateway::current();
    let rank_req = ListRankingRequest {
        kind: RankingKind::Hot,
        page: first_page(HOME_RANKING_FETCH_SIZE),
    };
    spawn(async move {
        let (feed_res, cats_res, rank_res, ()) = (
            gw.home_feed().await,
            gw.categories().await,
            gw.list_ranking(rank_req).await,
            sync_watched_series().await,
        );
        match (feed_res, cats_res, rank_res) {
            (Ok(mut feed), Ok(categories), Ok(rankings)) => {
                if feed.latest.len() < HOME_SHELF_TARGET_ITEMS
                    && let Ok(extra) = gw
                        .list_series(ListSeriesRequest {
                            filter: SeriesFilter::All { category_id: None },
                            sort: SeriesSort::New,
                            page: first_page(HOME_SHELF_TARGET_ITEMS as u16),
                        })
                        .await
                {
                    dedup_append(&mut feed.latest, extra.items, |item| item.id);
                }

                for rec in &mut feed.category_recommendations {
                    if rec.items.len() >= HOME_SHELF_TARGET_ITEMS {
                        continue;
                    }
                    let filter = categories
                        .iter()
                        .find(|c| c.id == rec.category_id && !c.route.is_empty())
                        .map_or(
                            SeriesFilter::All {
                                category_id: Some(rec.category_id),
                            },
                            |c| SeriesFilter::CategoryRoute {
                                route: c.route.clone(),
                            },
                        );
                    if let Ok(extra) = gw
                        .list_series(ListSeriesRequest {
                            filter,
                            sort: SeriesSort::Hot,
                            page: first_page(HOME_SHELF_TARGET_ITEMS as u16),
                        })
                        .await
                    {
                        dedup_append(&mut rec.items, extra.items, |item| item.id);
                    }
                }

                store::set_home_loaded(feed, rankings.items, categories);
            }
            (Err(err), _, _) | (_, Err(err), _) | (_, _, Err(err)) => {
                store::set_home_error(err.to_string());
            }
        }
    });
}
