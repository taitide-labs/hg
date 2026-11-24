use nsh_core::prelude::*;

use super::handler;
use crate::ui::button::{ButtonVariant, ControlSize, app_button};
use crate::ui::token;

pub fn clear_cache_button() -> impl IntoElement {
    let label = computed(|| {
        if handler::is_clearing() {
            SharedString::from("正在清理...")
        } else {
            SharedString::from("清理缓存")
        }
    });
    let hint = computed(handler::status_hint);
    let error = computed(handler::error_message);

    view()
        .w_full()
        .pt_1()
        .gap_2()
        .child(view().flex_row().items_center().child(app_button(
            move || label.get(),
            ButtonVariant::Outline,
            ControlSize::Sm,
            handler::clear_cache,
        )))
        .child(when(
            move || hint.with(Option::is_some),
            move || {
                text(move || hint.get().unwrap_or_default())
                    .text_xs()
                    .text_color(token::success)
            },
        ))
        .child(when(
            move || error.with(Option::is_some),
            move || {
                text(move || error.get().unwrap_or_default())
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(token::destructive)
            },
        ))
}
