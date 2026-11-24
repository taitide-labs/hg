use hg_backend_contracts::catalog::SeriesSnapshot;
use hg_contracts::ids::{EpisodeNumber, SeriesId};
use hg_contracts::viewing::ReportWatchProgressResult;
use hg_viewing_domain::progress::evaluate_watch_progress;

use super::command::RecordWatchProgressCommand;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchHistoryWrite {
    pub series_id: SeriesId,
    pub title: String,
    pub cover_url: String,
    pub last_episode: EpisodeNumber,
    pub position_secs: u32,
    pub duration_secs: u32,
    pub episode_completed: bool,
    pub total_episodes: u32,
    pub watched_at_epoch_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordWatchProgressOutcome {
    pub history_write: WatchHistoryWrite,
    pub result: ReportWatchProgressResult,
}

#[must_use]
pub fn plan_record_watch_progress(
    command: RecordWatchProgressCommand,
    series: &SeriesSnapshot,
    watched_at_epoch_secs: u64,
) -> RecordWatchProgressOutcome {
    let result = evaluate_watch_progress(
        command.key,
        command.position_secs,
        command.duration_secs,
        series.total_episodes,
    );

    let history_write = WatchHistoryWrite {
        series_id: series.series_id,
        title: series.title.clone(),
        cover_url: series.cover_url.clone(),
        last_episode: result.progress.key.ep_num,
        position_secs: result.progress.position_secs,
        duration_secs: result.progress.duration_secs,
        episode_completed: result.progress.completed,
        total_episodes: series.total_episodes,
        watched_at_epoch_secs,
    };

    RecordWatchProgressOutcome {
        history_write,
        result,
    }
}
