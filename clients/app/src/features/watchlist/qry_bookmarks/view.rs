use hg_contracts::ids::{EpisodeNumber, SeriesId};
use nsh_core::prelude::*;

use super::super::store;

pub fn bookmarks_section<R>(
    _on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
    _row_action: impl Fn(SeriesId, String) -> R + Copy + 'static,
) -> impl IntoElement
where
    R: IntoElement + 'static,
{
    let empty = store::state().with(|s| s.bookmarks.is_empty() && !s.loading_bookmarks);
    view().children(empty.then(view))
}
