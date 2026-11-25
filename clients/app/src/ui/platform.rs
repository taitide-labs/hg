use hg_contracts::viewing::{WindowPlacementMode, WindowPlacementPref};
use nsh_core::desktop::{
    TitleBarOptions, TitleBarStyle, WindowAttributes, WindowMode, WindowPlacement,
};
use nsh_core::element::desktop::{current_window, title_bar};
use nsh_core::prelude::*;

use crate::ui::token;

#[derive(Debug, Clone, Copy, Default)]
struct PendingWindowRestore(Option<WindowMode>);

const fn to_nsh_window_mode(mode: WindowPlacementMode) -> WindowMode {
    match mode {
        WindowPlacementMode::Normal => WindowMode::Normal,
        WindowPlacementMode::Maximized => WindowMode::Maximized,
        WindowPlacementMode::FullScreen => WindowMode::FullScreen,
    }
}

const fn from_nsh_window_mode(mode: WindowMode) -> WindowPlacementMode {
    match mode {
        WindowMode::Normal => WindowPlacementMode::Normal,
        WindowMode::Maximized => WindowPlacementMode::Maximized,
        WindowMode::FullScreen => WindowPlacementMode::FullScreen,
    }
}

pub fn restore_window_placement(placement: WindowPlacementPref) {
    if !is_desktop() {
        return;
    }
    let mode = to_nsh_window_mode(placement.mode);
    global::<PendingWindowRestore>().set(PendingWindowRestore(Some(mode)));
    let bounds = Bounds::new(
        point(px(placement.x as f32), px(placement.y as f32)),
        Size::new(
            px(f32::from(placement.width)),
            px(f32::from(placement.height)),
        ),
    );
    current_window().set_placement(WindowPlacement::new(mode, bounds));
}

pub fn observe_window_placement(
    suppress_capture: impl Fn() -> bool + 'static,
    on_change: impl Fn(WindowPlacementPref) + 'static,
) {
    if !is_desktop() {
        return;
    }
    effect(move || {
        let Some(state) = current_window().state() else {
            return;
        };
        if state.minimized || untracked(&suppress_capture) {
            return;
        }
        let placement = state.placement;
        if let Some(target_mode) = global::<PendingWindowRestore>().with_untracked(|p| p.0) {
            if placement.mode != target_mode {
                return;
            }
            global::<PendingWindowRestore>().set(PendingWindowRestore(None));
        }
        let width = placement.bounds.size.width.0.round();
        let height = placement.bounds.size.height.0.round();
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        on_change(WindowPlacementPref {
            mode: from_nsh_window_mode(placement.mode),
            x: placement.bounds.origin.x.0.round() as i32,
            y: placement.bounds.origin.y.0.round() as i32,
            width: width.clamp(1.0, f32::from(u16::MAX)) as u16,
            height: height.clamp(1.0, f32::from(u16::MAX)) as u16,
        });
    });
}

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
