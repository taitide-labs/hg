mod bookmarks_query;
mod bookmarks_remove;

use std::sync::Arc;

use hg_gateway::mock::MockGateway;
use nsh_engine::ColorScheme;
use nsh_testing::Headless;

use crate::pages::tests::start_desktop_on_home;

pub(super) fn start_on_watchlist() -> (Arc<MockGateway>, Headless) {
    start_on_watchlist_with(Arc::new(MockGateway::new()), ColorScheme::Light)
}

pub(super) fn start_on_watchlist_with(
    gw: Arc<MockGateway>,
    scheme: ColorScheme,
) -> (Arc<MockGateway>, Headless) {
    let mut app = start_desktop_on_home(Arc::clone(&gw), scheme);
    app.press("追剧").unwrap();
    (gw, app)
}
