use hg_contracts::ids::EpisodeKey;
use hg_contracts::offline::{ControlAllDownloadsAction, ControlDownloadAction, DownloadTaskState};
use nsh_core::prelude::*;

use super::command::{can_cancel_state, cancel_a11y_label, primary_action_for_state};
use super::handler;
use crate::ui::button::{ButtonVariant, ControlSize, app_button, text_link_button};
use crate::ui::icons::IconKind;

pub fn queue_bulk_controls() -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .child(app_button(
            "全部暂停",
            ButtonVariant::Secondary,
            ControlSize::Sm,
            || handler::control_all(ControlAllDownloadsAction::PauseAll),
        ))
        .child(app_button(
            "全部继续",
            ButtonVariant::Primary,
            ControlSize::Sm,
            || handler::control_all(ControlAllDownloadsAction::ResumeAll),
        ))
}

pub fn task_control_buttons(key: EpisodeKey, state: DownloadTaskState) -> impl IntoElement {
    let primary = primary_action_for_state(state);
    let show_cancel = can_cancel_state(state);
    let cancel_a11y = cancel_a11y_label(key);

    view()
        .flex_row()
        .items_center()
        .gap_1p5()
        .children(primary.map(|spec| {
            let variant = if spec.emphasized {
                ButtonVariant::Primary
            } else {
                ButtonVariant::Secondary
            };
            app_button(spec.label, variant, ControlSize::Sm, move || {
                handler::control_task(key, spec.action);
            })
        }))
        .children(show_cancel.then(|| {
            text_link_button("取消", cancel_a11y, Some(IconKind::Close), move || {
                handler::control_task(key, ControlDownloadAction::Cancel);
            })
        }))
}
