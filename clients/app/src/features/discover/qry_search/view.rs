use hg_contracts::catalog::{HotKeyword, SeriesSort, TagGroup};
use hg_contracts::ids::SeriesId;
use hg_contracts::viewing::SearchHistoryEntry;
use nsh_core::prelude::*;

use super::super::parts::{pagination_footer, series_grid};
use super::super::store::{self, SearchTarget};
use super::query;
use crate::ui::button::{ButtonVariant, ControlSize, app_button, text_link_button};
use crate::ui::controls::{chip_remove_button, submit_button, tag_chip_button};
use crate::ui::icons::IconKind;
use crate::ui::input::search_input;
use crate::ui::surface::empty_state;
use crate::ui::tabs::segmented_tabs;
use crate::ui::token;

const INFINITE_SCROLL_THRESHOLD_PX: f32 = 240.0;

pub fn search_view(on_open_series: impl Fn(SeriesId) + Copy + 'static) -> impl IntoElement {
    let page_owner = Owner::current();
    query::init_search_page();
    let search_sig = store::search_input_signal();

    scroll()
        .flex_1()
        .on_scroll(move |e: &ScrollEvent| {
            if e.content_size.height > px(0.0)
                && e.offset.y + e.size.height
                    >= e.content_size.height - px(INFINITE_SCROLL_THRESHOLD_PX)
            {
                page_owner.run(query::load_next_search_page);
            }
        })
        .child(
            view()
                .w_full()
                .p_6()
                .gap_5()
                .pb_safe_area_plus(px(24.0))
                .child(search_bar_row(search_sig, page_owner))
                .child(
                    view()
                        .w_full()
                        .gap_3()
                        .child(dynamic(move || search_history_section(page_owner)))
                        .child(dynamic(move || hot_keywords_section(page_owner)))
                        .child(dynamic(move || tag_filter_section(page_owner))),
                )
                .child(dynamic(move || results_panel(page_owner, on_open_series)))
                .child(dynamic(move || {
                    let snap = store::search_snapshot();
                    if snap.target.is_none() {
                        return view().into_element();
                    }
                    pagination_footer(snap.loading_more, snap.load_more_error, move || {
                        page_owner.run(query::load_next_search_page);
                    })
                    .into_element()
                })),
        )
}

fn search_bar_row(search_sig: Signal<String>, page_owner: Owner) -> impl IntoElement {
    view()
        .w_full()
        .flex_row()
        .items_center()
        .gap_2()
        .child(view().flex_1().max_w(px(520.0)).child(search_input(
            search_sig,
            "搜索剧名、标签...",
            move |submitted| {
                page_owner.run(|| query::submit_keyword(&submitted));
            },
        )))
        .child(submit_button("搜索", "执行搜索", move || {
            page_owner.run(|| query::submit_keyword(&search_sig.get_untracked()));
        }))
}

fn search_history_section(page_owner: Owner) -> impl IntoElement {
    let history = store::search_snapshot().history;
    if history.is_empty() {
        return view().into_element();
    }
    search_history_strip(history, page_owner).into_element()
}

fn hot_keywords_section(page_owner: Owner) -> impl IntoElement {
    let keywords = store::browse_meta_snapshot().hot_keywords;
    if keywords.is_empty() {
        return view().into_element();
    }
    hot_keywords_strip(keywords, page_owner).into_element()
}

fn tag_filter_section(page_owner: Owner) -> impl IntoElement {
    let groups = store::browse_meta_snapshot().tag_groups;
    if groups.is_empty() {
        return view().into_element();
    }
    tag_filter_strip(&groups, page_owner).into_element()
}

fn search_history_strip(entries: Vec<SearchHistoryEntry>, page_owner: Owner) -> impl IntoElement {
    let mut chips = view().flex_row().flex_wrap().items_center().gap_2();
    for entry in entries {
        let keyword = entry.keyword;
        let keyword_for_search = keyword.clone();
        let keyword_for_remove = keyword.clone();
        let remove_label = format!("删除搜索历史 {keyword}");
        chips = chips.child(
            view()
                .flex_row()
                .items_center()
                .gap_0p5()
                .pr_1()
                .rounded(token::radius_control)
                .bg(token::secondary)
                .child(app_button(
                    keyword,
                    ButtonVariant::Ghost,
                    ControlSize::Sm,
                    move || {
                        page_owner.run(|| query::submit_keyword(&keyword_for_search));
                    },
                ))
                .child(chip_remove_button(remove_label, move || {
                    let kw = keyword_for_remove.clone();
                    page_owner.run(move || query::remove_history(kw));
                })),
        );
    }

    view()
        .w_full()
        .gap_2()
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    text("搜索历史")
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(token::muted_fg),
                )
                .child(text_link_button(
                    "清空",
                    "清空搜索历史",
                    None,
                    move || page_owner.run(query::clear_history),
                )),
        )
        .child(chips)
}

fn hot_keywords_strip(keywords: Vec<HotKeyword>, page_owner: Owner) -> impl IntoElement {
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
            move || page_owner.run(|| query::submit_keyword(&query_word)),
        ));
    }
    row
}

fn tag_filter_strip(groups: &[TagGroup], page_owner: Owner) -> impl IntoElement {
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
            let slug_for_active = slug.clone();
            let name = tag.name.clone();
            row = row.child(tag_chip_button(
                tag.name.clone(),
                move || {
                    let s = slug_for_active.clone();
                    matches!(
                        store::search_snapshot().target,
                        Some(SearchTarget::Tag { slug: current, .. }) if current == s
                    )
                },
                move || {
                    let s = slug.clone();
                    let n = name.clone();
                    page_owner.run(move || query::toggle_tag(s, n));
                },
            ));
        }
        container = container.child(row);
    }
    container
}

fn results_panel(
    page_owner: Owner,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
) -> impl IntoElement {
    let snap = store::search_snapshot();
    let Some(target) = snap.target else {
        return view().into_element();
    };

    if snap.error.is_some() {
        return empty_state(
            IconKind::Search,
            "搜索加载失败",
            "请稍后重试",
            Some((
                "重试",
                Box::new(move || match &target {
                    SearchTarget::Keyword(k) => page_owner.run(|| query::submit_keyword(k)),
                    SearchTarget::Tag { slug, name } => {
                        let s = slug.clone();
                        let n = name.clone();
                        page_owner.run(move || query::toggle_tag(s, n));
                    }
                }),
            )),
        )
        .into_element();
    }

    let is_tag = matches!(target, SearchTarget::Tag { .. });
    let mut container = view().w_full().gap_4();
    if is_tag {
        let sort_items = vec![(SeriesSort::Hot, "最热"), (SeriesSort::New, "最新")];
        container = container.child(segmented_tabs(
            sort_items,
            move || store::search_snapshot().sort,
            move |next| page_owner.run(move || query::switch_sort(next)),
        ));
    }

    if snap.loading && snap.items.is_empty() {
        return container
            .child(
                view()
                    .py_6()
                    .items_center()
                    .child(text("正在搜索...").text_sm().text_color(token::muted_fg)),
            )
            .into_element();
    }

    container
        .child(series_grid(snap.items, on_open_series))
        .into_element()
}
