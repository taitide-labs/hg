use nsh_core::JsonValue;

use crate::protocol::{PropChange, VideoAspect, VideoPlayerEvent, VideoPlayerProps};

use super::commands::{MpvCommand, commands_for_prop_change};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpvEndFileReason {
    Eof,
    Stop,
    Quit,
    Error,
    Redirect,
    Unknown(i32),
}

impl MpvEndFileReason {
    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Eof,
            2 => Self::Stop,
            3 => Self::Quit,
            4 => Self::Error,
            5 => Self::Redirect,
            other => Self::Unknown(other),
        }
    }
}

#[must_use]
pub fn map_end_file_event(
    reason: i32,
    error_code: i32,
    error_detail: Option<&str>,
) -> Option<VideoPlayerEvent> {
    match MpvEndFileReason::from_raw(reason) {
        MpvEndFileReason::Eof => Some(VideoPlayerEvent::Ended),
        MpvEndFileReason::Error => {
            let message = match error_detail.filter(|s| !s.is_empty()) {
                Some(detail) => format!("libmpv playback failed ({error_code}): {detail}"),
                None => format!("libmpv playback failed (code {error_code})"),
            };
            Some(VideoPlayerEvent::Error { message })
        }
        MpvEndFileReason::Stop
        | MpvEndFileReason::Quit
        | MpvEndFileReason::Redirect
        | MpvEndFileReason::Unknown(_) => None,
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ProgressTracker {
    position_secs: Option<u32>,
    duration_secs: Option<u32>,
    last_emitted: Option<(u32, u32)>,
}

impl ProgressTracker {
    pub fn reset(&mut self) {
        self.position_secs = None;
        self.duration_secs = None;
        self.last_emitted = None;
    }

    pub fn on_time_pos(&mut self, secs: f64) -> Option<VideoPlayerEvent> {
        self.position_secs = Some(sanitize_secs(secs));
        self.maybe_emit()
    }

    pub fn on_duration(&mut self, secs: f64) -> Option<VideoPlayerEvent> {
        self.duration_secs = Some(sanitize_secs(secs));
        self.maybe_emit()
    }

    fn maybe_emit(&mut self) -> Option<VideoPlayerEvent> {
        let pos = self.position_secs?;
        let dur = self.duration_secs.unwrap_or(0);
        let pair = (pos, dur);
        if self.last_emitted == Some(pair) {
            return None;
        }
        self.last_emitted = Some(pair);
        Some(VideoPlayerEvent::Progress {
            position_secs: pos,
            duration_secs: dur,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropApplyOutcome {
    pub commands: Vec<MpvCommand>,
    pub needs_redraw: bool,
}

#[derive(Debug, Default)]
pub struct WindowsPlayerState {
    props: VideoPlayerProps,
    progress: ProgressTracker,
    last_error: Option<String>,
}

impl WindowsPlayerState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply_prop(&mut self, name: &str, value: Option<&JsonValue>) -> PropApplyOutcome {
        let change = self.props.apply_prop(name, value);
        if matches!(change, PropChange::Source) {
            self.progress.reset();
            self.last_error = None;
        }
        let commands = commands_for_prop_change(&self.props, change);
        let needs_redraw = matches!(change, PropChange::Aspect)
            || (matches!(change, PropChange::Source) && self.props.src.is_none());
        PropApplyOutcome {
            commands,
            needs_redraw,
        }
    }

    #[must_use]
    pub const fn props(&self) -> &VideoPlayerProps {
        &self.props
    }

    #[must_use]
    pub const fn aspect(&self) -> VideoAspect {
        self.props.aspect
    }

    pub fn on_time_pos(&mut self, secs: f64) -> Option<VideoPlayerEvent> {
        self.progress.on_time_pos(secs)
    }

    pub fn on_duration(&mut self, secs: f64) -> Option<VideoPlayerEvent> {
        self.progress.on_duration(secs)
    }

    pub fn on_end_file(
        &mut self,
        reason: i32,
        error_code: i32,
        error_detail: Option<&str>,
    ) -> Option<VideoPlayerEvent> {
        map_end_file_event(reason, error_code, error_detail)
    }

    pub fn record_error_once(&mut self, message: String) -> Option<VideoPlayerEvent> {
        if self.last_error.as_deref() == Some(message.as_str()) {
            return None;
        }
        self.last_error = Some(message.clone());
        Some(VideoPlayerEvent::Error { message })
    }
}

fn sanitize_secs(secs: f64) -> u32 {
    if !secs.is_finite() || secs <= 0.0 {
        return 0;
    }
    let floored = secs.floor();
    if floored >= f64::from(u32::MAX) {
        u32::MAX
    } else {
        floored as u32
    }
}
