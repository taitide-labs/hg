use hg_contracts::ids::EpisodeKey;
use hg_contracts::offline::{
    ControlAllDownloadsAction, ControlDownloadAction, ControlDownloadRequest, DownloadTaskState,
};
use nsh_core::prelude::*;

use super::super::store;
use super::command;
use crate::gateway;

pub fn control_all(action: ControlAllDownloadsAction) {
    let gw = gateway::current();
    spawn(async move {
        match gw.control_all_downloads(action).await {
            Ok(()) => {
                store::state().update(|s| {
                    command::apply_bulk_control(&mut s.tasks, action);
                    s.error = None;
                });
                if let Ok(mut tasks) = gw.download_tasks().await {
                    tasks.retain(|t| {
                        !matches!(
                            t.state,
                            DownloadTaskState::Completed | DownloadTaskState::Cancelled
                        )
                    });
                    store::state().update(|s| {
                        s.tasks = tasks;
                    });
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

pub fn control_task(key: EpisodeKey, action: ControlDownloadAction) {
    let gw = gateway::current();
    spawn(async move {
        match gw
            .control_download(ControlDownloadRequest { key, action })
            .await
        {
            Ok(()) => {
                store::state().update(|s| {
                    command::apply_single_control(&mut s.tasks, key, action);
                    s.error = None;
                });
                if let Ok(mut tasks) = gw.download_tasks().await {
                    tasks.retain(|t| {
                        !matches!(
                            t.state,
                            DownloadTaskState::Completed | DownloadTaskState::Cancelled
                        )
                    });
                    store::state().update(|s| {
                        s.tasks = tasks;
                    });
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
