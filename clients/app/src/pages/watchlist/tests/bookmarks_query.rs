use std::sync::Arc;

use hg_contracts::error::ContractError;
use hg_gateway::mock::MockGateway;
use nsh_engine::ColorScheme;

use super::{start_on_watchlist, start_on_watchlist_with};
use crate::pages::tests::{DESKTOP_VIEWPORT, assert_above, assert_visible_in_viewport};

#[test]
fn bookmarks_list_layout_shows_saved_series_and_emits_open_intent() {
    let (_gw, mut app, opened) = start_on_watchlist();

    app.wait_for("我的追剧").unwrap();
    assert!(!app.screen().shows("最近观看"));
    app.wait_for("云端之上：千金归来").unwrap();
    app.wait_for("雾港七日").unwrap();
    assert_visible_in_viewport(&app, "我的追剧", DESKTOP_VIEWPORT);
    assert_above(&app, "我的追剧", "云端之上：千金归来");
    assert_above(&app, "云端之上：千金归来", "雾港七日");

    app.press("雾港七日 >> nth=0").unwrap();
    let emitted = opened
        .last()
        .expect("pressing bookmark must emit open intent");
    assert_eq!(emitted.0.get(), 1005);
    assert_eq!(emitted.1, None);
}

#[test]
fn empty_bookmarks_shows_empty_state() {
    let gw = Arc::new(MockGateway::new().with_empty_bookmarks());
    let (_gw, mut app, _opened) = start_on_watchlist_with(gw, ColorScheme::Dark);

    app.wait_for("暂无追剧收藏").unwrap();
}

#[test]
fn bookmarks_error_recovers_on_retry() {
    let gw = Arc::new(MockGateway::new());
    gw.inject_method_error(
        "bookmarks",
        Some(ContractError::NetworkUnavailable {
            detail: "bookmarks down".to_owned(),
        }),
    );
    let (_gw, mut app, _opened) = start_on_watchlist_with(Arc::clone(&gw), ColorScheme::Light);
    app.wait_for("追剧收藏加载失败").unwrap();

    gw.inject_method_error("bookmarks", None);
    app.press("重新加载").unwrap();
    app.wait_for("雾港七日").unwrap();
}
