use std::sync::Arc;
use std::time::Duration;

use hg_gateway::mock::MockGateway;
use nsh_engine::{ColorScheme, EngineConfig, Environment, Os, Points, Size};
use nsh_testing::Headless;

use crate::root_with_gateway;

fn start_desktop(scheme: ColorScheme) -> Headless {
    let environment = Environment {
        os: Os::MacOs,
        color_scheme: scheme,
        ..Environment::default()
    };
    let mut app = Headless::start(EngineConfig {
        viewport: Size::new(Points(1240.0), Points(800.0)),
        environment,
        ..EngineConfig::new(root_with_gateway(Arc::new(MockGateway::new())))
    })
    .unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
    app
}

#[test]
fn discover_home_shows_featured_ranking_and_categories_without_hero_banner() {
    let app = start_desktop(ColorScheme::Light);
    let screen = app.screen();

    assert!(screen.shows("精选推荐"));
    assert!(screen.shows("排行榜"));
    assert!(screen.shows("热门短剧"));
    assert!(screen.shows("漫剧与悬疑"));
    assert!(!screen.shows("今日焦点"));
}

#[test]
fn opening_featured_and_ranking_hides_search_bar() {
    let mut app = start_desktop(ColorScheme::Light);

    app.press("查看排行榜全部").unwrap();
    app.wait_for("热播榜").unwrap();
    assert!(!app.screen().shows("热搜"));
    app.press("潜力榜").unwrap();
    app.wait_for("潜力榜").unwrap();

    app.press("返回").unwrap();
    app.wait_for("查看精选推荐全部").unwrap();

    app.press("查看精选推荐全部").unwrap();
    app.wait_for("春风十里不及卿").unwrap();
    assert!(!app.screen().shows("热搜"));
    assert!(!app.screen().shows("题材"));
}

#[test]
fn searching_from_home_enters_search_page_with_tag_filters() {
    let mut app = start_desktop(ColorScheme::Dark);

    app.press("搜索").unwrap();
    app.wait_for("热搜").unwrap();
    app.press("千金归来").unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
    app.wait_for_gone("星穹机甲纪元", Duration::from_secs(2))
        .unwrap();

    app.press("悬疑").unwrap();
    app.wait_for("长安暗夜行").unwrap();
    app.wait_for_gone("春风十里不及卿", Duration::from_secs(2))
        .unwrap();
}

#[test]
fn sidebar_navigation_keeps_settings_separated_from_primary_tabs() {
    let mut app = start_desktop(ColorScheme::Light);

    app.press("追剧").unwrap();
    app.wait_for("最近观看").unwrap();
    app.wait_for("我的追剧").unwrap();

    app.press("离线").unwrap();
    app.wait_for("已下载").unwrap();
    app.wait_for("下载中").unwrap();

    app.press("设置").unwrap();
    app.wait_for("外观").unwrap();
    app.wait_for("存储空间").unwrap();

    app.press("发现").unwrap();
    app.wait_for("精选推荐").unwrap();
}
