use std::sync::Arc;

use hg_gateway::mock::MockGateway;

use crate::gateway;
use crate::ui::platform;

pub fn initialize() {
    platform::configure_hidden_title_bar();
    gateway::install(Arc::new(MockGateway::new()));
}
