use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use nsh_core::JsonValue;
use nsh_core::prelude::*;

pub const VIDEO_PLAYER_COMPONENT_NAME: &str = "hg-video-player";

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

pub fn video_surface(
    state: impl Fn() -> VideoSurfaceState + Copy + 'static,
    on_progress: impl Fn(u32, u32) + 'static,
    on_ended: impl Fn() + 'static,
    on_error: impl Fn(String) + 'static,
) -> impl IntoElement {
    native(VIDEO_PLAYER_COMPONENT_NAME)
        .size_full()
        .prop("src", move || match state().src {
            Some(uri) => JsonValue::String(uri),
            None => JsonValue::Null,
        })
        .prop("paused", move || JsonValue::Bool(state().paused))
        .prop("rate", move || JsonValue::from(state().rate))
        .prop("volume", move || JsonValue::from(state().volume))
        .prop("muted", move || JsonValue::Bool(state().muted))
        .prop("seek_secs", move || JsonValue::from(state().seek_secs))
        .prop("seek_seq", move || JsonValue::from(state().seek_seq))
        .prop("aspect", move || {
            JsonValue::String(state().aspect.to_owned())
        })
        .on_native_event(move |event: &NativeEvent| {
            dispatch_video_event(event, &on_progress, &on_ended, &on_error);
        })
}

pub fn bind_auto_hide_timer(
    activity_seq: impl Fn() -> u64 + 'static,
    keep_visible: impl Fn() -> bool + 'static,
    delay: Duration,
    on_hide: impl Fn() + Copy + 'static,
) {
    let pending: Rc<RefCell<Option<Spawned<()>>>> = Rc::new(RefCell::new(None));
    let pending_cleanup = Rc::clone(&pending);
    cleanup(move || {
        if let Some(handle) = pending_cleanup.borrow_mut().take() {
            handle.cancel();
        }
    });
    effect(move || {
        let seq = activity_seq();
        let keep = keep_visible();
        if let Some(handle) = pending.borrow_mut().take() {
            handle.cancel();
        }
        if seq > 0 && !keep {
            let handle = set_timeout(delay, on_hide);
            *pending.borrow_mut() = Some(handle);
        }
    });
}

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
