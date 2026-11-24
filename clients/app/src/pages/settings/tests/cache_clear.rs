use super::start_on_settings;

#[test]
fn clearing_cache_zeroes_cache_without_touching_offline_library() {
    let mut app = start_on_settings();

    app.wait_for("清理缓存").unwrap();
    app.press("清理缓存").unwrap();
    app.wait_for("0 MB").unwrap();
    assert!(app.screen().shows("离线片库"));
    assert!(app.screen().shows("55.7 MB") || app.screen().shows("58.4 MB"));
}
