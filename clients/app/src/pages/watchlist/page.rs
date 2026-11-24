use hg_contracts::ids::{EpisodeNumber, SeriesId};
use nsh_core::prelude::*;

use crate::ui::icons::IconKind;
use crate::ui::surface::empty_state;

pub fn watchlist_page(
    enabled: bool,
    _on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
) -> impl IntoElement {
    let _ = enabled;
    empty_state(
        IconKind::Bookmark,
        "追剧与观看历史",
        "即将由独立切片补齐观看历史续播与本地追剧收藏列表",
        None,
    )
}
