use std::time::Duration;

use nsh_engine::ColorScheme;

use super::start_and_open_first_series;

#[test]
fn batch_downloading_episodes_from_player_enqueues_and_closes_cleanly() {
    let (gw, mut app) = start_and_open_first_series(ColorScheme::Light);
    let tasks_before = gw.snapshot_download_tasks().len();

    app.press("离线下载").unwrap();
    app.wait_for("选择离线下载分集").unwrap();
    app.press("关闭").unwrap();
    app.wait_for_gone("选择离线下载分集", Duration::from_secs(2))
        .unwrap();
    assert_eq!(gw.snapshot_download_tasks().len(), tasks_before);

    app.press("离线下载").unwrap();
    app.wait_for("选择离线下载分集").unwrap();
    app.press("全选当前段").unwrap();
    app.press("加入离线队列").unwrap();
    app.wait_for("text:/已加入离线下载/").unwrap();
    app.wait_for_gone("选择离线下载分集", Duration::from_secs(2))
        .unwrap();
    assert!(gw.snapshot_download_tasks().len() > tasks_before);
}
