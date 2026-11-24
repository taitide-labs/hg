use hg_contracts::ids::SeriesId;
use nsh_core::prelude::*;

use crate::features::discover::qry_browse::view::browse_view;
use crate::features::discover::qry_home_feed::view::home_feed_view;
use crate::features::discover::store::BrowseKind;

pub fn discover_page(
    active_browse: impl Fn() -> Option<BrowseKind> + Copy + 'static,
    on_open_browse: impl Fn(BrowseKind) + Copy + 'static,
    on_close_browse: impl Fn() + Copy + 'static,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
) -> impl IntoElement {
    view()
        .flex_1()
        .min_h_0()
        .child(dynamic(move || match active_browse() {
            Some(kind) => browse_view(kind, on_close_browse, on_open_series).into_element(),
            None => home_feed_view(on_open_browse, on_open_series).into_element(),
        }))
}
