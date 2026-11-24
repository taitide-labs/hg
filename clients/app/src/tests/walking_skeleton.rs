use std::time::Duration;

use nsh_engine::{ColorScheme, EngineConfig, Environment, Os, Points, Root, Size};
use nsh_testing::Headless;

use crate::App;

fn start_desktop(scheme: ColorScheme) -> Headless {
    let environment = Environment {
        os: Os::MacOs,
        color_scheme: scheme,
        ..Environment::default()
    };
    let mut app = Headless::start(EngineConfig {
        viewport: Size::new(Points(1240.0), Points(800.0)),
        environment,
        ..EngineConfig::new(Root::of::<App>())
    })
    .unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
    app
}

#[test]
fn discover_home_loads_and_filters_by_category() {
    let mut app = start_desktop(ColorScheme::Light);
    assert!(app.screen().shows("精选推荐"));
    assert!(app.screen().shows("长安暗夜行"));

    app.press("漫剧").unwrap();
    app.wait_for("星穹机甲纪元").unwrap();
    app.wait_for_gone("春风十里不及卿", Duration::from_secs(2))
        .unwrap();
}

#[test]
fn clicking_series_opens_player_switches_mode_and_records_progress() {
    let mut app = start_desktop(ColorScheme::Dark);

    app.press("云端之上：千金归来").unwrap();
    app.wait_for("本地离线 · HEVC").unwrap();
    assert!(app.screen().shows("电影模式"));

    app.press("第 5 集").unwrap();
    app.wait_for("在线流 · HEVC").unwrap();

    app.press("比例 3").unwrap();
    app.wait_for("16:9 剧场").unwrap();

    app.press("切至短剧模式").unwrap();
    app.wait_for("切至电影模式").unwrap();

    app.press("完播并连播").unwrap();
    app.wait_for("text:/正在播放 · 第 6 集/").unwrap();

    app.press("返回").unwrap();
    app.wait_for("精选推荐").unwrap();
}

#[test]
fn navigation_separates_settings_from_three_primary_tabs() {
    let mut app = start_desktop(ColorScheme::Light);

    app.press("追剧").unwrap();
    app.wait_for("追剧与观看历史").unwrap();

    app.press("离线").unwrap();
    app.wait_for("离线片库与下载队列").unwrap();

    app.press("设置").unwrap();
    app.wait_for("偏好与缓存设置").unwrap();

    app.press("发现").unwrap();
    app.wait_for("精选推荐").unwrap();
}
