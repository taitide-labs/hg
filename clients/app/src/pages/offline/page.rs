use hg_contracts::ids::{EpisodeNumber, SeriesId};
use nsh_core::prelude::*;

use crate::features::offline::qry_library::view::offline_library_section;
use crate::features::offline::qry_queue::view::download_queue_section;
use crate::ui::icons::IconKind;
use crate::ui::surface::empty_state;
use crate::ui::tabs::segmented_tabs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum OfflineTab {
    #[default]
    Downloaded,
    Queue,
}

pub fn offline_page(
    library_enabled: bool,
    queue_enabled: bool,
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
) -> impl IntoElement {
    if !library_enabled && !queue_enabled {
        return empty_state(
            IconKind::Download,
            "离线片库与下载队列",
            "即将由独立切片补齐托管离线片库与分片断点下载队列",
            None,
        )
        .into_element();
    }

    let tab_sig = global::<OfflineTab>();
    let default_tab = if library_enabled {
        OfflineTab::Downloaded
    } else {
        OfflineTab::Queue
    };
    tab_sig.set(default_tab);

    let tabs = vec![
        (OfflineTab::Downloaded, "已下载"),
        (OfflineTab::Queue, "下载中"),
    ];

    view()
        .flex_1()
        .min_h_0()
        .p_6()
        .gap_5()
        .child(segmented_tabs(tabs, tab_sig, move |next| tab_sig.set(next)))
        .child(
            scroll()
                .flex_1()
                .min_h_0()
                .child(dynamic(move || match tab_sig.get() {
                    OfflineTab::Downloaded if library_enabled => {
                        offline_library_section(on_open_series).into_element()
                    }
                    OfflineTab::Queue if queue_enabled => download_queue_section().into_element(),
                    OfflineTab::Downloaded | OfflineTab::Queue => view().into_element(),
                })),
        )
        .into_element()
}
