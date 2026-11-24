use std::sync::{Arc, OnceLock};

use hg_contracts::error::ContractError;
use hg_contracts::viewing::StorageOverview;
use hg_gateway::contract::AppGateway;
use hg_gateway::mock::MockGateway;
use nsh_core::prelude::*;
use nsh_engine::{ColorScheme, EngineConfig, Environment, Os, Points, Root, Size};
use nsh_testing::Headless;

use super::query::{cache_breakdown, cache_total_bytes, format_mb};
use super::view::storage_section;
use crate::gateway;
use crate::ui::token;

static RETRY_GW: OnceLock<Arc<MockGateway>> = OnceLock::new();

#[component]
fn RetryHost() -> impl IntoElement {
    if let Some(gw) = RETRY_GW.get() {
        gateway::install(Arc::clone(gw) as Arc<dyn AppGateway>);
    }
    view()
        .flex_1()
        .bg(token::bg)
        .p_4()
        .child(storage_section(view))
}

#[test]
fn storage_formatting_separates_cache_total_from_offline_library() {
    let overview = StorageOverview {
        offline_library_bytes: 58_400_000,
        playback_cache_bytes: 134_217_728,
        catalog_cache_bytes: 25_165_824,
    };

    assert_eq!(cache_total_bytes(overview), 159_383_552);
    assert_eq!(format_mb(0), "0 MB");
    assert_eq!(format_mb(overview.offline_library_bytes), "55.7 MB");
    assert_eq!(format_mb(cache_total_bytes(overview)), "152.0 MB");
    assert_eq!(
        cache_breakdown(overview),
        "播放缓存 128.0 MB · 目录与图片 24.0 MB"
    );
}

#[test]
fn failed_storage_load_shows_retry_and_recovers() {
    let gw = RETRY_GW.get_or_init(|| Arc::new(MockGateway::new()));
    gw.inject_error(Some(ContractError::NetworkUnavailable {
        detail: "storage unavailable".to_owned(),
    }));

    let mut app = Headless::start(EngineConfig {
        viewport: Size::new(Points(390.0), Points(844.0)),
        environment: Environment {
            os: Os::Android,
            color_scheme: ColorScheme::Dark,
            ..Environment::default()
        },
        ..EngineConfig::new(Root::of::<RetryHost>())
    })
    .unwrap();

    app.wait_for("存储占用加载失败").unwrap();
    gw.inject_error(None);
    app.press("重新加载").unwrap();
    app.wait_for("在线播放与图片缓存").unwrap();
    app.wait_for("152.0 MB").unwrap();
    app.wait_for("55.7 MB").unwrap();
}
