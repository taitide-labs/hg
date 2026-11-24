use crate::app::flags::FeatureFlags;

use super::start_on_offline;

#[test]
fn controlling_download_queue_updates_task_states() {
    let mut app = start_on_offline(FeatureFlags {
        offline_queue: true,
        offline_control_download: true,
        ..FeatureFlags::default()
    });

    app.wait_for("全部暂停").unwrap();
    app.press("全部暂停").unwrap();
    app.wait_for("已暂停").unwrap();

    app.press("全部继续").unwrap();
    app.wait_for("排队中").unwrap();
}
