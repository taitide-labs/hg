use hg_contracts::catalog::{EpisodeComment, SeriesDetail};
use hg_contracts::ids::EpisodeNumber;
use nsh_core::prelude::*;

use super::super::store::{self, MobileDetailTab};
use super::command::{
    episode_segment_ranges, folded_episode_window, nickname_initial, truncate_synopsis,
};
use super::handler;
use crate::ui::button::{
    ButtonVariant, ControlSize, app_button, app_icon_text_button, text_link_button,
};
use crate::ui::icons::IconKind;
use crate::ui::surface::{BadgeTone, avatar_circle, badge, separator};
use crate::ui::tabs::segmented_tabs;
use crate::ui::token;

pub fn desktop_action_row() -> impl IntoElement {
    view()
        .w_full()
        .flex_shrink_0()
        .flex_row()
        .items_center()
        .gap_2()
        .child(app_icon_text_button(
            IconKind::Bookmark,
            move || {
                if store::snapshot().bookmarked {
                    "已追剧"
                } else {
                    "追剧"
                }
            },
            ButtonVariant::Secondary,
            ControlSize::Md,
            handler::toggle_bookmark,
        ))
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
                .child(view().p_4().gap_4().child(dynamic(feed_sections))),
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
        .child(scroll().flex_1().min_h_0().child(dynamic(move || {
            let snap = store::snapshot();
            let Some(detail) = snap.detail else {
                return view().into_element();
            };
            match tab_sig.get() {
                MobileDetailTab::Info => view()
                    .gap_4()
                    .child(synopsis_section(&detail, snap.info_expanded))
                    .child(desktop_action_row())
                    .child(separator())
                    .child(episodes_section(
                        &detail,
                        snap.current_episode,
                        snap.episodes_expanded,
                        snap.episode_segment_index,
                    ))
                    .into_element(),
                MobileDetailTab::Comments => comments_section(&snap.comments).into_element(),
            }
        })))
}

fn feed_sections() -> impl IntoElement {
    let snap = store::snapshot();
    let Some(detail) = snap.detail else {
        return view().into_element();
    };

    view()
        .gap_4()
        .child(synopsis_section(&detail, snap.info_expanded))
        .child(separator())
        .child(episodes_section(
            &detail,
            snap.current_episode,
            snap.episodes_expanded,
            snap.episode_segment_index,
        ))
        .child(separator())
        .child(comments_section(&snap.comments))
        .into_element()
}

fn synopsis_section(detail: &SeriesDetail, expanded: bool) -> impl IntoElement {
    let status = if detail.summary.is_finished {
        format!("全 {} 集 · 已完结", detail.total_episodes)
    } else {
        format!("更新至 {} 集", detail.total_episodes)
    };
    let meta = format!("{:.1} 分 · {status}", detail.summary.score);
    let (snippet, truncated) = truncate_synopsis(&detail.summary.description, 24);
    let body_text = if expanded {
        detail.summary.description.clone()
    } else {
        snippet
    };

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
        .child(text(body_text).text_sm().text_color(token::muted_fg))
        .children(expanded.then_some(tag_row))
        .children(truncated.then(|| {
            let (label, a11y, chevron) = if expanded {
                ("收起", "收起简介", IconKind::ChevronUp)
            } else {
                ("展开", "展开简介", IconKind::ChevronDown)
            };
            view().flex_row().justify_end().child(text_link_button(
                label,
                a11y,
                Some(chevron),
                store::toggle_info_expanded,
            ))
        }))
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
        let variant = if current == num {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Secondary
        };
        grid = grid.child(view().w(px(72.0)).child(app_button(
            format!("{num}"),
            variant,
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

fn comments_section(comments: &[EpisodeComment]) -> impl IntoElement {
    let header = format!("评论 ({})", comments.len());
    let mut list = view().gap_3().child(
        text(header)
            .text_sm()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(token::fg),
    );

    if comments.is_empty() {
        return list.child(text("暂无评论").text_xs().text_color(token::muted_fg));
    }

    for comment in comments {
        list = list.child(comment_row(comment));
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
                .children((!comment.preview_replies.is_empty()).then_some(replies_box)),
        )
}
