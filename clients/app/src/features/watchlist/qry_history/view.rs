use hg_contracts::ids::{EpisodeNumber, SeriesId};
use nsh_core::prelude::*;

pub fn history_section(
    _on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
) -> impl IntoElement {
    view()
}
