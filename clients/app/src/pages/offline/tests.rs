use std::time::Duration;

use nsh_engine::{ColorScheme, EngineConfig, Environment, Os, Points, Root, Size};
use nsh_testing::Headless;

use crate::App;
use crate::app::flags::{self, FeatureFlags};

fn start_on_offline(override_flags: FeatureFlags) -> Headless {
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
    app.press("离线").unwrap();
    app
}

#[test]
#[ignore = "待 offline/qry_library 切片实现后激活"]
fn offline_library_displays_downloaded_series_and_supports_deletion() {
    let mut app = start_on_offline(FeatureFlags {
        offline_library: true,
        ..FeatureFlags::default()
    });

    app.wait_for("已下载").unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
    app.wait_for("text:/3 集/").unwrap();

    app.press("删除 云端之上：千金归来 离线剧集").unwrap();
    app.wait_for_gone("云端之上：千金归来", Duration::from_secs(2))
        .unwrap();
}

#[test]
#[ignore = "待 offline/qry_queue 切片实现后激活"]
fn download_queue_displays_tasks_and_supports_pause_and_resume_all() {
    let mut app = start_on_offline(FeatureFlags {
        offline_queue: true,
        ..FeatureFlags::default()
    });

    app.wait_for("第 4 集 · 云城风云转折 4").unwrap();
    app.wait_for("长安暗夜行").unwrap();

    app.press("全部暂停").unwrap();
    app.wait_for("已暂停").unwrap();

    app.press("全部继续").unwrap();
    app.wait_for("排队中").unwrap();
}
