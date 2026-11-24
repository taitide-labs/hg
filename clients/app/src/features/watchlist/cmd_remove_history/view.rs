use hg_contracts::ids::SeriesId;
use nsh_core::prelude::*;

pub fn clear_history_button() -> impl IntoElement {
    view()
}

pub fn remove_history_button(_series_id: SeriesId, _title: String) -> impl IntoElement {
    view()
}
