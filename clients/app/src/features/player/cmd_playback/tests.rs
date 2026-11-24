use std::num::NonZeroU32;

use hg_contracts::ids::EpisodeNumber;

use super::command::{
    AspectPreset, adjacent_episode, clamp_seek, episode_segment_ranges, folded_episode_window,
    format_rate_trigger, step_rate,
};

#[test]
fn potplayer_aspect_digits_map_one_through_four() {
    assert_eq!(AspectPreset::from_digit(1), Some(AspectPreset::Original));
    assert_eq!(AspectPreset::from_digit(2), Some(AspectPreset::Fill));
    assert_eq!(AspectPreset::from_digit(3), Some(AspectPreset::Cinema16x9));
    assert_eq!(
        AspectPreset::from_digit(4),
        Some(AspectPreset::Portrait9x16)
    );
    assert_eq!(AspectPreset::from_digit(5), None);
}

#[test]
fn seek_and_rate_clamping_stay_in_bounds() {
    assert_eq!(clamp_seek(10, -25, 120), 0);
    assert_eq!(clamp_seek(115, 15, 120), 120);
    assert_eq!(step_rate(100, true), 125);
    assert_eq!(step_rate(200, true), 200);
    assert_eq!(step_rate(50, false), 50);
    assert_eq!(format_rate_trigger(100), "倍速");
    assert_eq!(format_rate_trigger(125), "1.25×");
    assert_eq!(format_rate_trigger(150), "1.5×");
}

#[test]
fn adjacent_episode_respects_boundaries() {
    let ep1 = EpisodeNumber::new(NonZeroU32::new(1).unwrap());
    let ep2 = EpisodeNumber::new(NonZeroU32::new(2).unwrap());
    assert_eq!(adjacent_episode(ep1, 2, false), None);
    assert_eq!(adjacent_episode(ep1, 2, true), Some(ep2));
    assert_eq!(adjacent_episode(ep2, 2, true), None);
}

#[test]
fn folded_episode_window_centers_around_current_episode() {
    assert_eq!(folded_episode_window(1, 68, 8), (1, 8));
    assert_eq!(folded_episode_window(18, 68, 8), (14, 21));
    assert_eq!(folded_episode_window(68, 68, 8), (61, 68));
    assert_eq!(folded_episode_window(3, 6, 8), (1, 6));
    assert_eq!(episode_segment_ranges(68, 50), vec![(1, 50), (51, 68)]);
}
