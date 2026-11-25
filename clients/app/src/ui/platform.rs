use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use hg_contracts::viewing::{WindowPlacementMode, WindowPlacementPref};
use nsh_core::desktop::{
    TitleBarOptions, TitleBarStyle, WindowAttributes, WindowMode, WindowPlacement,
};
use nsh_core::element::desktop::{current_window, title_bar};
use nsh_core::prelude::*;

use crate::ui::token;

const WINDOW_PLACEMENT_DEBOUNCE: Duration = Duration::from_millis(300);

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
    let pending_timer: Rc<RefCell<Option<Spawned<()>>>> = Rc::new(RefCell::new(None));
    let pending_timer_cleanup = Rc::clone(&pending_timer);
    let last_mode: Rc<Cell<Option<WindowPlacementMode>>> = Rc::new(Cell::new(None));
    let latest_pref: Rc<Cell<Option<WindowPlacementPref>>> = Rc::new(Cell::new(None));
    let on_change = Rc::new(on_change);

    cleanup(move || {
        if let Some(handle) = pending_timer_cleanup.borrow_mut().take() {
            handle.cancel();
        }
    });

    effect(move || {
        let Some(next) = read_window_placement(&suppress_capture) else {
            return;
        };
        if let Some(handle) = pending_timer.borrow_mut().take() {
            handle.cancel();
        }
        latest_pref.set(Some(next));
        let mode_changed = last_mode.get() != Some(next.mode);
        if mode_changed {
            last_mode.set(Some(next.mode));
            on_change(next);
            return;
        }
        let emit = Rc::clone(&on_change);
        let latest = Rc::clone(&latest_pref);
        let mode_track = Rc::clone(&last_mode);
        let handle = set_timeout(WINDOW_PLACEMENT_DEBOUNCE, move || {
            if let Some(pref) = latest.take() {
                mode_track.set(Some(pref.mode));
                emit(pref);
            }
        });
        *pending_timer.borrow_mut() = Some(handle);
    });
}

fn read_window_placement(suppress_capture: &impl Fn() -> bool) -> Option<WindowPlacementPref> {
    let state = current_window().state()?;
    if state.minimized || untracked(suppress_capture) {
        return None;
    }
    let placement = state.placement;
    if let Some(target_mode) = global::<PendingWindowRestore>().with_untracked(|p| p.0) {
        if placement.mode != target_mode {
            return None;
        }
        global::<PendingWindowRestore>().set(PendingWindowRestore(None));
    }
    let width = placement.bounds.size.width.0.round();
    let height = placement.bounds.size.height.0.round();
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    Some(WindowPlacementPref {
        mode: from_nsh_window_mode(placement.mode),
        x: placement.bounds.origin.x.0.round() as i32,
        y: placement.bounds.origin.y.0.round() as i32,
        width: width.clamp(1.0, f32::from(u16::MAX)) as u16,
        height: height.clamp(1.0, f32::from(u16::MAX)) as u16,
    })
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
