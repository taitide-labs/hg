use nsh_core::prelude::*;

use crate::ui::icons::IconKind;
use crate::ui::surface::empty_state;

pub fn settings_page(enabled: bool) -> impl IntoElement {
    let _ = enabled;
    empty_state(
        IconKind::Settings,
        "偏好与缓存设置",
        "即将由独立切片补齐外观切换、播放/下载偏好与缓存独立清理",
        None,
    )
}
