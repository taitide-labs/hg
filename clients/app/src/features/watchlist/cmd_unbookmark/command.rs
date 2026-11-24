use hg_contracts::ids::SeriesId;
use hg_contracts::viewing::SeriesBookmark;

pub(super) fn remove_bookmark_by_id(
    entries: &mut Vec<SeriesBookmark>,
    series_id: SeriesId,
) -> bool {
    let before = entries.len();
    entries.retain(|entry| entry.series_id != series_id);
    entries.len() != before
}
