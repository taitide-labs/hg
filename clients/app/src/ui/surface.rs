use nsh_base::button;
use nsh_core::prelude::*;

use crate::ui::button::{ButtonVariant, ControlSize, app_button, text_link_button};
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

pub fn shelf_header(
    title: impl Into<SharedString>,
    action_label: &'static str,
    on_action: impl Fn() + 'static,
) -> impl IntoElement {
    let title_str: SharedString = title.into();
    let a11y_label = format!("查看{title_str}{action_label}");
    view()
        .w_full()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .child(
            text(title_str)
                .role(AccessibilityRole::Header)
                .text_base()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(token::fg),
        )
        .child(text_link_button(
            action_label,
            a11y_label,
            Some(IconKind::ChevronRight),
            on_action,
        ))
}

pub fn toggle_switch_row(
    label: &'static str,
    checked: impl Fn() -> bool + Copy + 'static,
    dark_surface: bool,
    on_toggle: impl Fn() + 'static,
) -> impl IntoElement {
    button()
        .on_press(on_toggle)
        .accessibility_label(label)
        .w_full()
        .h(px(34.0))
        .px_2p5()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_4()
        .rounded(token::radius_control)
        .transition_colors()
        .duration(token::INSTANT_MS)
        .hover(move |s| {
            s.bg(move || {
                if dark_surface {
                    token::cinema_surface_elevated()
                } else {
                    token::accent()
                }
            })
        })
        .child(
            text(label)
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || {
                    if dark_surface {
                        token::cinema_fg()
                    } else {
                        token::fg()
                    }
                }),
        )
        .child(
            view()
                .w(px(32.0))
                .h(px(18.0))
                .p_0p5()
                .rounded_full()
                .flex_row()
                .items_center()
                .bg(move || {
                    if checked() {
                        token::primary()
                    } else if dark_surface {
                        token::cinema_border()
                    } else {
                        token::border()
                    }
                })
                .child(
                    view()
                        .flex_1()
                        .style_when(move || !checked(), |s| s.w(px(0.0)).flex_grow(0.0)),
                )
                .child(view().size(px(14.0)).rounded_full().bg(token::primary_fg)),
        )
}

pub fn avatar_circle(initial: String, tint_key: String) -> impl IntoElement {
    view()
        .size(px(28.0))
        .flex_shrink_0()
        .rounded_full()
        .bg(move || token::poster_surface(&tint_key))
        .items_center()
        .justify_center()
        .child(
            text(initial)
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(token::cinema_fg),
        )
}

pub fn separator() -> impl IntoElement {
    view().h(px(1.0)).w_full().bg(token::border)
}

pub fn progress_track(ratio: impl Fn() -> f32 + Copy + 'static) -> impl IntoElement {
    view()
        .w_full()
        .h(px(3.0))
        .rounded_full()
        .bg(token::cinema_border)
        .overflow_hidden()
        .flex_row()
        .child(
            view()
                .h_full()
                .flex_grow(move || ratio().clamp(0.0, 1.0))
                .bg(token::primary),
        )
        .child(
            view()
                .h_full()
                .flex_grow(move || (1.0 - ratio().clamp(0.0, 1.0)).max(0.0)),
        )
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

pub fn poster_grid_skeleton(card_count: usize, label: &'static str) -> impl IntoElement {
    let mut grid = view()
        .label(label)
        .w_full()
        .p_6()
        .flex_row()
        .flex_wrap()
        .gap_4();
    for _ in 0..card_count {
        grid = grid.child(
            view()
                .w(px(172.0))
                .flex_shrink_0()
                .gap_1p5()
                .p_0p5()
                .child(
                    view()
                        .w_full()
                        .aspect_ratio(160.0 / 224.0)
                        .rounded(token::radius)
                        .bg(token::surface_subtle),
                )
                .child(
                    view()
                        .w(px(110.0))
                        .h(px(14.0))
                        .rounded(token::radius_badge)
                        .bg(token::surface_subtle),
                )
                .child(
                    view()
                        .w(px(72.0))
                        .h(px(10.0))
                        .rounded(token::radius_badge)
                        .bg(token::muted),
                ),
        );
    }
    grid
}

pub fn list_row_skeleton(row_count: usize, label: &'static str) -> impl IntoElement {
    let mut stack = view().label(label).w_full().gap_2p5();
    for _ in 0..row_count {
        stack = stack.child(
            view()
                .w_full()
                .rounded(token::radius_card)
                .bg(token::card)
                .border_1()
                .border_color(token::border)
                .p_3()
                .flex_row()
                .items_center()
                .gap_3p5()
                .child(
                    view()
                        .w(px(52.0))
                        .h(px(70.0))
                        .flex_shrink_0()
                        .rounded(token::radius)
                        .bg(token::surface_subtle),
                )
                .child(
                    view()
                        .flex_1()
                        .min_w_0()
                        .gap_2()
                        .child(
                            view()
                                .w(px(160.0))
                                .h(px(14.0))
                                .rounded(token::radius_badge)
                                .bg(token::surface_subtle),
                        )
                        .child(
                            view()
                                .w(px(110.0))
                                .h(px(10.0))
                                .rounded(token::radius_badge)
                                .bg(token::muted),
                        ),
                ),
        );
    }
    stack
}

pub fn player_skeleton(label: &'static str) -> impl IntoElement {
    let mut episodes = view().w_full().flex_row().flex_wrap().gap_2();
    for _ in 0..8 {
        episodes = episodes.child(
            view()
                .w(px(58.0))
                .h(px(32.0))
                .rounded(token::radius_control)
                .bg(token::surface_subtle),
        );
    }
    view()
        .label(label)
        .w_full()
        .flex_1()
        .min_h_0()
        .p_4()
        .gap_4()
        .flex_row()
        .child(
            view()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .rounded(token::radius_card)
                .bg(token::cinema_surface)
                .p_4()
                .justify_end()
                .child(
                    view()
                        .w_full()
                        .h(px(36.0))
                        .rounded(token::radius)
                        .bg(token::cinema_surface_elevated),
                ),
        )
        .child(
            view()
                .w(px(320.0))
                .flex_shrink_0()
                .gap_4()
                .child(
                    view()
                        .w(px(180.0))
                        .h(px(20.0))
                        .rounded(token::radius_badge)
                        .bg(token::surface_subtle),
                )
                .child(
                    view()
                        .w(px(240.0))
                        .h(px(14.0))
                        .rounded(token::radius_badge)
                        .bg(token::muted),
                )
                .child(episodes)
                .child(comment_skeleton_list(3)),
        )
}

pub fn comment_skeleton_list(row_count: usize) -> impl IntoElement {
    let mut list = view().w_full().gap_2p5();
    for _ in 0..row_count {
        list = list.child(
            view()
                .w_full()
                .flex_row()
                .items_start()
                .gap_2p5()
                .child(
                    view()
                        .size(px(28.0))
                        .rounded_full()
                        .bg(token::surface_subtle),
                )
                .child(
                    view()
                        .flex_1()
                        .gap_1p5()
                        .child(
                            view()
                                .w(px(120.0))
                                .h(px(12.0))
                                .rounded(token::radius_badge)
                                .bg(token::surface_subtle),
                        )
                        .child(
                            view()
                                .w(px(200.0))
                                .h(px(10.0))
                                .rounded(token::radius_badge)
                                .bg(token::muted),
                        ),
                ),
        );
    }
    list
}
