use nsh_engine::ColorScheme;

use crate::app::flags::FeatureFlags;

use super::start_with_flags;

#[test]
fn batch_downloading_episodes_from_player_enqueues_selection() {
    let mut app = start_with_flags(
        ColorScheme::Light,
        FeatureFlags {
            player_batch_download: true,
            ..FeatureFlags::default()
        },
    );

    app.press("离线下载").unwrap();
    app.wait_for("选择离线下载分集").unwrap();
    app.press("全选当前段").unwrap();
    app.press("加入离线队列").unwrap();
    app.wait_for("text:/已加入离线下载/").unwrap();
}
