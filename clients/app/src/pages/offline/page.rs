use hg_contracts::ids::{EpisodeNumber, SeriesId};
use nsh_core::prelude::*;

use crate::features::offline::cmd_control_download::view::{
    queue_bulk_controls, task_control_buttons,
};
use crate::features::offline::cmd_delete_offline::view::delete_series_button;
use crate::features::offline::qry_library::view::offline_library_section;
use crate::features::offline::qry_queue::view::download_queue_section;

pub fn offline_page(
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
) -> impl IntoElement {
    scroll()
        .flex_1()
        .min_h_0()
        .child(
            view()
                .w_full()
                .p_6()
                .pb_safe_area_plus(px(24.0))
                .child(download_queue_section(
                    queue_bulk_controls,
                    task_control_buttons,
                ))
                .child(offline_library_section(
                    on_open_series,
                    delete_series_button,
                )),
        )
        .into_element()
}
