use hg_contracts::catalog::{CommentReply, EpisodeComment};
use nsh_core::prelude::*;

use super::super::store;
use super::query::{
    self, CommentRepliesState, format_parent_meta, format_reply_meta, nickname_initial,
};
use crate::ui::button::{ButtonVariant, ControlSize, app_button, app_icon_button};
use crate::ui::icons::IconKind;
use crate::ui::platform::is_desktop;
use crate::ui::surface::{BadgeTone, avatar_circle, badge, empty_state, separator};
use crate::ui::token;

pub fn comment_replies_drawer() -> impl IntoElement {
    query::reset_state();
    let active_target = computed(store::active_replies_target);

    dynamic(move || {
        let Some((series_id, comment_id)) = active_target.get() else {
            query::reset_state();
            return view().into_element();
        };
        untracked(move || query::open_from_player_store(series_id, comment_id));
        on_key("escape", |_| query::close());
        on_back_press(|| {
            query::close();
            true
        });
        drawer_overlay().into_element()
    })
}

fn drawer_overlay() -> impl IntoElement {
    if is_desktop() {
        view()
            .absolute()
            .inset_0()
            .flex_row()
            .child(scrim_dismiss())
            .child(desktop_drawer_panel())
            .into_element()
    } else {
        view()
            .absolute()
            .inset_0()
            .child(scrim_dismiss())
            .child(mobile_drawer_sheet())
            .into_element()
    }
}

fn scrim_dismiss() -> impl IntoElement {
    view()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .bg(token::cinema_scrim)
        .on_press(query::close)
}

fn desktop_drawer_panel() -> impl IntoElement {
    let enter = drawer_motion(true);
    view()
        .bind_motion(&enter)
        .w(px(396.0))
        .h_full()
        .bg(token::bg)
        .border_l_1()
        .border_color(token::border)
        .child(drawer_header())
        .child(drawer_scroll_body())
}

fn mobile_drawer_sheet() -> impl IntoElement {
    let enter = drawer_motion(false);
    view()
        .bind_motion(&enter)
        .w_full()
        .h(px(460.0))
        .bg(token::bg)
        .border_t_1()
        .border_color(token::border)
        .rounded_t(token::radius_card)
        .overflow_hidden()
        .child(drawer_header())
        .child(drawer_scroll_body())
}

fn drawer_motion(desktop: bool) -> Motion {
    if reduce_motion() {
        return token::fade_enter();
    }
    let preset = if desktop {
        MotionPreset::FadeInRight
    } else {
        MotionPreset::FadeInUp
    };
    motion().entering(preset.spec().timing(|mut t| {
        t.duration_ms = token::BASE_MS;
        t
    }))
}

fn drawer_header() -> impl IntoElement {
    view()
        .w_full()
        .px_4()
        .py_3()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .border_b_1()
        .border_color(token::border)
        .child(
            view()
                .flex_row()
                .items_baseline()
                .gap_2()
                .child(
                    text("全部回复")
                        .role(AccessibilityRole::Header)
                        .text_base()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(token::fg),
                )
                .child(
                    text(move || {
                        let count = query::snapshot().total_count;
                        if count > 0 {
                            format!("{count} 条")
                        } else {
                            String::new()
                        }
                    })
                    .text_xs()
                    .tabular_nums()
                    .text_color(token::muted_fg),
                ),
        )
        .child(app_icon_button(
            IconKind::Close,
            "关闭",
            ButtonVariant::Ghost,
            ControlSize::Sm,
            query::close,
        ))
}

fn drawer_scroll_body() -> impl IntoElement {
    scroll()
        .flex_1()
        .min_h_0()
        .child(view().p_4().gap_4().child(dynamic(drawer_body_content)))
}

fn drawer_body_content() -> impl IntoElement {
    let snap = query::snapshot();
    let parent_block = snap.parent_comment.as_ref().map(|comment| {
        view()
            .gap_3()
            .child(parent_comment_section(comment))
            .child(separator())
    });

    view()
        .gap_3()
        .children(parent_block)
        .child(replies_body(&snap))
}

fn parent_comment_section(comment: &EpisodeComment) -> impl IntoElement {
    let meta = format_parent_meta(comment);
    view()
        .p_3()
        .rounded(token::radius)
        .bg(token::card)
        .border_1()
        .border_color(token::border)
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
                .child(
                    text(meta)
                        .text_xs()
                        .tabular_nums()
                        .text_color(token::muted_fg),
                ),
        )
}

fn replies_body(snap: &CommentRepliesState) -> impl IntoElement {
    if snap.loading && snap.replies.is_empty() {
        return view()
            .py_6()
            .items_center()
            .child(
                text("正在加载回复...")
                    .text_sm()
                    .text_color(token::muted_fg),
            )
            .into_element();
    }

    if snap.error.is_some() && snap.replies.is_empty() {
        return empty_state(
            IconKind::Message,
            "回复加载失败",
            "请检查网络后重试",
            Some(("重新加载", Box::new(query::retry))),
        )
        .into_element();
    }

    if snap.replies.is_empty() {
        return empty_state(
            IconKind::Message,
            "暂无回复",
            "该评论暂无更多楼中楼回复",
            None,
        )
        .into_element();
    }

    let mut list = view().gap_3();
    for reply in &snap.replies {
        list = list.child(reply_row(reply));
    }

    if snap.has_more {
        let label = if snap.loading_more {
            "加载中..."
        } else {
            "加载更多"
        };
        list = list.child(view().pt_1().items_center().child(app_button(
            label,
            ButtonVariant::Secondary,
            ControlSize::Sm,
            query::load_more,
        )));
    } else if snap.error.is_some() {
        list = list.child(view().pt_1().items_center().child(app_button(
            "加载更多失败，重试",
            ButtonVariant::Secondary,
            ControlSize::Sm,
            query::load_more,
        )));
    }

    list.into_element()
}

fn reply_row(reply: &CommentReply) -> impl IntoElement {
    let meta = format_reply_meta(reply);
    let reply_target = reply.reply_to_nickname.as_ref().map(|target| {
        text(format!("回复 {target}"))
            .text_xs()
            .text_color(token::muted_fg)
    });

    view()
        .flex_row()
        .items_start()
        .gap_2p5()
        .child(avatar_circle(
            nickname_initial(&reply.nickname),
            reply.avatar_color.clone(),
        ))
        .child(
            view()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(
                    view()
                        .flex_row()
                        .flex_wrap()
                        .items_center()
                        .gap_1p5()
                        .child(
                            text(reply.nickname.clone())
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(token::fg),
                        )
                        .children(reply_target),
                )
                .child(text(reply.text.clone()).text_sm().text_color(token::fg))
                .child(
                    text(meta)
                        .text_xs()
                        .tabular_nums()
                        .text_color(token::muted_fg),
                ),
        )
}
