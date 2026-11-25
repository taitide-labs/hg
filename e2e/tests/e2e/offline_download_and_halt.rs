use std::time::Duration;

use hg_contracts::ids::SeriesId;
use hg_gateway::contract::AppGateway as _;
use reqwest::StatusCode;

use super::harness::E2eHarness;

#[test]
fn offline_batch_download_offline_first_playback_and_delete_halt_flow() {
    let mut harness = E2eHarness::start().expect("start e2e harness");

    harness.app.wait_for("夜雨长灯").expect("home loaded");
    harness
        .app
        .press("夜雨长灯 >> nth=0")
        .expect("open series 101 player");
    harness.app.wait_for("选集 (1/3)").expect("player loaded");

    harness
        .app
        .press("离线下载")
        .expect("open batch download drawer");
    harness
        .app
        .wait_for("选择离线下载分集")
        .expect("batch download drawer opened");
    harness
        .app
        .press("全选当前段")
        .expect("select all episodes");
    harness
        .app
        .press("加入离线队列")
        .expect("enqueue offline downloads");
    harness
        .app
        .wait_for("text:/已加入离线下载/")
        .expect("enqueue feedback displayed");

    let completed = harness
        .drain_download_queue()
        .expect("drain offline download queue");
    assert_eq!(completed, 3);

    let series_id = SeriesId::try_from(101).expect("series 101");
    let library = harness
        .block_on(harness.gateway.offline_library())
        .expect("query offline library");
    let offline_series = library
        .iter()
        .find(|item| item.series_id == series_id)
        .expect("series 101 committed to offline library");
    assert_eq!(offline_series.episodes.len(), 3);

    harness.app.press("返回").expect("return from player");
    harness.app.press("离线").expect("switch to offline tab");
    harness
        .app
        .wait_for("托管离线片库")
        .expect("offline library header visible");
    harness
        .app
        .wait_for("夜雨长灯")
        .expect("downloaded series visible in offline library");
    harness
        .app
        .wait_for("text:/3 集/")
        .expect("downloaded episode count visible");

    harness.upstream.set_offline(true);
    let upstream_calls_before = harness.upstream.call_count();

    harness
        .app
        .press("夜雨长灯 >> nth=0")
        .expect("open offline series playback while upstream down");
    harness
        .app
        .wait_for("选集 (1/3)")
        .expect("offline player loaded without upstream");
    assert_eq!(harness.upstream.call_count(), upstream_calls_before);

    let offline_uri = harness
        .capture_active_playable_uri()
        .expect("capture offline loopback uri");
    assert!(offline_uri.starts_with("http://127.0.0.1:"));
    assert!(offline_uri.contains("/offline/"));
    harness
        .verify_hls_stream_ok(&offline_uri)
        .expect("offline loopback HLS 200 OK while upstream down");

    let series_dir = harness
        .gateway
        .runtime()
        .storage()
        .library_series_dir(series_id);
    assert!(series_dir.exists());

    harness
        .app
        .press("返回")
        .expect("return to offline library");
    harness
        .app
        .wait_for("托管离线片库")
        .expect("back on offline library");
    harness
        .app
        .press("删除 夜雨长灯 离线剧目")
        .expect("delete offline series");
    harness
        .app
        .wait_for_gone("夜雨长灯", Duration::from_secs(2))
        .expect("deleted series removed from offline library");

    let halted_status = harness
        .fetch_status(&offline_uri)
        .expect("probe halted loopback uri");
    assert_eq!(halted_status, StatusCode::GONE);
    assert!(!series_dir.exists());
}
