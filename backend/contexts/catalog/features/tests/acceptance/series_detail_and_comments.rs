use std::num::{NonZeroU16, NonZeroU32};

use aes::Aes128;
use cbc::cipher::block_padding::Pkcs7;
use cbc::cipher::{BlockModeEncrypt as _, KeyIvInit as _};
use hg_backend_contracts::catalog::CatalogMetadataPort as _;
use hg_catalog_features::flags::CatalogFeatureFlags;
use hg_catalog_features::qry_series_detail::{self, CatalogMetadataAdapter};
use hg_contracts::catalog::{ListCommentRepliesRequest, ListEpisodeCommentsRequest};
use hg_contracts::ids::{CommentId, EpisodeKey, EpisodeNumber, PageRequest, SeriesId};
use serde_json::{Value, json};

use super::support::{CatalogTestHarness, FakeCatalogUpstream};

type Aes128CbcEncryptor = cbc::Encryptor<Aes128>;

fn sample_detail_payload() -> Value {
    json!({
        "status": 1,
        "msg": "ok",
        "req_time": 1_700_000_000_u64,
        "data": {
            "id": 101,
            "title": "夜雨长灯",
            "description": "悬疑古装短剧",
            "cover": "/covers/101.enc",
            "score": 9.3,
            "is_finished": true,
            "episode_count": 2,
            "total_episodes": 2,
            "tags": ["悬疑", "古装"],
            "counts": { "hot": 99000, "like": 4200, "collect": 1800, "comment": 320 },
            "episodes": [
                { "ep_num": 1, "title": "第1集 雨夜", "duration": 115, "status": 1 },
                { "ep_num": 2, "title": "第2集 长灯", "duration": 122, "status": 1 }
            ]
        }
    })
}

fn sample_comments_payload() -> Value {
    json!({
        "status": 1,
        "msg": "ok",
        "req_time": 1_700_000_000_u64,
        "data": {
            "ep": 1,
            "items": [{
                "id": 5001,
                "nickname": "听雨客",
                "avatar_color": "#336699",
                "text": "节奏很稳",
                "time": "2小时前",
                "like_count": 18,
                "is_pinned": true,
                "ip_region": "浙江",
                "reply_count": 1,
                "reply_list": []
            }],
            "pagination": { "page": 1, "pages": 1, "size": 20, "total": 1 }
        }
    })
}

fn sample_replies_payload() -> Value {
    json!({
        "status": 1,
        "msg": "ok",
        "req_time": 1_700_000_000_u64,
        "data": {
            "items": [{
                "id": 5002,
                "nickname": "南山",
                "avatar_color": "#996633",
                "text": "同感",
                "time": "1小时前",
                "like_count": 3,
                "reply_to_nickname": "听雨客"
            }],
            "pagination": { "page": 1, "pages": 1, "size": 20, "total": 1 },
            "total": 1
        }
    })
}

#[tokio::test]
async fn series_detail_disabled_by_default_returns_feature_flag_error() {
    let harness = CatalogTestHarness::new(
        CatalogFeatureFlags::default(),
        FakeCatalogUpstream::default(),
    )
    .await;

    let series_id = SeriesId::try_from(101).expect("series 101");
    let err = qry_series_detail::series_detail(&harness.deps, series_id)
        .await
        .expect_err("disabled feature flag should fail");
    assert!(err.to_string().contains("特性开关未开启"));
}

#[tokio::test]
#[ignore = "待实现剧目详情、评论与封面解密切片后激活"]
async fn series_detail_comments_and_metadata_port_decrypt_and_cache_covers() {
    let cover_plain = b"\x89PNG\r\n\x1a\ncatalog-cover-image";
    let key: [u8; 16] = *b"f5d965df75336270";
    let iv: [u8; 16] = *b"97b60394abc2fbe1";
    let encrypted_cover =
        Aes128CbcEncryptor::new(&key.into(), &iv.into()).encrypt_padded_vec::<Pkcs7>(cover_plain);

    let upstream = FakeCatalogUpstream::default()
        .with_json("/api/videos/101", sample_detail_payload())
        .with_json("/api/videos/101/comments", sample_comments_payload())
        .with_json(
            "/api/videos/101/comments/5001/replies",
            sample_replies_payload(),
        )
        .with_cover("/covers/101.enc", encrypted_cover);

    let flags = CatalogFeatureFlags {
        detail_and_comments: true,
        ..CatalogFeatureFlags::default()
    };
    let harness = CatalogTestHarness::new(flags, upstream).await;

    let series_id = SeriesId::try_from(101).expect("series 101");
    let ep1 = EpisodeNumber::try_from(1).expect("ep 1");
    let cid = CommentId::try_from(5001).expect("comment 5001");
    let page = PageRequest::new(
        NonZeroU32::new(1).expect("page 1"),
        NonZeroU16::new(20).expect("size 20"),
    );

    let detail = qry_series_detail::series_detail(&harness.deps, series_id)
        .await
        .expect("series detail should succeed");
    assert_eq!(detail.summary.title, "夜雨长灯");
    assert_eq!(detail.episodes.len(), 2);

    let comments = qry_series_detail::list_episode_comments(
        &harness.deps,
        ListEpisodeCommentsRequest {
            series_id,
            ep_num: ep1,
            page,
        },
    )
    .await
    .expect("comments should succeed");
    assert_eq!(comments.items.len(), 1);

    let replies = qry_series_detail::list_comment_replies(
        &harness.deps,
        ListCommentRepliesRequest {
            series_id,
            comment_id: cid,
            page,
        },
    )
    .await
    .expect("replies should succeed");
    assert_eq!(replies.items.len(), 1);

    let port = CatalogMetadataAdapter::new(harness.deps.clone());
    assert_eq!(
        port.fetch_series_snapshot(series_id)
            .await
            .expect("series snapshot")
            .total_episodes,
        2
    );
    assert_eq!(
        port.fetch_episode_snapshot(EpisodeKey::new(series_id, ep1))
            .await
            .expect("episode snapshot")
            .duration_secs,
        115
    );

    let cover_first = port
        .fetch_decrypted_cover("/covers/101.enc")
        .await
        .expect("decrypted cover first fetch");
    assert_eq!(cover_first, cover_plain);
    let cover_second = port
        .fetch_decrypted_cover("/covers/101.enc")
        .await
        .expect("decrypted cover cache hit");
    assert_eq!(cover_second, cover_plain);
    assert_eq!(harness.upstream.recorded_cover_calls().len(), 1);
}
