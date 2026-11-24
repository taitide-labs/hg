use nsh_base::input;
use nsh_core::prelude::*;

use crate::ui::icons::{IconKind, icon};
use crate::ui::token;

pub fn search_input(
    value: Signal<String>,
    placeholder: &'static str,
    on_submit: impl Fn(String) + 'static,
) -> impl IntoElement {
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
}
