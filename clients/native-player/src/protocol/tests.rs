use serde_json::json;

use super::{PropChange, VideoAspect, VideoPlayerEvent, VideoPlayerProps};

#[test]
fn applies_props_and_triggers_seek_only_on_seek_seq_advance() {
    let mut props = VideoPlayerProps::default();

    assert_eq!(
        props.apply_prop(
            "src",
            Some(&json!("http://127.0.0.1:8888/online/1/index.m3u8"))
        ),
        PropChange::Source
    );
    assert_eq!(
        props.apply_prop(
            "src",
            Some(&json!("http://127.0.0.1:8888/online/1/index.m3u8"))
        ),
        PropChange::Unchanged
    );
    assert_eq!(
        props.apply_prop("paused", Some(&json!(true))),
        PropChange::Paused
    );
    assert_eq!(
        props.apply_prop("rate", Some(&json!(1.5))),
        PropChange::Rate
    );
    assert_eq!(
        props.apply_prop("volume", Some(&json!(0.5))),
        PropChange::Volume
    );
    assert_eq!(
        props.apply_prop("muted", Some(&json!(true))),
        PropChange::Muted
    );
    assert_eq!(
        props.apply_prop("aspect", Some(&json!("9:16"))),
        PropChange::Aspect
    );
    assert_eq!(props.aspect, VideoAspect::Portrait9x16);

    assert_eq!(
        props.apply_prop("seek_secs", Some(&json!(42))),
        PropChange::Unchanged
    );
    assert_eq!(
        props.apply_prop("seek_seq", Some(&json!(1))),
        PropChange::Seek { position_secs: 42 }
    );
    assert_eq!(
        props.apply_prop("seek_seq", Some(&json!(1))),
        PropChange::Unchanged
    );
}

#[test]
fn encodes_video_player_events_for_nsh_bridge() {
    let (name, payload) = VideoPlayerEvent::Progress {
        position_secs: 18,
        duration_secs: 90,
    }
    .into_name_and_payload();
    assert_eq!(name, "progress");
    assert_eq!(payload.get("position_secs"), Some(&json!(18)));
    assert_eq!(payload.get("duration_secs"), Some(&json!(90)));

    let (ended_name, ended_payload) = VideoPlayerEvent::Ended.into_name_and_payload();
    assert_eq!(ended_name, "ended");
    assert!(ended_payload.is_null());

    let (err_name, err_payload) = VideoPlayerEvent::Error {
        message: "decode failed".to_owned(),
    }
    .into_name_and_payload();
    assert_eq!(err_name, "error");
    assert_eq!(err_payload.get("message"), Some(&json!("decode failed")));
}

#[test]
fn handles_prop_reset_unknown_keys_and_boundary_clamping() {
    let mut props = VideoPlayerProps::default();
    assert_eq!(
        props.apply_prop("src", Some(&json!("http://127.0.0.1:9000/a.m3u8"))),
        PropChange::Source
    );
    assert_eq!(props.apply_prop("src", None), PropChange::Source);
    assert_eq!(props.src, None);
    assert_eq!(props.apply_prop("src", None), PropChange::Unchanged);

    assert_eq!(
        props.apply_prop("unknown_prop", Some(&json!(123))),
        PropChange::Unchanged
    );
    assert_eq!(
        props.apply_prop("rate", Some(&json!(99.0))),
        PropChange::Rate
    );
    assert!(props.rate <= 4.0 && props.rate >= 0.25);

    assert_eq!(
        props.apply_prop("volume", Some(&json!(-5.0))),
        PropChange::Volume
    );
    assert!((0.0..=1.0).contains(&props.volume));

    assert_eq!(
        props.apply_prop("aspect", Some(&json!("invalid-aspect"))),
        PropChange::Unchanged
    );
    assert_eq!(props.aspect, VideoAspect::default());
}
