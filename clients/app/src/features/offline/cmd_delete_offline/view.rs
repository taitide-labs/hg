use std::collections::BTreeSet;
use std::num::NonZeroU32;

use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId};
use hg_contracts::offline::OfflineSeriesView;
use nsh_core::prelude::*;

use super::super::store;
use super::command::{
    delete_episode_a11y_label, delete_series_a11y_label, episode_segment_ranges,
    manage_episodes_a11y_label, play_episode_a11y_label, undownloaded_episode_a11y_label,
};
use super::handler;
use crate::ui::button::{
    ButtonVariant, ControlSize, app_button, app_icon_button, episode_tile_button, text_link_button,
};
use crate::ui::controls::confirm_destructive;
use crate::ui::icons::IconKind;
use crate::ui::platform::is_desktop;
use crate::ui::surface::separator;
use crate::ui::token;

const MODAL_SEGMENT_SIZE: u32 = 50;

pub fn delete_series_button(series_id: SeriesId, title: &str) -> impl IntoElement + use<> {
    let series_a11y = delete_series_a11y_label(title);
    let manage_a11y = manage_episodes_a11y_label(title);
    let delete_desc = format!("确定要删除《{title}》的全部离线分集吗？删除后将无法离线观看。");

    view()
        .pr_3()
        .flex_row()
        .items_center()
        .gap_1p5()
        .child(text_link_button(
            "分集",
            manage_a11y,
            Some(IconKind::List),
            move || store::open_episodes_modal(series_id),
        ))
        .child(text_link_button(
            "删除",
            series_a11y,
            Some(IconKind::Close),
            move || {
                confirm_destructive(
                    "删除离线剧目",
                    delete_desc.clone(),
                    "删除",
                    move || {
                        handler::delete_series(series_id);
                    },
                );
            },
        ))
}

pub fn offline_episodes_drawer(
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
) -> impl IntoElement {
    let open = computed(|| store::state().with(|s| s.episodes_modal_series.is_some()));

    when(
        move || open.get(),
        move || {
            on_key("escape", |_| store::close_episodes_modal());
            on_back_press(|| {
                store::close_episodes_modal();
                true
            });
            drawer_overlay(on_open_series)
        },
    )
}

fn drawer_overlay(
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
) -> impl IntoElement {
    if is_desktop() {
        view()
            .absolute()
            .inset_0()
            .flex_row()
            .child(scrim_dismiss())
            .child(desktop_drawer_panel(on_open_series))
            .into_element()
    } else {
        view()
            .absolute()
            .inset_0()
            .child(scrim_dismiss())
            .child(mobile_drawer_sheet(on_open_series))
            .into_element()
    }
}

fn scrim_dismiss() -> impl IntoElement {
    view()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .bg(token::cinema_scrim)
        .on_press(store::close_episodes_modal)
}

fn desktop_drawer_panel(
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
) -> impl IntoElement {
    let enter = drawer_motion(true);
    view()
        .bind_motion(&enter)
        .w(px(440.0))
        .h_full()
        .bg(token::bg)
        .border_l_1()
        .border_color(token::border)
        .child(dynamic(move || drawer_inner(on_open_series)))
}

fn mobile_drawer_sheet(
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
) -> impl IntoElement {
    let enter = drawer_motion(false);
    view()
        .bind_motion(&enter)
        .w_full()
        .h(px(500.0))
        .bg(token::bg)
        .border_t_1()
        .border_color(token::border)
        .rounded_t(token::radius_card)
        .overflow_hidden()
        .child(dynamic(move || drawer_inner(on_open_series)))
}

fn drawer_motion(desktop: bool) -> Motion {
    if reduce_motion() {
        return token::fade_enter();
    }
    let preset = if desktop {
        MotionPreset::FadeInRight
    } else {
        MotionPreset::FadeInUp
    };
    motion().entering(preset.spec().timing(|mut t| {
        t.duration_ms = token::BASE_MS;
        t
    }))
}

fn drawer_inner(
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
) -> impl IntoElement {
    let snap = store::state().get();
    let Some(series_id) = snap.episodes_modal_series else {
        return view().into_element();
    };
    let Some(series) = snap
        .library
        .into_iter()
        .find(|item| item.series_id == series_id)
    else {
        return view().into_element();
    };

    let max_downloaded = series
        .episodes
        .iter()
        .map(|ep| ep.ep_num.get())
        .max()
        .unwrap_or(0);
    let total_episodes = series.total_episodes.max(max_downloaded);
    let downloaded_set: BTreeSet<u32> = series.episodes.iter().map(|ep| ep.ep_num.get()).collect();
    let delete_mode = snap.episodes_modal_delete_mode;

    view()
        .flex_1()
        .min_h_0()
        .child(drawer_header(
            &series,
            total_episodes,
            downloaded_set.len(),
            delete_mode,
        ))
        .child(
            scroll().flex_1().min_h_0().child(
                view()
                    .p_4()
                    .gap_3()
                    .child(legend_row(delete_mode))
                    .child(segment_and_grid(
                        &series,
                        total_episodes,
                        &downloaded_set,
                        snap.episodes_modal_segment,
                        delete_mode,
                        on_open_series,
                    )),
            ),
        )
        .into_element()
}

fn drawer_header(
    series: &OfflineSeriesView,
    total_episodes: u32,
    downloaded_count: usize,
    delete_mode: bool,
) -> impl IntoElement {
    let mode_label = if delete_mode {
        "完成"
    } else {
        "删除分集"
    };
    let mode_variant = if delete_mode {
        ButtonVariant::Primary
    } else {
        ButtonVariant::Outline
    };
    let subtitle = format!("共 {total_episodes} 集 · 已下载 {downloaded_count} 集");

    view()
        .w_full()
        .px_4()
        .py_3()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .border_b_1()
        .border_color(token::border)
        .child(
            view()
                .flex_1()
                .min_w_0()
                .gap_0p5()
                .child(
                    text(series.title.clone())
                        .role(AccessibilityRole::Header)
                        .text_base()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(token::fg)
                        .truncate(),
                )
                .child(
                    text(subtitle)
                        .text_xs()
                        .tabular_nums()
                        .text_color(token::muted_fg),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_1p5()
                .child(app_button(
                    mode_label,
                    mode_variant,
                    ControlSize::Sm,
                    store::toggle_episodes_modal_delete_mode,
                ))
                .child(app_icon_button(
                    IconKind::Close,
                    "关闭",
                    ButtonVariant::Ghost,
                    ControlSize::Sm,
                    store::close_episodes_modal,
                )),
        )
}

fn legend_row(delete_mode: bool) -> impl IntoElement {
    let tip = if delete_mode {
        "点击高亮分集确认删除本地缓存"
    } else {
        "点击高亮分集直接离线播放"
    };
    view()
        .flex_row()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap_2()
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_3()
                .child(legend_chip(true, "已下载"))
                .child(legend_chip(false, "未下载")),
        )
        .child(text(tip).text_xs().text_color(token::muted_fg))
}

fn legend_chip(downloaded: bool, label: &'static str) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap_1p5()
        .child(view().size(px(10.0)).rounded_full().bg(move || {
            if downloaded {
                token::primary()
            } else {
                token::border()
            }
        }))
        .child(text(label).text_xs().text_color(token::muted_fg))
}

fn segment_and_grid(
    series: &OfflineSeriesView,
    total_episodes: u32,
    downloaded_set: &BTreeSet<u32>,
    segment_index: usize,
    delete_mode: bool,
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
) -> impl IntoElement {
    let ranges = episode_segment_ranges(total_episodes, MODAL_SEGMENT_SIZE);
    let active_idx = segment_index.min(ranges.len().saturating_sub(1));
    let (start, end) = ranges
        .get(active_idx)
        .copied()
        .unwrap_or((1, total_episodes));

    let mut seg_row = view().flex_row().flex_wrap().gap_1p5();
    if ranges.len() > 1 {
        for (idx, (s, e)) in ranges.iter().enumerate() {
            let variant = if idx == active_idx {
                ButtonVariant::Primary
            } else {
                ButtonVariant::Ghost
            };
            seg_row = seg_row.child(app_button(
                format!("{s}-{e}"),
                variant,
                ControlSize::Sm,
                move || store::set_episodes_modal_segment(idx),
            ));
        }
    }

    let mut grid = view().flex_row().flex_wrap().gap_2();
    for num in start..=end {
        let is_downloaded = downloaded_set.contains(&num);
        grid = grid.child(episode_modal_tile(
            series.series_id,
            &series.title,
            num,
            is_downloaded,
            delete_mode,
            on_open_series,
        ));
    }

    view()
        .gap_3()
        .children((ranges.len() > 1).then_some(seg_row))
        .children((ranges.len() > 1).then(separator))
        .child(grid)
}

fn episode_modal_tile(
    series_id: SeriesId,
    title: &str,
    num: u32,
    downloaded: bool,
    delete_mode: bool,
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
) -> impl IntoElement {
    if !downloaded {
        return view()
            .label(undownloaded_episode_a11y_label(title, num))
            .w(px(64.0))
            .h(px(30.0))
            .rounded(token::radius_control)
            .bg(token::muted)
            .border_1()
            .border_color(token::border)
            .items_center()
            .justify_center()
            .opacity(0.55)
            .child(
                text(format!("{num}"))
                    .text_xs()
                    .tabular_nums()
                    .text_color(token::muted_fg),
            )
            .into_element();
    }

    let ep_num = EpisodeNumber::new(NonZeroU32::new(num).unwrap_or(NonZeroU32::MIN));
    if delete_mode {
        let key = EpisodeKey::new(series_id, ep_num);
        let a11y = delete_episode_a11y_label(title, ep_num);
        let desc = format!("确定要删除《{title}》第 {num} 集的离线缓存吗？");
        episode_tile_button(format!("{num}"), a11y, ButtonVariant::Danger, move || {
            confirm_destructive("删除离线分集", desc.clone(), "删除", move || {
                handler::delete_episode(key);
            });
        })
        .into_element()
    } else {
        let a11y = play_episode_a11y_label(title, ep_num);
        episode_tile_button(
            format!("{num}"),
            a11y,
            ButtonVariant::BrandSoft,
            move || {
                store::close_episodes_modal();
                on_open_series(series_id, Some(ep_num));
            },
        )
        .into_element()
    }
}
