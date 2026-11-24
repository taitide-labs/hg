use hg_contracts::ids::{EpisodeNumber, SeriesId};
use nsh_core::prelude::*;

use crate::ui::icons::IconKind;
use crate::ui::surface::empty_state;

pub fn offline_page(
    enabled: bool,
    _on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
) -> impl IntoElement {
    let _ = enabled;
    empty_state(
        IconKind::Download,
        "离线片库与下载队列",
        "即将由独立切片补齐托管离线片库与分片断点下载队列",
        None,
    )
}
