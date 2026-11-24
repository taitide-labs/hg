use hg_contracts::ids::{EpisodeNumber, SeriesId};
use hg_contracts::viewing::{PlaybackMode, PlaybackOrigin};
use nsh_core::prelude::*;

use super::super::store::{self, CompanionTab, PlayerSurfaceKey};
use super::command::{AspectPreset, format_time, origin_badge_label};
use super::handler;
use crate::ui::button::{ButtonVariant, ControlSize, app_button, app_icon_button};
use crate::ui::icons::IconKind;
use crate::ui::platform::is_desktop;
use crate::ui::surface::{BadgeTone, badge, cinema_separator, empty_state};
use crate::ui::tabs::cinema_tabs;
use crate::ui::token;

pub fn player_view(
    series_id: SeriesId,
    initial_episode: Option<EpisodeNumber>,
    on_back: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let default_mode = if is_desktop() {
        PlaybackMode::Cinema
    } else {
        PlaybackMode::ShortDrama
    };
    handler::open_series(series_id, initial_episode, default_mode);
    bind_potplayer_shortcuts(on_back);

    let key = computed(store::surface_key);
    view()
        .w_full()
        .flex_1()
        .min_h_0()
        .overflow_hidden()
        .bg(token::cinema_bg)
        .child(dynamic(move || match key.get() {
            PlayerSurfaceKey::Error => empty_state(
                IconKind::Play,
                "播放加载失败",
                "请检查网络或返回片库重试",
                Some(("返回片库", Box::new(on_back))),
            )
            .into_element(),
            PlayerSurfaceKey::Loading => view()
                .flex_1()
                .items_center()
                .justify_center()
                .child(
                    text("正在准备播放源...")
                        .text_sm()
                        .text_color(token::cinema_muted),
                )
                .into_element(),
            PlayerSurfaceKey::Cinema { fullscreen } => {
                cinema_layout(fullscreen, on_back).into_element()
            }
            PlayerSurfaceKey::ShortDrama => short_drama_layout(on_back).into_element(),
        }))
}

fn bind_potplayer_shortcuts(on_back: impl Fn() + Copy + 'static) {
    on_key("space", |_| store::toggle_paused());
    on_key("enter", |_| store::toggle_fullscreen());
    for digit in 1..=4_u8 {
        if let Some(preset) = AspectPreset::from_digit(digit) {
            let key_str = match digit {
                1 => "1",
                2 => "2",
                3 => "3",
                _ => "4",
            };
            on_key(key_str, move |_| store::set_aspect(preset));
        }
    }
    on_key("left", |_| handler::seek_by(-5));
    on_key("right", |_| handler::seek_by(5));
    on_key("up", |_| handler::adjust_volume_or_episode(true));
    on_key("down", |_| handler::adjust_volume_or_episode(false));
    on_key("m", |_| store::toggle_muted());
    on_key("z", |_| handler::reset_rate());
    on_key("x", |_| handler::adjust_rate(false));
    on_key("c", |_| handler::adjust_rate(true));
    on_key("escape", move |_| {
        if store::snapshot().fullscreen {
            store::toggle_fullscreen();
        } else {
            on_back();
        }
    });
}

fn cinema_layout(fullscreen: bool, on_back: impl Fn() + Copy + 'static) -> impl IntoElement {
    let enter = token::fade_enter();
    view()
        .bind_motion(&enter)
        .w_full()
        .flex_1()
        .min_h_0()
        .flex_row()
        .bg(token::cinema_bg)
        .child(
            view()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .overflow_hidden()
                .p_4()
                .gap_3()
                .child(stage_header(on_back))
                .child(cinema_stage_canvas())
                .child(cinema_transport_bar()),
        )
        .children((!fullscreen).then(companion_sidebar))
}

fn stage_header(on_back: impl Fn() + Copy + 'static) -> impl IntoElement {
    view()
        .flex_shrink_0()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .children((!is_desktop()).then(|| {
                    app_button("返回", ButtonVariant::CinemaGlass, ControlSize::Sm, on_back)
                }))
                .child(
                    text(move || {
                        store::snapshot()
                            .detail
                            .map_or_else(String::new, |d| d.summary.title)
                    })
                    .text_base()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(token::cinema_fg),
                )
                .child(
                    text(move || {
                        let ep = store::snapshot()
                            .current_episode
                            .map_or(1, EpisodeNumber::get);
                        format!("正在播放 · 第 {ep} 集")
                    })
                    .text_xs()
                    .text_color(token::cinema_muted),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(badge("电影模式", BadgeTone::Cinema))
                .child(dynamic(move || {
                    let origin = store::snapshot().source.map(|s| s.origin);
                    let label = origin.map_or("准备中", origin_badge_label);
                    let tone = match origin {
                        Some(PlaybackOrigin::OfflineLocal { .. }) => BadgeTone::CinemaSuccess,
                        _ => BadgeTone::Cinema,
                    };
                    badge(label, tone).into_element()
                }))
                .child(badge(
                    move || store::snapshot().aspect.label(),
                    BadgeTone::Cinema,
                )),
        )
}

fn cinema_stage_canvas() -> impl IntoElement {
    view()
        .flex_1()
        .min_h_0()
        .rounded(token::radius_card)
        .bg(token::cinema_surface)
        .border_1()
        .border_color(token::cinema_border)
        .items_center()
        .justify_center()
        .p_5()
        .gap_3()
        .child(cinema_video_frame())
        .child(
            text("快捷键：Space 播停 · Enter 全屏 · 1/2/3/4 比例 · ←/→ 快进 · Z/X/C 倍速 · M 静音")
                .text_xs()
                .text_color(token::cinema_muted),
        )
}

fn cinema_video_frame() -> impl IntoElement {
    view()
        .w(move || match store::snapshot().aspect {
            AspectPreset::Portrait9x16 => px(240.0),
            AspectPreset::Fill => px(640.0),
            AspectPreset::Original | AspectPreset::Cinema16x9 => px(560.0),
        })
        .h(px(320.0))
        .rounded(token::radius)
        .bg(move || {
            let cover = store::snapshot()
                .detail
                .map_or_else(String::new, |d| d.summary.cover_url);
            token::poster_surface(&cover)
        })
        .p_4()
        .justify_between()
        .child(
            view()
                .flex_row()
                .justify_between()
                .child(badge(
                    move || {
                        if store::snapshot().paused {
                            "已暂停"
                        } else {
                            "播放中"
                        }
                    },
                    BadgeTone::CinemaBrand,
                ))
                .child(
                    text(move || store::snapshot().status_hint.unwrap_or_default())
                        .text_xs()
                        .text_color(token::cinema_fg),
                ),
        )
        .child(
            view()
                .gap_1()
                .child(
                    text(move || {
                        let snap = store::snapshot();
                        let title = snap.detail.map_or_else(String::new, |d| d.summary.title);
                        let ep = snap.current_episode.map_or(1, EpisodeNumber::get);
                        format!("{title} · 第 {ep} 集")
                    })
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(token::cinema_fg),
                )
                .child(
                    text(move || {
                        let snap = store::snapshot();
                        format!(
                            "画面：{} · 倍速 {:.2}× · 音量 {}%",
                            snap.aspect.label(),
                            f32::from(snap.rate_hundredths) / 100.0,
                            if snap.muted { 0 } else { snap.volume_percent }
                        )
                    })
                    .text_xs()
                    .text_color(token::cinema_fg),
                ),
        )
}

fn cinema_transport_bar() -> impl IntoElement {
    view()
        .flex_shrink_0()
        .p_3()
        .rounded(token::radius_card)
        .bg(token::cinema_surface)
        .border_1()
        .border_color(token::cinema_border)
        .gap_2p5()
        .child(transport_primary_row())
        .child(transport_secondary_row())
}

fn transport_primary_row() -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_2()
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(app_button(
                    move || {
                        if store::snapshot().paused {
                            "播放"
                        } else {
                            "暂停"
                        }
                    },
                    ButtonVariant::CinemaPrimary,
                    ControlSize::Sm,
                    store::toggle_paused,
                ))
                .child(app_button(
                    "上一集",
                    ButtonVariant::CinemaGlass,
                    ControlSize::Sm,
                    || handler::step_episode(false),
                ))
                .child(app_button(
                    "下一集",
                    ButtonVariant::CinemaGlass,
                    ControlSize::Sm,
                    || handler::step_episode(true),
                ))
                .child(app_button(
                    "完播并连播",
                    ButtonVariant::CinemaGlass,
                    ControlSize::Sm,
                    handler::complete_current_and_advance,
                ))
                .child(
                    text(move || {
                        let snap = store::snapshot();
                        format!(
                            "{} / {}",
                            format_time(snap.position_secs),
                            format_time(snap.duration_secs)
                        )
                    })
                    .text_xs()
                    .text_color(token::cinema_muted),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_1p5()
                .child(app_button(
                    "切至短剧模式",
                    ButtonVariant::CinemaGlass,
                    ControlSize::Sm,
                    || store::set_mode(PlaybackMode::ShortDrama),
                ))
                .child(app_icon_button(
                    IconKind::Volume,
                    "静音切换",
                    ButtonVariant::CinemaGlass,
                    ControlSize::Sm,
                    store::toggle_muted,
                ))
                .child(app_icon_button(
                    IconKind::Expand,
                    "全屏切换",
                    ButtonVariant::CinemaGlass,
                    ControlSize::Sm,
                    store::toggle_fullscreen,
                ))
                .child(app_icon_button(
                    IconKind::ExternalLink,
                    "外部播放器打开",
                    ButtonVariant::CinemaGlass,
                    ControlSize::Sm,
                    handler::launch_external_player_hint,
                )),
        )
}

fn transport_secondary_row() -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_2()
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_1p5()
                .child(aspect_button(AspectPreset::Original))
                .child(aspect_button(AspectPreset::Fill))
                .child(aspect_button(AspectPreset::Cinema16x9))
                .child(aspect_button(AspectPreset::Portrait9x16)),
        )
        .child(app_button(
            move || {
                format!(
                    "{:.2}×",
                    f32::from(store::snapshot().rate_hundredths) / 100.0
                )
            },
            ButtonVariant::CinemaGlass,
            ControlSize::Sm,
            || handler::adjust_rate(true),
        ))
}

fn aspect_button(preset: AspectPreset) -> impl IntoElement {
    app_button(
        preset.key_hint(),
        ButtonVariant::CinemaGlass,
        ControlSize::Sm,
        move || store::set_aspect(preset),
    )
}

fn companion_sidebar() -> impl IntoElement {
    let active_tab = store::companion_tab_signal();
    let tabs = vec![
        (CompanionTab::Episodes, "选集"),
        (CompanionTab::Info, "简介"),
        (CompanionTab::Comments, "评论"),
    ];

    view()
        .w(px(320.0))
        .flex_shrink_0()
        .h_full()
        .bg(token::cinema_surface)
        .border_l_1()
        .border_color(token::cinema_border)
        .p_4()
        .gap_3()
        .child(cinema_tabs(tabs, active_tab, move |next| {
            active_tab.set(next);
        }))
        .child(cinema_separator())
        .child(
            scroll()
                .flex_1()
                .child(dynamic(move || match active_tab.get() {
                    CompanionTab::Episodes => episode_grid().into_element(),
                    CompanionTab::Info => series_info_panel().into_element(),
                    CompanionTab::Comments => comments_panel().into_element(),
                })),
        )
}

fn episode_grid() -> impl IntoElement {
    let snap = store::snapshot();
    let Some(detail) = snap.detail else {
        return view();
    };
    let current_ep = snap.current_episode;
    let mut grid = view().flex_row().flex_wrap().gap_2();
    for ep in detail.episodes.iter().take(24) {
        let ep_num = ep.ep_num;
        let variant = if current_ep == Some(ep_num) {
            ButtonVariant::CinemaPrimary
        } else {
            ButtonVariant::CinemaGlass
        };
        grid = grid.child(view().w(px(64.0)).child(app_button(
            format!("第 {} 集", ep_num.get()),
            variant,
            ControlSize::Sm,
            move || handler::switch_episode(ep_num),
        )));
    }
    grid
}

fn series_info_panel() -> impl IntoElement {
    let Some(detail) = store::snapshot().detail else {
        return view();
    };
    view()
        .gap_3()
        .child(
            text(detail.summary.title)
                .text_base()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(token::cinema_fg),
        )
        .child(
            text(format!(
                "评分 {:.1} · 全 {} 集 · 热度 {}",
                detail.summary.score, detail.total_episodes, detail.stats.play_heat
            ))
            .text_xs()
            .text_color(token::cinema_muted),
        )
        .child(
            text(detail.summary.description)
                .text_sm()
                .text_color(token::cinema_fg),
        )
}

fn comments_panel() -> impl IntoElement {
    let comments = store::snapshot().comments;
    if comments.is_empty() {
        return view()
            .py_6()
            .items_center()
            .child(
                text("当前分集暂无评论")
                    .text_sm()
                    .text_color(token::cinema_muted),
            )
            .into_element();
    }
    let mut list = view().gap_3();
    for comment in comments {
        list = list.child(
            view()
                .p_3()
                .rounded(token::radius)
                .bg(token::cinema_bg)
                .border_1()
                .border_color(token::cinema_border)
                .gap_1p5()
                .child(
                    view()
                        .flex_row()
                        .justify_between()
                        .child(
                            text(comment.nickname)
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(token::cinema_fg),
                        )
                        .child(
                            text(comment.relative_time)
                                .text_xs()
                                .text_color(token::cinema_muted),
                        ),
                )
                .child(text(comment.text).text_sm().text_color(token::cinema_fg)),
        );
    }
    list.into_element()
}

fn short_drama_layout(on_back: impl Fn() + Copy + 'static) -> impl IntoElement {
    let enter = token::fade_enter();
    view()
        .bind_motion(&enter)
        .w_full()
        .flex_1()
        .bg(token::cinema_bg)
        .p_5()
        .items_center()
        .justify_center()
        .child(
            view()
                .flex_row()
                .items_end()
                .gap_5()
                .child(short_drama_stage(on_back))
                .child(short_drama_rail()),
        )
}

fn short_drama_stage(on_back: impl Fn() + Copy + 'static) -> impl IntoElement {
    view()
        .w(px(360.0))
        .h(px(620.0))
        .rounded(token::radius_card)
        .bg(move || {
            let cover = store::snapshot()
                .detail
                .map_or_else(String::new, |d| d.summary.cover_url);
            token::poster_surface(&cover)
        })
        .border_1()
        .border_color(token::cinema_border)
        .p_4()
        .justify_between()
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .child(view().children((!is_desktop()).then(|| {
                    app_button("返回", ButtonVariant::CinemaGlass, ControlSize::Sm, on_back)
                })))
                .child(badge(
                    move || {
                        store::snapshot()
                            .source
                            .map_or("准备中", |s| origin_badge_label(s.origin))
                    },
                    BadgeTone::Cinema,
                )),
        )
        .child(
            view()
                .p_3()
                .rounded(token::radius)
                .bg(token::cinema_scrim)
                .gap_1()
                .child(
                    text(move || {
                        store::snapshot()
                            .detail
                            .map_or_else(String::new, |d| d.summary.title)
                    })
                    .text_base()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(token::cinema_fg),
                )
                .child(
                    text(move || {
                        let snap = store::snapshot();
                        let ep = snap.current_episode.map_or(1, EpisodeNumber::get);
                        let total = snap.detail.map_or(0, |d| d.total_episodes);
                        format!("正在播放 · 第 {ep} 集 / 共 {total} 集")
                    })
                    .text_xs()
                    .text_color(token::cinema_muted),
                ),
        )
}

fn short_drama_rail() -> impl IntoElement {
    view()
        .gap_2()
        .child(app_icon_button(
            IconKind::ChevronUp,
            "上一集",
            ButtonVariant::CinemaGlass,
            ControlSize::Lg,
            || handler::step_episode(false),
        ))
        .child(app_icon_button(
            IconKind::ChevronDown,
            "下一集",
            ButtonVariant::CinemaGlass,
            ControlSize::Lg,
            || handler::step_episode(true),
        ))
        .child(app_button(
            move || {
                if store::snapshot().paused {
                    "播放"
                } else {
                    "暂停"
                }
            },
            ButtonVariant::CinemaPrimary,
            ControlSize::Md,
            store::toggle_paused,
        ))
        .child(app_button(
            "完播并连播",
            ButtonVariant::CinemaGlass,
            ControlSize::Md,
            handler::complete_current_and_advance,
        ))
        .child(app_button(
            "切至电影模式",
            ButtonVariant::CinemaGlass,
            ControlSize::Md,
            || store::set_mode(PlaybackMode::Cinema),
        ))
}
