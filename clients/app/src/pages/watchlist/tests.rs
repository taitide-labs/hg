use std::time::Duration;

use nsh_engine::{ColorScheme, EngineConfig, Environment, Os, Points, Root, Size};
use nsh_testing::Headless;

use crate::App;
use crate::app::flags::{self, FeatureFlags};

fn start_on_watchlist(override_flags: FeatureFlags) -> Headless {
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
    app.press("追剧").unwrap();
    app
}

#[test]
#[ignore = "待 watchlist/qry_history 切片实现后激活"]
fn watch_history_shows_recent_items_resumes_episode_and_removes_entry() {
    let mut app = start_on_watchlist(FeatureFlags {
        watchlist_history: true,
        ..FeatureFlags::default()
    });

    app.wait_for("最近观看").unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
    app.wait_for("第 3 集").unwrap();
    app.wait_for("长安暗夜行").unwrap();

    app.press("移除 长安暗夜行 观看记录").unwrap();
    app.wait_for_gone("长安暗夜行", Duration::from_secs(2))
        .unwrap();

    app.press("云端之上：千金归来 >> nth=0").unwrap();
    app.wait_for("选集 (3/68)").unwrap();
}

#[test]
#[ignore = "待 watchlist/qry_bookmarks 切片实现后激活"]
fn bookmarks_shows_saved_series_and_allows_unbookmarking() {
    let mut app = start_on_watchlist(FeatureFlags {
        watchlist_bookmarks: true,
        ..FeatureFlags::default()
    });

    app.wait_for("我的追剧").unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
    app.wait_for("雾港七日").unwrap();

    app.press("取消追剧 雾港七日").unwrap();
    app.wait_for_gone("雾港七日", Duration::from_secs(2))
        .unwrap();
}
