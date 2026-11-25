use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

#[cfg(not(target_os = "android"))]
use hg_native_player::VIDEO_PLAYER_COMPONENT_NAME;
use nsh_base::{SliderValue, slider};
#[cfg(not(target_os = "android"))]
use nsh_core::JsonValue;
use nsh_core::platform::Lifecycle;
use nsh_core::prelude::*;

use crate::ui::token;

#[derive(Debug, Clone, PartialEq)]
pub struct VideoSurfaceState {
    pub src: Option<String>,
    pub paused: bool,
    pub rate: f64,
    pub volume: f64,
    pub muted: bool,
    pub seek_secs: u32,
    pub seek_seq: u64,
    pub aspect: &'static str,
}

pub fn scrub_slider(
    value_secs: impl Fn() -> u32 + Copy + 'static,
    duration_secs: impl Fn() -> u32 + Copy + 'static,
    on_scrub: impl Fn(u32) + 'static,
    on_commit: impl Fn(u32) + 'static,
) -> impl IntoElement {
    slider()
        .value(move || Some(SliderValue::Single(f64::from(value_secs()))))
        .min(0.0)
        .max(move || f64::from(duration_secs().max(1)))
        .step(1.0)
        .disabled(move || duration_secs() == 0)
        .accessibility_label("播放进度")
        .track_style(|s| s.w_full().h(px(18.0)).justify_center())
        .indicator_style(|s| {
            s.w_full()
                .h(px(4.0))
                .rounded_full()
                .bg(token::cinema_border)
        })
        .range_style(|s| s.rounded_full().bg(token::primary))
        .thumb_style(|s| s.size(px(12.0)).rounded_full().bg(token::cinema_fg))
        .on_change(move |val: &SliderValue| {
            on_scrub(f64_to_u32_secs(val.start()));
        })
        .on_change_end(move |val: &SliderValue| {
            on_commit(f64_to_u32_secs(val.start()));
        })
}

#[cfg(not(target_os = "android"))]
pub fn video_surface(
    state: impl Fn() -> VideoSurfaceState + Copy + 'static,
    on_progress: impl Fn(u32, u32) + 'static,
    on_ended: impl Fn() + 'static,
    on_error: impl Fn(String) + 'static,
) -> impl IntoElement {
    let src = computed(move || state().src);
    let paused = computed(move || state().paused);
    let rate = computed(move || state().rate);
    let volume = computed(move || state().volume);
    let muted = computed(move || state().muted);
    let seek_secs = computed(move || state().seek_secs);
    let seek_seq = computed(move || state().seek_seq);
    let aspect = computed(move || state().aspect);

    native(VIDEO_PLAYER_COMPONENT_NAME)
        .size_full()
        .prop("src", move || match src.get() {
            Some(uri) => JsonValue::String(uri),
            None => JsonValue::Null,
        })
        .prop("paused", move || JsonValue::Bool(paused.get()))
        .prop("rate", move || JsonValue::from(rate.get()))
        .prop("volume", move || JsonValue::from(volume.get()))
        .prop("muted", move || JsonValue::Bool(muted.get()))
        .prop("seek_secs", move || JsonValue::from(seek_secs.get()))
        .prop("seek_seq", move || JsonValue::from(seek_seq.get()))
        .prop("aspect", move || JsonValue::String(aspect.get().to_owned()))
        .on_native_event(move |event: &NativeEvent| {
            dispatch_video_event(event, &on_progress, &on_ended, &on_error);
        })
}

#[cfg(target_os = "android")]
pub fn video_surface(
    state: impl Fn() -> VideoSurfaceState + Copy + 'static,
    on_progress: impl Fn(u32, u32) + 'static,
    on_ended: impl Fn() + 'static,
    on_error: impl Fn(String) + 'static,
) -> impl IntoElement {
    let src = computed(move || state().src);
    let paused = computed(move || state().paused);
    let muted = computed(move || state().muted);
    let volume = computed(move || state().volume as f32);
    let rate = computed(move || state().rate as f32);
    let seek_target = computed(move || {
        let s = state();
        (s.seek_seq, s.seek_secs)
    });
    let video_ref = element_ref();
    effect(move || {
        let (seq, secs) = seek_target.get();
        if seq > 0 {
            video_ref.seek(f64::from(secs));
        }
    });

    when(
        move || src.get().is_some(),
        move || {
            video(move || src.get().unwrap_or_default())
                .element_ref(&video_ref)
                .size_full()
                .controls(false)
                .progress_interval(500.0)
                .paused(move || paused.get())
                .muted(move || muted.get())
                .volume(move || volume.get())
                .rate(move || rate.get())
                .on_load(move |_: &VideoInfo| {
                    let (seq, secs) = seek_target.get_untracked();
                    if seq > 0 && secs > 0 {
                        video_ref.seek(f64::from(secs));
                    }
                })
                .on_progress(move |p: &VideoProgress| {
                    on_progress(
                        f64_to_u32_secs(f64::from(p.current_time)),
                        f64_to_u32_secs(f64::from(p.duration)),
                    );
                })
                .on_end(move |(): &()| on_ended())
                .on_error(move |msg: &str| on_error(msg.to_owned()))
        },
    )
}

pub fn bind_auto_hide_timer(
    activity_seq: impl Fn() -> u64 + 'static,
    keep_visible: impl Fn() -> bool + 'static,
    delay: Duration,
    on_hide: impl Fn() + Copy + 'static,
) {
    let seq_sig = computed(activity_seq);
    let keep_sig = computed(keep_visible);
    let pending: Rc<RefCell<Option<Spawned<()>>>> = Rc::new(RefCell::new(None));
    let pending_cleanup = Rc::clone(&pending);
    cleanup(move || {
        if let Some(handle) = pending_cleanup.borrow_mut().take() {
            handle.cancel();
        }
    });
    effect(move || {
        let seq = seq_sig.get();
        let keep = keep_sig.get();
        if let Some(handle) = pending.borrow_mut().take() {
            handle.cancel();
        }
        if seq > 0 && !keep {
            let handle = set_timeout(delay, on_hide);
            *pending.borrow_mut() = Some(handle);
        }
    });
}

pub fn bind_lifecycle_pause(on_pause: impl Fn() + Copy + 'static) {
    let current = computed(lifecycle);
    effect(move || {
        if current.get() == Lifecycle::Background {
            on_pause();
        }
    });
}

fn f64_to_u32_secs(raw: f64) -> u32 {
    if !raw.is_finite() || raw <= 0.0 {
        return 0;
    }
    let rounded = raw.round();
    if rounded >= f64::from(u32::MAX) {
        u32::MAX
    } else {
        rounded as u32
    }
}

#[cfg(not(target_os = "android"))]
fn dispatch_video_event(
    event: &NativeEvent,
    on_progress: &impl Fn(u32, u32),
    on_ended: &impl Fn(),
    on_error: &impl Fn(String),
) {
    match event.name.as_ref() {
        "progress" => {
            let position = event
                .data
                .get("position_secs")
                .and_then(JsonValue::as_u64)
                .map_or(0, |v| u32::try_from(v).unwrap_or(u32::MAX));
            let duration = event
                .data
                .get("duration_secs")
                .and_then(JsonValue::as_u64)
                .map_or(0, |v| u32::try_from(v).unwrap_or(u32::MAX));
            on_progress(position, duration);
        }
        "ended" => on_ended(),
        "error" => {
            let message = event
                .data
                .get("message")
                .and_then(JsonValue::as_str)
                .unwrap_or("视频播放异常");
            on_error(message.to_owned());
        }
        _ => {}
    }
}
