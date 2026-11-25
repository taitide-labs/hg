use std::num::NonZeroU64;
use std::sync::Arc;
use std::time::Duration;

use hg_contracts::catalog::{
    Category, CommentReply, EpisodeComment, HomeFeed, HotKeyword, HotTag,
    ListCommentRepliesRequest, ListEpisodeCommentsRequest, ListRankingRequest, ListSeriesRequest,
    PagedResult, RankingEntry, SearchSeriesRequest, SearchSuggestion, SeriesDetail, SeriesSort,
    SeriesSummary, TagGroup,
};
use hg_contracts::ids::{EpisodeKey, PageInfo, SeriesId};
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
use nsh_core::prelude::px;
use nsh_engine::{ColorScheme, EngineConfig, Environment, Os, Points, Size};
use nsh_testing::Headless;

use crate::root_with_gateway;

struct PagedSearchGateway {
    inner: MockGateway,
}

impl PagedSearchGateway {
    fn make_series(id: u64, title: &str, hot: u64) -> SeriesSummary {
        SeriesSummary {
            id: SeriesId::new(NonZeroU64::new(id).unwrap_or(NonZeroU64::MIN)),
            title: title.to_owned(),
            cover_url: "poster://amber-noir".to_owned(),
            score: 9.0,
            tags: vec!["悬疑".to_owned()],
            description: "搜索分页测试".to_owned(),
            episode_count: 16,
            is_finished: true,
            hot,
            badge: None,
        }
    }
}

impl AppGateway for PagedSearchGateway {
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
        let mut all: Vec<SeriesSummary> = (1..=30)
            .map(|i| Self::make_series(4000 + i, &format!("标签剧目 {i:02}"), 1000 - i))
            .collect();
        if request.sort == SeriesSort::New {
            all.sort_by_key(|item| std::cmp::Reverse(item.id));
        }
        let page_num = request.page.page.get();
        let size = usize::from(request.page.size.get());
        let start = (page_num as usize).saturating_sub(1) * size;
        let slice = all.into_iter().skip(start).take(size).collect();
        Box::pin(std::future::ready(Ok(PagedResult {
            items: slice,
            pagination: PageInfo {
                page: page_num,
                pages: 2,
                size: u32::from(request.page.size.get()),
                total: 30,
            },
        })))
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
        let all: Vec<SeriesSummary> = (1..=30)
            .map(|i| {
                Self::make_series(
                    5000 + i,
                    &format!("{} 结果 {i:02}", request.keyword),
                    1000 - i,
                )
            })
            .collect();
        let page_num = request.page.page.get();
        let size = usize::from(request.page.size.get());
        let start = (page_num as usize).saturating_sub(1) * size;
        let slice = all.into_iter().skip(start).take(size).collect();
        Box::pin(std::future::ready(Ok(PagedResult {
            items: slice,
            pagination: PageInfo {
                page: page_num,
                pages: 2,
                size: u32::from(request.page.size.get()),
                total: 30,
            },
        })))
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
        self.inner.offline_library()
    }
    fn download_tasks(&self) -> GatewayFuture<Vec<DownloadTaskView>> {
        self.inner.download_tasks()
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

fn start_on_search() -> Headless {
    let mut app = Headless::start(EngineConfig {
        viewport: Size::new(Points(1240.0), Points(800.0)),
        environment: Environment {
            os: Os::MacOs,
            color_scheme: ColorScheme::Dark,
            ..Environment::default()
        },
        ..EngineConfig::new(root_with_gateway(Arc::new(MockGateway::new())))
    })
    .unwrap();
    app.wait_for("精选推荐").unwrap();
    app.press("搜索").unwrap();
    app.wait_for("热搜").unwrap();
    app
}

#[test]
fn search_page_starts_empty_and_supports_history_hot_and_tag_toggle() {
    let mut app = start_on_search();

    assert!(!app.screen().shows("春风十里不及卿"));
    assert!(!app.screen().shows("全 80 集"));
    app.wait_for("搜索历史").unwrap();
    app.wait_for("千金").unwrap();

    app.type_text("text:搜索剧名、标签...", "暗夜").unwrap();
    app.press("label:执行搜索").unwrap();
    app.wait_for("全 80 集").unwrap();
    assert!(!app.screen().shows("春风十里不及卿"));
    app.wait_for("暗夜").unwrap();

    app.press("label:删除搜索历史 千金").unwrap();
    app.wait_for_gone("label:删除搜索历史 千金", Duration::from_secs(2))
        .unwrap();

    app.press("label:清空搜索历史").unwrap();
    app.wait_for_gone("搜索历史", Duration::from_secs(2))
        .unwrap();

    app.press("悬疑 >> nth=0").unwrap();
    app.wait_for("最热").unwrap();
    app.wait_for("最新").unwrap();
    app.wait_for("全 80 集").unwrap();

    app.press("悬疑 >> nth=0").unwrap();
    app.wait_for_gone("全 80 集", Duration::from_secs(2))
        .unwrap();
}

#[test]
fn search_page_results_and_tags_use_infinite_scroll_without_load_more_button() {
    let gw = Arc::new(PagedSearchGateway {
        inner: MockGateway::new(),
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
    app.press("搜索").unwrap();
    app.wait_for("热搜").unwrap();

    app.type_text("text:搜索剧名、标签...", "风云").unwrap();
    app.press("label:执行搜索").unwrap();
    app.wait_for("风云 结果 01").unwrap();
    assert!(!app.screen().shows("加载更多"));
    assert!(!app.screen().shows("风云 结果 28"));

    app.scroll("风云 结果 01", px(5000.0)).unwrap();
    app.wait_for("风云 结果 28").unwrap();
    assert!(!app.screen().shows("加载更多"));

    app.press("悬疑 >> nth=0").unwrap();
    app.wait_for("标签剧目 01").unwrap();
    app.scroll("标签剧目 01", px(5000.0)).unwrap();
    app.wait_for("标签剧目 28").unwrap();

    app.press("最新").unwrap();
    app.wait_for("标签剧目 30").unwrap();
    assert!(!app.screen().shows("标签剧目 01"));
}
