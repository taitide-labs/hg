use std::sync::Arc;

use hg_runtime::catalog::upstream::{CatalogUpstream as _, EdgeCatalogUpstream};
use hg_runtime::infra::clock::SystemClock;
use hg_runtime::infra::edge_crypto::{EdgeChannelScope, EdgeHttpClient};
use serde_json::{Value, json};

#[expect(
    clippy::disallowed_methods,
    reason = "实网冒烟测试按环境变量可选覆盖上游基地址"
)]
fn resolve_live_base_url() -> String {
    std::env::var("HG_UPSTREAM_BASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| hg_gateway::local::DEFAULT_UPSTREAM_BASE_URL.to_owned())
}

fn extract_first_series_id(home_json: &Value) -> Option<u64> {
    let data = home_json.get("data")?;
    for key in ["featured", "banners"] {
        if let Some(id) = data
            .get(key)
            .and_then(Value::as_array)
            .and_then(|arr| arr.first())
            .and_then(|item| {
                item.get("id")
                    .or_else(|| item.get("video_id"))
                    .or_else(|| item.get("series_id"))
            })
            .and_then(Value::as_u64)
        {
            return Some(id);
        }
    }
    data.get("cate_rec")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|rec| rec.get("videos").or_else(|| rec.get("items")))
        .filter_map(Value::as_array)
        .flatten()
        .find_map(|item| {
            item.get("id")
                .or_else(|| item.get("video_id"))
                .or_else(|| item.get("series_id"))
                .and_then(Value::as_u64)
        })
}

#[tokio::test]
#[ignore = "live upstream smoke test"]
async fn live_upstream_catalog_detail_and_playback_smoke() {
    let base_url = resolve_live_base_url();
    let clock = Arc::new(SystemClock);
    let edge_client = EdgeHttpClient::new(&base_url, clock)
        .unwrap_or_else(|err| panic!("初始化实网边缘客户端失败（base_url={base_url}）: {err}"));
    let upstream = EdgeCatalogUpstream::new(edge_client);

    let home = upstream
        .request_json(EdgeChannelScope::Dynamic, "GET", "/api/home", None)
        .await
        .unwrap_or_else(|err| {
            panic!(
                "实网冒烟请求 /api/home 失败（base_url={base_url}，若当前环境无外网连通性或上游限流属外部环境诊断）: {err}"
            )
        });

    let series_id = extract_first_series_id(&home).unwrap_or_else(|| {
        panic!("实网 /api/home 响应未包含可用剧目 ID: {home}");
    });

    let detail_path = format!("/api/videos/{series_id}");
    let detail = upstream
        .request_json(EdgeChannelScope::Dynamic, "GET", &detail_path, None)
        .await
        .unwrap_or_else(|err| {
            panic!("实网冒烟请求 {detail_path} 失败（base_url={base_url}）: {err}")
        });
    assert!(
        detail.get("data").and_then(|d| d.get("title")).is_some(),
        "实网剧目详情缺少 title 字段: {detail}"
    );

    let playback_path = format!("/api/videos/{series_id}/episodes/1/playback");
    let playback = upstream
        .request_json(
            EdgeChannelScope::Playback,
            "POST",
            &playback_path,
            Some(&json!({})),
        )
        .await
        .unwrap_or_else(|err| {
            panic!("实网冒烟请求 POST {playback_path} 失败（base_url={base_url}）: {err}")
        });

    let has_stream = playback
        .get("data")
        .and_then(|d| d.get("streams"))
        .and_then(Value::as_object)
        .is_some_and(|streams| {
            ["hevc", "main", "preview"].into_iter().any(|k| {
                streams
                    .get(k)
                    .and_then(Value::as_str)
                    .is_some_and(|url| !url.trim().is_empty())
            })
        });
    assert!(has_stream, "实网 playback 响应缺少有效流地址: {playback}");
}
