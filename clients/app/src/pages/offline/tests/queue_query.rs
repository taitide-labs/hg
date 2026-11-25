use super::start_on_offline;

#[test]
fn download_queue_displays_active_tasks_and_progress() {
    let mut app = start_on_offline();

    app.wait_for("下载中").unwrap();
    app.wait_for("已下载").unwrap();
    app.wait_for("第 4 集 · 云城风云转折 4").unwrap();
    app.wait_for("长安暗夜行").unwrap();
}
