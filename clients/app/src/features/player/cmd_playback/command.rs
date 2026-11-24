use std::num::NonZeroU32;

use hg_contracts::ids::{EpisodeNumber, StreamProfile};
use hg_contracts::viewing::{OnlineStreamKind, PlaybackOrigin};

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
            Self::Fill => "铺满填充",
            Self::Cinema16x9 => "16:9 剧场",
            Self::Portrait9x16 => "9:16 竖屏",
        }
    }

    #[must_use]
    pub const fn key_hint(self) -> &'static str {
        match self {
            Self::Original => "比例 1",
            Self::Fill => "比例 2",
            Self::Cinema16x9 => "比例 3",
            Self::Portrait9x16 => "比例 4",
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
pub const fn origin_badge_label(origin: PlaybackOrigin) -> &'static str {
    match origin {
        PlaybackOrigin::OfflineLocal {
            profile: StreamProfile::Hevc,
        } => "本地离线 · HEVC",
        PlaybackOrigin::OfflineLocal {
            profile: StreamProfile::Main,
        } => "本地离线 · H.264",
        PlaybackOrigin::OnlineStream {
            kind: OnlineStreamKind::Hevc,
        } => "在线流 · HEVC",
        PlaybackOrigin::OnlineStream {
            kind: OnlineStreamKind::Main,
        } => "在线流 · H.264",
        PlaybackOrigin::OnlineStream {
            kind: OnlineStreamKind::Preview,
        } => "在线预览流",
    }
}

#[must_use]
pub fn format_time(seconds: u32) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{mins:02}:{secs:02}")
}
