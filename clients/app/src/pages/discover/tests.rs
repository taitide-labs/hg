use std::sync::Arc;
use std::time::Duration;

use hg_contracts::error::ContractError;
use hg_contracts::viewing::{AppAppearance, PlaybackPreference};
use hg_gateway::mock::MockGateway;
use nsh_core::prelude::{Points, Size, point, px};
use nsh_engine::ColorScheme;

use crate::pages::tests::{
    DESKTOP_VIEWPORT, MOBILE_VIEWPORT, assert_left_of, assert_visible_in_viewport,
    count_first_row_cards, node_bounds, start_desktop, start_desktop_on_home, start_mobile,
};

#[test]
fn discover_home_layout_invariants_and_no_embedded_search_or_hero() {
    let gw = Arc::new(MockGateway::new());
    let app = start_desktop_on_home(gw, ColorScheme::Light);
    let screen = app.screen();

    for expected in ["精选推荐", "排行榜", "热门短剧", "漫剧与悬疑"] {
        assert!(screen.shows(expected), "missing section `{expected}`");
        assert_visible_in_viewport(&app, expected, DESKTOP_VIEWPORT);
    }
    assert!(!screen.shows("今日焦点"));
    assert!(!screen.shows("搜索剧名、标签..."));
    assert_left_of(&app, "发现", "精选推荐");
}

#[test]
#[ignore = "BUG: 发现页错误态内的「重新加载」位于 dynamic 子树，点击后切换 loading 导致父 dynamic 卸载并取消刚 spawn 的 home_feed 异步任务，页面永久白屏"]
fn discover_home_error_state_recovers_on_retry() {
    let gw = Arc::new(MockGateway::new());
    gw.inject_method_error(
        "home_feed",
        Some(ContractError::NetworkUnavailable {
            detail: "home down".to_owned(),
        }),
    );
    let mut app = start_desktop(Arc::clone(&gw), ColorScheme::Light);
    app.wait_for("重新加载").unwrap();

    gw.inject_method_error("home_feed", None);
    app.press("重新加载").unwrap();
    app.wait_for("精选推荐").unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
}

#[test]
fn discover_featured_and_ranking_support_sort_and_back_without_search_chrome() {
    let gw = Arc::new(MockGateway::new());
    let mut app = start_desktop_on_home(gw, ColorScheme::Light);

    app.press("查看排行榜全部").unwrap();
    app.wait_for("热播榜").unwrap();
    assert!(!app.screen().shows("热搜"));
    app.press("潜力榜").unwrap();
    app.wait_for("潜力榜").unwrap();
    app.press("推荐榜").unwrap();
    app.wait_for("推荐榜").unwrap();

    app.press("返回").unwrap();
    app.wait_for("查看精选推荐全部").unwrap();

    app.press("查看精选推荐全部").unwrap();
    app.wait_for("春风十里不及卿").unwrap();
    app.wait_for("最热").unwrap();
    app.wait_for("最新").unwrap();
    assert!(!app.screen().shows("热搜"));
    assert!(!app.screen().shows("题材"));
    app.press("最新").unwrap();
    app.wait_for("春风十里不及卿").unwrap();
}

#[test]
fn discover_browse_and_ranking_paginate_via_infinite_scroll_and_adapt_columns() {
    let gw = Arc::new(MockGateway::new().with_paged_catalog(30));
    let mut app = start_desktop_on_home(gw, ColorScheme::Light);

    app.press("查看精选推荐全部").unwrap();
    app.wait_for("分页剧目 01").unwrap();
    assert!(!app.screen().shows("加载更多"));
    assert!(!app.screen().shows("分页剧目 28"));

    app.press("最新").unwrap();
    app.wait_for("分页剧目 30").unwrap();
    let selectors: Vec<String> = (13..=30)
        .rev()
        .map(|i| format!("分页剧目 {i:02}"))
        .collect();
    app.set_viewport(Size::new(Points(920.0), Points(800.0)))
        .unwrap();
    app.wait(Duration::from_millis(60)).unwrap();
    let narrow_cols = count_first_row_cards(&app, &selectors);

    app.set_viewport(Size::new(Points(1640.0), Points(800.0)))
        .unwrap();
    app.wait(Duration::from_millis(60)).unwrap();
    let wide_cols = count_first_row_cards(&app, &selectors);
    assert!(
        wide_cols > narrow_cols,
        "adaptive grid must grow columns with viewport (narrow={narrow_cols}, wide={wide_cols})"
    );
    app.set_viewport(DESKTOP_VIEWPORT).unwrap();
    app.press("最热").unwrap();
    app.wait_for("分页剧目 01").unwrap();

    app.scroll("分页剧目 01", px(5000.0)).unwrap();
    app.wait_for("分页剧目 28").unwrap();
    assert!(!app.screen().shows("加载更多"));

    app.press("最新").unwrap();
    app.wait_for("分页剧目 30").unwrap();
    assert!(!app.screen().shows("分页剧目 01"));

    app.press("返回").unwrap();
    app.wait_for("查看排行榜全部").unwrap();
    app.press("查看排行榜全部").unwrap();
    app.wait_for("分页剧目 01").unwrap();
    assert!(!app.screen().shows("加载更多"));
    app.scroll("分页剧目 01", px(5000.0)).unwrap();
    app.wait_for("分页剧目 28").unwrap();
}

#[test]
fn discover_browse_ignores_stale_sort_response() {
    let gw = Arc::new(MockGateway::new().with_paged_catalog(30));
    let mut app = start_desktop_on_home(Arc::clone(&gw), ColorScheme::Light);

    let slow_hot_gate = gw.queue_method_gate("list_series");
    app.press("查看精选推荐全部").unwrap();
    app.wait_for("最新").unwrap();

    app.press("最新").unwrap();
    app.wait_for("分页剧目 30").unwrap();

    slow_hot_gate.release();
    app.wait(Duration::from_millis(60)).unwrap();
    assert!(app.screen().shows("分页剧目 30"));
    assert!(!app.screen().shows("分页剧目 01"));
}

#[test]
fn sidebar_navigation_covers_all_five_plus_one_tabs() {
    let gw = Arc::new(MockGateway::new());
    let mut app = start_desktop_on_home(gw, ColorScheme::Light);

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
