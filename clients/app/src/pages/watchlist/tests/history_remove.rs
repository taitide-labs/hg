use std::time::Duration;

use super::start_on_watchlist;

#[test]
fn removing_and_clearing_watch_history_updates_list() {
    let mut app = start_on_watchlist();

    app.wait_for("长安暗夜行").unwrap();
    app.press("移除 长安暗夜行 观看记录").unwrap();
    app.wait_for_gone("长安暗夜行", Duration::from_secs(2))
        .unwrap();

    app.press("清空观看历史").unwrap();
    app.wait_for("暂无观看历史").unwrap();
    app.wait_for_gone("第 3 集", Duration::from_secs(2))
        .unwrap();
}
