use std::rc::Rc;

use nsh_base::{button, input};
use nsh_core::prelude::*;

use crate::ui::icons::{IconKind, icon};
use crate::ui::token;

pub fn search_input(
    value: Signal<String>,
    placeholder: &'static str,
    on_submit: impl Fn(String) + 'static,
    on_clear: impl Fn() + 'static,
) -> impl IntoElement {
    let on_clear = Rc::new(on_clear);
    let has_text = computed(move || !value.get().is_empty());

    view()
        .h(px(34.0))
        .px_3()
        .flex_row()
        .items_center()
        .gap_2()
        .rounded(token::radius_control)
        .bg(token::card)
        .border_1()
        .border_color(token::input_border)
        .child(icon(IconKind::Search, 14.0, token::muted_fg))
        .child(
            input()
                .value(move || value.get())
                .on_change(move |next: &str| value.set(next.to_owned()))
                .on_submit(move |submitted: &str| on_submit(submitted.to_owned()))
                .placeholder(placeholder)
                .placeholder_color(|| color(token::muted_fg()))
                .caret_color(|| color(token::primary()))
                .text_sm()
                .text_color(token::fg)
                .flex_1(),
        )
        .child(when(
            move || has_text.get(),
            move || {
                let clear_cb = Rc::clone(&on_clear);
                button()
                    .on_press(move || {
                        value.set(String::new());
                        clear_cb();
                    })
                    .accessibility_label("清空搜索")
                    .size(px(22.0))
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .transition_colors()
                    .duration(token::INSTANT_MS)
                    .hover(|s| s.bg(token::accent))
                    .focus_visible(|s| s.outline(px(2.0)).outline_color(token::ring))
                    .child(icon(IconKind::Close, 12.0, token::muted_fg))
            },
        ))
}
