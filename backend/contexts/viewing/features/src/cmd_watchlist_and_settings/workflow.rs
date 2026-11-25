use hg_backend_contracts::catalog::SeriesSnapshot;
use hg_contracts::error::ContractError;
use hg_contracts::ids::{SeriesId, StreamProfile};
use hg_contracts::viewing::{AppAppearance, PlaybackPreference};
use hg_viewing_domain::preference::{MAX_SEARCH_HISTORY_ENTRIES, validate_playback_preference};

#[derive(Debug, Clone, PartialEq)]
pub struct BookmarkWrite {
    pub series_id: SeriesId,
    pub title: String,
    pub cover_url: String,
    pub episode_count: u32,
    pub is_finished: bool,
    pub score: f32,
    pub bookmarked_at_epoch_secs: u64,
}

pub fn check_playback_preference(pref: &PlaybackPreference) -> Result<(), ContractError> {
    validate_playback_preference(pref).map_err(ContractError::from)
}

#[must_use]
pub fn normalize_search_keyword(keyword: &str) -> Option<String> {
    let trimmed = keyword.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

#[must_use]
pub const fn search_history_limit() -> i64 {
    MAX_SEARCH_HISTORY_ENTRIES as i64
}

#[must_use]
pub fn plan_bookmark_series(
    series: &SeriesSnapshot,
    bookmarked_at_epoch_secs: u64,
) -> BookmarkWrite {
    BookmarkWrite {
        series_id: series.series_id,
        title: series.title.clone(),
        cover_url: series.cover_url.clone(),
        episode_count: series.total_episodes,
        is_finished: series.is_finished,
        score: series.score,
        bookmarked_at_epoch_secs,
    }
}

#[must_use]
pub const fn profile_as_str(profile: StreamProfile) -> &'static str {
    match profile {
        StreamProfile::Hevc => "hevc",
        StreamProfile::Main => "main",
    }
}

#[must_use]
pub const fn appearance_as_str(appearance: AppAppearance) -> &'static str {
    match appearance {
        AppAppearance::System => "system",
        AppAppearance::Light => "light",
        AppAppearance::Dark => "dark",
    }
}
