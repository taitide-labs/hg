use hg_contracts::ids::{EpisodeNumber, SeriesId};
use nsh_core::prelude::*;

use crate::features::watchlist::cmd_remove_history::view::{
    clear_history_button, remove_history_button,
};
use crate::features::watchlist::cmd_unbookmark::view::unbookmark_button;
use crate::features::watchlist::qry_bookmarks::view::bookmarks_section;
use crate::features::watchlist::qry_history::view::history_section;
use crate::ui::icons::IconKind;
use crate::ui::surface::empty_state;

#[derive(Debug, Clone, Copy, Default)]
pub struct WatchlistFlags {
    pub history: bool,
    pub remove_history: bool,
    pub bookmarks: bool,
    pub unbookmark: bool,
}

pub fn watchlist_page(
    flags: WatchlistFlags,
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
) -> impl IntoElement {
    if !flags.history && !flags.bookmarks {
        return empty_state(
            IconKind::Bookmark,
            "追剧与观看历史",
            "即将由独立功能补齐观看历史续播与本地追剧收藏列表",
            None,
        )
        .into_element();
    }

    scroll()
        .flex_1()
        .min_h_0()
        .child(
            view()
                .p_6()
                .gap_6()
                .pb_safe_area_plus(px(24.0))
                .children(flags.history.then(|| {
                    history_section(
                        on_open_series,
                        move || {
                            if flags.remove_history {
                                clear_history_button().into_element()
                            } else {
                                view().into_element()
                            }
                        },
                        move |id, title| {
                            if flags.remove_history {
                                remove_history_button(id, title).into_element()
                            } else {
                                view().into_element()
                            }
                        },
                    )
                }))
                .children(flags.bookmarks.then(|| {
                    bookmarks_section(on_open_series, move |id, title| {
                        if flags.unbookmark {
                            unbookmark_button(id, title).into_element()
                        } else {
                            view().into_element()
                        }
                    })
                })),
        )
        .into_element()
}
