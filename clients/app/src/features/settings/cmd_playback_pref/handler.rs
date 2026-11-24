use hg_contracts::ids::StreamProfile;
use hg_contracts::viewing::PlaybackPreference;
use nsh_core::prelude::*;

use super::super::store;
use super::command::{self, AppearanceOption};
use crate::gateway;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct PrefSaveGen(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SelectedProfile(StreamProfile);

impl Default for SelectedProfile {
    fn default() -> Self {
        Self(StreamProfile::Hevc)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SelectedRate(u16);

impl Default for SelectedRate {
    fn default() -> Self {
        Self(100)
    }
}

#[must_use]
pub fn appearance_signal() -> Signal<AppearanceOption> {
    global::<AppearanceOption>()
}

#[must_use]
pub fn profile_signal() -> Signal<StreamProfile> {
    let sig = global::<SelectedProfile>();
    let ui = signal(sig.with_untracked(|p| p.0));
    effect(move || {
        ui.set(sig.get().0);
    });
    ui
}

#[must_use]
pub fn rate_signal() -> Signal<u16> {
    let sig = global::<SelectedRate>();
    let ui = signal(sig.with_untracked(|r| r.0));
    effect(move || {
        ui.set(sig.get().0);
    });
    ui
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
                global::<SelectedProfile>().set(SelectedProfile(pref.preferred_profile));
                global::<SelectedRate>().set(SelectedRate(command::normalize_rate_hundredths(
                    pref.default_rate_hundredths,
                )));
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
    global::<SelectedProfile>().set(SelectedProfile(next.preferred_profile));
    save_preference(current, next);
}

pub fn select_default_rate(rate_hundredths: u16) {
    let Some(current) = store::state().with_untracked(|s| s.playback_pref) else {
        return;
    };
    let next = command::with_default_rate(current, rate_hundredths);
    global::<SelectedRate>().set(SelectedRate(next.default_rate_hundredths));
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
            global::<SelectedProfile>().set(SelectedProfile(previous.preferred_profile));
            global::<SelectedRate>().set(SelectedRate(command::normalize_rate_hundredths(
                previous.default_rate_hundredths,
            )));
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
