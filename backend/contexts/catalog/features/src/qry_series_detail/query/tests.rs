use std::num::{NonZeroU16, NonZeroU32};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use hg_backend_contracts::catalog::CatalogMetadataPort as _;
use hg_backend_contracts::error::{PortError, PortFuture};
use hg_contracts::ids::{CommentId, EpisodeKey, EpisodeNumber, PageRequest, SeriesId};
use hg_infra::clock::FixedClock;
use hg_infra::db::LocalDatabase;
use hg_infra::edge_crypto::EdgeChannelScope;
use hg_infra::storage::StorageLayout;
use serde_json::{Value, json};

use super::super::handler::{list_comment_replies, list_episode_comments, series_detail};
use super::super::port::CatalogMetadataAdapter;
use crate::CatalogDeps;
use crate::upstream::CatalogUpstream;

use super::cover_cache::build_cover_rel_path;
use super::parser::{parse_comment_replies, parse_episode_comments, parse_series_detail};

struct StubUpstream {
    detail_calls: AtomicUsize,
    cover_calls: AtomicUsize,
    last_scope: parking_lot::Mutex<Option<EdgeChannelScope>>,
    cover_bytes: Vec<u8>,
}

impl StubUpstream {
    fn new(cover_bytes: Vec<u8>) -> Self {
        Self {
            detail_calls: AtomicUsize::new(0),
            cover_calls: AtomicUsize::new(0),
            last_scope: parking_lot::Mutex::new(None),
            cover_bytes,
        }
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
        *self.last_scope.lock() = Some(scope);
        let payload = if path_with_query.contains("/replies") {
            json!({
                "status": 1,
                "data": { "items": [], "pagination": { "page": 1, "pages": 0, "size": 20, "total": 0 } }
            })
        } else if path_with_query.contains("/comments") {
            json!({
                "status": 1,
                "data": { "ep": 1, "items": [], "pagination": { "page": 1, "pages": 0, "size": 20, "total": 0 } }
            })
        } else {
            self.detail_calls.fetch_add(1, Ordering::SeqCst);
            json!({
                "status": 1,
                "data": {
                    "id": 101,
                    "title": "测试剧目",
                    "description": "简介",
                    "cover": "/covers/101.png",
                    "score": 9.0,
                    "is_finished": true,
                    "episode_count": 1,
                    "total_episodes": 1,
                    "tags": ["悬疑"],
                    "counts": { "hot": 100, "like": 10, "collect": 5, "comment": 2 },
                    "episodes": [{ "ep_num": 1, "title": "第1集", "duration": 60, "status": 1 }]
                }
            })
        };
        Box::pin(async move { Ok(payload) })
    }

    fn fetch_raw_cover<'a>(
        &'a self,
        _cover_url: &'a str,
    ) -> PortFuture<'a, Result<Vec<u8>, PortError>> {
        self.cover_calls.fetch_add(1, Ordering::SeqCst);
        let bytes = self.cover_bytes.clone();
        Box::pin(async move { Ok(bytes) })
    }
}

async fn make_deps(
    upstream: Arc<StubUpstream>,
) -> (tempfile::TempDir, Arc<FixedClock>, CatalogDeps) {
    let dir = tempfile::tempdir().expect("tempdir");
    let storage = Arc::new(
        StorageLayout::new(&dir.path().join("data"), &dir.path().join("cache")).expect("storage"),
    );
    storage.ensure_dirs().await.expect("ensure dirs");
    let db = LocalDatabase::open_in_memory().await.expect("sqlite");
    let clock = Arc::new(FixedClock::new(1_700_000_000));
    let deps = CatalogDeps {
        db,
        storage,
        clock: Arc::clone(&clock) as _,
        upstream,
        cache: crate::cache::CatalogMemoryCache::default(),
    };
    (dir, clock, deps)
}

#[test]
fn parses_series_detail_with_aliases_and_status_flags() {
    let series_id = SeriesId::try_from(42).expect("series 42");
    let payload = json!({
        "status": 1,
        "data": {
            "title": "归雁",
            "desc": "古风短剧",
            "cover_url": "/covers/42.enc",
            "score": 8.9,
            "is_finished": false,
            "is_original": 1,
            "is_hot": 0,
            "tags": ["古风", "悬疑"],
            "counts": { "play_heat": 1200, "likes": 34, "bookmarks": 56, "comments": 78 },
            "episodes": [
                { "ep": 1, "title": "", "duration_secs": 90, "status": 1 },
                { "ep": 2, "title": "第2集", "duration_secs": 105, "status": 0 }
            ]
        }
    });

    let detail = parse_series_detail(series_id, &payload).expect("detail should parse");
    assert_eq!(detail.summary.id, series_id);
    assert_eq!(detail.summary.description, "古风短剧");
    assert_eq!(detail.summary.cover_url, "/covers/42.enc");
    assert!(detail.summary.is_original);
    assert!(!detail.summary.is_hot);
    assert!(!detail.summary.is_new);
    assert_eq!(detail.summary.hot, 1200);
    assert_eq!(detail.total_episodes, 2);
    assert_eq!(detail.stats.likes, 34);
    assert_eq!(detail.stats.bookmarks, 56);
    assert_eq!(detail.stats.comments, 78);

    let ep1 = detail.episodes.first().expect("ep1");
    assert_eq!(ep1.ep_num, EpisodeNumber::try_from(1).expect("ep1 num"));
    assert_eq!(ep1.title, "第1集");
    assert!(ep1.playable);

    let ep2 = detail.episodes.get(1).expect("ep2");
    assert!(!ep2.playable);
}

#[test]
fn falls_back_to_episode_list_when_upstream_reports_zero_totals() {
    let series_id = SeriesId::try_from(117).expect("series 117");
    let payload = json!({
        "status": 1,
        "data": {
            "title": "连载短剧",
            "total_episodes": 0,
            "episode_count": 0,
            "episodes": [
                { "ep": 1, "duration_secs": 90, "status": 1 },
                { "ep": 2, "duration_secs": 95, "status": 1 },
                { "ep": 3, "duration_secs": 99, "status": 1 }
            ]
        }
    });

    let detail = parse_series_detail(series_id, &payload).expect("detail should parse");
    assert_eq!(detail.total_episodes, 3);
    assert_eq!(detail.summary.episode_count, 3);
}

#[test]
fn rejects_series_detail_without_data_or_zero_episode() {
    let series_id = SeriesId::try_from(7).expect("series 7");
    let missing_data = json!({ "status": 1 });
    let err = parse_series_detail(series_id, &missing_data).expect_err("missing data");
    assert!(matches!(err, PortError::UpstreamRejected { .. }));

    let bad_ep = json!({
        "status": 1,
        "data": {
            "title": "测试",
            "episodes": [{ "ep_num": 0 }]
        }
    });
    let err = parse_series_detail(series_id, &bad_ep).expect_err("ep 0 should fail");
    assert!(matches!(err, PortError::UpstreamRejected { .. }));
}

#[test]
fn parses_comments_and_replies_with_pagination_fallback() {
    let page = PageRequest::new(
        NonZeroU32::new(2).expect("page 2"),
        NonZeroU16::new(10).expect("size 10"),
    );

    let comments_payload = json!({
        "status": 1,
        "data": {
            "items": [{
                "id": 100,
                "nickname": "甲",
                "avatar_color": "#112233",
                "text": "不错",
                "time": "刚刚",
                "like_count": 5,
                "is_pinned": false,
                "ip_region": " ",
                "reply_count": 1,
                "reply_list": [{
                    "id": 101,
                    "nickname": "乙",
                    "avatar_color": "#445566",
                    "text": "确实",
                    "time": "刚刚",
                    "like_count": 1,
                    "reply_to_nickname": ""
                }]
            }],
            "total": 15
        }
    });

    let comments = parse_episode_comments(&comments_payload, page).expect("comments");
    assert_eq!(comments.pagination.page, 2);
    assert_eq!(comments.pagination.size, 10);
    assert_eq!(comments.pagination.total, 15);
    assert_eq!(comments.pagination.pages, 2);
    let first = comments.items.first().expect("first comment");
    assert_eq!(first.ip_region, None);
    assert_eq!(first.preview_replies.len(), 1);
    assert_eq!(
        first
            .preview_replies
            .first()
            .expect("preview")
            .reply_to_nickname,
        None
    );

    let replies_payload = json!({
        "status": 1,
        "data": {
            "items": [{
                "id": 102,
                "nickname": "丙",
                "avatar_color": "#778899",
                "text": "赞同",
                "time": "1分钟前",
                "like_count": 2,
                "reply_to_nickname": "甲"
            }],
            "pagination": { "page": 2, "pages": 3, "size": 10, "total": 25 }
        }
    });
    let replies = parse_comment_replies(&replies_payload, page).expect("replies");
    assert_eq!(replies.pagination.pages, 3);
    assert_eq!(replies.pagination.total, 25);
    assert_eq!(
        replies
            .items
            .first()
            .expect("first reply")
            .reply_to_nickname
            .as_deref(),
        Some("甲")
    );
}

#[test]
fn builds_deterministic_safe_cover_cache_paths() {
    let first = build_cover_rel_path("/covers/101.enc");
    let repeat = build_cover_rel_path("/covers/101.enc");
    let other = build_cover_rel_path("/covers/102.enc");

    assert_eq!(first, repeat);
    assert_ne!(first, other);
    assert!(first.starts_with("covers/"));
    assert!(first.ends_with(".bin"));
    assert!(!first.contains(".."));
    assert!(!first.contains('\\'));
}

#[tokio::test]
async fn metadata_port_rejects_missing_episode() {
    let upstream = Arc::new(StubUpstream::new(b"\x89PNGraw".to_vec()));
    let series_id = SeriesId::try_from(101).expect("series 101");
    let (_dir, _clock, deps) = make_deps(upstream).await;
    let port = CatalogMetadataAdapter::new(deps);
    let ep99 = EpisodeNumber::try_from(99).expect("ep 99");
    assert!(matches!(
        port.fetch_episode_snapshot(EpisodeKey::new(series_id, ep99))
            .await,
        Err(PortError::NotFound { .. })
    ));
}

#[tokio::test]
async fn detail_cache_respects_ttl_and_comments_use_dynamic_scope() {
    let upstream = Arc::new(StubUpstream::new(b"\x89PNGraw".to_vec()));
    let (_dir, clock, deps) = make_deps(Arc::clone(&upstream)).await;
    let series_id = SeriesId::try_from(101).expect("series 101");

    let first = series_detail(&deps, series_id).await.expect("first detail");
    assert_eq!(first.summary.title, "测试剧目");
    assert_eq!(*upstream.last_scope.lock(), Some(EdgeChannelScope::Dynamic));
    assert_eq!(upstream.detail_calls.load(Ordering::SeqCst), 1);

    let cached = series_detail(&deps, series_id)
        .await
        .expect("cached detail");
    assert_eq!(cached, first);
    assert_eq!(upstream.detail_calls.load(Ordering::SeqCst), 1);

    clock.advance_secs(301);
    let refetched = series_detail(&deps, series_id)
        .await
        .expect("expired detail refetches");
    assert_eq!(refetched, first);
    assert_eq!(upstream.detail_calls.load(Ordering::SeqCst), 2);

    let page = PageRequest::new(
        NonZeroU32::new(1).expect("page 1"),
        NonZeroU16::new(20).expect("size 20"),
    );
    let comments = list_episode_comments(
        &deps,
        hg_contracts::catalog::ListEpisodeCommentsRequest {
            series_id,
            ep_num: EpisodeNumber::try_from(1).expect("ep 1"),
            page,
        },
    )
    .await
    .expect("comments");
    assert!(comments.items.is_empty());
    assert_eq!(*upstream.last_scope.lock(), Some(EdgeChannelScope::Dynamic));

    let replies = list_comment_replies(
        &deps,
        hg_contracts::catalog::ListCommentRepliesRequest {
            series_id,
            comment_id: CommentId::try_from(5001).expect("comment 5001"),
            page,
        },
    )
    .await
    .expect("replies");
    assert!(replies.items.is_empty());
    assert_eq!(*upstream.last_scope.lock(), Some(EdgeChannelScope::Dynamic));
}

#[tokio::test]
async fn cover_cache_heals_missing_disk_file_and_rejects_empty_url() {
    let png = b"\x89PNG\r\n\x1a\ncover-bytes".to_vec();
    let upstream = Arc::new(StubUpstream::new(png.clone()));
    let (_dir, _clock, deps) = make_deps(Arc::clone(&upstream)).await;
    let port = CatalogMetadataAdapter::new(deps.clone());

    assert!(matches!(
        port.fetch_decrypted_cover("   ").await,
        Err(PortError::InvalidState { .. })
    ));

    let first = port
        .fetch_decrypted_cover("/covers/101.png")
        .await
        .expect("first fetch");
    assert_eq!(first, png);
    assert_eq!(upstream.cover_calls.load(Ordering::SeqCst), 1);

    let rel_path = build_cover_rel_path("/covers/101.png");
    tokio::fs::remove_file(deps.storage.catalog_cache_dir().join(rel_path))
        .await
        .expect("remove cached file");

    let refetched = port
        .fetch_decrypted_cover("/covers/101.png")
        .await
        .expect("self heal fetch");
    assert_eq!(refetched, png);
    assert_eq!(upstream.cover_calls.load(Ordering::SeqCst), 2);
}
