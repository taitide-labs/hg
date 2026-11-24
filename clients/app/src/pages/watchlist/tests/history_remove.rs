use std::time::Duration;

use crate::app::flags::FeatureFlags;

use super::start_on_watchlist;

#[test]
fn removing_and_clearing_watch_history_updates_list() {
    let mut app = start_on_watchlist(FeatureFlags {
        watchlist_history: true,
        watchlist_remove_history: true,
        ..FeatureFlags::default()
    });

    app.wait_for("长安暗夜行").unwrap();
    app.press("移除 长安暗夜行 观看记录").unwrap();
    app.wait_for_gone("长安暗夜行", Duration::from_secs(2))
        .unwrap();

    app.press("清空观看历史").unwrap();
    app.wait_for_gone("云端之上：千金归来", Duration::from_secs(2))
        .unwrap();
}
