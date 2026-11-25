use std::num::{NonZeroU16, NonZeroU32};

use hg_contracts::catalog::{ListSeriesRequest, SeriesFilter, SeriesSort};
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, PageRequest};
use hg_gateway::contract::AppGateway as _;
use hg_gateway::local::{LocalGateway, LocalGatewayConfig};

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

fn page_req(page: u32, size: u16) -> PageRequest {
    PageRequest::new(
        NonZeroU32::new(page).unwrap_or(NonZeroU32::MIN),
        NonZeroU16::new(size).unwrap_or(NonZeroU16::MIN),
    )
}

#[test]
#[ignore = "live upstream smoke test"]
fn live_upstream_catalog_detail_and_playback_smoke() {
    let base_url = resolve_live_base_url();
    let temp = tempfile::tempdir().expect("create live smoke tempdir");
    let data_root = temp.path().join("data");
    let config = LocalGatewayConfig {
        db_path: data_root.join("hg.sqlite"),
        data_root,
        cache_root: temp.path().join("cache"),
        upstream_base_url: base_url.clone(),
    };
    let gateway = LocalGateway::open_with_config(&config)
        .unwrap_or_else(|err| panic!("初始化实网 LocalGateway 失败（base_url={base_url}）: {err}"));
    let rt = tokio::runtime::Runtime::new().expect("create tokio runtime");

    let feed = rt
        .block_on(gateway.home_feed())
        .unwrap_or_else(|err| panic!("实网 home_feed 失败（base_url={base_url}）: {err}"));
    assert!(
        !feed.featured.is_empty(),
        "实网精选推荐不应为空（base_url={base_url}）"
    );

    for summary in &feed.featured {
        let detail = rt
            .block_on(gateway.series_detail(summary.id))
            .unwrap_or_else(|err| panic!("实网剧目详情 {} 失败: {err}", summary.id));
        assert!(
            !detail.episodes.is_empty(),
            "实网剧目 {} 分集列表不应为空",
            summary.id
        );
        assert!(
            detail.total_episodes as usize >= detail.episodes.len(),
            "实网剧目 {} total_episodes={} 小于 episodes.len()={}",
            summary.id,
            detail.total_episodes,
            detail.episodes.len()
        );
    }

    for sort in [SeriesSort::Hot, SeriesSort::New] {
        let page1 = rt
            .block_on(gateway.list_series(ListSeriesRequest {
                filter: SeriesFilter::All { category_id: None },
                sort,
                page: page_req(1, 24),
            }))
            .unwrap_or_else(|err| panic!("实网列表 {sort:?} 第1页失败: {err}"));
        let page2 = rt
            .block_on(gateway.list_series(ListSeriesRequest {
                filter: SeriesFilter::All { category_id: None },
                sort,
                page: page_req(2, 24),
            }))
            .unwrap_or_else(|err| panic!("实网列表 {sort:?} 第2页失败: {err}"));
        assert!(!page1.items.is_empty(), "实网列表 {sort:?} 第1页为空");
        assert!(!page2.items.is_empty(), "实网列表 {sort:?} 第2页为空");
        assert_ne!(
            page1.items.first().map(|item| item.id),
            page2.items.first().map(|item| item.id),
            "实网列表 {sort:?} 第1页与第2页首项重复"
        );
    }

    let first_id = feed.featured.first().expect("featured non-empty").id;
    let playback = rt
        .block_on(gateway.prepare_episode_playback(EpisodeKey::new(
            first_id,
            EpisodeNumber::new(NonZeroU32::MIN),
        )))
        .unwrap_or_else(|err| panic!("实网起播 {first_id} 第1集失败: {err}"));
    assert!(
        playback.playable_uri.starts_with("http://127.0.0.1:"),
        "实网起播应走回环代理: {}",
        playback.playable_uri
    );
}
