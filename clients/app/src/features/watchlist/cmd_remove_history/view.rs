use hg_contracts::ids::SeriesId;
use nsh_core::prelude::*;

use super::handler;
use crate::ui::button::{ButtonVariant, ControlSize, app_button, text_link_button};
use crate::ui::controls::confirm_destructive;
use crate::ui::icons::IconKind;

pub fn clear_history_button() -> impl IntoElement {
    app_button(
        "清空观看历史",
        ButtonVariant::Ghost,
        ControlSize::Sm,
        || {
            confirm_destructive(
                "清空观看历史",
                "确定要清空全部观看历史记录吗？",
                "清空",
                handler::clear_history,
            );
        },
    )
}

pub fn remove_history_button(series_id: SeriesId, title: &str) -> impl IntoElement + use<> {
    let a11y_label = format!("移除 {title} 观看记录");
    let description = format!("确定要移除《{title}》的观看记录吗？");
    view().pr_3().child(text_link_button(
        "移除",
        a11y_label,
        Some(IconKind::Close),
        move || {
            confirm_destructive(
                "移除观看记录",
                description.clone(),
                "移除",
                move || {
                    handler::remove_history(series_id);
                },
            );
        },
    ))
}
