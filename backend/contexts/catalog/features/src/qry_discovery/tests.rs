use std::collections::HashMap;
use std::num::{NonZeroU16, NonZeroU32};
use std::sync::Arc;

use hg_backend_contracts::error::{PortError, PortFuture};
use hg_contracts::catalog::{
    ListRankingRequest, ListSeriesRequest, RankingKind, SearchSeriesRequest, SeriesFilter,
    SeriesSort,
};
use hg_contracts::error::ContractError;
use hg_contracts::ids::{PageRequest, SeriesId};
use hg_infra::clock::FixedClock;
use hg_infra::db::LocalDatabase;
use hg_infra::edge_crypto::EdgeChannelScope;
use hg_infra::storage::StorageLayout;
use parking_lot::Mutex;
use serde_json::{Value, json};
use tempfile::TempDir;

use crate::CatalogDeps;
use crate::upstream::CatalogUpstream;

use super::codec::{
    extract_data, parse_categories, parse_home_feed, parse_hot_keywords, parse_hot_tags,
    parse_paged_series, parse_ranking, parse_search_suggestions, parse_tag_groups,
};
use super::handler::{
    categories, hot_keywords, hot_tags, list_ranking, list_series, search_suggestions, tag_groups,
};
use super::query::{
    CATALOG_LIST_CACHE_TTL_SECS, STATIC_META_CACHE_TTL_SECS, categories_route, home_feed_route,
    hot_keywords_route, hot_tags_route, list_ranking_route, list_series_route, percent_encode,
    search_series_route, search_suggestions_route, tag_groups_route,
};

#[derive(Default)]
struct StubUpstream {
    routes: HashMap<String, Value>,
    calls: Mutex<Vec<(EdgeChannelScope, String)>>,
}

impl StubUpstream {
    fn with_route(mut self, path: impl Into<String>, body: Value) -> Self {
        self.routes.insert(path.into(), body);
        self
    }

    fn calls(&self) -> Vec<(EdgeChannelScope, String)> {
        self.calls.lock().clone()
    }
}

impl CatalogUpstream for StubUpstream {
    fn request_json<'a>(
        &'a self,
        scope: EdgeChannelScope,
        _method: &'a str,
        path_with_query: &'a str,
        _body: Option<&'a Value>,
    ) -> PortFuture<'a, Result<Value, PortError>> {
        self.calls.lock().push((scope, path_with_query.to_owned()));
        let found = self.routes.get(path_with_query).cloned();
        Box::pin(async move {
            found.ok_or_else(|| PortError::NotFound {
                resource: path_with_query.to_owned(),
            })
        })
    }

    fn fetch_raw_cover<'a>(
        &'a self,
        cover_url: &'a str,
    ) -> PortFuture<'a, Result<Vec<u8>, PortError>> {
        let url = cover_url.to_owned();
        Box::pin(async move { Err(PortError::NotFound { resource: url }) })
    }
}

struct TestEnv {
    _temp_dir: TempDir,
    clock: Arc<FixedClock>,
    upstream: Arc<StubUpstream>,
    deps: CatalogDeps,
}

impl TestEnv {
    async fn new(upstream: StubUpstream) -> Self {
        let temp_dir = tempfile::tempdir().expect("tempdir should create");
        let storage = Arc::new(
            StorageLayout::new(
                &temp_dir.path().join("data"),
                &temp_dir.path().join("cache"),
            )
            .expect("storage layout"),
        );
        storage.ensure_dirs().await.expect("ensure dirs");
        let db = LocalDatabase::open_in_memory()
            .await
            .expect("in-memory sqlite");
        let clock = Arc::new(FixedClock::new(1_700_000_000));
        let upstream = Arc::new(upstream);
        let deps = CatalogDeps {
            db,
            storage,
            clock: Arc::clone(&clock) as _,
            upstream: Arc::clone(&upstream) as _,
        };
        Self {
            _temp_dir: temp_dir,
            clock,
            upstream,
            deps,
        }
    }
}

fn page_req(page: u32, size: u16) -> PageRequest {
    PageRequest::new(
        NonZeroU32::new(page).expect("non-zero page"),
        NonZeroU16::new(size).expect("non-zero size"),
    )
}

#[test]
fn static_routes_use_public_scope_and_expected_ttls() {
    assert_eq!(home_feed_route().path, "/api/home");
    assert_eq!(home_feed_route().scope, EdgeChannelScope::Dynamic);
    assert_eq!(
        home_feed_route().ttl_secs,
        Some(CATALOG_LIST_CACHE_TTL_SECS)
    );
    assert_eq!(categories_route().path, "/api/site/categories/video");
    assert_eq!(
        categories_route().ttl_secs,
        Some(STATIC_META_CACHE_TTL_SECS)
    );
    assert_eq!(tag_groups_route().path, "/api/tags");
    assert_eq!(
        tag_groups_route().ttl_secs,
        Some(STATIC_META_CACHE_TTL_SECS)
    );
    let rank_route = list_ranking_route(ListRankingRequest {
        kind: RankingKind::Potential,
        page: page_req(1, 15),
    });
    assert_eq!(rank_route.path, "/api/ranks/potential?page=1&size=15");
    assert_eq!(rank_route.scope, EdgeChannelScope::Public);
}

#[test]
fn series_and_search_routes_assign_paths_and_dynamic_scopes() {
    let all_hot = list_series_route(&ListSeriesRequest {
        filter: SeriesFilter::All { category_id: None },
        sort: SeriesSort::Hot,
        page: page_req(2, 24),
    })
    .expect("all hot route");
    assert_eq!(all_hot.path, "/api/videos?page=2&size=24&sort=hot");
    assert_eq!(all_hot.scope, EdgeChannelScope::Public);

    let cate_window = list_series_route(&ListSeriesRequest {
        filter: SeriesFilter::All {
            category_id: Some(25),
        },
        sort: SeriesSort::HotWindow,
        page: page_req(1, 20),
    })
    .expect("cate hot_window route");
    assert_eq!(
        cate_window.path,
        "/api/videos?cate_id=25&page=1&size=20&sort=hot_window"
    );

    let random_route = list_series_route(&ListSeriesRequest {
        filter: SeriesFilter::TagSlug {
            slug: "都市".to_owned(),
        },
        sort: SeriesSort::Random,
        page: page_req(1, 10),
    })
    .expect("random tag route");
    assert_eq!(
        random_route.path,
        "/api/videos/tag/%E9%83%BD%E5%B8%82?page=1&size=10&sort=random"
    );
    assert_eq!(random_route.scope, EdgeChannelScope::Dynamic);
    assert_eq!(random_route.ttl_secs, None);

    let cate_route = list_series_route(&ListSeriesRequest {
        filter: SeriesFilter::CategoryRoute {
            route: "manju".to_owned(),
        },
        sort: SeriesSort::New,
        page: page_req(1, 20),
    })
    .expect("category route");
    assert_eq!(
        cate_route.path,
        "/api/videos/category/manju?page=1&size=20&sort=new"
    );

    let search = search_series_route(&SearchSeriesRequest {
        keyword: "长安".to_owned(),
        page: page_req(1, 20),
    })
    .expect("search route");
    assert_eq!(
        search.path,
        "/api/search?page=1&q=%E9%95%BF%E5%AE%89&size=20"
    );
    assert_eq!(search.scope, EdgeChannelScope::Dynamic);
    assert_eq!(search_suggestions_route("   "), None);
    assert_eq!(
        search_suggestions_route("夜雨").expect("suggest").path,
        "/api/search/suggest?q=%E5%A4%9C%E9%9B%A8&type=video"
    );
    assert_eq!(hot_keywords_route().scope, EdgeChannelScope::Public);
    assert_eq!(hot_tags_route().scope, EdgeChannelScope::Public);
    assert_eq!(percent_encode("a-b_c.d~1 "), "a-b_c.d~1%20");
}

#[test]
fn route_builders_reject_empty_inputs() {
    let empty_route = list_series_route(&ListSeriesRequest {
        filter: SeriesFilter::CategoryRoute {
            route: "   ".to_owned(),
        },
        sort: SeriesSort::Hot,
        page: page_req(1, 20),
    })
    .expect_err("blank route should fail");
    assert!(matches!(empty_route, ContractError::InvalidInput { .. }));

    let empty_slug = list_series_route(&ListSeriesRequest {
        filter: SeriesFilter::TagSlug {
            slug: String::new(),
        },
        sort: SeriesSort::Hot,
        page: page_req(1, 20),
    })
    .expect_err("blank slug should fail");
    assert!(matches!(empty_slug, ContractError::InvalidInput { .. }));

    let empty_kw = search_series_route(&SearchSeriesRequest {
        keyword: " \t ".to_owned(),
        page: page_req(1, 20),
    })
    .expect_err("blank keyword should fail");
    assert!(matches!(empty_kw, ContractError::InvalidInput { .. }));
}

#[test]
fn codec_parses_home_feed_and_normalizes_urls() {
    let home_data = extract_data(
        json!({
            "status": 1,
            "data": {
                "banners": [{
                    "id": 1,
                    "video_id": 1001,
                    "title": "风云再起",
                    "image": "/upload_01/banner.jpg",
                    "score": "9.4",
                    "desc": "热播"
                }],
                "featured": [{
                    "id": "1002",
                    "title": "暗夜行",
                    "cover": "https://cdn.example.com/1002.jpg?auth_key=abc",
                    "hot": 5000,
                    "score": 9.1,
                    "tags": ["古风"],
                    "desc": "权谋",
                    "badge": "",
                    "episode_count": 30,
                    "is_finished": 1
                }],
                "cate_rec": [{
                    "cate_id": 24,
                    "cate_name": "短剧",
                    "videos": [{
                        "id": 1003,
                        "title": "协议",
                        "cover": "//img.example.com/1003.jpg",
                        "total_episodes": 20,
                        "is_finished": false
                    }]
                }]
            }
        }),
        "首页",
    )
    .expect("home data");
    let feed = parse_home_feed(home_data).expect("parse home");
    assert_eq!(
        feed.banners.first().expect("banner").image_url,
        "https://pic.qnlycc.cn/upload_01/banner.jpg"
    );
    let featured = feed.featured.first().expect("featured");
    assert_eq!(
        featured.cover_url,
        "https://cdn.example.com/1002.jpg?auth_key=abc"
    );
    assert_eq!(featured.badge, None);
    assert!(featured.is_finished);
    let rec_item = feed
        .category_recommendations
        .first()
        .expect("cate rec")
        .items
        .first()
        .expect("rec item");
    assert_eq!(rec_item.cover_url, "https://img.example.com/1003.jpg");
    assert_eq!(rec_item.episode_count, 20);

    let rejected = extract_data(json!({ "status": 0, "msg": "限流" }), "测试")
        .expect_err("status 0 should fail");
    assert!(matches!(rejected, ContractError::UpstreamRejected { .. }));
}

#[test]
fn codec_parses_catalog_lists_rankings_and_search_models() {
    let rank_data = extract_data(
        json!({
            "status": 1,
            "data": {
                "type": "recommend",
                "items": [{
                    "rank": 1,
                    "video_id": 88,
                    "title": "高分剧",
                    "cover": "/covers/88.jpg",
                    "episode_count": 12,
                    "is_finished": true,
                    "score": 9.6,
                    "tags": ["悬疑"],
                    "metrics": { "play": 77700 }
                }]
            }
        }),
        "榜单",
    )
    .expect("rank data");
    let ranks =
        parse_ranking(rank_data, RankingKind::Recommend, page_req(1, 20)).expect("parse ranking");
    let entry = ranks.items.first().expect("rank 1");
    assert_eq!(entry.series.id, SeriesId::try_from(88).expect("id 88"));
    assert_eq!(entry.metric_label, "口碑推荐");
    assert_eq!(entry.metric_value, 77700);
    assert_eq!(ranks.pagination.total, 1);

    let cats =
        parse_categories(json!({ "items": [{ "id": 24, "name": "短剧", "route": "duanju" }] }))
            .expect("categories");
    assert_eq!(cats.len(), 1);

    let groups = parse_tag_groups(json!({
        "categories": [{
            "id": 1,
            "code": "genre",
            "name": "题材",
            "tags": [{ "id": 10, "name": "都市", "slug": "dushi", "is_hot": true }]
        }]
    }))
    .expect("tag groups");
    assert_eq!(groups.first().expect("group").tags.len(), 1);

    let paged = parse_paged_series(
        json!({
            "items": [{ "id": 9, "title": "九重", "cover": "/9.jpg", "episode_count": 10 }]
        }),
        page_req(1, 20),
        "列表",
    )
    .expect("paged");
    assert_eq!(paged.pagination.total, 1);
    assert_eq!(paged.pagination.pages, 1);

    let suggest = parse_search_suggestions(json!({
        "content": [{ "id": 5, "title": "星河", "cover": "/5.jpg" }],
        "items": ["星河万里"],
        "tags": ["科幻"]
    }))
    .expect("suggest");
    assert_eq!(suggest.series.len(), 1);
    assert_eq!(suggest.phrases, vec!["星河万里".to_owned()]);
    assert_eq!(suggest.tags, vec!["科幻".to_owned()]);

    let kws = parse_hot_keywords(json!({ "items": [{ "word": "逆袭", "heat": 900 }] }))
        .expect("hot keywords");
    assert_eq!(kws.first().expect("kw").word, "逆袭");

    let tags =
        parse_hot_tags(json!({ "items": [{ "name": "古风", "heat": 800 }] })).expect("hot tags");
    assert_eq!(tags.first().expect("tag").name, "古风");
}

fn cache_test_upstream() -> StubUpstream {
    StubUpstream::default()
        .with_route(
            "/api/site/categories/video",
            json!({
                "status": 1,
                "data": { "items": [{ "id": 24, "name": "短剧", "route": "duanju" }] }
            }),
        )
        .with_route(
            "/api/tags",
            json!({
                "status": 1,
                "data": {
                    "categories": [{
                        "id": 1,
                        "code": "genre",
                        "name": "题材",
                        "tags": [{ "id": 101, "name": "都市", "slug": "dushi", "is_hot": true }]
                    }]
                }
            }),
        )
        .with_route(
            "/api/ranks/hot?page=1&size=10",
            json!({
                "status": 1,
                "data": {
                    "type": "hot",
                    "items": [{
                        "rank": 1,
                        "video_id": 101,
                        "title": "热榜第一",
                        "cover": "/covers/101.jpg",
                        "episode_count": 50,
                        "is_finished": true,
                        "score": 9.5,
                        "tags": ["都市"],
                        "metric_value": 120_000
                    }]
                }
            }),
        )
        .with_route(
            "/api/videos?page=1&size=10&sort=random",
            json!({
                "status": 1,
                "data": {
                    "items": [{
                        "id": 202,
                        "title": "随机剧",
                        "cover": "/covers/202.jpg",
                        "episode_count": 30,
                        "is_finished": false,
                        "score": 8.7,
                        "tags": ["反转"],
                        "hot": 3000
                    }],
                    "pagination": { "page": 1, "pages": 1, "size": 10, "total": 1 }
                }
            }),
        )
        .with_route(
            "/api/search/hot",
            json!({ "status": 1, "data": { "items": [{ "word": "热词", "heat": 100 }] } }),
        )
        .with_route(
            "/api/search/tags/hot",
            json!({ "status": 1, "data": { "items": [{ "name": "热标", "heat": 200 }] } }),
        )
        .with_route(
            "/api/search/suggest?q=%E7%83%AD&type=video",
            json!({
                "status": 1,
                "data": { "content": [], "phrases": ["热播"], "tags": ["都市"] }
            }),
        )
}

#[tokio::test]
async fn public_endpoints_use_ttl_cache_and_expire() {
    let env = TestEnv::new(cache_test_upstream()).await;

    let cats1 = categories(&env.deps).await.expect("categories 1");
    env.clock.advance_secs(1799);
    let cats2 = categories(&env.deps).await.expect("categories cached");
    assert_eq!(cats1, cats2);
    assert_eq!(env.upstream.calls().len(), 1);

    env.clock.advance_secs(1);
    let cats3 = categories(&env.deps).await.expect("categories expired");
    assert_eq!(cats1, cats3);
    assert_eq!(env.upstream.calls().len(), 2);

    let groups = tag_groups(&env.deps).await.expect("tag groups");
    assert_eq!(groups.len(), 1);
    let ranks = list_ranking(
        &env.deps,
        ListRankingRequest {
            kind: RankingKind::Hot,
            page: page_req(1, 10),
        },
    )
    .await
    .expect("ranking");
    assert_eq!(ranks.items.len(), 1);

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM catalog_cache_entries")
        .fetch_one(env.deps.pool())
        .await
        .expect("count cache rows");
    assert_eq!(count.0, 3);
}

#[tokio::test]
async fn dynamic_endpoints_bypass_and_never_write_cache() {
    let env = TestEnv::new(cache_test_upstream()).await;
    let random_req = ListSeriesRequest {
        filter: SeriesFilter::All { category_id: None },
        sort: SeriesSort::Random,
        page: page_req(1, 10),
    };
    let rand1 = list_series(&env.deps, random_req.clone())
        .await
        .expect("random 1");
    let rand2 = list_series(&env.deps, random_req).await.expect("random 2");
    assert_eq!(rand1.items.len(), 1);
    assert_eq!(rand2.items.len(), 1);
    assert_eq!(env.upstream.calls().len(), 2);

    let kws = hot_keywords(&env.deps).await.expect("hot keywords");
    assert_eq!(kws.len(), 1);
    let tags = hot_tags(&env.deps).await.expect("hot tags");
    assert_eq!(tags.len(), 1);
    let blank_suggest = search_suggestions(&env.deps, "  ".to_owned())
        .await
        .expect("blank suggest");
    assert!(blank_suggest.phrases.is_empty());
    let suggest = search_suggestions(&env.deps, "热".to_owned())
        .await
        .expect("suggest");
    assert_eq!(suggest.phrases, vec!["热播".to_owned()]);

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM catalog_cache_entries")
        .fetch_one(env.deps.pool())
        .await
        .expect("count cache rows after dynamic");
    assert_eq!(count.0, 0);
}

#[test]
fn list_series_and_ranking_preserve_original_badge_from_is_original_flag() {
    let list_json = json!({
        "items": [
            {
                "id": 15941,
                "title": "列表原创短剧",
                "cover": "/c/15941.jpg",
                "score": 8.9,
                "tags": ["逆袭"],
                "episode_count": 0,
                "total_episodes": 21,
                "is_finished": 0,
                "is_original": 1
            },
            {
                "id": 15942,
                "title": "显式角标短剧",
                "cover": "/c/15942.jpg",
                "score": 9.1,
                "badge": "黄果原创",
                "is_original": 1
            },
            {
                "id": 15943,
                "title": "普通非原创短剧",
                "cover": "/c/15943.jpg",
                "score": 8.0,
                "is_original": 0
            }
        ],
        "total": 3
    });
    let paged = parse_paged_series(list_json, page_req(1, 20), "列表").expect("parse paged");
    let first = paged.items.first().expect("first item");
    let second = paged.items.get(1).expect("second item");
    let third = paged.items.get(2).expect("third item");
    assert_eq!(first.badge.as_deref(), Some("原创"));
    assert_eq!(first.episode_count, 21);
    assert_eq!(second.badge.as_deref(), Some("原创"));
    assert_eq!(third.badge, None);

    let rank_json = json!({
        "items": [
            {
                "id": 15941,
                "title": "榜单原创短剧",
                "cover": "/c/15941.jpg",
                "score": 9.3,
                "episode_count": 0,
                "total_episodes": 30,
                "is_original": true
            }
        ]
    });
    let ranked = parse_ranking(rank_json, RankingKind::Hot, page_req(1, 20)).expect("parse rank");
    let ranked_first = ranked.items.first().expect("first rank item");
    assert_eq!(ranked_first.series.badge.as_deref(), Some("原创"));
    assert_eq!(ranked_first.series.episode_count, 30);
}

#[tokio::test]
async fn home_feed_fetches_and_parses_latest_shelf_from_api_home_latest() {
    let upstream = StubUpstream::default()
        .with_route(
            "/api/home",
            json!({
                "status": 1,
                "data": {
                    "banners": [],
                    "featured": [{ "id": 100, "title": "精选剧", "cover": "/c/100.jpg" }],
                    "cate_rec": []
                }
            }),
        )
        .with_route(
            "/api/home/latest",
            json!({
                "status": 1,
                "data": {
                    "items": [
                        {
                            "id": 15941,
                            "title": "刚刚更新剧",
                            "cover": "/c/15941.jpg",
                            "score": 8.7,
                            "episode_count": 2,
                            "total_episodes": 0,
                            "badge": "黄果原创"
                        }
                    ],
                    "total": 1,
                    "page": 1,
                    "size": 20
                }
            }),
        );
    let env = TestEnv::new(upstream).await;
    let feed = super::handler::home_feed(&env.deps)
        .await
        .expect("home_feed");
    assert_eq!(feed.featured.len(), 1);
    assert_eq!(feed.latest.len(), 1);
    let latest_first = feed.latest.first().expect("latest");
    assert_eq!(latest_first.title, "刚刚更新剧");
    assert_eq!(latest_first.badge.as_deref(), Some("原创"));
}
