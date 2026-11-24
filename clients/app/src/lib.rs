mod app;
mod features;
mod gateway;
mod pages;
pub mod ui;

use std::sync::Arc;

use hg_gateway::contract::AppGateway;
use nsh_core::prelude::*;

#[native_module("hg/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}

#[cfg(target_os = "macos")]
#[native_components(macos)]
fn native_components_macos(components: &nsh_platform_macos::Components) {
    hg_native_player::register_macos(components);
}

#[cfg(target_os = "windows")]
#[native_components(windows)]
fn native_components_windows(components: &nsh_platform_windows::Components) {
    hg_native_player::register_windows(components);
}

#[component]
pub fn App() -> impl IntoElement {
    app::init::initialize_default();
    view()
        .flex_1()
        .bg(ui::token::bg)
        .child(app::shell::app_shell())
}

#[derive(Clone)]
struct InjectedAppProps(Arc<dyn AppGateway>);

impl PartialEq for InjectedAppProps {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl nsh_core::element::Props for InjectedAppProps {
    fn spec(_: &mut nsh_core::element::TsTypes) -> Vec<nsh_core::element::PropSpec> {
        Vec::new()
    }

    fn initial() -> Self {
        Self(Arc::new(hg_gateway::mock::MockGateway::new()))
    }

    fn set(
        &mut self,
        name: &str,
        _: Option<nsh_core::element::AnyValue>,
    ) -> Result<bool, nsh_core::element::PropError> {
        Err(nsh_core::element::PropError(format!(
            "unknown prop `{name}`"
        )))
    }

    fn set_slot(&mut self, _: &str, _: Option<nsh_core::element::Slot>) -> bool {
        false
    }

    fn set_children(&mut self, _: nsh_core::element::Children) -> bool {
        false
    }
}

struct InjectedApp;

impl nsh_core::element::Component for InjectedApp {
    type Props = InjectedAppProps;
    type Event = nsh_core::element::NoEvent;

    fn view(props: Self::Props, _cx: Scope<Self::Event>) -> impl IntoElement {
        app::init::initialize_with_gateway(props.0);
        view()
            .flex_1()
            .bg(ui::token::bg)
            .child(app::shell::app_shell())
    }
}

#[must_use]
pub fn root_with_gateway(gateway: Arc<dyn AppGateway>) -> nsh_core::element::Root {
    nsh_core::element::Root::new::<InjectedApp>(move || InjectedAppProps(Arc::clone(&gateway)))
}
