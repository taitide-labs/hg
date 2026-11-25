use hg_contracts::ids::StreamProfile;
use hg_contracts::viewing::{AppAppearance, PlaybackPreference};
use nsh_core::prelude::Appearance;

pub const SIDEBAR_WIDTH_MIN: u16 = 160;
pub const SIDEBAR_WIDTH_MAX: u16 = 320;

pub const RATE_PRESETS: &[(u16, &str)] = &[
    (75, "0.75×"),
    (100, "1.0×"),
    (125, "1.25×"),
    (150, "1.5×"),
    (200, "2.0×"),
];

pub const PROFILE_OPTIONS: &[(StreamProfile, &str)] = &[
    (StreamProfile::Hevc, "HEVC 高清"),
    (StreamProfile::Main, "标准兼容"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppearanceOption {
    System,
    Light,
    #[default]
    Dark,
}

impl From<AppearanceOption> for Appearance {
    fn from(option: AppearanceOption) -> Self {
        match option {
            AppearanceOption::System => Self::System,
            AppearanceOption::Light => Self::Light,
            AppearanceOption::Dark => Self::Dark,
        }
    }
}

impl From<AppAppearance> for AppearanceOption {
    fn from(value: AppAppearance) -> Self {
        match value {
            AppAppearance::System => Self::System,
            AppAppearance::Light => Self::Light,
            AppAppearance::Dark => Self::Dark,
        }
    }
}

impl From<AppearanceOption> for AppAppearance {
    fn from(option: AppearanceOption) -> Self {
        match option {
            AppearanceOption::System => Self::System,
            AppearanceOption::Light => Self::Light,
            AppearanceOption::Dark => Self::Dark,
        }
    }
}

#[must_use]
pub const fn normalize_rate_hundredths(rate_hundredths: u16) -> u16 {
    match rate_hundredths {
        75 | 100 | 125 | 150 | 200 => rate_hundredths,
        _ => 100,
    }
}

#[must_use]
pub const fn clamp_sidebar_width(width: u16) -> u16 {
    if width < SIDEBAR_WIDTH_MIN {
        SIDEBAR_WIDTH_MIN
    } else if width > SIDEBAR_WIDTH_MAX {
        SIDEBAR_WIDTH_MAX
    } else {
        width
    }
}

#[must_use]
pub const fn with_preferred_profile(
    current: PlaybackPreference,
    preferred_profile: StreamProfile,
) -> PlaybackPreference {
    PlaybackPreference {
        preferred_profile,
        ..current
    }
}

#[must_use]
pub const fn with_default_rate(
    current: PlaybackPreference,
    rate_hundredths: u16,
) -> PlaybackPreference {
    PlaybackPreference {
        default_rate_hundredths: normalize_rate_hundredths(rate_hundredths),
        ..current
    }
}

#[must_use]
pub const fn with_appearance(
    current: PlaybackPreference,
    appearance: AppAppearance,
) -> PlaybackPreference {
    PlaybackPreference {
        appearance,
        ..current
    }
}

#[must_use]
pub const fn with_sidebar(
    current: PlaybackPreference,
    sidebar_collapsed: bool,
    sidebar_width: u16,
) -> PlaybackPreference {
    PlaybackPreference {
        sidebar_collapsed,
        sidebar_width: clamp_sidebar_width(sidebar_width),
        ..current
    }
}
