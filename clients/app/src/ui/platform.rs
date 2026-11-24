use nsh_core::desktop::{TitleBarOptions, TitleBarStyle, WindowAttributes, WindowMode};
use nsh_core::element::desktop::{current_window, title_bar};
use nsh_core::prelude::*;

use crate::ui::token;

#[must_use]
pub fn is_desktop() -> bool {
    matches!(platform_name(), "macos" | "windows" | "linux")
}

pub fn set_window_fullscreen(fullscreen: bool) {
    if !is_desktop() {
        return;
    }
    let mode = if fullscreen {
        WindowMode::FullScreen
    } else {
        WindowMode::Normal
    };
    current_window().set_mode(mode);
}

pub fn configure_hidden_title_bar() {
    if !is_desktop() {
        return;
    }
    let mut bar = TitleBarOptions::default();
    bar.style = TitleBarStyle::Hidden;
    bar.height = Some(px(44.0));
    let mut attributes = WindowAttributes::default();
    attributes.title_bar = Some(bar);
    current_window().set(attributes);
}

pub fn desktop_title_bar(
    leading: impl IntoElement,
    trailing: impl IntoElement,
) -> impl IntoElement {
    view()
        .h(px(44.0))
        .flex_shrink_0()
        .w_full()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .window_drag(true)
        .pl(move || title_bar().left + px(14.0))
        .pr(move || title_bar().right + px(14.0))
        .bg(token::bg)
        .border_b_1()
        .border_color(token::border)
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .window_drag(false)
                .child(leading),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .window_drag(false)
                .child(trailing),
        )
}
