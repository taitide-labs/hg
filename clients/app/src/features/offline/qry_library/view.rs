use hg_contracts::ids::{EpisodeNumber, SeriesId};
use nsh_core::prelude::*;

use super::super::store;

pub fn offline_library_section<R>(
    _on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
    _series_action: impl Fn(SeriesId, String) -> R + Copy + 'static,
) -> impl IntoElement
where
    R: IntoElement + 'static,
{
    let empty =
        store::state().with(|s| s.library.is_empty() && !s.loading_library && s.error.is_none());
    view().children(empty.then(view))
}
