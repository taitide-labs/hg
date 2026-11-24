use hg_contracts::ids::SeriesId;
use nsh_core::prelude::*;

use super::handler;
use crate::ui::button::text_link_button;
use crate::ui::icons::IconKind;

pub fn unbookmark_button(series_id: SeriesId, mut title: String) -> impl IntoElement {
    title.insert_str(0, "取消追剧 ");
    view().pr_3().child(text_link_button(
        "取消追剧",
        title,
        Some(IconKind::Close),
        move || handler::unbookmark(series_id),
    ))
}
