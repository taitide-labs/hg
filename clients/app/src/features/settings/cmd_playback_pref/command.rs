use hg_contracts::ids::StreamProfile;
use hg_contracts::viewing::PlaybackPreference;
use nsh_core::prelude::Appearance;

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
    #[default]
    System,
    Light,
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

#[must_use]
pub const fn normalize_rate_hundredths(rate_hundredths: u16) -> u16 {
    match rate_hundredths {
        75 | 100 | 125 | 150 | 200 => rate_hundredths,
        _ => 100,
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
