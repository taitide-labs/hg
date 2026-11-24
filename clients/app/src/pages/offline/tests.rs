mod library_delete;
mod library_query;
mod queue_control;
mod queue_query;

use std::sync::Arc;

use hg_gateway::mock::MockGateway;
use nsh_engine::{ColorScheme, EngineConfig, Environment, Os, Points, Size};
use nsh_testing::Headless;

use crate::root_with_gateway;

pub(super) fn start_on_offline() -> Headless {
    let environment = Environment {
        os: Os::MacOs,
        color_scheme: ColorScheme::Light,
        ..Environment::default()
    };
    let mut app = Headless::start(EngineConfig {
        viewport: Size::new(Points(1240.0), Points(800.0)),
        environment,
        ..EngineConfig::new(root_with_gateway(Arc::new(MockGateway::new())))
    })
    .unwrap();
    app.wait_for("精选推荐").unwrap();
    app.press("离线").unwrap();
    app
}
