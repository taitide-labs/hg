use nsh_core::prelude::*;

use crate::features::settings::cmd_clear_cache::view::clear_cache_button;
use crate::features::settings::cmd_download_policy::view::download_policy_section;
use crate::features::settings::cmd_playback_pref::view::playback_pref_section;
use crate::features::settings::qry_storage::view::storage_section;
use crate::ui::icons::IconKind;
use crate::ui::surface::empty_state;

#[derive(Debug, Clone, Copy, Default)]
pub struct SettingsFlags {
    pub playback_pref: bool,
    pub download_policy: bool,
    pub storage: bool,
    pub clear_cache: bool,
}

pub fn settings_page(flags: SettingsFlags) -> impl IntoElement {
    if !flags.playback_pref && !flags.download_policy && !flags.storage {
        return empty_state(
            IconKind::Settings,
            "偏好与缓存设置",
            "即将由独立功能补齐外观切换、播放/下载偏好与缓存独立清理",
            None,
        )
        .into_element();
    }

    scroll()
        .flex_1()
        .min_h_0()
        .child(
            view()
                .p_6()
                .gap_6()
                .pb_safe_area_plus(px(24.0))
                .children(flags.playback_pref.then(playback_pref_section))
                .children(flags.download_policy.then(download_policy_section))
                .children(flags.storage.then(|| {
                    storage_section(move || {
                        if flags.clear_cache {
                            clear_cache_button().into_element()
                        } else {
                            view().into_element()
                        }
                    })
                })),
        )
        .into_element()
}
