use nsh_core::prelude::*;
use nsh_core::{EventKind, EventPayload, PayloadValue, ViewKind};
use nsh_engine::EngineConfig;
use nsh_testing::Headless;

use super::{VideoSurfaceState, nsh_video_surface};

fn sample_state(src: Option<&str>) -> VideoSurfaceState {
    VideoSurfaceState {
        src: src.map(str::to_owned),
        paused: false,
        rate: 1.0,
        volume: 1.0,
        muted: false,
        seek_secs: 0,
        seek_seq: 0,
        aspect: "contain",
    }
}

#[component]
fn VideoSurfaceHarness() -> impl IntoElement {
    let src_state = signal(sample_state(Some("http://127.0.0.1/ep1.m3u8")));
    let last_progress = signal((0_u32, 0_u32));
    let ended_count = signal(0_u32);
    let last_error = signal(String::new());

    view()
        .size_full()
        .child(
            view()
                .child(text("清空视频源"))
                .on_press(move |_: &PressEvent| src_state.set(sample_state(None))),
        )
        .child(
            view()
                .child(text("切集视频源"))
                .on_press(move |_: &PressEvent| {
                    src_state.set(sample_state(Some("http://127.0.0.1/ep2.m3u8")));
                }),
        )
        .child(text(move || {
            let (pos, dur) = last_progress.get();
            let ended = ended_count.get();
            let err = last_error.get();
            format!("progress:{pos}/{dur}|ended:{ended}|err:{err}")
        }))
        .child(view().w(px(320.0)).h(px(180.0)).child(nsh_video_surface(
            move || src_state.get(),
            move |pos, dur| last_progress.set((pos, dur)),
            move || ended_count.update(|n| *n += 1),
            move |msg| last_error.set(msg),
        )))
}

#[test]
fn nsh_video_surface_remounts_across_source_transitions_and_dispatches_callbacks() {
    let root = nsh_core::element::Root::new::<VideoSurfaceHarness>(Default::default);
    let mut app = Headless::start(EngineConfig::new(root)).unwrap();
    app.wait_for("清空视频源").unwrap();

    assert!(
        app.screen()
            .layout_nodes()
            .iter()
            .any(|node| node.kind == ViewKind::Video)
    );

    app.press("清空视频源").unwrap();
    assert!(
        !app.screen()
            .layout_nodes()
            .iter()
            .any(|node| node.kind == ViewKind::Video)
    );

    app.press("切集视频源").unwrap();
    let video_id = app
        .screen()
        .layout_nodes()
        .into_iter()
        .find(|node| node.kind == ViewKind::Video)
        .map(|node| node.id)
        .expect("video node must remount after source returns");

    app.engine().control().dispatch_event(
        video_id,
        EventKind::Progress,
        EventPayload::Map(vec![
            ("currentTime", PayloadValue::Number(12.6)),
            ("duration", PayloadValue::Number(90.2)),
        ]),
    );
    app.engine()
        .control()
        .dispatch_event(video_id, EventKind::End, EventPayload::None);
    app.engine().control().dispatch_event(
        video_id,
        EventKind::Error,
        EventPayload::Error {
            message: "decode failed".to_owned(),
        },
    );

    app.wait_for("text:progress:13/90|ended:1|err:decode failed")
        .unwrap();
}
