pub mod android;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod protocol;
pub mod windows;

pub use protocol::{
    PropChange, VIDEO_PLAYER_COMPONENT_NAME, VideoAspect, VideoPlayerEvent, VideoPlayerProps,
};

#[cfg(target_os = "macos")]
pub fn register_macos(components: &nsh_platform_macos::Components) {
    macos::register(components);
}

#[cfg(target_os = "windows")]
pub fn register_windows(components: &nsh_platform_windows::Components) {
    windows::register(components);
}
