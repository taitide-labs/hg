use hg_backend_contracts::offline::OfflineAssetQueryPort as _;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId, StreamProfile};
use hg_offline_features::flags::OfflineFeatureFlags;
use hg_offline_features::qry_offline_library::OfflineAssetQueryAdapter;
use hg_offline_features::{cmd_offline_library, qry_offline_library};

use super::support::{FakeSegmentFetcher, OfflineTestHarness, StubCatalogPorts};

async fn seed_committed_series(
    harness: &OfflineTestHarness,
    series_id: SeriesId,
) -> (EpisodeKey, EpisodeKey) {
    let ep1 = EpisodeKey::new(series_id, EpisodeNumber::try_from(1).expect("ep 1"));
    let ep2 = EpisodeKey::new(series_id, EpisodeNumber::try_from(2).expect("ep 2"));

    for key in [ep1, ep2] {
        let dir = harness.deps.storage.library_episode_dir(key);
        tokio::fs::create_dir_all(&dir)
            .await
            .expect("create ep dir");
        tokio::fs::write(dir.join("index.m3u8"), b"#EXTM3U\n#EXT-X-ENDLIST\n")
            .await
            .expect("write playlist");
    }

    sqlx::query(
        "INSERT INTO offline_series (series_id, title, cover_local_path, total_episodes, updated_at_epoch_secs)
         VALUES (?1, '夜雨长灯', 's101/cover.png', 2, 1700000000)",
    )
    .bind(i64::try_from(series_id.get()).expect("i64"))
    .execute(harness.deps.pool())
    .await
    .expect("seed offline_series");

    for (ep, size) in [(1_i64, 1024_i64), (2_i64, 2048_i64)] {
        sqlx::query(
            "INSERT INTO offline_episodes (series_id, ep_num, title, duration_secs, profile, playlist_rel_path, size_bytes, downloaded_at_epoch_secs)
             VALUES (?1, ?2, '分集', 90, 'hevc', ?3, ?4, 1700000000)",
        )
        .bind(i64::try_from(series_id.get()).expect("i64"))
        .bind(ep)
        .bind(format!("s{}/e{ep}/index.m3u8", series_id.get()))
        .bind(size)
        .execute(harness.deps.pool())
        .await
        .expect("seed offline_episodes");
    }

    (ep1, ep2)
}

#[tokio::test]
async fn offline_library_disabled_by_default_returns_feature_flag_error() {
    let harness = OfflineTestHarness::new(
        OfflineFeatureFlags::default(),
        StubCatalogPorts::default(),
        FakeSegmentFetcher::default(),
    )
    .await;

    let err = qry_offline_library::offline_library(&harness.deps)
        .await
        .expect_err("disabled feature flag should fail");
    assert!(err.to_string().contains("特性开关未开启"));
}

#[tokio::test]
#[ignore = "待实现离线片库查询与 Outbox 删档停播切片后激活"]
async fn delete_offline_assets_dispatches_outbox_halts_playback_and_removes_dirs() {
    let flags = OfflineFeatureFlags {
        library_and_outbox: true,
        ..OfflineFeatureFlags::default()
    };
    let harness = OfflineTestHarness::new(
        flags,
        StubCatalogPorts::default(),
        FakeSegmentFetcher::default(),
    )
    .await;

    let series_id = SeriesId::try_from(101).expect("series 101");
    let (ep1, ep2) = seed_committed_series(&harness, series_id).await;

    let library = qry_offline_library::offline_library(&harness.deps)
        .await
        .expect("query offline library");
    assert_eq!(library.len(), 1);
    assert_eq!(library.first().expect("series").episodes.len(), 2);

    let port = OfflineAssetQueryAdapter::new(harness.deps.clone());
    let committed = port
        .find_committed_episode(ep1)
        .await
        .expect("find ep1")
        .expect("ep1 exists");
    assert_eq!(committed.profile, StreamProfile::Hevc);
    assert_eq!(
        port.total_offline_library_bytes().await.expect("bytes"),
        3072
    );

    cmd_offline_library::delete_offline_episode(&harness.deps, ep1)
        .await
        .expect("delete ep1");
    assert_eq!(harness.halt.halted_episodes(), vec![ep1]);
    assert!(!harness.deps.storage.library_episode_dir(ep1).exists());
    assert!(harness.deps.storage.library_episode_dir(ep2).exists());

    cmd_offline_library::delete_offline_series(&harness.deps, series_id)
        .await
        .expect("delete series");
    assert_eq!(harness.halt.halted_series(), vec![series_id]);
    assert!(!harness.deps.storage.library_series_dir(series_id).exists());
    assert_eq!(port.total_offline_library_bytes().await.expect("bytes"), 0);

    let pending_outbox: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM offline_outbox_events WHERE dispatched_at_epoch_secs IS NULL",
    )
    .fetch_one(harness.deps.pool())
    .await
    .expect("count pending outbox");
    assert_eq!(pending_outbox.0, 0);
}
