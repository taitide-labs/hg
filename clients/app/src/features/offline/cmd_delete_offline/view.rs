use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId};
use nsh_core::prelude::*;

use super::super::store;
use super::command::{
    delete_episode_a11y_label, delete_series_a11y_label, manage_episodes_a11y_label,
};
use super::handler;
use crate::ui::button::text_link_button;
use crate::ui::icons::IconKind;

pub fn delete_series_button(series_id: SeriesId, title: String) -> impl IntoElement {
    let expanded = signal(false);
    let series_a11y = delete_series_a11y_label(&title);
    let manage_a11y = manage_episodes_a11y_label(&title);

    view()
        .pr_3()
        .items_end()
        .gap_1p5()
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_1p5()
                .child(text_link_button(
                    "分集",
                    manage_a11y,
                    Some(IconKind::ChevronDown),
                    move || expanded.update(|v| *v = !*v),
                ))
                .child(text_link_button(
                    "删除",
                    series_a11y,
                    Some(IconKind::Close),
                    move || handler::delete_series(series_id),
                )),
        )
        .child(dynamic(move || {
            if !expanded.get() {
                return view().into_element();
            }
            let episodes: Vec<EpisodeNumber> = store::state().with(|s| {
                s.library
                    .iter()
                    .find(|item| item.series_id == series_id)
                    .map(|item| item.episodes.iter().map(|ep| ep.ep_num).collect())
                    .unwrap_or_default()
            });
            if episodes.is_empty() {
                return view().into_element();
            }

            let mut row = view().max_w(px(260.0)).flex_row().flex_wrap().gap_1();
            for ep_num in episodes {
                let key = EpisodeKey::new(series_id, ep_num);
                let ep_label = format!("第 {} 集", ep_num.get());
                let ep_a11y = delete_episode_a11y_label(&title, ep_num);
                row = row.child(text_link_button(
                    ep_label,
                    ep_a11y,
                    Some(IconKind::Close),
                    move || handler::delete_episode(key),
                ));
            }
            row.into_element()
        }))
}
