use hg_contracts::catalog::RankingKind;
use nsh_core::prelude::*;

use crate::app::nav::{self, DiscoverSubPage, PrimaryTab};
use crate::features::discover::store::BrowseKind;
use crate::pages;
use crate::ui::button::{
    ButtonVariant, ControlSize, app_icon_button, app_icon_text_button, mobile_tab_button,
    sidebar_nav_button,
};
use crate::ui::icons::IconKind;
use crate::ui::platform::{desktop_title_bar, is_desktop};
use crate::ui::surface::separator;
use crate::ui::token;

fn subpage_to_browse(subpage: DiscoverSubPage) -> BrowseKind {
    match subpage {
        DiscoverSubPage::Featured => BrowseKind::Featured,
        DiscoverSubPage::Ranking => BrowseKind::Ranking(RankingKind::Hot),
        DiscoverSubPage::Category { id, name, route } => BrowseKind::Category { id, name, route },
        DiscoverSubPage::Search { query } => BrowseKind::Search { query },
    }
}

fn browse_to_subpage(kind: BrowseKind) -> DiscoverSubPage {
    match kind {
        BrowseKind::Featured => DiscoverSubPage::Featured,
        BrowseKind::Ranking(_) => DiscoverSubPage::Ranking,
        BrowseKind::Category { id, name, route } => DiscoverSubPage::Category { id, name, route },
        BrowseKind::Search { query } => DiscoverSubPage::Search { query },
        BrowseKind::Tag { name, .. } => DiscoverSubPage::Search { query: name },
    }
}

pub fn app_shell() -> impl IntoElement {
    let in_player = || nav::active_player().is_some();
    let show_title_bar = || is_desktop() && !nav::is_chrome_fullscreen();

    view()
        .flex_1()
        .bg(token::bg)
        .child(when(show_title_bar, move || {
            desktop_title_bar(
                when(in_player, || {
                    app_icon_text_button(
                        IconKind::ChevronLeft,
                        "返回",
                        ButtonVariant::Ghost,
                        ControlSize::Sm,
                        nav::close_player,
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
                    Some(route) => pages::player::page::player_page(
                        route.series_id,
                        route.initial_episode,
                        nav::close_player,
                        nav::set_chrome_fullscreen,
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
            .min_h_0()
            .flex_row()
            .child(desktop_sidebar())
            .child(active_tab_content())
            .into_element()
    } else {
        view()
            .flex_1()
            .min_h_0()
            .child(mobile_header())
            .child(active_tab_content())
            .child(mobile_bottom_bar())
            .into_element()
    }
}

fn desktop_sidebar() -> impl IntoElement {
    view()
        .w(px(208.0))
        .flex_shrink_0()
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
        .flex_shrink_0()
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
        .flex_shrink_0()
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
    view()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .child(dynamic(|| match nav::current_tab() {
            PrimaryTab::Discover => pages::discover::page::discover_page(
                || nav::active_discover_subpage().map(subpage_to_browse),
                |kind| nav::open_discover_subpage(browse_to_subpage(kind)),
                nav::close_discover_subpage,
                |id| nav::open_player(id, None),
            )
            .into_element(),
            PrimaryTab::Watchlist => pages::placeholder::watchlist_placeholder().into_element(),
            PrimaryTab::Offline => pages::placeholder::offline_placeholder().into_element(),
            PrimaryTab::Settings => pages::placeholder::settings_placeholder().into_element(),
        }))
}
