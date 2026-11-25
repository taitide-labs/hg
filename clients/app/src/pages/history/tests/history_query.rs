use std::sync::Arc;

use hg_contracts::error::ContractError;
use hg_gateway::mock::MockGateway;
use nsh_engine::ColorScheme;

use super::{start_on_history, start_on_history_with};
use crate::pages::tests::{DESKTOP_VIEWPORT, assert_above, assert_visible_in_viewport};

#[test]
fn watch_history_layout_shows_recent_items_and_resumes_saved_episode() {
    let (_gw, mut app) = start_on_history();

    app.wait_for("最近观看").unwrap();
    assert!(!app.screen().shows("我的追剧"));
    app.wait_for("云端之上：千金归来").unwrap();
    app.wait_for("第 3 集").unwrap();
    app.wait_for("长安暗夜行").unwrap();
    assert_visible_in_viewport(&app, "最近观看", DESKTOP_VIEWPORT);
    assert_above(&app, "最近观看", "云端之上：千金归来");
    assert_above(&app, "云端之上：千金归来", "长安暗夜行");

    app.press("云端之上：千金归来 >> nth=0").unwrap();
    app.wait_for("选集 (3/68)").unwrap();
}

#[test]
fn empty_watch_history_shows_empty_state_without_clear_button() {
    let gw = Arc::new(MockGateway::new().with_empty_watch_history());
    let (_gw, mut app) = start_on_history_with(gw, ColorScheme::Dark);

    app.wait_for("暂无观看历史").unwrap();
    assert!(!app.screen().shows("清空观看历史"));
}

#[test]
#[ignore = "BUG: 历史页错误态内的「重新加载」位于 dynamic 子树，点击后切换 loading_history 导致父 dynamic 卸载并取消刚 spawn 的 watch_history 异步任务，页面永久卡在加载态"]
fn watch_history_error_recovers_on_retry() {
    let gw = Arc::new(MockGateway::new());
    gw.inject_method_error(
        "watch_history",
        Some(ContractError::NetworkUnavailable {
            detail: "history down".to_owned(),
        }),
    );
    let (_gw, mut app) = start_on_history_with(Arc::clone(&gw), ColorScheme::Light);
    app.wait_for("观看历史加载失败").unwrap();

    gw.inject_method_error("watch_history", None);
    app.press("重新加载").unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
}
