use hg_contracts::catalog::SeriesSummary;
use hg_contracts::ids::SeriesId;
use nsh_core::prelude::*;

use crate::ui::button::{ButtonVariant, ControlSize, app_button};
use crate::ui::icons::IconKind;
use crate::ui::poster::{SeriesCardProps, series_card};
use crate::ui::surface::empty_state;
use crate::ui::token;

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

    let mut grid = view()
        .w_full()
        .grid()
        .grid_template_columns("repeat(auto-fill, minmax(156px, 1fr))")
        .gap_4();
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

pub fn dedup_append<T, K: Eq>(existing: &mut Vec<T>, incoming: Vec<T>, key: impl Fn(&T) -> K) {
    for item in incoming {
        let k = key(&item);
        if !existing.iter().any(|cur| key(cur) == k) {
            existing.push(item);
        }
    }
}

pub fn pagination_footer(
    loading_more: bool,
    load_more_error: Option<String>,
    on_retry: impl Fn() + 'static,
) -> impl IntoElement {
    if loading_more {
        return view()
            .w_full()
            .py_4()
            .items_center()
            .child(
                text("正在加载更多...")
                    .text_xs()
                    .text_color(token::muted_fg),
            )
            .into_element();
    }

    if let Some(err) = load_more_error {
        return view()
            .w_full()
            .py_4()
            .flex_row()
            .items_center()
            .justify_center()
            .gap_3()
            .child(text(err).text_xs().text_color(token::destructive))
            .child(app_button(
                "重试",
                ButtonVariant::Secondary,
                ControlSize::Sm,
                on_retry,
            ))
            .into_element();
    }

    view().into_element()
}
