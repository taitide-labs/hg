use nsh_core::prelude::*;

use super::super::store;
use super::command;
use crate::gateway;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct ClearCacheState {
    clearing: bool,
    hint: Option<String>,
    error: Option<String>,
}

fn cache_state() -> Signal<ClearCacheState> {
    global::<ClearCacheState>()
}

#[must_use]
pub fn is_clearing() -> bool {
    cache_state().with(|s| s.clearing)
}

#[must_use]
pub fn status_hint() -> Option<String> {
    cache_state().with(|s| s.hint.clone())
}

#[must_use]
pub fn error_message() -> Option<String> {
    cache_state().with(|s| s.error.clone())
}

pub fn clear_cache() {
    let clearing = cache_state().with_untracked(|s| s.clearing);
    let storage = store::state().with_untracked(|s| s.storage);
    if !command::can_trigger_clear(clearing, storage) {
        return;
    }

    cache_state().update(|s| {
        s.clearing = true;
        s.hint = None;
        s.error = None;
    });

    let gw = gateway::current();
    spawn(async move {
        let playback_res = gw.clear_playback_cache().await;
        let catalog_res = gw.clear_catalog_cache().await;

        match (playback_res, catalog_res) {
            (Ok(()), Ok(())) => {
                refresh_after_clear(&*gw).await;
                cache_state().update(|s| {
                    s.clearing = false;
                    s.hint = Some("已清空在线播放与图片缓存，离线片库不受影响".to_owned());
                    s.error = None;
                });
            }
            (Err(err), _) | (_, Err(err)) => {
                if let Ok(overview) = gw.storage_overview().await {
                    store::state().update(|s| s.storage = Some(overview));
                }
                cache_state().update(|s| {
                    s.clearing = false;
                    s.hint = None;
                    s.error = Some(err.to_string());
                });
            }
        }
    });
}

async fn refresh_after_clear(gw: &dyn hg_gateway::contract::AppGateway) {
    match gw.storage_overview().await {
        Ok(overview) => {
            store::state().update(|s| {
                s.storage = Some(overview);
            });
        }
        Err(_) => {
            store::state().update(|s| {
                if let Some(current) = s.storage {
                    s.storage = Some(command::zeroed_cache_overview(current));
                }
            });
        }
    }
}
