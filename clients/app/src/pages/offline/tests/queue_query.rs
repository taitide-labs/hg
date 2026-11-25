use std::num::{NonZeroU32, NonZeroU64};
use std::sync::Arc;
use std::time::Duration;

use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId};
use hg_contracts::offline::DownloadTaskState;
use hg_gateway::mock::MockGateway;
use nsh_engine::ColorScheme;

use super::{start_on_offline, start_on_offline_manual_clock};
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

#[test]
fn downloading_tasks_auto_refresh_progress_and_completion_while_staying_on_offline_page() {
    let (gw, mut app) =
        start_on_offline_manual_clock(Arc::new(MockGateway::new()), ColorScheme::Light);
    app.wait_for("text:/14\\/24/").unwrap();

    let key = EpisodeKey::new(
        SeriesId::new(NonZeroU64::new(1001).unwrap()),
        EpisodeNumber::new(NonZeroU32::new(4).unwrap()),
    );
    gw.set_download_task_state(
        key,
        DownloadTaskState::Downloading {
            completed_segments: 21,
            total_segments: 24,
            downloaded_bytes: 17_200_000,
        },
    );
    app.fast_forward(Duration::from_secs(2)).unwrap();
    app.wait_for("text:/21\\/24/").unwrap();

    gw.complete_download_task(key);
    app.fast_forward(Duration::from_secs(2)).unwrap();
    app.wait_for("text:/已下载 4 集/").unwrap();
}
