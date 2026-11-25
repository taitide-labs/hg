use std::rc::Rc;

use nsh_base::button;
use nsh_core::{ContextMenuEvent, Point, prelude::*};

use crate::ui::icons::{IconKind, icon};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeriesRank {
    pub rank: u32,
    pub label: SharedString,
}

impl SeriesRank {
    #[must_use]
    pub fn top(rank: u32) -> Self {
        Self {
            rank,
            label: SharedString::from(format!("TOP {rank}")),
        }
    }

    #[must_use]
    pub fn hash(rank: u32) -> Self {
        Self {
            rank,
            label: SharedString::from(format!("#{rank}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SeriesCardProps {
    pub id: u64,
    pub title: SharedString,
    pub cover_url: SharedString,
    pub score: f32,
    pub tags_text: SharedString,
    pub episode_count: u32,
    pub is_finished: bool,
    pub flags: SeriesFlags,
    pub rank: Option<SeriesRank>,
}

#[derive(Clone)]
pub struct PosterContextActions {
    pub is_bookmarked: Rc<dyn Fn(u64) -> bool>,
    pub is_disliked: Rc<dyn Fn(u64) -> bool>,
    pub on_toggle_bookmark: Rc<dyn Fn(u64)>,
    pub on_open_download: Rc<dyn Fn(u64)>,
    pub on_toggle_dislike: Rc<dyn Fn(u64)>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct HoveredPosterCard(Option<u64>);

fn hovered_card_signal() -> Signal<HoveredPosterCard> {
    global::<HoveredPosterCard>()
}

fn is_card_hovered(item: Memo<Option<SeriesCardProps>>) -> bool {
    let hovered_id = hovered_card_signal().get().0;
    hovered_id.is_some() && item.with(|opt| opt.as_ref().map(|p| p.id) == hovered_id)
}

pub fn series_card(
    item: Memo<Option<SeriesCardProps>>,
    watch_status_fn: impl Fn(u64, u32) -> PosterWatchStatus + Copy + 'static,
    on_open: impl Fn(u64) + 'static,
    context_actions: Option<PosterContextActions>,
) -> impl IntoElement {
    let status = computed(move || {
        item.with(|opt| match opt {
            Some(p) => watch_status_fn(p.id, p.episode_count),
            None => PosterWatchStatus::Unwatched,
        })
    });

    let context_handler = context_actions;
    button()
        .style_when(move || item.with(Option::is_none), |s| s.hidden())
        .on_mouse_enter(move |_: &nsh_core::MouseEvent| {
            let id = item.with_untracked(|opt| opt.as_ref().map(|p| p.id));
            hovered_card_signal().set(HoveredPosterCard(id));
        })
        .on_mouse_leave(move |_: &nsh_core::MouseEvent| {
            let id = item.with_untracked(|opt| opt.as_ref().map(|p| p.id));
            hovered_card_signal().update(|cur| {
                if cur.0 == id {
                    cur.0 = None;
                }
            });
        })
        .on_press(move || {
            if let Some(id) = item.with_untracked(|opt| opt.as_ref().map(|p| p.id)) {
                on_open(id);
            }
        })
        .on_context_menu(move |e: &ContextMenuEvent| {
            let (Some(actions), Some(id)) = (
                context_handler.as_ref(),
                item.with_untracked(|opt| opt.as_ref().map(|p| p.id)),
            ) else {
                return;
            };
            open_poster_menu(id, e.page_position, actions.clone());
        })
        .accessibility_label(move || {
            item.with(|opt| {
                opt.as_ref().map(|p| match &p.rank {
                    Some(rank) => SharedString::from(format!("{} {}", rank.label, p.title)),
                    None => p.title.clone(),
                })
            })
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
        .child(reactive_poster_cover(item, status))
        .child(
            view()
                .w_full()
                .min_w_0()
                .px_0p5()
                .gap_0p5()
                .child(
                    text(move || {
                        item.with(|opt| opt.as_ref().map(|p| p.title.clone()).unwrap_or_default())
                    })
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(token::fg)
                    .truncate(),
                )
                .child(when(
                    move || item.with(|opt| opt.as_ref().is_some_and(|p| !p.tags_text.is_empty())),
                    move || {
                        text(move || {
                            item.with(|opt| {
                                opt.as_ref()
                                    .map(|p| p.tags_text.clone())
                                    .unwrap_or_default()
                            })
                        })
                        .text_xs()
                        .text_color(token::muted_fg)
                        .truncate()
                    },
                )),
        )
}

#[derive(Clone, Default)]
struct PosterMenuState {
    open: bool,
    series_id: Option<u64>,
    at: Point<Points>,
    actions: Option<PosterContextActions>,
}

fn poster_menu_state() -> Signal<PosterMenuState> {
    global::<PosterMenuState>()
}

fn open_poster_menu(series_id: u64, at: Point<Points>, actions: PosterContextActions) {
    poster_menu_state().set(PosterMenuState {
        open: true,
        series_id: Some(series_id),
        at,
        actions: Some(actions),
    });
}

fn close_poster_menu() {
    poster_menu_state().update(|s| {
        s.open = false;
        s.series_id = None;
        s.actions = None;
    });
}

pub fn poster_context_menu_host() -> impl IntoElement {
    let is_open = computed(|| poster_menu_state().with(|s| s.open));

    when(
        move || is_open.get(),
        move || {
            cleanup(close_poster_menu);
            let snapshot = poster_menu_state().with_untracked(Clone::clone);
            let (Some(series_id), Some(actions)) = (snapshot.series_id, snapshot.actions) else {
                return view().into_element();
            };
            let bookmarked = (actions.is_bookmarked)(series_id);
            let disliked = (actions.is_disliked)(series_id);
            let on_toggle_bookmark = Rc::clone(&actions.on_toggle_bookmark);
            let on_open_download = Rc::clone(&actions.on_open_download);
            let on_toggle_dislike = Rc::clone(&actions.on_toggle_dislike);

            overlay()
                .anchor_point(move || snapshot.at)
                .placement(Placement::Bottom)
                .align(Align::Start)
                .offset(px(4.0))
                .margin(px(12.0))
                .on_press_outside(|_: &Point<Points>| close_poster_menu())
                .on_cancel(close_poster_menu)
                .min_w(px(148.0))
                .p_1p5()
                .rounded(token::radius_card)
                .bg(token::popover)
                .border_1()
                .border_color(token::border)
                .child(poster_menu_item(
                    IconKind::Bookmark,
                    if bookmarked { "取消追剧" } else { "追剧" },
                    false,
                    move || {
                        close_poster_menu();
                        on_toggle_bookmark(series_id);
                    },
                ))
                .child(poster_menu_item(
                    IconKind::Download,
                    "离线下载",
                    false,
                    move || {
                        close_poster_menu();
                        on_open_download(series_id);
                    },
                ))
                .child(view().h(px(1.0)).w_full().my_1().bg(token::border))
                .child(poster_menu_item(
                    IconKind::ThumbsDown,
                    if disliked {
                        "取消不喜欢"
                    } else {
                        "不喜欢"
                    },
                    true,
                    move || {
                        close_poster_menu();
                        on_toggle_dislike(series_id);
                    },
                ))
                .into_element()
        },
    )
}

fn poster_menu_item(
    kind: IconKind,
    label: &'static str,
    destructive: bool,
    on_select: impl Fn() + 'static,
) -> impl IntoElement {
    let text_color = if destructive {
        token::destructive
    } else {
        token::popover_fg
    };
    let icon_color = if destructive {
        token::destructive
    } else {
        token::muted_fg
    };
    button()
        .on_press(on_select)
        .h(px(30.0))
        .px_2p5()
        .w_full()
        .rounded(token::radius_control)
        .flex_row()
        .items_center()
        .gap_2()
        .hover(|s| s.bg(token::accent))
        .child(icon(kind, 14.0, icon_color))
        .child(
            text(label)
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(text_color),
        )
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
            .object_fit(ResizeMode::Cover),
    )
}

fn card_cover_image(item: Memo<Option<SeriesCardProps>>) -> impl IntoElement {
    when(
        move || {
            item.with(|opt| {
                opt.as_ref()
                    .is_some_and(|p| is_loadable_image_uri(&p.cover_url))
            })
        },
        move || {
            image(move || {
                let uri = item.with(|opt| {
                    opt.as_ref()
                        .filter(|p| is_loadable_image_uri(&p.cover_url))
                        .map(|p| p.cover_url.clone())
                        .unwrap_or_default()
                });
                nsh_core::element::ImageSrc::from(uri)
            })
            .absolute()
            .inset_0()
            .size_full()
            .object_fit(ResizeMode::Cover)
        },
    )
}

fn reactive_cover_image(cover_url: Memo<String>) -> impl IntoElement {
    let has_image = computed(move || cover_url.with(|u| is_loadable_image_uri(u)));
    when(has_image, move || {
        image(move || {
            let uri = cover_url.with(|u| {
                if is_loadable_image_uri(u) {
                    SharedString::from(u.clone())
                } else {
                    SharedString::default()
                }
            });
            nsh_core::element::ImageSrc::from(uri)
        })
        .absolute()
        .inset_0()
        .size_full()
        .object_fit(ResizeMode::Cover)
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

fn flag_pill(spec: &FlagPillSpec) -> impl IntoElement {
    view()
        .px_1p5()
        .py_0p5()
        .rounded(token::radius_badge)
        .bg(spec.bg)
        .child(
            text(spec.label)
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(spec.fg),
        )
}

fn rank_badge_pill(item: Memo<Option<SeriesCardProps>>) -> impl IntoElement {
    view()
        .self_start()
        .px_2()
        .py_0p5()
        .rounded(token::radius_badge)
        .bg(move || {
            let top3 = item.with(|opt| {
                opt.as_ref()
                    .and_then(|p| p.rank.as_ref())
                    .is_some_and(|r| r.rank <= 3)
            });
            if top3 {
                token::cinema_brand_bg()
            } else {
                token::cinema_surface_elevated()
            }
        })
        .child(
            text(move || {
                item.with(|opt| {
                    opt.as_ref()
                        .and_then(|p| p.rank.as_ref())
                        .map_or_else(SharedString::default, |r| r.label.clone())
                })
            })
            .text_xs()
            .font_weight(FontWeight::MEDIUM)
            .text_color(move || {
                let top3 = item.with(|opt| {
                    opt.as_ref()
                        .and_then(|p| p.rank.as_ref())
                        .is_some_and(|r| r.rank <= 3)
                });
                if top3 {
                    token::cinema_brand_fg()
                } else {
                    token::cinema_fg()
                }
            }),
        )
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
    watch_status: Memo<PosterWatchStatus>,
) -> impl IntoElement {
    view()
        .w_full()
        .aspect_ratio(160.0 / 224.0)
        .relative()
        .rounded(token::radius)
        .overflow_hidden()
        .bg(move || {
            item.with(|opt| token::poster_surface(opt.as_ref().map_or("", |p| &p.cover_url)))
        })
        .p_1p5()
        .justify_between()
        .child(card_cover_image(item))
        .child(when(
            move || is_card_hovered(item),
            move || poster_hover_play_overlay(item),
        ))
        .child(poster_top_bar(item))
        .child(
            view()
                .self_start()
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
        )
}

fn poster_hover_play_overlay(item: Memo<Option<SeriesCardProps>>) -> impl IntoElement {
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
                .child(
                    text("▶")
                        .text_sm()
                        .font_weight(FontWeight::BOLD)
                        .text_color(token::cinema_fg),
                ),
        )
}

fn has_any_flag(flags: SeriesFlags) -> bool {
    flags.is_original || flags.is_hot || flags.is_new
}

fn poster_left_badges(item: Memo<Option<SeriesCardProps>>) -> impl IntoElement {
    when(
        move || {
            item.with(|opt| {
                opt.as_ref()
                    .is_some_and(|p| p.rank.is_some() || has_any_flag(p.flags))
            })
        },
        move || {
            view()
                .flex_row()
                .items_center()
                .gap_1()
                .child(when(
                    move || item.with(|opt| opt.as_ref().is_some_and(|p| p.rank.is_some())),
                    move || rank_badge_pill(item),
                ))
                .child(when(
                    move || item.with(|opt| opt.as_ref().is_some_and(|p| has_any_flag(p.flags))),
                    move || {
                        dynamic(move || {
                            let flags =
                                item.with(|opt| opt.as_ref().map(|p| p.flags).unwrap_or_default());
                            let mut row = view().flex_row().items_center().gap_1();
                            for spec in &FLAG_PILL_SPECS {
                                if (spec.pred)(&flags) {
                                    row = row.child(flag_pill(spec));
                                }
                            }
                            row
                        })
                    },
                ))
        },
    )
}

fn poster_top_bar(item: Memo<Option<SeriesCardProps>>) -> impl IntoElement {
    let has_left = move || {
        item.with(|opt| {
            opt.as_ref()
                .is_some_and(|p| p.rank.is_some() || has_any_flag(p.flags))
        })
    };

    view()
        .w_full()
        .flex_row()
        .items_center()
        .gap_1()
        .style_when(has_left, |s| s.justify_between())
        .style_when(move || !has_left(), |s| s.justify_end())
        .child(poster_left_badges(item))
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
        )
}
