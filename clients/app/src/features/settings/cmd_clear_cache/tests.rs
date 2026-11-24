use std::sync::{Arc, OnceLock};

use hg_contracts::error::ContractError;
use hg_contracts::viewing::StorageOverview;
use hg_gateway::contract::AppGateway;
use hg_gateway::mock::MockGateway;
use nsh_core::prelude::*;
use nsh_engine::{ColorScheme, EngineConfig, Environment, Os, Points, Root, Size};
use nsh_testing::Headless;

use super::command::{can_trigger_clear, zeroed_cache_overview};
use super::view::clear_cache_button;
use crate::gateway;
use crate::ui::token;

static CLEAR_GW: OnceLock<Arc<MockGateway>> = OnceLock::new();
static FAIL_GW: OnceLock<Arc<MockGateway>> = OnceLock::new();

#[component]
fn ClearHost() -> impl IntoElement {
    if let Some(gw) = CLEAR_GW.get() {
        gateway::install(Arc::clone(gw) as Arc<dyn AppGateway>);
    }
    view()
        .flex_1()
        .bg(token::bg)
        .p_4()
        .child(clear_cache_button())
}

#[component]
fn FailHost() -> impl IntoElement {
    if let Some(gw) = FAIL_GW.get() {
        gateway::install(Arc::clone(gw) as Arc<dyn AppGateway>);
    }
    view()
        .flex_1()
        .bg(token::bg)
        .p_4()
        .child(clear_cache_button())
}

#[test]
fn zeroed_cache_overview_preserves_offline_library_bytes() {
    let before = StorageOverview {
        offline_library_bytes: 58_400_000,
        playback_cache_bytes: 134_217_728,
        catalog_cache_bytes: 25_165_824,
    };

    assert!(can_trigger_clear(false, Some(before)));
    assert!(!can_trigger_clear(true, Some(before)));
    let after = zeroed_cache_overview(before);
    assert_eq!(after.offline_library_bytes, 58_400_000);
    assert_eq!(after.playback_cache_bytes, 0);
    assert_eq!(after.catalog_cache_bytes, 0);
    assert!(!can_trigger_clear(false, Some(after)));
    assert!(can_trigger_clear(false, None));
}

#[test]
fn clearing_cache_zeroes_both_caches_in_gateway_without_touching_offline_library() {
    let gw = CLEAR_GW.get_or_init(|| Arc::new(MockGateway::new()));
    let mut app = Headless::start(EngineConfig {
        viewport: Size::new(Points(1240.0), Points(800.0)),
        environment: Environment {
            os: Os::MacOs,
            color_scheme: ColorScheme::Light,
            ..Environment::default()
        },
        ..EngineConfig::new(Root::of::<ClearHost>())
    })
    .unwrap();

    app.wait_for("清理缓存").unwrap();
    app.press("清理缓存").unwrap();
    app.wait_for("已清空在线播放与图片缓存，离线片库不受影响")
        .unwrap();

    let overview = futures_blocking_storage(gw.as_ref());
    assert_eq!(overview.playback_cache_bytes, 0);
    assert_eq!(overview.catalog_cache_bytes, 0);
    assert_eq!(overview.offline_library_bytes, 58_400_000);
}

#[test]
fn failed_cache_clear_shows_error_message() {
    let gw = FAIL_GW.get_or_init(|| Arc::new(MockGateway::new()));
    let mut app = Headless::start(EngineConfig {
        viewport: Size::new(Points(390.0), Points(844.0)),
        environment: Environment {
            os: Os::Android,
            color_scheme: ColorScheme::Dark,
            ..Environment::default()
        },
        ..EngineConfig::new(Root::of::<FailHost>())
    })
    .unwrap();

    app.wait_for("清理缓存").unwrap();
    gw.inject_error(Some(ContractError::Internal {
        detail: "cache lock".to_owned(),
    }));
    app.press("清理缓存").unwrap();
    app.wait_for("text:/cache lock/").unwrap();
}

fn futures_blocking_storage(gw: &MockGateway) -> StorageOverview {
    let rt = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(rt);
    let mut fut = gw.storage_overview();
    match fut.as_mut().poll(&mut cx) {
        std::task::Poll::Ready(Ok(overview)) => overview,
        std::task::Poll::Ready(Err(_)) | std::task::Poll::Pending => {
            panic!("MockGateway storage_overview did not resolve immediately")
        }
    }
}
