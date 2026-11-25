use hg_contracts::ids::{EpisodeNumber, SeriesId};
use hg_contracts::offline::OfflineSeriesView;
use nsh_core::prelude::*;

use super::super::store;
use super::query::{
    self, episodes_preview_text, first_downloaded_episode, format_size_bytes, library_totals,
    series_stats_text,
};
use crate::ui::icons::IconKind;
use crate::ui::poster;
use crate::ui::surface::{BadgeTone, badge, empty_state, list_row_skeleton};
use crate::ui::token;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LibraryPhase {
    Error,
    Loading,
    EmptyNoTasks,
    EmptyWithTasks,
    Ready,
}

pub fn offline_library_section<R>(
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
    series_action: impl Fn(SeriesId, &str) -> R + Copy + 'static,
) -> impl IntoElement
where
    R: IntoElement + 'static,
{
    let section_owner = Owner::current();
    query::load_library();
    let phase = computed(|| {
        store::state().with(|s| {
            if s.library_error.is_some() && s.library.is_empty() {
                LibraryPhase::Error
            } else if !s.has_loaded_library && s.loading_library && s.library.is_empty() {
                LibraryPhase::Loading
            } else if s.library.is_empty() && s.tasks.is_empty() {
                LibraryPhase::EmptyNoTasks
            } else if s.library.is_empty() {
                LibraryPhase::EmptyWithTasks
            } else {
                LibraryPhase::Ready
            }
        })
    });
    let library = computed(|| store::state().with(|s| s.library.clone()));
    let summary = computed(move || {
        library.with(|items| {
            let (series_count, episode_count, total_bytes) = library_totals(items);
            if series_count > 0 {
                SharedString::from(format!(
                    "共 {series_count} 部 · {episode_count} 集 · {}",
                    format_size_bytes(total_bytes)
                ))
            } else {
                SharedString::default()
            }
        })
    });

    view()
        .w_full()
        .gap_3()
        .child(section_header_row(summary))
        .child(dynamic(move || match phase.get() {
            LibraryPhase::Error => empty_state(
                IconKind::Download,
                "离线片库加载失败",
                "请稍后重试刷新本地已下载内容",
                Some((
                    "重新加载",
                    Box::new(move || section_owner.run(query::load_library)),
                )),
            )
            .into_element(),
            LibraryPhase::Loading => list_row_skeleton(4, "离线片库加载占位").into_element(),
            LibraryPhase::EmptyNoTasks => empty_state(
                IconKind::Download,
                "暂无离线剧目",
                "在播放页选择分集离线下载后，可在此断网观看",
                None,
            )
            .into_element(),
            LibraryPhase::EmptyWithTasks => view()
                .py_6()
                .items_center()
                .child(
                    text("暂无已下载剧目，下载完成后将自动归档至此")
                        .text_sm()
                        .text_color(token::muted_fg),
                )
                .into_element(),
            LibraryPhase::Ready => view()
                .w_full()
                .gap_2p5()
                .child(each(
                    library,
                    |s: &OfflineSeriesView| s.series_id,
                    move |series_sig, _idx| series_row(series_sig, on_open_series, series_action),
                ))
                .into_element(),
        }))
}

fn section_header_row(summary: Memo<SharedString>) -> impl IntoElement {
    view()
        .w_full()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .child(
            view()
                .gap_0p5()
                .child(
                    text("已下载")
                        .role(AccessibilityRole::Header)
                        .text_base()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(token::fg),
                )
                .child(text("托管离线片库").text_xs().text_color(token::muted_fg)),
        )
        .child(
            text(summary)
                .style_when(move || summary.with(|s| s.is_empty()), |s| s.hidden())
                .text_xs()
                .tabular_nums()
                .text_color(token::muted_fg),
        )
}

fn series_row<R>(
    series_sig: ReadSignal<OfflineSeriesView>,
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
    series_action: impl Fn(SeriesId, &str) -> R + Copy + 'static,
) -> impl IntoElement
where
    R: IntoElement + 'static,
{
    let (series_id, title) = series_sig.with_untracked(|s| (s.series_id, s.title.clone()));
    let cover_key = computed(move || series_sig.with(|s| s.cover_local_uri.clone()));

    let open_trigger = view()
        .on_press(move || {
            let first_ep = series_sig.with_untracked(first_downloaded_episode);
            on_open_series(series_id, first_ep);
        })
        .role(AccessibilityRole::Button)
        .focusable(true)
        .label(move || series_sig.with(|s| SharedString::from(s.title.clone())))
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
        .child(series_meta(series_sig));

    view()
        .w_full()
        .rounded(token::radius_card)
        .bg(token::card)
        .border_1()
        .border_color(token::border)
        .flex_row()
        .items_center()
        .child(open_trigger)
        .child(series_action(series_id, &title))
}

fn poster_thumb(cover_key: Memo<String>) -> impl IntoElement {
    poster::reactive_poster_thumb(cover_key, 52.0, 70.0, IconKind::Play, 26.0, 12.0)
}

fn series_meta(series_sig: ReadSignal<OfflineSeriesView>) -> impl IntoElement {
    view()
        .flex_1()
        .min_w_0()
        .gap_1p5()
        .child(
            text(move || series_sig.with(|s| SharedString::from(s.title.clone())))
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
                .child(badge("离线就绪", BadgeTone::Success))
                .child(
                    text(move || series_sig.with(|s| SharedString::from(series_stats_text(s))))
                        .text_xs()
                        .tabular_nums()
                        .text_color(token::muted_fg)
                        .truncate(),
                ),
        )
        .child(
            text(move || {
                series_sig.with(|s| SharedString::from(episodes_preview_text(&s.episodes)))
            })
            .text_xs()
            .text_color(token::muted_fg)
            .truncate(),
        )
}
