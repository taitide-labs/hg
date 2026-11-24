mod app;
mod features;
mod gateway;
mod pages;
pub mod ui;

use nsh_core::prelude::*;

#[native_module("hg/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}

#[component]
pub fn App() -> impl IntoElement {
    app::init::initialize();
    view()
        .flex_1()
        .bg(ui::token::bg)
        .child(app::shell::app_shell())
}
