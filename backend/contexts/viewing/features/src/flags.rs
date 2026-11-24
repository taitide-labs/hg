use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewingFeatureFlags {
    pub playback_resolution: bool,
    pub watchlist_and_settings: bool,
}

impl ViewingFeatureFlags {
    #[must_use]
    pub const fn all_enabled() -> Self {
        Self {
            playback_resolution: true,
            watchlist_and_settings: true,
        }
    }
}
