use hg_contracts::viewing::DislikedSeriesEntry;
use nsh_core::prelude::*;

use super::super::store;
use super::handler;
use crate::ui::button::{
    ButtonVariant, ControlSize, app_button, app_icon_button, confirm_app_trigger,
    confirm_link_trigger,
};
use crate::ui::controls::confirm_popover;
use crate::ui::icons::IconKind;
use crate::ui::platform::is_desktop;
use crate::ui::poster;
use crate::ui::surface::{BadgeTone, badge, empty_state, section_header};
use crate::ui::token;

pub fn disliked_section() -> impl IntoElement {
    handler::load_disliked();
    let count_text = computed(|| {
        store::state().with(|s| {
            let count = s.disliked.len();
            if count == 0 {
                SharedString::from("暂无已标记为不喜欢的剧目")
            } else {
                SharedString::from(format!("当前已隐藏 {count} 部不喜欢的剧目"))
            }
        })
    });

    view()
        .w_full()
        .gap_3()
        .child(section_header(
            "内容偏好",
            Some("标记为不喜欢的剧目将从发现与搜索列表隐藏，可随时恢复"),
        ))
        .child(
            view()
                .w_full()
                .p_5()
                .rounded(token::radius_card)
                .bg(token::card)
                .border_1()
                .border_color(token::border)
                .flex_row()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap_4()
                .child(
                    view()
                        .flex_1()
                        .min_w(px(200.0))
                        .gap_1()
                        .child(
                            text("不喜欢的剧目")
                                .text_sm()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(token::card_fg),
                        )
                        .child(
                            text(move || count_text.get())
                                .text_xs()
                                .tabular_nums()
                                .text_color(token::muted_fg),
                        ),
                )
                .child(app_button(
                    "管理不喜欢",
                    ButtonVariant::Outline,
                    ControlSize::Sm,
                    handler::open_drawer,
                )),
        )
}

pub fn disliked_drawer() -> impl IntoElement {
    let open = computed(|| store::state().with(|s| s.disliked_drawer_open));

    when(
        move || open.get(),
        move || {
            on_key("escape", |_| handler::close_drawer());
            on_back_press(|| {
                handler::close_drawer();
                true
            });
            drawer_overlay()
        },
    )
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
        .on_press(handler::close_drawer)
}

fn desktop_drawer_panel() -> impl IntoElement {
    let enter = drawer_motion(true);
    view()
        .bind_motion(&enter)
        .w(px(460.0))
        .h_full()
        .bg(token::bg)
        .border_l_1()
        .border_color(token::border)
        .child(drawer_header())
        .child(drawer_body())
}

fn mobile_drawer_sheet() -> impl IntoElement {
    let enter = drawer_motion(false);
    view()
        .bind_motion(&enter)
        .w_full()
        .h(px(500.0))
        .bg(token::bg)
        .border_t_1()
        .border_color(token::border)
        .rounded_t(token::radius_card)
        .overflow_hidden()
        .child(drawer_header())
        .child(drawer_body())
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
    let has_items = computed(|| store::state().with(|s| !s.disliked.is_empty()));
    let drawer_owner = Owner::current();

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
                    text("不喜欢的剧目")
                        .role(AccessibilityRole::Header)
                        .text_base()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(token::fg),
                )
                .child(
                    text(move || store::state().with(|s| format!("共 {} 部", s.disliked.len())))
                        .text_xs()
                        .tabular_nums()
                        .text_color(token::muted_fg),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(when(
                    move || has_items.get(),
                    move || {
                        confirm_popover(
                            confirm_app_trigger("清空", ButtonVariant::Ghost, ControlSize::Sm),
                            "确认清空并恢复全部推荐？",
                            "确认清空",
                            move || drawer_owner.run(handler::clear_disliked),
                        )
                    },
                ))
                .child(app_icon_button(
                    IconKind::Close,
                    "关闭",
                    ButtonVariant::Ghost,
                    ControlSize::Sm,
                    handler::close_drawer,
                )),
        )
}

fn drawer_body() -> impl IntoElement {
    let body_owner = Owner::current();
    let entries = computed(|| store::state().with(|s| s.disliked.clone()));
    let status = computed(|| {
        store::state().with(|s| {
            (
                s.disliked_loading,
                s.disliked_error.clone(),
                s.disliked.is_empty(),
            )
        })
    });

    scroll()
        .flex_1()
        .min_h_0()
        .child(view().p_4().gap_3().child(dynamic(move || {
            let (loading, error, empty) = status.get();
            if error.is_some() && empty {
                return empty_state(
                    IconKind::ThumbsDown,
                    "不喜欢列表加载失败",
                    "请稍后重试",
                    Some((
                        "重新加载",
                        Box::new(move || body_owner.run(handler::load_disliked)),
                    )),
                )
                .into_element();
            }
            if loading && empty {
                return view()
                    .py_6()
                    .items_center()
                    .child(
                        text("正在加载不喜欢列表...")
                            .text_sm()
                            .text_color(token::muted_fg),
                    )
                    .into_element();
            }
            if empty {
                return empty_state(
                    IconKind::ThumbsDown,
                    "暂无不喜欢的剧目",
                    "在海报右键菜单或播放页点击“不喜欢”的剧目会显示在这里",
                    None,
                )
                .into_element();
            }

            view()
                .w_full()
                .gap_2p5()
                .child(each(
                    entries,
                    |e: &DislikedSeriesEntry| e.series_id,
                    move |entry_sig, index_sig| disliked_row(index_sig, entry_sig, body_owner),
                ))
                .into_element()
        })))
}

fn disliked_row(
    index_sig: ReadSignal<usize>,
    entry_sig: ReadSignal<DislikedSeriesEntry>,
    row_owner: Owner,
) -> impl IntoElement {
    let enter = token::stagger_enter(index_sig.get_untracked());
    let (series_id, title, episode_count, is_finished, score) = entry_sig.with_untracked(|e| {
        (
            e.series_id,
            e.title.clone(),
            e.episode_count,
            e.is_finished,
            e.score,
        )
    });
    let cover_key = computed(move || entry_sig.with(|e| e.cover_url.clone()));
    let episode_label = if is_finished {
        format!("全 {episode_count} 集")
    } else {
        format!("更新至 {episode_count} 集")
    };
    let a11y_label = format!("取消不喜欢 {title}");

    view()
        .bind_motion(&enter)
        .w_full()
        .rounded(token::radius_card)
        .bg(token::card)
        .border_1()
        .border_color(token::border)
        .p_3()
        .flex_row()
        .items_center()
        .gap_3p5()
        .child(poster::reactive_poster_thumb(
            cover_key,
            52.0,
            70.0,
            IconKind::Play,
            26.0,
            12.0,
        ))
        .child(
            view()
                .flex_1()
                .min_w_0()
                .gap_1p5()
                .child(
                    text(title.clone())
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(token::card_fg)
                        .truncate(),
                )
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(badge(format!("{score:.1} 分"), BadgeTone::Neutral))
                        .child(
                            text(episode_label)
                                .text_xs()
                                .tabular_nums()
                                .text_color(token::muted_fg),
                        ),
                ),
        )
        .child(confirm_popover(
            confirm_link_trigger("取消不喜欢", a11y_label, Some(IconKind::Close)),
            "确认移出不喜欢列表？",
            "确认移除",
            move || row_owner.run(move || handler::undislike(series_id)),
        ))
}
