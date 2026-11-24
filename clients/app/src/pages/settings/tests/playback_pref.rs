use super::start_on_settings;

#[test]
fn settings_controls_appearance_and_playback_preference() {
    let mut app = start_on_settings();

    app.wait_for("外观").unwrap();
    app.wait_for("跟随系统").unwrap();
    app.wait_for("播放偏好").unwrap();
    app.press("浅色").unwrap();
}
