use nsh_core::prelude::*;

use crate::ui::icons::IconKind;
use crate::ui::surface::empty_state;

pub fn watchlist_placeholder() -> impl IntoElement {
    empty_state(
        IconKind::Bookmark,
        "追剧与观看历史",
        "即将由独立切片补齐观看历史续播与本地追剧收藏列表",
        None,
    )
}

pub fn offline_placeholder() -> impl IntoElement {
    empty_state(
        IconKind::Download,
        "离线片库与下载队列",
        "即将由独立切片补齐托管离线片库与分片断点下载队列",
        None,
    )
}

pub fn settings_placeholder() -> impl IntoElement {
    empty_state(
        IconKind::Settings,
        "偏好与缓存设置",
        "即将由独立切片补齐外观切换、播放/下载偏好与缓存独立清理",
        None,
    )
}
