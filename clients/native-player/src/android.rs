use core::fmt;

use nsh_core::JsonValue;
use serde::{Deserialize, Serialize};

use crate::protocol::{VIDEO_PLAYER_COMPONENT_NAME, VideoPlayerEvent};

pub const ANDROID_COMPONENT_NAME: &str = VIDEO_PLAYER_COMPONENT_NAME;
pub const ANDROID_HOST_PACKAGE: &str = "com.taitidelabs.hg.player";
pub const ANDROID_REGISTRATION_ENTRY: &str =
    "com.taitidelabs.hg.player.HgVideoPlayerRegistration.register";
pub const ANDROID_COMPONENT_INTERFACE: &str = "com.longbridge.nsh.NshComponent";
pub const ANDROID_DEFAULT_POOL_CAPACITY: usize = 3;
pub const ANDROID_PROGRESS_INTERVAL_MS: u64 = 500;

pub const ANDROID_PROP_KEYS: [&str; 8] = [
    "src",
    "paused",
    "rate",
    "volume",
    "muted",
    "seek_secs",
    "seek_seq",
    "aspect",
];

pub const ANDROID_EVENT_PROGRESS: &str = "progress";
pub const ANDROID_EVENT_ENDED: &str = "ended";
pub const ANDROID_EVENT_ERROR: &str = "error";

pub const ANDROID_EVENT_NAMES: [&str; 3] = [
    ANDROID_EVENT_PROGRESS,
    ANDROID_EVENT_ENDED,
    ANDROID_EVENT_ERROR,
];

pub const ANDROID_PAYLOAD_POSITION_SECS: &str = "position_secs";
pub const ANDROID_PAYLOAD_DURATION_SECS: &str = "duration_secs";
pub const ANDROID_PAYLOAD_MESSAGE: &str = "message";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AndroidRenderSurface {
    TextureView,
}

impl AndroidRenderSurface {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TextureView => "TextureView",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AndroidBridgeError {
    UnknownEvent(String),
    MissingOrInvalidField {
        event: &'static str,
        field: &'static str,
    },
    ExpectedNullPayload {
        event: &'static str,
    },
}

impl fmt::Display for AndroidBridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownEvent(name) => write!(f, "unknown android player event: {name}"),
            Self::MissingOrInvalidField { event, field } => {
                write!(f, "invalid or missing field `{field}` in `{event}` event")
            }
            Self::ExpectedNullPayload { event } => {
                write!(f, "expected null payload for `{event}` event")
            }
        }
    }
}

impl std::error::Error for AndroidBridgeError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AndroidPlayerBridgeSpec {
    pub component_name: &'static str,
    pub host_package: &'static str,
    pub registration_entry: &'static str,
    pub component_interface: &'static str,
    pub render_surface: AndroidRenderSurface,
    pub default_pool_capacity: usize,
    pub progress_interval_ms: u64,
    pub prop_keys: [&'static str; 8],
    pub event_names: [&'static str; 3],
}

impl Default for AndroidPlayerBridgeSpec {
    fn default() -> Self {
        Self::current()
    }
}

impl AndroidPlayerBridgeSpec {
    #[must_use]
    pub const fn current() -> Self {
        Self {
            component_name: ANDROID_COMPONENT_NAME,
            host_package: ANDROID_HOST_PACKAGE,
            registration_entry: ANDROID_REGISTRATION_ENTRY,
            component_interface: ANDROID_COMPONENT_INTERFACE,
            render_surface: AndroidRenderSurface::TextureView,
            default_pool_capacity: ANDROID_DEFAULT_POOL_CAPACITY,
            progress_interval_ms: ANDROID_PROGRESS_INTERVAL_MS,
            prop_keys: ANDROID_PROP_KEYS,
            event_names: ANDROID_EVENT_NAMES,
        }
    }

    #[must_use]
    pub fn supports_prop(&self, name: &str) -> bool {
        self.prop_keys.contains(&name)
    }

    #[must_use]
    pub fn supports_event(&self, name: &str) -> bool {
        self.event_names.contains(&name)
    }

    #[must_use]
    pub const fn uses_texture_view(&self) -> bool {
        matches!(self.render_surface, AndroidRenderSurface::TextureView)
    }

    #[must_use]
    pub fn encode_event(&self, event: VideoPlayerEvent) -> (&'static str, JsonValue) {
        event.into_name_and_payload()
    }

    pub fn parse_event(
        &self,
        name: &str,
        payload: &JsonValue,
    ) -> Result<VideoPlayerEvent, AndroidBridgeError> {
        parse_bridge_event(name, payload)
    }
}

pub fn parse_bridge_event(
    name: &str,
    payload: &JsonValue,
) -> Result<VideoPlayerEvent, AndroidBridgeError> {
    match name {
        ANDROID_EVENT_PROGRESS => Ok(VideoPlayerEvent::Progress {
            position_secs: parse_u32_field(
                payload,
                ANDROID_EVENT_PROGRESS,
                ANDROID_PAYLOAD_POSITION_SECS,
            )?,
            duration_secs: parse_u32_field(
                payload,
                ANDROID_EVENT_PROGRESS,
                ANDROID_PAYLOAD_DURATION_SECS,
            )?,
        }),
        ANDROID_EVENT_ENDED => {
            if payload.is_null() {
                Ok(VideoPlayerEvent::Ended)
            } else {
                Err(AndroidBridgeError::ExpectedNullPayload {
                    event: ANDROID_EVENT_ENDED,
                })
            }
        }
        ANDROID_EVENT_ERROR => {
            let message = payload
                .get(ANDROID_PAYLOAD_MESSAGE)
                .and_then(JsonValue::as_str)
                .ok_or(AndroidBridgeError::MissingOrInvalidField {
                    event: ANDROID_EVENT_ERROR,
                    field: ANDROID_PAYLOAD_MESSAGE,
                })?;
            Ok(VideoPlayerEvent::Error {
                message: message.to_owned(),
            })
        }
        other => Err(AndroidBridgeError::UnknownEvent(other.to_owned())),
    }
}

fn parse_u32_field(
    payload: &JsonValue,
    event: &'static str,
    field: &'static str,
) -> Result<u32, AndroidBridgeError> {
    let raw = payload
        .get(field)
        .and_then(JsonValue::as_u64)
        .ok_or(AndroidBridgeError::MissingOrInvalidField { event, field })?;
    u32::try_from(raw).map_err(|_| AndroidBridgeError::MissingOrInvalidField { event, field })
}

#[cfg(test)]
mod tests;
