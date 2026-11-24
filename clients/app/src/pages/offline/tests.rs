use std::time::Duration;

use nsh_engine::{ColorScheme, EngineConfig, Environment, Os, Points, Root, Size};
use nsh_testing::Headless;

use crate::App;

fn start_on_offline() -> Headless {
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
#[ignore = "待 offline 切片实现并启用 OFFLINE_LIBRARY_PAGE 开关后激活"]
fn offline_page_shows_downloaded_library_and_switches_to_download_queue() {
    let mut app = start_on_offline();

    app.wait_for("已下载").unwrap();
    app.wait_for("下载中").unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
    app.wait_for("text:/3 集/").unwrap();

    app.press("下载中").unwrap();
    app.wait_for("第 4 集 · 云城风云转折 4").unwrap();
    app.wait_for("长安暗夜行").unwrap();
}

#[test]
#[ignore = "待 offline 切片实现并启用 OFFLINE_LIBRARY_PAGE 开关后激活"]
fn controlling_download_queue_and_deleting_offline_series_updates_state() {
    let mut app = start_on_offline();

    app.press("下载中").unwrap();
    app.wait_for("全部暂停").unwrap();
    app.press("全部暂停").unwrap();
    app.wait_for("已暂停").unwrap();

    app.press("全部继续").unwrap();
    app.wait_for("排队中").unwrap();

    app.press("已下载").unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
    app.press("删除 云端之上：千金归来 离线剧集").unwrap();
    app.wait_for_gone("云端之上：千金归来", Duration::from_secs(2))
        .unwrap();
}
