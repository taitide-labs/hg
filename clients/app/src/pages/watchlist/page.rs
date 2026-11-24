use hg_contracts::ids::{EpisodeNumber, SeriesId};
use nsh_core::prelude::*;

use crate::features::watchlist::cmd_remove_history::view::{
    clear_history_button, remove_history_button,
};
use crate::features::watchlist::cmd_unbookmark::view::unbookmark_button;
use crate::features::watchlist::qry_bookmarks::view::bookmarks_section;
use crate::features::watchlist::qry_history::view::history_section;

pub fn watchlist_page(
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
) -> impl IntoElement {
    scroll()
        .flex_1()
        .min_h_0()
        .child(
            view()
                .p_6()
                .gap_6()
                .pb_safe_area_plus(px(24.0))
                .child(history_section(
                    on_open_series,
                    clear_history_button,
                    remove_history_button,
                ))
                .child(bookmarks_section(on_open_series, unbookmark_button)),
        )
        .into_element()
}
