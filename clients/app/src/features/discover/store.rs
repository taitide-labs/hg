use hg_contracts::catalog::{
    Category, HomeFeed, HotKeyword, RankingEntry, RankingKind, SeriesSummary, TagGroup,
};
use nsh_core::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RankingTab {
    #[default]
    Hot,
    Potential,
    Recommend,
}

impl From<RankingKind> for RankingTab {
    fn from(kind: RankingKind) -> Self {
        match kind {
            RankingKind::Hot => Self::Hot,
            RankingKind::Potential => Self::Potential,
            RankingKind::Recommend => Self::Recommend,
        }
    }
}

impl From<RankingTab> for RankingKind {
    fn from(tab: RankingTab) -> Self {
        match tab {
            RankingTab::Hot => Self::Hot,
            RankingTab::Potential => Self::Potential,
            RankingTab::Recommend => Self::Recommend,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BrowseKind {
    #[default]
    Featured,
    Ranking(RankingKind),
    Category {
        id: u32,
        name: String,
        route: String,
    },
    Tag {
        slug: String,
        name: String,
    },
    Search {
        query: String,
    },
}

#[derive(Debug, Clone, Default)]
pub struct DiscoverHomeState {
    pub feed: Option<HomeFeed>,
    pub rankings: Vec<RankingEntry>,
    pub categories: Vec<Category>,
    pub loading: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct BrowseMetaState {
    pub tag_groups: Vec<TagGroup>,
    pub hot_keywords: Vec<HotKeyword>,
}

#[derive(Debug, Clone, Default)]
pub struct DiscoverBrowseState {
    pub kind: BrowseKind,
    pub items: Vec<SeriesSummary>,
    pub ranking_items: Vec<RankingEntry>,
    pub loading: bool,
    pub error: Option<String>,
}

fn home_state() -> Signal<DiscoverHomeState> {
    global::<DiscoverHomeState>()
}

fn browse_meta_state() -> Signal<BrowseMetaState> {
    global::<BrowseMetaState>()
}

fn browse_state() -> Signal<DiscoverBrowseState> {
    global::<DiscoverBrowseState>()
}

#[must_use]
pub fn home_snapshot() -> DiscoverHomeState {
    home_state().get()
}

#[must_use]
pub fn browse_meta_snapshot() -> BrowseMetaState {
    browse_meta_state().get()
}

#[must_use]
pub fn browse_snapshot() -> DiscoverBrowseState {
    browse_state().get()
}

#[must_use]
pub fn search_input_signal() -> Signal<String> {
    global::<String>()
}

#[must_use]
pub fn ranking_tab_signal() -> Signal<RankingTab> {
    global::<RankingTab>()
}

pub fn set_home_loading() {
    home_state().update(|s| {
        s.loading = true;
        s.error = None;
    });
}

pub fn set_home_loaded(feed: HomeFeed, rankings: Vec<RankingEntry>, categories: Vec<Category>) {
    home_state().update(|s| {
        s.feed = Some(feed);
        s.rankings = rankings;
        s.categories = categories;
        s.loading = false;
        s.error = None;
    });
}

pub fn set_home_error(message: String) {
    home_state().update(|s| {
        s.loading = false;
        s.error = Some(message);
    });
}

pub fn set_browse_loading(kind: BrowseKind) {
    if let BrowseKind::Ranking(rk) = &kind {
        ranking_tab_signal().set(RankingTab::from(*rk));
    }
    browse_state().update(|s| {
        s.kind = kind;
        s.loading = true;
        s.error = None;
    });
}

pub fn set_browse_series(items: Vec<SeriesSummary>) {
    browse_state().update(|s| {
        s.items = items;
        s.ranking_items.clear();
        s.loading = false;
        s.error = None;
    });
}

pub fn set_browse_rankings(items: Vec<RankingEntry>) {
    browse_state().update(|s| {
        s.ranking_items = items;
        s.items.clear();
        s.loading = false;
        s.error = None;
    });
}

pub fn set_browse_meta(tag_groups: Vec<TagGroup>, hot_keywords: Vec<HotKeyword>) {
    browse_meta_state().update(|s| {
        s.tag_groups = tag_groups;
        s.hot_keywords = hot_keywords;
    });
}

pub fn set_browse_error(message: String) {
    browse_state().update(|s| {
        s.loading = false;
        s.error = Some(message);
    });
}
