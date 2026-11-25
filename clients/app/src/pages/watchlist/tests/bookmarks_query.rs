use super::start_on_watchlist;

#[test]
fn bookmarks_list_shows_saved_series_and_opens_player() {
    let mut app = start_on_watchlist();

    app.wait_for("我的追剧").unwrap();
    assert!(!app.screen().shows("最近观看"));
    app.wait_for("云端之上：千金归来").unwrap();
    app.wait_for("雾港七日").unwrap();

    app.press("雾港七日 >> nth=0").unwrap();
    app.wait_for("选集 (1/36)").unwrap();
}
