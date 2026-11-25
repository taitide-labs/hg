use std::time::Duration;

use hg_contracts::viewing::PlaybackMode;
use nsh_core::prelude::*;

use super::super::store::{self, ActivePopover};
use super::command::{AspectPreset, format_rate_trigger, format_time};
use super::handler;
use crate::ui::button::{
    ButtonVariant, ControlSize, app_button, app_icon_button, app_icon_text_button,
};
use crate::ui::icons::IconKind;
use crate::ui::platform::is_desktop;
use crate::ui::surface::{cinema_separator, toggle_switch_row};
use crate::ui::token;
use crate::ui::video_surface::{
    VideoSurfaceState, bind_auto_hide_timer, scrub_slider, video_surface,
};

const CONTROLS_AUTO_HIDE_DELAY: Duration = Duration::from_secs(3);

pub(super) fn current_video_surface_state() -> VideoSurfaceState {
    let snap = store::snapshot();
    let controls = store::controls_snapshot();
    let progress = store::progress_snapshot();
    VideoSurfaceState {
        src: snap.source.map(|s| s.playable_uri),
        paused: controls.paused,
        rate: f64::from(controls.rate_hundredths) / 100.0,
        volume: f64::from(controls.volume_percent) / 100.0,
        muted: controls.muted,
        seek_secs: progress.position_secs,
        seek_seq: progress.seek_seq,
        aspect: controls.aspect.protocol_key(),
    }
}

pub fn video_surface_box(
    fullscreen: bool,
    on_back: impl Fn() + Copy + 'static,
    on_toggle_fullscreen: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    bind_auto_hide_timer(
        store::controls_activity_seq,
        store::should_keep_controls_visible,
        CONTROLS_AUTO_HIDE_DELAY,
        store::hide_controls,
    );

    let show_controls = computed(store::should_show_controls);
    let frame = view()
        .label("视频画面")
        .w_full()
        .flex_1()
        .min_h_0()
        .relative()
        .overflow_hidden()
        .bg(token::cinema_bg)
        .on_mouse_enter(|_: &nsh_core::MouseEvent| store::reveal_controls())
        .on_mouse_move(|_: &nsh_core::MouseEvent| store::reveal_controls())
        .on_mouse_leave(|_: &nsh_core::MouseEvent| store::leave_surface())
        .child(video_picture_area(on_back))
        .child(bottom_right_popover())
        .child(when(
            move || show_controls.get(),
            move || bottom_control_bar(on_toggle_fullscreen),
        ));

    if fullscreen {
        frame.into_element()
    } else {
        frame.rounded(token::radius_card).into_element()
    }
}

fn video_picture_area(on_back: impl Fn() + Copy + 'static) -> impl IntoElement {
    view()
        .size_full()
        .relative()
        .items_center()
        .justify_center()
        .child(aspect_frame_preview())
        .child(top_overlay_strip(on_back))
        .child(osd_hint_pill())
}

fn aspect_frame_preview() -> impl IntoElement {
    let is_portrait = computed(|| store::controls_snapshot().aspect == AspectPreset::Portrait9x16);
    view().size_full().items_center().justify_center().child(
        view()
            .h_full()
            .w_full()
            .relative()
            .style_when(move || is_portrait.get(), |s| s.w(px(340.0)))
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
            )),
    )
}

fn top_overlay_strip(on_back: impl Fn() + Copy + 'static) -> impl IntoElement {
    let show_back = !is_desktop();
    view()
        .absolute()
        .top_0()
        .pt_safe_area_plus(px(12.0))
        .left_3()
        .children(show_back.then(|| {
            app_icon_button(
                IconKind::ChevronLeft,
                "返回",
                ButtonVariant::CinemaGlass,
                ControlSize::Sm,
                on_back,
            )
        }))
}

fn osd_hint_pill() -> impl IntoElement {
    let hint_sig = computed(|| store::controls_snapshot().status_hint);
    view()
        .absolute()
        .top_0()
        .pt_safe_area_plus(px(12.0))
        .right_3()
        .child(dynamic(move || {
            let Some(hint) = hint_sig.get() else {
                return view().into_element();
            };
            view()
                .px_3()
                .py_1()
                .rounded(token::radius_badge)
                .bg(token::cinema_scrim)
                .child(
                    text(hint)
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(token::cinema_fg),
                )
                .into_element()
        }))
}

fn bottom_right_popover() -> impl IntoElement {
    let popover_sig = computed(|| store::controls_snapshot().popover);
    view()
        .absolute()
        .bottom(px(56.0))
        .right(move || match popover_sig.get() {
            ActivePopover::Speed => px(124.0),
            ActivePopover::Settings => px(48.0),
            ActivePopover::None => px(12.0),
        })
        .child(dynamic(move || match popover_sig.get() {
            ActivePopover::None => view().into_element(),
            ActivePopover::Settings => settings_popover().into_element(),
            ActivePopover::Speed => speed_popover().into_element(),
        }))
}

fn settings_popover() -> impl IntoElement {
    let enter = token::fade_enter();
    view()
        .bind_motion(&enter)
        .w(px(208.0))
        .p_2()
        .rounded(token::radius_card)
        .bg(token::cinema_surface)
        .border_1()
        .border_color(token::cinema_border)
        .gap_1()
        .child(toggle_switch_row(
            "自动连播",
            || store::snapshot().auto_advance,
            true,
            store::toggle_auto_advance,
        ))
        .child(toggle_switch_row(
            "单集循环",
            || store::snapshot().loop_single,
            true,
            store::toggle_loop_single,
        ))
        .children(is_desktop().then(|| {
            view()
                .gap_1()
                .child(cinema_separator())
                .child(app_icon_text_button(
                    IconKind::ExternalLink,
                    "外部播放器打开",
                    ButtonVariant::OverlayGhost,
                    ControlSize::Sm,
                    handler::launch_external_player_hint,
                ))
        }))
}

fn speed_popover() -> impl IntoElement {
    let enter = token::fade_enter();
    let presets: &[(u16, &str)] = &[
        (200, "2.0×"),
        (150, "1.5×"),
        (125, "1.25×"),
        (100, "1.0×"),
        (75, "0.75×"),
    ];
    let current = computed(|| store::controls_snapshot().rate_hundredths);
    view()
        .bind_motion(&enter)
        .w(px(86.0))
        .p_1p5()
        .rounded(token::radius_card)
        .bg(token::cinema_surface)
        .border_1()
        .border_color(token::cinema_border)
        .child(dynamic(move || {
            let active_rate = current.get();
            let mut panel = view().gap_1();
            for &(rate, label) in presets {
                let variant = if active_rate == rate {
                    ButtonVariant::CinemaPrimary
                } else {
                    ButtonVariant::OverlayGhost
                };
                panel = panel.child(app_button(label, variant, ControlSize::Sm, move || {
                    store::set_rate(rate);
                }));
            }
            panel
        }))
}

fn bottom_control_bar(on_toggle_fullscreen: impl Fn() + Copy + 'static) -> impl IntoElement {
    cleanup(|| store::set_controls_hovered(false));
    let enter = token::fade_enter();
    view()
        .bind_motion(&enter)
        .absolute()
        .left_0()
        .right_0()
        .bottom_0()
        .bg(token::cinema_scrim)
        .px_3()
        .pt_2()
        .pb_2()
        .gap_2()
        .on_mouse_enter(|_: &nsh_core::MouseEvent| store::set_controls_hovered(true))
        .on_mouse_leave(|_: &nsh_core::MouseEvent| store::set_controls_hovered(false))
        .child(scrub_progress_bar())
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .justify_between()
                .gap_2()
                .child(left_transport_cluster())
                .child(right_controls_cluster(on_toggle_fullscreen)),
        )
}

fn scrub_progress_bar() -> impl IntoElement {
    scrub_slider(
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
    )
}

fn left_transport_cluster() -> impl IntoElement {
    let paused_sig = computed(|| store::controls_snapshot().paused);
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .child(dynamic(move || {
            let icon_kind = if paused_sig.get() {
                IconKind::Play
            } else {
                IconKind::Pause
            };
            app_icon_button(
                icon_kind,
                "播放或暂停",
                ButtonVariant::OverlayGhost,
                ControlSize::Sm,
                store::toggle_paused,
            )
            .into_element()
        }))
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
        )
}

fn right_controls_cluster(on_toggle_fullscreen: impl Fn() + Copy + 'static) -> impl IntoElement {
    let muted_sig = computed(|| store::controls_snapshot().muted);
    let rate_sig = computed(|| store::controls_snapshot().rate_hundredths);
    view()
        .flex_row()
        .items_center()
        .gap_1()
        .child(app_button(
            move || format_rate_trigger(rate_sig.get()),
            ButtonVariant::OverlayGhost,
            ControlSize::Sm,
            || store::toggle_popover(ActivePopover::Speed),
        ))
        .child(dynamic(move || {
            let icon_kind = if muted_sig.get() {
                IconKind::VolumeMuted
            } else {
                IconKind::Volume
            };
            app_icon_button(
                icon_kind,
                "静音",
                ButtonVariant::OverlayGhost,
                ControlSize::Sm,
                store::toggle_muted,
            )
            .into_element()
        }))
        .child(app_icon_button(
            IconKind::Settings,
            "播放设置",
            ButtonVariant::OverlayGhost,
            ControlSize::Sm,
            || store::toggle_popover(ActivePopover::Settings),
        ))
        .child(app_icon_button(
            IconKind::Smartphone,
            "竖屏视图",
            ButtonVariant::OverlayGhost,
            ControlSize::Sm,
            || store::set_mode(PlaybackMode::ShortDrama),
        ))
        .child(app_icon_button(
            IconKind::Expand,
            "全屏",
            ButtonVariant::OverlayGhost,
            ControlSize::Sm,
            on_toggle_fullscreen,
        ))
}
