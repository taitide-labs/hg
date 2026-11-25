use std::sync::Arc;
use std::time::Duration;

use hg_contracts::viewing::{AppAppearance, PlaybackPreference};
use hg_gateway::mock::MockGateway;
use nsh_core::prelude::{Points, Size, point, px};
use nsh_engine::ColorScheme;

use crate::support::{
    DESKTOP_VIEWPORT, MOBILE_VIEWPORT, assert_left_of, assert_visible_in_viewport, node_bounds,
    start_desktop_on_home, start_mobile,
};

#[test]
fn sidebar_navigation_covers_all_five_plus_one_tabs() {
    let gw = Arc::new(MockGateway::new());
    let mut app = start_desktop_on_home(gw, ColorScheme::Light);

    assert_left_of(&app, "发现", "精选推荐");
    for tab in ["发现", "搜索", "追剧", "历史", "离线", "设置"] {
        assert_visible_in_viewport(&app, tab, DESKTOP_VIEWPORT);
    }

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
fn desktop_sidebar_collapse_and_drag_resize_update_layout_and_persist() {
    let gw = Arc::new(MockGateway::new());
    let mut app = start_desktop_on_home(Arc::clone(&gw), ColorScheme::Light);

    let content_x_before = node_bounds(&app, "精选推荐").origin.x.0;
    app.drag("label:调整侧边栏宽度", point(px(52.0), px(0.0)))
        .unwrap();
    let resized = gw.snapshot_preference();
    assert_eq!(resized.sidebar_width, 260);
    assert!(!resized.sidebar_collapsed);
    let content_x_wider = node_bounds(&app, "精选推荐").origin.x.0;
    assert!(
        content_x_wider > content_x_before + 30.0,
        "wider sidebar must shift main content right ({content_x_before} -> {content_x_wider})"
    );

    app.press("label:收起侧边栏").unwrap();
    app.wait_for("label:展开侧边栏").unwrap();
    app.wait(Duration::from_millis(50)).unwrap();
    let collapsed = gw.snapshot_preference();
    assert!(collapsed.sidebar_collapsed);
    assert_eq!(collapsed.sidebar_width, 260);
    let content_x_collapsed = node_bounds(&app, "精选推荐").origin.x.0;
    assert!(
        content_x_collapsed < content_x_before - 100.0,
        "collapsed icon rail must reclaim horizontal space ({content_x_before} -> {content_x_collapsed})"
    );

    app.press("label:展开侧边栏").unwrap();
    app.wait_for("label:收起侧边栏").unwrap();
    app.wait(Duration::from_millis(50)).unwrap();
    assert!(!gw.snapshot_preference().sidebar_collapsed);
}

#[test]
fn narrow_viewport_auto_collapses_sidebar_without_overwriting_preference() {
    let gw = Arc::new(MockGateway::new());
    let mut app = start_desktop_on_home(Arc::clone(&gw), ColorScheme::Light);

    app.set_viewport(Size::new(Points(540.0), Points(800.0)))
        .unwrap();
    app.wait(Duration::from_millis(80)).unwrap();
    let content_x_narrow = node_bounds(&app, "精选推荐").origin.x.0;
    assert!(
        content_x_narrow < 100.0,
        "narrow desktop window must auto-collapse sidebar into icon rail (content_x={content_x_narrow})"
    );
    assert!(
        !gw.snapshot_preference().sidebar_collapsed,
        "responsive auto-collapse must not overwrite saved user sidebar preference"
    );

    app.set_viewport(DESKTOP_VIEWPORT).unwrap();
    app.wait(Duration::from_millis(80)).unwrap();
    let content_x_restored = node_bounds(&app, "精选推荐").origin.x.0;
    assert!(
        content_x_restored > 180.0,
        "widening back to desktop viewport must restore expanded sidebar (content_x={content_x_restored})"
    );
}

#[test]
fn startup_hydrates_dark_appearance_and_collapsed_sidebar_from_gateway() {
    let light_app = start_desktop_on_home(Arc::new(MockGateway::new()), ColorScheme::Light);
    let light_fg = light_app.screen().span_style("精选推荐").unwrap().color;

    let gw = Arc::new(
        MockGateway::new().with_playback_preference(PlaybackPreference {
            appearance: AppAppearance::Dark,
            sidebar_collapsed: true,
            sidebar_width: 280,
            ..PlaybackPreference::default()
        }),
    );
    let mut dark_app = start_desktop_on_home(gw, ColorScheme::Light);
    dark_app.wait_for("label:展开侧边栏").unwrap();
    let dark_fg = dark_app.screen().span_style("精选推荐").unwrap().color;
    assert_ne!(
        dark_fg, light_fg,
        "hydrated dark preference must switch rendered text colors"
    );
}

#[test]
fn mobile_uses_slide_out_drawer_quick_search_and_android_back_without_bottom_bar() {
    let gw = Arc::new(MockGateway::new());
    let mut app = start_mobile(gw, ColorScheme::Light);
    app.wait_for("精选推荐").unwrap();
    assert_visible_in_viewport(&app, "label:打开导航菜单", MOBILE_VIEWPORT);
    assert_visible_in_viewport(&app, "label:前往搜索", MOBILE_VIEWPORT);
    assert!(!app.screen().shows("历史"));
    assert!(!app.screen().shows("离线"));

    app.press("label:打开导航菜单").unwrap();
    app.wait_for("历史").unwrap();
    app.wait_for("离线").unwrap();
    app.wait_for("设置").unwrap();

    app.back().unwrap();
    app.wait_for_gone("label:导航菜单", Duration::from_secs(2))
        .unwrap();

    app.press("label:打开导航菜单").unwrap();
    app.wait_for("历史").unwrap();
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
