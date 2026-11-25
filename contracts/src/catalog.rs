use serde::{Deserialize, Serialize};

use crate::ids::{CommentId, EpisodeNumber, PageInfo, PageRequest, SeriesId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SeriesSort {
    New,
    Hot,
    HotWindow,
    Random,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RankingKind {
    Hot,
    Potential,
    Recommend,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeriesSummary {
    pub id: SeriesId,
    pub title: String,
    pub cover_url: String,
    pub score: f32,
    pub tags: Vec<String>,
    pub description: String,
    pub episode_count: u32,
    pub is_finished: bool,
    pub hot: u64,
    #[serde(default)]
    pub is_original: bool,
    #[serde(default)]
    pub is_hot: bool,
    #[serde(default)]
    pub is_new: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeBanner {
    pub series_id: SeriesId,
    pub title: String,
    pub image_url: String,
    pub score: f32,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CategoryRecommendation {
    pub category_id: u32,
    pub category_name: String,
    pub items: Vec<SeriesSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeFeed {
    pub banners: Vec<HomeBanner>,
    pub featured: Vec<SeriesSummary>,
    #[serde(default)]
    pub latest: Vec<SeriesSummary>,
    pub category_recommendations: Vec<CategoryRecommendation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeSnapshot {
    pub feed: HomeFeed,
    pub categories: Vec<Category>,
    pub rankings: Vec<RankingEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Category {
    pub id: u32,
    pub name: String,
    pub route: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagItem {
    pub id: u32,
    pub name: String,
    pub slug: String,
    pub is_hot: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagGroup {
    pub id: u32,
    pub code: String,
    pub name: String,
    pub tags: Vec<TagItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RankingEntry {
    pub rank: u32,
    pub series: SeriesSummary,
    pub metric_label: String,
    pub metric_value: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchSuggestion {
    pub series: Vec<SeriesSummary>,
    pub phrases: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotKeyword {
    pub word: String,
    pub heat: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotTag {
    pub name: String,
    pub heat: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpisodeSummary {
    pub ep_num: EpisodeNumber,
    pub title: String,
    pub duration_secs: u32,
    pub playable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeriesStats {
    pub play_heat: u64,
    pub likes: u64,
    pub bookmarks: u64,
    pub comments: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeriesDetail {
    pub summary: SeriesSummary,
    pub total_episodes: u32,
    pub stats: SeriesStats,
    pub episodes: Vec<EpisodeSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentReply {
    pub id: CommentId,
    pub nickname: String,
    pub avatar_color: String,
    pub text: String,
    pub relative_time: String,
    pub like_count: u32,
    pub reply_to_nickname: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpisodeComment {
    pub id: CommentId,
    pub nickname: String,
    pub avatar_color: String,
    pub text: String,
    pub relative_time: String,
    pub like_count: u32,
    pub is_pinned: bool,
    pub ip_region: Option<String>,
    pub reply_count: u32,
    pub preview_replies: Vec<CommentReply>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PagedResult<T> {
    pub items: Vec<T>,
    pub pagination: PageInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeriesFilter {
    All { category_id: Option<u32> },
    CategoryRoute { route: String },
    TagSlug { slug: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListSeriesRequest {
    pub filter: SeriesFilter,
    pub sort: SeriesSort,
    pub page: PageRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListRankingRequest {
    pub kind: RankingKind,
    pub page: PageRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchSeriesRequest {
    pub keyword: String,
    pub page: PageRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListEpisodeCommentsRequest {
    pub series_id: SeriesId,
    pub ep_num: EpisodeNumber,
    pub page: PageRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListCommentRepliesRequest {
    pub series_id: SeriesId,
    pub comment_id: CommentId,
    pub page: PageRequest,
}
