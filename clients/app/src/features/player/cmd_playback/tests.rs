use std::num::NonZeroU32;

use hg_contracts::ids::EpisodeNumber;

use super::command::{AspectPreset, adjacent_episode, clamp_seek, step_rate};

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
}

#[test]
fn adjacent_episode_respects_boundaries() {
    let ep1 = EpisodeNumber::new(NonZeroU32::new(1).unwrap());
    let ep2 = EpisodeNumber::new(NonZeroU32::new(2).unwrap());
    assert_eq!(adjacent_episode(ep1, 2, false), None);
    assert_eq!(adjacent_episode(ep1, 2, true), Some(ep2));
    assert_eq!(adjacent_episode(ep2, 2, true), None);
}
