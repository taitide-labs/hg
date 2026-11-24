use hg_contracts::catalog::{Category, HomeFeed, SeriesSummary};
use nsh_core::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CategoryTab {
    #[default]
    All,
    Drama,
    Anime,
    Suspense,
    Costume,
}

impl CategoryTab {
    #[must_use]
    pub const fn route(self) -> Option<&'static str> {
        match self {
            Self::All => None,
            Self::Drama => Some("duanju"),
            Self::Anime => Some("manju"),
            Self::Suspense => Some("xuanyi"),
            Self::Costume => Some("gufeng"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DiscoverState {
    pub feed: Option<HomeFeed>,
    pub categories: Vec<Category>,
    pub active_category: CategoryTab,
    pub items: Vec<SeriesSummary>,
    pub loading: bool,
    pub error: Option<String>,
}

fn state() -> Signal<DiscoverState> {
    global::<DiscoverState>()
}

#[must_use]
pub fn snapshot() -> DiscoverState {
    state().get()
}

#[must_use]
pub fn active_category_signal() -> Signal<CategoryTab> {
    global::<CategoryTab>()
}

pub fn set_loading() {
    state().update(|s| {
        s.loading = true;
        s.error = None;
    });
}

pub fn set_home(feed: HomeFeed, categories: Vec<Category>) {
    let items = feed.featured.clone();
    state().update(|s| {
        s.feed = Some(feed);
        s.categories = categories;
        s.items = items;
        s.loading = false;
        s.error = None;
    });
}

pub fn set_filtered(category: CategoryTab, items: Vec<SeriesSummary>) {
    global::<CategoryTab>().set(category);
    state().update(|s| {
        s.active_category = category;
        s.items = items;
        s.loading = false;
        s.error = None;
    });
}

pub fn set_error(message: String) {
    state().update(|s| {
        s.loading = false;
        s.error = Some(message);
    });
}
