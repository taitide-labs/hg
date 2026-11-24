use hg_contracts::catalog::{HomeBanner, SeriesSummary};
use hg_contracts::ids::SeriesId;
use nsh_core::prelude::*;

use super::super::store::{self, CategoryTab};
use super::query;
use crate::ui::button::{ButtonVariant, ControlSize, app_button};
use crate::ui::icons::IconKind;
use crate::ui::poster::{SeriesCardProps, series_card};
use crate::ui::surface::{BadgeTone, badge, empty_state, section_header};
use crate::ui::tabs::segmented_tabs;
use crate::ui::token;

pub fn home_feed_view(on_open_series: impl Fn(SeriesId) + Copy + 'static) -> impl IntoElement {
    query::load_home();
    let active_cat = store::active_category_signal();
    let categories = vec![
        (CategoryTab::All, "全部"),
        (CategoryTab::Drama, "短剧"),
        (CategoryTab::Anime, "漫剧"),
        (CategoryTab::Suspense, "悬疑"),
        (CategoryTab::Costume, "古风"),
    ];

    scroll().flex_1().child(
        view()
            .p_6()
            .gap_6()
            .pb_safe_area_plus(px(24.0))
            .child(
                view()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .child(section_header(
                        "发现剧场",
                        Some("免登录清透片库 · 支持离线与双模观影"),
                    ))
                    .child(segmented_tabs(
                        categories,
                        active_cat,
                        query::select_category,
                    )),
            )
            .child(dynamic(move || {
                let snap = store::snapshot();
                if snap.error.is_some() {
                    return empty_state(
                        IconKind::Discover,
                        "片库加载失败",
                        "请稍后重试刷新发现内容",
                        Some(("重新加载", Box::new(query::load_home))),
                    )
                    .into_element();
                }

                let banner = if snap.active_category == CategoryTab::All {
                    snap.feed.as_ref().and_then(|f| f.banners.first()).cloned()
                } else {
                    None
                };

                view()
                    .gap_6()
                    .children(banner.map(|b| spotlight_banner(b, on_open_series)))
                    .child(section_header("精选推荐", Some("点击海报直接进入播放页")))
                    .child(series_grid(snap.items, on_open_series))
                    .into_element()
            })),
    )
}

fn spotlight_banner(
    banner: HomeBanner,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
) -> impl IntoElement {
    let enter = token::fade_enter();
    let series_id = banner.series_id;
    let cover_key = banner.image_url;
    let score = format!("评分 {:.1}", banner.score);

    view()
        .bind_motion(&enter)
        .w_full()
        .p_5()
        .rounded(token::radius_card)
        .bg(token::card)
        .border_1()
        .border_color(token::border)
        .flex_row()
        .items_center()
        .gap_5()
        .child(
            view()
                .w(px(148.0))
                .h(px(110.0))
                .rounded(token::radius)
                .bg(move || token::poster_surface(&cover_key))
                .p_2p5()
                .justify_between()
                .child(badge("今日焦点", BadgeTone::Brand))
                .child(badge(score, BadgeTone::Cinema)),
        )
        .child(
            view()
                .flex_1()
                .gap_2()
                .child(
                    text(banner.title)
                        .text_xl()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(token::card_fg),
                )
                .child(
                    text(banner.description)
                        .text_sm()
                        .text_color(token::muted_fg),
                )
                .child(view().pt_1().flex_row().gap_2().child(app_button(
                    "立即观看",
                    ButtonVariant::Primary,
                    ControlSize::Md,
                    move || on_open_series(series_id),
                ))),
        )
}

fn series_grid(
    items: Vec<SeriesSummary>,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
) -> impl IntoElement {
    if items.is_empty() {
        return empty_state(
            IconKind::Search,
            "暂无匹配剧目",
            "尝试切换其他分类查看全部精选内容",
            None,
        )
        .into_element();
    }

    let mut grid = view().flex_row().flex_wrap().gap_4();
    for (index, item) in items.into_iter().enumerate() {
        let id = item.id;
        let status = if item.is_finished {
            format!("全 {} 集", item.episode_count)
        } else {
            format!("更新至 {} 集", item.episode_count)
        };
        let subtitle = item.tags.join(" · ");
        grid = grid.child(series_card(
            SeriesCardProps {
                title: item.title,
                cover_url: item.cover_url,
                score: item.score,
                subtitle,
                episode_label: status,
                badge_text: item.badge,
                index,
            },
            move || on_open_series(id),
        ));
    }
    grid.into_element()
}
