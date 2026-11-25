use std::time::Duration;

use hg_contracts::ids::SeriesId;
use hg_gateway::contract::AppGateway as _;

use super::harness::E2eHarness;

#[test]
fn discovery_search_player_replies_seek_and_watchlist_persistence_flow() {
    let mut harness = E2eHarness::start().expect("start e2e harness");

    discover_and_search(&mut harness);
    verify_player_replies_and_loopback(&mut harness);
    seek_bookmark_and_verify_watchlist(&mut harness);

    let recorded = harness.upstream.calls();
    assert!(
        recorded.iter().any(|call| call.scope == "dynamic"
            && call.method == "GET"
            && call.path == "/api/home")
    );
    assert!(
        recorded
            .iter()
            .any(|call| call.scope == "playback" && call.method == "POST")
    );
}

fn discover_and_search(harness: &mut E2eHarness) {
    harness.app.wait_for("精选推荐").expect("home loaded");
    let feed = harness
        .block_on(harness.gateway.home_feed())
        .expect("query home feed through local gateway");
    assert!(
        !feed.latest.is_empty(),
        "home feed latest should not be empty"
    );
    for item in &feed.latest {
        assert!(
            item.cover_url.starts_with("http://127.0.0.1:") && item.cover_url.contains("/cover/"),
            "latest cover_url must route through loopback proxy, got {}",
            item.cover_url
        );
    }
    let first_cover = &feed.latest.first().expect("latest non-empty").cover_url;
    let cover_status = harness
        .fetch_status(first_cover)
        .expect("fetch proxied latest cover");
    assert_eq!(cover_status.as_u16(), 200);
    harness
        .app
        .wait_for("夜雨长灯")
        .expect("series 101 visible");
    harness
        .app
        .wait_for("星辰暗涌")
        .expect("series 102 visible");

    harness
        .app
        .press("搜索")
        .expect("open standalone search page");
    harness
        .app
        .wait_for("热搜")
        .expect("search page meta loaded");
    harness
        .app
        .type_text("搜索剧名、标签...", "夜雨")
        .expect("type search keyword");
    harness.app.press("执行搜索").expect("submit search");
    harness
        .app
        .wait_for("夜雨长灯")
        .expect("search matched 101");
    harness
        .app
        .wait_for_gone("星辰暗涌", Duration::from_secs(2))
        .expect("non-matching series filtered out");
}

fn verify_player_replies_and_loopback(harness: &mut E2eHarness) {
    harness
        .app
        .press("夜雨长灯 >> nth=0")
        .expect("open series 101 player");
    harness.app.wait_for("选集 (1/3)").expect("player loaded");
    harness.app.wait_for("评论 (22)").expect("comments loaded");
    harness.app.wait_for("林深见鹿").expect("comment visible");

    harness
        .app
        .wait_for("共 2 条回复")
        .expect("reply trigger visible");
    harness
        .app
        .press("共 2 条回复")
        .expect("open comment replies");
    harness
        .app
        .wait_for("全部回复")
        .expect("replies drawer opened");
    harness
        .app
        .wait_for("南风知意")
        .expect("first reply loaded");
    harness.app.wait_for("周游记").expect("second reply loaded");
    harness.app.press("关闭").expect("close replies drawer");
    harness
        .app
        .wait_for_gone("全部回复", Duration::from_secs(2))
        .expect("replies drawer closed");

    let playable_uri = harness
        .capture_active_playable_uri()
        .expect("capture loopback online uri");
    assert!(playable_uri.starts_with("http://127.0.0.1:"));
    assert!(playable_uri.contains("/online/"));
    harness
        .verify_hls_stream_ok(&playable_uri)
        .expect("online loopback HLS 200 OK");
}

fn seek_bookmark_and_verify_watchlist(harness: &mut E2eHarness) {
    harness
        .emit_video_progress(0, 90)
        .expect("report initial duration from native video surface");
    harness.app.press_key("right").expect("seek forward 5s");
    harness.app.press_key("right").expect("seek forward 10s");
    harness
        .app
        .press_key("space")
        .expect("pause to reveal player controls");
    harness
        .app
        .wait_for("text:/00:10/")
        .expect("sought time displayed");

    harness.app.press("追剧").expect("bookmark series");
    harness
        .app
        .wait_for("已追剧")
        .expect("bookmark state updated");
    harness
        .app
        .wait(Duration::from_millis(150))
        .expect("wait async persistence");

    harness.app.press("返回").expect("return from player");
    harness.app.press("追剧").expect("switch to watchlist tab");
    harness
        .app
        .wait_for("我的追剧")
        .expect("bookmarks section visible");
    harness
        .app
        .wait_for("夜雨长灯 >> nth=0")
        .expect("series persisted in watchlist");

    harness.app.press("历史").expect("switch to history tab");
    harness
        .app
        .wait_for("最近观看")
        .expect("watch history section visible");
    harness
        .app
        .wait_for("text:/00:10/")
        .expect("watch progress persisted in history");

    let series_id = SeriesId::try_from(101).expect("series 101");
    let history = harness
        .block_on(harness.gateway.watch_history())
        .expect("query watch history");
    let entry = history
        .iter()
        .find(|item| item.series_id == series_id)
        .expect("series 101 in watch history");
    assert_eq!(entry.last_episode.get(), 1);
    assert_eq!(entry.position_secs, 10);

    let bookmarks = harness
        .block_on(harness.gateway.bookmarks())
        .expect("query bookmarks");
    assert!(bookmarks.iter().any(|item| item.series_id == series_id));

    harness
        .app
        .press("夜雨长灯 >> nth=0")
        .expect("resume playback from watchlist");
    harness
        .app
        .wait_for("选集 (1/3)")
        .expect("resumed player loaded");
}
