use nsh_base::button;
use nsh_core::prelude::*;

use crate::ui::icons::{IconKind, icon};
use crate::ui::token;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Ghost,
    Outline,
    CinemaGlass,
    CinemaPrimary,
    Danger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlSize {
    Sm,
    Md,
    Lg,
}

fn height_for(size: ControlSize) -> Points {
    match size {
        ControlSize::Sm => px(28.0),
        ControlSize::Md => px(34.0),
        ControlSize::Lg => px(42.0),
    }
}

fn px_for(size: ControlSize) -> Points {
    match size {
        ControlSize::Sm => px(10.0),
        ControlSize::Md => px(14.0),
        ControlSize::Lg => px(18.0),
    }
}

fn text_color_for(variant: ButtonVariant) -> AdaptiveColor {
    match variant {
        ButtonVariant::Primary | ButtonVariant::CinemaPrimary | ButtonVariant::Danger => {
            token::primary_fg()
        }
        ButtonVariant::Secondary | ButtonVariant::Ghost | ButtonVariant::Outline => token::fg(),
        ButtonVariant::CinemaGlass => token::cinema_fg(),
    }
}

fn bg_for(variant: ButtonVariant) -> Option<AdaptiveColor> {
    match variant {
        ButtonVariant::Primary | ButtonVariant::CinemaPrimary => Some(token::primary()),
        ButtonVariant::Secondary => Some(token::secondary()),
        ButtonVariant::CinemaGlass => Some(token::cinema_surface_elevated()),
        ButtonVariant::Danger => Some(token::destructive()),
        ButtonVariant::Ghost | ButtonVariant::Outline => None,
    }
}

fn hover_bg_for(variant: ButtonVariant) -> AdaptiveColor {
    match variant {
        ButtonVariant::Primary | ButtonVariant::CinemaPrimary => token::primary().opacity(0.9),
        ButtonVariant::Secondary | ButtonVariant::Ghost | ButtonVariant::Outline => token::accent(),
        ButtonVariant::CinemaGlass => token::cinema_border(),
        ButtonVariant::Danger => token::destructive().opacity(0.9),
    }
}

pub fn app_button(
    label: impl IntoReactive<SharedString>,
    variant: ButtonVariant,
    size: ControlSize,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let base = button()
        .on_press(on_press)
        .h(height_for(size))
        .px(px_for(size))
        .flex_row()
        .items_center()
        .justify_center()
        .gap_2()
        .rounded(token::radius_control)
        .transition_colors()
        .duration(token::INSTANT_MS)
        .hover(move |s| s.bg(move || hover_bg_for(variant)))
        .active(move |s| {
            let s = s.opacity(0.88);
            if reduce_motion() {
                s
            } else {
                s.scale(token::PRESS_SCALE)
            }
        })
        .focus_visible(|s| s.outline(px(2.0)).outline_color(token::ring))
        .disabled_style(|s| s.opacity(0.45));

    let base = match bg_for(variant) {
        Some(_) => base.bg(move || bg_for(variant).unwrap_or_else(token::secondary)),
        None => base,
    };
    let base = if matches!(variant, ButtonVariant::Outline) {
        base.border_1().border_color(token::border)
    } else if matches!(variant, ButtonVariant::CinemaGlass) {
        base.border_1().border_color(token::cinema_border)
    } else {
        base
    };

    base.child(
        text(label)
            .text_sm()
            .font_weight(FontWeight::MEDIUM)
            .text_color(move || text_color_for(variant)),
    )
}

pub fn app_icon_button(
    kind: IconKind,
    label: &'static str,
    variant: ButtonVariant,
    size: ControlSize,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let dim = height_for(size);
    let icon_pt = match size {
        ControlSize::Sm => 14.0,
        ControlSize::Md => 16.0,
        ControlSize::Lg => 18.0,
    };

    let base = button()
        .on_press(on_press)
        .accessibility_label(label)
        .size(dim)
        .items_center()
        .justify_center()
        .rounded(token::radius_control)
        .transition_colors()
        .duration(token::INSTANT_MS)
        .hover(move |s| s.bg(move || hover_bg_for(variant)))
        .active(move |s| {
            let s = s.opacity(0.85);
            if reduce_motion() {
                s
            } else {
                s.scale(token::ICON_PRESS_SCALE)
            }
        })
        .focus_visible(|s| s.outline(px(2.0)).outline_color(token::ring));

    let base = match bg_for(variant) {
        Some(_) => base.bg(move || bg_for(variant).unwrap_or_else(token::secondary)),
        None => base,
    };
    let base = if matches!(variant, ButtonVariant::CinemaGlass) {
        base.border_1().border_color(token::cinema_border)
    } else if matches!(variant, ButtonVariant::Outline) {
        base.border_1().border_color(token::border)
    } else {
        base
    };

    base.child(icon(kind, icon_pt, move || text_color_for(variant)))
}

pub fn sidebar_nav_button(
    kind: IconKind,
    label: &'static str,
    selected: impl Fn() -> bool + Copy + 'static,
    quiet: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    button()
        .on_press(on_press)
        .selected(selected)
        .accessibility_label(label)
        .h(px(36.0))
        .px_3()
        .flex_row()
        .items_center()
        .gap_3()
        .rounded(token::radius_control)
        .transition_colors()
        .duration(token::BASE_MS)
        .hover(|s| s.bg(token::sidebar_accent))
        .style_when(selected, |s| s.bg(token::sidebar_accent))
        .active(|s| {
            if reduce_motion() {
                s.opacity(0.85)
            } else {
                s.scale(token::PRESS_SCALE)
            }
        })
        .focus_visible(|s| s.outline(px(2.0)).outline_color(token::ring))
        .child(icon(kind, 16.0, move || {
            if selected() {
                token::primary()
            } else if quiet {
                token::muted_fg()
            } else {
                token::sidebar_fg()
            }
        }))
        .child(
            text(label)
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || {
                    if selected() {
                        token::sidebar_fg()
                    } else {
                        token::muted_fg()
                    }
                }),
        )
}

pub fn mobile_tab_button(
    kind: IconKind,
    label: &'static str,
    selected: impl Fn() -> bool + Copy + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    button()
        .on_press(on_press)
        .selected(selected)
        .accessibility_label(label)
        .flex_1()
        .h(px(52.0))
        .items_center()
        .justify_center()
        .gap_1()
        .rounded(token::radius_control)
        .active(|s| s.opacity(0.75))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(token::ring))
        .child(icon(kind, 18.0, move || {
            if selected() {
                token::primary()
            } else {
                token::muted_fg()
            }
        }))
        .child(text(label).text_xs().text_color(move || {
            if selected() {
                token::primary()
            } else {
                token::muted_fg()
            }
        }))
}
