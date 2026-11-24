use std::time::Duration;

use crate::app::flags::FeatureFlags;

use super::start_on_watchlist;

#[test]
#[ignore = "待 watchlist/cmd_unbookmark 实现后激活"]
fn unbookmarking_series_removes_item_from_watchlist() {
    let mut app = start_on_watchlist(FeatureFlags {
        watchlist_bookmarks: true,
        watchlist_unbookmark: true,
        ..FeatureFlags::default()
    });

    app.wait_for("雾港七日").unwrap();
    app.press("取消追剧 雾港七日").unwrap();
    app.wait_for_gone("雾港七日", Duration::from_secs(2))
        .unwrap();
}
