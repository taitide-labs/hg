use std::sync::Arc;

use hg_contracts::error::ContractError;
use hg_gateway::mock::MockGateway;
use nsh_engine::ColorScheme;

use super::{start_on_history, start_on_history_with};
use crate::pages::tests::{DESKTOP_VIEWPORT, assert_above, assert_visible_in_viewport};

#[test]
fn watch_history_layout_shows_recent_items_and_emits_resume_episode() {
    let (_gw, mut app, opened) = start_on_history();

    app.wait_for("最近观看").unwrap();
    assert!(!app.screen().shows("我的追剧"));
    app.wait_for("云端之上：千金归来").unwrap();
    app.wait_for("第 3 集").unwrap();
    app.wait_for("长安暗夜行").unwrap();
    assert_visible_in_viewport(&app, "最近观看", DESKTOP_VIEWPORT);
    assert_above(&app, "最近观看", "云端之上：千金归来");
    assert_above(&app, "云端之上：千金归来", "长安暗夜行");

    app.press("云端之上：千金归来 >> nth=0").unwrap();
    let emitted = opened
        .last()
        .expect("pressing history row must emit open intent");
    assert_eq!(emitted.0.get(), 1001);
    assert_eq!(emitted.1.map(|ep| ep.get()), Some(3));
}

#[test]
fn empty_watch_history_shows_empty_state_without_clear_button() {
    let gw = Arc::new(MockGateway::new().with_empty_watch_history());
    let (_gw, mut app, _opened) = start_on_history_with(gw, ColorScheme::Dark);

    app.wait_for("暂无观看历史").unwrap();
    assert!(!app.screen().shows("清空观看历史"));
}

#[test]
fn watch_history_error_recovers_on_retry() {
    let gw = Arc::new(MockGateway::new());
    gw.inject_method_error(
        "watch_history",
        Some(ContractError::NetworkUnavailable {
            detail: "history down".to_owned(),
        }),
    );
    let (_gw, mut app, _opened) = start_on_history_with(Arc::clone(&gw), ColorScheme::Light);
    app.wait_for("观看历史加载失败").unwrap();

    gw.inject_method_error("watch_history", None);
    app.press("重新加载").unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
}
