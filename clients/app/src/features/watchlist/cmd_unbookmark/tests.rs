use std::num::NonZeroU64;

use hg_contracts::ids::SeriesId;
use hg_contracts::viewing::SeriesBookmark;

use super::command::remove_bookmark_by_id;

fn sample_bookmark(id: u64) -> SeriesBookmark {
    SeriesBookmark {
        series_id: SeriesId::new(NonZeroU64::new(id).unwrap_or(NonZeroU64::MIN)),
        title: format!("剧目 {id}"),
        cover_url: "poster://teal-mist".to_owned(),
        episode_count: 36,
        is_finished: true,
        score: 9.1,
        bookmarked_at_epoch_secs: 1_763_955_000,
    }
}

#[test]
fn remove_bookmark_by_id_removes_only_matching_series() {
    let mut entries = vec![sample_bookmark(1001), sample_bookmark(1005)];
    let target = SeriesId::new(NonZeroU64::new(1005).unwrap_or(NonZeroU64::MIN));
    let missing = SeriesId::new(NonZeroU64::new(9999).unwrap_or(NonZeroU64::MIN));

    assert!(remove_bookmark_by_id(&mut entries, target));
    assert!(!remove_bookmark_by_id(&mut entries, missing));
    let remaining_ids: Vec<u64> = entries.iter().map(|entry| entry.series_id.get()).collect();
    assert_eq!(remaining_ids, vec![1001]);
}
