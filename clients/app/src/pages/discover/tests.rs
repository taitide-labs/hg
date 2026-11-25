use std::sync::Arc;
use std::time::Duration;

use hg_contracts::error::ContractError;
use hg_gateway::mock::MockGateway;
use nsh_core::prelude::{Points, Size, signal, stored};
use nsh_engine::ColorScheme;
use nsh_testing::Headless;

use crate::features::discover::store::BrowseKind;
use crate::pages::discover::page::discover_page;
use crate::pages::tests::{
    DESKTOP_VIEWPORT, OpenSeriesSpy, assert_visible_in_viewport, count_first_row_cards,
    mount_desktop_page, node_bounds,
};

fn mount_discover(gw: Arc<MockGateway>, scheme: ColorScheme) -> (Headless, OpenSeriesSpy) {
    let opened = OpenSeriesSpy::default();
    let opened_for_page = opened.clone();
    let app = mount_desktop_page(gw, scheme, move || {
        let active_browse = signal(None::<BrowseKind>);
        let sink = stored(opened_for_page.clone());
        discover_page(
            move || active_browse.get(),
            move |kind| active_browse.set(Some(kind)),
            move || active_browse.set(None),
            move |id| sink.with(|s| s.record(id, None)),
        )
    });
    (app, opened)
}

fn mount_discover_ready(gw: Arc<MockGateway>, scheme: ColorScheme) -> (Headless, OpenSeriesSpy) {
    let (mut app, opened) = mount_discover(gw, scheme);
    app.wait_for("精选推荐").unwrap();
    (app, opened)
}

#[test]
fn discover_home_layout_invariants_and_no_embedded_search_or_hero() {
    let gw = Arc::new(MockGateway::new());
    let (mut app, opened) = mount_discover_ready(gw, ColorScheme::Light);
    let screen = app.screen();

    for expected in ["精选推荐", "排行榜", "热门短剧", "漫剧与悬疑"] {
        assert!(screen.shows(expected), "missing section `{expected}`");
        assert_visible_in_viewport(&app, expected, DESKTOP_VIEWPORT);
    }
    assert!(!screen.shows("今日焦点"));
    assert!(!screen.shows("搜索剧名、标签..."));

    app.press("云端之上：千金归来 >> nth=0").unwrap();
    let emitted = opened
        .last()
        .expect("pressing discover card must emit open intent");
    assert_eq!(emitted.0.get(), 1001);
    assert_eq!(emitted.1, None);
}

#[test]
fn discover_home_error_state_recovers_on_retry() {
    let gw = Arc::new(MockGateway::new());
    gw.inject_method_error(
        "home_feed",
        Some(ContractError::NetworkUnavailable {
            detail: "home down".to_owned(),
        }),
    );
    let (mut app, _opened) = mount_discover(Arc::clone(&gw), ColorScheme::Light);
    app.wait_for("重新加载").unwrap();

    gw.inject_method_error("home_feed", None);
    app.press("重新加载").unwrap();
    app.wait_for("精选推荐").unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
}

#[test]
fn discover_featured_and_ranking_support_sort_and_back_without_search_chrome() {
    let gw = Arc::new(MockGateway::new());
    let (mut app, _opened) = mount_discover_ready(gw, ColorScheme::Light);

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
    let (mut app, _opened) = mount_discover_ready(gw, ColorScheme::Light);

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

    app.scroll("分页剧目 01", Points(5000.0)).unwrap();
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
    app.scroll("分页剧目 01", Points(5000.0)).unwrap();
    app.wait_for("分页剧目 28").unwrap();
}

#[test]
fn discover_browse_ignores_stale_sort_response() {
    let gw = Arc::new(MockGateway::new().with_paged_catalog(30));
    let (mut app, _opened) = mount_discover_ready(Arc::clone(&gw), ColorScheme::Light);

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
fn discover_home_fullscreen_grows_visible_series() {
    let gw = Arc::new(
        MockGateway::new()
            .with_paged_catalog(20)
            .with_capped_home_feed(5),
    );
    let (mut app, _opened) = mount_discover_ready(Arc::clone(&gw), ColorScheme::Light);
    app.wait(Duration::from_millis(80)).unwrap();

    let all_titles: Vec<String> = (1..=20)
        .map(|i| format!("分页剧目 {i:02}"))
        .chain([
            "云端之上：千金归来".to_owned(),
            "长安暗夜行".to_owned(),
            "重返 1998：科技大亨".to_owned(),
            "雾港七日".to_owned(),
            "春风十里不及卿".to_owned(),
        ])
        .collect();
    let count_visible_featured = |headless: &Headless| -> usize {
        let rank_top = node_bounds(headless, "排行榜").origin.y.0;
        let nodes = headless.screen().layout_nodes();
        all_titles
            .iter()
            .filter(|title| {
                headless
                    .screen()
                    .id(&format!("{title} >> nth=0"))
                    .ok()
                    .and_then(|id| nodes.iter().find(|n| n.id == id))
                    .is_some_and(|n| n.bounds.origin.y.0 < rank_top)
            })
            .count()
    };

    let base_featured = count_visible_featured(&app);
    assert!(
        base_featured >= 4,
        "desktop base width should show a full row (got {base_featured})"
    );
    assert!(app.screen().shows("TOP 4"));
    assert!(!app.screen().shows("TOP 6"));

    app.set_viewport(Size::new(Points(1920.0), Points(1080.0)))
        .unwrap();
    app.wait(Duration::from_millis(80)).unwrap();

    let wide_featured = count_visible_featured(&app);
    assert!(
        wide_featured > base_featured && wide_featured >= 8,
        "fullscreen must dynamically reveal more featured series even when /api/home caps at 5 (base={base_featured}, wide={wide_featured})"
    );
    assert!(
        app.screen().shows("TOP 6"),
        "fullscreen ranking shelf must dynamically reveal more than 4 cards"
    );
}

#[test]
fn watched_series_cards_show_distinct_watched_color() {
    let gw = Arc::new(MockGateway::new());
    let (mut app, _opened) = mount_discover_ready(gw, ColorScheme::Light);
    app.wait_for("全 68 集").unwrap();
    app.wait_for("更新至 72 集").unwrap();

    let watched_fg = app.screen().span_style("全 68 集 >> nth=0").unwrap().color;
    let unwatched_fg = app
        .screen()
        .span_style("更新至 72 集 >> nth=0")
        .unwrap()
        .color;
    assert_ne!(
        watched_fg, unwatched_fg,
        "watched series card must visually mark watched status with distinct color"
    );
}

#[test]
fn ranking_views_display_series_badge() {
    let gw = Arc::new(MockGateway::new());
    let (mut app, _opened) = mount_discover_ready(gw, ColorScheme::Light);

    app.press("查看排行榜全部").unwrap();
    app.wait_for("#1").unwrap();
    app.wait_for("热播 Top 1").unwrap();
}
