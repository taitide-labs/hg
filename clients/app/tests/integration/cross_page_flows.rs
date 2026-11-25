use std::sync::Arc;
use std::time::Duration;

use hg_gateway::mock::MockGateway;
use nsh_engine::ColorScheme;

use crate::support::{start_desktop_on_home, start_mobile};

#[test]
fn discover_to_player_fullscreen_hides_shell_chrome_and_back_returns_to_discover() {
    let gw = Arc::new(MockGateway::new());
    let mut app = start_desktop_on_home(gw, ColorScheme::Light);

    app.press("云端之上：千金归来 >> nth=0").unwrap();
    app.wait_for("选集 (3/68)").unwrap();
    assert!(app.screen().shows("返回"));

    app.hover("视频画面").unwrap();
    app.wait_for("全屏").unwrap();
    app.press("全屏").unwrap();
    app.wait_for_gone("返回", Duration::from_secs(2)).unwrap();

    app.press_key("enter").unwrap();
    app.wait_for("返回").unwrap();

    app.press("返回").unwrap();
    app.wait_for("精选推荐").unwrap();
}

#[test]
fn watchlist_and_history_navigate_to_player_with_expected_episode() {
    let gw = Arc::new(MockGateway::new());
    let mut app = start_desktop_on_home(gw, ColorScheme::Light);

    app.press("追剧").unwrap();
    app.wait_for("雾港七日").unwrap();
    app.press("雾港七日 >> nth=0").unwrap();
    app.wait_for("选集 (1/36)").unwrap();

    app.press("返回").unwrap();
    app.wait_for("我的追剧").unwrap();

    app.press("历史").unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
    app.press("云端之上：千金归来 >> nth=0").unwrap();
    app.wait_for("选集 (3/68)").unwrap();
}

#[test]
fn offline_drawer_episode_opens_player_and_mobile_back_exits_player_to_shell() {
    let gw = Arc::new(MockGateway::new());
    let mut desktop = start_desktop_on_home(Arc::clone(&gw), ColorScheme::Light);

    desktop.press("离线").unwrap();
    desktop
        .wait_for("label:管理 云端之上：千金归来 离线分集")
        .unwrap();
    desktop
        .press("label:管理 云端之上：千金归来 离线分集")
        .unwrap();
    desktop
        .wait_for("label:播放 云端之上：千金归来 第 1 集")
        .unwrap();
    desktop
        .press("label:播放 云端之上：千金归来 第 1 集")
        .unwrap();
    desktop.wait_for("选集 (1/68)").unwrap();

    let mut mobile = start_mobile(gw, ColorScheme::Light);
    mobile.wait_for("云端之上：千金归来").unwrap();
    mobile.press("云端之上：千金归来 >> nth=0").unwrap();
    mobile.wait_for("详情视图").unwrap();
    mobile.press("详情视图").unwrap();
    mobile.wait_for("离线下载").unwrap();

    mobile.press("离线下载").unwrap();
    mobile.wait_for("选择离线下载分集").unwrap();
    mobile.back().unwrap();
    mobile
        .wait_for_gone("选择离线下载分集", Duration::from_secs(2))
        .unwrap();
    assert!(mobile.screen().shows("离线下载"));

    mobile.back().unwrap();
    mobile.wait_for("精选推荐").unwrap();
}

#[test]
fn returning_from_player_preserves_browse_sort_and_category_context() {
    let gw = Arc::new(MockGateway::new().with_paged_catalog(30));
    let mut app = start_desktop_on_home(gw, ColorScheme::Light);

    app.press("查看热门短剧全部").unwrap();
    app.wait_for("分页剧目 01").unwrap();
    assert!(!app.screen().shows("分页剧目 30"));

    app.press("最近更新").unwrap();
    app.wait_for("分页剧目 30").unwrap();
    app.scroll("分页剧目 30", nsh_core::prelude::Points(2600.0))
        .unwrap();
    app.wait_for("分页剧目 02").unwrap();

    app.press("分页剧目 02 >> nth=0").unwrap();
    app.wait_for("选集 (1/12)").unwrap();

    app.press("返回").unwrap();
    app.wait_for("分页剧目 02").unwrap();
    assert!(
        app.screen().shows("分页剧目 01") && !app.screen().shows("分页剧目 30"),
        "returning from player must preserve scrolled position, loaded pages, and 最近更新 sort"
    );
}
