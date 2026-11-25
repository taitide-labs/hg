use hg_contracts::catalog::{HotKeyword, RankingKind, SeriesSort, TagGroup};
use hg_contracts::ids::SeriesId;
use nsh_core::geometry::point;
use nsh_core::prelude::*;

use super::super::parts::{
    pagination_footer, ranking_grid_columns_for_width, series_flags, series_grid_columns_for_width,
    virtual_series_grid,
};
use super::super::store::{self, BrowseKind, RankingTab};
use super::query;
use crate::ui::button::{ButtonVariant, ControlSize, app_button, app_icon_text_button};
use crate::ui::icons::IconKind;
use crate::ui::input::search_input;
use crate::ui::platform::is_desktop;
use crate::ui::poster::{RankingCardProps, ranking_list_row};
use crate::ui::surface::empty_state;
use crate::ui::tabs::segmented_tabs;
use crate::ui::token;

pub fn browse_view(
    initial_kind: BrowseKind,
    on_back: impl Fn() + Copy + 'static,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
) -> impl IntoElement {
    let page_owner = Owner::current();
    let is_search_mode = matches!(
        initial_kind,
        BrowseKind::Search { .. } | BrowseKind::Tag { .. }
    );
    let is_ranking_kind = matches!(initial_kind, BrowseKind::Ranking(_));
    if is_search_mode {
        query::load_meta();
    }
    query::load_browse(initial_kind);
    let search_sig = store::search_input_signal();
    let container_width = signal(if is_desktop() { 960.0_f32 } else { 360.0_f32 });
    let is_sortable = computed(|| {
        store::browse_state().with(|s| {
            matches!(
                s.kind,
                BrowseKind::Latest | BrowseKind::Category { .. } | BrowseKind::Tag { .. }
            )
        })
    });
    let footer_state = computed(|| {
        store::browse_state().with(|s| {
            let has_items = !s.items.is_empty() || !s.ranking_items.is_empty();
            let all_loaded = !s.loading
                && !s.loading_more
                && s.error.is_none()
                && has_items
                && s.pagination.is_some_and(|p| p.page >= p.pages);
            (s.loading_more, s.load_more_error.clone(), all_loaded)
        })
    });
    let scroller = element_ref();

    let header_block = view()
        .w_full()
        .gap_5()
        .pb_5()
        .child(browse_header(is_search_mode, search_sig, on_back))
        .children(is_search_mode.then(|| dynamic(browse_filters_panel)))
        .child(when(
            move || store::browse_state().with(|s| matches!(s.kind, BrowseKind::Ranking(_))),
            move || ranking_tabs(scroller),
        ))
        .child(when(move || is_sortable.get(), move || sort_tabs(scroller)));

    let footer_block = view()
        .w_full()
        .pt_4()
        .pb_safe_area()
        .child(dynamic(move || {
            let (loading_more, load_more_error, all_loaded) = footer_state.get();
            pagination_footer(loading_more, load_more_error, all_loaded, move || {
                page_owner.run(query::load_next_browse_page);
            })
        }));

    let list_element = if is_ranking_kind {
        ranking_list(
            scroller,
            page_owner,
            container_width,
            on_back,
            on_open_series,
            header_block,
            footer_block,
        )
        .into_element()
    } else {
        series_browse_list(
            scroller,
            page_owner,
            container_width,
            on_back,
            on_open_series,
            header_block,
            footer_block,
        )
        .into_element()
    };

    view()
        .flex_1()
        .min_h_0()
        .on_layout(move |bounds: &Bounds<Points>| {
            let w = bounds.size.width.0;
            let h = bounds.size.height.0;
            store::set_viewport_size(w, h);
            if w > 0.0 && (container_width.get_untracked() - w).abs() > 1.0 {
                container_width.set(w);
            }
            page_owner.run(query::ensure_viewport_filled);
        })
        .child(list_element)
}

fn series_browse_list(
    scroller: ElementRef,
    page_owner: Owner,
    container_width: Signal<f32>,
    on_back: impl Fn() + Copy + 'static,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
    header_view: impl IntoElement,
    footer_view: impl IntoElement,
) -> impl IntoElement {
    virtual_series_grid(
        || store::browse_state().with(|s| if s.error.is_some() { 0 } else { s.items.len() }),
        || store::browse_state().with(|s| s.items.iter().map(|i| i.id).collect()),
        |idx| store::browse_state().with_untracked(|s| s.items.get(idx).cloned()),
        move || {
            let inner_w = (container_width.get() - 48.0).max(0.0);
            series_grid_columns_for_width(inner_w) as u32
        },
        on_open_series,
        move || page_owner.run(query::load_next_browse_page),
        move || {
            let (has_error, loading) =
                store::browse_state().with(|s| (s.error.is_some(), s.loading));
            if has_error {
                empty_state(
                    IconKind::Search,
                    "列表加载失败",
                    "请稍后重试或返回发现首页",
                    Some(("返回发现", Box::new(on_back))),
                )
                .into_element()
            } else if loading {
                view().into_element()
            } else {
                empty_state(
                    IconKind::Search,
                    "暂无匹配剧目",
                    "尝试更换关键词或筛选条件",
                    None,
                )
                .into_element()
            }
        },
    )
    .header(header_view)
    .footer(footer_view)
    .element_ref(&scroller)
}

fn browse_filters_panel() -> impl IntoElement {
    let meta = store::browse_meta_snapshot();
    view()
        .gap_3()
        .children((!meta.hot_keywords.is_empty()).then(|| hot_keywords_strip(meta.hot_keywords)))
        .children((!meta.tag_groups.is_empty()).then(|| tag_filter_strip(&meta.tag_groups)))
        .into_element()
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
                || {},
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
    text(move || {
        store::browse_state().with(|s| match &s.kind {
            BrowseKind::Latest => "最近更新".to_owned(),
            BrowseKind::Ranking(_) => "排行榜".to_owned(),
            BrowseKind::Category { name, .. } => name.clone(),
            BrowseKind::Tag { name, .. } => format!("标签 · {name}"),
            BrowseKind::Search { query } if query.trim().is_empty() => "搜索与标签筛选".to_owned(),
            BrowseKind::Search { query } => format!("搜索 “{query}”"),
        })
    })
    .role(AccessibilityRole::Header)
    .text_lg()
    .font_weight(FontWeight::SEMIBOLD)
    .text_color(token::fg)
}

fn sort_tabs(scroller: ElementRef) -> impl IntoElement {
    let tabs = vec![(SeriesSort::Hot, "最热"), (SeriesSort::New, "最近更新")];
    segmented_tabs(
        tabs,
        move || store::browse_state().with(|s| s.sort),
        move |next| {
            scroller.scroll_to(point(px(0.0), px(0.0)), false);
            query::switch_sort(next);
        },
    )
}

fn ranking_tabs(scroller: ElementRef) -> impl IntoElement {
    let active = store::ranking_tab_signal();
    let tabs = vec![
        (RankingTab::Hot, "热播榜"),
        (RankingTab::Potential, "潜力榜"),
        (RankingTab::Recommend, "推荐榜"),
    ];
    segmented_tabs(
        tabs,
        move || active.get(),
        move |tab| {
            scroller.scroll_to(point(px(0.0), px(0.0)), false);
            query::switch_ranking(RankingKind::from(tab));
        },
    )
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
    scroller: ElementRef,
    page_owner: Owner,
    container_width: Signal<f32>,
    on_back: impl Fn() + Copy + 'static,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
    header_view: impl IntoElement,
    footer_view: impl IntoElement,
) -> impl IntoElement {
    virtual_list(
        || {
            store::browse_state().with(|s| {
                if s.error.is_some() {
                    0
                } else {
                    s.ranking_items.len()
                }
            })
        },
        move |at: Position| {
            view().child(dynamic(move || {
                let entry = store::browse_state()
                    .with_untracked(|s| s.ranking_items.get(at.index()).cloned());
                match entry {
                    Some(entry) => {
                        let id = entry.series.id;
                        let flags = series_flags(&entry.series);
                        let meta = format!(
                            "{:.1} 分 · {} · {} 集",
                            entry.series.score,
                            entry.series.tags.join(" · "),
                            entry.series.episode_count
                        );
                        let description = (!entry.series.description.is_empty())
                            .then_some(entry.series.description);
                        ranking_list_row(
                            RankingCardProps {
                                rank: entry.rank,
                                title: entry.series.title,
                                cover_url: entry.series.cover_url,
                                flags,
                                meta,
                                description,
                            },
                            move || on_open_series(id),
                        )
                        .into_element()
                    }
                    None => view().into_element(),
                }
            }))
        },
    )
    .keys(|| {
        store::browse_state().with(|s| {
            s.ranking_items
                .iter()
                .map(|r| r.series.id)
                .collect::<Vec<_>>()
        })
    })
    .columns(move || {
        u32::try_from(ranking_grid_columns_for_width(container_width.get())).unwrap_or(1)
    })
    .column_gap(px(12.0))
    .row_gap(px(12.0))
    .content_padding(px(24.0))
    .estimated_item_size(px(122.0))
    .load_more_threshold(1.0)
    .on_load_more(move || page_owner.run(query::load_next_browse_page))
    .header(header_view)
    .footer(footer_view)
    .empty(dynamic(move || {
        let (has_error, loading) = store::browse_state().with(|s| (s.error.is_some(), s.loading));
        if has_error {
            empty_state(
                IconKind::Search,
                "列表加载失败",
                "请稍后重试或返回发现首页",
                Some(("返回发现", Box::new(on_back))),
            )
            .into_element()
        } else if loading {
            view().into_element()
        } else {
            empty_state(IconKind::Flame, "暂无榜单数据", "请切换其他榜单查看", None).into_element()
        }
    }))
    .element_ref(&scroller)
    .flex_1()
    .w_full()
    .min_h_0()
}
