use std::sync::Arc;

use hg_contracts::error::ContractError;
use hg_gateway::mock::MockGateway;
use nsh_engine::ColorScheme;

use super::{start_on_settings, start_on_settings_with};
use crate::pages::tests::assert_above;

#[test]
fn settings_storage_separates_cache_and_offline_library_sizes() {
    let (_gw, mut app) = start_on_settings();

    app.wait_for("存储空间").unwrap();
    app.wait_for("在线播放与图片缓存").unwrap();
    app.wait_for("离线片库").unwrap();
    assert_above(&app, "下载策略", "存储空间");
}

#[test]
fn settings_storage_load_error_recovers_on_retry() {
    let gw = Arc::new(MockGateway::new());
    gw.inject_method_error(
        "storage_overview",
        Some(ContractError::NetworkUnavailable {
            detail: "storage read failed".to_owned(),
        }),
    );
    let (gw, mut app) = start_on_settings_with(gw, ColorScheme::Light);
    app.wait_for("存储占用加载失败").unwrap();

    gw.inject_method_error("storage_overview", None);
    app.press("重新加载").unwrap();
    app.wait_for("在线播放与图片缓存").unwrap();
}
