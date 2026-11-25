use hg_contracts::ids::SeriesId;
use nsh_core::prelude::*;

use crate::features::discover::qry_search::view::search_view;

pub fn search_page(on_open_series: impl Fn(SeriesId) + Copy + 'static) -> impl IntoElement {
    view().flex_1().min_h_0().child(search_view(on_open_series))
}
