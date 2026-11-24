use hg_contracts::ids::SeriesId;
use hg_contracts::viewing::WatchHistoryEntry;

pub(super) fn remove_entry_by_id(
    entries: &mut Vec<WatchHistoryEntry>,
    series_id: SeriesId,
) -> bool {
    let before = entries.len();
    entries.retain(|entry| entry.series_id != series_id);
    entries.len() != before
}

pub(super) fn clear_entries(entries: &mut Vec<WatchHistoryEntry>) {
    entries.clear();
}
