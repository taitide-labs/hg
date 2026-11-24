use std::sync::Arc;

use hg_infra::clock::FixedClock;
use tempfile::tempdir;

use crate::app::{BackendRuntime, REGISTERED_CONTEXTS, runtime_name};

#[test]
fn runtime_name_matches_package() {
    assert_eq!(runtime_name(), "hg-runtime");
    assert_eq!(REGISTERED_CONTEXTS, ["catalog", "offline", "viewing"]);
}

#[tokio::test]
async fn runtime_boots_in_memory_with_isolated_storage_and_loopback() {
    let root = tempdir().expect("tempdir should create");
    let data_root = root.path().join("data");
    let cache_root = root.path().join("cache");
    let clock = Arc::new(FixedClock::new(1_700_000_000));

    let runtime = BackendRuntime::open_in_memory(&data_root, &cache_root, clock)
        .await
        .expect("in-memory runtime should start");

    assert!(
        runtime
            .loopback()
            .base_url()
            .starts_with("http://127.0.0.1:")
    );
    assert!(runtime.storage().offline_library_dir().exists());
    assert!(runtime.storage().catalog_cache_dir().exists());
    assert!(runtime.storage().playback_cache_dir().exists());

    runtime.shutdown().await;
}
