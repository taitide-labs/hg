use hg_contracts::ids::{EpisodeNumber, SeriesId};
use nsh_core::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrimaryTab {
    #[default]
    Discover,
    Watchlist,
    Offline,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerRoute {
    pub series_id: SeriesId,
    pub initial_episode: Option<EpisodeNumber>,
}

#[derive(Debug, Clone, Default)]
struct NavState {
    tab: PrimaryTab,
    player: Option<PlayerRoute>,
}

fn state() -> Signal<NavState> {
    global::<NavState>()
}

#[must_use]
pub fn current_tab() -> PrimaryTab {
    state().with(|s| s.tab)
}

#[must_use]
pub fn active_player() -> Option<PlayerRoute> {
    state().with(|s| s.player)
}

pub fn select_tab(tab: PrimaryTab) {
    state().update(|s| {
        s.tab = tab;
        s.player = None;
    });
}

pub fn open_player(series_id: SeriesId, initial_episode: Option<EpisodeNumber>) {
    state().update(|s| {
        s.player = Some(PlayerRoute {
            series_id,
            initial_episode,
        });
    });
}

pub fn close_player() {
    state().update(|s| {
        s.player = None;
    });
}
