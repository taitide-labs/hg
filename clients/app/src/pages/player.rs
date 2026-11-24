use hg_contracts::ids::{EpisodeNumber, SeriesId};
use nsh_core::prelude::*;

use crate::features::player::cmd_playback::view::player_view;

pub fn player_page(
    series_id: SeriesId,
    initial_episode: Option<EpisodeNumber>,
    on_back: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    view()
        .flex_1()
        .child(player_view(series_id, initial_episode, on_back))
}
