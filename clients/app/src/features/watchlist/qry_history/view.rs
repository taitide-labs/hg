use hg_contracts::ids::{EpisodeNumber, SeriesId};
use hg_contracts::viewing::WatchHistoryEntry;
use nsh_core::prelude::*;

use super::super::store;
use super::query::{self, progress_ratio, progress_summary};
use crate::ui::icons::IconKind;
use crate::ui::poster;
use crate::ui::surface::{BadgeTone, badge, empty_state, list_row_skeleton, progress_track};
use crate::ui::token;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HistoryPhase {
    Error,
    Loading,
    Empty,
    Ready,
}

pub fn history_section<H, R>(
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
    header_action: impl Fn() -> H + Copy + 'static,
    row_action: impl Fn(SeriesId, &str) -> R + Copy + 'static,
) -> impl IntoElement
where
    H: IntoElement + 'static,
    R: IntoElement + 'static,
{
    let section_owner = Owner::current();
    query::load_history();
    let phase = computed(|| {
        store::state().with(|s| {
            if s.error.is_some() && s.history.is_empty() {
                HistoryPhase::Error
            } else if s.loading_history && s.history.is_empty() {
                HistoryPhase::Loading
            } else if s.history.is_empty() {
                HistoryPhase::Empty
            } else {
                HistoryPhase::Ready
            }
        })
    });
    let history = computed(|| store::state().with(|s| s.history.clone()));

    view()
        .w_full()
        .gap_3()
        .child(dynamic(move || match phase.get() {
            HistoryPhase::Error => view()
                .w_full()
                .gap_3()
                .child(section_header_row(false, header_action))
                .child(empty_state(
                    IconKind::Clock,
                    "观看历史加载失败",
                    "请稍后重试刷新观看记录",
                    Some((
                        "重新加载",
                        Box::new(move || section_owner.run(query::load_history)),
                    )),
                ))
                .into_element(),
            HistoryPhase::Loading => view()
                .w_full()
                .gap_3()
                .child(section_header_row(false, header_action))
                .child(list_row_skeleton(4, "历史记录加载占位"))
                .into_element(),
            HistoryPhase::Empty => view()
                .w_full()
                .gap_3()
                .child(section_header_row(false, header_action))
                .child(empty_state(
                    IconKind::Clock,
                    "暂无观看历史",
                    "开始观看任意剧目后将在此保留续播进度",
                    None,
                ))
                .into_element(),
            HistoryPhase::Ready => view()
                .w_full()
                .gap_3()
                .child(section_header_row(true, header_action))
                .child(view().w_full().gap_2p5().child(each(
                    history,
                    |h: &WatchHistoryEntry| h.series_id,
                    move |entry_sig, index_sig| {
                        history_row(index_sig, entry_sig, on_open_series, row_action)
                    },
                )))
                .into_element(),
        }))
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
    index_sig: ReadSignal<usize>,
    entry_sig: ReadSignal<WatchHistoryEntry>,
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
    row_action: impl Fn(SeriesId, &str) -> R + Copy + 'static,
) -> impl IntoElement
where
    R: IntoElement + 'static,
{
    let enter = token::stagger_enter(index_sig.get_untracked());
    let (series_id, title) = entry_sig.with_untracked(|e| (e.series_id, e.title.clone()));
    let cover_key = computed(move || entry_sig.with(|e| e.cover_url.clone()));

    let open_trigger = view()
        .on_press(move || {
            let last_ep = entry_sig.with_untracked(|e| e.last_episode);
            on_open_series(series_id, Some(last_ep));
        })
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
        .child(history_meta(entry_sig));

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
        .child(row_action(series_id, &title))
}

fn poster_thumb(cover_key: Memo<String>) -> impl IntoElement {
    poster::reactive_poster_thumb(cover_key, 52.0, 70.0, IconKind::Play, 26.0, 12.0)
}

fn history_meta(entry_sig: ReadSignal<WatchHistoryEntry>) -> impl IntoElement {
    let completed = computed(move || entry_sig.with(|e| e.episode_completed));

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
                .child(dynamic(move || {
                    let tone = if completed.get() {
                        BadgeTone::Success
                    } else {
                        BadgeTone::Brand
                    };
                    badge(
                        move || {
                            entry_sig.with(|e| {
                                SharedString::from(format!("第 {} 集", e.last_episode.get()))
                            })
                        },
                        tone,
                    )
                }))
                .child(
                    text(move || entry_sig.with(|e| SharedString::from(progress_summary(e))))
                        .text_xs()
                        .tabular_nums()
                        .text_color(token::muted_fg)
                        .truncate(),
                ),
        )
        .child(progress_track(move || entry_sig.with(progress_ratio)))
}
