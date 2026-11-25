use std::time::Duration;

use hg_contracts::ids::EpisodeKey;
use hg_contracts::offline::{DownloadTaskState, DownloadTaskView};
use nsh_core::prelude::*;

use super::super::qry_library::query as library_query;
use super::super::store;
use super::query::{
    self, TaskStatusTone, progress_detail_text, progress_ratio, queue_summary_text, state_badge,
};
use crate::ui::controls::bind_interval;
use crate::ui::icons::IconKind;
use crate::ui::poster;
use crate::ui::surface::{BadgeTone, badge, empty_state, progress_track};
use crate::ui::token;

const QUEUE_POLL_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, PartialEq, Eq)]
enum QueuePhase {
    Error,
    Empty,
    Ready,
}

pub fn download_queue_section<H, R>(
    header_action: impl Fn() -> H + Copy + 'static,
    task_action: impl Fn(EpisodeKey, DownloadTaskState) -> R + Copy + 'static,
) -> impl IntoElement
where
    H: IntoElement + 'static,
    R: IntoElement + 'static,
{
    let section_owner = Owner::current();
    query::load_queue();
    bind_interval(QUEUE_POLL_INTERVAL, store::has_running_tasks, move || {
        section_owner.run(|| {
            query::load_queue();
            library_query::load_library();
        });
    });
    let phase = computed(|| {
        store::state().with(|s| {
            if s.queue_error.is_some() && s.tasks.is_empty() {
                QueuePhase::Error
            } else if s.tasks.is_empty() {
                QueuePhase::Empty
            } else {
                QueuePhase::Ready
            }
        })
    });
    let tasks = computed(|| store::state().with(|s| s.tasks.clone()));
    let summary = computed(|| {
        store::state().with(|s| {
            if s.tasks.is_empty() {
                SharedString::default()
            } else {
                SharedString::from(queue_summary_text(&s.tasks))
            }
        })
    });

    view().w_full().child(dynamic(move || match phase.get() {
        QueuePhase::Error => view()
            .w_full()
            .pb_6()
            .gap_3()
            .child(section_header_row(summary, false, header_action))
            .child(empty_state(
                IconKind::Download,
                "下载队列加载失败",
                "请稍后重试刷新下载任务",
                Some((
                    "重新加载",
                    Box::new(move || section_owner.run(query::load_queue)),
                )),
            ))
            .into_element(),
        QueuePhase::Empty => view().into_element(),
        QueuePhase::Ready => view()
            .w_full()
            .pb_6()
            .gap_3()
            .child(section_header_row(summary, true, header_action))
            .child(view().w_full().gap_2p5().child(each(
                tasks,
                |task: &DownloadTaskView| task.key,
                move |task_sig, _idx| task_row(task_sig, task_action),
            )))
            .into_element(),
    }))
}

fn section_header_row<H>(
    summary: Memo<SharedString>,
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
                    text("下载中")
                        .role(AccessibilityRole::Header)
                        .text_base()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(token::fg),
                )
                .child(
                    text(summary)
                        .style_when(move || summary.with(|s| s.is_empty()), |s| s.hidden())
                        .text_xs()
                        .tabular_nums()
                        .text_color(token::muted_fg),
                ),
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
    task_sig: ReadSignal<DownloadTaskView>,
    task_action: impl Fn(EpisodeKey, DownloadTaskState) -> R + Copy + 'static,
) -> impl IntoElement
where
    R: IntoElement + 'static,
{
    let key = task_sig.with_untracked(|t| t.key);
    let cover_url = computed(move || task_sig.with(|t| t.cover_url.clone()));
    let badge_meta = computed(move || {
        let (label, tone) = state_badge(task_sig.with(|t| t.state));
        (label, map_tone(tone))
    });
    let action_variant_key = computed(move || match task_sig.with(|t| t.state) {
        DownloadTaskState::Downloading { .. } => 0_u8,
        DownloadTaskState::FetchingSource => 1,
        DownloadTaskState::Queued => 2,
        DownloadTaskState::Paused { .. } => 3,
        DownloadTaskState::Failed { .. } => 4,
        DownloadTaskState::Completed => 5,
        DownloadTaskState::Cancelled => 6,
    });

    view()
        .w_full()
        .p_3()
        .rounded(token::radius_card)
        .bg(token::card)
        .border_1()
        .border_color(token::border)
        .flex_row()
        .items_center()
        .gap_3p5()
        .child(poster_thumb(cover_url))
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
                            text(move || {
                                task_sig.with(|t| SharedString::from(t.episode_title.clone()))
                            })
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(token::card_fg)
                            .truncate(),
                        )
                        .child(
                            text(move || {
                                task_sig.with(|t| SharedString::from(t.series_title.clone()))
                            })
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
                        .child(dynamic(move || {
                            let (label, tone) = badge_meta.get();
                            badge(label, tone)
                        }))
                        .child(
                            text(move || {
                                task_sig.with(|t| {
                                    SharedString::from(progress_detail_text(t.state, t.profile))
                                })
                            })
                            .text_xs()
                            .tabular_nums()
                            .text_color(token::muted_fg)
                            .truncate(),
                        ),
                )
                .child(progress_track(move || {
                    progress_ratio(task_sig.with(|t| t.state))
                })),
        )
        .child(dynamic(move || {
            let _ = action_variant_key.get();
            let state = task_sig.with_untracked(|t| t.state);
            task_action(key, state)
        }))
}

fn poster_thumb(cover_key: Memo<String>) -> impl IntoElement {
    poster::reactive_poster_thumb(cover_key, 46.0, 62.0, IconKind::Download, 24.0, 11.0)
}
