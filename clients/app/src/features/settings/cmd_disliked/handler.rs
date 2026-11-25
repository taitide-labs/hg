use hg_contracts::ids::SeriesId;
use nsh_core::prelude::*;

use super::super::store;
use crate::gateway;

pub fn open_drawer() {
    store::state().update(|s| {
        s.disliked_drawer_open = true;
    });
    load_disliked();
}

pub fn close_drawer() {
    store::state().update(|s| {
        s.disliked_drawer_open = false;
    });
}

pub fn load_disliked() {
    store::state().update(|s| {
        s.disliked_loading = true;
        s.disliked_error = None;
    });
    let gw = gateway::current();
    spawn(async move {
        match gw.disliked_series().await {
            Ok(entries) => {
                store::state().update(|s| {
                    s.disliked = entries;
                    s.disliked_loading = false;
                    s.disliked_error = None;
                });
            }
            Err(err) => {
                store::state().update(|s| {
                    s.disliked_loading = false;
                    s.disliked_error = Some(err.to_string());
                });
            }
        }
    });
}

pub fn undislike(series_id: SeriesId) {
    store::state().update(|s| {
        s.disliked.retain(|entry| entry.series_id != series_id);
    });
    let gw = gateway::current();
    spawn(async move {
        if gw.undislike_series(series_id).await.is_err() {
            load_disliked();
        }
    });
}

pub fn clear_disliked() {
    store::state().update(|s| {
        s.disliked.clear();
    });
    let gw = gateway::current();
    spawn(async move {
        if gw.clear_disliked_series().await.is_err() {
            load_disliked();
        }
    });
}
