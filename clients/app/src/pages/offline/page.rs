use hg_contracts::ids::{EpisodeNumber, SeriesId};
use nsh_core::prelude::*;

use crate::features::offline::cmd_control_download::view::{
    queue_bulk_controls, task_control_buttons,
};
use crate::features::offline::cmd_delete_offline::view::delete_series_button;
use crate::features::offline::qry_library::view::offline_library_section;
use crate::features::offline::qry_queue::view::download_queue_section;
use crate::ui::icons::IconKind;
use crate::ui::surface::empty_state;
use crate::ui::tabs::segmented_tabs;

#[derive(Debug, Clone, Copy, Default)]
pub struct OfflineFlags {
    pub library: bool,
    pub delete: bool,
    pub queue: bool,
    pub control_download: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum OfflineTab {
    #[default]
    Downloaded,
    Queue,
}

pub fn offline_page(
    flags: OfflineFlags,
    on_open_series: impl Fn(SeriesId, Option<EpisodeNumber>) + Copy + 'static,
) -> impl IntoElement {
    if !flags.library && !flags.queue {
        return empty_state(
            IconKind::Download,
            "离线片库与下载队列",
            "即将由独立功能补齐托管离线片库与分片断点下载队列",
            None,
        )
        .into_element();
    }

    let default_tab = if flags.library {
        OfflineTab::Downloaded
    } else {
        OfflineTab::Queue
    };
    let tab_sig = signal(default_tab);

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
        .child(scroll().flex_1().min_h_0().child(dynamic(move || {
            match tab_sig.get() {
                OfflineTab::Downloaded if flags.library => {
                    offline_library_section(on_open_series, move |id, title| {
                        if flags.delete {
                            delete_series_button(id, title).into_element()
                        } else {
                            view().into_element()
                        }
                    })
                    .into_element()
                }
                OfflineTab::Queue if flags.queue => download_queue_section(
                    move || {
                        if flags.control_download {
                            queue_bulk_controls().into_element()
                        } else {
                            view().into_element()
                        }
                    },
                    move |key, state| {
                        if flags.control_download {
                            task_control_buttons(key, state).into_element()
                        } else {
                            view().into_element()
                        }
                    },
                )
                .into_element(),
                OfflineTab::Downloaded | OfflineTab::Queue => view().into_element(),
            }
        })))
        .into_element()
}
