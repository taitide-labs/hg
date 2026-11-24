use hg_contracts::ids::{EpisodeNumber, SeriesId};
use nsh_core::prelude::*;

use crate::ui::platform;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrimaryTab {
    #[default]
    Discover,
    Watchlist,
    Offline,
    Settings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoverSubPage {
    Featured,
    Ranking,
    Category {
        id: u32,
        name: String,
        route: String,
    },
    Search {
        query: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerRoute {
    pub series_id: SeriesId,
    pub initial_episode: Option<EpisodeNumber>,
}

#[derive(Debug, Clone, Default)]
struct NavState {
    tab: PrimaryTab,
    discover_subpage: Option<DiscoverSubPage>,
    player: Option<PlayerRoute>,
}

#[derive(Debug, Clone, Copy, Default)]
struct ChromeFullscreen(bool);

fn state() -> Signal<NavState> {
    global::<NavState>()
}

fn fullscreen_signal() -> Signal<ChromeFullscreen> {
    global::<ChromeFullscreen>()
}

#[must_use]
pub fn current_tab() -> PrimaryTab {
    state().with(|s| s.tab)
}

#[must_use]
pub fn active_discover_subpage() -> Option<DiscoverSubPage> {
    state().with(|s| s.discover_subpage.clone())
}

#[must_use]
pub fn active_player() -> Option<PlayerRoute> {
    state().with(|s| s.player)
}

#[must_use]
pub fn is_chrome_fullscreen() -> bool {
    fullscreen_signal().with(|f| f.0)
}

pub fn select_tab(tab: PrimaryTab) {
    set_chrome_fullscreen(false);
    state().update(|s| {
        s.tab = tab;
        s.discover_subpage = None;
        s.player = None;
    });
}

pub fn open_discover_subpage(subpage: DiscoverSubPage) {
    set_chrome_fullscreen(false);
    state().update(|s| {
        s.tab = PrimaryTab::Discover;
        s.discover_subpage = Some(subpage);
        s.player = None;
    });
}

pub fn close_discover_subpage() {
    state().update(|s| {
        s.discover_subpage = None;
    });
}

pub fn open_player(series_id: SeriesId, initial_episode: Option<EpisodeNumber>) {
    set_chrome_fullscreen(false);
    state().update(|s| {
        s.player = Some(PlayerRoute {
            series_id,
            initial_episode,
        });
    });
}

pub fn set_chrome_fullscreen(fullscreen: bool) {
    let sig = fullscreen_signal();
    if sig.with_untracked(|f| f.0) != fullscreen {
        sig.set(ChromeFullscreen(fullscreen));
        platform::set_window_fullscreen(fullscreen);
    }
}

pub fn close_player() {
    set_chrome_fullscreen(false);
    state().update(|s| {
        s.player = None;
    });
}
