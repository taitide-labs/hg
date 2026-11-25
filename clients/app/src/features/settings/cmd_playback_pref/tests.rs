use std::sync::{Arc, OnceLock};

use hg_contracts::error::ContractError;
use hg_contracts::ids::StreamProfile;
use hg_contracts::viewing::PlaybackPreference;
use hg_gateway::contract::AppGateway;
use hg_gateway::mock::MockGateway;
use nsh_core::prelude::*;
use nsh_engine::{ColorScheme, EngineConfig, Environment, Os, Points, Root, Size};
use nsh_testing::Headless;

use super::command::{
    AppearanceOption, normalize_rate_hundredths, with_default_rate, with_preferred_profile,
};
use super::view::playback_pref_section;
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
        .child(playback_pref_section())
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
        .child(playback_pref_section())
}

#[test]
fn appearance_option_maps_to_nsh_appearance() {
    assert_eq!(
        Appearance::from(AppearanceOption::System),
        Appearance::System
    );
    assert_eq!(Appearance::from(AppearanceOption::Light), Appearance::Light);
    assert_eq!(Appearance::from(AppearanceOption::Dark), Appearance::Dark);
}

#[test]
fn playback_preference_updates_profile_and_rate_preserving_cache_limit() {
    let base = PlaybackPreference::default();

    let updated_profile = with_preferred_profile(base, StreamProfile::Main);
    assert_eq!(updated_profile.preferred_profile, StreamProfile::Main);
    assert_eq!(updated_profile.playback_cache_limit_mb, 512);
    assert_eq!(updated_profile.default_rate_hundredths, 100);

    let updated_rate = with_default_rate(updated_profile, 150);
    assert_eq!(updated_rate.default_rate_hundredths, 150);
    assert_eq!(updated_rate.preferred_profile, StreamProfile::Main);
    assert_eq!(updated_rate.playback_cache_limit_mb, 512);

    assert_eq!(normalize_rate_hundredths(999), 100);
    assert_eq!(normalize_rate_hundredths(125), 125);
}

#[test]
fn profile_and_default_rate_persist_through_gateway() {
    let gw = PERSIST_GW.get_or_init(|| Arc::new(MockGateway::new()));
    let mut app = Headless::start(EngineConfig {
        viewport: Size::new(Points(1240.0), Points(800.0)),
        environment: Environment {
            os: Os::MacOs,
            color_scheme: ColorScheme::Dark,
            ..Environment::default()
        },
        ..EngineConfig::new(Root::of::<PersistHost>())
    })
    .unwrap();

    app.wait_for("标准兼容").unwrap();
    app.press("标准兼容").unwrap();
    app.press("1.5×").unwrap();
    app.press("深色").unwrap();

    let saved = futures_blocking_pref(gw.as_ref());
    assert_eq!(saved.preferred_profile, StreamProfile::Main);
    assert_eq!(saved.default_rate_hundredths, 150);
}

#[test]
fn failed_initial_load_shows_retry_and_recovers() {
    let gw = RETRY_GW.get_or_init(|| Arc::new(MockGateway::new()));
    gw.inject_error(Some(ContractError::NetworkUnavailable {
        detail: "network unavailable".to_owned(),
    }));

    let mut app = Headless::start(EngineConfig {
        viewport: Size::new(Points(390.0), Points(844.0)),
        environment: Environment {
            os: Os::Android,
            color_scheme: ColorScheme::Light,
            ..Environment::default()
        },
        ..EngineConfig::new(Root::of::<RetryHost>())
    })
    .unwrap();

    app.wait_for("播放偏好加载失败").unwrap();
    gw.inject_error(None);
    app.press("重新加载").unwrap();
    app.wait_for("清晰度优先档位").unwrap();
}

fn futures_blocking_pref(gw: &MockGateway) -> PlaybackPreference {
    let rt = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(rt);
    let mut fut = gw.playback_preference();
    match fut.as_mut().poll(&mut cx) {
        std::task::Poll::Ready(Ok(pref)) => pref,
        std::task::Poll::Ready(Err(_)) | std::task::Poll::Pending => {
            panic!("MockGateway playback_preference did not resolve immediately")
        }
    }
}
