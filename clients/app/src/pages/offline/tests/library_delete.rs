use std::time::Duration;

use super::start_on_offline;

#[test]
fn deleting_offline_series_removes_assets_from_ui_and_gateway() {
    let (gw, mut app) = start_on_offline();

    app.wait_for("删除 云端之上：千金归来 离线剧目").unwrap();
    app.press("删除 云端之上：千金归来 离线剧目").unwrap();
    app.wait_for_gone("删除 云端之上：千金归来 离线剧目", Duration::from_secs(2))
        .unwrap();
    app.wait_for("暂无已下载剧目，下载完成后将自动归档至此")
        .unwrap();
    assert!(gw.snapshot_offline_library().is_empty());
}
