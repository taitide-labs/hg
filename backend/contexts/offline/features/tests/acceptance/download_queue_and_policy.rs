use hg_backend_contracts::catalog::{EpisodeSnapshot, SeriesSnapshot};
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId, StreamProfile};
use hg_contracts::offline::{
    ControlDownloadAction, ControlDownloadRequest, DownloadFailureReason, DownloadPolicy,
    DownloadTaskState, EnqueueEpisodeDownloadsRequest,
};
use hg_offline_features::flags::OfflineFeatureFlags;
use hg_offline_features::{cmd_download_queue, qry_download_queue};

use super::support::{FakeSegmentFetcher, OfflineTestHarness, StubCatalogPorts, sample_ticket};

fn build_queue_harness_inputs() -> (
    StubCatalogPorts,
    FakeSegmentFetcher,
    SeriesId,
    EpisodeKey,
    EpisodeKey,
) {
    let series_id = SeriesId::try_from(101).expect("series 101");
    let ep1 = EpisodeNumber::try_from(1).expect("ep 1");
    let ep2 = EpisodeNumber::try_from(2).expect("ep 2");
    let key1 = EpisodeKey::new(series_id, ep1);
    let key2 = EpisodeKey::new(series_id, ep2);

    let catalog = StubCatalogPorts::default()
        .with_series(SeriesSnapshot {
            series_id,
            title: "夜雨长灯".to_owned(),
            cover_url: "/covers/101.jpg".to_owned(),
            total_episodes: 2,
            is_finished: true,
            score: 9.2,
        })
        .with_episode(EpisodeSnapshot {
            key: key1,
            title: "第1集".to_owned(),
            duration_secs: 90,
            playable: true,
        })
        .with_episode(EpisodeSnapshot {
            key: key2,
            title: "第2集".to_owned(),
            duration_secs: 95,
            playable: true,
        })
        .with_cover("/covers/101.jpg", b"\x89PNGcover".to_vec())
        .with_ticket(sample_ticket(
            key1,
            Some("https://cdn.example.com/s101/e1/index.m3u8"),
            None,
        ))
        .with_ticket(sample_ticket(
            key2,
            None,
            Some("https://cdn.example.com/s101/e2/preview.m3u8"),
        ));

    let playlist = b"#EXTM3U\n#EXTINF:5.0,\nseg0.ts\n#EXTINF:5.0,\nseg1.ts\n#EXT-X-ENDLIST\n";
    let fetcher = FakeSegmentFetcher::default()
        .with_file(
            "https://cdn.example.com/s101/e1/index.m3u8",
            playlist.to_vec(),
        )
        .with_file(
            "https://cdn.example.com/s101/e1/seg0.ts",
            b"chunk-zero".to_vec(),
        )
        .with_file(
            "https://cdn.example.com/s101/e1/seg1.ts",
            b"chunk-one!".to_vec(),
        );

    (catalog, fetcher, series_id, key1, key2)
}

#[tokio::test]
async fn download_queue_disabled_by_default_returns_feature_flag_error() {
    let harness = OfflineTestHarness::new(
        OfflineFeatureFlags::default(),
        StubCatalogPorts::default(),
        FakeSegmentFetcher::default(),
    )
    .await;

    let err = qry_download_queue::download_tasks(&harness.deps)
        .await
        .expect_err("disabled feature flag should fail");
    assert!(err.to_string().contains("特性开关未开启"));
}

#[tokio::test]
#[ignore = "待实现离线下载队列、策略与原子提交切片后激活"]
async fn download_queue_enqueues_commits_atomically_and_rejects_preview_only() {
    let (catalog, fetcher, series_id, key1, key2) = build_queue_harness_inputs();
    let flags = OfflineFeatureFlags {
        download_queue: true,
        ..OfflineFeatureFlags::default()
    };
    let harness = OfflineTestHarness::new(flags, catalog, fetcher).await;

    let updated_policy = DownloadPolicy {
        preferred_profile: StreamProfile::Hevc,
        wifi_only: false,
        max_concurrent_tasks: 3,
        storage_directory: None,
    };
    cmd_download_queue::update_download_policy(&harness.deps, updated_policy.clone())
        .await
        .expect("update policy");
    assert_eq!(
        qry_download_queue::download_policy(&harness.deps)
            .await
            .expect("query policy"),
        updated_policy
    );

    let enqueued = cmd_download_queue::enqueue_episode_downloads(
        &harness.deps,
        EnqueueEpisodeDownloadsRequest {
            series_id,
            episodes: vec![key1.ep_num, key2.ep_num, key1.ep_num],
        },
    )
    .await
    .expect("enqueue episodes");
    assert_eq!(enqueued.queued, vec![key1, key2]);

    cmd_download_queue::control_download(
        &harness.deps,
        ControlDownloadRequest {
            key: key1,
            action: ControlDownloadAction::Pause,
        },
    )
    .await
    .expect("pause ep1");
    cmd_download_queue::control_download(
        &harness.deps,
        ControlDownloadRequest {
            key: key1,
            action: ControlDownloadAction::Resume,
        },
    )
    .await
    .expect("resume ep1");

    let first_done = cmd_download_queue::execute_next_queued_task(&harness.deps)
        .await
        .expect("execute ep1");
    assert_eq!(first_done, Some(key1));
    assert!(
        harness
            .deps
            .storage
            .library_episode_dir(key1)
            .join("index.m3u8")
            .exists()
    );

    let second_run = cmd_download_queue::execute_next_queued_task(&harness.deps)
        .await
        .expect("execute ep2 preview-only should record failure");
    assert_eq!(second_run, Some(key2));

    let tasks = qry_download_queue::download_tasks(&harness.deps)
        .await
        .expect("list tasks");
    let task2 = tasks.iter().find(|t| t.key == key2).expect("ep2 task");
    assert!(matches!(
        task2.state,
        DownloadTaskState::Failed {
            reason: DownloadFailureReason::PreviewStreamOnly,
            ..
        }
    ));
}
