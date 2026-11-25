use hg_contracts::offline::DownloadPolicy;
use hg_contracts::viewing::{DislikedSeriesEntry, PlaybackPreference, StorageOverview};
use nsh_core::prelude::*;

#[derive(Debug, Clone, Default)]
pub struct SettingsState {
    pub playback_pref: Option<PlaybackPreference>,
    pub download_policy: Option<DownloadPolicy>,
    pub storage: Option<StorageOverview>,
    pub disliked: Vec<DislikedSeriesEntry>,
    pub disliked_loading: bool,
    pub disliked_error: Option<String>,
    pub disliked_drawer_open: bool,
    pub error: Option<String>,
}

#[must_use]
pub fn state() -> Signal<SettingsState> {
    global::<SettingsState>()
}
