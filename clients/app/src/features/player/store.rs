use hg_contracts::catalog::{EpisodeComment, SeriesDetail};
use hg_contracts::ids::EpisodeNumber;
use hg_contracts::viewing::{PlaybackMode, PlaybackSource};
use nsh_core::prelude::*;

use super::cmd_playback::command::AspectPreset;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompanionTab {
    #[default]
    Episodes,
    Info,
    Comments,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerSurfaceKey {
    Loading,
    Error,
    Cinema { fullscreen: bool },
    ShortDrama,
}

#[derive(Debug, Clone)]
pub struct PlayerState {
    pub detail: Option<SeriesDetail>,
    pub current_episode: Option<EpisodeNumber>,
    pub source: Option<PlaybackSource>,
    pub mode: PlaybackMode,
    pub aspect: AspectPreset,
    pub paused: bool,
    pub muted: bool,
    pub fullscreen: bool,
    pub volume_percent: u8,
    pub rate_hundredths: u16,
    pub position_secs: u32,
    pub duration_secs: u32,
    pub comments: Vec<EpisodeComment>,
    pub status_hint: Option<String>,
    pub error: Option<String>,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            detail: None,
            current_episode: None,
            source: None,
            mode: PlaybackMode::Cinema,
            aspect: AspectPreset::Original,
            paused: false,
            muted: false,
            fullscreen: false,
            volume_percent: 80,
            rate_hundredths: 100,
            position_secs: 0,
            duration_secs: 0,
            comments: Vec::new(),
            status_hint: None,
            error: None,
        }
    }
}

fn state() -> Signal<PlayerState> {
    global::<PlayerState>()
}

#[must_use]
pub fn snapshot() -> PlayerState {
    state().get()
}

#[must_use]
pub fn surface_key() -> PlayerSurfaceKey {
    state().with(|s| {
        if s.error.is_some() {
            PlayerSurfaceKey::Error
        } else if s.detail.is_none() {
            PlayerSurfaceKey::Loading
        } else {
            match s.mode {
                PlaybackMode::Cinema => PlayerSurfaceKey::Cinema {
                    fullscreen: s.fullscreen,
                },
                PlaybackMode::ShortDrama => PlayerSurfaceKey::ShortDrama,
            }
        }
    })
}

#[must_use]
pub fn companion_tab_signal() -> Signal<CompanionTab> {
    global::<CompanionTab>()
}

pub fn reset_for_open(default_mode: PlaybackMode) {
    global::<CompanionTab>().set(CompanionTab::Episodes);
    state().set(PlayerState {
        mode: default_mode,
        ..PlayerState::default()
    });
}

pub fn set_detail(detail: SeriesDetail) {
    state().update(|s| {
        s.detail = Some(detail);
        s.error = None;
    });
}

pub fn set_source(ep_num: EpisodeNumber, source: PlaybackSource) {
    state().update(|s| {
        s.current_episode = Some(ep_num);
        s.duration_secs = source.duration_secs;
        s.position_secs = 0;
        s.paused = false;
        s.source = Some(source);
        s.error = None;
    });
}

pub fn set_comments(comments: Vec<EpisodeComment>) {
    state().update(|s| s.comments = comments);
}

pub fn set_position(position_secs: u32) {
    state().update(|s| s.position_secs = position_secs);
}

pub fn set_mode(mode: PlaybackMode) {
    state().update(|s| {
        s.mode = mode;
        s.status_hint = Some(match mode {
            PlaybackMode::Cinema => "已切换至电影模式".to_owned(),
            PlaybackMode::ShortDrama => "已切换至短剧模式".to_owned(),
        });
    });
}

pub fn set_aspect(aspect: AspectPreset) {
    state().update(|s| {
        s.aspect = aspect;
        s.status_hint = Some(format!("画面比例：{}", aspect.label()));
    });
}

pub fn toggle_paused() {
    state().update(|s| s.paused = !s.paused);
}

pub fn toggle_muted() {
    state().update(|s| s.muted = !s.muted);
}

pub fn toggle_fullscreen() {
    state().update(|s| {
        s.fullscreen = !s.fullscreen;
        s.status_hint = Some(if s.fullscreen {
            "全屏模式（Enter 退出）".to_owned()
        } else {
            "窗口模式".to_owned()
        });
    });
}

pub fn set_volume(volume_percent: u8) {
    state().update(|s| {
        s.volume_percent = volume_percent;
        s.status_hint = Some(format!("音量 {volume_percent}%"));
    });
}

pub fn set_rate(rate_hundredths: u16) {
    state().update(|s| {
        s.rate_hundredths = rate_hundredths;
        s.status_hint = Some(format!("倍速 {:.2}×", f32::from(rate_hundredths) / 100.0));
    });
}

pub fn set_hint(message: String) {
    state().update(|s| s.status_hint = Some(message));
}

pub fn set_error(message: String) {
    state().update(|s| s.error = Some(message));
}
