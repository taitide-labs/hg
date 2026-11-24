use std::time::Duration;

use nsh_engine::{ColorScheme, EngineConfig, Environment, Os, Points, Root, Size};
use nsh_testing::Headless;

use crate::App;

fn start_on_watchlist() -> Headless {
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
    app.press("追剧").unwrap();
    app
}

#[test]
#[ignore = "待 watchlist 切片实现并启用 WATCHLIST_PAGE 开关后激活"]
fn watchlist_displays_recent_history_and_bookmarks_and_resumes_episode() {
    let mut app = start_on_watchlist();

    app.wait_for("最近观看").unwrap();
    app.wait_for("我的追剧").unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
    app.wait_for("第 3 集").unwrap();
    app.wait_for("雾港七日").unwrap();

    app.press("云端之上：千金归来 >> nth=0").unwrap();
    app.wait_for("选集 (3/68)").unwrap();
}

#[test]
#[ignore = "待 watchlist 切片实现并启用 WATCHLIST_PAGE 开关后激活"]
fn removing_history_entry_and_bookmark_updates_watchlist() {
    let mut app = start_on_watchlist();

    app.wait_for("长安暗夜行").unwrap();
    app.press("移除 长安暗夜行 观看记录").unwrap();
    app.wait_for_gone("长安暗夜行", Duration::from_secs(2))
        .unwrap();

    app.wait_for("雾港七日").unwrap();
    app.press("取消追剧 雾港七日").unwrap();
    app.wait_for_gone("雾港七日", Duration::from_secs(2))
        .unwrap();
}
