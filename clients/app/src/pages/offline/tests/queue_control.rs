use super::start_on_offline;

#[test]
fn controlling_download_queue_updates_task_states() {
    let mut app = start_on_offline();

    app.wait_for("下载中").unwrap();
    app.press("下载中").unwrap();
    app.wait_for("全部暂停").unwrap();
    app.press("全部暂停").unwrap();
    app.wait_for("已暂停").unwrap();

    app.press("全部继续").unwrap();
    app.wait_for("排队中").unwrap();
}
