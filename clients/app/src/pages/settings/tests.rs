mod cache_clear;
mod download_policy;
mod playback_pref;
mod storage_query;

use std::sync::Arc;

use hg_gateway::mock::MockGateway;
use nsh_engine::ColorScheme;
use nsh_testing::Headless;

use crate::pages::settings::page::settings_page;
use crate::pages::tests::mount_desktop_page;

pub(super) fn start_on_settings() -> (Arc<MockGateway>, Headless) {
    start_on_settings_with(Arc::new(MockGateway::new()), ColorScheme::Light)
}

pub(super) fn start_on_settings_with(
    gw: Arc<MockGateway>,
    scheme: ColorScheme,
) -> (Arc<MockGateway>, Headless) {
    let app = mount_desktop_page(Arc::clone(&gw), scheme, settings_page);
    (gw, app)
}
