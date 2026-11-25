mod batch_download;
mod comment_replies;

use std::num::NonZeroU64;
use std::sync::Arc;
use std::time::Duration;

use hg_contracts::catalog::{
    Category, CommentReply, EpisodeComment, HomeFeed, HotKeyword, HotTag,
    ListCommentRepliesRequest, ListEpisodeCommentsRequest, ListRankingRequest, ListSeriesRequest,
    PagedResult, RankingEntry, SearchSeriesRequest, SearchSuggestion, SeriesDetail, SeriesSummary,
    TagGroup,
};
use hg_contracts::ids::{CommentId, EpisodeKey, PageInfo, SeriesId};
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
use nsh_core::geometry::point;
use nsh_engine::{ColorScheme, EngineConfig, Environment, Os, Points, Size};
use nsh_testing::Headless;

use crate::root_with_gateway;

pub(super) fn start_and_open_first_series(scheme: ColorScheme) -> Headless {
    start_with_gateway(
        Arc::new(MockGateway::new()),
        Os::MacOs,
        scheme,
        (1240.0, 800.0),
    )
}

fn start_with_gateway(
    gw: Arc<dyn AppGateway>,
    os: Os,
    scheme: ColorScheme,
    viewport: (f32, f32),
) -> Headless {
    let environment = Environment {
        os,
        color_scheme: scheme,
        ..Environment::default()
    };
    let mut app = Headless::start(EngineConfig {
        viewport: Size::new(Points(viewport.0), Points(viewport.1)),
        environment,
        ..EngineConfig::new(root_with_gateway(gw))
    })
    .unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
    app.press("云端之上：千金归来 >> nth=0").unwrap();
    app.wait_for("选集 (3/68)").unwrap();
    app.press("1").unwrap();
    app.wait_for("选集 (1/68)").unwrap();
    app
}

struct PagedCommentsGateway {
    inner: MockGateway,
    total_comments: u64,
}

impl PagedCommentsGateway {
    fn new(total_comments: u64) -> Self {
        Self {
            inner: MockGateway::new(),
            total_comments,
        }
    }
}

impl AppGateway for PagedCommentsGateway {
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
        let total = self.total_comments;
        let page = request.page.page.get().max(1);
        let size = request.page.size.get().max(1);
        let size_u64 = u64::from(size);
        let pages = u32::try_from(total.div_ceil(size_u64)).unwrap_or(1).max(1);
        let start = u64::from(page - 1) * size_u64;
        let end = (start + size_u64).min(total);
        let items = (start..end)
            .map(|idx| {
                let id = idx + 1;
                EpisodeComment {
                    id: CommentId::new(NonZeroU64::new(id).unwrap_or(NonZeroU64::MIN)),
                    nickname: format!("评论用户{id}"),
                    avatar_color: "#0F766E".to_owned(),
                    text: format!("分页评论内容 #{id}"),
                    relative_time: "5 分钟前".to_owned(),
                    like_count: 10,
                    is_pinned: id == 1,
                    ip_region: None,
                    reply_count: if id == 1 { 3 } else { 0 },
                    preview_replies: Vec::new(),
                }
            })
            .collect();
        Box::pin(async move {
            Ok(PagedResult {
                items,
                pagination: PageInfo {
                    page,
                    pages,
                    size: u32::from(size),
                    total,
                },
            })
        })
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

#[test]
fn desktop_player_shows_clean_single_column_feed_without_mode_or_ratio_clutter() {
    let app = start_and_open_first_series(ColorScheme::Light);
    let screen = app.screen();

    assert!(screen.shows("云端之上：千金归来"));
    assert!(screen.shows("选集 (1/68)"));
    assert!(screen.shows("评论 (2)"));
    assert!(screen.shows("林深见鹿"));
    assert!(screen.shows("共 2 条回复"));
    assert!(screen.shows("离线下载"));

    assert!(!screen.shows("电影模式"));
    assert!(!screen.shows("短剧模式"));
    assert!(!screen.shows("正在播放"));
    assert!(!screen.shows("比例 1"));
    assert!(!screen.shows("完播并连播"));
}

#[test]
fn synopsis_and_episodes_fold_by_default_and_expand_in_place() {
    let mut app = start_and_open_first_series(ColorScheme::Light);

    assert!(!app.screen().shows("揭开当年真相"));
    app.press("展开简介").unwrap();
    app.wait_for("text:/揭开当年真相/").unwrap();
    app.press("收起简介").unwrap();
    app.wait_for_gone("text:/揭开当年真相/", Duration::from_secs(2))
        .unwrap();

    assert!(!app.screen().shows("51-68"));
    app.press("展开选集").unwrap();
    app.wait_for("51-68").unwrap();

    app.press("51-68").unwrap();
    app.wait_for("68").unwrap();
    app.press("68").unwrap();
    app.wait_for("选集 (68/68)").unwrap();

    app.press("收起选集").unwrap();
    app.wait_for_gone("51-68", Duration::from_secs(2)).unwrap();
    assert!(app.screen().shows("61"));
    assert!(app.screen().shows("68"));
}

#[test]
fn in_video_bottom_right_settings_and_speed_popovers_work() {
    let mut app = start_and_open_first_series(ColorScheme::Dark);

    assert!(!app.screen().shows("倍速"));
    app.hover("视频画面").unwrap();
    app.wait_for("倍速").unwrap();

    app.press("倍速").unwrap();
    app.wait_for("1.5×").unwrap();
    app.press("1.5×").unwrap();
    app.wait_for("1.5×").unwrap();

    app.press("播放设置").unwrap();
    app.wait_for("自动连播").unwrap();
    app.wait_for("单集循环").unwrap();
    app.wait_for("外部播放器打开").unwrap();

    app.press("自动连播").unwrap();
    app.press("外部播放器打开").unwrap();
    app.wait_for("text:/已准备外部播放地址/").unwrap();
}

#[test]
fn true_fullscreen_and_vertical_view_hide_chrome_and_restore_cleanly() {
    let mut app = start_and_open_first_series(ColorScheme::Light);

    app.hover("视频画面").unwrap();
    app.wait_for("倍速").unwrap();
    app.press("全屏").unwrap();
    app.wait_for_gone("选集 (1/68)", Duration::from_secs(2))
        .unwrap();
    app.wait_for_gone("离线下载", Duration::from_secs(2))
        .unwrap();

    app.press_key("enter").unwrap();
    app.wait_for("选集 (1/68)").unwrap();

    app.hover("视频画面").unwrap();
    app.press("竖屏视图").unwrap();
    app.wait_for("详情视图").unwrap();
    app.press("详情视图").unwrap();
    app.wait_for("选集 (1/68)").unwrap();
}

#[test]
fn bookmark_and_offline_download_actions_update_state() {
    let mut app = start_and_open_first_series(ColorScheme::Light);

    app.wait_for("已追剧").unwrap();
    app.press("已追剧").unwrap();
    app.wait_for("追剧").unwrap();
    app.press("追剧").unwrap();
    app.wait_for("已追剧").unwrap();

    app.press("离线下载").unwrap();
    app.wait_for("选择离线下载分集").unwrap();
}

#[test]
fn bottom_control_bar_auto_fades_after_three_seconds_of_inactivity() {
    let mut app = start_and_open_first_series(ColorScheme::Dark);

    assert!(!app.screen().shows("倍速"));
    app.hover("视频画面").unwrap();
    app.wait_for("倍速").unwrap();
    app.wait_for_gone("倍速", Duration::from_secs(5)).unwrap();

    app.hover("视频画面").unwrap();
    app.wait_for("倍速").unwrap();
    app.press("播放或暂停").unwrap();
    assert!(app.screen().shows("倍速"));
}

#[test]
fn scrub_slider_drags_and_seeks_in_overlay() {
    let mut app = start_and_open_first_series(ColorScheme::Dark);

    app.hover("视频画面").unwrap();
    app.wait_for("播放进度").unwrap();
    app.drag("播放进度", point(Points(120.0), Points(0.0)))
        .unwrap();
    assert!(!app.screen().shows("00:00 /"));
}

#[test]
fn infinite_scroll_comments_shows_total_count_and_reply_link_without_load_more_button() {
    let gw = Arc::new(PagedCommentsGateway::new(25));
    let mut app = start_with_gateway(gw, Os::MacOs, ColorScheme::Light, (1240.0, 800.0));

    app.wait_for("评论 (25)").unwrap();
    app.wait_for("共 3 条回复").unwrap();
    assert!(!app.screen().shows("加载更多"));
    assert!(app.screen().shows("分页评论内容 #1"));
    assert!(!app.screen().shows("分页评论内容 #25"));

    app.scroll("分页评论内容 #1", Points(2200.0)).unwrap();
    app.wait_for("分页评论内容 #25").unwrap();
    assert!(!app.screen().shows("加载更多"));
}

#[test]
fn android_system_back_closes_drawer_before_exiting_player() {
    let gw = Arc::new(MockGateway::new());
    let mut app = Headless::start(EngineConfig {
        viewport: Size::new(Points(390.0), Points(844.0)),
        environment: Environment {
            os: Os::Android,
            color_scheme: ColorScheme::Light,
            ..Environment::default()
        },
        ..EngineConfig::new(root_with_gateway(gw))
    })
    .unwrap();

    app.wait_for("云端之上：千金归来").unwrap();
    app.press("云端之上：千金归来 >> nth=0").unwrap();
    app.wait_for("详情视图").unwrap();
    app.press("详情视图").unwrap();
    app.wait_for("离线下载").unwrap();

    app.press("离线下载").unwrap();
    app.wait_for("选择离线下载分集").unwrap();
    app.back().unwrap();
    app.wait_for_gone("选择离线下载分集", Duration::from_secs(2))
        .unwrap();
    assert!(app.screen().shows("离线下载"));

    app.back().unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
}
