use hg_contracts::offline::DownloadPolicy;
use hg_contracts::viewing::{PlaybackPreference, StorageOverview};
use nsh_core::prelude::*;

#[derive(Debug, Clone, Default)]
pub struct SettingsState {
    pub playback_pref: Option<PlaybackPreference>,
    pub download_policy: Option<DownloadPolicy>,
    pub storage: Option<StorageOverview>,
    pub error: Option<String>,
}

#[must_use]
pub fn state() -> Signal<SettingsState> {
    global::<SettingsState>()
}
