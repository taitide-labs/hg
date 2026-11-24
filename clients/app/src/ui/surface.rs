use nsh_core::prelude::*;

use crate::ui::button::{ButtonVariant, ControlSize, app_button};
use crate::ui::icons::{IconKind, icon};
use crate::ui::token;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadgeTone {
    Neutral,
    Brand,
    Success,
    Warning,
    Cinema,
    CinemaSuccess,
    CinemaBrand,
}

pub fn badge(label: impl IntoReactive<SharedString>, tone: BadgeTone) -> impl IntoElement {
    view()
        .self_start()
        .px_2()
        .py_0p5()
        .rounded(token::radius_badge)
        .bg(move || match tone {
            BadgeTone::Neutral => token::muted(),
            BadgeTone::Brand => token::brand_soft(),
            BadgeTone::Success => token::success_soft(),
            BadgeTone::Warning => token::warning_soft(),
            BadgeTone::Cinema => token::cinema_surface_elevated(),
            BadgeTone::CinemaSuccess => token::cinema_success_bg(),
            BadgeTone::CinemaBrand => token::cinema_brand_bg(),
        })
        .child(
            text(label)
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || match tone {
                    BadgeTone::Neutral => token::muted_fg(),
                    BadgeTone::Brand => token::brand_soft_fg(),
                    BadgeTone::Success => token::success(),
                    BadgeTone::Warning => token::warning(),
                    BadgeTone::Cinema => token::cinema_fg(),
                    BadgeTone::CinemaSuccess => token::cinema_success_fg(),
                    BadgeTone::CinemaBrand => token::cinema_brand_fg(),
                }),
        )
}

pub fn section_header(title: &'static str, subtitle: Option<&'static str>) -> impl IntoElement {
    view()
        .flex_row()
        .items_end()
        .justify_between()
        .gap_3()
        .child(
            view()
                .gap_0p5()
                .child(
                    text(title)
                        .role(AccessibilityRole::Header)
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(token::fg),
                )
                .children(subtitle.map(|sub| text(sub).text_xs().text_color(token::muted_fg))),
        )
}

pub fn separator() -> impl IntoElement {
    view().h(px(1.0)).w_full().bg(token::border)
}

pub fn cinema_separator() -> impl IntoElement {
    view().h(px(1.0)).w_full().bg(token::cinema_border)
}

pub type EmptyAction = (&'static str, Box<dyn Fn() + 'static>);

pub fn empty_state(
    kind: IconKind,
    title: &'static str,
    description: &'static str,
    action: Option<EmptyAction>,
) -> impl IntoElement {
    view()
        .flex_1()
        .items_center()
        .justify_center()
        .p_8()
        .gap_3()
        .child(
            view()
                .size(px(48.0))
                .rounded_full()
                .bg(token::muted)
                .items_center()
                .justify_center()
                .child(icon(kind, 22.0, token::muted_fg)),
        )
        .child(
            text(title)
                .text_base()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(token::fg),
        )
        .child(text(description).text_sm().text_color(token::muted_fg))
        .children(action.map(|(label, on_press)| {
            app_button(label, ButtonVariant::Primary, ControlSize::Md, move || {
                on_press();
            })
        }))
}
