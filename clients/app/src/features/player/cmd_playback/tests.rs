use std::future::Future;
use std::num::{NonZeroU32, NonZeroU64};
use std::pin::Pin;
use std::sync::atomic::{AtomicU16, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::task::{Context, Poll, Waker};

use hg_contracts::catalog::{
    Category, CommentReply, EpisodeComment, HomeFeed, HotKeyword, HotTag,
    ListCommentRepliesRequest, ListEpisodeCommentsRequest, ListRankingRequest, ListSeriesRequest,
    PagedResult, RankingEntry, SearchSeriesRequest, SearchSuggestion, SeriesDetail, SeriesSummary,
    TagGroup,
};
use hg_contracts::error::ContractError;
use hg_contracts::ids::{CommentId, EpisodeKey, EpisodeNumber, PageInfo, SeriesId};
use hg_contracts::offline::{
    ControlAllDownloadsAction, ControlDownloadRequest, DownloadPolicy, DownloadTaskView,
    EnqueueEpisodeDownloadsRequest, EnqueueEpisodeDownloadsResult, OfflineSeriesView,
};
use hg_contracts::viewing::{
    DislikedSeriesEntry, PlaybackMode, PlaybackPreference, PlaybackSource,
    ReportWatchProgressRequest, ReportWatchProgressResult, SearchHistoryEntry, SeriesBookmark,
    StorageOverview, WatchHistoryEntry, WatchProgress,
};
use hg_gateway::contract::{AppGateway, GatewayFuture};
use hg_gateway::mock::MockGateway;
use nsh_core::prelude::*;
use nsh_engine::{ColorScheme, EngineConfig, Environment, Os, Points, Size};
use nsh_testing::Headless;

use super::super::store;
use super::command::{
    AspectPreset, ResumePlan, adjacent_episode, clamp_seek, episode_segment_ranges,
    episode_switch_resume_secs, folded_episode_window, format_rate_trigger, is_tail_completed,
    resume_plan, should_refresh_stream_on_player_error, step_rate,
};
use super::handler;
use crate::gateway;

fn ep(value: u32) -> EpisodeNumber {
    EpisodeNumber::new(NonZeroU32::new(value).unwrap_or(NonZeroU32::MIN))
}

fn sid(value: u64) -> SeriesId {
    SeriesId::new(NonZeroU64::new(value).unwrap_or(NonZeroU64::MIN))
}

#[derive(Default)]
struct DeferredGate {
    ready: bool,
    waker: Option<Waker>,
}

struct DeferredFuture<T> {
    gate: Arc<RwLock<DeferredGate>>,
    result: Option<Result<T, ContractError>>,
}

impl<T: Unpin> Future for DeferredFuture<T> {
    type Output = Result<T, ContractError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let is_ready = {
            let mut guard = self.gate.write().unwrap();
            if guard.ready {
                true
            } else {
                guard.waker = Some(cx.waker().clone());
                false
            }
        };
        if is_ready {
            Poll::Ready(self.result.take().unwrap())
        } else {
            Poll::Pending
        }
    }
}

struct TestGateway {
    inner: MockGateway,
    default_rate: AtomicU16,
    delay_episode: AtomicU32,
    ep_gate: Arc<RwLock<DeferredGate>>,
    comments_total: AtomicU64,
    reported_calls: AtomicU32,
    last_reported_pos: AtomicU32,
    refresh_calls: AtomicU32,
}

impl TestGateway {
    fn new(default_rate: u16, comments_total: u64) -> Self {
        Self {
            inner: MockGateway::new(),
            default_rate: AtomicU16::new(default_rate),
            delay_episode: AtomicU32::new(0),
            ep_gate: Arc::new(RwLock::new(DeferredGate::default())),
            comments_total: AtomicU64::new(comments_total),
            reported_calls: AtomicU32::new(0),
            last_reported_pos: AtomicU32::new(0),
            refresh_calls: AtomicU32::new(0),
        }
    }

    fn set_delayed_episode(&self, ep_num: u32) {
        let mut guard = self.ep_gate.write().unwrap();
        guard.ready = false;
        guard.waker = None;
        self.delay_episode.store(ep_num, Ordering::SeqCst);
    }

    fn release_delayed_episode(&self) {
        let waker = {
            let mut guard = self.ep_gate.write().unwrap();
            guard.ready = true;
            guard.waker.take()
        };
        if let Some(w) = waker {
            w.wake();
        }
    }
}

fn make_test_comments(page: u32, size: u16, total: u64) -> PagedResult<EpisodeComment> {
    let page = page.max(1);
    let size_u64 = u64::from(size.max(1));
    let pages = u32::try_from(total.div_ceil(size_u64)).unwrap_or(1).max(1);
    let start = u64::from(page - 1) * size_u64;
    let end = (start + size_u64).min(total);
    let mut items = Vec::new();
    if page == 2 && total > size_u64 {
        items.push(sample_comment(size_u64, 0));
    }
    for idx in start..end {
        items.push(sample_comment(idx + 1, if idx == 0 { 3 } else { 0 }));
    }
    PagedResult {
        items,
        pagination: PageInfo {
            page,
            pages,
            size: u32::from(size),
            total,
        },
    }
}

fn sample_comment(id: u64, reply_count: u32) -> EpisodeComment {
    EpisodeComment {
        id: CommentId::new(NonZeroU64::new(id).unwrap_or(NonZeroU64::MIN)),
        nickname: format!("观众{id}"),
        avatar_color: "#0F766E".to_owned(),
        text: format!("第 {id} 条测试评论"),
        relative_time: "刚刚".to_owned(),
        like_count: 5,
        is_pinned: id == 1,
        ip_region: None,
        reply_count,
        preview_replies: Vec::new(),
    }
}

impl AppGateway for TestGateway {
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
        let total = self.comments_total.load(Ordering::SeqCst);
        if total > 0 {
            let paged = make_test_comments(request.page.page.get(), request.page.size.get(), total);
            return Box::pin(async move { Ok(paged) });
        }
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
        let fut = self.inner.prepare_episode_playback(key);
        if self.delay_episode.load(Ordering::SeqCst) == key.ep_num.get() {
            let gate = Arc::clone(&self.ep_gate);
            return Box::pin(async move {
                let res = fut.await;
                DeferredFuture {
                    gate,
                    result: Some(res),
                }
                .await
            });
        }
        fut
    }
    fn refresh_online_playback(&self, key: EpisodeKey) -> GatewayFuture<PlaybackSource> {
        self.refresh_calls.fetch_add(1, Ordering::SeqCst);
        self.inner.refresh_online_playback(key)
    }
    fn report_watch_progress(
        &self,
        request: ReportWatchProgressRequest,
    ) -> GatewayFuture<ReportWatchProgressResult> {
        self.reported_calls.fetch_add(1, Ordering::SeqCst);
        self.last_reported_pos
            .store(request.position_secs, Ordering::SeqCst);
        self.inner.report_watch_progress(request)
    }
    fn watch_history(&self) -> GatewayFuture<Vec<WatchHistoryEntry>> {
        self.inner.watch_history()
    }
    fn episode_progress(&self, series_id: SeriesId) -> GatewayFuture<Vec<WatchProgress>> {
        self.inner.episode_progress(series_id)
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
    fn disliked_series(&self) -> GatewayFuture<Vec<DislikedSeriesEntry>> {
        self.inner.disliked_series()
    }
    fn is_disliked(&self, series_id: SeriesId) -> GatewayFuture<bool> {
        self.inner.is_disliked(series_id)
    }
    fn dislike_series(&self, series_id: SeriesId) -> GatewayFuture<()> {
        self.inner.dislike_series(series_id)
    }
    fn undislike_series(&self, series_id: SeriesId) -> GatewayFuture<()> {
        self.inner.undislike_series(series_id)
    }
    fn clear_disliked_series(&self) -> GatewayFuture<()> {
        self.inner.clear_disliked_series()
    }
    fn playback_preference(&self) -> GatewayFuture<PlaybackPreference> {
        let rate = self.default_rate.load(Ordering::SeqCst);
        Box::pin(async move {
            Ok(PlaybackPreference {
                default_rate_hundredths: rate,
                ..PlaybackPreference::default()
            })
        })
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

#[derive(Clone)]
struct HarnessProps {
    gw: Arc<TestGateway>,
    initial_ep: Option<EpisodeNumber>,
}

impl PartialEq for HarnessProps {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.gw, &other.gw) && self.initial_ep == other.initial_ep
    }
}

impl nsh_core::element::Props for HarnessProps {
    fn spec(_: &mut nsh_core::element::TsTypes) -> Vec<nsh_core::element::PropSpec> {
        Vec::new()
    }
    fn initial() -> Self {
        Self {
            gw: Arc::new(TestGateway::new(100, 0)),
            initial_ep: Some(ep(1)),
        }
    }
    fn set(
        &mut self,
        name: &str,
        _: Option<nsh_core::element::AnyValue>,
    ) -> Result<bool, nsh_core::element::PropError> {
        Err(nsh_core::element::PropError(format!(
            "unknown prop `{name}`"
        )))
    }
    fn set_slot(&mut self, _: &str, _: Option<nsh_core::element::Slot>) -> bool {
        false
    }
    fn set_children(&mut self, _: nsh_core::element::Children) -> bool {
        false
    }
}

struct PlaybackHarness;

impl nsh_core::element::Component for PlaybackHarness {
    type Props = HarnessProps;
    type Event = nsh_core::element::NoEvent;

    fn view(props: Self::Props, _cx: Scope<Self::Event>) -> impl IntoElement {
        let gw_dyn: Arc<dyn AppGateway> = Arc::clone(&props.gw) as Arc<dyn AppGateway>;
        gateway::install(gw_dyn);
        handler::open_series(sid(1001), props.initial_ep, PlaybackMode::Cinema);
        view()
            .child(harness_metrics())
            .child(harness_actions(Arc::clone(&props.gw)))
    }
}

fn harness_metrics() -> impl IntoElement {
    view()
        .child(text(move || {
            format!(
                "ep={}",
                store::snapshot()
                    .current_episode
                    .map_or(0, EpisodeNumber::get)
            )
        }))
        .child(text(move || {
            format!("seq={}", store::snapshot().episode_seq)
        }))
        .child(text(move || {
            format!("pos={}", store::progress_snapshot().position_secs)
        }))
        .child(text(move || {
            format!("dur={}", store::progress_snapshot().duration_secs)
        }))
        .child(text(move || {
            format!("seek={}", store::progress_snapshot().seek_seq)
        }))
        .child(text(move || {
            format!("rate={}", store::controls_snapshot().rate_hundredths)
        }))
        .child(text(move || {
            format!(
                "hint={}",
                store::controls_snapshot()
                    .status_hint
                    .as_deref()
                    .unwrap_or("none")
            )
        }))
        .child(text(move || {
            format!("paused={}", store::controls_snapshot().paused)
        }))
        .child(text(move || {
            format!("hover={}", store::controls_snapshot().controls_hovered)
        }))
        .child(text(move || {
            format!("keep={}", store::should_keep_controls_visible())
        }))
        .child(text(move || {
            let cm = store::comments_snapshot();
            format!(
                "total={};items={};more={}",
                cm.total_count,
                cm.items.len(),
                cm.has_more
            )
        }))
}

fn harness_actions(gw_release: Arc<TestGateway>) -> impl IntoElement {
    view()
        .child(
            nsh_base::button()
                .on_press(|| handler::on_playback_progress(45, 127))
                .child(text("progress_45")),
        )
        .child(
            nsh_base::button()
                .on_press(|| handler::on_playback_progress(125, 127))
                .child(text("progress_98")),
        )
        .child(
            nsh_base::button()
                .on_press(|| handler::seek_by(125))
                .child(text("seek_by_end")),
        )
        .child(
            nsh_base::button()
                .on_press(handler::on_playback_ended)
                .child(text("emit_ended")),
        )
        .child(
            nsh_base::button()
                .on_press(store::toggle_loop_single)
                .child(text("toggle_loop")),
        )
        .child(
            nsh_base::button()
                .on_press(|| handler::switch_episode(ep(1)))
                .child(text("switch_ep1")),
        )
        .child(
            nsh_base::button()
                .on_press(|| handler::switch_episode(ep(2)))
                .child(text("switch_ep2")),
        )
        .child(
            nsh_base::button()
                .on_press(|| handler::switch_episode(ep(3)))
                .child(text("switch_ep3")),
        )
        .child(
            nsh_base::button()
                .on_press(move || gw_release.release_delayed_episode())
                .child(text("release_ep")),
        )
        .child(
            nsh_base::button()
                .on_press(handler::cleanup_on_exit)
                .child(text("cleanup_exit")),
        )
        .child(
            nsh_base::button()
                .on_press(|| handler::seek_to(9999))
                .child(text("seek_to_clamp")),
        )
        .child(
            nsh_base::button()
                .on_press(handler::load_more_comments)
                .child(text("load_more_comments")),
        )
        .child(
            nsh_base::button()
                .on_press(|| store::set_controls_hovered(true))
                .child(text("hover_bar")),
        )
        .child(
            nsh_base::button()
                .on_press(|| {
                    let _toggled = store::toggle_fullscreen();
                })
                .child(text("toggle_fs")),
        )
        .child(
            nsh_base::button()
                .on_press(|| store::set_scrub_preview(Some(42)))
                .child(text("scrub_42")),
        )
        .child(
            nsh_base::button()
                .on_press(store::leave_surface)
                .child(text("leave_surface")),
        )
        .child(
            nsh_base::button()
                .on_press(|| {
                    handler::on_playback_error(
                        "unable to load libmpv runtime (libmpv-2.dll: missing)".to_owned(),
                    );
                })
                .child(text("emit_libmpv_error")),
        )
}

fn start_harness(gw: Arc<TestGateway>) -> Headless {
    let props = HarnessProps {
        gw,
        initial_ep: Some(ep(1)),
    };
    let mut app = Headless::start(EngineConfig {
        viewport: Size::new(Points(800.0), Points(600.0)),
        environment: Environment {
            os: Os::MacOs,
            color_scheme: ColorScheme::Light,
            ..Environment::default()
        },
        ..EngineConfig::new(nsh_core::element::Root::new::<PlaybackHarness>(move || {
            props.clone()
        }))
    })
    .unwrap();
    app.wait_for("ep=1").unwrap();
    app
}

#[test]
fn resume_plan_continues_last_unfinished_episode_at_saved_position() {
    assert_eq!(
        resume_plan(None, Some((ep(7), 83, false)), 40),
        ResumePlan {
            episode: ep(7),
            position_secs: Some(83)
        }
    );
    assert_eq!(
        resume_plan(Some(ep(7)), Some((ep(7), 83, false)), 40),
        ResumePlan {
            episode: ep(7),
            position_secs: Some(83)
        }
    );
}

#[test]
fn resume_plan_advances_past_completed_episode_and_respects_explicit_choice() {
    assert_eq!(
        resume_plan(None, Some((ep(7), 120, true)), 40),
        ResumePlan {
            episode: ep(8),
            position_secs: None
        }
    );
    assert_eq!(
        resume_plan(None, Some((ep(40), 120, true)), 40),
        ResumePlan {
            episode: ep(40),
            position_secs: None
        }
    );
    assert_eq!(
        resume_plan(Some(ep(3)), Some((ep(7), 83, false)), 40),
        ResumePlan {
            episode: ep(3),
            position_secs: None
        }
    );
    assert_eq!(
        resume_plan(None, None, 40),
        ResumePlan {
            episode: ep(1),
            position_secs: None
        }
    );
    assert_eq!(
        resume_plan(None, Some((ep(2), 0, false)), 40),
        ResumePlan {
            episode: ep(2),
            position_secs: None
        }
    );
}

#[test]
fn potplayer_aspect_digits_map_one_through_four() {
    assert_eq!(AspectPreset::from_digit(1), Some(AspectPreset::Original));
    assert_eq!(AspectPreset::from_digit(2), Some(AspectPreset::Fill));
    assert_eq!(AspectPreset::from_digit(3), Some(AspectPreset::Cinema16x9));
    assert_eq!(
        AspectPreset::from_digit(4),
        Some(AspectPreset::Portrait9x16)
    );
    assert_eq!(AspectPreset::from_digit(5), None);
}

#[test]
fn seek_and_rate_clamping_stay_in_bounds() {
    assert_eq!(clamp_seek(10, -25, 120), 0);
    assert_eq!(clamp_seek(115, 15, 120), 120);
    assert_eq!(step_rate(100, true), 125);
    assert_eq!(step_rate(200, true), 200);
    assert_eq!(step_rate(50, false), 50);
    assert_eq!(format_rate_trigger(100), "倍速");
    assert_eq!(format_rate_trigger(125), "1.25×");
    assert_eq!(format_rate_trigger(150), "1.5×");
}

#[test]
fn adjacent_episode_respects_boundaries() {
    let ep1 = EpisodeNumber::new(NonZeroU32::new(1).unwrap());
    let ep2 = EpisodeNumber::new(NonZeroU32::new(2).unwrap());
    assert_eq!(adjacent_episode(ep1, 2, false), None);
    assert_eq!(adjacent_episode(ep1, 2, true), Some(ep2));
    assert_eq!(adjacent_episode(ep2, 2, true), None);
}

#[test]
fn folded_episode_window_centers_around_current_episode() {
    assert_eq!(folded_episode_window(1, 68, 8), (1, 8));
    assert_eq!(folded_episode_window(18, 68, 8), (14, 21));
    assert_eq!(folded_episode_window(68, 68, 8), (61, 68));
    assert_eq!(folded_episode_window(3, 6, 8), (1, 6));
    assert_eq!(episode_segment_ranges(68, 50), vec![(1, 50), (51, 68)]);
}

#[test]
fn progress_at_completion_threshold_does_not_skip_or_loop() {
    let gw = Arc::new(TestGateway::new(100, 0));
    let mut app = start_harness(Arc::clone(&gw));

    app.press("progress_98").unwrap();
    app.wait_for("pos=125").unwrap();
    assert!(app.screen().shows("ep=1"));
    assert!(gw.reported_calls.load(Ordering::SeqCst) >= 1);

    app.press("toggle_loop").unwrap();
    app.press("seek_by_end").unwrap();
    app.wait_for("seek=1").unwrap();
    assert!(app.screen().shows("ep=1"));
    assert!(app.screen().shows("pos=127"));
}

#[test]
fn playback_ended_handles_loop_single_and_auto_advance() {
    let gw = Arc::new(TestGateway::new(100, 0));
    let mut app = start_harness(gw);

    app.press("toggle_loop").unwrap();
    app.press("emit_ended").unwrap();
    app.wait_for("pos=0").unwrap();
    assert!(app.screen().shows("ep=1"));
    assert!(app.screen().shows("seek=1"));

    app.press("toggle_loop").unwrap();
    app.press("emit_ended").unwrap();
    app.wait_for("ep=2").unwrap();
}

#[test]
fn episode_seq_guard_ignores_stale_loads_and_cleanup_cancels() {
    let gw = Arc::new(TestGateway::new(100, 0));
    let mut app = start_harness(Arc::clone(&gw));

    gw.set_delayed_episode(2);
    app.press("switch_ep2").unwrap();
    app.press("emit_ended").unwrap();
    app.press("switch_ep3").unwrap();
    app.wait_for("ep=3").unwrap();

    app.press("release_ep").unwrap();
    app.settle();
    assert!(app.screen().shows("ep=3"));

    gw.set_delayed_episode(2);
    app.press("switch_ep2").unwrap();
    app.press("cleanup_exit").unwrap();
    app.wait_for("paused=true").unwrap();
    app.press("release_ep").unwrap();
    app.settle();
    assert!(app.screen().shows("ep=3"));
}

#[test]
fn open_series_applies_saved_default_rate_silently() {
    let gw = Arc::new(TestGateway::new(150, 0));
    let mut app = start_harness(gw);

    app.wait_for("rate=150").unwrap();
    assert!(app.screen().shows("hint=none"));
}

#[test]
fn seek_to_clamps_and_reports_progress() {
    let gw = Arc::new(TestGateway::new(100, 0));
    let mut app = start_harness(Arc::clone(&gw));

    app.press("seek_to_clamp").unwrap();
    app.wait_for("pos=127").unwrap();
    assert!(app.screen().shows("seek=1"));
    assert_eq!(gw.last_reported_pos.load(Ordering::SeqCst), 127);
}

#[test]
fn comment_pagination_appends_deduped_pages_and_tracks_total_count() {
    let gw = Arc::new(TestGateway::new(100, 25));
    let mut app = start_harness(gw);

    app.wait_for("total=25;items=20;more=true").unwrap();
    app.press("load_more_comments").unwrap();
    app.wait_for("total=25;items=25;more=false").unwrap();
}

#[test]
fn controls_hover_and_scrub_preview_keep_controls_visible_and_reset_cleanly() {
    let gw = Arc::new(TestGateway::new(100, 0));
    let mut app = start_harness(gw);

    assert!(app.screen().shows("hover=false"));
    assert!(app.screen().shows("keep=false"));
    app.press("hover_bar").unwrap();
    app.wait_for("hover=true").unwrap();
    app.wait_for("keep=true").unwrap();

    app.press("toggle_fs").unwrap();
    app.wait_for("hover=false").unwrap();
    app.wait_for("keep=false").unwrap();

    app.press("scrub_42").unwrap();
    app.wait_for("keep=true").unwrap();

    app.press("hover_bar").unwrap();
    app.press("leave_surface").unwrap();
    app.wait_for("hover=false").unwrap();
}

struct PlayerViewTickHarness;

impl nsh_core::element::Component for PlayerViewTickHarness {
    type Props = HarnessProps;
    type Event = nsh_core::element::NoEvent;

    fn view(props: Self::Props, _cx: Scope<Self::Event>) -> impl IntoElement {
        let gw_dyn: Arc<dyn AppGateway> = Arc::clone(&props.gw) as Arc<dyn AppGateway>;
        gateway::install(gw_dyn);
        let tick_pos = signal(1_u32);
        view()
            .size_full()
            .child(super::view::player_view(
                sid(1001),
                props.initial_ep,
                || {},
                |_| {},
                |_| {},
            ))
            .child(
                nsh_base::button()
                    .on_press(move || {
                        let next = tick_pos.get_untracked().saturating_add(1);
                        tick_pos.set(next);
                        handler::on_playback_progress(next, 127);
                    })
                    .child(text("tick_progress")),
            )
    }
}

#[test]
fn progress_ticks_do_not_reset_auto_hide_or_teardown_popovers_and_feed() {
    let props = HarnessProps {
        gw: Arc::new(TestGateway::new(100, 0)),
        initial_ep: Some(ep(1)),
    };
    let mut app = Headless::start(EngineConfig {
        viewport: Size::new(Points(1240.0), Points(800.0)),
        clock: nsh_core::time::Clock::manual(std::time::SystemTime::UNIX_EPOCH),
        environment: Environment {
            os: Os::MacOs,
            color_scheme: ColorScheme::Dark,
            ..Environment::default()
        },
        ..EngineConfig::new(nsh_core::element::Root::new::<PlayerViewTickHarness>(
            move || props.clone(),
        ))
    })
    .unwrap();

    app.wait_for("选集 (1/68)").unwrap();
    app.hover("视频画面").unwrap();
    app.wait_for("倍速").unwrap();

    for _ in 0..7 {
        app.run_for(std::time::Duration::from_millis(500)).unwrap();
        app.press("tick_progress").unwrap();
    }
    assert!(!app.screen().shows("倍速"));

    app.hover("视频画面").unwrap();
    app.wait_for("倍速").unwrap();
    app.press("倍速").unwrap();
    app.wait_for("1.5×").unwrap();
    app.press("tick_progress").unwrap();
    assert!(app.screen().shows("1.5×"));

    app.press("展开选集").unwrap();
    app.press("tick_progress").unwrap();
    app.wait_for("收起选集").unwrap();
}

#[test]
fn episode_switch_resets_progress_and_follows_industry_resume_rules() {
    assert!(!is_tail_completed(45, 127));
    assert!(is_tail_completed(125, 127));
    assert_eq!(episode_switch_resume_secs(None), None);
    let incomplete = WatchProgress {
        key: EpisodeKey::new(sid(1001), ep(1)),
        position_secs: 45,
        duration_secs: 127,
        completed: false,
    };
    assert_eq!(episode_switch_resume_secs(Some(&incomplete)), Some(45));
    let completed = WatchProgress {
        completed: true,
        ..incomplete
    };
    assert_eq!(episode_switch_resume_secs(Some(&completed)), None);

    let gw = Arc::new(TestGateway::new(100, 0));
    let mut app = start_harness(gw);

    app.press("progress_45").unwrap();
    app.wait_for("pos=45").unwrap();

    app.press("switch_ep2").unwrap();
    app.wait_for("ep=2").unwrap();
    assert!(app.screen().shows("pos=0"));
    assert!(app.screen().shows("seek=0"));

    app.press("progress_98").unwrap();
    app.wait_for("pos=125").unwrap();

    app.press("switch_ep1").unwrap();
    app.wait_for("ep=1").unwrap();
    app.wait_for("pos=45").unwrap();
    assert!(app.screen().shows("seek=1"));

    app.press("switch_ep2").unwrap();
    app.wait_for("ep=2").unwrap();
    assert!(app.screen().shows("pos=0"));
    assert!(app.screen().shows("seek=0"));
}

#[test]
fn player_runtime_init_error_shows_hint_without_refreshing_stream() {
    assert!(!should_refresh_stream_on_player_error(
        "unable to load libmpv runtime (libmpv-2.dll: missing)"
    ));
    assert!(should_refresh_stream_on_player_error(
        "HTTP 403 upstream auth expired"
    ));

    let gw = Arc::new(TestGateway::new(100, 0));
    let mut app = start_harness(Arc::clone(&gw));

    app.press("emit_libmpv_error").unwrap();
    app.wait_for("hint=unable to load libmpv runtime (libmpv-2.dll: missing)")
        .unwrap();
    assert_eq!(gw.refresh_calls.load(Ordering::SeqCst), 0);
}
