use crate::app::flags::FeatureFlags;

use super::start_on_watchlist;

#[test]
#[ignore = "待 watchlist/qry_history 实现后激活"]
fn watch_history_shows_recent_items_and_resumes_episode() {
    let mut app = start_on_watchlist(FeatureFlags {
        watchlist_history: true,
        ..FeatureFlags::default()
    });

    app.wait_for("最近观看").unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
    app.wait_for("第 3 集").unwrap();
    app.wait_for("长安暗夜行").unwrap();

    app.press("云端之上：千金归来 >> nth=0").unwrap();
    app.wait_for("选集 (3/68)").unwrap();
}
