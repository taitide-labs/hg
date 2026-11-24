use std::num::NonZeroU64;

use hg_contracts::ids::SeriesId;
use hg_contracts::viewing::SeriesBookmark;

use super::query::{episode_status, format_score, sort_recent_first};

fn sample_bookmark(
    id: u64,
    episode_count: u32,
    is_finished: bool,
    score: f32,
    bookmarked_at_epoch_secs: u64,
) -> SeriesBookmark {
    SeriesBookmark {
        series_id: SeriesId::new(NonZeroU64::new(id).unwrap_or(NonZeroU64::MIN)),
        title: format!("剧目 {id}"),
        cover_url: "poster://teal-mist".to_owned(),
        episode_count,
        is_finished,
        score,
        bookmarked_at_epoch_secs,
    }
}

#[test]
fn sort_recent_first_orders_bookmarks_by_time_descending() {
    let mut entries = vec![
        sample_bookmark(1005, 36, true, 9.1, 1_763_955_000),
        sample_bookmark(1001, 68, true, 9.4, 1_763_960_000),
    ];

    sort_recent_first(&mut entries);

    let ordered_ids: Vec<u64> = entries.iter().map(|entry| entry.series_id.get()).collect();
    assert_eq!(ordered_ids, vec![1001, 1005]);
}

#[test]
fn format_score_and_episode_status_reflect_completion_state() {
    let finished = sample_bookmark(1001, 68, true, 9.4, 100);
    let ongoing = sample_bookmark(1002, 24, false, 8.7, 90);

    assert_eq!(format_score(finished.score), "9.4 分");
    assert_eq!(episode_status(&finished), "全 68 集");
    assert_eq!(episode_status(&ongoing), "更新至 24 集");
}
