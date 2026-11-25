use hg_contracts::catalog::RankingKind;
use hg_contracts::ids::SeriesId;
use nsh_core::prelude::*;

use super::super::parts::{
    poster_context_actions, series_card_props, series_card_props_with_rank,
    series_grid_columns_for_width, series_horizontal_shelf, shelf_card_width,
};
use super::super::store::{self, BrowseKind};
use super::query;
use crate::ui::icons::IconKind;
use crate::ui::platform::is_desktop;
use crate::ui::poster::{PosterContextActions, SeriesCardProps, SeriesRank};
use crate::ui::surface::{empty_state, shelf_header};
use crate::ui::token;

const HOME_SHELF_MAX_ITEMS: usize = 12;

pub fn home_feed_view(
    on_open_browse: impl Fn(BrowseKind) + Copy + 'static,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
    on_open_download: impl Fn(SeriesId) + Copy + 'static,
) -> impl IntoElement {
    let page_owner = Owner::current();
    query::load_home();
    let container_width = signal(if is_desktop() { 960.0_f32 } else { 360.0_f32 });
    let card_width = computed(move || {
        let inner_w = (container_width.get() - 48.0).max(0.0);
        let cols = series_grid_columns_for_width(inner_w);
        shelf_card_width(inner_w, cols)
    });
    let context_actions = poster_context_actions(on_open_download, move || {
        page_owner.run(query::refill_disliked_home_shelves);
    });
    let shell_meta = computed(|| {
        let snap = store::home_snapshot();
        let has_error = snap.error.is_some();
        let categories = snap
            .feed
            .as_ref()
            .map(|feed| {
                feed.category_recommendations
                    .iter()
                    .map(|rec| {
                        let route = snap
                            .categories
                            .iter()
                            .find(|c| c.id == rec.category_id)
                            .map_or_else(|| "duanju".to_owned(), |c| c.route.clone());
                        (rec.category_id, rec.category_name.clone(), route)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        (has_error, snap.feed.is_some(), categories)
    });

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
                let (has_error, has_feed, category_meta) = shell_meta.get();
                untracked(|| {
                    if has_error {
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

                    if !has_feed {
                        return home_skeleton_view(card_width).into_element();
                    }

                    render_home_shelves(
                        category_meta,
                        card_width,
                        on_open_browse,
                        on_open_series,
                        &context_actions,
                    )
                    .into_element()
                })
            })),
    )
}

fn render_home_shelves(
    category_meta: Vec<(u32, String, String)>,
    card_width: Memo<f32>,
    on_open_browse: impl Fn(BrowseKind) + Copy + 'static,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
    context_actions: &PosterContextActions,
) -> impl IntoElement {
    let latest_items = computed(|| {
        store::home_latest_items(HOME_SHELF_MAX_ITEMS)
            .into_iter()
            .map(|s| series_card_props(&s))
            .collect::<Vec<_>>()
    });
    let ranking_items = computed(|| {
        store::home_ranking_items(HOME_SHELF_MAX_ITEMS)
            .into_iter()
            .enumerate()
            .map(|(idx, s)| {
                let rank = u32::try_from(idx + 1).unwrap_or(u32::MAX);
                series_card_props_with_rank(&s, Some(SeriesRank::top(rank)))
            })
            .collect::<Vec<_>>()
    });

    let mut body = view()
        .w_full()
        .gap_6()
        .child(series_shelf_block(
            "最近更新",
            BrowseKind::Latest,
            latest_items,
            card_width,
            on_open_browse,
            on_open_series,
            context_actions,
        ))
        .child(series_shelf_block(
            "排行榜",
            BrowseKind::Ranking(RankingKind::Hot),
            ranking_items,
            card_width,
            on_open_browse,
            on_open_series,
            context_actions,
        ));

    for (cat_id, cat_name, route) in category_meta {
        let cat_items = computed(move || {
            store::home_category_items(cat_id, HOME_SHELF_MAX_ITEMS)
                .into_iter()
                .map(|s| series_card_props(&s))
                .collect::<Vec<_>>()
        });
        body = body.child(series_shelf_block(
            cat_name.clone(),
            BrowseKind::Category {
                id: cat_id,
                name: cat_name,
                route,
            },
            cat_items,
            card_width,
            on_open_browse,
            on_open_series,
            context_actions,
        ));
    }
    body
}

fn series_shelf_block(
    title: impl Into<SharedString>,
    browse_kind: BrowseKind,
    items: Memo<Vec<SeriesCardProps>>,
    card_w: Memo<f32>,
    on_open_browse: impl Fn(BrowseKind) + Copy + 'static,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
    context_actions: &PosterContextActions,
) -> impl IntoElement {
    view()
        .w_full()
        .gap_3()
        .child(shelf_header(title, "全部", move || {
            on_open_browse(browse_kind.clone());
        }))
        .child(series_horizontal_shelf(
            items,
            card_w,
            on_open_series,
            context_actions,
        ))
}

fn home_skeleton_view(card_width: Memo<f32>) -> impl IntoElement {
    let mut body = view().label("发现页加载占位").w_full().gap_6();
    for _ in 0..2 {
        let mut row = view().w_full().flex_row().items_start().gap_3();
        for _ in 0..6 {
            row = row.child(
                view()
                    .w(move || px(card_width.get()))
                    .flex_shrink_0()
                    .gap_1p5()
                    .p_0p5()
                    .child(
                        view()
                            .w_full()
                            .aspect_ratio(160.0 / 224.0)
                            .rounded(token::radius)
                            .bg(token::surface_subtle),
                    )
                    .child(
                        view()
                            .w(px(110.0))
                            .h(px(14.0))
                            .rounded(token::radius_badge)
                            .bg(token::surface_subtle),
                    )
                    .child(
                        view()
                            .w(px(72.0))
                            .h(px(10.0))
                            .rounded(token::radius_badge)
                            .bg(token::muted),
                    ),
            );
        }
        body = body.child(
            view()
                .w_full()
                .gap_3()
                .child(
                    view()
                        .w(px(96.0))
                        .h(px(20.0))
                        .rounded(token::radius_badge)
                        .bg(token::surface_subtle),
                )
                .child(scroll().w_full().horizontal().scrollbar(false).child(row)),
        );
    }
    body
}
