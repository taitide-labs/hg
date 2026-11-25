use std::num::{NonZeroU16, NonZeroU32};

use hg_catalog_features::qry_discovery;
use hg_contracts::catalog::SearchSeriesRequest;
use hg_contracts::ids::{PageRequest, SeriesId};
use hg_infra::edge_crypto::EdgeChannelScope;
use serde_json::{Value, json};

use super::support::{CatalogTestHarness, FakeCatalogUpstream};

fn sample_home_payload() -> Value {
    json!({
        "status": 1,
        "msg": "ok",
        "req_time": 1_700_000_000_u64,
        "data": {
            "banners": [{
                "id": 1,
                "title": "风云再起",
                "video_id": 1001,
                "image": "/covers/1001.jpg",
                "score": 9.4,
                "desc": "都市逆袭热播"
            }],
            "featured": [{
                "id": 1001,
                "title": "风云再起",
                "cover": "/covers/1001.jpg",
                "hot": 88000,
                "score": 9.4,
                "tags": ["都市", "逆袭"],
                "desc": "都市逆袭热播",
                "is_original": 1,
                "is_hot": 1,
                "episode_count": 60,
                "is_finished": true
            }],
            "cate_rec": [{
                "cate_id": 1,
                "cate_name": "短剧",
                "videos": [{
                    "id": 1001,
                    "title": "风云再起",
                    "cover": "/covers/1001.jpg",
                    "hot": 88000,
                    "score": 9.4,
                    "tags": ["都市"],
                    "desc": "都市逆袭热播",
                    "is_original": 0,
                    "is_hot": 1,
                    "episode_count": 60,
                    "is_finished": true
                }]
            }]
        }
    })
}

fn sample_search_payload() -> Value {
    json!({
        "status": 1,
        "msg": "ok",
        "req_time": 1_700_000_000_u64,
        "data": {
            "items": [{
                "id": 2002,
                "title": "长安旧梦",
                "description": "古风权谋",
                "cover": "/covers/2002.jpg",
                "score": 9.1,
                "tags": ["古风"],
                "episode_count": 48,
                "total_episodes": 48,
                "is_finished": true,
                "hot": 52000
            }],
            "pagination": { "page": 1, "pages": 1, "size": 20, "total": 1 }
        }
    })
}

#[tokio::test]
async fn discovery_caches_home_feed_and_routes_search_dynamically() {
    let upstream = FakeCatalogUpstream::default()
        .with_json("/api/home", sample_home_payload())
        .with_json(
            "/api/home/latest",
            json!({
                "status": 1,
                "data": {
                    "items": [],
                    "total": 0,
                    "page": 1,
                    "size": 20
                }
            }),
        )
        .with_json("/api/search", sample_search_payload());
    let harness = CatalogTestHarness::new(upstream).await;

    let first_home = qry_discovery::home_feed(&harness.deps)
        .await
        .expect("home feed should succeed");
    assert_eq!(first_home.banners.len(), 1);
    assert_eq!(
        first_home.featured.first().expect("featured").id,
        SeriesId::try_from(1001).expect("series 1001")
    );

    let second_home = qry_discovery::home_feed(&harness.deps)
        .await
        .expect("cached home feed should succeed");
    assert_eq!(first_home, second_home);
    assert_eq!(harness.upstream.recorded_calls().len(), 2);
    assert_eq!(
        harness
            .upstream
            .recorded_calls()
            .first()
            .expect("call")
            .scope,
        EdgeChannelScope::Dynamic
    );

    let search = qry_discovery::search_series(
        &harness.deps,
        SearchSeriesRequest {
            keyword: "长安".to_owned(),
            page: PageRequest::new(
                NonZeroU32::new(1).expect("page 1"),
                NonZeroU16::new(20).expect("size 20"),
            ),
        },
    )
    .await
    .expect("search should succeed");
    assert_eq!(search.items.len(), 1);
    assert_eq!(search.pagination.total, 1);
    assert_eq!(
        harness
            .upstream
            .recorded_calls()
            .last()
            .expect("search call")
            .scope,
        EdgeChannelScope::Dynamic
    );
}
