mod cache_clear;
mod download_policy;
mod playback_pref;
mod storage_query;

use std::sync::Arc;

use hg_gateway::mock::MockGateway;
use nsh_engine::ColorScheme;
use nsh_testing::Headless;

use crate::pages::tests::start_desktop_on_home;

pub(super) fn start_on_settings() -> (Arc<MockGateway>, Headless) {
    start_on_settings_with(Arc::new(MockGateway::new()), ColorScheme::Light)
}

pub(super) fn start_on_settings_with(
    gw: Arc<MockGateway>,
    scheme: ColorScheme,
) -> (Arc<MockGateway>, Headless) {
    let mut app = start_desktop_on_home(Arc::clone(&gw), scheme);
    app.press("设置").unwrap();
    (gw, app)
}
