use std::sync::Arc;

use hg_gateway::mock::MockGateway;
use nsh_engine::ColorScheme;

use super::start_on_offline_with_spy;

#[test]
fn offline_library_displays_downloaded_series_and_emits_open_intent() {
    let (_gw, mut app, opened) =
        start_on_offline_with_spy(Arc::new(MockGateway::new()), ColorScheme::Light);

    app.wait_for("已下载").unwrap();
    app.wait_for("云端之上：千金归来").unwrap();
    app.wait_for("text:/3 集/").unwrap();

    app.press("云端之上：千金归来 >> nth=0").unwrap();
    let emitted = opened
        .last()
        .expect("pressing offline series must emit open intent");
    assert_eq!(emitted.0.get(), 1001);
    assert_eq!(emitted.1.map(|ep| ep.get()), Some(1));
}
