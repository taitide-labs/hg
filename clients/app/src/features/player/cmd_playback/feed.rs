use hg_contracts::catalog::{EpisodeComment, SeriesDetail};
use hg_contracts::ids::EpisodeNumber;
use nsh_core::prelude::*;

use super::super::store::{self, EpisodeCommentsState, MobileDetailTab};
use super::command::{episode_segment_ranges, folded_episode_window, nickname_initial};
use super::handler;
use crate::ui::button::{
    ButtonVariant, ControlSize, app_button, app_icon_text_button, app_reactive_button,
    text_link_button,
};
use crate::ui::icons::IconKind;
use crate::ui::surface::{BadgeTone, avatar_circle, badge, separator};
use crate::ui::tabs::segmented_tabs;
use crate::ui::token;

const COMMENTS_SCROLL_THRESHOLD_PX: f32 = 160.0;

pub fn desktop_action_row() -> impl IntoElement {
    let bookmarked_sig = computed(|| store::snapshot().bookmarked);
    view()
        .w_full()
        .flex_shrink_0()
        .flex_row()
        .items_center()
        .gap_2()
        .child(dynamic(move || {
            let bookmarked = bookmarked_sig.get();
            let (icon_kind, label, variant) = if bookmarked {
                (IconKind::BookmarkFilled, "已追剧", ButtonVariant::BrandSoft)
            } else {
                (IconKind::Bookmark, "追剧", ButtonVariant::Secondary)
            };
            app_icon_text_button(
                icon_kind,
                label,
                variant,
                ControlSize::Md,
                handler::toggle_bookmark,
            )
            .into_element()
        }))
        .child(app_icon_text_button(
            IconKind::Download,
            "离线下载",
            ButtonVariant::Secondary,
            ControlSize::Md,
            || store::set_batch_download_open(true),
        ))
}

pub fn desktop_info_feed() -> impl IntoElement {
    view()
        .w(px(356.0))
        .flex_shrink_0()
        .h_full()
        .min_h_0()
        .bg(token::bg)
        .border_l_1()
        .border_color(token::border)
        .child(
            scroll()
                .flex_1()
                .min_h_0()
                .on_scroll(|e: &ScrollEvent| {
                    if should_load_more_on_scroll(e) {
                        handler::load_more_comments();
                    }
                })
                .child(
                    view()
                        .p_4()
                        .gap_4()
                        .child(synopsis_block())
                        .child(separator())
                        .child(episodes_block())
                        .child(separator())
                        .child(comments_block()),
                ),
        )
}

pub fn mobile_detail_sheet() -> impl IntoElement {
    let tab_sig = store::mobile_tab_signal();
    let tabs = vec![
        (MobileDetailTab::Info, "简介"),
        (MobileDetailTab::Comments, "评论"),
    ];

    view()
        .flex_1()
        .min_h_0()
        .bg(token::bg)
        .border_t_1()
        .border_color(token::border)
        .p_4()
        .gap_3()
        .child(segmented_tabs(
            tabs,
            move || tab_sig.get(),
            move |next| tab_sig.set(next),
        ))
        .child(
            scroll()
                .flex_1()
                .min_h_0()
                .on_scroll(move |e: &ScrollEvent| {
                    if tab_sig.get() == MobileDetailTab::Comments && should_load_more_on_scroll(e) {
                        handler::load_more_comments();
                    }
                })
                .child(dynamic(move || match tab_sig.get() {
                    MobileDetailTab::Info => view()
                        .gap_4()
                        .child(synopsis_block())
                        .child(desktop_action_row())
                        .child(separator())
                        .child(episodes_block())
                        .into_element(),
                    MobileDetailTab::Comments => comments_block().into_element(),
                })),
        )
}

fn should_load_more_on_scroll(e: &ScrollEvent) -> bool {
    e.content_size.height > px(0.0)
        && e.offset.y + e.size.height >= e.content_size.height - px(COMMENTS_SCROLL_THRESHOLD_PX)
}

fn synopsis_block() -> impl IntoElement {
    let detail_sig = computed(|| store::snapshot().detail);
    dynamic(move || match detail_sig.get() {
        Some(detail) => synopsis_section(&detail).into_element(),
        None => view().into_element(),
    })
}

fn episodes_block() -> impl IntoElement {
    let episodes_sig = computed(|| {
        let s = store::snapshot();
        (
            s.detail,
            s.current_episode,
            s.episodes_expanded,
            s.episode_segment_index,
        )
    });
    dynamic(move || match episodes_sig.get() {
        (Some(detail), current_episode, expanded, seg_idx) => {
            episodes_section(&detail, current_episode, expanded, seg_idx).into_element()
        }
        (None, _, _, _) => view().into_element(),
    })
}

fn comments_block() -> impl IntoElement {
    let comments_sig = computed(store::comments_snapshot);
    dynamic(move || comments_section(&comments_sig.get()).into_element())
}

fn synopsis_section(detail: &SeriesDetail) -> impl IntoElement {
    let status = if detail.summary.is_finished {
        format!("全 {} 集 · 已完结", detail.total_episodes)
    } else {
        format!("更新至 {} 集", detail.total_episodes)
    };
    let meta = format!("{:.1} 分 · {status}", detail.summary.score);

    let mut tag_row = view().flex_row().flex_wrap().gap_1p5();
    for tag in &detail.summary.tags {
        tag_row = tag_row.child(badge(tag.clone(), BadgeTone::Neutral));
    }

    view()
        .gap_2()
        .child(
            text(detail.summary.title.clone())
                .text_base()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(token::fg),
        )
        .child(text(meta).text_xs().text_color(token::muted_fg))
        .child(
            text(detail.summary.description.clone())
                .text_sm()
                .text_color(token::muted_fg),
        )
        .children((!detail.summary.tags.is_empty()).then_some(tag_row))
}

fn episodes_section(
    detail: &SeriesDetail,
    current_episode: Option<EpisodeNumber>,
    expanded: bool,
    segment_index: usize,
) -> impl IntoElement {
    let current = current_episode.map_or(1, EpisodeNumber::get);
    let header_title = format!("选集 ({current}/{})", detail.total_episodes);
    let (toggle_label, toggle_a11y, toggle_icon) = if expanded {
        ("收起", "收起选集", IconKind::ChevronUp)
    } else {
        ("展开", "展开选集", IconKind::ChevronDown)
    };

    let ranges = episode_segment_ranges(detail.total_episodes, 50);
    let active_seg = segment_index.min(ranges.len().saturating_sub(1));
    let (start, end) = if expanded {
        ranges
            .get(active_seg)
            .copied()
            .unwrap_or((1, detail.total_episodes))
    } else {
        folded_episode_window(current, detail.total_episodes, 8)
    };

    let mut grid = view().flex_row().flex_wrap().gap_2();
    for ep in &detail.episodes {
        let num = ep.ep_num.get();
        if num < start || num > end {
            continue;
        }
        let ep_num = ep.ep_num;
        grid = grid.child(view().w(px(72.0)).child(app_reactive_button(
            format!("{num}"),
            move || {
                if current == num {
                    ButtonVariant::Primary
                } else if store::is_episode_watched(ep_num) {
                    ButtonVariant::Watched
                } else {
                    ButtonVariant::Secondary
                }
            },
            ControlSize::Sm,
            move || handler::switch_episode(ep_num),
        )));
    }

    let show_segments = expanded && ranges.len() > 1;
    let mut seg_row = view().flex_row().flex_wrap().gap_1p5();
    if show_segments {
        for (idx, (seg_start, seg_end)) in ranges.iter().enumerate() {
            let variant = if idx == active_seg {
                ButtonVariant::Primary
            } else {
                ButtonVariant::Ghost
            };
            seg_row = seg_row.child(app_button(
                format!("{seg_start}-{seg_end}"),
                variant,
                ControlSize::Sm,
                move || store::set_episode_segment(idx),
            ));
        }
    }

    view()
        .gap_2p5()
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    text(header_title)
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(token::fg),
                )
                .child(text_link_button(
                    toggle_label,
                    toggle_a11y,
                    Some(toggle_icon),
                    store::toggle_episodes_expanded,
                )),
        )
        .children(show_segments.then_some(seg_row))
        .child(grid)
}

fn comments_section(comments: &EpisodeCommentsState) -> impl IntoElement {
    let header = format!("评论 ({})", comments.total_count);
    let mut list = view().gap_3().child(
        text(header)
            .text_sm()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(token::fg),
    );

    if comments.loading && comments.items.is_empty() {
        return list.child(
            text("正在加载评论...")
                .text_xs()
                .text_color(token::muted_fg),
        );
    }

    if comments.items.is_empty() {
        let message = if comments.error.is_some() {
            "评论加载失败"
        } else {
            "暂无评论"
        };
        return list.child(text(message).text_xs().text_color(token::muted_fg));
    }

    for comment in &comments.items {
        list = list.child(comment_row(comment));
    }
    if comments.loading_more {
        list = list.child(
            text("正在加载更多评论...")
                .text_xs()
                .text_color(token::muted_fg),
        );
    } else if comments.error.is_some() {
        list = list.child(
            text("加载失败，请下滑重试")
                .text_xs()
                .text_color(token::muted_fg),
        );
    }
    list
}

fn comment_row(comment: &EpisodeComment) -> impl IntoElement {
    let meta = match comment.ip_region.as_deref() {
        Some(region) => format!(
            "{} · {region} · 赞 {}",
            comment.relative_time, comment.like_count
        ),
        None => format!("{} · 赞 {}", comment.relative_time, comment.like_count),
    };

    let comment_id = comment.id;
    let mut replies_box = view()
        .p_2p5()
        .rounded(token::radius)
        .bg(token::muted)
        .gap_1p5();
    for reply in &comment.preview_replies {
        replies_box = replies_box.child(
            text(format!("{}：{}", reply.nickname, reply.text))
                .text_xs()
                .text_color(token::muted_fg),
        );
    }
    if comment.reply_count > 0 {
        let count_label = format!("共 {} 条回复", comment.reply_count);
        replies_box = replies_box.child(text_link_button(
            count_label.clone(),
            count_label,
            Some(IconKind::ChevronRight),
            move || store::open_comment_replies(comment_id),
        ));
    }

    view()
        .flex_row()
        .items_start()
        .gap_2p5()
        .child(avatar_circle(
            nickname_initial(&comment.nickname),
            comment.avatar_color.clone(),
        ))
        .child(
            view()
                .flex_1()
                .min_w_0()
                .gap_1p5()
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_1p5()
                        .child(
                            text(comment.nickname.clone())
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(token::fg),
                        )
                        .children(comment.is_pinned.then(|| badge("置顶", BadgeTone::Brand))),
                )
                .child(text(comment.text.clone()).text_sm().text_color(token::fg))
                .child(text(meta).text_xs().text_color(token::muted_fg))
                .children(
                    (!comment.preview_replies.is_empty() || comment.reply_count > 0)
                        .then_some(replies_box),
                ),
        )
}
