use hg_contracts::ids::SeriesId;
use nsh_core::prelude::*;

use super::super::store;
use super::command;
use crate::gateway;

pub fn unbookmark(series_id: SeriesId) {
    let gw = gateway::current();
    spawn(async move {
        match gw.unbookmark_series(series_id).await {
            Ok(()) => {
                store::state().update(|s| {
                    command::remove_bookmark_by_id(&mut s.bookmarks, series_id);
                    s.error = None;
                });
            }
            Err(err) => {
                store::state().update(|s| {
                    s.error = Some(err.to_string());
                });
            }
        }
    });
}
