use super::start_on_history;

#[test]
fn watch_history_shows_recent_items_and_resumes_episode() {
    let mut app = start_on_history();

    app.wait_for("最近观看").unwrap();
    assert!(!app.screen().shows("我的追剧"));
    app.wait_for("云端之上：千金归来").unwrap();
    app.wait_for("第 3 集").unwrap();
    app.wait_for("长安暗夜行").unwrap();

    app.press("云端之上：千金归来 >> nth=0").unwrap();
    app.wait_for("选集 (3/68)").unwrap();
}
