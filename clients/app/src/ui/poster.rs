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
    pub index: usize,
}

pub fn series_card(props: SeriesCardProps, on_press: impl Fn() + 'static) -> impl IntoElement {
    let enter = token::stagger_enter(props.index);
    let cover_key = props.cover_url;
    let score_text = format!("{:.1}", props.score);
    let badge_text = props.badge_text;
    let episode_label = props.episode_label;
    let title = props.title;
    let subtitle = props.subtitle;

    button()
        .on_press(on_press)
        .accessibility_label(title.clone())
        .bind_motion(&enter)
        .w(px(176.0))
        .gap_2()
        .p_2()
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
        ))
        .child(
            view()
                .px_0p5()
                .gap_0p5()
                .child(
                    text(title)
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(token::card_fg),
                )
                .child(text(subtitle).text_xs().text_color(token::muted_fg)),
        )
}

fn poster_cover(
    cover_key: String,
    badge_text: Option<String>,
    score_text: String,
    episode_label: String,
) -> impl IntoElement {
    let cover_for_bg = cover_key;
    view()
        .w_full()
        .h(px(224.0))
        .rounded(token::radius)
        .overflow_hidden()
        .bg(move || token::poster_surface(&cover_for_bg))
        .p_2p5()
        .justify_between()
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
                        .bg(token::cinema_scrim)
                        .child(text(episode_label).text_xs().text_color(token::cinema_fg)),
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

    button()
        .on_press(on_press)
        .accessibility_label(a11y)
        .w(px(230.0))
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
                .rounded(token::radius)
                .bg(move || token::poster_surface(&cover_key)),
        )
        .child(
            view()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(badge(rank_label, tone))
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
                .rounded(token::radius)
                .bg(move || token::poster_surface(&cover_key)),
        )
        .child(
            view()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(
                    text(props.title)
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(token::card_fg),
                )
                .child(text(props.meta).text_xs().text_color(token::muted_fg))
                .children(
                    props
                        .description
                        .map(|desc| text(desc).text_xs().text_color(token::muted_fg).truncate()),
                ),
        )
}
