use hg_contracts::ids::{EpisodeKey, EpisodeNumber};
use hg_contracts::viewing::{ReportWatchProgressResult, WatchProgress};

pub const TAIL_COMPLETION_REMAINING_SECS: u32 = 3;
pub const TAIL_COMPLETION_PERCENT: u64 = 95;

#[must_use]
pub fn evaluate_watch_progress(
    key: EpisodeKey,
    position_secs: u32,
    duration_secs: u32,
    total_episodes: u32,
) -> ReportWatchProgressResult {
    let clamped_position = if duration_secs > 0 {
        position_secs.min(duration_secs)
    } else {
        position_secs
    };

    let completed = is_episode_completed(clamped_position, duration_secs);
    let next_episode = if completed && key.ep_num.get() < total_episodes {
        EpisodeNumber::try_from(key.ep_num.get().saturating_add(1)).ok()
    } else {
        None
    };

    ReportWatchProgressResult {
        progress: WatchProgress {
            key,
            position_secs: clamped_position,
            duration_secs,
            completed,
        },
        next_episode,
    }
}

#[must_use]
pub const fn is_episode_completed(position_secs: u32, duration_secs: u32) -> bool {
    if duration_secs == 0 {
        return false;
    }
    let remaining = duration_secs.saturating_sub(position_secs);
    if remaining <= TAIL_COMPLETION_REMAINING_SECS {
        return true;
    }
    (position_secs as u64).saturating_mul(100)
        >= (duration_secs as u64).saturating_mul(TAIL_COMPLETION_PERCENT)
}
