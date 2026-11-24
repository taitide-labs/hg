use nsh_core::prelude::*;

use super::super::store;
use super::command::CONCURRENCY_PRESETS;
use super::handler;
use crate::ui::icons::IconKind;
use crate::ui::surface::{empty_state, section_header, separator, toggle_switch_row};
use crate::ui::tabs::segmented_tabs;
use crate::ui::token;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoadStatus {
    Loading,
    Error,
    Loaded,
}

pub fn download_policy_section() -> impl IntoElement {
    handler::load_download_policy();

    let status = computed(|| {
        let has_policy = store::state().with(|s| s.download_policy.is_some());
        if has_policy {
            LoadStatus::Loaded
        } else if handler::error_message().is_some() {
            LoadStatus::Error
        } else {
            LoadStatus::Loading
        }
    });

    view()
        .w_full()
        .gap_3()
        .child(section_header(
            "下载策略",
            Some("配置离线分集下载的网络限制与最大并发任务数"),
        ))
        .child(dynamic(move || match status.get() {
            LoadStatus::Error => empty_state(
                IconKind::Download,
                "下载策略加载失败",
                "请稍后重试获取离线下载策略",
                Some(("重新加载", Box::new(handler::load_download_policy))),
            )
            .into_element(),
            LoadStatus::Loading => settings_card()
                .child(
                    text("正在加载下载策略...")
                        .text_sm()
                        .text_color(token::muted_fg),
                )
                .into_element(),
            LoadStatus::Loaded => untracked(loaded_policy_card).into_element(),
        }))
}

fn loaded_policy_card() -> impl IntoElement {
    let concurrency_sig = handler::concurrency_signal();
    let error_msg = computed(handler::error_message);
    let wifi_only = || {
        store::state().with(|s| {
            s.download_policy
                .as_ref()
                .is_some_and(|policy| policy.wifi_only)
        })
    };

    settings_card()
        .child(when(
            move || error_msg.with(Option::is_some),
            move || {
                text(move || error_msg.get().unwrap_or_default())
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(token::destructive)
            },
        ))
        .child(
            view()
                .w_full()
                .gap_1()
                .child(toggle_switch_row(
                    "仅 Wi-Fi 下载",
                    wifi_only,
                    false,
                    handler::toggle_wifi_only,
                ))
                .child(
                    text("开启后仅在 Wi-Fi 网络下执行离线缓存与队列下载")
                        .text_xs()
                        .text_color(token::muted_fg)
                        .px_2p5(),
                ),
        )
        .child(separator())
        .child(
            view()
                .w_full()
                .flex_row()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap_4()
                .child(
                    view()
                        .flex_1()
                        .min_w(px(200.0))
                        .gap_0p5()
                        .child(
                            text("最大并发下载任务数")
                                .text_sm()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(token::card_fg),
                        )
                        .child(
                            text("控制同时进行的离线分集下载任务数量上限")
                                .text_xs()
                                .text_color(token::muted_fg),
                        ),
                )
                .child(segmented_tabs(
                    CONCURRENCY_PRESETS.to_vec(),
                    concurrency_sig,
                    handler::select_max_concurrent,
                )),
        )
}

fn settings_card() -> View {
    view()
        .w_full()
        .p_5()
        .gap_4()
        .rounded(token::radius_card)
        .bg(token::card)
        .border_1()
        .border_color(token::border)
}
