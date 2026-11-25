use hg_backend_contracts::offline::CommittedOfflineAsset;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId, StreamProfile};
use hg_contracts::viewing::{PlaybackPreference, ReportWatchProgressRequest};
use hg_viewing_features::{
    cmd_record_watch_progress, cmd_watchlist_and_settings, qry_watch_history,
    qry_watchlist_and_settings,
};

use super::support::{StubOfflineAssets, StubViewingCatalog, ViewingTestHarness, sample_series};

#[tokio::test]
async fn watchlist_settings_and_cache_clear_preserve_offline_library() {
    let s101 = SeriesId::try_from(101).expect("series 101");
    let ep1 = EpisodeKey::new(s101, EpisodeNumber::try_from(1).expect("ep 1"));
    let catalog = StubViewingCatalog::default().with_series([sample_series(101, "夜雨长灯", 20)]);
    let offline = StubOfflineAssets::default().with_asset(CommittedOfflineAsset {
        key: ep1,
        profile: StreamProfile::Hevc,
        playlist_rel_path: "s101/e1/index.m3u8".to_owned(),
        duration_secs: 90,
        size_bytes: 4096,
    });
    let harness = ViewingTestHarness::new(catalog, offline).await;

    cmd_watchlist_and_settings::bookmark_series(&harness.deps, s101)
        .await
        .expect("bookmark series");
    assert!(
        qry_watchlist_and_settings::is_bookmarked(&harness.deps, s101)
            .await
            .expect("is_bookmarked")
    );
    assert_eq!(
        qry_watchlist_and_settings::bookmarks(&harness.deps)
            .await
            .expect("list bookmarks")
            .len(),
        1
    );
    cmd_watchlist_and_settings::unbookmark_series(&harness.deps, s101)
        .await
        .expect("unbookmark");
    assert!(
        !qry_watchlist_and_settings::is_bookmarked(&harness.deps, s101)
            .await
            .expect("not bookmarked")
    );

    cmd_record_watch_progress::execute(
        &harness.deps,
        ReportWatchProgressRequest {
            key: ep1,
            position_secs: 30,
            duration_secs: 90,
        },
    )
    .await
    .expect("seed history");
    cmd_watchlist_and_settings::remove_watch_history(&harness.deps, s101)
        .await
        .expect("remove history");
    assert!(
        qry_watch_history::execute(&harness.deps)
            .await
            .expect("empty history")
            .is_empty()
    );

    let pref = PlaybackPreference {
        preferred_profile: StreamProfile::Main,
        playback_cache_limit_mb: 256,
        default_rate_hundredths: 150,
        appearance: hg_contracts::viewing::AppAppearance::Dark,
        sidebar_collapsed: true,
        sidebar_width: 240,
    };
    cmd_watchlist_and_settings::update_playback_preference(&harness.deps, pref)
        .await
        .expect("update playback pref");
    assert_eq!(
        qry_watchlist_and_settings::playback_preference(&harness.deps)
            .await
            .expect("query playback pref"),
        pref
    );

    let cache_seg = harness.deps.storage.playback_cache_dir().join("seg.ts");
    tokio::fs::write(&cache_seg, b"playback-buffer")
        .await
        .expect("write playback cache");
    let offline_dir = harness.deps.storage.library_episode_dir(ep1);
    tokio::fs::create_dir_all(&offline_dir)
        .await
        .expect("create offline dir");
    let offline_file = offline_dir.join("index.m3u8");
    tokio::fs::write(&offline_file, b"#EXTM3U\n")
        .await
        .expect("write offline file");

    let overview = qry_watchlist_and_settings::storage_overview(&harness.deps)
        .await
        .expect("storage overview");
    assert_eq!(overview.offline_library_bytes, 4096);
    assert!(overview.playback_cache_bytes >= 15);

    cmd_watchlist_and_settings::clear_playback_cache(&harness.deps)
        .await
        .expect("clear playback cache");
    assert!(!cache_seg.exists());
    assert!(offline_file.exists());

    harness.shutdown().await;
}
