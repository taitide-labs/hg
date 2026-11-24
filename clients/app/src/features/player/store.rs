use hg_contracts::catalog::{EpisodeComment, SeriesDetail};
use hg_contracts::ids::{CommentId, EpisodeNumber};
use hg_contracts::viewing::{PlaybackMode, PlaybackSource};
use nsh_core::prelude::*;

use super::cmd_playback::command::AspectPreset;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MobileDetailTab {
    #[default]
    Info,
    Comments,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActivePopover {
    #[default]
    None,
    Speed,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerSurfaceKey {
    Loading,
    Error,
    Regular { fullscreen: bool },
    VerticalImmersive,
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
    pub controls_hovered: bool,
    pub auto_advance: bool,
    pub loop_single: bool,
    pub bookmarked: bool,
    pub info_expanded: bool,
    pub episodes_expanded: bool,
    pub episode_segment_index: usize,
    pub popover: ActivePopover,
    pub active_replies_comment: Option<CommentId>,
    pub batch_download_open: bool,
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
            controls_hovered: false,
            auto_advance: true,
            loop_single: false,
            bookmarked: false,
            info_expanded: false,
            episodes_expanded: false,
            episode_segment_index: 0,
            popover: ActivePopover::None,
            active_replies_comment: None,
            batch_download_open: false,
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
                PlaybackMode::Cinema => PlayerSurfaceKey::Regular {
                    fullscreen: s.fullscreen,
                },
                PlaybackMode::ShortDrama => PlayerSurfaceKey::VerticalImmersive,
            }
        }
    })
}

#[must_use]
pub fn mobile_tab_signal() -> Signal<MobileDetailTab> {
    global::<MobileDetailTab>()
}

pub fn reset_for_open(default_mode: PlaybackMode) {
    global::<MobileDetailTab>().set(MobileDetailTab::Info);
    state().set(PlayerState {
        mode: default_mode,
        ..PlayerState::default()
    });
}

pub fn set_detail(detail: SeriesDetail, bookmarked: bool) {
    state().update(|s| {
        s.detail = Some(detail);
        s.bookmarked = bookmarked;
        s.error = None;
    });
}

pub fn set_source(ep_num: EpisodeNumber, source: PlaybackSource) {
    let segment_index = (ep_num.get().saturating_sub(1) / 50) as usize;
    state().update(|s| {
        s.current_episode = Some(ep_num);
        s.episode_segment_index = segment_index;
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

pub fn set_bookmarked(bookmarked: bool) {
    state().update(|s| s.bookmarked = bookmarked);
}

pub fn set_mode(mode: PlaybackMode) {
    state().update(|s| {
        s.mode = mode;
        s.popover = ActivePopover::None;
    });
}

pub fn set_aspect(aspect: AspectPreset) {
    state().update(|s| {
        s.aspect = aspect;
        s.status_hint = Some(aspect.label().to_owned());
    });
}

pub fn toggle_paused() {
    state().update(|s| s.paused = !s.paused);
}

pub fn toggle_muted() {
    state().update(|s| s.muted = !s.muted);
}

#[must_use]
pub fn should_show_controls() -> bool {
    state().with(|s| s.controls_hovered || s.paused || s.popover != ActivePopover::None)
}

pub fn set_controls_hovered(hovered: bool) {
    let current = state().with_untracked(|s| s.controls_hovered);
    if current != hovered {
        state().update(|s| s.controls_hovered = hovered);
    }
}

#[must_use]
pub fn toggle_fullscreen() -> bool {
    let mut next = false;
    state().update(|s| {
        s.fullscreen = !s.fullscreen;
        s.popover = ActivePopover::None;
        next = s.fullscreen;
    });
    next
}

pub fn toggle_auto_advance() {
    state().update(|s| s.auto_advance = !s.auto_advance);
}

pub fn toggle_loop_single() {
    state().update(|s| s.loop_single = !s.loop_single);
}

pub fn toggle_info_expanded() {
    state().update(|s| s.info_expanded = !s.info_expanded);
}

pub fn toggle_episodes_expanded() {
    state().update(|s| s.episodes_expanded = !s.episodes_expanded);
}

pub fn set_episode_segment(index: usize) {
    state().update(|s| s.episode_segment_index = index);
}

pub fn toggle_popover(target: ActivePopover) {
    state().update(|s| {
        s.popover = if s.popover == target {
            ActivePopover::None
        } else {
            target
        };
    });
}

pub fn close_popover() {
    state().update(|s| s.popover = ActivePopover::None);
}

pub fn open_comment_replies(comment_id: CommentId) {
    state().update(|s| s.active_replies_comment = Some(comment_id));
}

pub fn set_batch_download_open(open: bool) {
    state().update(|s| s.batch_download_open = open);
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
        s.popover = ActivePopover::None;
        s.status_hint = Some(format!("{:.2}×", f32::from(rate_hundredths) / 100.0));
    });
}

pub fn set_hint(message: String) {
    state().update(|s| s.status_hint = Some(message));
}

pub fn set_error(message: String) {
    state().update(|s| s.error = Some(message));
}
