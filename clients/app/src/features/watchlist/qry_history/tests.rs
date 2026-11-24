use std::num::{NonZeroU32, NonZeroU64};

use hg_contracts::ids::{EpisodeNumber, SeriesId};
use hg_contracts::viewing::WatchHistoryEntry;

use super::query::{format_duration, progress_ratio, progress_summary, sort_recent_first};

fn sample_entry(
    id: u64,
    ep: u32,
    position_secs: u32,
    duration_secs: u32,
    completed: bool,
    watched_at_epoch_secs: u64,
) -> WatchHistoryEntry {
    WatchHistoryEntry {
        series_id: SeriesId::new(NonZeroU64::new(id).unwrap_or(NonZeroU64::MIN)),
        title: format!("剧目 {id}"),
        cover_url: "poster://amber-noir".to_owned(),
        last_episode: EpisodeNumber::new(NonZeroU32::new(ep).unwrap_or(NonZeroU32::MIN)),
        position_secs,
        duration_secs,
        episode_completed: completed,
        total_episodes: 68,
        watched_at_epoch_secs,
    }
}

#[test]
fn sort_recent_first_orders_entries_by_watched_at_descending() {
    let mut entries = vec![
        sample_entry(1002, 1, 125, 127, true, 1_763_964_000),
        sample_entry(1003, 5, 30, 120, false, 1_763_970_000),
        sample_entry(1001, 3, 64, 139, false, 1_763_968_000),
    ];

    sort_recent_first(&mut entries);

    let ordered_ids: Vec<u64> = entries.iter().map(|e| e.series_id.get()).collect();
    assert_eq!(ordered_ids, vec![1003, 1001, 1002]);
}

#[test]
fn progress_ratio_and_summary_handle_in_progress_completed_and_zero_duration() {
    let in_progress = sample_entry(1001, 3, 64, 139, false, 100);
    assert!((progress_ratio(&in_progress) - (64.0 / 139.0)).abs() < 0.001);
    assert_eq!(
        progress_summary(&in_progress),
        "已看 01:04 / 02:19 · 共 68 集"
    );

    let completed = sample_entry(1002, 1, 125, 127, true, 90);
    assert!((progress_ratio(&completed) - 1.0).abs() < f32::EPSILON);
    assert_eq!(progress_summary(&completed), "已看完 · 共 68 集");

    let zero_duration = sample_entry(1003, 1, 10, 0, false, 80);
    assert!(progress_ratio(&zero_duration).abs() < f32::EPSILON);
    assert_eq!(format_duration(64), "01:04");
}
