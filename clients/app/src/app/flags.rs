use nsh_core::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FeatureFlags {
    pub watchlist_page: bool,
    pub offline_library_page: bool,
    pub settings_page: bool,
    pub player_enhancements: bool,
}

fn slot() -> Signal<FeatureFlags> {
    global::<FeatureFlags>()
}

#[must_use]
pub fn watchlist_enabled() -> bool {
    slot().with(|f| f.watchlist_page)
}

#[must_use]
pub fn offline_enabled() -> bool {
    slot().with(|f| f.offline_library_page)
}

#[must_use]
pub fn settings_enabled() -> bool {
    slot().with(|f| f.settings_page)
}

#[must_use]
pub fn player_enhancements_enabled() -> bool {
    slot().with(|f| f.player_enhancements)
}
