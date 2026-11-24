use super::start_on_offline;

#[test]
fn offline_library_displays_downloaded_series_and_opens_player() {
    let mut app = start_on_offline();

    app.wait_for("已下载").unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
    app.wait_for("text:/3 集/").unwrap();

    app.press("云端之上：千金归来 >> nth=0").unwrap();
    app.wait_for("选集 (1/68)").unwrap();
}
