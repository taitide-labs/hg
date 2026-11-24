use std::sync::{Arc, OnceLock};

use hg_contracts::error::ContractError;
use hg_contracts::ids::StreamProfile;
use hg_contracts::offline::DownloadPolicy;
use hg_gateway::contract::AppGateway;
use hg_gateway::mock::MockGateway;
use nsh_core::prelude::*;
use nsh_engine::{ColorScheme, EngineConfig, Environment, Os, Points, Root, Size};
use nsh_testing::Headless;

use super::command::{clamp_concurrency, toggle_wifi_only, with_max_concurrent_tasks};
use super::view::download_policy_section;
use crate::gateway;
use crate::ui::token;

static PERSIST_GW: OnceLock<Arc<MockGateway>> = OnceLock::new();
static RETRY_GW: OnceLock<Arc<MockGateway>> = OnceLock::new();

#[component]
fn PersistHost() -> impl IntoElement {
    if let Some(gw) = PERSIST_GW.get() {
        gateway::install(Arc::clone(gw) as Arc<dyn AppGateway>);
    }
    view()
        .flex_1()
        .bg(token::bg)
        .p_4()
        .child(download_policy_section())
}

#[component]
fn RetryHost() -> impl IntoElement {
    if let Some(gw) = RETRY_GW.get() {
        gateway::install(Arc::clone(gw) as Arc<dyn AppGateway>);
    }
    view()
        .flex_1()
        .bg(token::bg)
        .p_4()
        .child(download_policy_section())
}

#[test]
fn download_policy_helpers_toggle_wifi_and_clamp_concurrency() {
    let base = DownloadPolicy {
        preferred_profile: StreamProfile::Hevc,
        wifi_only: true,
        max_concurrent_tasks: 2,
        storage_directory: Some("/offline".to_owned()),
    };

    let toggled = toggle_wifi_only(&base);
    assert!(!toggled.wifi_only);
    assert_eq!(toggled.max_concurrent_tasks, 2);
    assert_eq!(toggled.storage_directory.as_deref(), Some("/offline"));

    let updated = with_max_concurrent_tasks(&toggled, 4);
    assert_eq!(updated.max_concurrent_tasks, 4);
    assert!(!updated.wifi_only);

    assert_eq!(clamp_concurrency(0), 1);
    assert_eq!(clamp_concurrency(99), 4);
}

#[test]
fn download_policy_changes_persist_through_gateway() {
    let gw = PERSIST_GW.get_or_init(|| Arc::new(MockGateway::new()));
    let mut app = Headless::start(EngineConfig {
        viewport: Size::new(Points(1240.0), Points(800.0)),
        environment: Environment {
            os: Os::MacOs,
            color_scheme: ColorScheme::Light,
            ..Environment::default()
        },
        ..EngineConfig::new(Root::of::<PersistHost>())
    })
    .unwrap();

    app.wait_for("仅 Wi-Fi 下载").unwrap();
    app.press("仅 Wi-Fi 下载").unwrap();
    app.press("3 个").unwrap();

    let saved = futures_blocking_policy(gw.as_ref());
    assert!(!saved.wifi_only);
    assert_eq!(saved.max_concurrent_tasks, 3);
}

#[test]
fn failed_policy_load_shows_retry_and_recovers() {
    let gw = RETRY_GW.get_or_init(|| Arc::new(MockGateway::new()));
    gw.inject_error(Some(ContractError::NetworkUnavailable {
        detail: "offline".to_owned(),
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

    app.wait_for("下载策略加载失败").unwrap();
    gw.inject_error(None);
    app.press("重新加载").unwrap();
    app.wait_for("最大并发下载任务数").unwrap();
}

fn futures_blocking_policy(gw: &MockGateway) -> DownloadPolicy {
    let rt = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(rt);
    let mut fut = gw.download_policy();
    match fut.as_mut().poll(&mut cx) {
        std::task::Poll::Ready(Ok(policy)) => policy,
        std::task::Poll::Ready(Err(_)) | std::task::Poll::Pending => {
            panic!("MockGateway download_policy did not resolve immediately")
        }
    }
}
