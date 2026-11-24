use hg_contracts::catalog::{HotKeyword, RankingEntry, RankingKind, TagGroup};
use hg_contracts::ids::SeriesId;
use nsh_core::prelude::*;

use super::super::parts::series_grid;
use super::super::store::{self, BrowseKind, RankingTab};
use super::query;
use crate::ui::button::{ButtonVariant, ControlSize, app_button, app_icon_text_button};
use crate::ui::icons::IconKind;
use crate::ui::input::search_input;
use crate::ui::poster::{RankingCardProps, ranking_list_row};
use crate::ui::surface::empty_state;
use crate::ui::tabs::segmented_tabs;
use crate::ui::token;

pub fn browse_view(
    initial_kind: BrowseKind,
    on_back: impl Fn() + Copy + 'static,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
) -> impl IntoElement {
    let is_search_mode = matches!(
        initial_kind,
        BrowseKind::Search { .. } | BrowseKind::Tag { .. }
    );
    if is_search_mode {
        query::load_meta();
    }
    query::load_browse(initial_kind);
    let search_sig = store::search_input_signal();
    let is_ranking = computed(|| matches!(store::browse_snapshot().kind, BrowseKind::Ranking(_)));

    scroll().flex_1().child(
        view()
            .p_6()
            .gap_5()
            .pb_safe_area_plus(px(24.0))
            .child(browse_header(is_search_mode, search_sig, on_back))
            .children(is_search_mode.then(|| dynamic(browse_filters_panel)))
            .children((!is_search_mode).then(move || when(move || is_ranking.get(), ranking_tabs)))
            .child(dynamic(move || {
                browse_results_panel(on_back, on_open_series)
            })),
    )
}

fn browse_filters_panel() -> impl IntoElement {
    let meta = store::browse_meta_snapshot();
    view()
        .gap_3()
        .children((!meta.hot_keywords.is_empty()).then(|| hot_keywords_strip(meta.hot_keywords)))
        .children((!meta.tag_groups.is_empty()).then(|| tag_filter_strip(&meta.tag_groups)))
        .into_element()
}

fn browse_results_panel(
    on_back: impl Fn() + Copy + 'static,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
) -> impl IntoElement {
    let snap = store::browse_snapshot();
    if snap.error.is_some() {
        return empty_state(
            IconKind::Search,
            "列表加载失败",
            "请稍后重试或返回发现首页",
            Some(("返回发现", Box::new(on_back))),
        )
        .into_element();
    }

    match snap.kind {
        BrowseKind::Ranking(_) => ranking_list(snap.ranking_items, on_open_series).into_element(),
        BrowseKind::Featured
        | BrowseKind::Category { .. }
        | BrowseKind::Tag { .. }
        | BrowseKind::Search { .. } => series_grid(snap.items, on_open_series).into_element(),
    }
}

fn browse_header(
    is_search_mode: bool,
    search_sig: Signal<String>,
    on_back: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let back = app_icon_text_button(
        IconKind::ChevronLeft,
        "返回",
        ButtonVariant::Ghost,
        ControlSize::Md,
        on_back,
    );

    if is_search_mode {
        view()
            .w_full()
            .flex_row()
            .items_center()
            .gap_3()
            .child(back)
            .child(view().flex_1().max_w(px(480.0)).child(search_input(
                search_sig,
                "搜索剧名、标签...",
                move |submitted| {
                    query::load_browse(BrowseKind::Search { query: submitted });
                },
            )))
            .child(app_button(
                "搜索",
                ButtonVariant::Primary,
                ControlSize::Md,
                move || {
                    query::load_browse(BrowseKind::Search {
                        query: search_sig.get(),
                    });
                },
            ))
            .into_element()
    } else {
        view()
            .w_full()
            .flex_row()
            .items_center()
            .gap_3()
            .child(back)
            .child(title_bar())
            .into_element()
    }
}

fn title_bar() -> impl IntoElement {
    text(move || match store::browse_snapshot().kind {
        BrowseKind::Featured => "精选推荐".to_owned(),
        BrowseKind::Ranking(_) => "排行榜".to_owned(),
        BrowseKind::Category { name, .. } => name,
        BrowseKind::Tag { name, .. } => format!("标签 · {name}"),
        BrowseKind::Search { query } if query.trim().is_empty() => "搜索与标签筛选".to_owned(),
        BrowseKind::Search { query } => format!("搜索 “{query}”"),
    })
    .role(AccessibilityRole::Header)
    .text_lg()
    .font_weight(FontWeight::SEMIBOLD)
    .text_color(token::fg)
}

fn ranking_tabs() -> impl IntoElement {
    let active = store::ranking_tab_signal();
    let tabs = vec![
        (RankingTab::Hot, "热播榜"),
        (RankingTab::Potential, "潜力榜"),
        (RankingTab::Recommend, "推荐榜"),
    ];
    segmented_tabs(tabs, active, |tab| {
        query::switch_ranking(RankingKind::from(tab));
    })
}

fn hot_keywords_strip(keywords: Vec<HotKeyword>) -> impl IntoElement {
    let mut row = view().flex_row().flex_wrap().items_center().gap_2().child(
        text("热搜")
            .text_xs()
            .font_weight(FontWeight::MEDIUM)
            .text_color(token::muted_fg),
    );
    for kw in keywords {
        let word = kw.word;
        let query_word = word.clone();
        row = row.child(app_button(
            word,
            ButtonVariant::Outline,
            ControlSize::Sm,
            move || {
                query::load_browse(BrowseKind::Search {
                    query: query_word.clone(),
                });
            },
        ));
    }
    row
}

fn tag_filter_strip(groups: &[TagGroup]) -> impl IntoElement {
    let mut container = view().gap_2();
    for group in groups {
        let mut row = view().flex_row().flex_wrap().items_center().gap_2().child(
            text(group.name.clone())
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(token::muted_fg),
        );
        for tag in &group.tags {
            let slug = tag.slug.clone();
            let name = tag.name.clone();
            row = row.child(app_button(
                tag.name.clone(),
                ButtonVariant::Secondary,
                ControlSize::Sm,
                move || {
                    query::load_browse(BrowseKind::Tag {
                        slug: slug.clone(),
                        name: name.clone(),
                    });
                },
            ));
        }
        container = container.child(row);
    }
    container
}

fn ranking_list(
    entries: Vec<RankingEntry>,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
) -> impl IntoElement {
    if entries.is_empty() {
        return empty_state(IconKind::Flame, "暂无榜单数据", "请切换其他榜单查看", None)
            .into_element();
    }

    let mut list = view().gap_2p5();
    for entry in entries {
        let id = entry.series.id;
        let meta = format!(
            "{:.1} 分 · {} · {} 集",
            entry.series.score,
            entry.series.tags.join(" · "),
            entry.series.episode_count
        );
        let description =
            (!entry.series.description.is_empty()).then_some(entry.series.description);
        list = list.child(ranking_list_row(
            RankingCardProps {
                rank: entry.rank,
                title: entry.series.title,
                cover_url: entry.series.cover_url,
                meta,
                description,
            },
            move || on_open_series(id),
        ));
    }
    list.into_element()
}
