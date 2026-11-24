use hg_contracts::ids::StreamProfile;
use hg_contracts::offline::EnqueueEpisodeDownloadsRequest;
use hg_contracts::viewing::{PlaybackOrigin, ReportWatchProgressRequest};
use hg_runtime::catalog::{cmd_playback_and_cache, qry_discovery, qry_series_detail};
use hg_runtime::offline::{
    cmd_download_queue, cmd_offline_library, qry_download_queue, qry_offline_library,
};
use hg_runtime::viewing::{
    cmd_playback_session, cmd_record_watch_progress, cmd_watchlist_and_settings, qry_watch_history,
    qry_watchlist_and_settings,
};

use super::support::IntegrationHarness;

#[tokio::test]
async fn full_cross_context_offline_playback_cache_isolation_and_outbox_halt_flow() {
    let harness = IntegrationHarness::start().await;
    let http = reqwest::Client::new();

    let home = qry_discovery::home_feed(&harness.contexts.catalog)
        .await
        .expect("home feed");
    assert_eq!(home.featured.len(), 1);
    let detail = qry_series_detail::series_detail(&harness.contexts.catalog, harness.series_id)
        .await
        .expect("series detail");
    assert_eq!(detail.summary.title, "夜雨长灯");

    let enqueue = cmd_download_queue::enqueue_episode_downloads(
        &harness.contexts.offline,
        EnqueueEpisodeDownloadsRequest {
            series_id: harness.series_id,
            episodes: vec![harness.ep1.ep_num, harness.ep2.ep_num],
        },
    )
    .await
    .expect("enqueue two episodes");
    assert_eq!(enqueue.queued.len(), 2);

    assert_eq!(
        cmd_download_queue::execute_next_queued_task(&harness.contexts.offline)
            .await
            .expect("download ep1"),
        Some(harness.ep1)
    );
    assert_eq!(
        cmd_download_queue::execute_next_queued_task(&harness.contexts.offline)
            .await
            .expect("download ep2"),
        Some(harness.ep2)
    );
    assert!(
        qry_download_queue::download_tasks(&harness.contexts.offline)
            .await
            .expect("tasks")
            .len()
            == 2
    );

    let library = qry_offline_library::offline_library(&harness.contexts.offline)
        .await
        .expect("offline library");
    assert_eq!(library.len(), 1);
    assert_eq!(library.first().expect("series").episodes.len(), 2);

    cmd_watchlist_and_settings::bookmark_series(&harness.contexts.viewing, harness.series_id)
        .await
        .expect("bookmark");
    let _progress = cmd_record_watch_progress::execute(
        &harness.contexts.viewing,
        ReportWatchProgressRequest {
            key: harness.ep1,
            position_secs: 40,
            duration_secs: 90,
        },
    )
    .await
    .expect("record progress");
    assert_eq!(
        qry_watch_history::execute(&harness.contexts.viewing)
            .await
            .expect("watch history")
            .len(),
        1
    );

    harness.upstream.set_offline(true);
    let calls_before_offline_play = harness.upstream.call_count();
    let offline_play =
        cmd_playback_session::prepare_episode_playback(&harness.contexts.viewing, harness.ep1)
            .await
            .expect("offline-first playback while network down");
    assert_eq!(harness.upstream.call_count(), calls_before_offline_play);
    assert_eq!(
        offline_play.origin,
        PlaybackOrigin::OfflineLocal {
            profile: StreamProfile::Hevc
        }
    );
    let playlist_resp = http
        .get(&offline_play.playable_uri)
        .send()
        .await
        .expect("GET offline playlist");
    assert_eq!(playlist_resp.status(), reqwest::StatusCode::OK);
    assert!(
        playlist_resp
            .text()
            .await
            .expect("playlist text")
            .contains("#EXTM3U")
    );

    verify_cache_clear_and_outbox_delete(&harness, &http, &offline_play.playable_uri).await;
    harness.shutdown().await;
}

async fn verify_cache_clear_and_outbox_delete(
    harness: &IntegrationHarness,
    http: &reqwest::Client,
    ep1_play_url: &str,
) {
    let playback_cache_file = harness
        .runtime
        .storage()
        .playback_cache_dir()
        .join("temp.ts");
    tokio::fs::write(&playback_cache_file, b"temporary-playback-cache")
        .await
        .expect("seed playback cache");

    let before_clear = qry_watchlist_and_settings::storage_overview(&harness.contexts.viewing)
        .await
        .expect("storage overview before clear");
    assert!(before_clear.offline_library_bytes > 0);
    assert!(before_clear.playback_cache_bytes > 0);

    cmd_playback_and_cache::clear_catalog_cache(&harness.contexts.catalog)
        .await
        .expect("clear catalog cache");
    cmd_watchlist_and_settings::clear_playback_cache(&harness.contexts.viewing)
        .await
        .expect("clear playback cache");

    let after_clear = qry_watchlist_and_settings::storage_overview(&harness.contexts.viewing)
        .await
        .expect("storage overview after clear");
    assert_eq!(
        after_clear.offline_library_bytes,
        before_clear.offline_library_bytes
    );
    assert_eq!(after_clear.catalog_cache_bytes, 0);
    assert_eq!(after_clear.playback_cache_bytes, 0);
    assert_eq!(
        http.get(ep1_play_url)
            .send()
            .await
            .expect("offline stream survives cache clear")
            .status(),
        reqwest::StatusCode::OK
    );

    let ep1_dir = harness.runtime.storage().library_episode_dir(harness.ep1);
    assert!(ep1_dir.exists());
    cmd_offline_library::delete_offline_episode(&harness.contexts.offline, harness.ep1)
        .await
        .expect("delete offline ep1");
    assert!(!ep1_dir.exists());
    assert_eq!(
        http.get(ep1_play_url)
            .send()
            .await
            .expect("GET after ep1 delete")
            .status(),
        reqwest::StatusCode::GONE
    );

    let series_dir = harness
        .runtime
        .storage()
        .library_series_dir(harness.series_id);
    assert!(series_dir.exists());
    cmd_offline_library::delete_offline_series(&harness.contexts.offline, harness.series_id)
        .await
        .expect("delete offline series");
    assert!(!series_dir.exists());
    assert!(
        qry_offline_library::offline_library(&harness.contexts.offline)
            .await
            .expect("empty offline library")
            .is_empty()
    );
}
