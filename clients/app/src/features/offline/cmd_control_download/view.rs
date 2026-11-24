use hg_contracts::ids::EpisodeKey;
use hg_contracts::offline::DownloadTaskState;
use nsh_core::prelude::*;

pub fn queue_bulk_controls() -> impl IntoElement {
    view()
}

pub fn task_control_buttons(_key: EpisodeKey, _state: DownloadTaskState) -> impl IntoElement {
    view()
}
