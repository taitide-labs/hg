use std::ffi::CStr;
use std::sync::OnceLock;

use nsh_platform_macos::objc2::runtime::AnyClass;
use nsh_platform_macos::objc2::{Encode, Encoding};
use nsh_platform_macos::objc2_foundation::{NSString, NSURL};

use crate::protocol::VideoAspect;

const AV_FOUNDATION_PATH: &CStr = c"/System/Library/Frameworks/AVFoundation.framework/AVFoundation";
const CM_TIME_TIMESCALE: i32 = 600;
const CM_TIME_VALID_FLAG: u32 = 1;
const CM_TIME_INDEFINITE_FLAG: u32 = 0x10;

static AV_FOUNDATION_LOADED: OnceLock<bool> = OnceLock::new();

pub(super) fn ensure_loaded() -> bool {
    *AV_FOUNDATION_LOADED.get_or_init(|| {
        #[expect(
            unsafe_code,
            reason = "按需通过 dlopen 延迟加载 macOS 系统 AVFoundation 框架"
        )]
        let handle = unsafe { libc::dlopen(AV_FOUNDATION_PATH.as_ptr(), libc::RTLD_LAZY) };
        !handle.is_null()
    })
}

pub(super) fn class(name: &CStr) -> Option<&'static AnyClass> {
    AnyClass::get(name)
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CMTime {
    pub(super) value: i64,
    pub(super) timescale: i32,
    pub(super) flags: u32,
    pub(super) epoch: i64,
}

#[expect(
    unsafe_code,
    reason = "CoreMedia CMTime C 结构体布局与 Objective-C 类型编码匹配"
)]
unsafe impl Encode for CMTime {
    const ENCODING: Encoding = Encoding::Struct(
        "?",
        &[i64::ENCODING, i32::ENCODING, u32::ENCODING, i64::ENCODING],
    );
}

impl CMTime {
    pub(super) const fn zero() -> Self {
        Self {
            value: 0,
            timescale: CM_TIME_TIMESCALE,
            flags: CM_TIME_VALID_FLAG,
            epoch: 0,
        }
    }

    pub(super) fn from_seconds(seconds: f64) -> Self {
        let clamped = if seconds.is_finite() {
            seconds.clamp(0.0, 1e12)
        } else {
            0.0
        };
        #[expect(
            clippy::cast_possible_truncation,
            reason = "clamped 已限制在 [0, 1e12]，乘以 600 后仍在 i64 安全范围内"
        )]
        let value = (clamped * f64::from(CM_TIME_TIMESCALE)).round() as i64;
        Self {
            value,
            timescale: CM_TIME_TIMESCALE,
            flags: CM_TIME_VALID_FLAG,
            epoch: 0,
        }
    }

    pub(super) fn seconds(self) -> Option<f64> {
        if (self.flags & CM_TIME_VALID_FLAG) == 0
            || (self.flags & CM_TIME_INDEFINITE_FLAG) != 0
            || self.timescale <= 0
        {
            return None;
        }
        let secs = self.value as f64 / f64::from(self.timescale);
        if secs.is_finite() && secs >= 0.0 {
            Some(secs)
        } else {
            None
        }
    }
}

pub(super) fn cmtime_to_secs_u32(time: CMTime) -> u32 {
    time.seconds().map_or(0, |secs| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "secs 非负且已钳位至 u32::MAX"
        )]
        let whole = secs.floor().min(f64::from(u32::MAX)) as u32;
        whole
    })
}

pub(super) const fn video_gravity_for_aspect(aspect: VideoAspect) -> &'static str {
    match aspect {
        VideoAspect::Original | VideoAspect::Cinema16x9 | VideoAspect::Portrait9x16 => {
            "AVLayerVideoGravityResizeAspect"
        }
        VideoAspect::Fill => "AVLayerVideoGravityResizeAspectFill",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MediaSourceKind {
    Url,
    FilePath,
}

pub(super) fn classify_source(raw: &str) -> Option<MediaSourceKind> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.contains("://") {
        Some(MediaSourceKind::Url)
    } else {
        Some(MediaSourceKind::FilePath)
    }
}

pub(super) fn parse_nsurl(raw: &str) -> Option<nsh_platform_macos::objc2::rc::Retained<NSURL>> {
    match classify_source(raw)? {
        MediaSourceKind::Url => NSURL::URLWithString(&NSString::from_str(raw.trim())),
        MediaSourceKind::FilePath => Some(NSURL::fileURLWithPath(&NSString::from_str(raw.trim()))),
    }
}
