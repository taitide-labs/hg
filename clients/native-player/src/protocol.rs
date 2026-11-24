use nsh_core::JsonValue;
use serde::{Deserialize, Serialize};

pub const VIDEO_PLAYER_COMPONENT_NAME: &str = "hg-video-player";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum VideoAspect {
    #[default]
    Original,
    Fill,
    Cinema16x9,
    Portrait9x16,
}

impl VideoAspect {
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        match raw {
            "fill" => Self::Fill,
            "16:9" => Self::Cinema16x9,
            "9:16" => Self::Portrait9x16,
            _ => Self::Original,
        }
    }

    #[must_use]
    pub const fn as_protocol_str(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Fill => "fill",
            Self::Cinema16x9 => "16:9",
            Self::Portrait9x16 => "9:16",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoPlayerProps {
    pub src: Option<String>,
    pub paused: bool,
    pub rate: f64,
    pub volume: f64,
    pub muted: bool,
    pub seek_secs: u32,
    pub seek_seq: u64,
    pub aspect: VideoAspect,
}

impl Default for VideoPlayerProps {
    fn default() -> Self {
        Self {
            src: None,
            paused: false,
            rate: 1.0,
            volume: 0.8,
            muted: false,
            seek_secs: 0,
            seek_seq: 0,
            aspect: VideoAspect::Original,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropChange {
    Unchanged,
    Source,
    Paused,
    Rate,
    Volume,
    Muted,
    Seek { position_secs: u32 },
    Aspect,
}

impl VideoPlayerProps {
    pub fn apply_prop(&mut self, name: &str, value: Option<&JsonValue>) -> PropChange {
        match name {
            "src" => self.apply_src(value),
            "paused" => self.apply_paused(value),
            "rate" => self.apply_rate(value),
            "volume" => self.apply_volume(value),
            "muted" => self.apply_muted(value),
            "seek_secs" => self.apply_seek_secs(value),
            "seek_seq" => self.apply_seek_seq(value),
            "aspect" => self.apply_aspect(value),
            _ => PropChange::Unchanged,
        }
    }

    fn apply_src(&mut self, value: Option<&JsonValue>) -> PropChange {
        let next = value
            .and_then(JsonValue::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_owned);
        if self.src == next {
            PropChange::Unchanged
        } else {
            self.src = next;
            PropChange::Source
        }
    }

    fn apply_paused(&mut self, value: Option<&JsonValue>) -> PropChange {
        let next = value.and_then(JsonValue::as_bool).unwrap_or(false);
        if self.paused == next {
            PropChange::Unchanged
        } else {
            self.paused = next;
            PropChange::Paused
        }
    }

    fn apply_rate(&mut self, value: Option<&JsonValue>) -> PropChange {
        let next = value
            .and_then(JsonValue::as_f64)
            .filter(|v| v.is_finite())
            .unwrap_or(1.0)
            .clamp(0.25, 4.0);
        if (self.rate - next).abs() < f64::EPSILON {
            PropChange::Unchanged
        } else {
            self.rate = next;
            PropChange::Rate
        }
    }

    fn apply_volume(&mut self, value: Option<&JsonValue>) -> PropChange {
        let next = value
            .and_then(JsonValue::as_f64)
            .filter(|v| v.is_finite())
            .unwrap_or(0.8)
            .clamp(0.0, 1.0);
        if (self.volume - next).abs() < f64::EPSILON {
            PropChange::Unchanged
        } else {
            self.volume = next;
            PropChange::Volume
        }
    }

    fn apply_muted(&mut self, value: Option<&JsonValue>) -> PropChange {
        let next = value.and_then(JsonValue::as_bool).unwrap_or(false);
        if self.muted == next {
            PropChange::Unchanged
        } else {
            self.muted = next;
            PropChange::Muted
        }
    }

    fn apply_seek_secs(&mut self, value: Option<&JsonValue>) -> PropChange {
        self.seek_secs = value
            .and_then(JsonValue::as_u64)
            .map_or(0, |v| u32::try_from(v).unwrap_or(u32::MAX));
        PropChange::Unchanged
    }

    fn apply_seek_seq(&mut self, value: Option<&JsonValue>) -> PropChange {
        let next = value.and_then(JsonValue::as_u64).unwrap_or(0);
        if next == 0 || next == self.seek_seq {
            self.seek_seq = next;
            PropChange::Unchanged
        } else {
            self.seek_seq = next;
            PropChange::Seek {
                position_secs: self.seek_secs,
            }
        }
    }

    fn apply_aspect(&mut self, value: Option<&JsonValue>) -> PropChange {
        let next = value
            .and_then(JsonValue::as_str)
            .map_or(VideoAspect::Original, VideoAspect::parse);
        if self.aspect == next {
            PropChange::Unchanged
        } else {
            self.aspect = next;
            PropChange::Aspect
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoPlayerEvent {
    Progress {
        position_secs: u32,
        duration_secs: u32,
    },
    Ended,
    Error {
        message: String,
    },
}

impl VideoPlayerEvent {
    #[must_use]
    pub fn into_name_and_payload(self) -> (&'static str, JsonValue) {
        match self {
            Self::Progress {
                position_secs,
                duration_secs,
            } => (
                "progress",
                serde_json::json!({
                    "position_secs": position_secs,
                    "duration_secs": duration_secs,
                }),
            ),
            Self::Ended => ("ended", JsonValue::Null),
            Self::Error { message } => ("error", serde_json::json!({ "message": message })),
        }
    }
}

#[cfg(test)]
mod tests;
