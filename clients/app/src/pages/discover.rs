use hg_contracts::ids::SeriesId;
use nsh_core::prelude::*;

use crate::features::discover::qry_home_feed::view::home_feed_view;

pub fn discover_page(on_open_series: impl Fn(SeriesId) + Copy + 'static) -> impl IntoElement {
    view().flex_1().child(home_feed_view(on_open_series))
}
