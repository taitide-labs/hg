mod history_query;
mod history_remove;

use std::sync::Arc;

use hg_gateway::mock::MockGateway;
use nsh_core::prelude::stored;
use nsh_engine::ColorScheme;
use nsh_testing::Headless;

use crate::pages::history::page::history_page;
use crate::pages::tests::{OpenSeriesSpy, mount_desktop_page};

pub(super) fn start_on_history() -> (Arc<MockGateway>, Headless, OpenSeriesSpy) {
    start_on_history_with(Arc::new(MockGateway::new()), ColorScheme::Light)
}

pub(super) fn start_on_history_with(
    gw: Arc<MockGateway>,
    scheme: ColorScheme,
) -> (Arc<MockGateway>, Headless, OpenSeriesSpy) {
    let spy = OpenSeriesSpy::default();
    let spy_for_page = spy.clone();
    let app = mount_desktop_page(Arc::clone(&gw), scheme, move || {
        let sink = stored(spy_for_page.clone());
        history_page(move |id, ep| sink.with(|s| s.record(id, ep)))
    });
    (gw, app, spy)
}
