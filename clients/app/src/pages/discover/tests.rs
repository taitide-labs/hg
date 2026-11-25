use std::future::Future;
use std::num::{NonZeroU32, NonZeroU64};
use std::pin::pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use hg_contracts::error::ContractError;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId};
use hg_contracts::viewing::ReportWatchProgressRequest;
use hg_gateway::contract::AppGateway as _;
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
    app.wait_for("最近更新").unwrap();
    (app, opened)
}

#[test]
fn discover_home_layout_invariants_and_no_embedded_search_or_hero() {
    let gw = Arc::new(MockGateway::new());
    let (mut app, opened) = mount_discover_ready(gw, ColorScheme::Light);
    let screen = app.screen();

    for expected in ["最近更新", "排行榜", "热门短剧", "漫剧与悬疑"] {
        assert!(screen.shows(expected), "missing section `{expected}`");
        assert_visible_in_viewport(&app, expected, DESKTOP_VIEWPORT);
    }
    assert!(!screen.shows("精选推荐"));
    assert!(!screen.shows("查看精选推荐全部"));
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
    app.wait_for("最近更新").unwrap();
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
    app.wait_for("查看最近更新全部").unwrap();

    app.press("查看最近更新全部").unwrap();
    app.wait_for("春风十里不及卿").unwrap();
    app.wait_for("最热").unwrap();
    app.wait_for("最近更新").unwrap();
    assert!(!app.screen().shows("热搜"));
    assert!(!app.screen().shows("题材"));
    app.press("最热").unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
}

#[test]
fn discover_browse_and_ranking_paginate_via_infinite_scroll_and_adapt_columns() {
    let gw = Arc::new(MockGateway::new().with_paged_catalog(30));
    let (mut app, _opened) = mount_discover_ready(gw, ColorScheme::Light);

    app.press("查看热门短剧全部").unwrap();
    app.wait_for("分页剧目 01").unwrap();
    assert!(!app.screen().shows("加载更多"));
    assert!(!app.screen().shows("分页剧目 28"));

    app.press("最近更新").unwrap();
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
    app.wait_for("分页剧目 26").unwrap();
    assert!(!app.screen().shows("加载更多"));

    app.press("最近更新").unwrap();
    app.wait_for("分页剧目 30").unwrap();
    assert!(!app.screen().shows("分页剧目 01"));

    app.press("返回").unwrap();
    app.wait_for("查看排行榜全部").unwrap();
    app.press("查看排行榜全部").unwrap();
    app.wait_for("分页剧目 01").unwrap();
    assert!(!app.screen().shows("加载更多"));
    app.scroll("分页剧目 01", Points(5000.0)).unwrap();
    app.wait_for("分页剧目 26").unwrap();
}

#[test]
fn discover_browse_ignores_stale_sort_response() {
    let gw = Arc::new(MockGateway::new().with_paged_catalog(30));
    let (mut app, _opened) = mount_discover_ready(Arc::clone(&gw), ColorScheme::Light);

    let slow_hot_gate = gw.queue_method_gate("list_series");
    app.press("查看热门短剧全部").unwrap();
    app.wait_for("最近更新").unwrap();

    app.press("最近更新").unwrap();
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
    let count_visible_featured = |headless: &Headless, vp_width: f32| -> usize {
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
                    .is_some_and(|n| {
                        n.bounds.origin.y.0 < rank_top
                            && n.bounds.origin.x.0 + n.bounds.size.width.0 <= vp_width + 4.0
                    })
            })
            .count()
    };

    let base_featured = count_visible_featured(&app, 1240.0);
    let base_card_w = node_bounds(&app, "label:分页剧目 20 >> nth=0").size.width.0;
    assert_eq!(
        base_featured, 4,
        "desktop base width should show 4 cards across (got {base_featured})"
    );
    assert!(app.screen().shows("TOP 4"));
    assert!(!app.screen().shows("TOP 6"));

    app.set_viewport(Size::new(Points(1920.0), Points(1080.0)))
        .unwrap();
    app.wait(Duration::from_millis(80)).unwrap();

    let wide_featured = count_visible_featured(&app, 1920.0);
    let wide_card_w = node_bounds(&app, "label:分页剧目 20 >> nth=0").size.width.0;
    assert_eq!(
        wide_featured, 6,
        "4K fullscreen must cap desktop columns at 6 (base={base_featured}, wide={wide_featured})"
    );
    assert!(
        wide_card_w >= base_card_w - 1.0,
        "4K fullscreen poster cards must not shrink below base window width (base={base_card_w}, wide={wide_card_w})"
    );
    assert!(
        app.screen().shows("TOP 6"),
        "fullscreen ranking shelf must dynamically reveal more than 4 cards"
    );
}

fn block_ready<F: Future>(future: F) -> F::Output {
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    match pin!(future).poll(&mut cx) {
        Poll::Ready(out) => out,
        Poll::Pending => panic!("mock gateway future should resolve immediately"),
    }
}

#[test]
fn watched_series_cards_show_distinct_watched_color() {
    let gw = Arc::new(MockGateway::new());
    let s1001 = SeriesId::new(NonZeroU64::new(1001).unwrap());
    let s1003 = SeriesId::new(NonZeroU64::new(1003).unwrap());
    let ep68 = EpisodeNumber::new(NonZeroU32::new(68).unwrap());
    let ep72 = EpisodeNumber::new(NonZeroU32::new(72).unwrap());
    block_ready(gw.report_watch_progress(ReportWatchProgressRequest {
        key: EpisodeKey::new(s1001, ep68),
        position_secs: 110,
        duration_secs: 120,
    }))
    .unwrap();
    block_ready(gw.report_watch_progress(ReportWatchProgressRequest {
        key: EpisodeKey::new(s1003, ep72),
        position_secs: 110,
        duration_secs: 120,
    }))
    .unwrap();
    gw.set_series_episode_count(s1003, 75);

    let (mut app, _opened) = mount_discover_ready(gw, ColorScheme::Light);
    app.wait_for("全 68 集").unwrap();
    app.wait_for("全 80 集").unwrap();
    app.wait_for("有更新 · 75 集").unwrap();

    let all_watched_fg = app.screen().span_style("全 68 集 >> nth=0").unwrap().color;
    let partial_unwatched_fg = app.screen().span_style("全 80 集 >> nth=0").unwrap().color;
    let new_unwatched_fg = app
        .screen()
        .span_style("更新至 48 集 >> nth=0")
        .unwrap()
        .color;
    let updated_fg = app
        .screen()
        .span_style("有更新 · 75 集 >> nth=0")
        .unwrap()
        .color;

    assert_eq!(
        partial_unwatched_fg, new_unwatched_fg,
        "partially watched series must stay default poster badge color"
    );
    assert_ne!(
        all_watched_fg, new_unwatched_fg,
        "fully watched series card must mark watched green color"
    );
    assert_ne!(
        updated_fg, new_unwatched_fg,
        "updated-after-watching series must mark distinct alert color"
    );
    assert_ne!(
        updated_fg, all_watched_fg,
        "updated-after-watching alert color must differ from all-watched green"
    );
}

#[test]
fn discover_and_ranking_views_render_combined_original_hot_and_new_badges() {
    let gw = Arc::new(MockGateway::new());
    let (mut app, _opened) = mount_discover_ready(gw, ColorScheme::Light);

    app.wait_for("9.4分").unwrap();
    assert!(app.screen().shows("原创"));
    assert!(app.screen().shows("热播"));
    assert!(app.screen().shows("新片"));

    app.press("查看排行榜全部").unwrap();
    app.wait_for("#1").unwrap();
    assert!(app.screen().shows("原创"));
    assert!(app.screen().shows("热播"));
}

#[test]
fn discover_poster_card_uses_compact_spacing_and_minimal_immersive_inset() {
    let gw = Arc::new(MockGateway::new().with_paged_catalog(12));
    let (mut app, _opened) = mount_discover_ready(gw, ColorScheme::Dark);

    app.press("查看热门短剧全部").unwrap();
    app.wait_for("分页剧目 01").unwrap();
    app.wait_for("分页剧目 02").unwrap();

    let first_card = node_bounds(&app, "label:分页剧目 01");
    let second_card = node_bounds(&app, "label:分页剧目 02");
    let col_gap = second_card.origin.x.0 - (first_card.origin.x.0 + first_card.size.width.0);
    assert!(
        (col_gap - 12.0).abs() <= 2.0,
        "desktop poster grid gap should match ranking 12px gap, got {col_gap}"
    );

    let title_bounds = node_bounds(&app, "text:分页剧目 01");
    let card_inset = title_bounds.origin.x.0 - first_card.origin.x.0;
    assert!(
        card_inset <= 6.0,
        "immersive borderless poster card must keep minimal inset <= 6px, got {card_inset}"
    );
}

#[test]
fn poster_card_centers_play_overlay_and_reveals_only_on_hover() {
    let gw = Arc::new(MockGateway::new());
    let (mut app, _opened) = mount_discover_ready(gw, ColorScheme::Dark);

    let selector = "label:悬浮播放 春风十里不及卿 >> nth=0";
    let rest_opacity = app
        .screen()
        .view_style(selector)
        .unwrap()
        .map_or(1.0, |s| s.opacity);
    assert!(
        rest_opacity < 0.05,
        "poster play icon should be hidden at rest, got opacity={rest_opacity}"
    );

    let card_bounds = node_bounds(&app, "label:春风十里不及卿 >> nth=0");
    let play_bounds = node_bounds(&app, selector);
    let play_cx = play_bounds.origin.x.0 + play_bounds.size.width.0 / 2.0;
    let card_cx = card_bounds.origin.x.0 + card_bounds.size.width.0 / 2.0;
    assert!(
        (play_cx - card_cx).abs() <= 4.0,
        "play overlay must be horizontally centered inside poster (play_cx={play_cx}, card_cx={card_cx})"
    );

    app.hover("label:春风十里不及卿 >> nth=0").unwrap();
    app.wait_until("hover play visible", Duration::from_secs(1), |screen| {
        screen
            .view_style(selector)
            .ok()
            .flatten()
            .is_some_and(|s| s.opacity > 0.9)
    })
    .unwrap();
}

#[test]
fn ranking_list_uses_responsive_multicolumn_high_density_cards() {
    let gw = Arc::new(MockGateway::new());
    let (mut app, _opened) = mount_discover_ready(gw, ColorScheme::Light);

    app.press("查看排行榜全部").unwrap();
    app.wait_for("#1").unwrap();
    app.wait_for("#2").unwrap();
    app.wait_for("9.4 分 · 都市 · 逆袭 · 女频 · 68 集").unwrap();
    assert!(!app.screen().shows("热度 98.4万"));
    assert!(!app.screen().shows("热力指数"));
    assert!(!app.screen().shows("飙升指数"));
    assert!(!app.screen().shows("口碑推荐"));

    let b1 = node_bounds(&app, "label:#1 云端之上：千金归来");
    let b2 = node_bounds(&app, "label:#2 长安暗夜行");
    assert!(
        (b1.origin.y.0 - b2.origin.y.0).abs() < 4.0 && b2.origin.x.0 > b1.origin.x.0 + 100.0,
        "1240px desktop ranking list should place #1 and #2 side-by-side (b1={b1:?}, b2={b2:?})"
    );

    app.set_viewport(Size::new(Points(1920.0), Points(1080.0)))
        .unwrap();
    app.wait(Duration::from_millis(60)).unwrap();
    let w1 = node_bounds(&app, "label:#1 云端之上：千金归来");
    let w2 = node_bounds(&app, "label:#2 长安暗夜行");
    let w3 = node_bounds(&app, "label:#3 重返 1998：科技大亨");
    assert!(
        (w1.origin.y.0 - w3.origin.y.0).abs() < 4.0 && w3.origin.x.0 > w2.origin.x.0 + 100.0,
        "1920px wide ranking list should place #1..#3 in 3 columns (w1={w1:?}, w2={w2:?}, w3={w3:?})"
    );

    app.set_viewport(Size::new(Points(560.0), Points(800.0)))
        .unwrap();
    app.wait(Duration::from_millis(60)).unwrap();
    let n1 = node_bounds(&app, "label:#1 云端之上：千金归来");
    let n2 = node_bounds(&app, "label:#2 长安暗夜行");
    assert!(
        n2.origin.y.0 > n1.origin.y.0 + 40.0,
        "narrow viewport should collapse ranking list to 1 column (n1={n1:?}, n2={n2:?})"
    );
}

#[test]
fn discover_home_shows_latest_updates_shelf_and_renames_new_sort_to_recently_updated() {
    let gw = Arc::new(MockGateway::new().with_paged_catalog(30));
    let (mut app, _opened) = mount_discover_ready(gw, ColorScheme::Light);

    assert!(app.screen().shows("最近更新"));
    app.press("查看最近更新全部").unwrap();
    app.wait_for("分页剧目 30").unwrap();
    assert!(!app.screen().shows("最新"));
    assert!(app.screen().shows("最近更新"));
    app.press("最热").unwrap();
    app.wait_for("分页剧目 01").unwrap();
}

#[test]
fn desktop_series_cards_use_roomy_poster_width_and_spacing() {
    let gw = Arc::new(MockGateway::new());
    let (mut app, _opened) = mount_discover_ready(gw, ColorScheme::Light);
    app.wait_for("云端之上：千金归来").unwrap();

    let card0 = node_bounds(&app, "label:春风十里不及卿 >> nth=0");
    let card1 = node_bounds(&app, "label:雾港七日 >> nth=0");
    assert!(
        card0.size.width.0 >= 200.0,
        "desktop series card should be roomy >= 200pt wide, got {}",
        card0.size.width.0
    );
    let horizontal_gap = card1.origin.x.0 - (card0.origin.x.0 + card0.size.width.0);
    assert!(
        (horizontal_gap - 12.0).abs() <= 2.0,
        "desktop series grid spacing should match ranking 12px gap, got {horizontal_gap}"
    );
}

#[test]
fn browse_pagination_virtualizes_and_preserves_visible_card_nodes() {
    let gw = Arc::new(MockGateway::new().with_paged_catalog(30));
    let (mut app, _opened) = mount_discover_ready(gw, ColorScheme::Light);
    app.set_viewport(Size::new(Points(1240.0), Points(1600.0)))
        .unwrap();

    app.press("查看热门短剧全部").unwrap();
    app.wait_for("分页剧目 01").unwrap();
    let before_id = app.screen().id("分页剧目 01").unwrap();

    app.scroll("分页剧目 01", Points(1800.0)).unwrap();
    app.wait_for("分页剧目 28").unwrap();
    let after_id = app.screen().id("分页剧目 01").unwrap();
    assert_eq!(
        after_id, before_id,
        "appending page 2 must not teardown and remount existing page 1 cards"
    );
}

#[test]
fn large_viewport_adapts_page_size_and_fills_screen_on_resize() {
    let gw = Arc::new(MockGateway::new().with_paged_catalog(60));
    let (mut app, _opened) = mount_discover_ready(gw, ColorScheme::Light);

    app.set_viewport(Size::new(Points(2560.0), Points(1440.0)))
        .unwrap();
    app.wait(Duration::from_millis(40)).unwrap();

    app.press("查看最近更新全部").unwrap();
    app.wait_for("分页剧目 31").unwrap();
    assert!(
        app.screen().shows("分页剧目 31"),
        "wide tall viewport must scale initial page size past 24 so half the screen is never left empty"
    );

    app.press("返回").unwrap();
    app.wait_for("查看排行榜全部").unwrap();
    app.set_viewport(DESKTOP_VIEWPORT).unwrap();
    app.wait(Duration::from_millis(40)).unwrap();

    app.press("查看热门短剧全部").unwrap();
    app.wait_for("分页剧目 01").unwrap();
    assert!(!app.screen().shows("分页剧目 24"));

    app.set_viewport(Size::new(Points(2560.0), Points(1440.0)))
        .unwrap();
    app.wait_for("分页剧目 24").unwrap();
    assert_eq!(
        crate::features::discover::parts::adaptive_page_size(1240.0, 720.0, false),
        24
    );
    assert!(crate::features::discover::parts::adaptive_page_size(2560.0, 1440.0, false) >= 36);
}
