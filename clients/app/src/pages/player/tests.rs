mod batch_download;
mod comment_replies;

use std::num::{NonZeroU32, NonZeroU64};
use std::sync::Arc;
use std::time::Duration;

use hg_contracts::ids::{EpisodeNumber, SeriesId};
use hg_contracts::viewing::PlaybackPreference;
use hg_gateway::mock::MockGateway;
use nsh_core::geometry::point;
use nsh_engine::{ColorScheme, Points};
use nsh_testing::Headless;

use crate::pages::tests::{
    DESKTOP_VIEWPORT, assert_above, assert_left_of, assert_node_stable, assert_same_height,
    assert_visible_in_viewport, emit_video_ended, emit_video_error, emit_video_progress,
    node_bounds, start_desktop, start_desktop_manual_clock, start_mobile,
};

pub(super) fn start_and_open_first_series(scheme: ColorScheme) -> (Arc<MockGateway>, Headless) {
    start_with_gateway(Arc::new(MockGateway::new()), scheme)
}

pub(super) fn start_with_gateway(
    gw: Arc<MockGateway>,
    scheme: ColorScheme,
) -> (Arc<MockGateway>, Headless) {
    let mut app = start_desktop(Arc::clone(&gw), scheme);
    app.wait_for("云端之上：千金归来").unwrap();
    app.press("云端之上：千金归来 >> nth=0").unwrap();
    app.wait_for("选集 (3/68)").unwrap();
    app.press("1").unwrap();
    app.wait_for("选集 (1/68)").unwrap();
    (gw, app)
}

#[test]
fn desktop_player_layout_shows_clean_single_column_feed_without_clutter() {
    let (_gw, app) = start_and_open_first_series(ColorScheme::Light);
    let screen = app.screen();

    for expected in [
        "云端之上：千金归来",
        "选集 (1/68)",
        "评论 (2)",
        "林深见鹿",
        "共 2 条回复",
        "离线下载",
    ] {
        assert!(
            screen.shows(expected),
            "missing player element `{expected}`"
        );
    }
    assert_visible_in_viewport(&app, "视频画面", DESKTOP_VIEWPORT);
    assert_left_of(&app, "视频画面", "选集 (1/68)");
    assert_above(&app, "选集 (1/68)", "评论 (2)");

    for forbidden in ["电影模式", "短剧模式", "正在播放", "比例 1", "完播并连播"]
    {
        assert!(!screen.shows(forbidden), "unexpected clutter `{forbidden}`");
    }
}

#[test]
fn synopsis_is_always_fully_expanded_and_episodes_fold_remains_stable() {
    let (_gw, mut app) = start_and_open_first_series(ColorScheme::Light);

    app.wait_for("text:/揭开当年真相/").unwrap();
    assert!(app.screen().shows("都市"));
    assert!(app.screen().id("label:展开简介").is_err());
    assert!(app.screen().id("label:收起简介").is_err());

    assert_node_stable(&mut app, "选集 (1/68)", |headless| {
        for sec in 1..=5 {
            emit_video_progress(headless, sec, 120);
        }
    });

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
fn in_video_speed_and_settings_popovers_work_without_tick_remount() {
    let (_gw, mut app) = start_and_open_first_series(ColorScheme::Dark);

    assert!(!app.screen().shows("倍速"));
    app.hover("视频画面").unwrap();
    app.wait_for("倍速").unwrap();

    app.press("倍速").unwrap();
    app.wait_for("1.5×").unwrap();
    assert_node_stable(&mut app, "1.5×", |headless| {
        for sec in 10..=15 {
            emit_video_progress(headless, sec, 120);
        }
    });
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
fn player_applies_persisted_default_rate_and_honors_shortcuts() {
    let gw = Arc::new(
        MockGateway::new().with_playback_preference(PlaybackPreference {
            default_rate_hundredths: 150,
            ..PlaybackPreference::default()
        }),
    );
    let (_gw, mut app) = start_with_gateway(gw, ColorScheme::Dark);

    app.hover("视频画面").unwrap();
    app.wait_for("1.5×").unwrap();

    emit_video_progress(&mut app, 20, 120);
    app.press_key("right").unwrap();
    app.wait_for("text:/00:25/").unwrap();
    app.press_key("left").unwrap();
    app.wait_for("text:/00:20/").unwrap();

    app.press_key("z").unwrap();
    app.wait_for("倍速").unwrap();
    app.press_key("c").unwrap();
    app.wait_for("1.25×").unwrap();
    app.press_key("x").unwrap();
    app.wait_for("倍速").unwrap();

    app.press_key("m").unwrap();
    app.press_key("space").unwrap();
    assert!(app.screen().shows("倍速"));
}

#[test]
fn player_auto_advance_and_loop_single_only_fire_on_real_ended_event() {
    let (_gw, mut app) = start_and_open_first_series(ColorScheme::Dark);

    emit_video_progress(&mut app, 0, 90);
    emit_video_progress(&mut app, 88, 90);
    app.wait(Duration::from_millis(80)).unwrap();
    assert!(
        app.screen().shows("选集 (1/68)"),
        "98% progress must not prematurely skip before ended event"
    );

    emit_video_ended(&mut app);
    app.wait_for("选集 (2/68)").unwrap();

    app.hover("视频画面").unwrap();
    app.press("播放设置").unwrap();
    app.wait_for("单集循环").unwrap();
    app.press("单集循环").unwrap();
    emit_video_ended(&mut app);
    app.wait(Duration::from_millis(80)).unwrap();
    assert!(
        app.screen().shows("选集 (2/68)"),
        "loop-single must stay on episode 2 after ended event"
    );
}

#[test]
fn player_native_video_error_surfaces_feedback() {
    let (_gw, mut app) = start_and_open_first_series(ColorScheme::Dark);

    emit_video_error(&mut app, "解码器初始化失败");
    app.wait_for("text:/解码器初始化失败/").unwrap();
}

#[test]
fn true_fullscreen_and_vertical_view_hide_chrome_and_restore_cleanly() {
    let (_gw, mut app) = start_and_open_first_series(ColorScheme::Light);

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
fn bookmark_button_toggles_red_brand_style_and_gateway_state() {
    let (gw, mut app) = start_and_open_first_series(ColorScheme::Light);

    app.wait_for("已追剧").unwrap();
    let active_bg = app.screen().view_style("role:button[已追剧]").unwrap();
    let active_fg = app.screen().span_style("已追剧").unwrap();

    app.press("已追剧").unwrap();
    app.wait_for("追剧").unwrap();
    app.wait(Duration::from_millis(40)).unwrap();
    assert!(
        !gw.snapshot_bookmarks()
            .iter()
            .any(|b| b.series_id.get() == 1001)
    );
    let inactive_bg = app.screen().view_style("role:button[追剧]").unwrap();
    let inactive_fg = app
        .screen()
        .span_style("role:button[追剧] >> 追剧")
        .unwrap();
    assert_ne!(active_bg, inactive_bg);
    assert_ne!(active_fg.color, inactive_fg.color);

    app.press("追剧").unwrap();
    app.wait_for("已追剧").unwrap();
    app.wait(Duration::from_millis(40)).unwrap();
    assert!(
        gw.snapshot_bookmarks()
            .iter()
            .any(|b| b.series_id.get() == 1001)
    );
}

#[test]
fn bottom_control_bar_auto_fades_via_manual_clock_even_during_ticks() {
    let gw = Arc::new(MockGateway::new());
    let mut app = start_desktop_manual_clock(gw, ColorScheme::Dark);
    app.wait_for("云端之上：千金归来").unwrap();
    app.press("云端之上：千金归来 >> nth=0").unwrap();
    app.wait_for("选集 (3/68)").unwrap();

    assert!(!app.screen().shows("倍速"));
    app.hover("视频画面").unwrap();
    app.wait_for("倍速").unwrap();

    emit_video_progress(&mut app, 10, 120);
    app.fast_forward(Duration::from_secs(2)).unwrap();
    assert!(app.screen().shows("倍速"));

    app.fast_forward(Duration::from_secs(2)).unwrap();
    assert!(
        !app.screen().shows("倍速"),
        "controls must hide once 3s elapse even after progress ticks"
    );

    app.hover("视频画面").unwrap();
    app.wait_for("倍速").unwrap();
    app.press("播放或暂停").unwrap();
    app.fast_forward(Duration::from_secs(5)).unwrap();
    assert!(
        app.screen().shows("倍速"),
        "paused playback must keep control bar visible"
    );
}

#[test]
fn scrub_slider_drags_and_seeks_in_overlay() {
    let (_gw, mut app) = start_and_open_first_series(ColorScheme::Dark);

    app.hover("视频画面").unwrap();
    app.wait_for("播放进度").unwrap();
    emit_video_progress(&mut app, 10, 120);
    app.wait_for("text:/00:10 \\/ 02:00/").unwrap();

    app.drag("播放进度", point(Points(120.0), Points(0.0)))
        .unwrap();
    assert!(!app.screen().shows("00:10 / 02:00"));

    emit_video_progress(&mut app, 10, 120);
    emit_video_progress(&mut app, 11, 120);
    assert!(
        !app.screen().shows("00:10 / 02:00") && !app.screen().shows("00:11 / 02:00"),
        "stale pre-seek progress ticks during active playback must not flash scrubber back"
    );
}

#[test]
fn player_control_bar_icons_and_rate_trigger_have_balanced_proportions() {
    let (_gw, mut app) = start_and_open_first_series(ColorScheme::Dark);

    app.hover("视频画面").unwrap();
    app.wait_for("倍速").unwrap();

    let time_style = app.screen().span_style("text:/00:00 \\/ /").unwrap();
    let rate_style = app.screen().span_style("倍速").unwrap();
    assert_eq!(
        rate_style.font_size, time_style.font_size,
        "rate trigger font size ({:?}) must match time readout font size ({:?})",
        rate_style.font_size, time_style.font_size
    );

    let time_bounds = node_bounds(&app, "text:/00:00 \\/ /");
    let rate_text_bounds = node_bounds(&app, "倍速");
    assert!(
        (rate_text_bounds.size.height.0 - time_bounds.size.height.0).abs() <= 1.0,
        "rate trigger text height ({}) must match time readout height ({})",
        rate_text_bounds.size.height.0,
        time_bounds.size.height.0
    );

    assert_same_height(&app, "label:播放或暂停", "label:全屏", 1.0);
    assert_same_height(&app, "label:静音", "label:播放设置", 1.0);
    assert_same_height(&app, "label:播放设置", "label:竖屏视图", 1.0);
    assert_same_height(&app, "role:button[倍速]", "label:全屏", 1.0);
}

#[test]
fn double_clicking_video_canvas_toggles_play_and_pause() {
    let gw = Arc::new(MockGateway::new());
    let mut app = start_desktop_manual_clock(gw, ColorScheme::Dark);
    app.wait_for("云端之上：千金归来").unwrap();
    app.press("云端之上：千金归来 >> nth=0").unwrap();
    app.wait_for("选集 (3/68)").unwrap();

    app.press("视频画面").unwrap();
    app.press("视频画面").unwrap();
    app.wait_for("倍速").unwrap();
    app.fast_forward(Duration::from_secs(5)).unwrap();
    assert!(
        app.screen().shows("倍速"),
        "double-clicking video canvas must pause playback and keep controls visible"
    );

    app.press("视频画面").unwrap();
    app.press("视频画面").unwrap();
    app.fast_forward(Duration::from_secs(5)).unwrap();
    assert!(
        !app.screen().shows("倍速"),
        "double-clicking video canvas again must resume playback so controls auto-hide"
    );
}

#[test]
fn infinite_scroll_comments_shows_total_count_and_reply_link_without_load_more() {
    let series_id = SeriesId::new(NonZeroU64::new(1001).unwrap());
    let ep1 = EpisodeNumber::new(NonZeroU32::MIN);
    let gw = Arc::new(MockGateway::new().with_paged_comments(series_id, ep1, 25));
    let (_gw, mut app) = start_with_gateway(gw, ColorScheme::Light);

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
    let mut app = start_mobile(gw, ColorScheme::Light);

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
