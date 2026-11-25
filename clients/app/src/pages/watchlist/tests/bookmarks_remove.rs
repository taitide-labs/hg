use std::time::Duration;

use super::start_on_watchlist;

#[test]
fn unbookmarking_series_updates_ui_and_gateway_until_empty() {
    let (gw, mut app, _opened) = start_on_watchlist();

    app.wait_for("雾港七日").unwrap();
    app.press("取消追剧 雾港七日").unwrap();
    app.wait_for_gone("雾港七日", Duration::from_secs(2))
        .unwrap();
    assert_eq!(gw.snapshot_bookmarks().len(), 1);

    app.press("取消追剧 云端之上：千金归来").unwrap();
    app.wait_for("暂无追剧收藏").unwrap();
    assert!(gw.snapshot_bookmarks().is_empty());
}
