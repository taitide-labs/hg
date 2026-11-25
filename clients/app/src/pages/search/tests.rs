use std::sync::Arc;
use std::time::Duration;

use hg_contracts::catalog::{TagGroup, TagItem};
use hg_contracts::error::ContractError;
use hg_gateway::mock::MockGateway;
use nsh_core::prelude::px;
use nsh_engine::ColorScheme;
use nsh_testing::DialogAnswer;

use crate::pages::search::page::search_page;
use crate::pages::tests::{
    DESKTOP_VIEWPORT, assert_above, assert_visible_in_viewport, mount_desktop_page,
};

fn start_on_search(gw: Arc<MockGateway>, scheme: ColorScheme) -> nsh_testing::Headless {
    let mut app = mount_desktop_page(gw, scheme, || search_page(|_| {}));
    app.wait_for("热搜").unwrap();
    app
}

#[test]
fn search_page_initial_layout_shows_controls_and_empty_results() {
    let gw = Arc::new(MockGateway::new());
    let app = start_on_search(gw, ColorScheme::Dark);

    assert_visible_in_viewport(&app, "text:搜索剧名、标签...", DESKTOP_VIEWPORT);
    assert_visible_in_viewport(&app, "label:执行搜索", DESKTOP_VIEWPORT);
    assert_visible_in_viewport(&app, "搜索历史", DESKTOP_VIEWPORT);
    assert_visible_in_viewport(&app, "热搜", DESKTOP_VIEWPORT);
    assert_visible_in_viewport(&app, "题材", DESKTOP_VIEWPORT);
    assert_above(&app, "text:搜索剧名、标签...", "热搜");
    assert_above(&app, "热搜", "题材");

    assert!(!app.screen().shows("春风十里不及卿"));
    assert!(!app.screen().shows("全 80 集"));
}

#[test]
fn search_page_keyword_hot_word_history_crud_and_tag_toggle_matrix() {
    let gw = Arc::new(MockGateway::new());
    let mut app = start_on_search(Arc::clone(&gw), ColorScheme::Light);

    app.type_text("text:搜索剧名、标签...", "暗夜").unwrap();
    app.press("label:执行搜索").unwrap();
    app.wait_for("全 80 集").unwrap();
    assert!(!app.screen().shows("春风十里不及卿"));
    assert!(
        gw.snapshot_search_history()
            .iter()
            .any(|entry| entry.keyword == "暗夜")
    );

    app.press("label:删除搜索历史 千金").unwrap();
    app.wait_for_gone("label:删除搜索历史 千金", Duration::from_secs(2))
        .unwrap();
    assert!(
        !gw.snapshot_search_history()
            .iter()
            .any(|entry| entry.keyword == "千金")
    );

    app.on_dialog(|_| DialogAnswer::Press("取消".to_owned()));
    app.press("label:清空搜索历史").unwrap();
    app.wait(Duration::from_millis(40)).unwrap();
    assert!(
        app.screen().shows("搜索历史"),
        "cancelling clear search history dialog must preserve history"
    );
    assert!(!gw.snapshot_search_history().is_empty());

    app.on_dialog(|_| DialogAnswer::Press("清空".to_owned()));
    app.press("label:清空搜索历史").unwrap();
    app.wait_for_gone("搜索历史", Duration::from_secs(2))
        .unwrap();
    assert!(gw.snapshot_search_history().is_empty());

    app.press("千金归来").unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
    app.wait_for("搜索历史").unwrap();
    assert_eq!(
        gw.snapshot_search_history()
            .first()
            .map(|entry| entry.keyword.as_str()),
        Some("千金归来")
    );

    app.press("悬疑 >> nth=0").unwrap();
    app.wait_for("最热").unwrap();
    app.wait_for("最近更新").unwrap();
    app.wait_for("全 80 集").unwrap();

    app.press("悬疑 >> nth=0").unwrap();
    app.wait_for_gone("全 80 集", Duration::from_secs(2))
        .unwrap();
}

#[test]
fn search_page_keyword_and_tag_paginate_via_infinite_scroll_without_load_more() {
    let gw = Arc::new(MockGateway::new().with_paged_catalog(30));
    let mut app = start_on_search(gw, ColorScheme::Light);

    app.type_text("text:搜索剧名、标签...", "分页剧目").unwrap();
    app.press("label:执行搜索").unwrap();
    app.wait_for("分页剧目 01").unwrap();
    assert!(!app.screen().shows("加载更多"));
    assert!(!app.screen().shows("分页剧目 28"));

    app.scroll("分页剧目 01", px(5000.0)).unwrap();
    app.wait_for("分页剧目 28").unwrap();
    assert!(!app.screen().shows("加载更多"));

    app.press("悬疑 >> nth=0").unwrap();
    app.wait_for("分页剧目 01").unwrap();
    app.scroll("分页剧目 01", px(5000.0)).unwrap();
    app.wait_for("分页剧目 28").unwrap();

    app.press("最近更新").unwrap();
    app.wait_for("分页剧目 30").unwrap();
    assert!(!app.screen().shows("分页剧目 01"));
}

#[test]
fn search_page_keyword_error_recovers_on_retry() {
    let gw = Arc::new(MockGateway::new());
    let mut app = start_on_search(Arc::clone(&gw), ColorScheme::Light);

    gw.inject_method_error(
        "search_series",
        Some(ContractError::NetworkUnavailable {
            detail: "search down".to_owned(),
        }),
    );
    app.type_text("text:搜索剧名、标签...", "暗夜").unwrap();
    app.press("label:执行搜索").unwrap();
    app.wait_for("搜索加载失败").unwrap();

    gw.inject_method_error("search_series", None);
    app.press("重试").unwrap();
    app.wait_for("长安暗夜行").unwrap();
}

#[test]
fn search_page_tag_error_retry_reloads_selected_tag() {
    let gw = Arc::new(MockGateway::new());
    let mut app = start_on_search(Arc::clone(&gw), ColorScheme::Light);

    gw.inject_method_error(
        "list_series",
        Some(ContractError::NetworkUnavailable {
            detail: "tag browse down".to_owned(),
        }),
    );
    app.press("悬疑 >> nth=0").unwrap();
    app.wait_for("搜索加载失败").unwrap();

    gw.inject_method_error("list_series", None);
    app.press("重试").unwrap();
    app.wait_for("长安暗夜行").unwrap();
}

#[test]
fn search_page_ignores_stale_keyword_response() {
    let gw = Arc::new(MockGateway::new());
    let mut app = start_on_search(Arc::clone(&gw), ColorScheme::Light);

    let slow_first = gw.queue_method_gate("search_series");
    app.type_text("text:搜索剧名、标签...", "暗夜").unwrap();
    app.press("label:执行搜索").unwrap();
    app.wait_for("正在搜索...").unwrap();

    app.press("千金归来").unwrap();
    app.wait_for("云端之上：千金归来").unwrap();

    slow_first.release();
    app.wait(Duration::from_millis(60)).unwrap();
    assert!(app.screen().shows("云端之上：千金归来"));
    assert!(!app.screen().shows("全 80 集"));
}

#[test]
fn search_input_clear_icon_clears_query_and_results() {
    let gw = Arc::new(MockGateway::new());
    let mut app = start_on_search(gw, ColorScheme::Light);

    assert!(!app.screen().shows("label:清空搜索"));

    app.type_text("text:搜索剧名、标签...", "暗夜").unwrap();
    app.wait_for("label:清空搜索").unwrap();
    app.press("label:执行搜索").unwrap();
    app.wait_for("全 80 集").unwrap();

    app.press("label:清空搜索").unwrap();
    app.wait_for_gone("label:清空搜索", Duration::from_secs(2))
        .unwrap();
    app.wait_for_gone("全 80 集", Duration::from_secs(2))
        .unwrap();
}

#[test]
fn search_selected_tag_shows_clear_icon_and_clears_active_filter() {
    let gw = Arc::new(MockGateway::new());
    let mut app = start_on_search(gw, ColorScheme::Dark);

    assert!(!app.screen().shows("label:清除标签 悬疑"));

    app.press("悬疑 >> nth=0").unwrap();
    app.wait_for("全 80 集").unwrap();
    app.wait_for("label:清除标签 悬疑").unwrap();

    app.press("label:清除标签 悬疑").unwrap();
    app.wait_for_gone("label:清除标签 悬疑", Duration::from_secs(2))
        .unwrap();
    app.wait_for_gone("全 80 集", Duration::from_secs(2))
        .unwrap();
}

#[test]
fn search_tags_hide_empty_groups_and_collapse_multiline_groups_behind_plus_icon() {
    let long_tags: Vec<TagItem> = (1..=24_u32)
        .map(|i| TagItem {
            id: 500 + i,
            name: format!("长标签{i:02}"),
            slug: format!("tag-{i:02}"),
            is_hot: i <= 3,
        })
        .collect();
    let gw = Arc::new(MockGateway::new().with_tag_groups(vec![
        TagGroup {
            id: 10,
            code: "long".to_owned(),
            name: "长题材".to_owned(),
            tags: long_tags,
        },
        TagGroup {
            id: 11,
            code: "empty".to_owned(),
            name: "吃瓜黑料".to_owned(),
            tags: Vec::new(),
        },
    ]));
    let mut app = start_on_search(gw, ColorScheme::Light);

    app.wait_for("长题材").unwrap();
    assert!(
        !app.screen().shows("吃瓜黑料"),
        "empty tag group with zero tags must not be rendered"
    );
    assert!(app.screen().shows("长标签01"));
    assert!(
        !app.screen().shows("长标签24"),
        "multi-line tag group must fold overflow tags by default"
    );
    app.wait_for("label:展开 长题材 标签").unwrap();

    app.press("label:展开 长题材 标签").unwrap();
    app.wait_for("长标签24").unwrap();
    app.wait_for("label:收起 长题材 标签").unwrap();

    app.press("label:收起 长题材 标签").unwrap();
    app.wait_for_gone("长标签24", Duration::from_secs(2))
        .unwrap();
}

#[test]
fn search_large_viewport_shows_exhausted_footer() {
    let gw = Arc::new(MockGateway::new().with_paged_catalog(30));
    let mut app = start_on_search(gw, ColorScheme::Light);
    app.set_viewport(nsh_core::prelude::Size::new(
        nsh_core::prelude::Points(2560.0),
        nsh_core::prelude::Points(1440.0),
    ))
    .unwrap();
    app.wait(Duration::from_millis(40)).unwrap();

    app.type_text("text:搜索剧名、标签...", "分页剧目").unwrap();
    app.press("label:执行搜索").unwrap();
    app.wait_for("分页剧目 30").unwrap();
    app.wait_for("已全部加载").unwrap();
}

#[test]
fn search_tags_filter_using_chinese_tag_name_not_pinyin_slug() {
    let gw = Arc::new(MockGateway::new().with_tag_groups(vec![TagGroup {
        id: 9,
        code: "theme".to_owned(),
        name: "题材".to_owned(),
        tags: vec![TagItem {
            id: 99,
            name: "都市".to_owned(),
            slug: "urban-unmapped-slug".to_owned(),
            is_hot: true,
        }],
    }]));
    let mut app = start_on_search(gw, ColorScheme::Light);

    app.press("都市 >> nth=0").unwrap();
    app.wait_for("最热").unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
    app.wait_for("春风十里不及卿").unwrap();
    assert!(!app.screen().shows("全 80 集"));
}
