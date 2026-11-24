use nsh_engine::ColorScheme;

use crate::app::flags::FeatureFlags;

use super::start_with_flags;

#[test]
fn expanding_comment_replies_loads_nested_reply_drawer() {
    let mut app = start_with_flags(
        ColorScheme::Light,
        FeatureFlags {
            player_comment_replies: true,
            ..FeatureFlags::default()
        },
    );

    app.wait_for("共 2 条回复").unwrap();
    app.press("共 2 条回复").unwrap();
    app.wait_for("全部回复").unwrap();
    app.wait_for("南风知意").unwrap();
    app.wait_for("周游记").unwrap();
}
