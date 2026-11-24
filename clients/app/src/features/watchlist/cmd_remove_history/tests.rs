use std::num::{NonZeroU32, NonZeroU64};

use hg_contracts::ids::{EpisodeNumber, SeriesId};
use hg_contracts::viewing::WatchHistoryEntry;

use super::command::{clear_entries, remove_entry_by_id};

fn sample_entry(id: u64) -> WatchHistoryEntry {
    WatchHistoryEntry {
        series_id: SeriesId::new(NonZeroU64::new(id).unwrap_or(NonZeroU64::MIN)),
        title: format!("剧目 {id}"),
        cover_url: "poster://amber-noir".to_owned(),
        last_episode: EpisodeNumber::new(NonZeroU32::new(1).unwrap_or(NonZeroU32::MIN)),
        position_secs: 30,
        duration_secs: 120,
        episode_completed: false,
        total_episodes: 40,
        watched_at_epoch_secs: 1_763_968_000,
    }
}

#[test]
fn remove_entry_by_id_removes_only_matching_series() {
    let mut entries = vec![sample_entry(1001), sample_entry(1002)];
    let target = SeriesId::new(NonZeroU64::new(1002).unwrap_or(NonZeroU64::MIN));
    let missing = SeriesId::new(NonZeroU64::new(9999).unwrap_or(NonZeroU64::MIN));

    assert!(remove_entry_by_id(&mut entries, target));
    assert!(!remove_entry_by_id(&mut entries, missing));
    let remaining_ids: Vec<u64> = entries.iter().map(|entry| entry.series_id.get()).collect();
    assert_eq!(remaining_ids, vec![1001]);
}

#[test]
fn clear_entries_empties_history_list() {
    let mut entries = vec![sample_entry(1001), sample_entry(1002)];

    clear_entries(&mut entries);

    assert!(entries.is_empty());
}
