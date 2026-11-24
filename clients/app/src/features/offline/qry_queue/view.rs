use hg_contracts::ids::EpisodeKey;
use hg_contracts::offline::{DownloadTaskState, DownloadTaskView};
use nsh_core::prelude::*;

use super::super::store;
use super::query::{
    self, TaskStatusTone, progress_detail_text, progress_ratio, queue_summary_text, state_badge,
};
use crate::ui::icons::{IconKind, icon};
use crate::ui::surface::{BadgeTone, badge, empty_state, progress_track};
use crate::ui::token;

pub fn download_queue_section<H, R>(
    header_action: impl Fn() -> H + Copy + 'static,
    task_action: impl Fn(EpisodeKey, DownloadTaskState) -> R + Copy + 'static,
) -> impl IntoElement
where
    H: IntoElement + 'static,
    R: IntoElement + 'static,
{
    query::load_queue();
    view()
        .w_full()
        .gap_3()
        .child(dynamic(move || queue_body(header_action, task_action)))
}

fn queue_body<H, R>(
    header_action: impl Fn() -> H + Copy + 'static,
    task_action: impl Fn(EpisodeKey, DownloadTaskState) -> R + Copy + 'static,
) -> impl IntoElement
where
    H: IntoElement + 'static,
    R: IntoElement + 'static,
{
    let snap = store::state().get();
    let summary = (!snap.tasks.is_empty()).then(|| queue_summary_text(&snap.tasks));
    let header = section_header_row(summary, !snap.tasks.is_empty(), header_action);

    if snap.error.is_some() && snap.tasks.is_empty() {
        return view()
            .w_full()
            .gap_3()
            .child(header)
            .child(empty_state(
                IconKind::Download,
                "下载队列加载失败",
                "请稍后重试刷新下载任务",
                Some(("重新加载", Box::new(query::load_queue))),
            ))
            .into_element();
    }

    if snap.loading_queue && snap.tasks.is_empty() {
        return view()
            .w_full()
            .gap_3()
            .child(header)
            .child(
                view().py_6().items_center().child(
                    text("正在加载下载队列...")
                        .text_sm()
                        .text_color(token::muted_fg),
                ),
            )
            .into_element();
    }

    if snap.tasks.is_empty() {
        return view()
            .w_full()
            .gap_3()
            .child(header)
            .child(empty_state(
                IconKind::Download,
                "暂无进行中的下载任务",
                "在播放页点击离线下载后，可在此查看分片进度",
                None,
            ))
            .into_element();
    }

    let mut list = view().w_full().gap_2p5();
    for (index, task) in snap.tasks.into_iter().enumerate() {
        list = list.child(task_row(index, task, task_action));
    }

    view()
        .w_full()
        .gap_3()
        .child(header)
        .child(list)
        .into_element()
}

fn section_header_row<H>(
    summary: Option<String>,
    show_action: bool,
    header_action: impl Fn() -> H + Copy + 'static,
) -> impl IntoElement
where
    H: IntoElement + 'static,
{
    view()
        .w_full()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .child(
            view()
                .flex_row()
                .items_baseline()
                .gap_3()
                .child(
                    text("下载任务队列")
                        .role(AccessibilityRole::Header)
                        .text_base()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(token::fg),
                )
                .children(summary.map(|label| {
                    text(label)
                        .text_xs()
                        .tabular_nums()
                        .text_color(token::muted_fg)
                })),
        )
        .children(show_action.then(header_action))
}

fn map_tone(tone: TaskStatusTone) -> BadgeTone {
    match tone {
        TaskStatusTone::Brand => BadgeTone::Brand,
        TaskStatusTone::Neutral => BadgeTone::Neutral,
        TaskStatusTone::Warning => BadgeTone::Warning,
        TaskStatusTone::Success => BadgeTone::Success,
    }
}

fn task_row<R>(
    index: usize,
    task: DownloadTaskView,
    task_action: impl Fn(EpisodeKey, DownloadTaskState) -> R + Copy + 'static,
) -> impl IntoElement
where
    R: IntoElement + 'static,
{
    let enter = token::stagger_enter(index);
    let key = task.key;
    let state = task.state;
    let (state_label, tone) = state_badge(state);
    let ratio = progress_ratio(state);
    let detail = progress_detail_text(state, task.profile);

    view()
        .bind_motion(&enter)
        .w_full()
        .p_3()
        .rounded(token::radius_card)
        .bg(token::card)
        .border_1()
        .border_color(token::border)
        .flex_row()
        .items_center()
        .gap_3p5()
        .child(poster_thumb(task.cover_url))
        .child(
            view()
                .flex_1()
                .min_w_0()
                .gap_1p5()
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(
                            text(task.episode_title)
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(token::card_fg)
                                .truncate(),
                        )
                        .child(
                            text(task.series_title)
                                .text_xs()
                                .text_color(token::muted_fg)
                                .truncate(),
                        ),
                )
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(badge(state_label, map_tone(tone)))
                        .child(
                            text(detail)
                                .text_xs()
                                .tabular_nums()
                                .text_color(token::muted_fg)
                                .truncate(),
                        ),
                )
                .child(progress_track(move || ratio)),
        )
        .child(task_action(key, state))
}

fn poster_thumb(cover_key: String) -> impl IntoElement {
    view()
        .w(px(46.0))
        .h(px(62.0))
        .flex_shrink_0()
        .rounded(token::radius)
        .bg(move || token::poster_surface(&cover_key))
        .items_center()
        .justify_center()
        .child(
            view()
                .size(px(24.0))
                .rounded_full()
                .bg(token::cinema_scrim)
                .items_center()
                .justify_center()
                .child(icon(IconKind::Download, 11.0, token::cinema_fg)),
        )
}
