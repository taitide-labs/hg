use std::num::{NonZeroU16, NonZeroU32};

use hg_contracts::catalog::{CommentReply, EpisodeComment, ListCommentRepliesRequest};
use hg_contracts::ids::{CommentId, PageInfo, PageRequest, SeriesId};
use nsh_core::prelude::*;

use super::super::store;
use crate::gateway;

pub const REPLIES_PAGE_SIZE: u16 = 20;

#[derive(Debug, Clone, Default)]
pub struct CommentRepliesState {
    pub series_id: Option<SeriesId>,
    pub comment_id: Option<CommentId>,
    pub parent_comment: Option<EpisodeComment>,
    pub replies: Vec<CommentReply>,
    pub total_count: u64,
    pub next_page: u32,
    pub has_more: bool,
    pub loading: bool,
    pub loading_more: bool,
    pub error: Option<String>,
    pub request_seq: u64,
}

fn state() -> Signal<CommentRepliesState> {
    global::<CommentRepliesState>()
}

#[must_use]
pub fn snapshot() -> CommentRepliesState {
    state().get()
}

pub fn reset_state() {
    state().update(|s| {
        let next_seq = s.request_seq.wrapping_add(1);
        *s = CommentRepliesState {
            request_seq: next_seq,
            ..CommentRepliesState::default()
        };
    });
}

pub fn close() {
    reset_state();
    store::close_comment_replies();
}

pub fn open_from_player_store(series_id: SeriesId, comment_id: CommentId) {
    let current = state().with_untracked(|s| (s.series_id, s.comment_id));
    if current == (Some(series_id), Some(comment_id)) {
        return;
    }
    let comments = store::comments_snapshot().items;
    let parent = find_parent_comment(&comments, comment_id);
    open_and_load(series_id, comment_id, parent);
}

pub fn retry() {
    let (series_id, comment_id, parent) =
        state().with_untracked(|s| (s.series_id, s.comment_id, s.parent_comment.clone()));
    if let (Some(sid), Some(cid)) = (series_id, comment_id) {
        open_and_load(sid, cid, parent);
    }
}

pub fn load_more() {
    let (series_id, comment_id, page, can_load) = state().with_untracked(|s| {
        (
            s.series_id,
            s.comment_id,
            s.next_page,
            s.has_more && !s.loading && !s.loading_more,
        )
    });
    if !can_load {
        return;
    }
    let (Some(sid), Some(cid)) = (series_id, comment_id) else {
        return;
    };
    fetch_page(sid, cid, page, false);
}

fn open_and_load(series_id: SeriesId, comment_id: CommentId, parent: Option<EpisodeComment>) {
    let seq = begin_initial_load(series_id, comment_id, parent);
    run_fetch(series_id, comment_id, 1, seq, true);
}

fn fetch_page(series_id: SeriesId, comment_id: CommentId, page: u32, initial: bool) {
    let mut seq = 0;
    state().update(|s| {
        s.request_seq = s.request_seq.wrapping_add(1);
        seq = s.request_seq;
        if initial {
            s.loading = true;
        } else {
            s.loading_more = true;
        }
        s.error = None;
    });
    run_fetch(series_id, comment_id, page, seq, initial);
}

fn begin_initial_load(
    series_id: SeriesId,
    comment_id: CommentId,
    parent: Option<EpisodeComment>,
) -> u64 {
    let mut seq = 0;
    let total_hint = parent.as_ref().map_or(0, |c| u64::from(c.reply_count));
    state().update(|s| {
        seq = s.request_seq.wrapping_add(1);
        *s = CommentRepliesState {
            series_id: Some(series_id),
            comment_id: Some(comment_id),
            parent_comment: parent,
            replies: Vec::new(),
            total_count: total_hint,
            next_page: 1,
            has_more: false,
            loading: true,
            loading_more: false,
            error: None,
            request_seq: seq,
        };
    });
    seq
}

fn run_fetch(series_id: SeriesId, comment_id: CommentId, page: u32, seq: u64, initial: bool) {
    let gw = gateway::current();
    let req = build_replies_request(series_id, comment_id, page);
    spawn(async move {
        match gw.list_comment_replies(req).await {
            Ok(paged) => apply_loaded_page(seq, initial, paged.items, paged.pagination),
            Err(err) => apply_load_error(seq, initial, err.to_string()),
        }
    });
}

fn apply_loaded_page(seq: u64, initial: bool, items: Vec<CommentReply>, pagination: PageInfo) {
    state().update(|s| {
        if s.request_seq != seq {
            return;
        }
        if initial {
            s.replies = items;
            s.loading = false;
        } else {
            merge_replies_page(&mut s.replies, items);
            s.loading_more = false;
        }
        s.total_count = pagination.total.max(s.replies.len() as u64);
        s.has_more = pagination.page < pagination.pages;
        s.next_page = pagination.page.saturating_add(1);
        s.error = None;
    });
}

fn apply_load_error(seq: u64, initial: bool, message: String) {
    state().update(|s| {
        if s.request_seq != seq {
            return;
        }
        if initial {
            s.loading = false;
        } else {
            s.loading_more = false;
        }
        s.error = Some(message);
    });
}

#[must_use]
pub(super) fn build_replies_request(
    series_id: SeriesId,
    comment_id: CommentId,
    page: u32,
) -> ListCommentRepliesRequest {
    let safe_page = NonZeroU32::new(page.max(1)).unwrap_or(NonZeroU32::MIN);
    let safe_size = NonZeroU16::new(REPLIES_PAGE_SIZE).unwrap_or(NonZeroU16::MIN);
    ListCommentRepliesRequest {
        series_id,
        comment_id,
        page: PageRequest::new(safe_page, safe_size),
    }
}

#[must_use]
pub(super) fn find_parent_comment(
    comments: &[EpisodeComment],
    comment_id: CommentId,
) -> Option<EpisodeComment> {
    comments.iter().find(|c| c.id == comment_id).cloned()
}

pub(super) fn merge_replies_page(existing: &mut Vec<CommentReply>, incoming: Vec<CommentReply>) {
    for reply in incoming {
        if !existing.iter().any(|item| item.id == reply.id) {
            existing.push(reply);
        }
    }
}

#[must_use]
pub(super) fn format_parent_meta(comment: &EpisodeComment) -> String {
    match comment.ip_region.as_deref() {
        Some(region) => format!(
            "{} · {region} · 赞 {}",
            comment.relative_time, comment.like_count
        ),
        None => format!("{} · 赞 {}", comment.relative_time, comment.like_count),
    }
}

#[must_use]
pub(super) fn format_reply_meta(reply: &CommentReply) -> String {
    format!("{} · 赞 {}", reply.relative_time, reply.like_count)
}

#[must_use]
pub(super) fn nickname_initial(nickname: &str) -> String {
    nickname
        .chars()
        .next()
        .map_or_else(|| "剧".to_owned(), |c| c.to_string())
}
