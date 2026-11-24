use std::sync::Arc;

use hg_gateway::contract::AppGateway;
use hg_gateway::local::LocalGateway;
use hg_gateway::mock::MockGateway;

use crate::gateway;
use crate::ui::platform;

pub fn initialize_default() {
    platform::configure_hidden_title_bar();
    let gw: Arc<dyn AppGateway> = match LocalGateway::open_default() {
        Ok(local) => {
            local.spawn_background_download_pump();
            Arc::new(local)
        }
        Err(_) => Arc::new(MockGateway::new()),
    };
    gateway::install(gw);
}

pub fn initialize_with_gateway(gw: Arc<dyn AppGateway>) {
    platform::configure_hidden_title_bar();
    gateway::install(gw);
}
