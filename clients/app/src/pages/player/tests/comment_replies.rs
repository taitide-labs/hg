use std::time::Duration;

use nsh_engine::ColorScheme;

use super::start_and_open_first_series;

#[test]
fn expanding_comment_replies_loads_nested_drawer_and_closes_on_escape() {
    let (_gw, mut app) = start_and_open_first_series(ColorScheme::Light);

    app.wait_for("共 2 条回复").unwrap();
    app.press("共 2 条回复").unwrap();
    app.wait_for("全部回复").unwrap();
    app.wait_for("南风知意").unwrap();
    app.wait_for("周游记").unwrap();

    app.press_key("escape").unwrap();
    app.wait_for_gone("全部回复", Duration::from_secs(2))
        .unwrap();
}
