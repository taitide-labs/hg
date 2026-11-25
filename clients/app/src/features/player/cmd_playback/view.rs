use hg_contracts::ids::{EpisodeNumber, SeriesId};
use hg_contracts::viewing::PlaybackMode;
use nsh_core::prelude::*;

use super::super::store::{self, PlayerSurfaceKey};
use super::command::{AspectPreset, format_time};
use super::feed::{desktop_action_row, desktop_info_feed, mobile_detail_sheet};
use super::handler;
use super::overlay::{current_video_surface_state, video_surface_box};
use crate::ui::button::{ButtonVariant, ControlSize, app_icon_button};
use crate::ui::icons::IconKind;
use crate::ui::platform::is_desktop;
use crate::ui::surface::empty_state;
use crate::ui::token;
use crate::ui::video_surface::{bind_lifecycle_pause, scrub_slider, video_surface};

pub fn player_view(
    series_id: SeriesId,
    initial_episode: Option<EpisodeNumber>,
    on_back: impl Fn() + Copy + 'static,
    on_fullscreen_change: impl Fn(bool) + Copy + 'static,
) -> impl IntoElement {
    let default_mode = if is_desktop() {
        PlaybackMode::Cinema
    } else {
        PlaybackMode::ShortDrama
    };
    handler::open_series(series_id, initial_episode, default_mode);
    cleanup(handler::cleanup_on_exit);
    bind_lifecycle_pause(handler::pause_on_background);

    let toggle_fs = move || {
        let next = store::toggle_fullscreen();
        on_fullscreen_change(next);
    };
    bind_potplayer_shortcuts(on_back, toggle_fs);
    on_back_press(move || {
        if store::is_fullscreen_untracked() {
            toggle_fs();
        } else {
            on_back();
        }
        true
    });

    let key = computed(store::surface_key);
    view()
        .w_full()
        .flex_1()
        .min_h_0()
        .overflow_hidden()
        .bg(token::bg)
        .child(dynamic(move || match key.get() {
            PlayerSurfaceKey::Error => empty_state(
                IconKind::Play,
                "播放加载失败",
                "请检查网络或返回重试",
                Some(("返回", Box::new(on_back))),
            )
            .into_element(),
            PlayerSurfaceKey::Loading => view()
                .flex_1()
                .items_center()
                .justify_center()
                .child(text("加载中...").text_sm().text_color(token::muted_fg))
                .into_element(),
            PlayerSurfaceKey::Regular { fullscreen } => {
                regular_layout(fullscreen, on_back, toggle_fs).into_element()
            }
            PlayerSurfaceKey::VerticalImmersive => {
                vertical_immersive_layout(on_back).into_element()
            }
        }))
}

fn bind_potplayer_shortcuts(
    on_back: impl Fn() + Copy + 'static,
    on_toggle_fullscreen: impl Fn() + Copy + 'static,
) {
    on_key("space", |_| store::toggle_paused());
    on_key("enter", move |_| on_toggle_fullscreen());
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
        if store::is_fullscreen_untracked() {
            on_toggle_fullscreen();
        } else {
            on_back();
        }
    });
}

fn regular_layout(
    fullscreen: bool,
    on_back: impl Fn() + Copy + 'static,
    on_toggle_fullscreen: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let enter = token::fade_enter();
    if fullscreen {
        return view()
            .bind_motion(&enter)
            .w_full()
            .flex_1()
            .min_h_0()
            .bg(token::cinema_bg)
            .child(video_surface_box(true, on_back, on_toggle_fullscreen))
            .into_element();
    }

    if is_desktop() {
        view()
            .bind_motion(&enter)
            .w_full()
            .flex_1()
            .min_h_0()
            .flex_row()
            .bg(token::bg)
            .child(
                view()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .p_4()
                    .gap_3()
                    .child(video_surface_box(false, on_back, on_toggle_fullscreen))
                    .child(desktop_action_row()),
            )
            .child(desktop_info_feed())
            .into_element()
    } else {
        view()
            .bind_motion(&enter)
            .w_full()
            .flex_1()
            .min_h_0()
            .bg(token::bg)
            .child(
                view()
                    .w_full()
                    .h(px(240.0))
                    .flex_shrink_0()
                    .child(video_surface_box(true, on_back, on_toggle_fullscreen)),
            )
            .child(mobile_detail_sheet())
            .into_element()
    }
}

fn vertical_immersive_layout(on_back: impl Fn() + Copy + 'static) -> impl IntoElement {
    let enter = token::fade_enter();
    view()
        .bind_motion(&enter)
        .w_full()
        .flex_1()
        .min_h_0()
        .relative()
        .bg(token::cinema_bg)
        .items_center()
        .justify_center()
        .child(vertical_video_canvas())
        .child(
            view()
                .absolute()
                .top_0()
                .pt_safe_area_plus(px(12.0))
                .left_4()
                .children((!is_desktop()).then(|| {
                    app_icon_button(
                        IconKind::ChevronLeft,
                        "返回",
                        ButtonVariant::CinemaGlass,
                        ControlSize::Md,
                        on_back,
                    )
                })),
        )
        .child(vertical_right_rail())
        .child(vertical_bottom_info())
}

fn vertical_video_canvas() -> impl IntoElement {
    let is_wide = is_desktop();
    let canvas = view()
        .relative()
        .overflow_hidden()
        .bg(move || {
            let cover = store::snapshot()
                .detail
                .map_or_else(String::new, |d| d.summary.cover_url);
            token::poster_surface(&cover)
        })
        .child(video_surface(
            current_video_surface_state,
            handler::on_playback_progress,
            handler::on_playback_ended,
            handler::on_playback_error,
        ));

    if is_wide {
        canvas
            .w(px(390.0))
            .h_full()
            .rounded(token::radius_card)
            .into_element()
    } else {
        canvas.size_full().into_element()
    }
}

fn vertical_right_rail() -> impl IntoElement {
    let bookmarked_sig = computed(|| store::snapshot().bookmarked);
    view()
        .absolute()
        .right_4()
        .bottom(px(88.0))
        .gap_3()
        .child(dynamic(move || {
            let bookmarked = bookmarked_sig.get();
            let (icon_kind, label, variant) = if bookmarked {
                (
                    IconKind::BookmarkFilled,
                    "已追剧",
                    ButtonVariant::CinemaBrand,
                )
            } else {
                (IconKind::Bookmark, "追剧", ButtonVariant::CinemaGlass)
            };
            app_icon_button(
                icon_kind,
                label,
                variant,
                ControlSize::Lg,
                handler::toggle_bookmark,
            )
            .into_element()
        }))
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
        .child(app_icon_button(
            IconKind::Monitor,
            "详情视图",
            ButtonVariant::CinemaGlass,
            ControlSize::Lg,
            || store::set_mode(PlaybackMode::Cinema),
        ))
}

fn vertical_bottom_info() -> impl IntoElement {
    let title_ep_sig = computed(|| {
        let snap = store::snapshot();
        let title = snap.detail.map_or_else(String::new, |d| d.summary.title);
        let ep = snap.current_episode.map_or(1, EpisodeNumber::get);
        format!("{title} · 第 {ep} 集")
    });
    view()
        .absolute()
        .left_4()
        .right_4()
        .bottom_4()
        .gap_2()
        .child(
            text(move || title_ep_sig.get())
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(token::cinema_fg),
        )
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .gap_2()
                .child(view().flex_1().child(scrub_slider(
                    || {
                        let p = store::progress_snapshot();
                        p.scrub_preview_secs.unwrap_or(p.position_secs)
                    },
                    || store::progress_snapshot().duration_secs,
                    |secs| store::set_scrub_preview(Some(secs)),
                    |secs| {
                        store::set_scrub_preview(None);
                        handler::seek_to(secs);
                    },
                )))
                .child(
                    text(move || {
                        let p = store::progress_snapshot();
                        format!(
                            "{} / {}",
                            format_time(p.scrub_preview_secs.unwrap_or(p.position_secs)),
                            format_time(p.duration_secs)
                        )
                    })
                    .text_xs()
                    .tabular_nums()
                    .text_color(token::cinema_fg),
                ),
        )
}
