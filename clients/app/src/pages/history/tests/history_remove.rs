use std::time::Duration;

use nsh_testing::DialogAnswer;

use super::start_on_history;

#[test]
fn removing_and_clearing_watch_history_updates_ui_and_gateway() {
    let (gw, mut app) = start_on_history();

    app.wait_for("长安暗夜行").unwrap();

    app.on_dialog(|_| DialogAnswer::Press("取消".to_owned()));
    app.press("移除 长安暗夜行 观看记录").unwrap();
    app.wait(Duration::from_millis(40)).unwrap();
    assert!(app.screen().shows("长安暗夜行"));
    assert_eq!(gw.snapshot_watch_history().len(), 2);

    app.on_dialog(|_| DialogAnswer::Press("移除".to_owned()));
    app.press("移除 长安暗夜行 观看记录").unwrap();
    app.wait_for_gone("长安暗夜行", Duration::from_secs(2))
        .unwrap();
    assert_eq!(gw.snapshot_watch_history().len(), 1);

    app.on_dialog(|_| DialogAnswer::Press("取消".to_owned()));
    app.press("清空观看历史").unwrap();
    app.wait(Duration::from_millis(40)).unwrap();
    assert_eq!(gw.snapshot_watch_history().len(), 1);

    app.on_dialog(|_| DialogAnswer::Press("清空".to_owned()));
    app.press("清空观看历史").unwrap();
    app.wait_for("暂无观看历史").unwrap();
    app.wait_for_gone("第 3 集", Duration::from_secs(2))
        .unwrap();
    assert!(gw.snapshot_watch_history().is_empty());
}
