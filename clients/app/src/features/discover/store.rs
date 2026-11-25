use std::collections::{BTreeSet, HashMap};

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
    Latest,
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

pub(crate) fn home_state() -> Signal<DiscoverHomeState> {
    global::<DiscoverHomeState>()
}

pub(crate) fn browse_meta_state() -> Signal<BrowseMetaState> {
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
pub fn browse_meta_untracked() -> BrowseMetaState {
    browse_meta_state().with_untracked(Clone::clone)
}

#[must_use]
pub fn search_input_signal() -> Signal<String> {
    global::<String>()
}

#[must_use]
pub fn ranking_tab_signal() -> Signal<RankingTab> {
    global::<RankingTab>()
}

#[derive(Debug, Clone, Copy)]
struct DiscoverViewport {
    width: f32,
    height: f32,
}

impl Default for DiscoverViewport {
    fn default() -> Self {
        if crate::ui::platform::is_desktop() {
            Self {
                width: 1240.0,
                height: 720.0,
            }
        } else {
            Self {
                width: 390.0,
                height: 844.0,
            }
        }
    }
}

fn viewport_state() -> Signal<DiscoverViewport> {
    global::<DiscoverViewport>()
}

pub fn set_viewport_size(width: f32, height: f32) -> bool {
    if width <= 0.0 || height <= 0.0 {
        return false;
    }
    let changed = viewport_state()
        .with_untracked(|v| (v.width - width).abs() > 1.0 || (v.height - height).abs() > 1.0);
    if changed {
        viewport_state().set(DiscoverViewport { width, height });
    }
    changed
}

#[must_use]
pub fn viewport_size_untracked() -> (f32, f32) {
    viewport_state().with_untracked(|v| (v.width, v.height))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SeriesWatchStatus {
    #[default]
    Unwatched,
    AllWatched,
    UpdatedAfterWatching,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchedSeriesSnapshot {
    pub last_episode: u32,
    pub recorded_total_episodes: u32,
    pub episode_over_80: bool,
}

#[derive(Debug, Clone, Default)]
struct WatchedSeriesMap(HashMap<SeriesId, WatchedSeriesSnapshot>);

fn watched_series_state() -> Signal<WatchedSeriesMap> {
    global::<WatchedSeriesMap>()
}

#[must_use]
pub fn series_watch_status(series_id: SeriesId, current_episode_count: u32) -> SeriesWatchStatus {
    watched_series_state().with(|map| {
        let Some(entry) = map.0.get(&series_id).copied() else {
            return SeriesWatchStatus::Unwatched;
        };
        if !entry.episode_over_80 || current_episode_count == 0 {
            return SeriesWatchStatus::Unwatched;
        }
        if entry.last_episode >= current_episode_count {
            return SeriesWatchStatus::AllWatched;
        }
        if entry.recorded_total_episodes > 0
            && entry.last_episode >= entry.recorded_total_episodes
            && current_episode_count > entry.last_episode
        {
            return SeriesWatchStatus::UpdatedAfterWatching;
        }
        SeriesWatchStatus::Unwatched
    })
}

pub fn set_watched_series(map: HashMap<SeriesId, WatchedSeriesSnapshot>) {
    watched_series_state().set(WatchedSeriesMap(map));
}

#[derive(Debug, Clone, Default)]
struct DiscoverPreferencesState {
    disliked_ids: BTreeSet<SeriesId>,
    bookmarked_ids: BTreeSet<SeriesId>,
}

fn preferences_state() -> Signal<DiscoverPreferencesState> {
    global::<DiscoverPreferencesState>()
}

pub fn set_disliked_ids(ids: BTreeSet<SeriesId>) {
    preferences_state().update(|p| p.disliked_ids = ids);
}

pub fn add_disliked_id(id: SeriesId) {
    preferences_state().update(|p| {
        p.disliked_ids.insert(id);
    });
}

pub fn remove_disliked_id(id: SeriesId) {
    preferences_state().update(|p| {
        p.disliked_ids.remove(&id);
    });
}

#[must_use]
pub fn is_disliked(id: SeriesId) -> bool {
    preferences_state().with(|p| p.disliked_ids.contains(&id))
}

#[must_use]
pub fn is_disliked_untracked(id: SeriesId) -> bool {
    preferences_state().with_untracked(|p| p.disliked_ids.contains(&id))
}

pub fn set_bookmarked_ids(ids: BTreeSet<SeriesId>) {
    preferences_state().update(|p| p.bookmarked_ids = ids);
}

#[must_use]
pub fn is_bookmarked_untracked(id: SeriesId) -> bool {
    preferences_state().with_untracked(|p| p.bookmarked_ids.contains(&id))
}

#[must_use]
pub fn toggle_bookmarked_local(id: SeriesId) -> bool {
    let mut now_bookmarked = false;
    preferences_state().update(|p| {
        if p.bookmarked_ids.remove(&id) {
            now_bookmarked = false;
        } else {
            p.bookmarked_ids.insert(id);
            now_bookmarked = true;
        }
    });
    now_bookmarked
}

pub fn set_bookmarked_local(id: SeriesId, bookmarked: bool) {
    preferences_state().update(|p| {
        if bookmarked {
            p.bookmarked_ids.insert(id);
        } else {
            p.bookmarked_ids.remove(&id);
        }
    });
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

#[must_use]
pub fn home_latest_items(limit: usize) -> Vec<SeriesSummary> {
    home_state().with(|s| {
        s.feed
            .as_ref()
            .map(|f| {
                f.latest
                    .iter()
                    .filter(|item| !is_disliked(item.id))
                    .take(limit)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    })
}

#[must_use]
pub fn home_ranking_items(limit: usize) -> Vec<SeriesSummary> {
    home_state().with(|s| {
        s.rankings
            .iter()
            .filter(|entry| !is_disliked(entry.series.id))
            .take(limit)
            .map(|entry| entry.series.clone())
            .collect()
    })
}

#[must_use]
pub fn home_category_items(category_id: u32, limit: usize) -> Vec<SeriesSummary> {
    home_state().with(|s| {
        s.feed
            .as_ref()
            .and_then(|f| {
                f.category_recommendations
                    .iter()
                    .find(|rec| rec.category_id == category_id)
            })
            .map(|rec| {
                rec.items
                    .iter()
                    .filter(|item| !is_disliked(item.id))
                    .take(limit)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    })
}
