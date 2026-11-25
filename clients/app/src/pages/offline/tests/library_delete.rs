use std::sync::Arc;
use std::time::Duration;

use hg_gateway::mock::MockGateway;
use nsh_engine::ColorScheme;

use super::{start_on_offline, start_on_offline_with_spy};

#[test]
fn deleting_offline_series_requires_confirmation_and_removes_assets() {
    let (gw, mut app) = start_on_offline();

    app.wait_for("删除 云端之上：千金归来 离线剧目").unwrap();

    app.press("删除 云端之上：千金归来 离线剧目").unwrap();
    app.wait_for("确认删除").unwrap();
    app.press("role:button[取消]").unwrap();
    app.wait_for_gone("确认删除", Duration::from_secs(2))
        .unwrap();
    assert!(
        app.screen()
            .id("label:删除 云端之上：千金归来 离线剧目")
            .is_ok()
    );
    assert_eq!(gw.snapshot_offline_library().len(), 1);

    app.press("删除 云端之上：千金归来 离线剧目").unwrap();
    app.wait_for("确认删除").unwrap();
    app.press("确认删除").unwrap();
    app.wait_for_gone("删除 云端之上：千金归来 离线剧目", Duration::from_secs(2))
        .unwrap();
    app.wait_for("暂无已下载剧目，下载完成后将自动归档至此")
        .unwrap();
    assert!(gw.snapshot_offline_library().is_empty());
}

#[test]
fn offline_episodes_modal_displays_total_episodes_color_codes_downloaded_and_supports_actions() {
    let (gw, mut app, opened) =
        start_on_offline_with_spy(Arc::new(MockGateway::new()), ColorScheme::Light);

    app.wait_for("label:管理 云端之上：千金归来 离线分集")
        .unwrap();
    app.press("label:管理 云端之上：千金归来 离线分集").unwrap();

    app.wait_for("text:/共 68 集/").unwrap();
    app.wait_for("text:/已下载 3 集/").unwrap();
    app.wait_for("51-68").unwrap();

    let downloaded_style = app
        .screen()
        .view_style("label:播放 云端之上：千金归来 第 1 集")
        .unwrap();
    let undownloaded_style = app
        .screen()
        .view_style("label:云端之上：千金归来 第 5 集 未下载")
        .unwrap();
    assert_ne!(
        downloaded_style, undownloaded_style,
        "downloaded episodes must use a visually distinct color from undownloaded episodes"
    );

    app.press("删除分集").unwrap();
    app.press("label:删除 云端之上：千金归来 第 2 集离线分集")
        .unwrap();
    app.wait_for("确认删除").unwrap();
    app.press("role:button[取消]").unwrap();
    app.wait_for_gone("确认删除", Duration::from_secs(2))
        .unwrap();
    assert_eq!(
        gw.snapshot_offline_library()
            .first()
            .map(|s| s.episodes.len()),
        Some(3)
    );

    app.press("label:删除 云端之上：千金归来 第 2 集离线分集")
        .unwrap();
    app.wait_for("确认删除").unwrap();
    app.press("确认删除").unwrap();
    app.wait_for("text:/已下载 2 集/").unwrap();
    assert_eq!(
        gw.snapshot_offline_library()
            .first()
            .map(|s| s.episodes.len()),
        Some(2)
    );

    app.press("完成").unwrap();
    app.press("label:播放 云端之上：千金归来 第 1 集").unwrap();
    let emitted = opened
        .last()
        .expect("pressing downloaded episode must emit open intent");
    assert_eq!(emitted.0.get(), 1001);
    assert_eq!(emitted.1.map(|ep| ep.get()), Some(1));
}
