use hg_contracts::ids::{EpisodeNumber, SeriesId};
use nsh_core::prelude::*;

use crate::ui::platform;

pub const SIDEBAR_MIN_WIDTH: f32 = 160.0;
pub const SIDEBAR_MAX_WIDTH: f32 = 320.0;
pub const SIDEBAR_DEFAULT_WIDTH: f32 = 208.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrimaryTab {
    #[default]
    Discover,
    Search,
    Watchlist,
    History,
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

#[derive(Debug, Clone, Copy)]
struct SidebarState {
    collapsed: bool,
    width: f32,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self {
            collapsed: false,
            width: SIDEBAR_DEFAULT_WIDTH,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct ChromeFullscreen(bool);

fn state() -> Signal<NavState> {
    global::<NavState>()
}

fn sidebar_state() -> Signal<SidebarState> {
    global::<SidebarState>()
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

#[must_use]
pub fn sidebar_collapsed() -> bool {
    sidebar_state().with(|s| s.collapsed)
}

#[must_use]
pub fn sidebar_width() -> f32 {
    sidebar_state().with(|s| s.width.clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH))
}

pub fn hydrate_sidebar(collapsed: bool, width: f32) {
    let clamped = width.clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH);
    sidebar_state().update(|s| {
        s.collapsed = collapsed;
        s.width = clamped;
    });
}

pub fn toggle_sidebar_collapsed() -> bool {
    let mut next = false;
    sidebar_state().update(|s| {
        s.collapsed = !s.collapsed;
        next = s.collapsed;
    });
    next
}

pub fn set_sidebar_width(width: f32) {
    let clamped = width.clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH);
    sidebar_state().update(|s| {
        if (s.width - clamped).abs() > 0.1 {
            s.width = clamped;
        }
    });
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
