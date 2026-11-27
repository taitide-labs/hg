use serde_json::json;

use crate::protocol::{PropChange, VideoAspect, VideoPlayerEvent, VideoPlayerProps};

use super::commands::{
    DEFAULT_INIT_OPTIONS, MpvCommand, commands_for_attach, commands_for_prop_change,
    commands_for_recycle, volume_to_mpv_percent,
};
use super::layout::{MAX_RENDER_DIMENSION, clamp_render_dimensions, compute_target_rect};
use super::pool::{DEFAULT_POOL_CAPACITY, DoubleFrameBuffer, InstancePool};
use super::state::{MpvEndFileReason, WindowsPlayerState, map_end_file_event};

#[test]
fn init_options_include_libmpv_d3d11va_copy_keep_open_and_cache() {
    let contains = |name: &str, value: &str| {
        DEFAULT_INIT_OPTIONS
            .iter()
            .any(|opt| opt.name == name && opt.value == value)
    };
    assert!(contains("vo", "libmpv"));
    assert!(contains("hwdec", "d3d11va-copy"));
    assert!(contains("keep-open", "yes"));
    assert!(contains("cache", "yes"));
}

#[test]
fn compute_target_rect_handles_all_aspect_modes_and_bounds() {
    let original = compute_target_rect(VideoAspect::Original, (1920.0, 1080.0), (960.0, 960.0));
    assert!((original.left - 0.0).abs() < 0.01);
    assert!((original.top - 210.0).abs() < 0.01);
    assert!((original.width - 960.0).abs() < 0.01);
    assert!((original.height - 540.0).abs() < 0.01);

    let fill = compute_target_rect(VideoAspect::Fill, (1080.0, 1920.0), (1080.0, 1080.0));
    assert!((fill.left - 0.0).abs() < 0.01);
    assert!((fill.top + 420.0).abs() < 0.01);
    assert!((fill.width - 1080.0).abs() < 0.01);
    assert!((fill.height - 1920.0).abs() < 0.01);

    let cinema = compute_target_rect(VideoAspect::Cinema16x9, (1080.0, 1920.0), (1600.0, 1000.0));
    assert!((cinema.width - 1600.0).abs() < 0.01);
    assert!((cinema.height - 900.0).abs() < 0.01);
    assert!((cinema.top - 50.0).abs() < 0.01);

    let portrait = compute_target_rect(
        VideoAspect::Portrait9x16,
        (1920.0, 1080.0),
        (1600.0, 1600.0),
    );
    assert!((portrait.width - 900.0).abs() < 0.01);
    assert!((portrait.height - 1600.0).abs() < 0.01);
    assert!((portrait.left - 350.0).abs() < 0.01);

    let zero_viewport = compute_target_rect(VideoAspect::Original, (1920.0, 1080.0), (0.0, 100.0));
    assert!((zero_viewport.width - 0.0).abs() < f32::EPSILON);
    assert!((zero_viewport.height - 0.0).abs() < f32::EPSILON);
}

#[test]
fn clamp_render_dimensions_preserves_ratio_and_guards_zero() {
    assert_eq!(
        clamp_render_dimensions((0, 720), MAX_RENDER_DIMENSION),
        (0, 0)
    );
    assert_eq!(
        clamp_render_dimensions((1920, 1080), MAX_RENDER_DIMENSION),
        (1920, 1080)
    );
    assert_eq!(
        clamp_render_dimensions((7680, 4320), MAX_RENDER_DIMENSION),
        (3840, 2160)
    );
    assert_eq!(
        clamp_render_dimensions((2160, 7680), MAX_RENDER_DIMENSION),
        (1080, 3840)
    );
}

#[test]
fn prop_changes_map_to_expected_mpv_commands() {
    let mut props = VideoPlayerProps::default();

    let change_src = props.apply_prop("src", Some(&json!("http://127.0.0.1:9000/ep1.m3u8")));
    assert_eq!(change_src, PropChange::Source);
    assert_eq!(
        commands_for_prop_change(&props, change_src),
        vec![
            MpvCommand::LoadFile {
                url: "http://127.0.0.1:9000/ep1.m3u8".to_owned()
            },
            MpvCommand::SetRate(1.0),
            MpvCommand::SetVolume(80.0),
            MpvCommand::SetMuted(false),
            MpvCommand::SetPaused(false),
        ]
    );

    let change_pause = props.apply_prop("paused", Some(&json!(true)));
    assert_eq!(
        commands_for_prop_change(&props, change_pause),
        vec![MpvCommand::SetPaused(true)]
    );

    let change_rate = props.apply_prop("rate", Some(&json!(2.0)));
    assert_eq!(
        commands_for_prop_change(&props, change_rate),
        vec![MpvCommand::SetRate(2.0)]
    );

    let change_vol = props.apply_prop("volume", Some(&json!(0.35)));
    assert_eq!(
        commands_for_prop_change(&props, change_vol),
        vec![MpvCommand::SetVolume(volume_to_mpv_percent(0.35))]
    );

    let change_mute = props.apply_prop("muted", Some(&json!(true)));
    assert_eq!(
        commands_for_prop_change(&props, change_mute),
        vec![MpvCommand::SetMuted(true)]
    );

    assert_eq!(
        props.apply_prop("seek_secs", Some(&json!(65))),
        PropChange::Unchanged
    );
    let change_seek = props.apply_prop("seek_seq", Some(&json!(1)));
    assert_eq!(
        commands_for_prop_change(&props, change_seek),
        vec![MpvCommand::SeekAbsolute(65)]
    );

    let change_clear = props.apply_prop("src", None);
    assert_eq!(
        commands_for_prop_change(&props, change_clear),
        vec![MpvCommand::Stop]
    );

    let attach = commands_for_attach(&VideoPlayerProps {
        src: Some("http://127.0.0.1:9000/ep2.m3u8".to_owned()),
        paused: true,
        rate: 1.5,
        volume: 0.5,
        muted: true,
        seek_secs: 0,
        seek_seq: 0,
        aspect: VideoAspect::Fill,
    });
    assert_eq!(attach.len(), 5);
    assert_eq!(
        commands_for_recycle(),
        vec![
            MpvCommand::Stop,
            MpvCommand::SetPaused(true),
            MpvCommand::SetRate(1.0),
            MpvCommand::SetVolume(80.0),
            MpvCommand::SetMuted(false),
        ]
    );
}

#[test]
fn instance_pool_enforces_capacity_and_lifo_reuse() {
    let mut pool = InstancePool::default();
    assert_eq!(pool.capacity(), DEFAULT_POOL_CAPACITY);
    assert!(pool.is_empty());

    assert!(pool.release(10).is_none());
    assert!(pool.release(20).is_none());
    assert!(pool.release(30).is_none());
    assert_eq!(pool.len(), 3);
    assert_eq!(pool.release(40), Some(40));

    assert_eq!(pool.acquire(), Some(30));
    assert_eq!(pool.acquire(), Some(20));
    assert_eq!(pool.drain_all(), vec![10]);
    assert!(pool.is_empty());
}

#[test]
fn double_frame_buffer_reuses_planes_and_advances_generation() {
    let mut buffer = DoubleFrameBuffer::new();
    assert!(buffer.front().is_none());
    assert!(buffer.checkout_render_target(0, 10).is_none());

    let mut first = match buffer.checkout_render_target(4, 2) {
        Some(target) => target,
        None => panic!("expected render target"),
    };
    assert_eq!(first.stride, 16);
    assert_eq!(first.pixels.len(), 32);
    first.pixels.fill(0xAA);
    let gen1 = buffer.publish_rendered_frame(first);
    assert_eq!(gen1, 1);
    assert_eq!(buffer.front().map(|f| (f.width, f.height)), Some((4, 2)));

    let mut second = match buffer.checkout_render_target(4, 2) {
        Some(target) => target,
        None => panic!("expected reusable spare target"),
    };
    second.pixels.fill(0x55);
    let gen2 = buffer.publish_rendered_frame(second);
    assert_eq!(gen2, 2);
    assert_eq!(
        buffer.front().and_then(|f| f.pixels.first().copied()),
        Some(0x55)
    );

    buffer.reset();
    assert!(buffer.front().is_none());
    assert_eq!(buffer.generation(), 3);
}

#[test]
fn player_state_tracks_progress_dedup_end_file_and_errors() {
    let mut state = WindowsPlayerState::new();
    let aspect_outcome = state.apply_prop("aspect", Some(&json!("16:9")));
    assert!(aspect_outcome.commands.is_empty());
    assert!(aspect_outcome.needs_redraw);

    assert_eq!(state.on_duration(90.9), None);
    assert_eq!(
        state.on_time_pos(12.8),
        Some(VideoPlayerEvent::Progress {
            position_secs: 12,
            duration_secs: 90,
        })
    );
    assert_eq!(state.on_time_pos(12.9), None);
    assert_eq!(
        state.on_time_pos(13.1),
        Some(VideoPlayerEvent::Progress {
            position_secs: 13,
            duration_secs: 90,
        })
    );
    assert!(state.on_time_pos(f64::NAN).is_some());

    assert_eq!(MpvEndFileReason::from_raw(0), MpvEndFileReason::Eof);
    assert_eq!(
        map_end_file_event(0, 0, None),
        Some(VideoPlayerEvent::Ended)
    );
    assert_eq!(map_end_file_event(2, 0, None), None);
    assert_eq!(map_end_file_event(3, 0, None), None);
    assert_eq!(
        map_end_file_event(4, -13, Some("loading failed")),
        Some(VideoPlayerEvent::Error {
            message: "libmpv playback failed (-13): loading failed".to_owned(),
        })
    );

    assert_eq!(
        state.record_error_once("missing dll".to_owned()),
        Some(VideoPlayerEvent::Error {
            message: "missing dll".to_owned(),
        })
    );
    assert_eq!(state.record_error_once("missing dll".to_owned()), None);

    let next_outcome = state.apply_prop("src", Some(&json!("http://127.0.0.1:9000/next.m3u8")));
    assert_eq!(next_outcome.commands.len(), 5);
    assert!(state.record_error_once("missing dll".to_owned()).is_some());
}
