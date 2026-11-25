use hg_contracts::ids::{EpisodeKey, SeriesId};
use nsh_core::prelude::*;

use super::super::store;
use super::command;
use crate::gateway;

pub fn delete_series(series_id: SeriesId) {
    let gw = gateway::current();
    spawn(async move {
        match gw.delete_offline_series(series_id).await {
            Ok(()) => {
                store::state().update(|s| {
                    command::remove_series_from_library(&mut s.library, series_id);
                    s.error = None;
                });
                if let Ok(library) = gw.offline_library().await {
                    store::set_library_loaded(library);
                }
            }
            Err(err) => {
                store::state().update(|s| {
                    s.error = Some(err.to_string());
                });
            }
        }
    });
}

pub fn delete_episode(key: EpisodeKey) {
    let gw = gateway::current();
    spawn(async move {
        match gw.delete_offline_episode(key).await {
            Ok(()) => {
                store::state().update(|s| {
                    command::remove_episode_from_library(&mut s.library, key);
                    s.error = None;
                });
                if let Ok(library) = gw.offline_library().await {
                    store::set_library_loaded(library);
                }
            }
            Err(err) => {
                store::state().update(|s| {
                    s.error = Some(err.to_string());
                });
            }
        }
    });
}
