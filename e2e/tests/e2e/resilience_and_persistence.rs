use std::time::Duration;

use hg_contracts::ids::{EpisodeNumber, SeriesId};
use hg_contracts::offline::EnqueueEpisodeDownloadsRequest;
use hg_contracts::viewing::AppAppearance;
use hg_gateway::contract::AppGateway as _;
use nsh_engine::{Os, Points, Size};

use super::harness::E2eHarness;

#[test]
fn dirty_upstream_total_episodes_fallback_sort_and_pagination_e2e() {
    let mut harness = E2eHarness::start().expect("start e2e harness");

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
        .press("查看精选推荐全部")
        .expect("open featured browse");
    harness.app.wait_for("最新").expect("sort tabs visible");
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
        "upstream sort=new must place 星辰暗涌 before 夜雨长灯: {texts:?}"
    );
    harness
        .app
        .scroll("精选推荐", Points(2400.0))
        .expect("scroll featured browse list to bottom");
    harness
        .app
        .wait_for("第2页剧目-25")
        .expect("upstream page 2 auto-loaded on scroll");

    harness.app.press("返回").expect("back to home");
    harness
        .app
        .press("夜雨长灯 >> nth=0")
        .expect("open series 101 player");
    harness.app.wait_for("选集 (1/3)").expect("player loaded");
    harness
        .app
        .wait_for("评论 (22)")
        .expect("upstream comments total visible");
    harness
        .app
        .scroll("评论 (22)", Points(2000.0))
        .expect("scroll comments to bottom");
    harness
        .app
        .wait_for("二页评论作者")
        .expect("upstream comments page 2 auto-loaded");
}

#[test]
fn cross_process_restart_persists_preferences_history_bookmarks_and_resume_e2e() {
    let mut harness = E2eHarness::start().expect("start e2e harness");
    seed_persistent_user_state(&mut harness);
    verify_hydrated_state_after_restart(&mut harness);
}

fn seed_persistent_user_state(harness: &mut E2eHarness) {
    harness.app.wait_for("精选推荐").expect("home loaded");

    harness.app.press("搜索").expect("open search page");
    harness.app.wait_for("热搜").expect("search meta loaded");
    harness
        .app
        .type_text("搜索剧名、标签...", "夜雨")
        .expect("type query");
    harness.app.press("执行搜索").expect("submit search");
    harness
        .app
        .wait_for("搜索历史")
        .expect("search history recorded");

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
    assert_eq!(
        harness
            .drain_download_queue()
            .expect("drain offline download"),
        1
    );

    harness.app.press("设置").expect("open settings");
    harness.app.wait_for("深色").expect("appearance options");
    harness.app.press("深色").expect("select dark appearance");
    harness.app.press("1.5×").expect("select 1.5x default rate");
    harness
        .app
        .press("收起侧边栏")
        .expect("collapse desktop sidebar");
    harness
        .app
        .wait(Duration::from_millis(150))
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
        .wait(Duration::from_millis(150))
        .expect("wait history and bookmark persistence");
}

fn verify_hydrated_state_after_restart(harness: &mut E2eHarness) {
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
        .expect("search history persisted");
    harness.app.wait_for("夜雨").expect("keyword persisted");

    harness.app.press("追剧").expect("open bookmarks page");
    harness
        .app
        .wait_for("夜雨长灯")
        .expect("bookmark persisted");

    harness.app.press("历史").expect("open history page");
    harness.app.wait_for("夜雨长灯").expect("history persisted");
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
