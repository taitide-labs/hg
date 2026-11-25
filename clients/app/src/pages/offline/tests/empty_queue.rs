use std::sync::Arc;

use hg_contracts::error::ContractError;
use hg_gateway::mock::MockGateway;
use nsh_engine::ColorScheme;

use super::start_on_offline_with;

#[test]
fn empty_download_queue_hides_queue_section_without_tabs() {
    let gw = Arc::new(MockGateway::new().with_empty_download_queue());
    let (_gw, mut app) = start_on_offline_with(gw, ColorScheme::Light);

    app.wait_for("已下载").unwrap();
    app.wait_for("托管离线片库").unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
    assert!(!app.screen().shows("下载中"));
    assert!(!app.screen().shows("下载任务队列"));
}

#[test]
fn empty_offline_library_and_queue_shows_unified_empty_state() {
    let gw = Arc::new(
        MockGateway::new()
            .with_empty_download_queue()
            .with_empty_offline_library(),
    );
    let (_gw, mut app) = start_on_offline_with(gw, ColorScheme::Dark);

    app.wait_for("暂无离线剧目").unwrap();
    assert!(!app.screen().shows("下载中"));
}

#[test]
fn offline_queue_and_library_errors_recover_on_retry() {
    let gw = Arc::new(MockGateway::new());
    gw.inject_method_error(
        "download_tasks",
        Some(ContractError::NetworkUnavailable {
            detail: "queue error".to_owned(),
        }),
    );
    gw.inject_method_error(
        "offline_library",
        Some(ContractError::NetworkUnavailable {
            detail: "library error".to_owned(),
        }),
    );
    let (_gw, mut app) = start_on_offline_with(Arc::clone(&gw), ColorScheme::Light);
    app.wait_for("下载队列加载失败").unwrap();
    app.wait_for("离线片库加载失败").unwrap();

    gw.inject_method_error("download_tasks", None);
    gw.inject_method_error("offline_library", None);
    app.press("重新加载 >> nth=0").unwrap();
    app.wait_for("第 4 集 · 云城风云转折 4").unwrap();
    app.press("重新加载").unwrap();
    app.wait_for("托管离线片库").unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
}
