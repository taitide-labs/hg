use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use hg_contracts::catalog::SeriesSummary;
use hg_contracts::ids::SeriesId;
use nsh_core::prelude::*;

use super::store::{self, SeriesWatchStatus, WatchedSeriesSnapshot};
use crate::ui::button::{ButtonVariant, ControlSize, app_button};
use crate::ui::icons::IconKind;
use crate::ui::platform::is_desktop;
use crate::ui::poster::{PosterWatchStatus, SeriesCardProps, series_card};
use crate::ui::surface::empty_state;
use crate::ui::token;

pub const DESKTOP_CARD_MIN_WIDTH: f32 = 188.0;
pub const DESKTOP_GRID_GAP: f32 = 20.0;
pub const MOBILE_CARD_MIN_WIDTH: f32 = 152.0;
pub const MOBILE_GRID_GAP: f32 = 16.0;

#[must_use]
pub fn series_grid_columns_for_width(inner_width: f32) -> usize {
    grid_columns_for(inner_width, is_desktop())
}

#[must_use]
pub fn ranking_grid_columns_for_width(container_width: f32) -> usize {
    if container_width >= 1360.0 {
        3
    } else if container_width >= 700.0 {
        2
    } else {
        1
    }
}

fn grid_columns_for(inner_width: f32, desktop_tracks: bool) -> usize {
    let (min_w, gap) = if desktop_tracks {
        (DESKTOP_CARD_MIN_WIDTH, DESKTOP_GRID_GAP)
    } else {
        (MOBILE_CARD_MIN_WIDTH, MOBILE_GRID_GAP)
    };
    ((inner_width + gap) / (min_w + gap)).floor().max(2.0) as usize
}

fn visible_grid_rows(viewport_width: f32, viewport_height: f32) -> (u32, u32) {
    let desktop_tracks = viewport_width >= 760.0;
    let inner_w = (viewport_width - 48.0).max(0.0);
    let cols = (grid_columns_for(inner_w, desktop_tracks) as u32).max(2);
    let gap = if desktop_tracks {
        DESKTOP_GRID_GAP
    } else {
        MOBILE_GRID_GAP
    };
    let col_w = ((inner_w - gap * (cols.saturating_sub(1) as f32)) / (cols as f32)).max(152.0);
    let card_h = col_w * 1.4 + 66.0 + gap;
    let rows = (viewport_height / card_h).ceil().max(2.0) as u32;
    (cols, rows)
}

#[must_use]
pub fn adaptive_page_size(viewport_width: f32, viewport_height: f32, is_ranking: bool) -> u16 {
    if is_ranking {
        let cols = ranking_grid_columns_for_width(viewport_width.max(0.0)) as u32;
        let visible_rows = (viewport_height / 132.0).ceil().max(4.0) as u32;
        let aligned = cols.saturating_mul(visible_rows.saturating_mul(2));
        return u16::try_from(aligned.clamp(24, 60)).unwrap_or(60);
    }
    let (cols, visible_rows) = visible_grid_rows(viewport_width, viewport_height);
    let desired_rows = visible_rows.saturating_mul(2).max(4);
    let aligned = cols.saturating_mul(desired_rows);
    u16::try_from(aligned.clamp(24, 60)).unwrap_or(60)
}

#[must_use]
pub fn min_items_to_fill_viewport(
    viewport_width: f32,
    viewport_height: f32,
    is_ranking: bool,
) -> usize {
    if is_ranking {
        let cols = ranking_grid_columns_for_width(viewport_width.max(0.0));
        let visible_rows = (viewport_height / 132.0).ceil().max(2.0) as usize;
        return cols.saturating_mul(visible_rows.saturating_add(1));
    }
    let (cols, visible_rows) = visible_grid_rows(viewport_width, viewport_height);
    (cols as usize).saturating_mul((visible_rows as usize).saturating_add(1))
}

pub async fn sync_watched_series() {
    if let Ok(history) = crate::gateway::current().watch_history().await {
        let watched: HashMap<SeriesId, WatchedSeriesSnapshot> = history
            .into_iter()
            .map(|entry| {
                let over_80 = entry.episode_completed
                    || (entry.duration_secs > 0
                        && u64::from(entry.position_secs).saturating_mul(100)
                            > u64::from(entry.duration_secs).saturating_mul(80));
                (
                    entry.series_id,
                    WatchedSeriesSnapshot {
                        last_episode: entry.last_episode.get(),
                        recorded_total_episodes: entry.total_episodes,
                        episode_over_80: over_80,
                    },
                )
            })
            .collect();
        store::set_watched_series(watched);
    }
}

pub fn series_card_from_summary(
    item: SeriesSummary,
    index: usize,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
) -> impl IntoElement {
    let id = item.id;
    let watch_status = match store::series_watch_status(id, item.episode_count) {
        SeriesWatchStatus::Unwatched => PosterWatchStatus::Unwatched,
        SeriesWatchStatus::AllWatched => PosterWatchStatus::AllWatched,
        SeriesWatchStatus::UpdatedAfterWatching => PosterWatchStatus::UpdatedAfterWatching,
    };
    let status = match watch_status {
        PosterWatchStatus::UpdatedAfterWatching => {
            format!("有更新 · {} 集", item.episode_count)
        }
        PosterWatchStatus::Unwatched | PosterWatchStatus::AllWatched if item.is_finished => {
            format!("全 {} 集", item.episode_count)
        }
        PosterWatchStatus::Unwatched | PosterWatchStatus::AllWatched => {
            format!("更新至 {} 集", item.episode_count)
        }
    };
    let subtitle = item.tags.join(" · ");
    series_card(
        SeriesCardProps {
            title: item.title,
            cover_url: item.cover_url,
            score: item.score,
            subtitle,
            episode_label: status,
            badge_text: item.badge,
            watch_status,
            index,
        },
        move || on_open_series(id),
    )
}

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

    let grid = view().w_full().grid();
    let mut grid = if is_desktop() {
        grid.grid_template_columns("repeat(auto-fill, minmax(188px, 1fr))")
            .gap_5()
    } else {
        grid.grid_template_columns("repeat(auto-fill, minmax(152px, 1fr))")
            .gap_4()
    };
    for (index, item) in items.into_iter().enumerate() {
        grid = grid.child(series_card_from_summary(item, index, on_open_series));
    }
    grid.into_element()
}

pub fn virtual_series_grid(
    count: impl Fn() -> usize + Copy + 'static,
    keys: impl Fn() -> Vec<SeriesId> + Copy + 'static,
    item_at: impl Fn(usize) -> Option<SeriesSummary> + Copy + 'static,
    columns: impl Fn() -> u32 + Copy + 'static,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
    on_load_more: impl Fn() + Copy + 'static,
    empty_view: impl Fn() -> AnyElement + Copy + 'static,
) -> VirtualList {
    let gap = if is_desktop() {
        DESKTOP_GRID_GAP
    } else {
        MOBILE_GRID_GAP
    };
    let estimated_h = if is_desktop() { 340.0 } else { 290.0 };

    virtual_list(count, move |at: Position| {
        view().child(dynamic(move || {
            let idx = at.index();
            match item_at(idx) {
                Some(item) => series_card_from_summary(item, idx, on_open_series).into_element(),
                None => view().into_element(),
            }
        }))
    })
    .keys(keys)
    .columns(columns)
    .row_gap(px(gap))
    .column_gap(px(gap))
    .content_padding(px(24.0))
    .estimated_item_size(px(estimated_h))
    .load_more_threshold(1.0)
    .on_load_more(on_load_more)
    .empty(dynamic(empty_view))
    .flex_1()
    .w_full()
    .min_h_0()
}

pub fn dedup_append<T, K: Eq + Hash>(
    existing: &mut Vec<T>,
    incoming: Vec<T>,
    key: impl Fn(&T) -> K,
) {
    let mut seen: HashSet<K> = existing.iter().map(&key).collect();
    for item in incoming {
        let k = key(&item);
        if seen.insert(k) {
            existing.push(item);
        }
    }
}

pub fn pagination_footer(
    loading_more: bool,
    load_more_error: Option<String>,
    all_loaded: bool,
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

    if all_loaded {
        return view()
            .w_full()
            .py_4()
            .items_center()
            .child(text("已全部加载").text_xs().text_color(token::muted_fg))
            .into_element();
    }

    view().into_element()
}
