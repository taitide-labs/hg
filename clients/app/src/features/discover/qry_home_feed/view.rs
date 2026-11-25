use hg_contracts::catalog::{CategoryRecommendation, RankingEntry, SeriesSummary};
use hg_contracts::ids::SeriesId;
use nsh_core::prelude::*;

use super::super::parts::{series_flags, series_grid, series_grid_columns_for_width};
use super::super::store::{self, BrowseKind};
use super::query;
use crate::ui::icons::IconKind;
use crate::ui::platform::is_desktop;
use crate::ui::poster::{RankingCardProps, ranking_compact_card};
use crate::ui::surface::{empty_state, shelf_header};

pub fn home_feed_view(
    on_open_browse: impl Fn(BrowseKind) + Copy + 'static,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
) -> impl IntoElement {
    let page_owner = Owner::current();
    query::load_home();
    let container_width = signal(if is_desktop() { 960.0_f32 } else { 360.0_f32 });

    scroll().flex_1().child(
        view()
            .w_full()
            .p_6()
            .gap_6()
            .pb_safe_area_plus(px(24.0))
            .on_layout(move |bounds: &Bounds<Points>| {
                let w = bounds.size.width.0;
                let h = bounds.size.height.0;
                store::set_viewport_size(w, h);
                if w > 0.0 && (container_width.get_untracked() - w).abs() > 1.0 {
                    container_width.set(w);
                }
            })
            .child(dynamic(move || {
                let snap = store::home_snapshot();
                if snap.error.is_some() {
                    return empty_state(
                        IconKind::Discover,
                        "片库加载失败",
                        "请稍后重试刷新发现内容",
                        Some((
                            "重新加载",
                            Box::new(move || page_owner.run(query::load_home)),
                        )),
                    )
                    .into_element();
                }

                let Some(feed) = snap.feed else {
                    return view().into_element();
                };

                let inner_w = (container_width.get() - 48.0).max(0.0);
                let cols = series_grid_columns_for_width(inner_w);
                let rank_cols = ((inner_w + 12.0) / 232.0).floor().max(2.0) as usize;

                let mut body =
                    view()
                        .w_full()
                        .gap_6()
                        .children((!feed.latest.is_empty()).then(|| {
                            latest_shelf(feed.latest, cols, on_open_browse, on_open_series)
                        }))
                        .child(ranking_shelf(
                            snap.rankings,
                            rank_cols,
                            on_open_browse,
                            on_open_series,
                        ));

                for rec in feed.category_recommendations {
                    let route = snap
                        .categories
                        .iter()
                        .find(|c| c.id == rec.category_id)
                        .map_or_else(|| "duanju".to_owned(), |c| c.route.clone());
                    body = body.child(category_shelf(
                        rec,
                        route,
                        cols,
                        on_open_browse,
                        on_open_series,
                    ));
                }
                body.into_element()
            })),
    )
}

fn latest_shelf(
    items: Vec<SeriesSummary>,
    cols: usize,
    on_open_browse: impl Fn(BrowseKind) + Copy + 'static,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
) -> impl IntoElement {
    series_shelf_block(
        "最近更新",
        BrowseKind::Latest,
        items,
        cols,
        on_open_browse,
        on_open_series,
    )
}

fn series_shelf_block(
    title: &'static str,
    browse_kind: BrowseKind,
    items: Vec<SeriesSummary>,
    cols: usize,
    on_open_browse: impl Fn(BrowseKind) + Copy + 'static,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
) -> impl IntoElement {
    let preview: Vec<SeriesSummary> = items.into_iter().take(cols).collect();
    view()
        .w_full()
        .gap_3()
        .child(shelf_header(title, "全部", move || {
            on_open_browse(browse_kind.clone());
        }))
        .child(series_grid(preview, on_open_series))
}

fn ranking_shelf(
    entries: Vec<RankingEntry>,
    cols: usize,
    on_open_browse: impl Fn(BrowseKind) + Copy + 'static,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
) -> impl IntoElement {
    let mut list = view()
        .w_full()
        .grid()
        .grid_template_columns("repeat(auto-fill, minmax(220px, 1fr))")
        .gap_3();
    for entry in entries.into_iter().take(cols) {
        list = list.child(ranking_preview_card(entry, on_open_series));
    }

    view()
        .w_full()
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
    let flags = series_flags(&entry.series);
    let meta = format!(
        "{:.1} 分 · {} 集",
        entry.series.score, entry.series.episode_count
    );
    ranking_compact_card(
        RankingCardProps {
            rank: entry.rank,
            title: entry.series.title,
            cover_url: entry.series.cover_url,
            flags,
            metric_text: None,
            meta,
            description: None,
        },
        move || on_open_series(id),
    )
}

fn category_shelf(
    rec: CategoryRecommendation,
    route: String,
    cols: usize,
    on_open_browse: impl Fn(BrowseKind) + Copy + 'static,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
) -> impl IntoElement {
    let cat_id = rec.category_id;
    let cat_name = rec.category_name.clone();
    let header_title = rec.category_name;
    let preview: Vec<SeriesSummary> = rec.items.into_iter().take(cols).collect();

    view()
        .w_full()
        .gap_3()
        .child(shelf_header(header_title, "全部", move || {
            on_open_browse(BrowseKind::Category {
                id: cat_id,
                name: cat_name.clone(),
                route: route.clone(),
            });
        }))
        .child(series_grid(preview, on_open_series))
}
