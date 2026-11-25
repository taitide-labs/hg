use nsh_base::button;
use nsh_core::prelude::*;

use crate::ui::icons::{IconKind, icon};
use crate::ui::surface::{BadgeTone, badge};
use crate::ui::token;

pub struct SeriesCardProps {
    pub title: String,
    pub cover_url: String,
    pub score: f32,
    pub subtitle: String,
    pub episode_label: String,
    pub badge_text: Option<String>,
    pub watched: bool,
    pub index: usize,
}

pub fn series_card(props: SeriesCardProps, on_press: impl Fn() + 'static) -> impl IntoElement {
    let cover_key = props.cover_url;
    let score_text = format!("{:.1}", props.score);
    let badge_text = props.badge_text;
    let episode_label = props.episode_label;
    let watched = props.watched;
    let title = props.title;
    let subtitle = props.subtitle;

    button()
        .on_press(on_press)
        .accessibility_label(title.clone())
        .w_full()
        .min_w_0()
        .gap_2()
        .p_2p5()
        .rounded(token::radius_card)
        .bg(token::card)
        .border_1()
        .border_color(token::border)
        .transition_colors()
        .duration(token::INSTANT_MS)
        .hover(|s| s.bg(token::surface_subtle))
        .active(|s| {
            if reduce_motion() {
                s.opacity(0.9)
            } else {
                s.scale(token::PRESS_SCALE)
            }
        })
        .focus_visible(|s| s.outline(px(2.0)).outline_color(token::ring))
        .child(poster_cover(
            cover_key,
            badge_text,
            score_text,
            episode_label,
            watched,
        ))
        .child(
            view()
                .w_full()
                .min_w_0()
                .px_0p5()
                .gap_0p5()
                .child(
                    text(title)
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(token::card_fg)
                        .truncate(),
                )
                .child(
                    text(subtitle)
                        .text_xs()
                        .text_color(token::muted_fg)
                        .truncate(),
                ),
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
            .object_fit(ResizeMode::Cover)
            .fade_duration(120.0),
    )
}

pub fn poster_thumb(
    cover_url: String,
    width: f32,
    height: f32,
    icon_kind: IconKind,
    badge_size: f32,
    icon_size: f32,
) -> impl IntoElement {
    let image_layer = cover_image_layer(&cover_url);
    let cover_for_bg = cover_url;
    view()
        .w(px(width))
        .h(px(height))
        .flex_shrink_0()
        .relative()
        .overflow_hidden()
        .rounded(token::radius)
        .bg(move || token::poster_surface(&cover_for_bg))
        .items_center()
        .justify_center()
        .children(image_layer)
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

fn poster_cover(
    cover_key: String,
    badge_text: Option<String>,
    score_text: String,
    episode_label: String,
    watched: bool,
) -> impl IntoElement {
    let image_layer = cover_image_layer(&cover_key);
    let cover_for_bg = cover_key;
    view()
        .w_full()
        .aspect_ratio(160.0 / 224.0)
        .relative()
        .rounded(token::radius)
        .overflow_hidden()
        .bg(move || token::poster_surface(&cover_for_bg))
        .p_2p5()
        .justify_between()
        .children(image_layer)
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .gap_2()
                .child(match badge_text {
                    Some(label) => badge(label, BadgeTone::Brand).into_element(),
                    None => view().into_element(),
                })
                .child(
                    view()
                        .px_1p5()
                        .py_0p5()
                        .rounded(token::radius_badge)
                        .bg(token::cinema_scrim)
                        .child(
                            text(score_text)
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(token::cinema_fg),
                        ),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    view()
                        .px_2()
                        .py_0p5()
                        .rounded(token::radius_badge)
                        .bg(move || {
                            if watched {
                                token::watched()
                            } else {
                                token::cinema_scrim()
                            }
                        })
                        .child(text(episode_label).text_xs().text_color(move || {
                            if watched {
                                token::watched_fg()
                            } else {
                                token::cinema_fg()
                            }
                        })),
                )
                .child(
                    view()
                        .size(px(28.0))
                        .rounded_full()
                        .bg(token::cinema_scrim)
                        .items_center()
                        .justify_center()
                        .child(icon(IconKind::Play, 12.0, token::cinema_fg)),
                ),
        )
}

pub struct RankingCardProps {
    pub rank: u32,
    pub title: String,
    pub cover_url: String,
    pub badge_text: Option<String>,
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
                        .children(props.badge_text.map(|label| badge(label, BadgeTone::Brand))),
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
    props: RankingCardProps,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let cover_key = props.cover_url;
    let rank_label = format!("#{}", props.rank);
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
        .p_3()
        .rounded(token::radius_card)
        .bg(token::card)
        .border_1()
        .border_color(token::border)
        .flex_row()
        .items_center()
        .gap_3p5()
        .transition_colors()
        .duration(token::INSTANT_MS)
        .hover(|s| s.bg(token::surface_subtle))
        .focus_visible(|s| s.outline(px(2.0)).outline_color(token::ring))
        .child(badge(rank_label, tone))
        .child(
            view()
                .w(px(48.0))
                .h(px(64.0))
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
                        .gap_2()
                        .child(
                            text(props.title)
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(token::card_fg),
                        )
                        .children(props.badge_text.map(|label| badge(label, BadgeTone::Brand))),
                )
                .child(text(props.meta).text_xs().text_color(token::muted_fg))
                .children(
                    props
                        .description
                        .map(|desc| text(desc).text_xs().text_color(token::muted_fg).truncate()),
                ),
        )
}
