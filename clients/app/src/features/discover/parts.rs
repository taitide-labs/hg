use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::Hash;
use std::rc::Rc;

use hg_contracts::catalog::SeriesSummary;
use hg_contracts::ids::SeriesId;
use nsh_core::prelude::*;

use super::store::{self, SeriesWatchStatus, WatchedSeriesSnapshot};
use crate::gateway;
use crate::ui::button::{ButtonVariant, ControlSize, app_button};
use crate::ui::icons::IconKind;
use crate::ui::platform::is_desktop;
use crate::ui::poster::{
    PosterContextActions, PosterWatchStatus, SeriesCardProps, SeriesFlags, SeriesRank, series_card,
};
use crate::ui::surface::empty_state;
use crate::ui::token;

pub const DESKTOP_CARD_MIN_WIDTH: f32 = 236.0;
pub const DESKTOP_MAX_COLUMNS: f32 = 6.0;
pub const DESKTOP_GRID_GAP: f32 = 12.0;
pub const MOBILE_CARD_MIN_WIDTH: f32 = 152.0;
pub const MOBILE_GRID_GAP: f32 = 12.0;

#[must_use]
pub fn series_grid_columns_for_width(inner_width: f32) -> usize {
    grid_columns_for(inner_width, is_desktop())
}

fn grid_columns_for(inner_width: f32, desktop_tracks: bool) -> usize {
    let (min_w, gap, max_cols) = if desktop_tracks {
        (
            DESKTOP_CARD_MIN_WIDTH,
            DESKTOP_GRID_GAP,
            DESKTOP_MAX_COLUMNS,
        )
    } else {
        (MOBILE_CARD_MIN_WIDTH, MOBILE_GRID_GAP, 4.0)
    };
    ((inner_width + gap) / (min_w + gap))
        .floor()
        .clamp(2.0, max_cols) as usize
}

#[must_use]
pub fn shelf_card_width(inner_width: f32, cols: usize) -> f32 {
    let gap = if is_desktop() {
        DESKTOP_GRID_GAP
    } else {
        MOBILE_GRID_GAP
    };
    let count = cols.max(2) as f32;
    ((inner_width - gap * (count - 1.0)) / count).max(MOBILE_CARD_MIN_WIDTH)
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
pub fn adaptive_page_size(viewport_width: f32, viewport_height: f32, _is_ranking: bool) -> u16 {
    let (cols, visible_rows) = visible_grid_rows(viewport_width, viewport_height);
    let desired_rows = visible_rows.saturating_mul(2).saturating_add(1).max(4);
    let aligned = cols.saturating_mul(desired_rows);
    u16::try_from(aligned.clamp(24, 60)).unwrap_or(60)
}

#[must_use]
pub fn min_items_to_fill_viewport(
    viewport_width: f32,
    viewport_height: f32,
    _is_ranking: bool,
) -> usize {
    let (cols, visible_rows) = visible_grid_rows(viewport_width, viewport_height);
    (cols as usize).saturating_mul((visible_rows as usize).saturating_add(1))
}

pub async fn sync_poster_preferences() {
    let gw = gateway::current();
    let (disliked_res, bookmarks_res) = gateway::join2(gw.disliked_series(), gw.bookmarks()).await;
    if let Ok(disliked) = disliked_res {
        let ids: BTreeSet<SeriesId> = disliked.into_iter().map(|entry| entry.series_id).collect();
        store::set_disliked_ids(ids);
    }
    if let Ok(bookmarks) = bookmarks_res {
        let ids: BTreeSet<SeriesId> = bookmarks.into_iter().map(|entry| entry.series_id).collect();
        store::set_bookmarked_ids(ids);
    }
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

#[must_use]
pub fn series_flags(item: &SeriesSummary) -> SeriesFlags {
    SeriesFlags {
        is_original: item.is_original,
        is_hot: item.is_hot,
        is_new: item.is_new,
    }
}

#[must_use]
pub fn series_card_props(item: &SeriesSummary) -> SeriesCardProps {
    series_card_props_with_rank(item, None)
}

#[must_use]
pub fn series_card_props_with_rank(
    item: &SeriesSummary,
    rank: Option<SeriesRank>,
) -> SeriesCardProps {
    let tags_text = item
        .tags
        .iter()
        .take(3)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(" · ");
    SeriesCardProps {
        id: item.id.get(),
        title: SharedString::from(item.title.clone()),
        cover_url: SharedString::from(item.cover_url.clone()),
        score: item.score,
        tags_text: SharedString::from(tags_text),
        episode_count: item.episode_count,
        is_finished: item.is_finished,
        flags: series_flags(item),
        rank,
    }
}

fn watch_status_for_raw_id(id: u64, episode_count: u32) -> PosterWatchStatus {
    let Ok(series_id) = SeriesId::try_from(id) else {
        return PosterWatchStatus::Unwatched;
    };
    match store::series_watch_status(series_id, episode_count) {
        SeriesWatchStatus::Unwatched => PosterWatchStatus::Unwatched,
        SeriesWatchStatus::AllWatched => PosterWatchStatus::AllWatched,
        SeriesWatchStatus::UpdatedAfterWatching => PosterWatchStatus::UpdatedAfterWatching,
    }
}

pub fn poster_context_actions(
    on_open_download: impl Fn(SeriesId) + Copy + 'static,
    on_after_dislike: impl Fn() + Copy + 'static,
) -> PosterContextActions {
    let owner = Owner::current();
    PosterContextActions {
        is_bookmarked: Rc::new(|raw_id| {
            SeriesId::try_from(raw_id).is_ok_and(store::is_bookmarked_untracked)
        }),
        is_disliked: Rc::new(|raw_id| {
            SeriesId::try_from(raw_id).is_ok_and(store::is_disliked_untracked)
        }),
        on_toggle_bookmark: Rc::new(move |raw_id| {
            let Ok(series_id) = SeriesId::try_from(raw_id) else {
                return;
            };
            let next_bookmarked = store::toggle_bookmarked_local(series_id);
            let gw = gateway::current();
            owner.run(move || {
                spawn(async move {
                    let res = if next_bookmarked {
                        gw.bookmark_series(series_id).await
                    } else {
                        gw.unbookmark_series(series_id).await
                    };
                    if res.is_err() {
                        store::set_bookmarked_local(series_id, !next_bookmarked);
                        crate::ui::toast::show_toast("追剧操作失败，请稍后重试");
                    } else {
                        crate::ui::toast::show_toast(if next_bookmarked {
                            "已加入追剧"
                        } else {
                            "已取消追剧"
                        });
                    }
                });
            });
        }),
        on_open_download: Rc::new(move |raw_id| {
            if let Ok(series_id) = SeriesId::try_from(raw_id) {
                owner.run(move || on_open_download(series_id));
            }
        }),
        on_toggle_dislike: Rc::new(move |raw_id| {
            let Ok(series_id) = SeriesId::try_from(raw_id) else {
                return;
            };
            store::add_disliked_id(series_id);
            owner.run(on_after_dislike);
            let gw = gateway::current();
            owner.run(move || {
                spawn(async move {
                    if gw.dislike_series(series_id).await.is_err() {
                        store::remove_disliked_id(series_id);
                        crate::ui::toast::show_toast("标记不喜欢失败，请稍后重试");
                    } else {
                        crate::ui::toast::show_toast("已隐藏，可在设置中恢复");
                    }
                });
            });
        }),
    }
}

pub fn series_card_reactive(
    item: Memo<Option<SeriesCardProps>>,
    on_open_series: impl Fn(SeriesId) + 'static,
    context_actions: PosterContextActions,
) -> impl IntoElement {
    series_card(
        item,
        watch_status_for_raw_id,
        move |raw_id| {
            if let Ok(series_id) = SeriesId::try_from(raw_id) {
                on_open_series(series_id);
            }
        },
        Some(context_actions),
    )
}

pub fn series_horizontal_shelf(
    items: Memo<Vec<SeriesCardProps>>,
    card_width: Memo<f32>,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
    context_actions: &PosterContextActions,
) -> impl IntoElement {
    let actions = context_actions.clone();
    virtual_list(
        move || items.with(Vec::len),
        move |at: Position| {
            let item = computed(move || items.with(|list| list.get(at.index()).cloned()));
            view()
                .w(move || px(card_width.get()))
                .child(series_card_reactive(item, on_open_series, actions.clone()))
        },
    )
    .horizontal()
    .scrollbar(false)
    .keys(move || items.with(|list| list.iter().map(|p| p.id).collect::<Vec<u64>>()))
    .estimated_item_size(move || px(card_width.get()))
    .column_gap(px(12.0))
    .w_full()
    .h(move || px((card_width.get() * (224.0 / 160.0)).ceil() + 54.0))
    .empty(empty_state(
        IconKind::Search,
        "暂无匹配剧目",
        "尝试更换关键词或筛选条件",
        None,
    ))
}

pub struct VirtualGridCallbacks {
    pub on_open_series: Rc<dyn Fn(SeriesId)>,
    pub context_actions: PosterContextActions,
    pub on_load_more: Rc<dyn Fn()>,
    pub empty_view: Rc<dyn Fn() -> AnyElement>,
}

pub fn virtual_series_grid(
    count: impl Fn() -> usize + Copy + 'static,
    keys: impl Fn() -> Vec<SeriesId> + Copy + 'static,
    item_at: impl Fn(usize) -> Option<SeriesCardProps> + Copy + 'static,
    columns: impl Fn() -> u32 + Copy + 'static,
    callbacks: VirtualGridCallbacks,
) -> VirtualList {
    let gap = if is_desktop() {
        DESKTOP_GRID_GAP
    } else {
        MOBILE_GRID_GAP
    };
    let estimated_h = if is_desktop() { 310.0 } else { 260.0 };
    let cols_memo = computed(columns);
    let on_open_series = callbacks.on_open_series;
    let context_actions = callbacks.context_actions;
    let on_load_more = callbacks.on_load_more;
    let empty_view = callbacks.empty_view;

    virtual_list(count, move |at: Position| {
        let item = computed(move || item_at(at.index()));
        let on_open = Rc::clone(&on_open_series);
        series_card_reactive(
            item,
            move |series_id| on_open(series_id),
            context_actions.clone(),
        )
    })
    .keys(keys)
    .columns(cols_memo)
    .row_gap(px(gap))
    .column_gap(px(gap))
    .content_padding(px(24.0))
    .estimated_item_size(px(estimated_h))
    .load_more_threshold(1.0)
    .on_load_more(move || on_load_more())
    .empty(dynamic(move || empty_view()))
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
