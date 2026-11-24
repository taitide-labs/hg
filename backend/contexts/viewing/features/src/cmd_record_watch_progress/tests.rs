use hg_backend_contracts::catalog::SeriesSnapshot;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId};

use super::command::RecordWatchProgressCommand;
use super::workflow::plan_record_watch_progress;

#[test]
fn workflow_advances_to_next_episode_on_tail_completion() {
    let series_id = SeriesId::try_from(12).expect("valid series id");
    let ep3 = EpisodeNumber::try_from(3).expect("ep 3");
    let ep4 = EpisodeNumber::try_from(4).expect("ep 4");
    let snapshot = SeriesSnapshot {
        series_id,
        title: "春风渡".to_owned(),
        cover_url: "https://cdn.example.com/12.jpg".to_owned(),
        total_episodes: 10,
        is_finished: false,
        score: 8.8,
    };

    let outcome = plan_record_watch_progress(
        RecordWatchProgressCommand {
            key: EpisodeKey::new(series_id, ep3),
            position_secs: 96,
            duration_secs: 100,
        },
        &snapshot,
        1_700_000_000,
    );

    assert!(outcome.result.progress.completed);
    assert_eq!(outcome.result.next_episode, Some(ep4));
    assert_eq!(outcome.history_write.last_episode, ep3);
    assert_eq!(outcome.history_write.watched_at_epoch_secs, 1_700_000_000);
}
