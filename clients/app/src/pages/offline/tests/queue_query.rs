use crate::app::flags::FeatureFlags;

use super::start_on_offline;

#[test]
#[ignore = "待 offline/qry_queue 实现后激活"]
fn download_queue_displays_active_tasks_and_progress() {
    let mut app = start_on_offline(FeatureFlags {
        offline_queue: true,
        ..FeatureFlags::default()
    });

    app.wait_for("第 4 集 · 云城风云转折 4").unwrap();
    app.wait_for("长安暗夜行").unwrap();
}
