use std::time::Duration;

use super::start_on_history;

#[test]
fn removing_and_clearing_watch_history_updates_ui_and_gateway() {
    let (gw, mut app, _opened) = start_on_history();

    app.wait_for("长安暗夜行").unwrap();

    app.press("移除 长安暗夜行 观看记录").unwrap();
    app.wait_for("确认移除").unwrap();
    app.press("取消").unwrap();
    app.wait_for_gone("确认移除", Duration::from_secs(2))
        .unwrap();
    assert!(app.screen().shows("长安暗夜行"));
    assert_eq!(gw.snapshot_watch_history().len(), 2);

    app.press("移除 长安暗夜行 观看记录").unwrap();
    app.wait_for("确认移除").unwrap();
    app.press("确认移除").unwrap();
    app.wait_for_gone("长安暗夜行", Duration::from_secs(2))
        .unwrap();
    assert_eq!(gw.snapshot_watch_history().len(), 1);

    app.press("清空观看历史").unwrap();
    app.wait_for("确认清空").unwrap();
    app.press("取消").unwrap();
    app.wait_for_gone("确认清空", Duration::from_secs(2))
        .unwrap();
    assert_eq!(gw.snapshot_watch_history().len(), 1);

    app.press("清空观看历史").unwrap();
    app.wait_for("确认清空").unwrap();
    app.press("确认清空").unwrap();
    app.wait_for("暂无观看历史").unwrap();
    app.wait_for_gone("第 3 集", Duration::from_secs(2))
        .unwrap();
    assert!(gw.snapshot_watch_history().is_empty());
}
