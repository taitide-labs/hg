use hg_contracts::ids::{EpisodeNumber, SeriesId};
use nsh_core::prelude::*;

use crate::features::offline::cmd_control_download::view::{
    queue_bulk_controls, task_control_buttons,
};
use crate::features::offline::cmd_delete_offline::view::delete_series_button;
use crate::features::offline::qry_library::view::offline_library_section;
use crate::features::offline::qry_queue::view::download_queue_section;
use crate::ui::tabs::segmented_tabs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum OfflineTab {
    #[default]
    Downloaded,
    Queue,
}

pub fn offline_page(
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
) -> impl IntoElement {
    let tab_sig = signal(OfflineTab::Downloaded);

    let tabs = vec![
        (OfflineTab::Downloaded, "已下载"),
        (OfflineTab::Queue, "下载中"),
    ];

    view()
        .flex_1()
        .min_h_0()
        .p_6()
        .gap_5()
        .child(segmented_tabs(
            tabs,
            move || tab_sig.get(),
            move |next| tab_sig.set(next),
        ))
        .child(
            scroll()
                .flex_1()
                .min_h_0()
                .child(dynamic(move || match tab_sig.get() {
                    OfflineTab::Downloaded => {
                        offline_library_section(on_open_series, delete_series_button).into_element()
                    }
                    OfflineTab::Queue => {
                        download_queue_section(queue_bulk_controls, task_control_buttons)
                            .into_element()
                    }
                })),
        )
        .into_element()
}
