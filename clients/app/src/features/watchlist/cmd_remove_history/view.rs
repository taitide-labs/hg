use hg_contracts::ids::SeriesId;
use nsh_core::prelude::*;

use super::handler;
use crate::ui::button::{ButtonVariant, ControlSize, confirm_app_trigger, confirm_link_trigger};
use crate::ui::controls::confirm_popover;
use crate::ui::icons::IconKind;

pub fn clear_history_button() -> impl IntoElement {
    confirm_popover(
        confirm_app_trigger("清空观看历史", ButtonVariant::Ghost, ControlSize::Sm),
        "确认清空全部观看历史？",
        "确认清空",
        handler::clear_history,
    )
}

pub fn remove_history_button(series_id: SeriesId, title: &str) -> impl IntoElement + use<> {
    let a11y_label = format!("移除 {title} 观看记录");
    view().pr_3().child(confirm_popover(
        confirm_link_trigger("移除", a11y_label, Some(IconKind::Close)),
        "确认移除此条记录？",
        "确认移除",
        move || handler::remove_history(series_id),
    ))
}
