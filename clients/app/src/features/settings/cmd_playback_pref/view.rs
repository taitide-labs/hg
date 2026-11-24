use nsh_core::prelude::*;

use super::super::store;
use super::command::{AppearanceOption, PROFILE_OPTIONS, RATE_PRESETS};
use super::handler;
use crate::ui::icons::IconKind;
use crate::ui::surface::{empty_state, section_header, separator};
use crate::ui::tabs::segmented_tabs;
use crate::ui::token;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoadStatus {
    Loading,
    Error,
    Loaded,
}

pub fn playback_pref_section() -> impl IntoElement {
    handler::load_playback_preference();

    view()
        .w_full()
        .gap_6()
        .child(appearance_card())
        .child(playback_pref_card())
}

fn appearance_card() -> impl IntoElement {
    let active = handler::appearance_signal();
    let options = vec![
        (AppearanceOption::System, "跟随系统"),
        (AppearanceOption::Light, "浅色"),
        (AppearanceOption::Dark, "深色"),
    ];

    view()
        .w_full()
        .gap_3()
        .child(section_header(
            "外观",
            Some("默认跟随系统外观自动切换，或固定为浅色、深色主题"),
        ))
        .child(settings_card().child(setting_row(
            "外观模式",
            "切换应用全局浅色与深色主题表现",
            segmented_tabs(options, move || active.get(), handler::select_appearance),
        )))
}

fn playback_pref_card() -> impl IntoElement {
    let status = computed(|| {
        store::state().with(|s| {
            if s.playback_pref.is_some() {
                LoadStatus::Loaded
            } else if s.error.is_some() {
                LoadStatus::Error
            } else {
                LoadStatus::Loading
            }
        })
    });

    view()
        .w_full()
        .gap_3()
        .child(section_header(
            "播放偏好",
            Some("设置在线起播时的默认清晰度档位与默认播放倍速"),
        ))
        .child(dynamic(move || match status.get() {
            LoadStatus::Error => empty_state(
                IconKind::Play,
                "播放偏好加载失败",
                "请稍后重试获取当前播放偏好",
                Some(("重新加载", Box::new(handler::load_playback_preference))),
            )
            .into_element(),
            LoadStatus::Loading => settings_card()
                .child(
                    text("正在加载播放偏好...")
                        .text_sm()
                        .text_color(token::muted_fg),
                )
                .into_element(),
            LoadStatus::Loaded => untracked(loaded_playback_card).into_element(),
        }))
}

fn loaded_playback_card() -> impl IntoElement {
    let error_msg = computed(|| store::state().with(|s| s.error.clone()));
    let cache_limit_text = computed(|| {
        store::state().with(|s| {
            s.playback_pref.map_or_else(SharedString::default, |p| {
                SharedString::from(format!("{} MB", p.playback_cache_limit_mb))
            })
        })
    });

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
        .child(setting_row(
            "清晰度优先档位",
            "优先选择高清 HEVC 或标准兼容流档位",
            segmented_tabs(
                PROFILE_OPTIONS.to_vec(),
                handler::selected_profile,
                handler::select_profile,
            ),
        ))
        .child(separator())
        .child(setting_row(
            "默认倍速",
            "新开剧集起播时应用的默认播放倍速",
            segmented_tabs(
                RATE_PRESETS.to_vec(),
                handler::selected_rate,
                handler::select_default_rate,
            ),
        ))
        .child(separator())
        .child(setting_row(
            "在线播放缓存上限",
            "自动淘汰在线临时缓冲时的容量上限",
            text(cache_limit_text)
                .text_sm()
                .tabular_nums()
                .text_color(token::muted_fg),
        ))
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

fn setting_row(
    title: &'static str,
    description: &'static str,
    control: impl IntoElement,
) -> impl IntoElement {
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
                    text(title)
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(token::card_fg),
                )
                .child(text(description).text_xs().text_color(token::muted_fg)),
        )
        .child(control)
}
