use nsh_base::{tab, tabs};
use nsh_core::prelude::*;

use crate::ui::token;

pub fn segmented_tabs<T>(
    items: Vec<(T, &'static str)>,
    active: Signal<T>,
    on_select: impl Fn(T) + Copy + 'static,
) -> impl IntoElement
where
    T: Copy + PartialEq + 'static,
{
    let mut row = tabs()
        .flex_row()
        .self_start()
        .gap_1()
        .p_1()
        .rounded(token::radius_control)
        .bg(token::muted)
        .indicator_style(|s| {
            s.top_1()
                .bottom_1()
                .rounded(token::radius_badge)
                .bg(token::card)
        });

    for (value, label) in items {
        let selected = move || active.get() == value;
        row = row.child(
            tab()
                .selected(selected)
                .on_press(move |(): &()| on_select(value))
                .px_3()
                .py_1p5()
                .rounded(token::radius_badge)
                .focus_visible(|s| s.outline(px(2.0)).outline_color(token::ring))
                .child(
                    text(label)
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(move || {
                            if selected() {
                                token::fg()
                            } else {
                                token::muted_fg()
                            }
                        }),
                ),
        );
    }

    row
}

pub fn cinema_tabs<T>(
    items: Vec<(T, &'static str)>,
    active: Signal<T>,
    on_select: impl Fn(T) + Copy + 'static,
) -> impl IntoElement
where
    T: Copy + PartialEq + 'static,
{
    let mut row = tabs()
        .flex_row()
        .w_full()
        .gap_1()
        .p_1()
        .rounded(token::radius_control)
        .bg(token::cinema_bg)
        .indicator_style(|s| {
            s.top_1()
                .bottom_1()
                .rounded(token::radius_badge)
                .bg(token::cinema_surface_elevated)
        });

    for (value, label) in items {
        let selected = move || active.get() == value;
        row = row.child(
            tab()
                .selected(selected)
                .on_press(move |(): &()| on_select(value))
                .flex_1()
                .py_1p5()
                .items_center()
                .justify_center()
                .rounded(token::radius_badge)
                .focus_visible(|s| s.outline(px(2.0)).outline_color(token::ring))
                .child(
                    text(label)
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(move || {
                            if selected() {
                                token::cinema_fg()
                            } else {
                                token::cinema_muted()
                            }
                        }),
                ),
        );
    }

    row
}
