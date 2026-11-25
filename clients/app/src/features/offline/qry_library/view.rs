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
use crate::ui::surface::{BadgeTone, badge, empty_state};
use crate::ui::token;

pub fn offline_library_section<R>(
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
    series_action: impl Fn(SeriesId, String) -> R + Copy + 'static,
) -> impl IntoElement
where
    R: IntoElement + 'static,
{
    let section_owner = Owner::current();
    query::load_library();
    view().w_full().gap_3().child(dynamic(move || {
        library_body(section_owner, on_open_series, series_action)
    }))
}

fn library_body<R>(
    section_owner: Owner,
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
    series_action: impl Fn(SeriesId, String) -> R + Copy + 'static,
) -> impl IntoElement
where
    R: IntoElement + 'static,
{
    let snap = store::state().get();
    let (series_count, episode_count, total_bytes) = library_totals(&snap.library);
    let summary = if series_count > 0 {
        Some(format!(
            "共 {series_count} 部 · {episode_count} 集 · {}",
            format_size_bytes(total_bytes)
        ))
    } else {
        None
    };
    let header = section_header_row(summary);

    if snap.library_error.is_some() && snap.library.is_empty() {
        return view()
            .w_full()
            .gap_3()
            .child(header)
            .child(empty_state(
                IconKind::Download,
                "离线片库加载失败",
                "请稍后重试刷新本地已下载内容",
                Some((
                    "重新加载",
                    Box::new(move || section_owner.run(query::load_library)),
                )),
            ))
            .into_element();
    }

    if !snap.has_loaded_library && snap.loading_library && snap.library.is_empty() {
        return view()
            .w_full()
            .gap_3()
            .child(header)
            .child(
                view().py_6().items_center().child(
                    text("正在加载离线片库...")
                        .text_sm()
                        .text_color(token::muted_fg),
                ),
            )
            .into_element();
    }

    if snap.library.is_empty() {
        let empty = if snap.tasks.is_empty() {
            empty_state(
                IconKind::Download,
                "暂无离线剧目",
                "在播放页选择分集离线下载后，可在此断网观看",
                None,
            )
            .into_element()
        } else {
            view()
                .py_6()
                .items_center()
                .child(
                    text("暂无已下载剧目，下载完成后将自动归档至此")
                        .text_sm()
                        .text_color(token::muted_fg),
                )
                .into_element()
        };
        return view()
            .w_full()
            .gap_3()
            .child(header)
            .child(empty)
            .into_element();
    }

    let mut list = view().w_full().gap_2p5();
    for series in snap.library {
        list = list.child(series_row(series, on_open_series, series_action));
    }

    view()
        .w_full()
        .gap_3()
        .child(header)
        .child(list)
        .into_element()
}

fn section_header_row(summary: Option<String>) -> impl IntoElement {
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
        .children(summary.map(|label| {
            text(label)
                .text_xs()
                .tabular_nums()
                .text_color(token::muted_fg)
        }))
}

fn series_row<R>(
    series: OfflineSeriesView,
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
    series_action: impl Fn(SeriesId, String) -> R + Copy + 'static,
) -> impl IntoElement
where
    R: IntoElement + 'static,
{
    let series_id = series.series_id;
    let first_ep = first_downloaded_episode(&series);
    let title = series.title.clone();
    let a11y_title = series.title.clone();
    let cover_key = series.cover_local_uri.clone();
    let stats = series_stats_text(&series);
    let episodes_preview = episodes_preview_text(&series.episodes);

    let open_trigger = view()
        .on_press(move || on_open_series(series_id, first_ep))
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
        .child(series_meta(series.title, stats, episodes_preview));

    view()
        .w_full()
        .rounded(token::radius_card)
        .bg(token::card)
        .border_1()
        .border_color(token::border)
        .flex_row()
        .items_center()
        .child(open_trigger)
        .child(series_action(series_id, title))
}

fn poster_thumb(cover_key: String) -> impl IntoElement {
    poster::poster_thumb(cover_key, 52.0, 70.0, IconKind::Play, 26.0, 12.0)
}

fn series_meta(title: String, stats: String, episodes_preview: String) -> impl IntoElement {
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
                .child(badge("离线就绪", BadgeTone::Success))
                .child(
                    text(stats)
                        .text_xs()
                        .tabular_nums()
                        .text_color(token::muted_fg)
                        .truncate(),
                ),
        )
        .child(
            text(episodes_preview)
                .text_xs()
                .text_color(token::muted_fg)
                .truncate(),
        )
}
