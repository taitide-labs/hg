use std::collections::BTreeSet;
use std::num::{NonZeroU32, NonZeroU64};

use hg_contracts::catalog::EpisodeSummary;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId, StreamProfile};
use hg_contracts::offline::{
    DownloadTaskState, DownloadTaskView, EnqueueEpisodeDownloadsResult, OfflineEpisodeView,
    OfflineSeriesView,
};

use super::command::{
    EpisodeAvailability, build_enqueue_request, classify_existing_episodes, episode_segment_ranges,
    format_enqueue_feedback, is_segment_fully_selected, prune_unavailable_selection,
    selectable_episodes_in_range, toggle_episode_selection, toggle_select_segment,
};

fn series_id(id: u64) -> SeriesId {
    SeriesId::new(NonZeroU64::new(id).unwrap())
}

fn ep_num(num: u32) -> EpisodeNumber {
    EpisodeNumber::new(NonZeroU32::new(num).unwrap())
}

fn sample_episode(num: u32, playable: bool) -> EpisodeSummary {
    EpisodeSummary {
        ep_num: ep_num(num),
        title: format!("第 {num} 集"),
        duration_secs: 120,
        playable,
    }
}

#[test]
fn classify_existing_episodes_marks_offline_and_active_tasks() {
    let sid = series_id(1001);
    let offline = vec![OfflineSeriesView {
        series_id: sid,
        title: "剧目".to_owned(),
        cover_local_uri: "file://cover".to_owned(),
        total_episodes: 10,
        total_size_bytes: 1000,
        episodes: vec![OfflineEpisodeView {
            ep_num: ep_num(1),
            title: "第 1 集".to_owned(),
            duration_secs: 120,
            profile: StreamProfile::Hevc,
            size_bytes: 1000,
            downloaded_at_epoch_secs: 100,
        }],
    }];
    let tasks = vec![
        DownloadTaskView {
            key: EpisodeKey::new(sid, ep_num(2)),
            series_title: "剧目".to_owned(),
            episode_title: "第 2 集".to_owned(),
            cover_url: "poster://amber-noir".to_owned(),
            profile: StreamProfile::Hevc,
            state: DownloadTaskState::Queued,
        },
        DownloadTaskView {
            key: EpisodeKey::new(sid, ep_num(3)),
            series_title: "剧目".to_owned(),
            episode_title: "第 3 集".to_owned(),
            cover_url: "poster://amber-noir".to_owned(),
            profile: StreamProfile::Hevc,
            state: DownloadTaskState::Cancelled,
        },
    ];

    let map = classify_existing_episodes(sid, &offline, &tasks);
    assert_eq!(map.get(&1), Some(&EpisodeAvailability::AlreadyOffline));
    assert_eq!(map.get(&2), Some(&EpisodeAvailability::AlreadyQueued));
    assert_eq!(map.get(&3), None);
}

#[test]
fn segment_selection_skips_existing_and_unplayable_episodes() {
    let sid = series_id(1001);
    let episodes = vec![
        sample_episode(1, true),
        sample_episode(2, true),
        sample_episode(3, false),
        sample_episode(4, true),
        sample_episode(51, true),
    ];
    let offline = vec![OfflineSeriesView {
        series_id: sid,
        title: "剧目".to_owned(),
        cover_local_uri: "file://cover".to_owned(),
        total_episodes: 10,
        total_size_bytes: 1000,
        episodes: vec![OfflineEpisodeView {
            ep_num: ep_num(1),
            title: "第 1 集".to_owned(),
            duration_secs: 120,
            profile: StreamProfile::Hevc,
            size_bytes: 1000,
            downloaded_at_epoch_secs: 100,
        }],
    }];
    let existing = classify_existing_episodes(sid, &offline, &[]);
    let ranges = episode_segment_ranges(51, 50);
    assert_eq!(ranges, vec![(1, 50), (51, 51)]);

    let selectable =
        selectable_episodes_in_range(&episodes, ranges.first().copied().unwrap(), &existing);
    assert_eq!(selectable, vec![2, 4]);

    let mut selected = BTreeSet::new();
    toggle_select_segment(&mut selected, &selectable);
    assert!(is_segment_fully_selected(&selected, &selectable));
    assert_eq!(selected.iter().copied().collect::<Vec<_>>(), vec![2, 4]);

    toggle_select_segment(&mut selected, &selectable);
    assert!(selected.is_empty());

    toggle_episode_selection(&mut selected, 1);
    toggle_episode_selection(&mut selected, 4);
    prune_unavailable_selection(&mut selected, &existing);
    assert_eq!(selected.iter().copied().collect::<Vec<_>>(), vec![4]);
}

#[test]
fn enqueue_request_and_feedback_handle_queued_and_skipped_counts() {
    let sid = series_id(1001);
    let empty = BTreeSet::new();
    assert!(build_enqueue_request(sid, &empty).is_none());

    let selected: BTreeSet<u32> = [2, 5].into_iter().collect();
    let req = build_enqueue_request(sid, &selected).unwrap();
    assert_eq!(req.episodes.len(), 2);

    let all_queued = EnqueueEpisodeDownloadsResult {
        queued: vec![EpisodeKey::new(sid, ep_num(2))],
        skipped_already_present: Vec::new(),
    };
    assert_eq!(
        format_enqueue_feedback(&all_queued),
        "已加入离线下载（1 集）"
    );

    let mixed = EnqueueEpisodeDownloadsResult {
        queued: vec![EpisodeKey::new(sid, ep_num(2))],
        skipped_already_present: vec![EpisodeKey::new(sid, ep_num(1))],
    };
    assert_eq!(
        format_enqueue_feedback(&mixed),
        "已加入离线下载（1 集，已跳过 1 集）"
    );

    let none_queued = EnqueueEpisodeDownloadsResult {
        queued: Vec::new(),
        skipped_already_present: vec![EpisodeKey::new(sid, ep_num(1))],
    };
    assert_eq!(
        format_enqueue_feedback(&none_queued),
        "所选分集已在离线片库或下载队列中"
    );
}
