use hg_contracts::ids::{EpisodeNumber, SeriesId};
use hg_contracts::viewing::SeriesBookmark;
use nsh_core::prelude::*;

use super::super::store;
use super::query::{self, episode_status, format_score};
use crate::ui::icons::IconKind;
use crate::ui::poster;
use crate::ui::surface::{BadgeTone, badge, empty_state};
use crate::ui::token;

pub fn bookmarks_section<R>(
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
    row_action: impl Fn(SeriesId, String) -> R + Copy + 'static,
) -> impl IntoElement
where
    R: IntoElement + 'static,
{
    let section_owner = Owner::current();
    query::load_bookmarks();
    view().w_full().gap_3().child(dynamic(move || {
        bookmarks_body(section_owner, on_open_series, row_action)
    }))
}

fn bookmarks_body<R>(
    section_owner: Owner,
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
    row_action: impl Fn(SeriesId, String) -> R + Copy + 'static,
) -> impl IntoElement
where
    R: IntoElement + 'static,
{
    let snap = store::state().get();
    let header = section_header_row();

    if snap.error.is_some() && snap.bookmarks.is_empty() {
        return view()
            .w_full()
            .gap_3()
            .child(header)
            .child(empty_state(
                IconKind::Bookmark,
                "追剧收藏加载失败",
                "请稍后重试刷新追剧列表",
                Some((
                    "重新加载",
                    Box::new(move || section_owner.run(query::load_bookmarks)),
                )),
            ))
            .into_element();
    }

    if snap.loading_bookmarks && snap.bookmarks.is_empty() {
        return view()
            .w_full()
            .gap_3()
            .child(header)
            .child(
                view().py_6().items_center().child(
                    text("正在加载追剧收藏...")
                        .text_sm()
                        .text_color(token::muted_fg),
                ),
            )
            .into_element();
    }

    if snap.bookmarks.is_empty() {
        return view()
            .w_full()
            .gap_3()
            .child(header)
            .child(empty_state(
                IconKind::Bookmark,
                "暂无追剧收藏",
                "在播放页点击追剧后将在此集中展示",
                None,
            ))
            .into_element();
    }

    let mut list = view().w_full().gap_2p5();
    for (index, entry) in snap.bookmarks.into_iter().enumerate() {
        list = list.child(bookmark_row(index, entry, on_open_series, row_action));
    }

    view()
        .w_full()
        .gap_3()
        .child(header)
        .child(list)
        .into_element()
}

fn section_header_row() -> impl IntoElement {
    view().w_full().flex_row().items_center().child(
        text("我的追剧")
            .role(AccessibilityRole::Header)
            .text_base()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(token::fg),
    )
}

fn bookmark_row<R>(
    index: usize,
    entry: SeriesBookmark,
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
    row_action: impl Fn(SeriesId, String) -> R + Copy + 'static,
) -> impl IntoElement
where
    R: IntoElement + 'static,
{
    let enter = token::stagger_enter(index);
    let series_id = entry.series_id;
    let title = entry.title.clone();
    let a11y_title = entry.title.clone();
    let cover_key = entry.cover_url.clone();
    let score_text = format_score(entry.score);
    let status_text = episode_status(&entry);

    let open_trigger = view()
        .on_press(move || on_open_series(series_id, None))
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
        .child(bookmark_meta(
            entry.title,
            score_text,
            status_text,
            entry.is_finished,
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

fn bookmark_meta(
    title: String,
    score_text: String,
    status_text: String,
    is_finished: bool,
) -> impl IntoElement {
    let status_badge = if is_finished {
        badge(status_text, BadgeTone::Success)
    } else {
        badge(status_text, BadgeTone::Brand)
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
                .child(badge(score_text, BadgeTone::Neutral))
                .child(status_badge),
        )
}
