use crate::app::flags::FeatureFlags;

use super::start_on_settings;

#[test]
fn settings_updates_offline_download_policy() {
    let mut app = start_on_settings(FeatureFlags {
        settings_download_policy: true,
        ..FeatureFlags::default()
    });

    app.wait_for("下载策略").unwrap();
    app.wait_for("仅 Wi-Fi 下载").unwrap();
    app.press("仅 Wi-Fi 下载").unwrap();
}
