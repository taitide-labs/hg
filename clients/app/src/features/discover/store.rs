use std::collections::HashSet;

use hg_contracts::catalog::{
    Category, HomeFeed, HotKeyword, RankingEntry, RankingKind, SeriesSort, SeriesSummary, TagGroup,
};
use hg_contracts::ids::{PageInfo, SeriesId};
use hg_contracts::viewing::SearchHistoryEntry;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchTarget {
    Keyword(String),
    Tag { slug: String, name: String },
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

#[derive(Debug, Clone)]
pub struct DiscoverBrowseState {
    pub kind: BrowseKind,
    pub sort: SeriesSort,
    pub items: Vec<SeriesSummary>,
    pub ranking_items: Vec<RankingEntry>,
    pub pagination: Option<PageInfo>,
    pub loading: bool,
    pub loading_more: bool,
    pub error: Option<String>,
    pub load_more_error: Option<String>,
    pub request_seq: u64,
}

impl Default for DiscoverBrowseState {
    fn default() -> Self {
        Self {
            kind: BrowseKind::default(),
            sort: SeriesSort::Hot,
            items: Vec::new(),
            ranking_items: Vec::new(),
            pagination: None,
            loading: false,
            loading_more: false,
            error: None,
            load_more_error: None,
            request_seq: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiscoverSearchState {
    pub target: Option<SearchTarget>,
    pub sort: SeriesSort,
    pub items: Vec<SeriesSummary>,
    pub pagination: Option<PageInfo>,
    pub history: Vec<SearchHistoryEntry>,
    pub loading: bool,
    pub loading_more: bool,
    pub error: Option<String>,
    pub load_more_error: Option<String>,
    pub request_seq: u64,
}

impl Default for DiscoverSearchState {
    fn default() -> Self {
        Self {
            target: None,
            sort: SeriesSort::Hot,
            items: Vec::new(),
            pagination: None,
            history: Vec::new(),
            loading: false,
            loading_more: false,
            error: None,
            load_more_error: None,
            request_seq: 0,
        }
    }
}

fn home_state() -> Signal<DiscoverHomeState> {
    global::<DiscoverHomeState>()
}

fn browse_meta_state() -> Signal<BrowseMetaState> {
    global::<BrowseMetaState>()
}

pub(crate) fn browse_state() -> Signal<DiscoverBrowseState> {
    global::<DiscoverBrowseState>()
}

pub(crate) fn search_state() -> Signal<DiscoverSearchState> {
    global::<DiscoverSearchState>()
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
pub fn search_snapshot() -> DiscoverSearchState {
    search_state().get()
}

#[must_use]
pub fn search_input_signal() -> Signal<String> {
    global::<String>()
}

#[must_use]
pub fn ranking_tab_signal() -> Signal<RankingTab> {
    global::<RankingTab>()
}

#[derive(Debug, Clone, Default)]
struct WatchedSeriesSet(HashSet<SeriesId>);

fn watched_series_state() -> Signal<WatchedSeriesSet> {
    global::<WatchedSeriesSet>()
}

#[must_use]
pub fn is_series_watched(series_id: SeriesId) -> bool {
    watched_series_state().with(|s| s.0.contains(&series_id))
}

pub fn set_watched_series(ids: HashSet<SeriesId>) {
    watched_series_state().set(WatchedSeriesSet(ids));
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

pub fn set_browse_meta(tag_groups: Vec<TagGroup>, hot_keywords: Vec<HotKeyword>) {
    browse_meta_state().update(|s| {
        s.tag_groups = tag_groups;
        s.hot_keywords = hot_keywords;
    });
}
