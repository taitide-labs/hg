use std::sync::Arc;
use std::time::Duration;

use hg_contracts::offline::DownloadTaskState;
use hg_gateway::mock::MockGateway;
use nsh_engine::ColorScheme;

use super::{start_on_offline, start_on_offline_with};

#[test]
fn bulk_pause_and_resume_controls_update_ui_and_gateway_states() {
    let (gw, mut app) = start_on_offline();

    app.wait_for("全部暂停").unwrap();
    app.press("全部暂停").unwrap();
    app.wait_for("已暂停").unwrap();
    assert!(
        gw.snapshot_download_tasks()
            .iter()
            .all(|t| matches!(t.state, DownloadTaskState::Paused { .. }))
    );

    app.press("全部继续").unwrap();
    app.wait_for("排队中").unwrap();
    assert!(
        gw.snapshot_download_tasks()
            .iter()
            .all(|t| matches!(t.state, DownloadTaskState::Queued))
    );
}

#[test]
fn single_task_pause_resume_retry_and_cancel_actions_work() {
    let gw = Arc::new(MockGateway::new().with_failed_download_task());
    let (gw, mut app) = start_on_offline_with(gw, ColorScheme::Light);

    app.wait_for("重试").unwrap();
    app.press("重试").unwrap();
    app.wait_for("排队中").unwrap();

    app.press("label:取消 剧目 1003 第 1 集下载任务").unwrap();
    app.wait_for_gone("重返 1998：科技大亨", Duration::from_secs(2))
        .unwrap();
    assert_eq!(gw.snapshot_download_tasks().len(), 2);

    app.press("暂停 >> nth=0").unwrap();
    app.wait_for("继续 >> nth=0").unwrap();
    app.press("继续 >> nth=0").unwrap();
    app.wait_for("排队中").unwrap();

    app.press("label:取消 剧目 1001 第 4 集下载任务").unwrap();
    app.wait_for_gone("第 4 集 · 云城风云转折 4", Duration::from_secs(2))
        .unwrap();
    app.press("label:取消 剧目 1002 第 2 集下载任务").unwrap();
    app.wait_for_gone("下载中", Duration::from_secs(2)).unwrap();
    assert!(gw.snapshot_download_tasks().is_empty());
}
