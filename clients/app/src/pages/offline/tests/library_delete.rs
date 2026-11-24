use std::time::Duration;

use crate::app::flags::FeatureFlags;

use super::start_on_offline;

#[test]
#[ignore = "待 offline/cmd_delete_offline 实现后激活"]
fn deleting_offline_series_removes_assets_from_library() {
    let mut app = start_on_offline(FeatureFlags {
        offline_library: true,
        offline_delete: true,
        ..FeatureFlags::default()
    });

    app.wait_for("云端之上：千金归来").unwrap();
    app.press("删除 云端之上：千金归来 离线剧集").unwrap();
    app.wait_for_gone("云端之上：千金归来", Duration::from_secs(2))
        .unwrap();
}
