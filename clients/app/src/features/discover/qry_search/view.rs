use std::collections::BTreeSet;

use hg_contracts::catalog::{HotKeyword, SeriesSort, TagGroup, TagItem};
use hg_contracts::ids::SeriesId;
use hg_contracts::viewing::SearchHistoryEntry;
use nsh_core::prelude::*;

use super::super::parts::{pagination_footer, series_grid};
use super::super::store::{self, SearchTarget};
use super::query;
use crate::ui::button::{ButtonVariant, ControlSize, app_button, text_link_button};
use crate::ui::controls::{
    chip_remove_button, confirm_destructive, submit_button, tag_chip_button, tag_fold_button,
};
use crate::ui::icons::IconKind;
use crate::ui::input::search_input;
use crate::ui::platform::is_desktop;
use crate::ui::surface::empty_state;
use crate::ui::tabs::segmented_tabs;
use crate::ui::token;

const INFINITE_SCROLL_THRESHOLD_PX: f32 = 240.0;
const TAG_GROUP_LABEL_RESERVE_PX: f32 = 76.0;
const TAG_FOLD_BUTTON_RESERVE_PX: f32 = 38.0;

pub fn search_view(on_open_series: impl Fn(SeriesId) + Copy + 'static) -> impl IntoElement {
    let page_owner = Owner::current();
    query::init_search_page();
    let search_sig = store::search_input_signal();
    let container_width = signal(if is_desktop() { 960.0_f32 } else { 360.0_f32 });
    let expanded_groups = signal(BTreeSet::<u32>::new());

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
                .on_layout(move |bounds: &Bounds<Points>| {
                    let w = bounds.size.width.0;
                    if w > 0.0 && (container_width.get_untracked() - w).abs() > 1.0 {
                        container_width.set(w);
                    }
                })
                .child(search_bar_row(search_sig, page_owner))
                .child(
                    view()
                        .w_full()
                        .gap_3()
                        .child(dynamic(move || search_history_section(page_owner)))
                        .child(dynamic(move || hot_keywords_section(page_owner)))
                        .child(dynamic(move || {
                            tag_filter_section(
                                (container_width.get() - 48.0).max(240.0),
                                expanded_groups,
                                page_owner,
                            )
                        })),
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
            move || {
                page_owner.run(query::clear_results);
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

fn tag_filter_section(
    inner_width: f32,
    expanded_groups: Signal<BTreeSet<u32>>,
    page_owner: Owner,
) -> impl IntoElement {
    let groups: Vec<TagGroup> = store::browse_meta_snapshot()
        .tag_groups
        .into_iter()
        .filter(|g| !g.tags.is_empty())
        .collect();
    if groups.is_empty() {
        return view().into_element();
    }
    tag_filter_strip(&groups, inner_width, expanded_groups, page_owner).into_element()
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
                    move || {
                        confirm_destructive(
                            "清空搜索历史",
                            "确定要清空全部搜索历史记录吗？",
                            "清空",
                            move || page_owner.run(query::clear_history),
                        );
                    },
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

fn estimate_tag_chip_width(tag: &TagItem) -> f32 {
    let text_w: f32 = tag
        .name
        .chars()
        .map(|ch| if ch.is_ascii() { 8.5 } else { 14.5 })
        .sum();
    text_w + 22.0
}

fn single_row_visible_count(tags: &[TagItem], inner_width: f32) -> usize {
    let mut total = TAG_GROUP_LABEL_RESERVE_PX;
    for tag in tags {
        total += estimate_tag_chip_width(tag) + 8.0;
    }
    if total <= inner_width {
        return tags.len();
    }
    let budget = (inner_width - TAG_GROUP_LABEL_RESERVE_PX - TAG_FOLD_BUTTON_RESERVE_PX).max(120.0);
    let mut used = 0.0;
    let mut count = 0;
    for tag in tags {
        let chip_w = estimate_tag_chip_width(tag) + 8.0;
        if count > 0 && used + chip_w > budget {
            break;
        }
        used += chip_w;
        count += 1;
    }
    count.max(1)
}

fn tag_filter_strip(
    groups: &[TagGroup],
    inner_width: f32,
    expanded_groups: Signal<BTreeSet<u32>>,
    page_owner: Owner,
) -> impl IntoElement {
    let expanded_set = expanded_groups.get();
    let mut container = view().gap_2();
    for group in groups {
        let group_id = group.id;
        let visible_cap = single_row_visible_count(&group.tags, inner_width);
        let needs_fold = group.tags.len() > visible_cap;
        let is_expanded = expanded_set.contains(&group_id);
        let limit = if !needs_fold || is_expanded {
            group.tags.len()
        } else {
            visible_cap
        };

        let mut row = view().flex_row().flex_wrap().items_center().gap_2().child(
            text(group.name.clone())
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(token::muted_fg),
        );
        for tag in group.tags.iter().take(limit) {
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
        if needs_fold {
            let toggle_a11y = if is_expanded {
                format!("收起 {} 标签", group.name)
            } else {
                format!("展开 {} 标签", group.name)
            };
            row = row.child(tag_fold_button(is_expanded, toggle_a11y, move || {
                expanded_groups.update(|set| {
                    if !set.remove(&group_id) {
                        set.insert(group_id);
                    }
                });
            }));
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
