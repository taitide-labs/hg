use hg_contracts::ids::StreamProfile;
use hg_contracts::viewing::PlaybackPreference;
use nsh_core::prelude::*;

use super::super::store;
use super::command::{self, AppearanceOption};
use crate::gateway;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct PrefSaveGen(u64);

#[must_use]
pub fn appearance_signal() -> Signal<AppearanceOption> {
    global::<AppearanceOption>()
}

#[must_use]
pub fn selected_profile() -> StreamProfile {
    store::state().with(|s| {
        s.playback_pref
            .map_or(StreamProfile::Hevc, |p| p.preferred_profile)
    })
}

#[must_use]
pub fn selected_rate() -> u16 {
    store::state().with(|s| {
        s.playback_pref.map_or(100, |p| {
            command::normalize_rate_hundredths(p.default_rate_hundredths)
        })
    })
}

pub fn select_appearance(option: AppearanceOption) {
    appearance_signal().set(option);
    set_color_scheme(option.into());
}

pub fn load_playback_preference() {
    let gw = gateway::current();
    spawn(async move {
        match gw.playback_preference().await {
            Ok(pref) => {
                store::state().update(|s| {
                    s.playback_pref = Some(pref);
                    s.error = None;
                });
            }
            Err(err) => {
                store::state().update(|s| s.error = Some(err.to_string()));
            }
        }
    });
}

pub fn select_profile(profile: StreamProfile) {
    let Some(current) = store::state().with_untracked(|s| s.playback_pref) else {
        return;
    };
    let next = command::with_preferred_profile(current, profile);
    save_preference(current, next);
}

pub fn select_default_rate(rate_hundredths: u16) {
    let Some(current) = store::state().with_untracked(|s| s.playback_pref) else {
        return;
    };
    let next = command::with_default_rate(current, rate_hundredths);
    save_preference(current, next);
}

fn save_preference(previous: PlaybackPreference, next: PlaybackPreference) {
    let seq = next_save_generation();
    store::state().update(|s| {
        s.playback_pref = Some(next);
        s.error = None;
    });

    let gw = gateway::current();
    spawn(async move {
        if let Err(err) = gw.update_playback_preference(next).await
            && is_latest_generation(seq)
        {
            store::state().update(|s| {
                s.playback_pref = Some(previous);
                s.error = Some(err.to_string());
            });
        }
    });
}

fn next_save_generation() -> u64 {
    let mut next = 0;
    global::<PrefSaveGen>().update(|g| {
        g.0 = g.0.saturating_add(1);
        next = g.0;
    });
    next
}

fn is_latest_generation(seq: u64) -> bool {
    global::<PrefSaveGen>().with_untracked(|g| g.0 == seq)
}
