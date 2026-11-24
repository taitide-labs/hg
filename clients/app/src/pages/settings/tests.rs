use nsh_engine::{ColorScheme, EngineConfig, Environment, Os, Points, Root, Size};
use nsh_testing::Headless;

use crate::App;
use crate::app::flags::{self, FeatureFlags};

fn start_on_settings(override_flags: FeatureFlags) -> Headless {
    flags::set_for_test(override_flags);
    let environment = Environment {
        os: Os::MacOs,
        color_scheme: ColorScheme::Light,
        ..Environment::default()
    };
    let mut app = Headless::start(EngineConfig {
        viewport: Size::new(Points(1240.0), Points(800.0)),
        environment,
        ..EngineConfig::new(Root::of::<App>())
    })
    .unwrap();
    app.wait_for("精选推荐").unwrap();
    app.press("设置").unwrap();
    app
}

#[test]
#[ignore = "待 settings/cmd_preferences 切片实现后激活"]
fn settings_preferences_controls_appearance_playback_and_download_policy() {
    let mut app = start_on_settings(FeatureFlags {
        settings_preferences: true,
        ..FeatureFlags::default()
    });

    app.wait_for("外观").unwrap();
    app.wait_for("跟随系统").unwrap();
    app.wait_for("播放偏好").unwrap();
    app.wait_for("下载策略").unwrap();
    app.wait_for("仅 Wi-Fi 下载").unwrap();
    app.press("仅 Wi-Fi 下载").unwrap();
}

#[test]
#[ignore = "待 settings/cmd_storage 切片实现后激活"]
fn settings_storage_clears_caches_without_touching_offline_library() {
    let mut app = start_on_settings(FeatureFlags {
        settings_storage: true,
        ..FeatureFlags::default()
    });

    app.wait_for("存储空间").unwrap();
    app.wait_for("在线播放与图片缓存").unwrap();
    app.wait_for("离线片库").unwrap();

    app.press("清理缓存").unwrap();
    app.wait_for("0 MB").unwrap();
    assert!(app.screen().shows("离线片库"));
    assert!(app.screen().shows("55.7 MB") || app.screen().shows("58.4 MB"));
}
