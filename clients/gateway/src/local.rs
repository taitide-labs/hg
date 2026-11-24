use std::future;

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

use crate::contract::{AppGateway, GatewayFuture};

#[derive(Debug, Default)]
pub struct LocalGateway;

impl LocalGateway {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    fn unready<T: Send + 'static>() -> GatewayFuture<T> {
        Box::pin(future::ready(Err(ContractError::Internal {
            detail: "本地运行时适配器尚未接入，当前处于客户端契约预览阶段".to_owned(),
        })))
    }
}

impl AppGateway for LocalGateway {
    fn home_feed(&self) -> GatewayFuture<HomeFeed> {
        Self::unready()
    }

    fn categories(&self) -> GatewayFuture<Vec<Category>> {
        Self::unready()
    }

    fn tag_groups(&self) -> GatewayFuture<Vec<TagGroup>> {
        Self::unready()
    }

    fn list_series(
        &self,
        _request: ListSeriesRequest,
    ) -> GatewayFuture<PagedResult<SeriesSummary>> {
        Self::unready()
    }

    fn list_ranking(
        &self,
        _request: ListRankingRequest,
    ) -> GatewayFuture<PagedResult<RankingEntry>> {
        Self::unready()
    }

    fn search_series(
        &self,
        _request: SearchSeriesRequest,
    ) -> GatewayFuture<PagedResult<SeriesSummary>> {
        Self::unready()
    }

    fn search_suggestions(&self, _keyword: String) -> GatewayFuture<SearchSuggestion> {
        Self::unready()
    }

    fn hot_keywords(&self) -> GatewayFuture<Vec<HotKeyword>> {
        Self::unready()
    }

    fn hot_tags(&self) -> GatewayFuture<Vec<HotTag>> {
        Self::unready()
    }

    fn series_detail(&self, _series_id: SeriesId) -> GatewayFuture<SeriesDetail> {
        Self::unready()
    }

    fn list_episode_comments(
        &self,
        _request: ListEpisodeCommentsRequest,
    ) -> GatewayFuture<PagedResult<EpisodeComment>> {
        Self::unready()
    }

    fn list_comment_replies(
        &self,
        _request: ListCommentRepliesRequest,
    ) -> GatewayFuture<PagedResult<CommentReply>> {
        Self::unready()
    }

    fn clear_catalog_cache(&self) -> GatewayFuture<()> {
        Self::unready()
    }

    fn prepare_episode_playback(&self, _key: EpisodeKey) -> GatewayFuture<PlaybackSource> {
        Self::unready()
    }

    fn refresh_online_playback(&self, _key: EpisodeKey) -> GatewayFuture<PlaybackSource> {
        Self::unready()
    }

    fn report_watch_progress(
        &self,
        _request: ReportWatchProgressRequest,
    ) -> GatewayFuture<ReportWatchProgressResult> {
        Self::unready()
    }

    fn watch_history(&self) -> GatewayFuture<Vec<WatchHistoryEntry>> {
        Self::unready()
    }

    fn remove_watch_history(&self, _series_id: SeriesId) -> GatewayFuture<()> {
        Self::unready()
    }

    fn clear_watch_history(&self) -> GatewayFuture<()> {
        Self::unready()
    }

    fn bookmarks(&self) -> GatewayFuture<Vec<SeriesBookmark>> {
        Self::unready()
    }

    fn is_bookmarked(&self, _series_id: SeriesId) -> GatewayFuture<bool> {
        Self::unready()
    }

    fn bookmark_series(&self, _series_id: SeriesId) -> GatewayFuture<()> {
        Self::unready()
    }

    fn unbookmark_series(&self, _series_id: SeriesId) -> GatewayFuture<()> {
        Self::unready()
    }

    fn playback_preference(&self) -> GatewayFuture<PlaybackPreference> {
        Self::unready()
    }

    fn update_playback_preference(&self, _preference: PlaybackPreference) -> GatewayFuture<()> {
        Self::unready()
    }

    fn clear_playback_cache(&self) -> GatewayFuture<()> {
        Self::unready()
    }

    fn storage_overview(&self) -> GatewayFuture<StorageOverview> {
        Self::unready()
    }

    fn offline_library(&self) -> GatewayFuture<Vec<OfflineSeriesView>> {
        Self::unready()
    }

    fn download_tasks(&self) -> GatewayFuture<Vec<DownloadTaskView>> {
        Self::unready()
    }

    fn enqueue_episode_downloads(
        &self,
        _request: EnqueueEpisodeDownloadsRequest,
    ) -> GatewayFuture<EnqueueEpisodeDownloadsResult> {
        Self::unready()
    }

    fn control_download(&self, _request: ControlDownloadRequest) -> GatewayFuture<()> {
        Self::unready()
    }

    fn control_all_downloads(&self, _action: ControlAllDownloadsAction) -> GatewayFuture<()> {
        Self::unready()
    }

    fn delete_offline_episode(&self, _key: EpisodeKey) -> GatewayFuture<()> {
        Self::unready()
    }

    fn delete_offline_series(&self, _series_id: SeriesId) -> GatewayFuture<()> {
        Self::unready()
    }

    fn download_policy(&self) -> GatewayFuture<DownloadPolicy> {
        Self::unready()
    }

    fn update_download_policy(&self, _policy: DownloadPolicy) -> GatewayFuture<()> {
        Self::unready()
    }
}
