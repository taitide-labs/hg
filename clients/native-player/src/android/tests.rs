use nsh_core::JsonValue;
use serde_json::json;

use super::{
    ANDROID_COMPONENT_INTERFACE, ANDROID_COMPONENT_NAME, ANDROID_DEFAULT_POOL_CAPACITY,
    ANDROID_EVENT_NAMES, ANDROID_HOST_PACKAGE, ANDROID_PROGRESS_INTERVAL_MS, ANDROID_PROP_KEYS,
    ANDROID_REGISTRATION_ENTRY, AndroidBridgeError, AndroidPlayerBridgeSpec, AndroidRenderSurface,
};
use crate::protocol::{
    PropChange, VIDEO_PLAYER_COMPONENT_NAME, VideoAspect, VideoPlayerEvent, VideoPlayerProps,
};

#[test]
fn android_bridge_spec_matches_kotlin_host_contract() {
    let spec = AndroidPlayerBridgeSpec::current();
    assert_eq!(spec, AndroidPlayerBridgeSpec::default());
    assert_eq!(spec.component_name, VIDEO_PLAYER_COMPONENT_NAME);
    assert_eq!(spec.component_name, ANDROID_COMPONENT_NAME);
    assert_eq!(spec.component_name, "hg-video-player");
    assert_eq!(spec.host_package, ANDROID_HOST_PACKAGE);
    assert_eq!(spec.host_package, "com.taitidelabs.hg.player");
    assert_eq!(spec.registration_entry, ANDROID_REGISTRATION_ENTRY);
    assert_eq!(spec.component_interface, ANDROID_COMPONENT_INTERFACE);
    assert_eq!(spec.render_surface, AndroidRenderSurface::TextureView);
    assert_eq!(spec.render_surface.as_str(), "TextureView");
    assert!(spec.uses_texture_view());
    assert_eq!(spec.default_pool_capacity, ANDROID_DEFAULT_POOL_CAPACITY);
    assert_eq!(spec.default_pool_capacity, 3);
    assert_eq!(spec.progress_interval_ms, ANDROID_PROGRESS_INTERVAL_MS);
    assert_eq!(spec.progress_interval_ms, 500);
    assert_eq!(spec.prop_keys, ANDROID_PROP_KEYS);
    assert_eq!(spec.event_names, ANDROID_EVENT_NAMES);
}

#[test]
fn android_bridge_prop_keys_align_with_protocol_state_machine() {
    let spec = AndroidPlayerBridgeSpec::current();
    for key in [
        "src",
        "paused",
        "rate",
        "volume",
        "muted",
        "seek_secs",
        "seek_seq",
        "aspect",
    ] {
        assert!(spec.supports_prop(key));
    }
    assert!(!spec.supports_prop("unknown"));

    let mut props = VideoPlayerProps::default();
    assert!((props.volume - 0.8).abs() < f64::EPSILON);
    assert!((props.rate - 1.0).abs() < f64::EPSILON);
    assert!(!props.paused);
    assert!(!props.muted);
    assert_eq!(props.aspect, VideoAspect::Original);

    assert_eq!(
        props.apply_prop(
            "src",
            Some(&json!("http://127.0.0.1:9000/live/stream.m3u8"))
        ),
        PropChange::Source
    );
    assert_eq!(
        props.apply_prop("paused", Some(&json!(true))),
        PropChange::Paused
    );
    assert_eq!(
        props.apply_prop("rate", Some(&json!(2.0))),
        PropChange::Rate
    );
    assert_eq!(
        props.apply_prop("volume", Some(&json!(0.4))),
        PropChange::Volume
    );
    assert_eq!(
        props.apply_prop("muted", Some(&json!(true))),
        PropChange::Muted
    );
    assert_eq!(
        props.apply_prop("seek_secs", Some(&json!(35))),
        PropChange::Unchanged
    );
    assert_eq!(
        props.apply_prop("seek_seq", Some(&json!(1))),
        PropChange::Seek { position_secs: 35 }
    );
    assert_eq!(
        props.apply_prop("aspect", Some(&json!("fill"))),
        PropChange::Aspect
    );
    assert_eq!(props.aspect.as_protocol_str(), "fill");
}

#[test]
fn android_bridge_events_roundtrip_serialize_and_parse() {
    let spec = AndroidPlayerBridgeSpec::current();
    for event_name in ["progress", "ended", "error"] {
        assert!(spec.supports_event(event_name));
    }
    assert!(!spec.supports_event("ready"));

    let cases = vec![
        VideoPlayerEvent::Progress {
            position_secs: 42,
            duration_secs: 180,
        },
        VideoPlayerEvent::Ended,
        VideoPlayerEvent::Error {
            message: "ExoPlayer source error".to_owned(),
        },
    ];

    for original in cases {
        let (name, payload) = spec.encode_event(original.clone());
        let json_text = serde_json::to_string(&payload).expect("serialize payload");
        let decoded_payload: JsonValue =
            serde_json::from_str(&json_text).expect("deserialize payload");
        let parsed = spec
            .parse_event(name, &decoded_payload)
            .expect("parse android bridge event");
        assert_eq!(parsed, original);
    }
}

#[test]
fn android_bridge_rejects_invalid_event_names_and_payloads() {
    let spec = AndroidPlayerBridgeSpec::current();

    assert_eq!(
        spec.parse_event("buffering", &JsonValue::Null),
        Err(AndroidBridgeError::UnknownEvent("buffering".to_owned()))
    );
    assert_eq!(
        spec.parse_event("ended", &json!({ "unexpected": true })),
        Err(AndroidBridgeError::ExpectedNullPayload { event: "ended" })
    );
    assert_eq!(
        spec.parse_event("progress", &json!({ "position_secs": 10 })),
        Err(AndroidBridgeError::MissingOrInvalidField {
            event: "progress",
            field: "duration_secs",
        })
    );
    assert_eq!(
        spec.parse_event(
            "progress",
            &json!({ "position_secs": u64::from(u32::MAX) + 1, "duration_secs": 10 })
        ),
        Err(AndroidBridgeError::MissingOrInvalidField {
            event: "progress",
            field: "position_secs",
        })
    );
    assert_eq!(
        spec.parse_event("error", &json!({ "code": 500 })),
        Err(AndroidBridgeError::MissingOrInvalidField {
            event: "error",
            field: "message",
        })
    );
}
