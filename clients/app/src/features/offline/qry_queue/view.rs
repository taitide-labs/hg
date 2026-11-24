use hg_contracts::ids::EpisodeKey;
use hg_contracts::offline::DownloadTaskState;
use nsh_core::prelude::*;

use super::super::store;

pub fn download_queue_section<H, R>(
    _header_action: impl Fn() -> H + Copy + 'static,
    _task_action: impl Fn(EpisodeKey, DownloadTaskState) -> R + Copy + 'static,
) -> impl IntoElement
where
    H: IntoElement + 'static,
    R: IntoElement + 'static,
{
    let empty = store::state().with(|s| s.tasks.is_empty() && !s.loading_queue);
    view().children(empty.then(view))
}
