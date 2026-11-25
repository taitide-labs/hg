use std::sync::Arc;

use hg_gateway::contract::AppGateway;
use hg_gateway::local::LocalGateway;
use hg_gateway::mock::MockGateway;
use nsh_core::prelude::*;

use crate::app::nav;
use crate::features::settings;
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
    gateway::install(Arc::clone(&gw));
    hydrate_startup_preferences(gw);
}

pub fn initialize_with_gateway(gw: Arc<dyn AppGateway>) {
    platform::configure_hidden_title_bar();
    gateway::install(Arc::clone(&gw));
    hydrate_startup_preferences(gw);
}

fn hydrate_startup_preferences(gw: Arc<dyn AppGateway>) {
    settings::cmd_playback_pref::handler::apply_default_appearance();
    spawn(async move {
        if let Ok(pref) = gw.playback_preference().await {
            if let Some(placement) = pref.window_placement {
                platform::restore_window_placement(placement);
            }
            settings::cmd_playback_pref::handler::apply_loaded_preference(pref);
            nav::hydrate_sidebar(pref.sidebar_collapsed, f32::from(pref.sidebar_width));
        }
    });
}
