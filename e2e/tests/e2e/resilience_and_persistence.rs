use std::time::Duration;

use hg_contracts::ids::{EpisodeNumber, SeriesId};
use hg_contracts::offline::EnqueueEpisodeDownloadsRequest;
use hg_contracts::viewing::AppAppearance;
use hg_gateway::contract::AppGateway as _;
use nsh_engine::{Os, Points, Size};

use super::harness::E2eHarness;

#[test]
fn player_stability_no_premature_skip_autohide_and_comments_scroll_flow() {
    let mut harness = E2eHarness::start().expect("start e2e harness");
    verify_zero_total_fallback_and_no_premature_skip(&mut harness);
    verify_bookmark_red_style_and_tick_stability(&mut harness);
    verify_autohide_and_comments_infinite_scroll(&mut harness);
}

fn verify_zero_total_fallback_and_no_premature_skip(harness: &mut E2eHarness) {
    harness.app.wait_for("星辰暗涌").expect("home loaded");
    harness
        .app
        .press("星辰暗涌 >> nth=0")
        .expect("open series 102 with upstream total_episodes=0");
    harness
        .app
        .wait_for("选集 (1/2)")
        .expect("upstream total_episodes=0 must fall back to episodes.len()");
    harness.app.press("返回").expect("return home");
    harness.app.wait_for("夜雨长灯").expect("home visible");

    harness
        .app
        .press("夜雨长灯 >> nth=0")
        .expect("open series 101 player");
    harness.app.wait_for("选集 (1/3)").expect("player loaded");
    harness
        .app
        .wait_for("评论 (22)")
        .expect("comments header shows real pagination total");
    assert!(
        !harness.app.screen().shows("加载更多"),
        "comments must use infinite scroll without load-more button"
    );

    harness
        .emit_video_progress(0, 90)
        .expect("seed duration 90s");
    harness
        .emit_video_progress(87, 90)
        .expect("96.7% progress near tail must not prematurely skip");
    harness
        .app
        .wait(Duration::from_millis(150))
        .expect("wait progress report roundtrip");
    assert!(
        harness.app.screen().shows("选集 (1/3)"),
        "episode must not skip before real ended event"
    );

    harness
        .emit_video_ended()
        .expect("dispatch real playback ended event");
    harness
        .app
        .wait_for("选集 (2/3)")
        .expect("real ended event auto-advances to episode 2");

    harness.app.press("1").expect("switch back to episode 1");
    harness.app.wait_for("选集 (1/3)").expect("back on ep 1");
    harness
        .app
        .wait_for("评论 (22)")
        .expect("ep 1 comments loaded cleanly without race");
    harness
        .emit_video_progress(10, 90)
        .expect("start playback ticks");
}

fn verify_bookmark_red_style_and_tick_stability(harness: &mut E2eHarness) {
    let unbookmarked_bg = harness
        .app
        .screen()
        .view_style("role:button[追剧]")
        .expect("read unbookmarked button style");
    let unbookmarked_fg = harness
        .app
        .screen()
        .span_style("追剧")
        .expect("read unbookmarked text style");
    harness.app.press("追剧").expect("bookmark series 101");
    harness
        .app
        .wait_for("已追剧")
        .expect("bookmark toggled to active");
    let bookmarked_bg = harness
        .app
        .screen()
        .view_style("role:button[已追剧]")
        .expect("read bookmarked button style");
    let bookmarked_fg = harness
        .app
        .screen()
        .span_style("已追剧")
        .expect("read bookmarked text style");
    assert_ne!(
        bookmarked_bg, unbookmarked_bg,
        "bookmarked button must change to highlighted red brand background"
    );
    assert_ne!(
        bookmarked_fg.color, unbookmarked_fg.color,
        "bookmarked label must change to highlighted red brand foreground"
    );

    harness.app.hover("视频画面").expect("reveal controls");
    harness.app.wait_for("倍速").expect("speed trigger visible");
    harness.app.press("倍速").expect("open speed popover");
    harness.app.wait_for("1.5×").expect("speed popover visible");
    let speed_node_before = harness
        .app
        .screen()
        .id("1.5×")
        .expect("capture speed option NodeId");
    for sec in 11..=16 {
        harness
            .emit_video_progress(sec, 90)
            .expect("progress tick while popover open");
    }
    assert_eq!(
        harness
            .app
            .screen()
            .id("1.5×")
            .expect("speed option still mounted"),
        speed_node_before,
        "speed popover must not remount/flicker across 0.5s progress ticks"
    );
    harness
        .app
        .press("1.5×")
        .expect("select 1.5x speed in single press");
    harness
        .app
        .wait_for("1.5×")
        .expect("rate button reflects 1.5x");

    harness.app.press("展开 >> nth=0").expect("expand synopsis");
    harness
        .app
        .wait_for("收起 >> nth=0")
        .expect("synopsis expanded");
    let collapse_node_before = harness
        .app
        .screen()
        .id("收起 >> nth=0")
        .expect("capture collapse button NodeId");
    for sec in 17..=20 {
        harness
            .emit_video_progress(sec, 90)
            .expect("progress tick before collapse");
    }
    assert_eq!(
        harness
            .app
            .screen()
            .id("收起 >> nth=0")
            .expect("collapse button still mounted"),
        collapse_node_before,
        "expand/collapse button must not remount across 0.5s progress ticks"
    );
    harness
        .app
        .press("收起 >> nth=0")
        .expect("collapse synopsis in a single press during ticks");
    harness
        .app
        .wait_for("展开 >> nth=0")
        .expect("synopsis collapsed immediately");
}

fn verify_autohide_and_comments_infinite_scroll(harness: &mut E2eHarness) {
    harness
        .app
        .hover("视频画面")
        .expect("hover video area outside control bar");
    harness.app.wait_for("播放设置").expect("controls visible");
    for step in 0..8 {
        harness
            .emit_video_progress(21 + step, 90)
            .expect("progress tick during idle countdown");
        harness
            .app
            .wait(Duration::from_millis(450))
            .expect("wait 450ms");
    }
    harness
        .app
        .wait_for_gone("播放设置", Duration::from_secs(1))
        .expect("controls must auto-hide after 3s inactivity even while progress ticks fire");

    harness
        .app
        .scroll("评论 (22)", Points(2000.0))
        .expect("scroll comments list to bottom");
    harness
        .app
        .wait_for("二页评论作者")
        .expect("scrolling comments to bottom auto-loads page 2");
}

#[test]
fn navigation_search_history_sort_pagination_offline_and_restart_persistence_flow() {
    let mut harness = E2eHarness::start().expect("start e2e harness");
    verify_nav_and_standalone_search(&mut harness);
    verify_sort_scroll_and_adaptive_grid(&mut harness);
    verify_single_page_offline_and_save_settings(&mut harness);
    verify_restart_persistence_and_resume(&mut harness);
}

fn verify_nav_and_standalone_search(harness: &mut E2eHarness) {
    harness.app.wait_for("精选推荐").expect("home loaded");
    assert!(
        !harness.app.screen().shows("搜索剧名、标签..."),
        "discover home must not embed search input bar"
    );
    for tab in ["发现", "搜索", "追剧", "历史", "离线", "设置"] {
        assert!(harness.app.screen().shows(tab), "missing nav tab {tab}");
    }

    harness
        .app
        .press("搜索")
        .expect("open standalone search page");
    harness
        .app
        .wait_for("搜索剧名、标签...")
        .expect("search input visible on search page");
    harness.app.wait_for("热搜").expect("hot keywords visible");
    harness.app.wait_for("悬疑").expect("tags visible");
    assert!(
        !harness.app.screen().shows("夜雨长灯"),
        "empty search page must not dump unrelated series before query"
    );

    harness
        .app
        .type_text("搜索剧名、标签...", "夜雨")
        .expect("type query");
    harness.app.press("执行搜索").expect("submit search");
    harness
        .app
        .wait_for("夜雨长灯")
        .expect("search result visible");
    harness
        .app
        .wait_for("搜索历史")
        .expect("search history strip visible after search");
}

fn verify_sort_scroll_and_adaptive_grid(harness: &mut E2eHarness) {
    harness.app.press("发现").expect("back to discover");
    harness
        .app
        .press("查看精选推荐全部")
        .expect("open featured browse");
    harness.app.wait_for("最热").expect("sort tabs visible");
    harness
        .app
        .wait_for("最新")
        .expect("sort by new tab visible");
    assert!(
        !harness.app.screen().shows("加载更多"),
        "browse list must use pure infinite scroll without load-more button"
    );

    harness.app.press("最新").expect("sort by latest");
    harness
        .app
        .wait(Duration::from_millis(150))
        .expect("wait latest sort load");
    let texts = harness.app.screen().texts();
    let idx_102 = texts.iter().position(|t| *t == "星辰暗涌");
    let idx_101 = texts.iter().position(|t| *t == "夜雨长灯");
    assert!(
        idx_102 < idx_101,
        "sort=new must place 星辰暗涌 before 夜雨长灯: {texts:?}"
    );

    harness
        .app
        .scroll("精选推荐", Points(2400.0))
        .expect("scroll featured browse list to bottom");
    harness
        .app
        .wait_for("第2页剧目-25")
        .expect("infinite scroll must auto-load page 2");

    harness
        .app
        .set_viewport(Size::new(Points(920.0), Points(800.0)))
        .expect("resize narrow viewport");
    harness
        .app
        .wait(Duration::from_millis(80))
        .expect("settle narrow grid layout");
    let narrow_row_count = first_row_card_count(&harness.app, "第1页剧目-");
    harness
        .app
        .set_viewport(Size::new(Points(1640.0), Points(800.0)))
        .expect("resize wide viewport");
    harness
        .app
        .wait(Duration::from_millis(80))
        .expect("settle wide grid layout");
    let wide_row_count = first_row_card_count(&harness.app, "第1页剧目-");
    assert!(
        wide_row_count > narrow_row_count,
        "wider viewport must show more poster columns per row (narrow={narrow_row_count}, wide={wide_row_count})"
    );
    harness
        .app
        .set_viewport(Size::new(Points(1240.0), Points(800.0)))
        .expect("restore default viewport");
}

fn verify_single_page_offline_and_save_settings(harness: &mut E2eHarness) {
    let series_id = SeriesId::try_from(101).expect("series 101");
    let ep1 = EpisodeNumber::new(std::num::NonZeroU32::MIN);
    harness
        .block_on(
            harness
                .gateway
                .enqueue_episode_downloads(EnqueueEpisodeDownloadsRequest {
                    series_id,
                    episodes: vec![ep1],
                }),
        )
        .expect("enqueue download");

    harness.app.press("离线").expect("open single-page offline");
    harness
        .app
        .wait_for("下载中")
        .expect("active queue section visible at top");
    harness
        .app
        .wait_for("已下载")
        .expect("downloaded section visible below");
    let completed = harness
        .drain_download_queue()
        .expect("drain offline download");
    assert_eq!(completed, 1);

    harness.app.press("设置").expect("open settings");
    harness.app.wait_for("深色").expect("appearance options");
    harness
        .app
        .wait_for("仅 Wi-Fi 下载")
        .expect("download policy loaded");
    assert_download_policy_rows_aligned(&harness.app);
    harness.app.press("深色").expect("select dark appearance");
    harness.app.press("1.5×").expect("select 1.5x default rate");
    harness
        .app
        .press("收起侧边栏")
        .expect("collapse desktop sidebar");
    harness
        .app
        .wait(Duration::from_millis(200))
        .expect("wait preference persistence");

    harness.app.press("发现").expect("navigate to discover");
    harness
        .app
        .press("夜雨长灯 >> nth=0")
        .expect("open series 101");
    harness.app.wait_for("选集 (1/3)").expect("player loaded");
    harness.app.press("2").expect("switch to ep 2");
    harness.app.wait_for("选集 (2/3)").expect("ep 2 loaded");
    harness
        .emit_video_progress(42, 95)
        .expect("record ep 2 progress at 42s");
    harness.app.press("追剧").expect("bookmark series 101");
    harness.app.wait_for("已追剧").expect("bookmarked");
    harness
        .app
        .wait(Duration::from_millis(200))
        .expect("wait history & bookmark persistence");
}

fn verify_restart_persistence_and_resume(harness: &mut E2eHarness) {
    harness.restart().expect("restart app on same sqlite");
    harness
        .app
        .wait_for("精选推荐")
        .expect("restarted app rendered home");
    harness
        .app
        .wait_for("展开侧边栏")
        .expect("collapsed sidebar state hydrated across restart");

    let persisted = harness
        .block_on(harness.gateway.playback_preference())
        .expect("read persisted preference");
    assert_eq!(persisted.appearance, AppAppearance::Dark);
    assert_eq!(persisted.default_rate_hundredths, 150);
    assert!(persisted.sidebar_collapsed);

    harness
        .app
        .press("搜索")
        .expect("open search after restart");
    harness
        .app
        .wait_for("搜索历史")
        .expect("search history persisted across restart");
    harness
        .app
        .wait_for("夜雨")
        .expect("keyword persisted across restart");

    harness.app.press("追剧").expect("open bookmarks page");
    harness
        .app
        .wait_for("夜雨长灯")
        .expect("bookmark persisted on standalone watchlist page");

    harness
        .app
        .press("历史")
        .expect("open standalone history page");
    harness
        .app
        .wait_for("夜雨长灯")
        .expect("watch history persisted on standalone history page");
    harness
        .app
        .wait_for("text:/00:42/")
        .expect("saved position shown in history");

    harness
        .app
        .press("夜雨长灯 >> nth=0")
        .expect("resume watched series after restart");
    harness
        .app
        .wait_for("选集 (2/3)")
        .expect("resumes saved episode 2");
    harness.app.hover("视频画面").expect("reveal controls");
    harness
        .app
        .wait_for("1.5×")
        .expect("applies persisted 1.5x default rate on player open");
    harness
        .app
        .wait_for("text:/00:42/")
        .expect("resumes saved 42s position");
}

#[test]
fn mobile_android_drawer_and_system_back_exits_player_flow() {
    let mut harness =
        E2eHarness::start_with_platform(Os::Android, Size::new(Points(390.0), Points(844.0)))
            .expect("start android e2e harness");
    harness
        .app
        .wait_for("精选推荐")
        .expect("android home loaded");

    harness
        .app
        .press("打开导航菜单")
        .expect("open mobile slide-out drawer");
    harness.app.wait_for("历史").expect("drawer shows history");
    harness.app.wait_for("离线").expect("drawer shows offline");
    harness
        .app
        .back()
        .expect("android back button closes drawer");
    harness
        .app
        .wait_for_gone("历史", Duration::from_secs(2))
        .expect("drawer closed on android back");

    harness
        .app
        .press("夜雨长灯 >> nth=0")
        .expect("open player on android");
    harness
        .app
        .wait_for("text:/夜雨长灯/")
        .expect("android player opened");

    harness
        .app
        .back()
        .expect("android system back button exits player");
    harness
        .app
        .wait_for("精选推荐")
        .expect("returned to discover home after android back");
}

fn first_row_card_count(app: &nsh_testing::Headless, prefix: &str) -> usize {
    let nodes = app.screen().layout_nodes();
    let mut ys = Vec::new();
    for idx in 3..=24 {
        let selector = format!("{prefix}{idx}");
        if let Ok(id) = app.screen().id(&selector)
            && let Some(node) = nodes.iter().find(|n| n.id == id)
        {
            ys.push(node.bounds.origin.y.0);
        }
    }
    let Some(min_y) = ys.iter().copied().reduce(f32::min) else {
        return 0;
    };
    ys.into_iter().filter(|y| (y - min_y).abs() <= 4.0).count()
}

fn assert_download_policy_rows_aligned(app: &nsh_testing::Headless) {
    let nodes = app.screen().layout_nodes();
    let id1 = app
        .screen()
        .id("text:仅 Wi-Fi 下载")
        .expect("find wifi-only label node");
    let id2 = app
        .screen()
        .id("text:最大并发下载任务数")
        .expect("find max-concurrent label node");
    let row1 = nodes
        .iter()
        .find(|node| node.id == id1)
        .expect("find wifi-only layout node");
    let row2 = nodes
        .iter()
        .find(|node| node.id == id2)
        .expect("find max-concurrent layout node");
    assert!(
        (row1.bounds.origin.x.0 - row2.bounds.origin.x.0).abs() <= 2.0,
        "download policy row labels must share left x alignment (row1={}, row2={})",
        row1.bounds.origin.x.0,
        row2.bounds.origin.x.0
    );
    assert!(
        (row1.bounds.size.height.0 - row2.bounds.size.height.0).abs() <= 2.0,
        "download policy row title typography heights must match (row1={}, row2={})",
        row1.bounds.size.height.0,
        row2.bounds.size.height.0
    );
}
