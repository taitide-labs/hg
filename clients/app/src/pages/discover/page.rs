use hg_contracts::ids::SeriesId;
use nsh_core::prelude::*;

use crate::features::discover::qry_browse::view::browse_view;
use crate::features::discover::qry_home_feed::view::home_feed_view;
use crate::features::discover::store::BrowseKind;
use crate::features::player::cmd_batch_download::handler::open_for_series;
use crate::ui::poster::poster_context_menu_host;

pub fn discover_page(
    active_browse: impl Fn() -> Option<BrowseKind> + Copy + 'static,
    on_open_browse: impl Fn(BrowseKind) + Copy + 'static,
    on_close_browse: impl Fn() + Copy + 'static,
    on_open_series: impl Fn(SeriesId) + Copy + 'static,
) -> impl IntoElement {
    let has_browse = computed(move || active_browse().is_some());

    view()
        .flex_1()
        .min_h_0()
        .relative()
        .child(
            view()
                .absolute()
                .inset_0()
                .style_when(move || has_browse.get(), |s| s.hidden())
                .child(home_feed_view(
                    on_open_browse,
                    on_open_series,
                    open_for_series,
                )),
        )
        .child(when(
            move || has_browse.get(),
            move || {
                view()
                    .absolute()
                    .inset_0()
                    .child(dynamic(move || match active_browse() {
                        Some(kind) => {
                            browse_view(kind, on_close_browse, on_open_series, open_for_series)
                                .into_element()
                        }
                        None => view().into_element(),
                    }))
            },
        ))
        .child(poster_context_menu_host())
}
