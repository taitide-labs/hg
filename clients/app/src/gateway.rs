use std::sync::Arc;

use hg_gateway::contract::AppGateway;
use hg_gateway::mock::MockGateway;
use nsh_core::prelude::*;

#[derive(Clone)]
struct GatewaySlot(Arc<dyn AppGateway>);

impl Default for GatewaySlot {
    fn default() -> Self {
        Self(Arc::new(MockGateway::new()))
    }
}

pub fn install(gateway: Arc<dyn AppGateway>) {
    global::<GatewaySlot>().set(GatewaySlot(gateway));
}

#[must_use]
pub fn current() -> Arc<dyn AppGateway> {
    global::<GatewaySlot>().with_untracked(|slot| Arc::clone(&slot.0))
}
