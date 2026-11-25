use std::sync::Arc;
use std::time::Duration;

use hg_contracts::error::ContractError;
use hg_contracts::ids::StreamProfile;
use hg_contracts::viewing::AppAppearance;
use hg_gateway::mock::MockGateway;
use nsh_engine::ColorScheme;

use super::{start_on_settings, start_on_settings_with};
use crate::pages::tests::{assert_above, assert_left_aligned, assert_same_height};

#[test]
fn settings_playback_preference_layout_and_all_controls_persist_to_gateway() {
    let (gw, mut app) = start_on_settings();

    app.wait_for("外观").unwrap();
    app.wait_for("清晰度优先档位").unwrap();
    app.wait_for("默认倍速").unwrap();
    app.wait_for("在线播放缓存上限").unwrap();

    assert_above(&app, "外观", "播放偏好");
    assert_left_aligned(&app, "text:清晰度优先档位", "text:默认倍速", 2.0);
    assert_left_aligned(&app, "text:默认倍速", "text:在线播放缓存上限", 2.0);
    assert_same_height(&app, "text:清晰度优先档位", "text:默认倍速", 2.0);

    assert_eq!(gw.snapshot_preference().appearance, AppAppearance::Dark);
    let dark_fg = app.screen().span_style("外观 >> nth=0").unwrap().color;
    app.press("浅色").unwrap();
    app.wait(Duration::from_millis(50)).unwrap();
    assert_eq!(gw.snapshot_preference().appearance, AppAppearance::Light);
    let light_fg = app.screen().span_style("外观 >> nth=0").unwrap().color;
    assert_ne!(
        dark_fg, light_fg,
        "switching between dark and light appearance must update rendered theme colors"
    );

    app.press("深色").unwrap();
    app.wait(Duration::from_millis(50)).unwrap();
    assert_eq!(gw.snapshot_preference().appearance, AppAppearance::Dark);

    app.press("标准兼容").unwrap();
    app.wait(Duration::from_millis(50)).unwrap();
    assert_eq!(
        gw.snapshot_preference().preferred_profile,
        StreamProfile::Main
    );
    assert!(app.screen().selected("role:tab[标准兼容]").unwrap());

    app.press("1.5×").unwrap();
    app.wait(Duration::from_millis(50)).unwrap();
    assert_eq!(gw.snapshot_preference().default_rate_hundredths, 150);
    assert!(app.screen().selected("role:tab[1.5×]").unwrap());

    app.press("2.0×").unwrap();
    app.wait(Duration::from_millis(50)).unwrap();
    assert_eq!(gw.snapshot_preference().default_rate_hundredths, 200);
}

#[test]
fn settings_playback_preference_load_error_and_update_error_rollback() {
    let gw = Arc::new(MockGateway::new());
    gw.inject_method_error(
        "playback_preference",
        Some(ContractError::NetworkUnavailable {
            detail: "pref read failed".to_owned(),
        }),
    );
    let (gw, mut app) = start_on_settings_with(gw, ColorScheme::Light);
    app.wait_for("播放偏好加载失败").unwrap();

    gw.inject_method_error("playback_preference", None);
    app.press("重新加载").unwrap();
    app.wait_for("标准兼容").unwrap();

    gw.inject_method_error(
        "update_playback_preference",
        Some(ContractError::NetworkUnavailable {
            detail: "pref save failed".to_owned(),
        }),
    );
    app.press("标准兼容").unwrap();
    app.wait(Duration::from_millis(80)).unwrap();
    assert_eq!(
        gw.snapshot_preference().preferred_profile,
        StreamProfile::Hevc
    );
    assert!(app.screen().selected("role:tab[HEVC 高清]").unwrap());
}

#[test]
fn settings_playback_preference_async_retry_survives_dynamic_teardown() {
    let gw = Arc::new(MockGateway::new());
    gw.inject_method_error(
        "playback_preference",
        Some(ContractError::NetworkUnavailable {
            detail: "pref read failed".to_owned(),
        }),
    );
    let (gw, mut app) = start_on_settings_with(gw, ColorScheme::Light);
    app.wait_for("播放偏好加载失败").unwrap();

    gw.inject_method_error("playback_preference", None);
    let gate = gw.queue_method_gate("playback_preference");
    app.press("重新加载").unwrap();
    app.wait(Duration::from_millis(50)).unwrap();
    gate.release();
    app.wait_for("标准兼容").unwrap();
}
