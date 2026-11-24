use hg_contracts::ids::SeriesId;
use nsh_core::prelude::*;

use super::super::store;
use super::command;
use crate::gateway;

pub fn remove_history(series_id: SeriesId) {
    let gw = gateway::current();
    spawn(async move {
        match gw.remove_watch_history(series_id).await {
            Ok(()) => {
                store::state().update(|s| {
                    command::remove_entry_by_id(&mut s.history, series_id);
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

pub fn clear_history() {
    let gw = gateway::current();
    spawn(async move {
        match gw.clear_watch_history().await {
            Ok(()) => {
                store::state().update(|s| {
                    command::clear_entries(&mut s.history);
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
