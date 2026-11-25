use nsh_core::prelude::*;

use crate::features::settings::cmd_clear_cache::view::clear_cache_button;
use crate::features::settings::cmd_disliked::view::{disliked_drawer, disliked_section};
use crate::features::settings::cmd_download_policy::view::download_policy_section;
use crate::features::settings::cmd_playback_pref::view::playback_pref_section;
use crate::features::settings::qry_storage::view::storage_section;

pub fn settings_page() -> impl IntoElement {
    view()
        .flex_1()
        .min_h_0()
        .relative()
        .child(
            scroll().flex_1().min_h_0().child(
                view()
                    .p_6()
                    .gap_6()
                    .pb_safe_area_plus(px(24.0))
                    .child(playback_pref_section())
                    .child(download_policy_section())
                    .child(storage_section(clear_cache_button))
                    .child(disliked_section()),
            ),
        )
        .child(disliked_drawer())
        .into_element()
}
