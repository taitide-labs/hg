use std::sync::Arc;
use std::time::Duration;

use hg_contracts::error::ContractError;
use hg_gateway::mock::MockGateway;
use nsh_engine::ColorScheme;

use super::{start_on_settings, start_on_settings_with};
use crate::pages::tests::{assert_above, assert_left_aligned, assert_same_height};

#[test]
fn settings_download_policy_alignment_and_controls_persist_to_gateway() {
    let (gw, mut app) = start_on_settings();

    app.wait_for("下载策略").unwrap();
    app.wait_for("仅 Wi-Fi 下载").unwrap();
    app.wait_for("最大并发下载任务数").unwrap();

    assert_above(&app, "播放偏好", "下载策略");
    assert_left_aligned(&app, "text:仅 Wi-Fi 下载", "text:最大并发下载任务数", 2.0);
    assert_same_height(&app, "text:仅 Wi-Fi 下载", "text:最大并发下载任务数", 2.0);

    assert!(gw.snapshot_download_policy().wifi_only);
    app.press("仅 Wi-Fi 下载").unwrap();
    app.wait(Duration::from_millis(50)).unwrap();
    assert!(!gw.snapshot_download_policy().wifi_only);

    app.press("4 个").unwrap();
    app.wait(Duration::from_millis(50)).unwrap();
    assert_eq!(gw.snapshot_download_policy().max_concurrent_tasks, 4);
    assert!(app.screen().selected("role:tab[4 个]").unwrap());

    app.press("1 个").unwrap();
    app.wait(Duration::from_millis(50)).unwrap();
    assert_eq!(gw.snapshot_download_policy().max_concurrent_tasks, 1);
    assert!(app.screen().selected("role:tab[1 个]").unwrap());
}

#[test]
fn settings_download_policy_load_error_and_update_error_rollback() {
    let gw = Arc::new(MockGateway::new());
    gw.inject_method_error(
        "download_policy",
        Some(ContractError::NetworkUnavailable {
            detail: "policy read failed".to_owned(),
        }),
    );
    let (gw, mut app) = start_on_settings_with(gw, ColorScheme::Light);
    app.wait_for("下载策略加载失败").unwrap();

    gw.inject_method_error("download_policy", None);
    app.press("重新加载").unwrap();
    app.wait_for("仅 Wi-Fi 下载").unwrap();

    gw.inject_method_error(
        "update_download_policy",
        Some(ContractError::NetworkUnavailable {
            detail: "policy save failed".to_owned(),
        }),
    );
    app.press("4 个").unwrap();
    app.wait(Duration::from_millis(80)).unwrap();
    assert_eq!(gw.snapshot_download_policy().max_concurrent_tasks, 2);
    assert!(app.screen().selected("role:tab[2 个]").unwrap());
}
