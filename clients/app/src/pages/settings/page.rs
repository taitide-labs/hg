use nsh_core::prelude::*;

use crate::features::settings::cmd_preferences::view::preferences_section;
use crate::features::settings::cmd_storage::view::storage_section;
use crate::ui::icons::IconKind;
use crate::ui::surface::empty_state;

pub fn settings_page(preferences_enabled: bool, storage_enabled: bool) -> impl IntoElement {
    if !preferences_enabled && !storage_enabled {
        return empty_state(
            IconKind::Settings,
            "偏好与缓存设置",
            "即将由独立切片补齐外观切换、播放/下载偏好与缓存独立清理",
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
                .children(preferences_enabled.then(preferences_section))
                .children(storage_enabled.then(storage_section)),
        )
        .into_element()
}
