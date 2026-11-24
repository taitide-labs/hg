use hg_contracts::catalog::{CategoryRecommendation, RankingEntry, SeriesSummary};
use hg_contracts::ids::SeriesId;
use nsh_core::prelude::*;

use super::super::parts::series_grid;
use super::super::store::{self, BrowseKind};
use super::query;
use crate::ui::button::{ButtonVariant, ControlSize, app_button};
use crate::ui::icons::IconKind;
use crate::ui::input::search_input;
use crate::ui::poster::{RankingCardProps, ranking_compact_card};
use crate::ui::surface::{empty_state, shelf_header};

pub fn home_feed_view(
    on_open_browse: impl Fn(BrowseKind) + Copy + 'static,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
) -> impl IntoElement {
    query::load_home();
    let search_sig = store::search_input_signal();

    scroll().flex_1().child(
        view()
            .p_6()
            .gap_6()
            .pb_safe_area_plus(px(24.0))
            .child(search_bar_row(search_sig, on_open_browse))
            .child(dynamic(move || {
                let snap = store::home_snapshot();
                if snap.error.is_some() {
                    return empty_state(
                        IconKind::Discover,
                        "片库加载失败",
                        "请稍后重试刷新发现内容",
                        Some(("重新加载", Box::new(query::load_home))),
                    )
                    .into_element();
                }

                let Some(feed) = snap.feed else {
                    return view().into_element();
                };

                let mut body = view()
                    .gap_6()
                    .child(featured_shelf(
                        feed.featured,
                        on_open_browse,
                        on_open_series,
                    ))
                    .child(ranking_shelf(snap.rankings, on_open_browse, on_open_series));

                for rec in feed.category_recommendations {
                    let route = snap
                        .categories
                        .iter()
                        .find(|c| c.id == rec.category_id)
                        .map_or_else(|| "duanju".to_owned(), |c| c.route.clone());
                    body = body.child(category_shelf(rec, route, on_open_browse, on_open_series));
                }
                body.into_element()
            })),
    )
}

fn search_bar_row(
    search_sig: Signal<String>,
    on_open_browse: impl Fn(BrowseKind) + Copy + 'static,
) -> impl IntoElement {
    view()
        .w_full()
        .flex_row()
        .items_center()
        .gap_2()
        .child(view().flex_1().max_w(px(520.0)).child(search_input(
            search_sig,
            "搜索剧名、标签...",
            move |submitted| {
                on_open_browse(BrowseKind::Search { query: submitted });
            },
        )))
        .child(app_button(
            "搜索",
            ButtonVariant::Secondary,
            ControlSize::Md,
            move || {
                on_open_browse(BrowseKind::Search {
                    query: search_sig.get(),
                });
            },
        ))
}

fn featured_shelf(
    items: Vec<SeriesSummary>,
    on_open_browse: impl Fn(BrowseKind) + Copy + 'static,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
) -> impl IntoElement {
    let preview: Vec<SeriesSummary> = items.into_iter().take(5).collect();
    view()
        .gap_3()
        .child(shelf_header("精选推荐", "全部", move || {
            on_open_browse(BrowseKind::Featured);
        }))
        .child(series_grid(preview, on_open_series))
}

fn ranking_shelf(
    entries: Vec<RankingEntry>,
    on_open_browse: impl Fn(BrowseKind) + Copy + 'static,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
) -> impl IntoElement {
    let mut list = view().flex_row().flex_wrap().gap_3();
    for entry in entries.into_iter().take(4) {
        list = list.child(ranking_preview_card(entry, on_open_series));
    }

    view()
        .gap_3()
        .child(shelf_header("排行榜", "全部", move || {
            on_open_browse(BrowseKind::Ranking(hg_contracts::catalog::RankingKind::Hot));
        }))
        .child(list)
}

fn ranking_preview_card(
    entry: RankingEntry,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
) -> impl IntoElement {
    let id = entry.series.id;
    let meta = format!(
        "{:.1} 分 · {} 集",
        entry.series.score, entry.series.episode_count
    );
    ranking_compact_card(
        RankingCardProps {
            rank: entry.rank,
            title: entry.series.title,
            cover_url: entry.series.cover_url,
            meta,
            description: None,
        },
        move || on_open_series(id),
    )
}

fn category_shelf(
    rec: CategoryRecommendation,
    route: String,
    on_open_browse: impl Fn(BrowseKind) + Copy + 'static,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
) -> impl IntoElement {
    let cat_id = rec.category_id;
    let cat_name = rec.category_name.clone();
    let header_title = rec.category_name;

    view()
        .gap_3()
        .child(shelf_header(header_title, "全部", move || {
            on_open_browse(BrowseKind::Category {
                id: cat_id,
                name: cat_name.clone(),
                route: route.clone(),
            });
        }))
        .child(series_grid(rec.items, on_open_series))
}
