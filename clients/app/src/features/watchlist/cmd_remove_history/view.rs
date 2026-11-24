use hg_contracts::ids::SeriesId;
use nsh_core::prelude::*;

use super::handler;
use crate::ui::button::{ButtonVariant, ControlSize, app_button, text_link_button};
use crate::ui::icons::IconKind;

pub fn clear_history_button() -> impl IntoElement {
    app_button(
        "清空观看历史",
        ButtonVariant::Ghost,
        ControlSize::Sm,
        handler::clear_history,
    )
}

pub fn remove_history_button(series_id: SeriesId, mut title: String) -> impl IntoElement {
    title.insert_str(0, "移除 ");
    title.push_str(" 观看记录");
    view().pr_3().child(text_link_button(
        "移除",
        title,
        Some(IconKind::Close),
        move || handler::remove_history(series_id),
    ))
}
