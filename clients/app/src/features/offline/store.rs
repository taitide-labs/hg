use hg_contracts::offline::{DownloadTaskView, OfflineSeriesView};
use nsh_core::prelude::*;

#[derive(Debug, Clone, Default)]
pub struct OfflineState {
    pub library: Vec<OfflineSeriesView>,
    pub tasks: Vec<DownloadTaskView>,
    pub loading_library: bool,
    pub loading_queue: bool,
    pub error: Option<String>,
}

#[must_use]
pub fn state() -> Signal<OfflineState> {
    global::<OfflineState>()
}
