use std::num::{NonZeroU16, NonZeroU32};

use hg_contracts::catalog::{ListRankingRequest, RankingKind};
use hg_contracts::ids::PageRequest;
use nsh_core::prelude::*;

use super::super::store;
use crate::gateway;

pub fn load_home() {
    store::set_home_loading();
    let gw = gateway::current();
    let rank_req = ListRankingRequest {
        kind: RankingKind::Hot,
        page: PageRequest::new(
            NonZeroU32::new(1).unwrap_or(NonZeroU32::MIN),
            NonZeroU16::new(6).unwrap_or(NonZeroU16::MIN),
        ),
    };
    spawn(async move {
        let feed_res = gw.home_feed().await;
        let cats_res = gw.categories().await;
        let rank_res = gw.list_ranking(rank_req).await;
        match (feed_res, cats_res, rank_res) {
            (Ok(feed), Ok(categories), Ok(rankings)) => {
                store::set_home_loaded(feed, rankings.items, categories);
            }
            (Err(err), _, _) | (_, Err(err), _) | (_, _, Err(err)) => {
                store::set_home_error(err.to_string());
            }
        }
    });
}
