use hg_contracts::ids::{EpisodeNumber, SeriesId};
use nsh_core::prelude::*;

use crate::features::watchlist::qry_bookmarks::view::bookmarks_section;
use crate::features::watchlist::qry_history::view::history_section;
use crate::ui::icons::IconKind;
use crate::ui::surface::empty_state;

pub fn watchlist_page(
    history_enabled: bool,
    bookmarks_enabled: bool,
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
) -> impl IntoElement {
    if !history_enabled && !bookmarks_enabled {
        return empty_state(
            IconKind::Bookmark,
            "追剧与观看历史",
            "即将由独立切片补齐观看历史续播与本地追剧收藏列表",
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
                .children(history_enabled.then(|| history_section(on_open_series)))
                .children(bookmarks_enabled.then(|| bookmarks_section(on_open_series))),
        )
        .into_element()
}
