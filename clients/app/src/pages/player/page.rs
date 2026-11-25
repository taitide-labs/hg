use hg_contracts::ids::{EpisodeNumber, SeriesId};
use nsh_core::prelude::*;

use crate::features::player::cmd_batch_download::view::batch_download_drawer;
use crate::features::player::cmd_playback::view::player_view;
use crate::features::player::qry_comment_replies::view::comment_replies_drawer;

pub fn player_page(
    series_id: SeriesId,
    initial_episode: Option<EpisodeNumber>,
    on_back: impl Fn() + Copy + 'static,
    on_fullscreen_change: impl Fn(bool) + Copy + 'static,
    on_select_tag: impl Fn(String) + Copy + 'static,
) -> impl IntoElement {
    view()
        .flex_1()
        .min_h_0()
        .relative()
        .child(player_view(
            series_id,
            initial_episode,
            on_back,
            on_fullscreen_change,
            on_select_tag,
        ))
        .child(comment_replies_drawer())
        .child(batch_download_drawer())
}
