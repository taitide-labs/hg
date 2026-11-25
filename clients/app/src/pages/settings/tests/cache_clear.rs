use std::time::Duration;

use hg_contracts::error::ContractError;

use super::start_on_settings;

#[test]
fn clearing_cache_zeroes_cache_and_preserves_offline_library() {
    let (gw, mut app) = start_on_settings();
    let offline_before = gw.snapshot_storage().offline_library_bytes;
    assert!(offline_before > 0);

    app.wait_for("清理缓存").unwrap();
    app.press("清理缓存").unwrap();
    app.wait_for("0 MB").unwrap();
    assert!(app.screen().shows("离线片库"));
    assert!(app.screen().shows("55.7 MB") || app.screen().shows("58.4 MB"));

    let after = gw.snapshot_storage();
    assert_eq!(after.catalog_cache_bytes, 0);
    assert_eq!(after.playback_cache_bytes, 0);
    assert_eq!(after.offline_library_bytes, offline_before);
}

#[test]
fn clearing_cache_failure_surfaces_error_without_zeroing_storage() {
    let (gw, mut app) = start_on_settings();
    gw.inject_method_error(
        "clear_playback_cache",
        Some(ContractError::NetworkUnavailable {
            detail: "disk busy".to_owned(),
        }),
    );

    app.wait_for("清理缓存").unwrap();
    app.press("清理缓存").unwrap();
    app.wait(Duration::from_millis(80)).unwrap();
    assert!(gw.snapshot_storage().playback_cache_bytes > 0);
    assert!(!app.screen().shows("0 MB"));
}
