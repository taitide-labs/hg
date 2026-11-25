mod empty_queue;
mod library_delete;
mod library_query;
mod queue_control;
mod queue_query;

use std::sync::Arc;

use hg_gateway::mock::MockGateway;
use nsh_core::prelude::stored;
use nsh_engine::ColorScheme;
use nsh_testing::Headless;

use crate::pages::offline::page::offline_page;
use crate::pages::tests::{OpenSeriesSpy, mount_desktop_page, mount_desktop_page_manual_clock};

pub(super) fn start_on_offline() -> (Arc<MockGateway>, Headless) {
    start_on_offline_with(Arc::new(MockGateway::new()), ColorScheme::Light)
}

pub(super) fn start_on_offline_with(
    gw: Arc<MockGateway>,
    scheme: ColorScheme,
) -> (Arc<MockGateway>, Headless) {
    let (gw, app, _spy) = start_on_offline_with_spy(gw, scheme);
    (gw, app)
}

pub(super) fn start_on_offline_with_spy(
    gw: Arc<MockGateway>,
    scheme: ColorScheme,
) -> (Arc<MockGateway>, Headless, OpenSeriesSpy) {
    let spy = OpenSeriesSpy::default();
    let spy_for_page = spy.clone();
    let app = mount_desktop_page(Arc::clone(&gw), scheme, move || {
        let sink = stored(spy_for_page.clone());
        offline_page(move |id, ep| sink.with(|s| s.record(id, ep)))
    });
    (gw, app, spy)
}

pub(super) fn start_on_offline_manual_clock(
    gw: Arc<MockGateway>,
    scheme: ColorScheme,
) -> (Arc<MockGateway>, Headless) {
    let app =
        mount_desktop_page_manual_clock(Arc::clone(&gw), scheme, move || offline_page(|_, _| {}));
    (gw, app)
}
