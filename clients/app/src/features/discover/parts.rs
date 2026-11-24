use hg_contracts::catalog::SeriesSummary;
use hg_contracts::ids::SeriesId;
use nsh_core::prelude::*;

use crate::ui::icons::IconKind;
use crate::ui::poster::{SeriesCardProps, series_card};
use crate::ui::surface::empty_state;

pub fn series_grid(
    items: Vec<SeriesSummary>,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
) -> impl IntoElement {
    if items.is_empty() {
        return empty_state(
            IconKind::Search,
            "暂无匹配剧目",
            "尝试更换关键词或筛选条件",
            None,
        )
        .into_element();
    }

    let mut grid = view().flex_row().flex_wrap().gap_4();
    for (index, item) in items.into_iter().enumerate() {
        let id = item.id;
        let status = if item.is_finished {
            format!("全 {} 集", item.episode_count)
        } else {
            format!("更新至 {} 集", item.episode_count)
        };
        let subtitle = item.tags.join(" · ");
        grid = grid.child(series_card(
            SeriesCardProps {
                title: item.title,
                cover_url: item.cover_url,
                score: item.score,
                subtitle,
                episode_label: status,
                badge_text: item.badge,
                index,
            },
            move || on_open_series(id),
        ));
    }
    grid.into_element()
}
