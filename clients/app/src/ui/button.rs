use nsh_base::button;
use nsh_core::prelude::*;

use crate::ui::icons::{IconKind, icon};
use crate::ui::token;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Watched,
    Ghost,
    Outline,
    BrandSoft,
    CinemaGlass,
    CinemaPrimary,
    CinemaBrand,
    OverlayGhost,
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

fn icon_pt_for(size: ControlSize) -> f32 {
    match size {
        ControlSize::Sm => 14.0,
        ControlSize::Md => 16.0,
        ControlSize::Lg => 18.0,
    }
}

fn text_color_for(variant: ButtonVariant) -> AdaptiveColor {
    match variant {
        ButtonVariant::Primary | ButtonVariant::CinemaPrimary | ButtonVariant::Danger => {
            token::primary_fg()
        }
        ButtonVariant::Watched => token::watched_fg(),
        ButtonVariant::BrandSoft => token::brand_soft_fg(),
        ButtonVariant::CinemaBrand => token::cinema_brand_fg(),
        ButtonVariant::Secondary | ButtonVariant::Ghost | ButtonVariant::Outline => token::fg(),
        ButtonVariant::CinemaGlass | ButtonVariant::OverlayGhost => token::cinema_fg(),
    }
}

fn bg_for(variant: ButtonVariant) -> Option<AdaptiveColor> {
    match variant {
        ButtonVariant::Primary | ButtonVariant::CinemaPrimary => Some(token::primary()),
        ButtonVariant::Watched => Some(token::watched()),
        ButtonVariant::BrandSoft => Some(token::brand_soft()),
        ButtonVariant::CinemaBrand => Some(token::cinema_brand_bg()),
        ButtonVariant::Secondary => Some(token::secondary()),
        ButtonVariant::CinemaGlass => Some(token::cinema_surface_elevated()),
        ButtonVariant::Danger => Some(token::destructive()),
        ButtonVariant::Ghost | ButtonVariant::Outline | ButtonVariant::OverlayGhost => None,
    }
}

fn hover_bg_for(variant: ButtonVariant) -> AdaptiveColor {
    match variant {
        ButtonVariant::Primary | ButtonVariant::CinemaPrimary => token::primary().opacity(0.9),
        ButtonVariant::Watched => token::watched().opacity(0.85),
        ButtonVariant::BrandSoft => token::brand_soft().opacity(0.85),
        ButtonVariant::CinemaBrand => token::cinema_brand_bg().opacity(0.85),
        ButtonVariant::Secondary | ButtonVariant::Ghost | ButtonVariant::Outline => token::accent(),
        ButtonVariant::CinemaGlass => token::cinema_border(),
        ButtonVariant::OverlayGhost => token::cinema_surface_elevated(),
        ButtonVariant::Danger => token::destructive().opacity(0.9),
    }
}

fn wrap_variant_button(
    variant: ButtonVariant,
    on_press: impl Fn() + 'static,
    content: impl IntoElement,
    size: ControlSize,
    square_label: Option<&'static str>,
) -> impl IntoElement {
    wrap_reactive_variant_button(move || variant, on_press, content, size, square_label)
}

fn wrap_reactive_variant_button(
    variant: impl Fn() -> ButtonVariant + Copy + 'static,
    on_press: impl Fn() + 'static,
    content: impl IntoElement,
    size: ControlSize,
    square_label: Option<&'static str>,
) -> impl IntoElement {
    let base = button()
        .on_press(on_press)
        .rounded(token::radius_control)
        .transition_colors()
        .duration(token::INSTANT_MS)
        .hover(move |s| s.bg(move || hover_bg_for(variant())))
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

    let base = match square_label {
        Some(a11y) => base
            .accessibility_label(a11y)
            .size(height_for(size))
            .items_center()
            .justify_center(),
        None => base
            .h(height_for(size))
            .px(px_for(size))
            .flex_row()
            .items_center()
            .justify_center()
            .gap_1p5(),
    };

    let base = base.bg(move || bg_for(variant()).unwrap_or_else(|| token::bg().opacity(0.0)));

    base.border_1()
        .border_color(move || match variant() {
            ButtonVariant::Outline => token::border(),
            ButtonVariant::CinemaGlass | ButtonVariant::CinemaBrand => token::cinema_border(),
            ButtonVariant::Primary
            | ButtonVariant::Secondary
            | ButtonVariant::Watched
            | ButtonVariant::Ghost
            | ButtonVariant::BrandSoft
            | ButtonVariant::CinemaPrimary
            | ButtonVariant::OverlayGhost
            | ButtonVariant::Danger => token::bg().opacity(0.0),
        })
        .child(content)
}

pub fn app_button(
    label: impl IntoReactive<SharedString>,
    variant: ButtonVariant,
    size: ControlSize,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    app_reactive_button(label, move || variant, size, on_press)
}

pub fn app_reactive_button(
    label: impl IntoReactive<SharedString>,
    variant: impl Fn() -> ButtonVariant + Copy + 'static,
    size: ControlSize,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let content = text(label)
        .text_sm()
        .font_weight(FontWeight::MEDIUM)
        .text_color(move || text_color_for(variant()));
    wrap_reactive_variant_button(variant, on_press, content, size, None)
}

pub fn episode_tile_button(
    label: impl IntoReactive<SharedString>,
    a11y_label: impl Into<SharedString>,
    variant: ButtonVariant,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let base = button()
        .on_press(on_press)
        .accessibility_label(a11y_label.into())
        .w(px(64.0))
        .h(px(30.0))
        .items_center()
        .justify_center()
        .rounded(token::radius_control)
        .transition_colors()
        .duration(token::INSTANT_MS)
        .hover(move |s| s.bg(move || hover_bg_for(variant)))
        .active(|s| {
            let s = s.opacity(0.88);
            if reduce_motion() {
                s
            } else {
                s.scale(token::PRESS_SCALE)
            }
        })
        .focus_visible(|s| s.outline(px(2.0)).outline_color(token::ring));
    let base = match bg_for(variant) {
        Some(_) => base.bg(move || bg_for(variant).unwrap_or_else(token::secondary)),
        None => base,
    };
    base.child(
        text(label)
            .text_xs()
            .tabular_nums()
            .font_weight(FontWeight::MEDIUM)
            .text_color(move || text_color_for(variant)),
    )
}

pub fn app_icon_text_button(
    kind: IconKind,
    label: impl IntoReactive<SharedString>,
    variant: ButtonVariant,
    size: ControlSize,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let content = view()
        .flex_row()
        .items_center()
        .gap_1p5()
        .child(icon(kind, icon_pt_for(size), move || {
            text_color_for(variant)
        }))
        .child(
            text(label)
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || text_color_for(variant)),
        );
    wrap_variant_button(variant, on_press, content, size, None)
}

pub fn app_icon_button(
    kind: IconKind,
    label: &'static str,
    variant: ButtonVariant,
    size: ControlSize,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let content = icon(kind, icon_pt_for(size), move || text_color_for(variant));
    wrap_variant_button(variant, on_press, content, size, Some(label))
}

pub fn overlay_icon_button(
    kind: IconKind,
    label: &'static str,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let content = icon(kind, 16.0, || text_color_for(ButtonVariant::OverlayGhost));
    wrap_variant_button(
        ButtonVariant::OverlayGhost,
        on_press,
        content,
        ControlSize::Sm,
        Some(label),
    )
}

pub fn overlay_text_button(
    label: impl IntoReactive<SharedString>,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let content = text(label)
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(|| text_color_for(ButtonVariant::OverlayGhost));
    wrap_variant_button(
        ButtonVariant::OverlayGhost,
        on_press,
        content,
        ControlSize::Sm,
        None,
    )
}

pub fn text_link_button(
    label: impl IntoReactive<SharedString>,
    a11y_label: impl Into<SharedString>,
    trailing_icon: Option<IconKind>,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    button()
        .on_press(on_press)
        .accessibility_label(a11y_label.into())
        .h(px(26.0))
        .px_2()
        .flex_row()
        .items_center()
        .gap_1()
        .rounded(token::radius_control)
        .transition_colors()
        .duration(token::INSTANT_MS)
        .hover(|s| s.bg(token::accent))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(token::ring))
        .child(
            text(label)
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(token::muted_fg),
        )
        .children(trailing_icon.map(|kind| icon(kind, 13.0, token::muted_fg)))
}

pub fn sidebar_nav_button(
    kind: IconKind,
    label: &'static str,
    selected: impl Fn() -> bool + Copy + 'static,
    quiet: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    sidebar_nav_button_collapsible(kind, label, selected, quiet, || false, on_press)
}

pub fn sidebar_nav_button_collapsible(
    kind: IconKind,
    label: &'static str,
    selected: impl Fn() -> bool + Copy + 'static,
    quiet: bool,
    collapsed: impl Fn() -> bool + Copy + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    button()
        .on_press(on_press)
        .selected(selected)
        .accessibility_label(label)
        .h(px(36.0))
        .px_3()
        .style_when(collapsed, |s| s.px_0().justify_center())
        .flex_row()
        .items_center()
        .gap_3()
        .style_when(collapsed, |s| s.gap_0())
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
        .child(when(
            move || !collapsed(),
            move || {
                text(label)
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(move || {
                        if selected() {
                            token::sidebar_fg()
                        } else {
                            token::muted_fg()
                        }
                    })
            },
        ))
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
