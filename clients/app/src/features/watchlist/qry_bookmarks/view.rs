use hg_contracts::ids::{EpisodeNumber, SeriesId};
use hg_contracts::viewing::SeriesBookmark;
use nsh_core::prelude::*;

use super::super::store;
use super::query::{self, episode_status, format_score};
use crate::ui::icons::IconKind;
use crate::ui::poster;
use crate::ui::surface::{BadgeTone, badge, empty_state, list_row_skeleton};
use crate::ui::token;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BookmarksPhase {
    Error,
    Loading,
    Empty,
    Ready,
}

pub fn bookmarks_section<R>(
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
    row_action: impl Fn(SeriesId, String) -> R + Copy + 'static,
) -> impl IntoElement
where
    R: IntoElement + 'static,
{
    let section_owner = Owner::current();
    query::load_bookmarks();
    let phase = computed(|| {
        store::state().with(|s| {
            if s.error.is_some() && s.bookmarks.is_empty() {
                BookmarksPhase::Error
            } else if s.loading_bookmarks && s.bookmarks.is_empty() {
                BookmarksPhase::Loading
            } else if s.bookmarks.is_empty() {
                BookmarksPhase::Empty
            } else {
                BookmarksPhase::Ready
            }
        })
    });
    let bookmarks = computed(|| store::state().with(|s| s.bookmarks.clone()));

    view()
        .w_full()
        .gap_3()
        .child(section_header_row())
        .child(dynamic(move || match phase.get() {
            BookmarksPhase::Error => empty_state(
                IconKind::Bookmark,
                "追剧收藏加载失败",
                "请稍后重试刷新追剧列表",
                Some((
                    "重新加载",
                    Box::new(move || section_owner.run(query::load_bookmarks)),
                )),
            )
            .into_element(),
            BookmarksPhase::Loading => list_row_skeleton(4, "追剧列表加载占位").into_element(),
            BookmarksPhase::Empty => empty_state(
                IconKind::Bookmark,
                "暂无追剧收藏",
                "在播放页点击追剧后将在此集中展示",
                None,
            )
            .into_element(),
            BookmarksPhase::Ready => view()
                .w_full()
                .gap_2p5()
                .child(each(
                    bookmarks,
                    |b: &SeriesBookmark| b.series_id,
                    move |entry_sig, index_sig| {
                        bookmark_row(index_sig, entry_sig, on_open_series, row_action)
                    },
                ))
                .into_element(),
        }))
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
    index_sig: ReadSignal<usize>,
    entry_sig: ReadSignal<SeriesBookmark>,
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
    row_action: impl Fn(SeriesId, String) -> R + Copy + 'static,
) -> impl IntoElement
where
    R: IntoElement + 'static,
{
    let enter = token::stagger_enter(index_sig.get_untracked());
    let (series_id, title) = entry_sig.with_untracked(|e| (e.series_id, e.title.clone()));
    let cover_key = computed(move || entry_sig.with(|e| e.cover_url.clone()));

    let open_trigger = view()
        .on_press(move || on_open_series(series_id, None))
        .role(AccessibilityRole::Button)
        .focusable(true)
        .label(move || entry_sig.with(|e| SharedString::from(e.title.clone())))
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
        .child(bookmark_meta(entry_sig));

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

fn poster_thumb(cover_key: Memo<String>) -> impl IntoElement {
    poster::reactive_poster_thumb(cover_key, 52.0, 70.0, IconKind::Play, 26.0, 12.0)
}

fn bookmark_meta(entry_sig: ReadSignal<SeriesBookmark>) -> impl IntoElement {
    let is_finished = computed(move || entry_sig.with(|e| e.is_finished));

    view()
        .flex_1()
        .min_w_0()
        .gap_1p5()
        .child(
            text(move || entry_sig.with(|e| SharedString::from(e.title.clone())))
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
                .child(badge(
                    move || entry_sig.with(|e| SharedString::from(format_score(e.score))),
                    BadgeTone::Neutral,
                ))
                .child(dynamic(move || {
                    let tone = if is_finished.get() {
                        BadgeTone::Success
                    } else {
                        BadgeTone::Brand
                    };
                    badge(
                        move || entry_sig.with(|e| SharedString::from(episode_status(e))),
                        tone,
                    )
                })),
        )
}
