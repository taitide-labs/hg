use std::num::{NonZeroU32, NonZeroU64};

use hg_contracts::catalog::{CommentReply, EpisodeComment};
use hg_contracts::ids::{CommentId, SeriesId};

use super::query::{
    REPLIES_PAGE_SIZE, build_replies_request, find_parent_comment, format_parent_meta,
    format_reply_meta, merge_replies_page, nickname_initial,
};

fn comment_id(id: u64) -> CommentId {
    CommentId::new(NonZeroU64::new(id).unwrap())
}

fn series_id(id: u64) -> SeriesId {
    SeriesId::new(NonZeroU64::new(id).unwrap())
}

fn sample_reply(id: u64, nickname: &str, reply_to: Option<&str>) -> CommentReply {
    CommentReply {
        id: comment_id(id),
        nickname: nickname.to_owned(),
        avatar_color: "#0F766E".to_owned(),
        text: format!("回复内容 {id}"),
        relative_time: "10 分钟前".to_owned(),
        like_count: 12,
        reply_to_nickname: reply_to.map(ToOwned::to_owned),
    }
}

fn sample_comment(id: u64, ip_region: Option<&str>) -> EpisodeComment {
    EpisodeComment {
        id: comment_id(id),
        nickname: "林深见鹿".to_owned(),
        avatar_color: "#C2410C".to_owned(),
        text: "测试主评论".to_owned(),
        relative_time: "2 小时前".to_owned(),
        like_count: 88,
        is_pinned: true,
        ip_region: ip_region.map(ToOwned::to_owned),
        reply_count: 2,
        preview_replies: Vec::new(),
    }
}

#[test]
fn build_replies_request_clamps_page_and_uses_page_size() {
    let req = build_replies_request(series_id(1001), comment_id(5001), 0);
    assert_eq!(req.series_id.get(), 1001);
    assert_eq!(req.comment_id.get(), 5001);
    assert_eq!(req.page.page, NonZeroU32::new(1).unwrap());
    assert_eq!(req.page.size.get(), REPLIES_PAGE_SIZE);

    let req2 = build_replies_request(series_id(1001), comment_id(5001), 3);
    assert_eq!(req2.page.page.get(), 3);
}

#[test]
fn find_parent_comment_and_merge_dedupes_replies() {
    let comments = vec![
        sample_comment(5001, Some("上海")),
        sample_comment(5002, None),
    ];
    let found = find_parent_comment(&comments, comment_id(5001));
    assert!(found.is_some());
    assert_eq!(found.unwrap().id.get(), 5001);
    assert!(find_parent_comment(&comments, comment_id(9999)).is_none());

    let mut replies = vec![sample_reply(5101, "南风知意", Some("林深见鹿"))];
    merge_replies_page(
        &mut replies,
        vec![
            sample_reply(5101, "南风知意", Some("林深见鹿")),
            sample_reply(5102, "周游记", None),
        ],
    );
    let ids: Vec<u64> = replies.iter().map(|r| r.id.get()).collect();
    assert_eq!(ids, vec![5101, 5102]);
}

#[test]
fn format_helpers_use_only_provided_fields() {
    let with_region = sample_comment(5001, Some("上海"));
    let without_region = sample_comment(5002, None);
    assert_eq!(format_parent_meta(&with_region), "2 小时前 · 上海 · 赞 88");
    assert_eq!(format_parent_meta(&without_region), "2 小时前 · 赞 88");

    let reply = sample_reply(5101, "南风知意", Some("林深见鹿"));
    assert_eq!(format_reply_meta(&reply), "10 分钟前 · 赞 12");
    assert_eq!(nickname_initial("南风知意"), "南");
    assert_eq!(nickname_initial(""), "剧");
}
