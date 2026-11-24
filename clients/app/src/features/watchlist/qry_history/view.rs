use hg_contracts::ids::{EpisodeNumber, SeriesId};
use nsh_core::prelude::*;

use super::super::store;

pub fn history_section<H, R>(
    _on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
    _header_action: impl Fn() -> H + Copy + 'static,
    _row_action: impl Fn(SeriesId, String) -> R + Copy + 'static,
) -> impl IntoElement
where
    H: IntoElement + 'static,
    R: IntoElement + 'static,
{
    let empty =
        store::state().with(|s| s.history.is_empty() && !s.loading_history && s.error.is_none());
    view().children(empty.then(view))
}
