use hg_contracts::ids::{EpisodeNumber, SeriesId};
use hg_contracts::viewing::WatchHistoryEntry;
use nsh_core::prelude::*;

use super::super::store;
use super::query::{self, progress_ratio, progress_summary};
use crate::ui::icons::IconKind;
use crate::ui::poster;
use crate::ui::surface::{BadgeTone, badge, empty_state, progress_track};
use crate::ui::token;

pub fn history_section<H, R>(
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
    header_action: impl Fn() -> H + Copy + 'static,
    row_action: impl Fn(SeriesId, String) -> R + Copy + 'static,
) -> impl IntoElement
where
    H: IntoElement + 'static,
    R: IntoElement + 'static,
{
    query::load_history();
    view().w_full().gap_3().child(dynamic(move || {
        history_body(on_open_series, header_action, row_action)
    }))
}

fn history_body<H, R>(
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
    header_action: impl Fn() -> H + Copy + 'static,
    row_action: impl Fn(SeriesId, String) -> R + Copy + 'static,
) -> impl IntoElement
where
    H: IntoElement + 'static,
    R: IntoElement + 'static,
{
    let snap = store::state().get();
    let header = section_header_row(!snap.history.is_empty(), header_action);

    if snap.error.is_some() && snap.history.is_empty() {
        return view()
            .w_full()
            .gap_3()
            .child(header)
            .child(empty_state(
                IconKind::Clock,
                "观看历史加载失败",
                "请稍后重试刷新观看记录",
                Some(("重新加载", Box::new(query::load_history))),
            ))
            .into_element();
    }

    if snap.loading_history && snap.history.is_empty() {
        return view()
            .w_full()
            .gap_3()
            .child(header)
            .child(
                view().py_6().items_center().child(
                    text("正在加载观看历史...")
                        .text_sm()
                        .text_color(token::muted_fg),
                ),
            )
            .into_element();
    }

    if snap.history.is_empty() {
        return view()
            .w_full()
            .gap_3()
            .child(header)
            .child(empty_state(
                IconKind::Clock,
                "暂无观看历史",
                "开始观看任意剧集后将在此保留续播进度",
                None,
            ))
            .into_element();
    }

    let mut list = view().w_full().gap_2p5();
    for (index, entry) in snap.history.into_iter().enumerate() {
        list = list.child(history_row(index, entry, on_open_series, row_action));
    }

    view()
        .w_full()
        .gap_3()
        .child(header)
        .child(list)
        .into_element()
}

fn section_header_row<H>(
    show_action: bool,
    header_action: impl Fn() -> H + Copy + 'static,
) -> impl IntoElement
where
    H: IntoElement + 'static,
{
    view()
        .w_full()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .child(
            text("最近观看")
                .role(AccessibilityRole::Header)
                .text_base()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(token::fg),
        )
        .children(show_action.then(header_action))
}

fn history_row<R>(
    index: usize,
    entry: WatchHistoryEntry,
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
    row_action: impl Fn(SeriesId, String) -> R + Copy + 'static,
) -> impl IntoElement
where
    R: IntoElement + 'static,
{
    let enter = token::stagger_enter(index);
    let series_id = entry.series_id;
    let last_episode = entry.last_episode;
    let title = entry.title.clone();
    let a11y_title = entry.title.clone();
    let cover_key = entry.cover_url.clone();
    let episode_text = format!("第 {} 集", last_episode.get());
    let summary_text = progress_summary(&entry);
    let ratio = progress_ratio(&entry);
    let completed = entry.episode_completed;

    let open_trigger = view()
        .on_press(move || on_open_series(series_id, Some(last_episode)))
        .role(AccessibilityRole::Button)
        .focusable(true)
        .label(a11y_title)
        .flex_1()
        .min_w_0()
        .p_3()
        .rounded(token::radius_card)
        .flex_row()
        .items_center()
        .gap_3p5()
        .transition_colors()
        .duration(token::INSTANT_MS)
        .hover(|s| s.bg(token::surface_subtle))
        .active(|s| {
            if reduce_motion() {
                s.opacity(0.9)
            } else {
                s.scale(token::PRESS_SCALE)
            }
        })
        .focus_visible(|s| s.outline(px(2.0)).outline_color(token::ring))
        .child(poster_thumb(cover_key))
        .child(history_meta(
            entry.title,
            episode_text,
            summary_text,
            completed,
            ratio,
        ));

    view()
        .bind_motion(&enter)
        .w_full()
        .rounded(token::radius_card)
        .bg(token::card)
        .border_1()
        .border_color(token::border)
        .flex_row()
        .items_center()
        .child(open_trigger)
        .child(row_action(series_id, title))
}

fn poster_thumb(cover_key: String) -> impl IntoElement {
    poster::poster_thumb(cover_key, 52.0, 70.0, IconKind::Play, 26.0, 12.0)
}

fn history_meta(
    title: String,
    episode_text: String,
    summary_text: String,
    completed: bool,
    ratio: f32,
) -> impl IntoElement {
    let status_badge = if completed {
        badge(episode_text, BadgeTone::Success)
    } else {
        badge(episode_text, BadgeTone::Brand)
    };

    view()
        .flex_1()
        .min_w_0()
        .gap_1p5()
        .child(
            text(title)
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(token::card_fg)
                .truncate(),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(status_badge)
                .child(
                    text(summary_text)
                        .text_xs()
                        .tabular_nums()
                        .text_color(token::muted_fg)
                        .truncate(),
                ),
        )
        .child(progress_track(move || ratio))
}
