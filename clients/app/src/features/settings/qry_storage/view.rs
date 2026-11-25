use nsh_core::prelude::*;

use super::super::store;
use super::query;
use crate::ui::icons::IconKind;
use crate::ui::surface::{BadgeTone, badge, empty_state, section_header};
use crate::ui::token;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoadStatus {
    Loading,
    Error,
    Loaded,
}

pub fn storage_section<A>(clear_action: impl Fn() -> A + Copy + 'static) -> impl IntoElement
where
    A: IntoElement + 'static,
{
    let section_owner = Owner::current();
    query::load_storage();

    let status = computed(|| {
        let has_storage = store::state().with(|s| s.storage.is_some());
        if has_storage {
            LoadStatus::Loaded
        } else if query::error_message().is_some() {
            LoadStatus::Error
        } else {
            LoadStatus::Loading
        }
    });

    view()
        .w_full()
        .gap_3()
        .child(section_header(
            "存储空间",
            Some("系统临时缓存与用户托管离线片库物理隔离，互不干扰"),
        ))
        .child(dynamic(move || match status.get() {
            LoadStatus::Error => empty_state(
                IconKind::Download,
                "存储占用加载失败",
                "请稍后重试获取缓存与离线片库空间信息",
                Some((
                    "重新加载",
                    Box::new(move || section_owner.run(query::load_storage)),
                )),
            )
            .into_element(),
            LoadStatus::Loading => loading_card().into_element(),
            LoadStatus::Loaded => {
                untracked(move || loaded_storage_grid(clear_action)).into_element()
            }
        }))
}

fn loading_card() -> impl IntoElement {
    view()
        .w_full()
        .p_5()
        .rounded(token::radius_card)
        .bg(token::card)
        .border_1()
        .border_color(token::border)
        .child(
            text("正在统计存储空间占用...")
                .text_sm()
                .text_color(token::muted_fg),
        )
}

fn loaded_storage_grid<A>(clear_action: impl Fn() -> A + Copy + 'static) -> impl IntoElement
where
    A: IntoElement + 'static,
{
    let cache_size = computed(|| {
        store::state().with(|s| {
            s.storage.map_or_else(SharedString::default, |ov| {
                SharedString::from(query::format_mb(query::cache_total_bytes(ov)))
            })
        })
    });
    let cache_detail = computed(|| {
        store::state().with(|s| {
            s.storage.map_or_else(SharedString::default, |ov| {
                SharedString::from(query::cache_breakdown(ov))
            })
        })
    });
    let offline_size = computed(|| {
        store::state().with(|s| {
            s.storage.map_or_else(SharedString::default, |ov| {
                SharedString::from(query::format_mb(ov.offline_library_bytes))
            })
        })
    });

    view()
        .w_full()
        .flex_row()
        .flex_wrap()
        .gap_3()
        .child(storage_card(
            "在线播放与图片缓存",
            "系统缓存",
            BadgeTone::Neutral,
            move || cache_size.get(),
            move || cache_detail.get(),
            Some(clear_action()),
        ))
        .child(storage_card(
            "离线片库",
            "托管资产",
            BadgeTone::Success,
            move || offline_size.get(),
            || SharedString::from("已下载分集与本地封面快照独立持久保存"),
            Option::<View>::None,
        ))
}

fn storage_card<A>(
    title: &'static str,
    tag_label: &'static str,
    tag_tone: BadgeTone,
    size_text: impl Fn() -> SharedString + Copy + 'static,
    detail_text: impl Fn() -> SharedString + Copy + 'static,
    action: Option<A>,
) -> impl IntoElement
where
    A: IntoElement + 'static,
{
    view()
        .flex_1()
        .min_w(px(260.0))
        .p_5()
        .gap_3()
        .rounded(token::radius_card)
        .bg(token::card)
        .border_1()
        .border_color(token::border)
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    text(title)
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(token::card_fg),
                )
                .child(badge(tag_label, tag_tone)),
        )
        .child(
            text(size_text)
                .text_xl()
                .font_weight(FontWeight::SEMIBOLD)
                .tabular_nums()
                .text_color(token::card_fg),
        )
        .child(text(detail_text).text_xs().text_color(token::muted_fg))
        .children(action)
}
