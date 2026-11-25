use nsh_base::button;
use nsh_core::prelude::*;

use crate::ui::icons::{IconKind, icon};
use crate::ui::surface::{BadgeTone, badge};
use crate::ui::token;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PosterWatchStatus {
    #[default]
    Unwatched,
    AllWatched,
    UpdatedAfterWatching,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SeriesFlags {
    pub is_original: bool,
    pub is_hot: bool,
    pub is_new: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SeriesCardProps {
    pub id: u64,
    pub title: String,
    pub cover_url: String,
    pub score: f32,
    pub tags: Vec<String>,
    pub episode_count: u32,
    pub is_finished: bool,
    pub flags: SeriesFlags,
}

pub fn series_card(
    item: Memo<Option<SeriesCardProps>>,
    watch_status_fn: impl Fn(u64, u32) -> PosterWatchStatus + Copy + 'static,
    on_open: impl Fn(u64) + Copy + 'static,
) -> impl IntoElement {
    let status = computed(move || {
        item.with(|opt| match opt {
            Some(p) => watch_status_fn(p.id, p.episode_count),
            None => PosterWatchStatus::Unwatched,
        })
    });
    let flags = computed(move || item.with(|opt| opt.as_ref().map_or_default(|p| p.flags)));
    let tags =
        computed(move || item.with(|opt| opt.as_ref().map(|p| p.tags.clone()).unwrap_or_default()));
    let cover_url = computed(move || {
        item.with(|opt| {
            opt.as_ref()
                .map(|p| p.cover_url.clone())
                .unwrap_or_default()
        })
    });
    let title = computed(move || {
        item.with(|opt| {
            opt.as_ref()
                .map(|p| SharedString::from(p.title.clone()))
                .unwrap_or_default()
        })
    });

    button()
        .group()
        .style_when(move || item.with(Option::is_none), |s| s.hidden())
        .on_press(move || {
            if let Some(id) = item.with_untracked(|opt| opt.as_ref().map(|p| p.id)) {
                on_open(id);
            }
        })
        .accessibility_label(move || {
            item.with(|opt| opt.as_ref().map(|p| SharedString::from(p.title.clone())))
        })
        .w_full()
        .min_w_0()
        .gap_1p5()
        .p_0p5()
        .rounded(token::radius)
        .active(|s| {
            if reduce_motion() {
                s.opacity(0.86)
            } else {
                s.scale(token::PRESS_SCALE)
            }
        })
        .focus_visible(|s| s.outline(px(2.0)).outline_color(token::ring))
        .child(reactive_poster_cover(item, cover_url, flags, status))
        .child(
            view()
                .w_full()
                .min_w_0()
                .px_0p5()
                .gap_1()
                .child(
                    text(title)
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(token::fg)
                        .truncate(),
                )
                .child(reactive_tags_row(tags)),
        )
}

fn reactive_tags_row(tags: Memo<Vec<String>>) -> impl IntoElement {
    let mut row = view()
        .w_full()
        .min_w_0()
        .flex_row()
        .items_center()
        .gap_1()
        .overflow_hidden();
    for slot in 0..3 {
        let label = computed(move || {
            tags.with(|list| {
                list.get(slot)
                    .cloned()
                    .map(SharedString::from)
                    .unwrap_or_default()
            })
        });
        row = row.child(
            view()
                .px_1p5()
                .py_0p5()
                .rounded(token::radius_badge)
                .bg(token::secondary)
                .style_when(move || label.with(|s| s.is_empty()), |s| s.hidden())
                .child(text(label).text_xs().text_color(token::muted_fg)),
        );
    }
    row
}

fn is_loadable_image_uri(uri: &str) -> bool {
    uri.starts_with("http://") || uri.starts_with("https://") || uri.starts_with("file://")
}

fn cover_image_layer(cover_url: &str) -> Option<impl IntoElement + use<>> {
    if !is_loadable_image_uri(cover_url) {
        return None;
    }
    Some(
        image(cover_url.to_owned())
            .absolute()
            .inset_0()
            .size_full()
            .object_fit(ResizeMode::Cover)
            .fade_duration(120.0),
    )
}

fn reactive_cover_image(cover_url: Memo<String>) -> impl IntoElement {
    let has_image = computed(move || cover_url.with(|u| is_loadable_image_uri(u)));
    let image_src = computed(move || {
        cover_url.with(|u| {
            if is_loadable_image_uri(u) {
                SharedString::from(u.clone())
            } else {
                SharedString::default()
            }
        })
    });
    when(has_image, move || {
        image(move || nsh_core::element::ImageSrc::from(image_src.get()))
            .absolute()
            .inset_0()
            .size_full()
            .object_fit(ResizeMode::Cover)
            .fade_duration(120.0)
    })
}

pub fn poster_thumb(
    cover_url: String,
    width: f32,
    height: f32,
    icon_kind: IconKind,
    badge_size: f32,
    icon_size: f32,
) -> impl IntoElement {
    let image_layer = match cover_image_layer(&cover_url) {
        Some(el) => el.into_element(),
        None => view().into_element(),
    };
    let cover_for_bg = cover_url;
    poster_thumb_shell(
        width,
        height,
        icon_kind,
        badge_size,
        icon_size,
        move || token::poster_surface(&cover_for_bg),
        image_layer,
    )
}

pub fn reactive_poster_thumb(
    cover_url: Memo<String>,
    width: f32,
    height: f32,
    icon_kind: IconKind,
    badge_size: f32,
    icon_size: f32,
) -> impl IntoElement {
    poster_thumb_shell(
        width,
        height,
        icon_kind,
        badge_size,
        icon_size,
        move || cover_url.with(|u| token::poster_surface(u)),
        reactive_cover_image(cover_url),
    )
}

fn poster_thumb_shell(
    width: f32,
    height: f32,
    icon_kind: IconKind,
    badge_size: f32,
    icon_size: f32,
    bg_color: impl Fn() -> AdaptiveColor + 'static,
    image_layer: impl IntoElement + 'static,
) -> impl IntoElement {
    view()
        .w(px(width))
        .h(px(height))
        .flex_shrink_0()
        .relative()
        .overflow_hidden()
        .rounded(token::radius)
        .bg(bg_color)
        .items_center()
        .justify_center()
        .child(image_layer)
        .child(
            view()
                .size(px(badge_size))
                .rounded_full()
                .bg(token::cinema_scrim)
                .items_center()
                .justify_center()
                .child(icon(icon_kind, icon_size, token::cinema_fg)),
        )
}

struct FlagPillSpec {
    pred: fn(&SeriesFlags) -> bool,
    label: &'static str,
    bg: fn() -> AdaptiveColor,
    fg: fn() -> AdaptiveColor,
}

const FLAG_PILL_SPECS: [FlagPillSpec; 3] = [
    FlagPillSpec {
        pred: |f| f.is_original,
        label: "原创",
        bg: token::badge_original,
        fg: token::badge_original_fg,
    },
    FlagPillSpec {
        pred: |f| f.is_hot,
        label: "热播",
        bg: token::badge_hot,
        fg: token::badge_hot_fg,
    },
    FlagPillSpec {
        pred: |f| f.is_new,
        label: "新片",
        bg: token::badge_new,
        fg: token::badge_new_fg,
    },
];

fn series_flag_capsule(flags: SeriesFlags) -> Option<impl IntoElement + use<>> {
    let mut row = view()
        .flex_row()
        .items_center()
        .rounded_full()
        .overflow_hidden();
    let mut has_any = false;
    for spec in FLAG_PILL_SPECS {
        if !(spec.pred)(&flags) {
            continue;
        }
        has_any = true;
        row = row.child(
            view().px_2().py_0p5().bg(spec.bg).child(
                text(spec.label)
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(spec.fg),
            ),
        );
    }
    has_any.then_some(row)
}

fn reactive_flag_capsule(flags: Memo<SeriesFlags>) -> impl IntoElement {
    let mut row = view()
        .flex_row()
        .items_center()
        .rounded_full()
        .overflow_hidden()
        .style_when(
            move || !flags.with(|f| f.is_original || f.is_hot || f.is_new),
            |s| s.hidden(),
        );
    for spec in FLAG_PILL_SPECS {
        row = row.child(
            view()
                .px_2()
                .py_0p5()
                .bg(spec.bg)
                .style_when(move || !flags.with(spec.pred), |s| s.hidden())
                .child(
                    text(spec.label)
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(spec.fg),
                ),
        );
    }
    row
}

fn format_episode_status(count: u32, is_finished: bool, status: PosterWatchStatus) -> String {
    match status {
        PosterWatchStatus::UpdatedAfterWatching => format!("有更新 · {count} 集"),
        PosterWatchStatus::Unwatched | PosterWatchStatus::AllWatched if is_finished => {
            format!("全 {count} 集")
        }
        PosterWatchStatus::Unwatched | PosterWatchStatus::AllWatched => {
            format!("更新至 {count} 集")
        }
    }
}

fn reactive_poster_cover(
    item: Memo<Option<SeriesCardProps>>,
    cover_url: Memo<String>,
    flags: Memo<SeriesFlags>,
    watch_status: Memo<PosterWatchStatus>,
) -> impl IntoElement {
    view()
        .w_full()
        .aspect_ratio(160.0 / 224.0)
        .relative()
        .rounded(token::radius)
        .overflow_hidden()
        .bg(move || cover_url.with(|u| token::poster_surface(u)))
        .p_1p5()
        .justify_between()
        .child(reactive_cover_image(cover_url))
        .child(
            view()
                .absolute()
                .inset_0()
                .items_center()
                .justify_center()
                .child(
                    view()
                        .label(move || {
                            item.with(|opt| {
                                opt.as_ref().map_or_else(SharedString::default, |p| {
                                    SharedString::from(format!("悬浮播放 {}", p.title))
                                })
                            })
                        })
                        .size(px(44.0))
                        .rounded_full()
                        .bg(token::cinema_scrim)
                        .border_1()
                        .border_color(token::cinema_border)
                        .items_center()
                        .justify_center()
                        .opacity(0.0)
                        .transition_opacity()
                        .duration(token::FAST_MS)
                        .group_hover(|s| s.opacity(1.0))
                        .child(icon(IconKind::Play, 18.0, token::cinema_fg)),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .gap_2()
                .child(reactive_flag_capsule(flags))
                .child(
                    view()
                        .px_1p5()
                        .py_0p5()
                        .rounded(token::radius_badge)
                        .bg(token::cinema_scrim)
                        .child(
                            text(move || {
                                item.with(|opt| {
                                    opt.as_ref().map_or_else(SharedString::default, |p| {
                                        SharedString::from(format!("{:.1}分", p.score))
                                    })
                                })
                            })
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(token::badge_original),
                        ),
                ),
        )
        .child(
            view().flex_row().items_center().justify_start().child(
                view()
                    .px_2()
                    .py_0p5()
                    .rounded(token::radius_badge)
                    .bg(move || match watch_status.get() {
                        PosterWatchStatus::AllWatched => token::watched(),
                        PosterWatchStatus::UpdatedAfterWatching => token::warning_soft(),
                        PosterWatchStatus::Unwatched => token::cinema_scrim(),
                    })
                    .child(
                        text(move || {
                            let ws = watch_status.get();
                            item.with(|opt| {
                                opt.as_ref().map_or_else(SharedString::default, |p| {
                                    SharedString::from(format_episode_status(
                                        p.episode_count,
                                        p.is_finished,
                                        ws,
                                    ))
                                })
                            })
                        })
                        .text_xs()
                        .text_color(move || match watch_status.get() {
                            PosterWatchStatus::AllWatched => token::watched_fg(),
                            PosterWatchStatus::UpdatedAfterWatching => token::warning(),
                            PosterWatchStatus::Unwatched => token::cinema_fg(),
                        }),
                    ),
            ),
        )
}

#[derive(Debug, Clone, PartialEq)]
pub struct RankingCardProps {
    pub id: u64,
    pub rank: u32,
    pub title: String,
    pub cover_url: String,
    pub flags: SeriesFlags,
    pub meta: String,
    pub description: Option<String>,
}

pub fn ranking_compact_card(
    props: RankingCardProps,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let cover_key = props.cover_url;
    let rank_label = format!("TOP {}", props.rank);
    let tone = if props.rank <= 3 {
        BadgeTone::Brand
    } else {
        BadgeTone::Neutral
    };
    let a11y = format!("{} {}", rank_label, props.title);

    let image_layer = cover_image_layer(&cover_key);
    button()
        .on_press(on_press)
        .accessibility_label(a11y)
        .w_full()
        .min_w_0()
        .p_3()
        .rounded(token::radius_card)
        .bg(token::card)
        .border_1()
        .border_color(token::border)
        .flex_row()
        .items_center()
        .gap_3()
        .transition_colors()
        .duration(token::INSTANT_MS)
        .hover(|s| s.bg(token::surface_subtle))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(token::ring))
        .child(
            view()
                .w(px(46.0))
                .h(px(62.0))
                .flex_shrink_0()
                .relative()
                .overflow_hidden()
                .rounded(token::radius)
                .bg(move || token::poster_surface(&cover_key))
                .children(image_layer),
        )
        .child(
            view()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_1p5()
                        .child(badge(rank_label, tone))
                        .children(series_flag_capsule(props.flags)),
                )
                .child(
                    text(props.title)
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(token::card_fg)
                        .truncate(),
                )
                .child(text(props.meta).text_xs().text_color(token::muted_fg)),
        )
}

pub fn ranking_list_row(
    item: Memo<Option<RankingCardProps>>,
    on_open: impl Fn(u64) + Copy + 'static,
) -> impl IntoElement {
    button()
        .style_when(move || item.with(Option::is_none), |s| s.hidden())
        .on_press(move || {
            if let Some(id) = item.with_untracked(|opt| opt.as_ref().map(|p| p.id)) {
                on_open(id);
            }
        })
        .accessibility_label(move || {
            item.with(|opt| {
                opt.as_ref()
                    .map(|p| SharedString::from(format!("#{} {}", p.rank, p.title)))
            })
        })
        .w_full()
        .min_w_0()
        .p_3()
        .rounded(token::radius_card)
        .bg(token::card)
        .border_1()
        .border_color(token::border)
        .flex_row()
        .items_start()
        .gap_3()
        .transition_colors()
        .duration(token::INSTANT_MS)
        .hover(|s| s.bg(token::surface_subtle))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(token::ring))
        .child(ranking_list_thumb(item))
        .child(ranking_list_meta(item))
}

fn ranking_list_thumb(item: Memo<Option<RankingCardProps>>) -> impl IntoElement {
    let cover_url = computed(move || {
        item.with(|opt| {
            opt.as_ref()
                .map(|p| p.cover_url.clone())
                .unwrap_or_default()
        })
    });
    let is_top3 = computed(move || item.with(|opt| opt.as_ref().is_some_and(|p| p.rank <= 3)));
    let rank_label = computed(move || {
        item.with(|opt| {
            opt.as_ref()
                .map_or_else(SharedString::default, |p| format!("#{}", p.rank).into())
        })
    });

    view()
        .w(px(72.0))
        .h(px(96.0))
        .flex_shrink_0()
        .relative()
        .overflow_hidden()
        .rounded(token::radius)
        .bg(move || cover_url.with(|u| token::poster_surface(u)))
        .child(reactive_cover_image(cover_url))
        .child(
            view()
                .absolute()
                .top_1()
                .left_1()
                .child(reactive_rank_badge(rank_label, is_top3)),
        )
}

fn ranking_list_meta(item: Memo<Option<RankingCardProps>>) -> impl IntoElement {
    let flags = computed(move || item.with(|opt| opt.as_ref().map_or_default(|p| p.flags)));
    let description = computed(move || {
        item.with(|opt| {
            opt.as_ref()
                .and_then(|p| p.description.clone())
                .map(SharedString::from)
                .unwrap_or_default()
        })
    });

    view()
        .flex_1()
        .min_w_0()
        .gap_1p5()
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_1p5()
                .child(
                    text(move || {
                        item.with(|opt| {
                            opt.as_ref()
                                .map(|p| SharedString::from(p.title.clone()))
                                .unwrap_or_default()
                        })
                    })
                    .flex_1()
                    .min_w_0()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(token::card_fg)
                    .truncate(),
                )
                .child(reactive_flag_capsule(flags)),
        )
        .child(
            text(move || {
                item.with(|opt| {
                    opt.as_ref()
                        .map(|p| SharedString::from(p.meta.clone()))
                        .unwrap_or_default()
                })
            })
            .text_xs()
            .text_color(token::muted_fg)
            .truncate(),
        )
        .child(
            text(description)
                .style_when(move || description.with(|d| d.is_empty()), |s| s.hidden())
                .text_xs()
                .text_color(token::muted_fg)
                .number_of_lines(2u32)
                .text_overflow(nsh_core::EllipsizeMode::Tail),
        )
}

fn reactive_rank_badge(label: Memo<SharedString>, is_top3: Memo<bool>) -> impl IntoElement {
    view()
        .self_start()
        .px_2()
        .py_0p5()
        .rounded(token::radius_badge)
        .bg(move || {
            if is_top3.get() {
                token::cinema_brand_bg()
            } else {
                token::cinema_surface_elevated()
            }
        })
        .child(
            text(label)
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || {
                    if is_top3.get() {
                        token::cinema_brand_fg()
                    } else {
                        token::cinema_fg()
                    }
                }),
        )
}
