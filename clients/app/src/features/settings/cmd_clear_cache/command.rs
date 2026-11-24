use hg_contracts::viewing::StorageOverview;

#[must_use]
pub const fn zeroed_cache_overview(current: StorageOverview) -> StorageOverview {
    StorageOverview {
        offline_library_bytes: current.offline_library_bytes,
        playback_cache_bytes: 0,
        catalog_cache_bytes: 0,
    }
}

#[must_use]
pub const fn can_trigger_clear(clearing: bool, storage: Option<StorageOverview>) -> bool {
    if clearing {
        return false;
    }
    match storage {
        Some(ov) => ov.playback_cache_bytes > 0 || ov.catalog_cache_bytes > 0,
        None => true,
    }
}
