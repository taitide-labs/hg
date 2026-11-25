use super::start_on_offline;
use crate::pages::tests::{DESKTOP_VIEWPORT, assert_above, assert_visible_in_viewport};

#[test]
fn download_queue_sits_above_library_in_single_page_layout() {
    let (_gw, mut app) = start_on_offline();

    app.wait_for("下载中").unwrap();
    app.wait_for("已下载").unwrap();
    app.wait_for("第 4 集 · 云城风云转折 4").unwrap();
    app.wait_for("长安暗夜行").unwrap();

    assert_visible_in_viewport(&app, "下载中 >> nth=0", DESKTOP_VIEWPORT);
    assert_visible_in_viewport(&app, "已下载", DESKTOP_VIEWPORT);
    assert_above(&app, "下载中 >> nth=0", "已下载");
    assert_above(&app, "第 4 集 · 云城风云转折 4", "托管离线片库");
}
