use std::sync::Arc;

use hg_contracts::catalog::{
    Category, CommentReply, EpisodeComment, HomeFeed, HotKeyword, HotTag,
    ListCommentRepliesRequest, ListEpisodeCommentsRequest, ListRankingRequest, ListSeriesRequest,
    PagedResult, RankingEntry, SearchSeriesRequest, SearchSuggestion, SeriesDetail, SeriesSummary,
    TagGroup,
};
use hg_contracts::ids::{EpisodeKey, SeriesId};
use hg_contracts::offline::{
    ControlAllDownloadsAction, ControlDownloadRequest, DownloadPolicy, DownloadTaskView,
    EnqueueEpisodeDownloadsRequest, EnqueueEpisodeDownloadsResult, OfflineSeriesView,
};
use hg_contracts::viewing::{
    PlaybackPreference, PlaybackSource, ReportWatchProgressRequest, ReportWatchProgressResult,
    SearchHistoryEntry, SeriesBookmark, StorageOverview, WatchHistoryEntry,
};
use hg_gateway::contract::{AppGateway, GatewayFuture};
use hg_gateway::mock::MockGateway;
use nsh_engine::{ColorScheme, EngineConfig, Environment, Os, Points, Size};
use nsh_testing::Headless;

use crate::root_with_gateway;

struct EmptyOfflineGateway {
    inner: MockGateway,
    empty_library: bool,
}

impl AppGateway for EmptyOfflineGateway {
    fn home_feed(&self) -> GatewayFuture<HomeFeed> {
        self.inner.home_feed()
    }
    fn categories(&self) -> GatewayFuture<Vec<Category>> {
        self.inner.categories()
    }
    fn tag_groups(&self) -> GatewayFuture<Vec<TagGroup>> {
        self.inner.tag_groups()
    }
    fn list_series(&self, request: ListSeriesRequest) -> GatewayFuture<PagedResult<SeriesSummary>> {
        self.inner.list_series(request)
    }
    fn list_ranking(
        &self,
        request: ListRankingRequest,
    ) -> GatewayFuture<PagedResult<RankingEntry>> {
        self.inner.list_ranking(request)
    }
    fn search_series(
        &self,
        request: SearchSeriesRequest,
    ) -> GatewayFuture<PagedResult<SeriesSummary>> {
        self.inner.search_series(request)
    }
    fn search_suggestions(&self, keyword: String) -> GatewayFuture<SearchSuggestion> {
        self.inner.search_suggestions(keyword)
    }
    fn hot_keywords(&self) -> GatewayFuture<Vec<HotKeyword>> {
        self.inner.hot_keywords()
    }
    fn hot_tags(&self) -> GatewayFuture<Vec<HotTag>> {
        self.inner.hot_tags()
    }
    fn series_detail(&self, series_id: SeriesId) -> GatewayFuture<SeriesDetail> {
        self.inner.series_detail(series_id)
    }
    fn list_episode_comments(
        &self,
        request: ListEpisodeCommentsRequest,
    ) -> GatewayFuture<PagedResult<EpisodeComment>> {
        self.inner.list_episode_comments(request)
    }
    fn list_comment_replies(
        &self,
        request: ListCommentRepliesRequest,
    ) -> GatewayFuture<PagedResult<CommentReply>> {
        self.inner.list_comment_replies(request)
    }
    fn clear_catalog_cache(&self) -> GatewayFuture<()> {
        self.inner.clear_catalog_cache()
    }
    fn prepare_episode_playback(&self, key: EpisodeKey) -> GatewayFuture<PlaybackSource> {
        self.inner.prepare_episode_playback(key)
    }
    fn refresh_online_playback(&self, key: EpisodeKey) -> GatewayFuture<PlaybackSource> {
        self.inner.refresh_online_playback(key)
    }
    fn report_watch_progress(
        &self,
        request: ReportWatchProgressRequest,
    ) -> GatewayFuture<ReportWatchProgressResult> {
        self.inner.report_watch_progress(request)
    }
    fn watch_history(&self) -> GatewayFuture<Vec<WatchHistoryEntry>> {
        self.inner.watch_history()
    }
    fn remove_watch_history(&self, series_id: SeriesId) -> GatewayFuture<()> {
        self.inner.remove_watch_history(series_id)
    }
    fn clear_watch_history(&self) -> GatewayFuture<()> {
        self.inner.clear_watch_history()
    }
    fn bookmarks(&self) -> GatewayFuture<Vec<SeriesBookmark>> {
        self.inner.bookmarks()
    }
    fn is_bookmarked(&self, series_id: SeriesId) -> GatewayFuture<bool> {
        self.inner.is_bookmarked(series_id)
    }
    fn bookmark_series(&self, series_id: SeriesId) -> GatewayFuture<()> {
        self.inner.bookmark_series(series_id)
    }
    fn unbookmark_series(&self, series_id: SeriesId) -> GatewayFuture<()> {
        self.inner.unbookmark_series(series_id)
    }
    fn playback_preference(&self) -> GatewayFuture<PlaybackPreference> {
        self.inner.playback_preference()
    }
    fn update_playback_preference(&self, preference: PlaybackPreference) -> GatewayFuture<()> {
        self.inner.update_playback_preference(preference)
    }
    fn search_history(&self) -> GatewayFuture<Vec<SearchHistoryEntry>> {
        self.inner.search_history()
    }
    fn record_search_history(&self, keyword: String) -> GatewayFuture<()> {
        self.inner.record_search_history(keyword)
    }
    fn remove_search_history(&self, keyword: String) -> GatewayFuture<()> {
        self.inner.remove_search_history(keyword)
    }
    fn clear_search_history(&self) -> GatewayFuture<()> {
        self.inner.clear_search_history()
    }
    fn clear_playback_cache(&self) -> GatewayFuture<()> {
        self.inner.clear_playback_cache()
    }
    fn storage_overview(&self) -> GatewayFuture<StorageOverview> {
        self.inner.storage_overview()
    }
    fn offline_library(&self) -> GatewayFuture<Vec<OfflineSeriesView>> {
        if self.empty_library {
            Box::pin(std::future::ready(Ok(Vec::new())))
        } else {
            self.inner.offline_library()
        }
    }
    fn download_tasks(&self) -> GatewayFuture<Vec<DownloadTaskView>> {
        Box::pin(std::future::ready(Ok(Vec::new())))
    }
    fn enqueue_episode_downloads(
        &self,
        request: EnqueueEpisodeDownloadsRequest,
    ) -> GatewayFuture<EnqueueEpisodeDownloadsResult> {
        self.inner.enqueue_episode_downloads(request)
    }
    fn control_download(&self, request: ControlDownloadRequest) -> GatewayFuture<()> {
        self.inner.control_download(request)
    }
    fn control_all_downloads(&self, action: ControlAllDownloadsAction) -> GatewayFuture<()> {
        self.inner.control_all_downloads(action)
    }
    fn delete_offline_episode(&self, key: EpisodeKey) -> GatewayFuture<()> {
        self.inner.delete_offline_episode(key)
    }
    fn delete_offline_series(&self, series_id: SeriesId) -> GatewayFuture<()> {
        self.inner.delete_offline_series(series_id)
    }
    fn download_policy(&self) -> GatewayFuture<DownloadPolicy> {
        self.inner.download_policy()
    }
    fn update_download_policy(&self, policy: DownloadPolicy) -> GatewayFuture<()> {
        self.inner.update_download_policy(policy)
    }
}

#[test]
fn empty_download_queue_hides_queue_section_without_tabs() {
    let gw = Arc::new(EmptyOfflineGateway {
        inner: MockGateway::new(),
        empty_library: false,
    });
    let mut app = Headless::start(EngineConfig {
        viewport: Size::new(Points(1240.0), Points(800.0)),
        environment: Environment {
            os: Os::MacOs,
            color_scheme: ColorScheme::Light,
            ..Environment::default()
        },
        ..EngineConfig::new(root_with_gateway(gw))
    })
    .unwrap();
    app.wait_for("精选推荐").unwrap();
    app.press("离线").unwrap();
    app.wait_for("已下载").unwrap();
    app.wait_for("托管离线片库").unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
    assert!(!app.screen().shows("下载中"));
    assert!(!app.screen().shows("下载任务队列"));
}

#[test]
fn empty_offline_library_and_queue_shows_unified_empty_state() {
    let gw = Arc::new(EmptyOfflineGateway {
        inner: MockGateway::new(),
        empty_library: true,
    });
    let mut app = Headless::start(EngineConfig {
        viewport: Size::new(Points(1240.0), Points(800.0)),
        environment: Environment {
            os: Os::MacOs,
            color_scheme: ColorScheme::Light,
            ..Environment::default()
        },
        ..EngineConfig::new(root_with_gateway(gw))
    })
    .unwrap();
    app.wait_for("精选推荐").unwrap();
    app.press("离线").unwrap();
    app.wait_for("暂无离线剧集").unwrap();
    assert!(!app.screen().shows("下载中"));
}
