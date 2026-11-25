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
    AppAppearance, PlaybackPreference, PlaybackSource, ReportWatchProgressRequest,
    ReportWatchProgressResult, SearchHistoryEntry, SeriesBookmark, StorageOverview,
    WatchHistoryEntry,
};
use hg_gateway::contract::{AppGateway, GatewayFuture};
use hg_gateway::mock::MockGateway;
use nsh_core::prelude::{Points, Size, point, px};
use nsh_engine::{ColorScheme, EngineConfig, Environment, Os};
use nsh_testing::Headless;

use crate::root_with_gateway;

struct PagedDiscoverGateway {
    inner: MockGateway,
}

impl PagedDiscoverGateway {
    fn make_series(id: u64, title: &str, hot: u64) -> SeriesSummary {
        SeriesSummary {
            id: SeriesId::new(NonZeroU64::new(id).unwrap_or(NonZeroU64::MIN)),
            title: title.to_owned(),
            cover_url: "poster://amber-noir".to_owned(),
            score: 9.0,
            tags: vec!["都市".to_owned()],
            description: "分页测试剧目".to_owned(),
            episode_count: 20,
            is_finished: true,
            hot,
            badge: None,
        }
    }
}

impl AppGateway for PagedDiscoverGateway {
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
            .map(|i| Self::make_series(2000 + i, &format!("分页剧集 {i:02}"), 1000 - i))
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
        let page_num = request.page.page.get();
        let start_idx = if page_num == 1 { 1 } else { 25 };
        let end_idx = if page_num == 1 { 24 } else { 30 };
        let items = (start_idx..=end_idx)
            .map(|i| RankingEntry {
                rank: i as u32,
                series: Self::make_series(3000 + i, &format!("榜单剧目 {i:02}"), 2000 - i),
                metric_label: "热度".to_owned(),
                metric_value: 2000 - i,
            })
            .collect();
        Box::pin(std::future::ready(Ok(PagedResult {
            items,
            pagination: PageInfo {
                page: page_num,
                pages: 2,
                size: 24,
                total: 30,
            },
        })))
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

fn start_desktop(scheme: ColorScheme) -> Headless {
    let environment = Environment {
        os: Os::MacOs,
        color_scheme: scheme,
        ..Environment::default()
    };
    let mut app = Headless::start(EngineConfig {
        viewport: Size::new(Points(1240.0), Points(800.0)),
        environment,
        ..EngineConfig::new(root_with_gateway(Arc::new(MockGateway::new())))
    })
    .unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
    app
}

fn blocking_pref(gw: &MockGateway) -> PlaybackPreference {
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(waker);
    let mut fut = gw.playback_preference();
    match fut.as_mut().poll(&mut cx) {
        std::task::Poll::Ready(Ok(pref)) => pref,
        std::task::Poll::Ready(Err(_)) | std::task::Poll::Pending => {
            panic!("MockGateway playback_preference did not resolve immediately")
        }
    }
}

fn set_blocking_pref(gw: &MockGateway, pref: PlaybackPreference) {
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(waker);
    let mut fut = gw.update_playback_preference(pref);
    match fut.as_mut().poll(&mut cx) {
        std::task::Poll::Ready(Ok(())) => {}
        std::task::Poll::Ready(Err(_)) | std::task::Poll::Pending => {
            panic!("MockGateway update_playback_preference did not resolve immediately")
        }
    }
}

#[test]
fn discover_home_shows_featured_ranking_and_categories_without_search_bar_or_hero() {
    let app = start_desktop(ColorScheme::Light);
    let screen = app.screen();

    assert!(screen.shows("精选推荐"));
    assert!(screen.shows("排行榜"));
    assert!(screen.shows("热门短剧"));
    assert!(screen.shows("漫剧与悬疑"));
    assert!(!screen.shows("今日焦点"));
    assert!(!screen.shows("搜索剧名、标签..."));
}

#[test]
fn opening_featured_and_ranking_supports_sort_without_search_bar() {
    let mut app = start_desktop(ColorScheme::Light);

    app.press("查看排行榜全部").unwrap();
    app.wait_for("热播榜").unwrap();
    assert!(!app.screen().shows("热搜"));
    app.press("潜力榜").unwrap();
    app.wait_for("潜力榜").unwrap();

    app.press("返回").unwrap();
    app.wait_for("查看精选推荐全部").unwrap();

    app.press("查看精选推荐全部").unwrap();
    app.wait_for("春风十里不及卿").unwrap();
    app.wait_for("最热").unwrap();
    app.wait_for("最新").unwrap();
    assert!(!app.screen().shows("热搜"));
    assert!(!app.screen().shows("题材"));
    app.press("最新").unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
}

#[test]
fn browse_and_ranking_paginate_via_pure_infinite_scroll_without_load_more_button() {
    let gw = Arc::new(PagedDiscoverGateway {
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

    app.press("查看精选推荐全部").unwrap();
    app.wait_for("分页剧集 01").unwrap();
    assert!(!app.screen().shows("加载更多"));
    assert!(!app.screen().shows("分页剧集 28"));

    app.scroll("分页剧集 01", px(5000.0)).unwrap();
    app.wait_for("分页剧集 28").unwrap();
    assert!(!app.screen().shows("加载更多"));

    app.press("最新").unwrap();
    app.wait_for("分页剧集 30").unwrap();
    assert!(!app.screen().shows("分页剧集 01"));

    app.press("返回").unwrap();
    app.wait_for("查看排行榜全部").unwrap();
    app.press("查看排行榜全部").unwrap();
    app.wait_for("榜单剧目 01").unwrap();
    assert!(!app.screen().shows("榜单剧目 28"));
    app.scroll("榜单剧目 01", px(5000.0)).unwrap();
    app.wait_for("榜单剧目 28").unwrap();
}

#[test]
fn sidebar_navigation_covers_all_five_plus_one_tabs() {
    let mut app = start_desktop(ColorScheme::Light);

    app.press("搜索").unwrap();
    app.wait_for("热搜").unwrap();

    app.press("追剧").unwrap();
    app.wait_for("我的追剧").unwrap();
    assert!(!app.screen().shows("最近观看"));

    app.press("历史").unwrap();
    app.wait_for("最近观看").unwrap();
    assert!(!app.screen().shows("我的追剧"));

    app.press("离线").unwrap();
    app.wait_for("已下载").unwrap();
    app.wait_for("下载中").unwrap();

    app.press("设置").unwrap();
    app.wait_for("外观").unwrap();
    app.wait_for("存储空间").unwrap();

    app.press("发现").unwrap();
    app.wait_for("精选推荐").unwrap();
}

#[test]
fn desktop_sidebar_collapse_and_drag_resize_persist_preferences() {
    let gw = Arc::new(MockGateway::new());
    let mut app = Headless::start(EngineConfig {
        viewport: Size::new(Points(1240.0), Points(800.0)),
        environment: Environment {
            os: Os::MacOs,
            color_scheme: ColorScheme::Light,
            ..Environment::default()
        },
        ..EngineConfig::new(root_with_gateway(Arc::clone(&gw) as _))
    })
    .unwrap();
    app.wait_for("精选推荐").unwrap();

    app.drag("label:调整侧边栏宽度", point(px(52.0), px(0.0)))
        .unwrap();
    let resized = blocking_pref(gw.as_ref());
    assert_eq!(resized.sidebar_width, 260);
    assert!(!resized.sidebar_collapsed);

    app.press("label:收起侧边栏").unwrap();
    app.wait_for("label:展开侧边栏").unwrap();
    app.wait(Duration::from_millis(50)).unwrap();
    let collapsed_pref = blocking_pref(gw.as_ref());
    assert!(collapsed_pref.sidebar_collapsed);
    assert_eq!(collapsed_pref.sidebar_width, 260);

    app.press("label:展开侧边栏").unwrap();
    app.wait_for("label:收起侧边栏").unwrap();
    app.wait(Duration::from_millis(50)).unwrap();
    let expanded_pref = blocking_pref(gw.as_ref());
    assert!(!expanded_pref.sidebar_collapsed);
}

#[test]
fn startup_hydrates_appearance_and_sidebar_state_from_gateway() {
    let gw = Arc::new(MockGateway::new());
    set_blocking_pref(
        gw.as_ref(),
        PlaybackPreference {
            appearance: AppAppearance::Dark,
            sidebar_collapsed: true,
            sidebar_width: 280,
            ..PlaybackPreference::default()
        },
    );

    let mut app = Headless::start(EngineConfig {
        viewport: Size::new(Points(1240.0), Points(800.0)),
        environment: Environment {
            os: Os::MacOs,
            color_scheme: ColorScheme::Light,
            ..Environment::default()
        },
        ..EngineConfig::new(root_with_gateway(Arc::clone(&gw) as _))
    })
    .unwrap();
    app.wait_for("精选推荐").unwrap();
    app.wait_for("label:展开侧边栏").unwrap();
}

#[test]
fn mobile_uses_slide_out_drawer_and_quick_search_without_bottom_bar() {
    let mut app = Headless::start(EngineConfig {
        viewport: Size::new(Points(390.0), Points(844.0)),
        environment: Environment {
            os: Os::Android,
            color_scheme: ColorScheme::Light,
            ..Environment::default()
        },
        ..EngineConfig::new(root_with_gateway(Arc::new(MockGateway::new())))
    })
    .unwrap();
    app.wait_for("精选推荐").unwrap();
    assert!(!app.screen().shows("历史"));
    assert!(!app.screen().shows("离线"));

    app.press("label:打开导航菜单").unwrap();
    app.wait_for("历史").unwrap();
    app.wait_for("离线").unwrap();
    app.wait_for("设置").unwrap();

    app.press("历史").unwrap();
    app.wait_for("最近观看").unwrap();
    app.wait_for_gone("label:导航菜单", Duration::from_secs(2))
        .unwrap();

    app.press("label:前往搜索").unwrap();
    app.wait_for("热搜").unwrap();
    assert!(!app.screen().shows("label:前往搜索"));

    app.press("label:打开导航菜单").unwrap();
    app.wait_for("发现").unwrap();
    app.press("发现").unwrap();
    app.wait_for("查看精选推荐全部").unwrap();

    app.press("查看精选推荐全部").unwrap();
    app.wait_for("春风十里不及卿").unwrap();
    app.back().unwrap();
    app.wait_for("查看精选推荐全部").unwrap();
}
