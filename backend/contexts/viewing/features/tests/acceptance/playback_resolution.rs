use hg_backend_contracts::offline::CommittedOfflineAsset;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId, StreamProfile};
use hg_contracts::viewing::{OnlineStreamKind, PlaybackOrigin};
use hg_viewing_features::cmd_playback_session;
use hg_viewing_features::flags::ViewingFeatureFlags;

use super::support::{
    StubOfflineAssets, StubViewingCatalog, ViewingTestHarness, sample_online_ticket,
};

#[tokio::test]
async fn playback_resolution_disabled_by_default_returns_feature_flag_error() {
    let harness = ViewingTestHarness::new(
        ViewingFeatureFlags::default(),
        StubViewingCatalog::default(),
        StubOfflineAssets::default(),
    )
    .await;

    let key = EpisodeKey::new(
        SeriesId::try_from(101).expect("series 101"),
        EpisodeNumber::try_from(1).expect("ep 1"),
    );
    let err = cmd_playback_session::prepare_episode_playback(&harness.deps, key)
        .await
        .expect_err("disabled feature flag should fail");
    assert!(err.to_string().contains("特性开关未开启"));
    harness.shutdown().await;
}

#[tokio::test]
async fn playback_prefers_offline_without_network_and_halts_on_delete() {
    let s101 = SeriesId::try_from(101).expect("series 101");
    let offline_key = EpisodeKey::new(s101, EpisodeNumber::try_from(1).expect("ep 1"));
    let online_key = EpisodeKey::new(s101, EpisodeNumber::try_from(2).expect("ep 2"));

    let offline_assets = StubOfflineAssets::default().with_asset(CommittedOfflineAsset {
        key: offline_key,
        profile: StreamProfile::Hevc,
        playlist_rel_path: "s101/e1/index.m3u8".to_owned(),
        duration_secs: 90,
        size_bytes: 2048,
    });
    let catalog = StubViewingCatalog::default().with_ticket(sample_online_ticket(online_key));
    let flags = ViewingFeatureFlags {
        playback_resolution: true,
        ..ViewingFeatureFlags::default()
    };
    let harness = ViewingTestHarness::new(flags, catalog, offline_assets).await;

    let ep_dir = harness.deps.storage.library_episode_dir(offline_key);
    tokio::fs::create_dir_all(&ep_dir).await.expect("ep dir");
    tokio::fs::write(ep_dir.join("index.m3u8"), b"#EXTM3U\n#EXT-X-ENDLIST\n")
        .await
        .expect("write playlist");

    let local_source = cmd_playback_session::prepare_episode_playback(&harness.deps, offline_key)
        .await
        .expect("offline playback should succeed");
    assert_eq!(
        local_source.origin,
        PlaybackOrigin::OfflineLocal {
            profile: StreamProfile::Hevc
        }
    );
    assert!(local_source.playable_uri.starts_with("http://127.0.0.1:"));
    assert!(harness.catalog.recorded_ticket_calls().is_empty());

    let client = reqwest::Client::new();
    let resp = client
        .get(&local_source.playable_uri)
        .send()
        .await
        .expect("fetch loopback playlist");
    assert_eq!(resp.status(), reqwest::StatusCode::OK);

    cmd_playback_session::stop_deleted_offline_episode(&harness.deps, offline_key)
        .await
        .expect("halt offline playback");
    let after_halt = client
        .get(&local_source.playable_uri)
        .send()
        .await
        .expect("fetch after halt");
    assert_eq!(after_halt.status(), reqwest::StatusCode::GONE);

    let online_source = cmd_playback_session::prepare_episode_playback(&harness.deps, online_key)
        .await
        .expect("online playback should succeed");
    assert_eq!(
        online_source.origin,
        PlaybackOrigin::OnlineStream {
            kind: OnlineStreamKind::Hevc
        }
    );
    assert!(online_source.playable_uri.starts_with("http://127.0.0.1:"));
    assert!(online_source.playable_uri.contains("/online/"));
    assert_eq!(harness.catalog.recorded_ticket_calls(), vec![online_key]);

    harness.shutdown().await;
}
