use hg_contracts::catalog::RankingKind;
use nsh_core::prelude::*;

use crate::app::nav::{self, DiscoverSubPage, PrimaryTab, SIDEBAR_MAX_WIDTH, SIDEBAR_MIN_WIDTH};
use crate::features::discover::store::BrowseKind;
use crate::features::settings;
use crate::pages;
use crate::ui::button::{
    ButtonVariant, ControlSize, app_icon_button, app_icon_text_button,
    sidebar_nav_button_collapsible,
};
use crate::ui::controls::{mobile_nav_drawer, sidebar_resize_handle, sidebar_toggle_button};
use crate::ui::icons::IconKind;
use crate::ui::platform::{desktop_title_bar, is_desktop};
use crate::ui::surface::separator;
use crate::ui::token;

const COLLAPSED_SIDEBAR_WIDTH: f32 = 60.0;
const NARROW_SIDEBAR_BREAKPOINT: f32 = 760.0;

const PRIMARY_NAV_ITEMS: &[(PrimaryTab, IconKind, &str)] = &[
    (PrimaryTab::Discover, IconKind::Discover, "发现"),
    (PrimaryTab::Search, IconKind::Search, "搜索"),
    (PrimaryTab::Watchlist, IconKind::Bookmark, "追剧"),
    (PrimaryTab::History, IconKind::Clock, "历史"),
    (PrimaryTab::Offline, IconKind::Download, "离线"),
];

fn tab_title(tab: PrimaryTab) -> &'static str {
    match tab {
        PrimaryTab::Discover => "发现",
        PrimaryTab::Search => "搜索",
        PrimaryTab::Watchlist => "追剧",
        PrimaryTab::History => "历史",
        PrimaryTab::Offline => "离线",
        PrimaryTab::Settings => "设置",
    }
}

fn close_player_and_refresh() {
    nav::close_player();
    spawn(crate::features::discover::parts::sync_watched_series());
}

fn subpage_to_browse(subpage: DiscoverSubPage) -> BrowseKind {
    match subpage {
        DiscoverSubPage::Featured => BrowseKind::Featured,
        DiscoverSubPage::Ranking => BrowseKind::Ranking(RankingKind::Hot),
        DiscoverSubPage::Category { id, name, route } => BrowseKind::Category { id, name, route },
    }
}

fn open_browse_from_discover(kind: BrowseKind) {
    match kind {
        BrowseKind::Featured => nav::open_discover_subpage(DiscoverSubPage::Featured),
        BrowseKind::Ranking(_) => nav::open_discover_subpage(DiscoverSubPage::Ranking),
        BrowseKind::Category { id, name, route } => {
            nav::open_discover_subpage(DiscoverSubPage::Category { id, name, route });
        }
        BrowseKind::Search { .. } | BrowseKind::Tag { .. } => nav::select_tab(PrimaryTab::Search),
    }
}

fn toggle_and_persist_sidebar(owner: Owner) {
    let next_collapsed = nav::toggle_sidebar_collapsed();
    let width = nav::sidebar_width().round() as u16;
    owner.run(|| {
        settings::cmd_playback_pref::handler::persist_sidebar_state(next_collapsed, width);
    });
}

pub fn app_shell() -> impl IntoElement {
    let shell_owner = Owner::current();
    let window_width = signal(1240.0_f32);
    let in_player = || nav::active_player().is_some();
    let show_title_bar = || is_desktop() && !nav::is_chrome_fullscreen();
    let effective_collapsed =
        move || nav::sidebar_collapsed() || window_width.get() < NARROW_SIDEBAR_BREAKPOINT;

    view()
        .flex_1()
        .bg(token::bg)
        .on_layout(move |bounds: &Bounds<Points>| {
            let w = bounds.size.width.0;
            if w > 0.0 && (window_width.get_untracked() - w).abs() > 1.0 {
                window_width.set(w);
            }
        })
        .child(when(show_title_bar, move || {
            desktop_title_bar(
                when(in_player, || {
                    app_icon_text_button(
                        IconKind::ChevronLeft,
                        "返回",
                        ButtonVariant::Ghost,
                        ControlSize::Sm,
                        close_player_and_refresh,
                    )
                })
                .otherwise(move || {
                    view()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(sidebar_toggle_button(effective_collapsed, move || {
                            toggle_and_persist_sidebar(shell_owner);
                        }))
                        .child(
                            text("HG")
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(token::fg),
                        )
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
                        close_player_and_refresh,
                        nav::set_chrome_fullscreen,
                    )
                    .into_element(),
                    None => view().into_element(),
                })
            })
            .otherwise(move || main_chrome(effective_collapsed)),
        )
}

fn main_chrome(effective_collapsed: impl Fn() -> bool + Copy + 'static) -> impl IntoElement {
    if is_desktop() {
        view()
            .flex_1()
            .min_h_0()
            .flex_row()
            .child(desktop_sidebar(effective_collapsed))
            .child(active_tab_content())
            .into_element()
    } else {
        let drawer_open = signal(false);
        on_back_press(|| {
            if nav::active_discover_subpage().is_some() {
                nav::close_discover_subpage();
                true
            } else {
                false
            }
        });
        view()
            .flex_1()
            .min_h_0()
            .child(mobile_header(drawer_open))
            .child(active_tab_content())
            .child(mobile_nav_drawer(
                move || drawer_open.get(),
                move |next| drawer_open.set(next),
                nav_menu_column(
                    || false,
                    move |tab| {
                        nav::select_tab(tab);
                        drawer_open.set(false);
                    },
                ),
            ))
            .into_element()
    }
}

fn nav_menu_column(
    collapsed: impl Fn() -> bool + Copy + 'static,
    on_select: impl Fn(PrimaryTab) + Copy + 'static,
) -> impl IntoElement {
    let mut upper = view().gap_1();
    for &(tab, icon_kind, label) in PRIMARY_NAV_ITEMS {
        upper = upper.child(sidebar_nav_button_collapsible(
            icon_kind,
            label,
            move || nav::current_tab() == tab,
            false,
            collapsed,
            move || on_select(tab),
        ));
    }

    view()
        .flex_1()
        .w_full()
        .justify_between()
        .child(upper)
        .child(
            view()
                .gap_2()
                .child(separator())
                .child(sidebar_nav_button_collapsible(
                    IconKind::Settings,
                    "设置",
                    || nav::current_tab() == PrimaryTab::Settings,
                    true,
                    collapsed,
                    move || on_select(PrimaryTab::Settings),
                )),
        )
}

fn desktop_sidebar(effective_collapsed: impl Fn() -> bool + Copy + 'static) -> impl IntoElement {
    let drag_base = stored(None::<f32>);

    view()
        .w(move || {
            if effective_collapsed() {
                px(COLLAPSED_SIDEBAR_WIDTH)
            } else {
                px(nav::sidebar_width())
            }
        })
        .flex_shrink_0()
        .h_full()
        .bg(token::sidebar)
        .border_r_1()
        .border_color(token::sidebar_border)
        .p_3()
        .style_when(effective_collapsed, |s| s.px_2())
        .child(nav_menu_column(effective_collapsed, nav::select_tab))
        .child(when(
            move || !effective_collapsed(),
            move || {
                sidebar_resize_handle(
                    move |delta| {
                        let base = match drag_base.get() {
                            Some(b) => b,
                            None => {
                                let cur = nav::sidebar_width();
                                drag_base.replace(Some(cur));
                                cur
                            }
                        };
                        nav::set_sidebar_width(base + delta);
                    },
                    move |delta| {
                        let base = drag_base.replace(None).unwrap_or_else(nav::sidebar_width);
                        let final_w = (base + delta).clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH);
                        nav::set_sidebar_width(final_w);
                        settings::cmd_playback_pref::handler::persist_sidebar_state(
                            nav::sidebar_collapsed(),
                            final_w.round() as u16,
                        );
                    },
                )
            },
        ))
}

fn mobile_header(drawer_open: Signal<bool>) -> impl IntoElement {
    view()
        .w_full()
        .flex_shrink_0()
        .pt_safe_area_plus(px(8.0))
        .pb_2()
        .px_3()
        .flex_row()
        .items_center()
        .justify_between()
        .bg(token::bg)
        .border_b_1()
        .border_color(token::border)
        .child(app_icon_button(
            IconKind::Menu,
            "打开导航菜单",
            ButtonVariant::Ghost,
            ControlSize::Md,
            move || drawer_open.set(true),
        ))
        .child(
            view().flex_1().items_center().justify_center().child(
                text(move || tab_title(nav::current_tab()))
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(token::fg),
            ),
        )
        .child(
            when(
                || nav::current_tab() != PrimaryTab::Search,
                || {
                    app_icon_button(
                        IconKind::Search,
                        "前往搜索",
                        ButtonVariant::Ghost,
                        ControlSize::Md,
                        || nav::select_tab(PrimaryTab::Search),
                    )
                },
            )
            .otherwise(|| view().size(px(34.0))),
        )
}

fn active_tab_content() -> impl IntoElement {
    view()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .child(dynamic(|| match nav::current_tab() {
            PrimaryTab::Discover => pages::discover::page::discover_page(
                || nav::active_discover_subpage().map(subpage_to_browse),
                open_browse_from_discover,
                nav::close_discover_subpage,
                |id| nav::open_player(id, None),
            )
            .into_element(),
            PrimaryTab::Search => {
                pages::search::page::search_page(|id| nav::open_player(id, None)).into_element()
            }
            PrimaryTab::Watchlist => {
                pages::watchlist::page::watchlist_page(nav::open_player).into_element()
            }
            PrimaryTab::History => {
                pages::history::page::history_page(nav::open_player).into_element()
            }
            PrimaryTab::Offline => {
                pages::offline::page::offline_page(nav::open_player).into_element()
            }
            PrimaryTab::Settings => pages::settings::page::settings_page().into_element(),
        }))
}
