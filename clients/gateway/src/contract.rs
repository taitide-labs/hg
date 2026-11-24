use std::future::Future;
use std::pin::Pin;

use hg_contracts::catalog::{
    Category, CommentReply, EpisodeComment, HomeFeed, HotKeyword, HotTag,
    ListCommentRepliesRequest, ListEpisodeCommentsRequest, ListRankingRequest, ListSeriesRequest,
    PagedResult, RankingEntry, SearchSeriesRequest, SearchSuggestion, SeriesDetail, SeriesSummary,
    TagGroup,
};
use hg_contracts::error::ContractError;
use hg_contracts::ids::{EpisodeKey, SeriesId};
use hg_contracts::offline::{
    ControlAllDownloadsAction, ControlDownloadRequest, DownloadPolicy, DownloadTaskView,
    EnqueueEpisodeDownloadsRequest, EnqueueEpisodeDownloadsResult, OfflineSeriesView,
};
use hg_contracts::viewing::{
    PlaybackPreference, PlaybackSource, ReportWatchProgressRequest, ReportWatchProgressResult,
    SeriesBookmark, StorageOverview, WatchHistoryEntry,
};

pub type GatewayFuture<T> =
    Pin<Box<dyn Future<Output = Result<T, ContractError>> + Send + 'static>>;

pub trait AppGateway: Send + Sync {
    fn home_feed(&self) -> GatewayFuture<HomeFeed>;
    fn categories(&self) -> GatewayFuture<Vec<Category>>;
    fn tag_groups(&self) -> GatewayFuture<Vec<TagGroup>>;
    fn list_series(&self, request: ListSeriesRequest) -> GatewayFuture<PagedResult<SeriesSummary>>;
    fn list_ranking(&self, request: ListRankingRequest)
    -> GatewayFuture<PagedResult<RankingEntry>>;
    fn search_series(
        &self,
        request: SearchSeriesRequest,
    ) -> GatewayFuture<PagedResult<SeriesSummary>>;
    fn search_suggestions(&self, keyword: String) -> GatewayFuture<SearchSuggestion>;
    fn hot_keywords(&self) -> GatewayFuture<Vec<HotKeyword>>;
    fn hot_tags(&self) -> GatewayFuture<Vec<HotTag>>;
    fn series_detail(&self, series_id: SeriesId) -> GatewayFuture<SeriesDetail>;
    fn list_episode_comments(
        &self,
        request: ListEpisodeCommentsRequest,
    ) -> GatewayFuture<PagedResult<EpisodeComment>>;
    fn list_comment_replies(
        &self,
        request: ListCommentRepliesRequest,
    ) -> GatewayFuture<PagedResult<CommentReply>>;
    fn clear_catalog_cache(&self) -> GatewayFuture<()>;

    fn prepare_episode_playback(&self, key: EpisodeKey) -> GatewayFuture<PlaybackSource>;
    fn refresh_online_playback(&self, key: EpisodeKey) -> GatewayFuture<PlaybackSource>;
    fn report_watch_progress(
        &self,
        request: ReportWatchProgressRequest,
    ) -> GatewayFuture<ReportWatchProgressResult>;
    fn watch_history(&self) -> GatewayFuture<Vec<WatchHistoryEntry>>;
    fn remove_watch_history(&self, series_id: SeriesId) -> GatewayFuture<()>;
    fn clear_watch_history(&self) -> GatewayFuture<()>;
    fn bookmarks(&self) -> GatewayFuture<Vec<SeriesBookmark>>;
    fn is_bookmarked(&self, series_id: SeriesId) -> GatewayFuture<bool>;
    fn bookmark_series(&self, series_id: SeriesId) -> GatewayFuture<()>;
    fn unbookmark_series(&self, series_id: SeriesId) -> GatewayFuture<()>;
    fn playback_preference(&self) -> GatewayFuture<PlaybackPreference>;
    fn update_playback_preference(&self, preference: PlaybackPreference) -> GatewayFuture<()>;
    fn clear_playback_cache(&self) -> GatewayFuture<()>;
    fn storage_overview(&self) -> GatewayFuture<StorageOverview>;

    fn offline_library(&self) -> GatewayFuture<Vec<OfflineSeriesView>>;
    fn download_tasks(&self) -> GatewayFuture<Vec<DownloadTaskView>>;
    fn enqueue_episode_downloads(
        &self,
        request: EnqueueEpisodeDownloadsRequest,
    ) -> GatewayFuture<EnqueueEpisodeDownloadsResult>;
    fn control_download(&self, request: ControlDownloadRequest) -> GatewayFuture<()>;
    fn control_all_downloads(&self, action: ControlAllDownloadsAction) -> GatewayFuture<()>;
    fn delete_offline_episode(&self, key: EpisodeKey) -> GatewayFuture<()>;
    fn delete_offline_series(&self, series_id: SeriesId) -> GatewayFuture<()>;
    fn download_policy(&self) -> GatewayFuture<DownloadPolicy>;
    fn update_download_policy(&self, policy: DownloadPolicy) -> GatewayFuture<()>;
}
