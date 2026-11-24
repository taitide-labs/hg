use std::time::Duration;

use super::start_on_watchlist;

#[test]
fn unbookmarking_series_removes_item_from_watchlist() {
    let mut app = start_on_watchlist();

    app.wait_for("雾港七日").unwrap();
    app.press("取消追剧 雾港七日").unwrap();
    app.wait_for_gone("雾港七日", Duration::from_secs(2))
        .unwrap();
}
