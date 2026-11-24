use std::num::{NonZeroU16, NonZeroU32};

use hg_contracts::catalog::{ListSeriesRequest, SeriesFilter, SeriesSort};
use hg_contracts::ids::PageRequest;
use nsh_core::prelude::*;

use super::super::store::{self, CategoryTab};
use crate::gateway;

pub fn load_home() {
    store::set_loading();
    let gw = gateway::current();
    spawn(async move {
        let feed_res = gw.home_feed().await;
        let cats_res = gw.categories().await;
        match (feed_res, cats_res) {
            (Ok(feed), Ok(categories)) => store::set_home(feed, categories),
            (Err(err), _) | (_, Err(err)) => store::set_error(err.to_string()),
        }
    });
}

pub fn select_category(tab: CategoryTab) {
    store::set_loading();
    let gw = gateway::current();
    let filter = match tab.route() {
        Some(route) => SeriesFilter::CategoryRoute {
            route: route.to_owned(),
        },
        None => SeriesFilter::All { category_id: None },
    };
    let request = ListSeriesRequest {
        filter,
        sort: SeriesSort::Hot,
        page: PageRequest::new(
            NonZeroU32::new(1).unwrap_or(NonZeroU32::MIN),
            NonZeroU16::new(24).unwrap_or(NonZeroU16::MIN),
        ),
    };
    spawn(async move {
        match gw.list_series(request).await {
            Ok(paged) => store::set_filtered(tab, paged.items),
            Err(err) => store::set_error(err.to_string()),
        }
    });
}
