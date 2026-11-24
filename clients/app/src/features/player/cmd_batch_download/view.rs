use hg_contracts::catalog::{EpisodeSummary, SeriesDetail};
use nsh_core::prelude::*;

use super::super::store;
use super::command::{
    BATCH_SEGMENT_SIZE, EpisodeAvailability, availability_for, episode_segment_ranges,
    is_segment_fully_selected, selectable_episodes_in_range,
};
use super::handler::{self, BatchDownloadState};
use crate::ui::button::{ButtonVariant, ControlSize, app_button, app_icon_button};
use crate::ui::icons::IconKind;
use crate::ui::platform::is_desktop;
use crate::ui::surface::{empty_state, separator};
use crate::ui::token;

pub fn batch_download_drawer() -> impl IntoElement {
    handler::reset_state();
    effect(handler::sync_from_player_store);

    when(handler::is_open, move || {
        on_key("escape", |_| handler::close());
        drawer_overlay()
    })
}

fn drawer_overlay() -> impl IntoElement {
    if is_desktop() {
        view()
            .absolute()
            .inset_0()
            .flex_row()
            .child(scrim_dismiss())
            .child(desktop_drawer_panel())
            .into_element()
    } else {
        view()
            .absolute()
            .inset_0()
            .child(scrim_dismiss())
            .child(mobile_drawer_sheet())
            .into_element()
    }
}

fn scrim_dismiss() -> impl IntoElement {
    view()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .bg(token::cinema_scrim)
        .on_press(handler::close)
}

fn desktop_drawer_panel() -> impl IntoElement {
    let enter = drawer_motion(true);
    view()
        .bind_motion(&enter)
        .w(px(440.0))
        .h_full()
        .bg(token::bg)
        .border_l_1()
        .border_color(token::border)
        .child(drawer_header())
        .child(drawer_content())
        .child(drawer_footer())
}

fn mobile_drawer_sheet() -> impl IntoElement {
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
        .child(drawer_header())
        .child(drawer_content())
        .child(drawer_footer())
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

fn drawer_header() -> impl IntoElement {
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
                .flex_row()
                .items_baseline()
                .gap_2()
                .child(
                    text("选择离线下载分集")
                        .role(AccessibilityRole::Header)
                        .text_base()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(token::fg),
                )
                .child(
                    text(move || format!("已选 {} 集", handler::snapshot().selected.len()))
                        .text_xs()
                        .tabular_nums()
                        .text_color(token::muted_fg),
                ),
        )
        .child(app_icon_button(
            IconKind::Close,
            "关闭",
            ButtonVariant::Ghost,
            ControlSize::Sm,
            handler::close,
        ))
}

fn drawer_content() -> impl IntoElement {
    scroll()
        .flex_1()
        .min_h_0()
        .child(view().p_4().gap_4().child(dynamic(drawer_body)))
}

fn drawer_body() -> impl IntoElement {
    let Some(detail) = store::snapshot().detail else {
        return empty_state(
            IconKind::Download,
            "暂无可选分集",
            "请先加载剧目分集列表",
            None,
        )
        .into_element();
    };
    let snap = handler::snapshot();
    if detail.episodes.is_empty() {
        return empty_state(
            IconKind::Download,
            "暂无分集",
            "该剧目暂未提供可离线下载的分集",
            None,
        )
        .into_element();
    }

    let ranges = episode_segment_ranges(detail.total_episodes, BATCH_SEGMENT_SIZE);
    let active_idx = snap.segment_index.min(ranges.len().saturating_sub(1));
    let active_range = ranges
        .get(active_idx)
        .copied()
        .unwrap_or((1, detail.total_episodes));

    view()
        .gap_3()
        .child(segment_toolbar(
            &detail,
            &snap,
            &ranges,
            active_idx,
            active_range,
        ))
        .child(separator())
        .child(episode_grid(&detail.episodes, active_range, &snap))
        .into_element()
}

fn segment_toolbar(
    detail: &SeriesDetail,
    snap: &BatchDownloadState,
    ranges: &[(u32, u32)],
    active_idx: usize,
    active_range: (u32, u32),
) -> impl IntoElement {
    let selectable = selectable_episodes_in_range(&detail.episodes, active_range, &snap.existing);
    let all_selected = is_segment_fully_selected(&snap.selected, &selectable);
    let select_label = if all_selected {
        "取消全选当前段"
    } else {
        "全选当前段"
    };

    let mut seg_row = view().flex_1().min_w_0().flex_row().flex_wrap().gap_1p5();
    for (idx, (start, end)) in ranges.iter().enumerate() {
        let variant = if idx == active_idx {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Ghost
        };
        seg_row = seg_row.child(app_button(
            format!("{start}-{end}"),
            variant,
            ControlSize::Sm,
            move || handler::set_segment(idx),
        ));
    }

    view()
        .gap_2p5()
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .gap_2()
                .child(seg_row)
                .child(app_button(
                    select_label,
                    ButtonVariant::Outline,
                    ControlSize::Sm,
                    handler::toggle_select_current_segment,
                )),
        )
        .child(
            text("自动跳过已离线或已在下载队列中的分集")
                .text_xs()
                .text_color(token::muted_fg),
        )
}

fn episode_grid(
    episodes: &[EpisodeSummary],
    range: (u32, u32),
    snap: &BatchDownloadState,
) -> impl IntoElement {
    let mut grid = view().flex_row().flex_wrap().gap_2();
    for ep in episodes {
        let num = ep.ep_num.get();
        if num < range.0 || num > range.1 {
            continue;
        }
        let availability = availability_for(num, &snap.existing);
        let selected = snap.selected.contains(&num);
        grid = grid.child(episode_tile(num, ep.playable, availability, selected));
    }
    grid
}

fn episode_tile(
    num: u32,
    playable: bool,
    availability: EpisodeAvailability,
    selected: bool,
) -> impl IntoElement {
    if !playable || availability != EpisodeAvailability::Available {
        let status_label = match availability {
            EpisodeAvailability::AlreadyOffline => format!("{num} 已离线"),
            EpisodeAvailability::AlreadyQueued => format!("{num} 队列中"),
            EpisodeAvailability::Available => format!("{num} 不可用"),
        };
        return view()
            .w(px(92.0))
            .h(px(30.0))
            .px_2()
            .rounded(token::radius_control)
            .bg(token::muted)
            .border_1()
            .border_color(token::border)
            .items_center()
            .justify_center()
            .opacity(0.65)
            .child(
                text(status_label)
                    .text_xs()
                    .tabular_nums()
                    .text_color(token::muted_fg),
            )
            .into_element();
    }

    let variant = if selected {
        ButtonVariant::Primary
    } else {
        ButtonVariant::Secondary
    };
    view()
        .w(px(92.0))
        .child(app_button(
            format!("第 {num} 集"),
            variant,
            ControlSize::Sm,
            move || handler::toggle_episode(num),
        ))
        .into_element()
}

fn drawer_footer() -> impl IntoElement {
    view()
        .w_full()
        .px_4()
        .py_3()
        .border_t_1()
        .border_color(token::border)
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .child(
            text(move || {
                let snap = handler::snapshot();
                if let Some(err) = snap.error {
                    err
                } else {
                    format!("已选 {} 集", snap.selected.len())
                }
            })
            .text_xs()
            .tabular_nums()
            .text_color(move || {
                if handler::snapshot().error.is_some() {
                    token::destructive()
                } else {
                    token::muted_fg()
                }
            }),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(app_button(
                    "取消",
                    ButtonVariant::Ghost,
                    ControlSize::Md,
                    handler::close,
                ))
                .child(app_button(
                    move || {
                        if handler::snapshot().submitting {
                            "加入中..."
                        } else {
                            "加入离线队列"
                        }
                    },
                    ButtonVariant::Primary,
                    ControlSize::Md,
                    handler::submit_downloads,
                )),
        )
}
