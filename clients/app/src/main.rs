#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "macos")]
use nsh_platform_macos as platform;
#[cfg(target_os = "windows")]
use nsh_platform_windows as platform;

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn main() -> impl std::process::Termination {
    #[cfg(target_os = "macos")]
    platform::set_components(hg_native_player::register_macos);
    #[cfg(target_os = "windows")]
    platform::set_components(hg_native_player::register_windows);

    platform::run_app(
        platform::AppSource::Rust(platform::Root::of::<hg_app::App>()),
        platform::AppOptions {
            title: "HG".to_owned(),
            #[cfg(target_os = "windows")]
            app_id: "com.taitidelabs.hg".to_owned(),
            size: (1240, 800),
            ..Default::default()
        },
    )
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn main() {}
