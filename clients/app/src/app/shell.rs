use nsh_core::prelude::*;

use crate::app::nav::{self, PrimaryTab};
use crate::pages;
use crate::ui::button::{
    ButtonVariant, ControlSize, app_icon_button, mobile_tab_button, sidebar_nav_button,
};
use crate::ui::icons::IconKind;
use crate::ui::platform::{desktop_title_bar, is_desktop};
use crate::ui::surface::separator;
use crate::ui::token;

pub fn app_shell() -> impl IntoElement {
    let in_player = || nav::active_player().is_some();

    view()
        .flex_1()
        .bg(move || {
            if in_player() {
                token::cinema_bg()
            } else {
                token::bg()
            }
        })
        .children(is_desktop().then(|| {
            desktop_title_bar(
                in_player,
                when(in_player, || {
                    view()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(app_icon_button(
                            IconKind::ChevronLeft,
                            "返回",
                            ButtonVariant::CinemaGlass,
                            ControlSize::Sm,
                            nav::close_player,
                        ))
                        .child(
                            text("正在观影")
                                .text_sm()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(token::cinema_fg),
                        )
                })
                .otherwise(|| {
                    text("HG")
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(token::fg)
                }),
                view(),
            )
        }))
        .child(
            when(in_player, || {
                dynamic(|| match nav::active_player() {
                    Some(route) => pages::player::player_page(
                        route.series_id,
                        route.initial_episode,
                        nav::close_player,
                    )
                    .into_element(),
                    None => view().into_element(),
                })
            })
            .otherwise(main_chrome),
        )
}

fn main_chrome() -> impl IntoElement {
    if is_desktop() {
        view()
            .flex_1()
            .flex_row()
            .child(desktop_sidebar())
            .child(active_tab_content())
            .into_element()
    } else {
        view()
            .flex_1()
            .child(mobile_header())
            .child(active_tab_content())
            .child(mobile_bottom_bar())
            .into_element()
    }
}

fn desktop_sidebar() -> impl IntoElement {
    view()
        .w(px(208.0))
        .h_full()
        .bg(token::sidebar)
        .border_r_1()
        .border_color(token::sidebar_border)
        .p_3()
        .justify_between()
        .child(
            view()
                .gap_1()
                .child(sidebar_nav_button(
                    IconKind::Discover,
                    "发现",
                    || nav::current_tab() == PrimaryTab::Discover,
                    false,
                    || nav::select_tab(PrimaryTab::Discover),
                ))
                .child(sidebar_nav_button(
                    IconKind::Bookmark,
                    "追剧",
                    || nav::current_tab() == PrimaryTab::Watchlist,
                    false,
                    || nav::select_tab(PrimaryTab::Watchlist),
                ))
                .child(sidebar_nav_button(
                    IconKind::Download,
                    "离线",
                    || nav::current_tab() == PrimaryTab::Offline,
                    false,
                    || nav::select_tab(PrimaryTab::Offline),
                )),
        )
        .child(view().gap_2().child(separator()).child(sidebar_nav_button(
            IconKind::Settings,
            "设置",
            || nav::current_tab() == PrimaryTab::Settings,
            true,
            || nav::select_tab(PrimaryTab::Settings),
        )))
}

fn mobile_header() -> impl IntoElement {
    view()
        .w_full()
        .pt_safe_area_plus(px(8.0))
        .pb_2()
        .px_4()
        .flex_row()
        .items_center()
        .justify_between()
        .bg(token::bg)
        .border_b_1()
        .border_color(token::border)
        .child(
            text(move || match nav::current_tab() {
                PrimaryTab::Discover => "发现",
                PrimaryTab::Watchlist => "追剧",
                PrimaryTab::Offline => "离线",
                PrimaryTab::Settings => "设置",
            })
            .text_lg()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(token::fg),
        )
        .child(app_icon_button(
            IconKind::Settings,
            "设置",
            ButtonVariant::Ghost,
            ControlSize::Md,
            || nav::select_tab(PrimaryTab::Settings),
        ))
}

fn mobile_bottom_bar() -> impl IntoElement {
    view()
        .w_full()
        .px_3()
        .pt_1p5()
        .pb_safe_area_plus(px(6.0))
        .flex_row()
        .items_center()
        .gap_2()
        .bg(token::card)
        .border_t_1()
        .border_color(token::border)
        .child(mobile_tab_button(
            IconKind::Discover,
            "发现",
            || nav::current_tab() == PrimaryTab::Discover,
            || nav::select_tab(PrimaryTab::Discover),
        ))
        .child(mobile_tab_button(
            IconKind::Bookmark,
            "追剧",
            || nav::current_tab() == PrimaryTab::Watchlist,
            || nav::select_tab(PrimaryTab::Watchlist),
        ))
        .child(mobile_tab_button(
            IconKind::Download,
            "离线",
            || nav::current_tab() == PrimaryTab::Offline,
            || nav::select_tab(PrimaryTab::Offline),
        ))
}

fn active_tab_content() -> impl IntoElement {
    view().flex_1().child(dynamic(|| match nav::current_tab() {
        PrimaryTab::Discover => {
            pages::discover::discover_page(|id| nav::open_player(id, None)).into_element()
        }
        PrimaryTab::Watchlist => pages::placeholder::watchlist_placeholder().into_element(),
        PrimaryTab::Offline => pages::placeholder::offline_placeholder().into_element(),
        PrimaryTab::Settings => pages::placeholder::settings_placeholder().into_element(),
    }))
}
