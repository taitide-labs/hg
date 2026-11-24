use std::num::NonZeroU32;

use hg_contracts::ids::EpisodeNumber;

use super::command::{
    AspectPreset, ResumePlan, adjacent_episode, clamp_seek, episode_segment_ranges,
    folded_episode_window, format_rate_trigger, resume_plan, step_rate,
};

fn ep(value: u32) -> EpisodeNumber {
    EpisodeNumber::new(NonZeroU32::new(value).unwrap_or(NonZeroU32::MIN))
}

#[test]
fn resume_plan_continues_last_unfinished_episode_at_saved_position() {
    assert_eq!(
        resume_plan(None, Some((ep(7), 83, false)), 40),
        ResumePlan {
            episode: ep(7),
            position_secs: Some(83)
        }
    );
    assert_eq!(
        resume_plan(Some(ep(7)), Some((ep(7), 83, false)), 40),
        ResumePlan {
            episode: ep(7),
            position_secs: Some(83)
        }
    );
}

#[test]
fn resume_plan_advances_past_completed_episode_and_respects_explicit_choice() {
    assert_eq!(
        resume_plan(None, Some((ep(7), 120, true)), 40),
        ResumePlan {
            episode: ep(8),
            position_secs: None
        }
    );
    assert_eq!(
        resume_plan(None, Some((ep(40), 120, true)), 40),
        ResumePlan {
            episode: ep(40),
            position_secs: None
        }
    );
    assert_eq!(
        resume_plan(Some(ep(3)), Some((ep(7), 83, false)), 40),
        ResumePlan {
            episode: ep(3),
            position_secs: None
        }
    );
    assert_eq!(
        resume_plan(None, None, 40),
        ResumePlan {
            episode: ep(1),
            position_secs: None
        }
    );
    assert_eq!(
        resume_plan(None, Some((ep(2), 0, false)), 40),
        ResumePlan {
            episode: ep(2),
            position_secs: None
        }
    );
}

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
