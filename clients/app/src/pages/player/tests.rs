use std::time::Duration;

use nsh_engine::{ColorScheme, EngineConfig, Environment, Os, Points, Root, Size};
use nsh_testing::Headless;

use crate::App;

fn start_and_open_first_series(scheme: ColorScheme) -> Headless {
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
    app.press("云端之上：千金归来 >> nth=0").unwrap();
    app.wait_for("选集 (1/68)").unwrap();
    app
}

#[test]
fn desktop_player_shows_clean_single_column_feed_without_mode_or_ratio_clutter() {
    let app = start_and_open_first_series(ColorScheme::Light);
    let screen = app.screen();

    assert!(screen.shows("云端之上：千金归来"));
    assert!(screen.shows("选集 (1/68)"));
    assert!(screen.shows("评论 (2)"));
    assert!(screen.shows("林深见鹿"));
    assert!(screen.shows("共 2 条回复"));
    assert!(screen.shows("离线下载"));

    assert!(!screen.shows("电影模式"));
    assert!(!screen.shows("短剧模式"));
    assert!(!screen.shows("正在播放"));
    assert!(!screen.shows("比例 1"));
    assert!(!screen.shows("完播并连播"));
}

#[test]
fn synopsis_and_episodes_fold_by_default_and_expand_in_place() {
    let mut app = start_and_open_first_series(ColorScheme::Light);

    assert!(!app.screen().shows("揭开当年真相"));
    app.press("展开简介").unwrap();
    app.wait_for("text:/揭开当年真相/").unwrap();
    app.press("收起简介").unwrap();
    app.wait_for_gone("text:/揭开当年真相/", Duration::from_secs(2))
        .unwrap();

    assert!(!app.screen().shows("51-68"));
    app.press("展开选集").unwrap();
    app.wait_for("51-68").unwrap();

    app.press("51-68").unwrap();
    app.wait_for("68").unwrap();
    app.press("68").unwrap();
    app.wait_for("选集 (68/68)").unwrap();

    app.press("收起选集").unwrap();
    app.wait_for_gone("51-68", Duration::from_secs(2)).unwrap();
    assert!(app.screen().shows("61"));
    assert!(app.screen().shows("68"));
}

#[test]
fn in_video_bottom_right_settings_and_speed_popovers_work() {
    let mut app = start_and_open_first_series(ColorScheme::Dark);

    assert!(!app.screen().shows("倍速"));
    app.hover("视频画面").unwrap();
    app.wait_for("倍速").unwrap();

    app.press("倍速").unwrap();
    app.wait_for("1.5×").unwrap();
    app.press("1.5×").unwrap();
    app.wait_for("1.5×").unwrap();

    app.press("播放设置").unwrap();
    app.wait_for("自动连播").unwrap();
    app.wait_for("单集循环").unwrap();
    app.wait_for("外部播放器打开").unwrap();

    app.press("自动连播").unwrap();
    app.press("外部播放器打开").unwrap();
    app.wait_for("text:/已准备外部播放地址/").unwrap();
}

#[test]
fn true_fullscreen_and_vertical_view_hide_chrome_and_restore_cleanly() {
    let mut app = start_and_open_first_series(ColorScheme::Light);

    app.hover("视频画面").unwrap();
    app.wait_for("倍速").unwrap();
    app.press("全屏").unwrap();
    app.wait_for_gone("选集 (1/68)", Duration::from_secs(2))
        .unwrap();
    app.wait_for_gone("离线下载", Duration::from_secs(2))
        .unwrap();

    app.press_key("enter").unwrap();
    app.wait_for("选集 (1/68)").unwrap();

    app.hover("视频画面").unwrap();
    app.press("竖屏视图").unwrap();
    app.wait_for("详情视图").unwrap();
    app.press("详情视图").unwrap();
    app.wait_for("选集 (1/68)").unwrap();
}

#[test]
fn bookmark_and_offline_download_actions_update_state() {
    let mut app = start_and_open_first_series(ColorScheme::Light);

    app.wait_for("已追剧").unwrap();
    app.press("已追剧").unwrap();
    app.wait_for("追剧").unwrap();
    app.press("追剧").unwrap();
    app.wait_for("已追剧").unwrap();

    app.press("离线下载").unwrap();
    app.wait_for("第 1 集已在离线片库中").unwrap();

    app.press("5").unwrap();
    app.wait_for("选集 (5/68)").unwrap();
    app.press("离线下载").unwrap();
    app.wait_for("第 5 集已加入离线下载").unwrap();
}

#[test]
#[ignore = "待 player 进阶切片实现并启用 PLAYER_ENHANCEMENTS 开关后激活"]
fn expanding_comment_replies_and_batch_downloading_episodes_from_player() {
    let mut app = start_and_open_first_series(ColorScheme::Light);

    app.wait_for("共 2 条回复").unwrap();
    app.press("共 2 条回复").unwrap();
    app.wait_for("南风知意").unwrap();
    app.wait_for("周游记").unwrap();

    app.press("离线下载").unwrap();
    app.wait_for("选择离线下载分集").unwrap();
    app.press("全选当前段").unwrap();
    app.press("加入离线队列").unwrap();
    app.wait_for("text:/已加入离线下载/").unwrap();
}
