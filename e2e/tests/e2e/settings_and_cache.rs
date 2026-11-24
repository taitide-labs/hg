use std::num::NonZeroU32;
use std::time::Duration;

use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId, StreamProfile};
use hg_contracts::offline::EnqueueEpisodeDownloadsRequest;
use hg_gateway::contract::AppGateway as _;

use super::harness::E2eHarness;

const CACHE_SEED_BYTES: usize = 163_840;

#[test]
fn settings_persistence_storage_stats_and_cache_clear_preserves_offline_assets() {
    let mut harness = E2eHarness::start().expect("start e2e harness");
    harness.app.wait_for("夜雨长灯").expect("home loaded");

    let series_id = SeriesId::try_from(101).expect("series 101");
    let ep1 = EpisodeNumber::new(NonZeroU32::new(1).expect("ep 1"));
    let ep1_key = EpisodeKey::new(series_id, ep1);

    seed_offline_and_cache(&harness, series_id, ep1);
    update_and_verify_settings_persistence(&mut harness);
    verify_storage_stats_and_clear_cache(&mut harness, ep1_key);
}

fn seed_offline_and_cache(harness: &E2eHarness, series_id: SeriesId, ep1: EpisodeNumber) {
    harness
        .block_on(
            harness
                .gateway
                .enqueue_episode_downloads(EnqueueEpisodeDownloadsRequest {
                    series_id,
                    episodes: vec![ep1],
                }),
        )
        .expect("enqueue ep 1");
    let completed = harness
        .drain_download_queue()
        .expect("download ep 1 to offline library");
    assert_eq!(completed, 1);

    let storage = harness.gateway.runtime().storage();
    let playback_seed_path = storage.playback_cache_dir().join("seed-segment.ts");
    let catalog_seed_dir = storage.catalog_cache_dir().join("covers");
    let catalog_seed_path = catalog_seed_dir.join("seed-cover.bin");
    harness
        .block_on(async {
            tokio::fs::create_dir_all(&catalog_seed_dir).await?;
            tokio::fs::write(&playback_seed_path, vec![0x47_u8; CACHE_SEED_BYTES]).await?;
            tokio::fs::write(&catalog_seed_path, vec![0xAA_u8; CACHE_SEED_BYTES]).await
        })
        .expect("seed cache directories");
}

fn update_and_verify_settings_persistence(harness: &mut E2eHarness) {
    harness.app.press("设置").expect("open settings tab");
    harness
        .app
        .wait_for("播放偏好")
        .expect("playback preference section visible");
    harness
        .app
        .wait_for("下载策略")
        .expect("download policy section visible");
    harness
        .app
        .wait_for("存储空间")
        .expect("storage section visible");
    harness
        .app
        .wait_for("标准兼容")
        .expect("playback preference controls loaded");
    harness
        .app
        .wait_for("仅 Wi-Fi 下载")
        .expect("download policy controls loaded");
    harness
        .app
        .wait_for("清理缓存")
        .expect("storage clear button loaded");

    harness
        .app
        .press("标准兼容")
        .expect("switch preferred profile to main");
    harness
        .app
        .press("1.5×")
        .expect("switch default playback rate");
    harness
        .app
        .press("仅 Wi-Fi 下载")
        .expect("toggle wifi-only policy");
    harness
        .app
        .press("4 个")
        .expect("set max concurrent downloads");
    harness
        .app
        .wait(Duration::from_millis(150))
        .expect("wait settings persistence");

    harness.app.press("发现").expect("leave settings tab");
    harness
        .app
        .wait_for("精选推荐")
        .expect("discover tab active");
    harness.app.press("设置").expect("re-enter settings tab");
    harness
        .app
        .wait_until(
            "persisted settings selected",
            Duration::from_secs(2),
            |screen| {
                screen.selected("role:tab[标准兼容]").unwrap_or(false)
                    && screen.selected("role:tab[1.5×]").unwrap_or(false)
                    && screen.selected("role:tab[4 个]").unwrap_or(false)
            },
        )
        .expect("reloaded settings selection persisted");

    let persisted_pref = harness
        .block_on(harness.gateway.playback_preference())
        .expect("query persisted playback preference");
    assert_eq!(persisted_pref.preferred_profile, StreamProfile::Main);
    assert_eq!(persisted_pref.default_rate_hundredths, 150);

    let persisted_policy = harness
        .block_on(harness.gateway.download_policy())
        .expect("query persisted download policy");
    assert!(!persisted_policy.wifi_only);
    assert_eq!(persisted_policy.max_concurrent_tasks, 4);
}

fn verify_storage_stats_and_clear_cache(harness: &mut E2eHarness, ep1_key: EpisodeKey) {
    let before_overview = harness
        .block_on(harness.gateway.storage_overview())
        .expect("storage overview before clear");
    assert!(before_overview.offline_library_bytes > 0);
    assert!(before_overview.playback_cache_bytes > 0);
    assert!(before_overview.catalog_cache_bytes > 0);

    harness.app.press("清理缓存").expect("press clear cache");
    harness
        .app
        .wait_for("已清空在线播放与图片缓存，离线片库不受影响")
        .expect("clear cache hint displayed");
    harness
        .app
        .wait_for("0 MB")
        .expect("cache zeroed on screen");

    let after_overview = harness
        .block_on(harness.gateway.storage_overview())
        .expect("storage overview after clear");
    assert_eq!(after_overview.catalog_cache_bytes, 0);
    assert_eq!(after_overview.playback_cache_bytes, 0);
    assert_eq!(
        after_overview.offline_library_bytes,
        before_overview.offline_library_bytes
    );

    let source = harness
        .block_on(harness.gateway.prepare_episode_playback(ep1_key))
        .expect("prepare offline playback after cache clear");
    assert!(source.playable_uri.contains("/offline/"));
    harness
        .verify_hls_stream_ok(&source.playable_uri)
        .expect("offline stream remains playable after cache clear");
}
