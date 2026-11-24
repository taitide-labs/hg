use hg_contracts::viewing::StorageOverview;
use nsh_core::prelude::*;

use super::super::store;
use crate::gateway;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct StorageError(Option<String>);

#[must_use]
pub fn error_message() -> Option<String> {
    global::<StorageError>().with(|e| e.0.clone())
}

pub fn load_storage() {
    let gw = gateway::current();
    spawn(async move {
        match gw.storage_overview().await {
            Ok(overview) => {
                global::<StorageError>().set(StorageError(None));
                store::state().update(|s| {
                    s.storage = Some(overview);
                });
            }
            Err(err) => {
                global::<StorageError>().set(StorageError(Some(err.to_string())));
            }
        }
    });
}

#[must_use]
pub const fn cache_total_bytes(overview: StorageOverview) -> u64 {
    overview
        .playback_cache_bytes
        .saturating_add(overview.catalog_cache_bytes)
}

#[must_use]
pub fn format_mb(bytes: u64) -> String {
    if bytes == 0 {
        return "0 MB".to_owned();
    }
    let mib = bytes as f64 / 1_048_576.0;
    format!("{mib:.1} MB")
}

#[must_use]
pub fn cache_breakdown(overview: StorageOverview) -> String {
    format!(
        "播放缓存 {} · 目录与图片 {}",
        format_mb(overview.playback_cache_bytes),
        format_mb(overview.catalog_cache_bytes)
    )
}
