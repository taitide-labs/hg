use crate::app::flags::FeatureFlags;

use super::start_on_settings;

#[test]
fn settings_storage_separates_cache_and_offline_library_sizes() {
    let mut app = start_on_settings(FeatureFlags {
        settings_storage: true,
        ..FeatureFlags::default()
    });

    app.wait_for("存储空间").unwrap();
    app.wait_for("在线播放与图片缓存").unwrap();
    app.wait_for("离线片库").unwrap();
}
