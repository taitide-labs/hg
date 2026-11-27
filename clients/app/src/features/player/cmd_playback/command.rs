use std::num::NonZeroU32;

use hg_contracts::ids::EpisodeNumber;
use hg_contracts::viewing::WatchProgress;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AspectPreset {
    #[default]
    Original,
    Fill,
    Cinema16x9,
    Portrait9x16,
}

impl AspectPreset {
    #[must_use]
    pub const fn from_digit(digit: u8) -> Option<Self> {
        match digit {
            1 => Some(Self::Original),
            2 => Some(Self::Fill),
            3 => Some(Self::Cinema16x9),
            4 => Some(Self::Portrait9x16),
            _ => None,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Original => "原始比例",
            Self::Fill => "铺满画面",
            Self::Cinema16x9 => "16:9",
            Self::Portrait9x16 => "9:16",
        }
    }

    #[must_use]
    pub const fn protocol_key(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Fill => "fill",
            Self::Cinema16x9 => "16:9",
            Self::Portrait9x16 => "9:16",
        }
    }
}

#[must_use]
pub const fn clamp_seek(current: u32, delta_secs: i32, duration: u32) -> u32 {
    if delta_secs >= 0 {
        let next = current.saturating_add(delta_secs.unsigned_abs());
        if next > duration { duration } else { next }
    } else {
        current.saturating_sub(delta_secs.unsigned_abs())
    }
}

#[must_use]
pub const fn step_rate(current_hundredths: u16, faster: bool) -> u16 {
    match (current_hundredths, faster) {
        (50, true) | (100, false) => 75,
        (75, true) | (125, false) => 100,
        (100, true) | (150, false) => 125,
        (125, true) | (200, false) => 150,
        (150 | 200, true) => 200,
        (50 | 75, false) => 50,
        _ => 100,
    }
}

#[must_use]
pub const fn step_volume(current_percent: u8, louder: bool) -> u8 {
    if louder {
        let next = current_percent.saturating_add(10);
        if next > 100 { 100 } else { next }
    } else {
        current_percent.saturating_sub(10)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResumePlan {
    pub episode: EpisodeNumber,
    pub position_secs: Option<u32>,
}

#[must_use]
pub fn resume_plan(
    requested: Option<EpisodeNumber>,
    last_watched: Option<(EpisodeNumber, u32, bool)>,
    total_episodes: u32,
) -> ResumePlan {
    let first = EpisodeNumber::new(NonZeroU32::MIN);
    match (requested, last_watched) {
        (Some(episode), Some((last, position, false))) if episode == last && position > 0 => {
            ResumePlan {
                episode,
                position_secs: Some(position),
            }
        }
        (Some(episode), _) => ResumePlan {
            episode,
            position_secs: None,
        },
        (None, Some((last, position, completed))) => {
            if completed {
                ResumePlan {
                    episode: adjacent_episode(last, total_episodes, true).unwrap_or(last),
                    position_secs: None,
                }
            } else {
                ResumePlan {
                    episode: last,
                    position_secs: (position > 0).then_some(position),
                }
            }
        }
        (None, None) => ResumePlan {
            episode: first,
            position_secs: None,
        },
    }
}

#[must_use]
pub const fn is_tail_completed(position_secs: u32, duration_secs: u32) -> bool {
    if duration_secs == 0 {
        return false;
    }
    let remaining = duration_secs.saturating_sub(position_secs);
    if remaining <= 3 {
        return true;
    }
    (position_secs as u64).saturating_mul(100) >= (duration_secs as u64).saturating_mul(95)
}

#[must_use]
pub const fn episode_switch_resume_secs(saved: Option<&WatchProgress>) -> Option<u32> {
    let Some(progress) = saved else {
        return None;
    };
    if progress.completed || progress.position_secs == 0 {
        None
    } else {
        Some(progress.position_secs)
    }
}

#[must_use]
pub fn adjacent_episode(
    current: EpisodeNumber,
    total_episodes: u32,
    forward: bool,
) -> Option<EpisodeNumber> {
    let value = current.get();
    if forward {
        if value < total_episodes {
            NonZeroU32::new(value + 1).map(EpisodeNumber::new)
        } else {
            None
        }
    } else if value > 1 {
        NonZeroU32::new(value - 1).map(EpisodeNumber::new)
    } else {
        None
    }
}

#[must_use]
pub const fn folded_episode_window(current: u32, total: u32, window: u32) -> (u32, u32) {
    if total == 0 || window == 0 {
        return (1, 0);
    }
    if total <= window {
        return (1, total);
    }
    let clamped = if current == 0 {
        1
    } else if current > total {
        total
    } else {
        current
    };
    let before = window / 2;
    let start = if clamped <= before {
        1
    } else if clamped + (window - before - 1) > total {
        total.saturating_sub(window) + 1
    } else {
        clamped - before
    };
    let end = start + window - 1;
    (start, if end > total { total } else { end })
}

#[must_use]
pub fn episode_segment_ranges(total: u32, chunk: u32) -> Vec<(u32, u32)> {
    if total == 0 || chunk == 0 {
        return Vec::new();
    }
    let mut ranges = Vec::new();
    let mut start = 1;
    while start <= total {
        let end = (start + chunk - 1).min(total);
        ranges.push((start, end));
        start = end.saturating_add(1);
    }
    ranges
}

#[must_use]
pub fn format_time(seconds: u32) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{mins:02}:{secs:02}")
}

#[must_use]
pub fn format_rate_trigger(rate_hundredths: u16) -> String {
    if rate_hundredths == 100 {
        "倍速".to_owned()
    } else if rate_hundredths.is_multiple_of(100) {
        format!("{}.0×", rate_hundredths / 100)
    } else if rate_hundredths.is_multiple_of(10) {
        format!("{:.1}×", f32::from(rate_hundredths) / 100.0)
    } else {
        format!("{:.2}×", f32::from(rate_hundredths) / 100.0)
    }
}

#[must_use]
pub fn should_refresh_stream_on_player_error(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    !(lower.contains("unable to load libmpv runtime")
        || lower.contains("mpv_create")
        || lower.contains("mpv_initialize")
        || lower.contains("mpv_render_context_create")
        || lower.contains("missing symbol")
        || lower.contains("player layer")
        || lower.contains("avfoundation"))
}

#[must_use]
pub fn nickname_initial(nickname: &str) -> String {
    nickname
        .chars()
        .next()
        .map_or_else(|| "剧".to_owned(), |c| c.to_string())
}
