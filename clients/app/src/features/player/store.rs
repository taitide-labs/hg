use std::collections::HashMap;

use hg_contracts::catalog::{EpisodeComment, SeriesDetail};
use hg_contracts::ids::{CommentId, EpisodeNumber, PageInfo, SeriesId};
use hg_contracts::viewing::{PlaybackMode, PlaybackSource, WatchProgress};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayerProgressState {
    pub position_secs: u32,
    pub duration_secs: u32,
    pub seek_seq: u64,
    pub scrub_preview_secs: Option<u32>,
    pub last_reported_secs: Option<u32>,
    pub pending_seek_target: Option<u32>,
    pub pending_seek_guard_ticks: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerControlsState {
    pub controls_visible: bool,
    pub controls_activity_seq: u64,
    pub controls_hovered: bool,
    pub popover: ActivePopover,
    pub paused: bool,
    pub muted: bool,
    pub rate_hundredths: u16,
    pub volume_percent: u8,
    pub aspect: AspectPreset,
    pub fullscreen: bool,
    pub status_hint: Option<String>,
}

impl Default for PlayerControlsState {
    fn default() -> Self {
        Self {
            controls_visible: false,
            controls_activity_seq: 0,
            controls_hovered: false,
            popover: ActivePopover::None,
            paused: false,
            muted: false,
            rate_hundredths: 100,
            volume_percent: 80,
            aspect: AspectPreset::Original,
            fullscreen: false,
            status_hint: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeCommentsState {
    pub items: Vec<EpisodeComment>,
    pub total_count: u64,
    pub next_page: u32,
    pub has_more: bool,
    pub loading: bool,
    pub loading_more: bool,
    pub error: Option<String>,
    pub request_seq: u64,
}

impl Default for EpisodeCommentsState {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            total_count: 0,
            next_page: 1,
            has_more: false,
            loading: false,
            loading_more: false,
            error: None,
            request_seq: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerState {
    pub detail: Option<SeriesDetail>,
    pub current_episode: Option<EpisodeNumber>,
    pub source: Option<PlaybackSource>,
    pub mode: PlaybackMode,
    pub bookmarked: bool,
    pub episodes_expanded: bool,
    pub episode_segment_index: usize,
    pub active_replies_comment: Option<CommentId>,
    pub batch_download_open: bool,
    pub auto_advance: bool,
    pub loop_single: bool,
    pub episode_seq: u64,
    pub error: Option<String>,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            detail: None,
            current_episode: None,
            source: None,
            mode: PlaybackMode::Cinema,
            bookmarked: false,
            episodes_expanded: false,
            episode_segment_index: 0,
            active_replies_comment: None,
            batch_download_open: false,
            auto_advance: true,
            loop_single: false,
            episode_seq: 0,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct LoadedPlaybackSeq(u64);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct EpisodeProgressMap(HashMap<EpisodeNumber, WatchProgress>);

fn state() -> Signal<PlayerState> {
    global::<PlayerState>()
}

fn progress_state() -> Signal<PlayerProgressState> {
    global::<PlayerProgressState>()
}

fn controls_state() -> Signal<PlayerControlsState> {
    global::<PlayerControlsState>()
}

fn comments_state() -> Signal<EpisodeCommentsState> {
    global::<EpisodeCommentsState>()
}

fn loaded_seq_state() -> Signal<LoadedPlaybackSeq> {
    global::<LoadedPlaybackSeq>()
}

fn episode_progress_state() -> Signal<EpisodeProgressMap> {
    global::<EpisodeProgressMap>()
}

#[must_use]
pub fn snapshot() -> PlayerState {
    state().get()
}

#[must_use]
pub fn progress_snapshot() -> PlayerProgressState {
    progress_state().get()
}

#[must_use]
pub fn controls_snapshot() -> PlayerControlsState {
    controls_state().get()
}

#[must_use]
pub fn comments_snapshot() -> EpisodeCommentsState {
    comments_state().get()
}

#[must_use]
pub fn snapshot_untracked() -> PlayerState {
    state().with_untracked(Clone::clone)
}

#[must_use]
pub fn progress_untracked() -> PlayerProgressState {
    progress_state().with_untracked(Clone::clone)
}

#[must_use]
pub fn controls_untracked() -> PlayerControlsState {
    controls_state().with_untracked(Clone::clone)
}

#[must_use]
pub fn is_fullscreen_untracked() -> bool {
    controls_state().with_untracked(|c| c.fullscreen)
}

#[must_use]
pub fn surface_key() -> PlayerSurfaceKey {
    let fullscreen = controls_state().with(|c| c.fullscreen);
    state().with(|s| {
        if s.error.is_some() {
            PlayerSurfaceKey::Error
        } else if s.detail.is_none() {
            PlayerSurfaceKey::Loading
        } else {
            match s.mode {
                PlaybackMode::Cinema => PlayerSurfaceKey::Regular { fullscreen },
                PlaybackMode::ShortDrama => PlayerSurfaceKey::VerticalImmersive,
            }
        }
    })
}

#[must_use]
pub fn mobile_tab_signal() -> Signal<MobileDetailTab> {
    global::<MobileDetailTab>()
}

pub fn reset_for_open(default_mode: PlaybackMode) -> u64 {
    global::<MobileDetailTab>().set(MobileDetailTab::Info);
    loaded_seq_state().set(LoadedPlaybackSeq(0));
    episode_progress_state().set(EpisodeProgressMap::default());
    progress_state().set(PlayerProgressState::default());
    controls_state().set(PlayerControlsState::default());
    comments_state().update(|c| {
        let next_req = c.request_seq.wrapping_add(1);
        *c = EpisodeCommentsState {
            request_seq: next_req,
            ..EpisodeCommentsState::default()
        };
    });
    let mut next_ep_seq = 0;
    state().update(|s| {
        next_ep_seq = s.episode_seq.wrapping_add(1).max(1);
        *s = PlayerState {
            mode: default_mode,
            episode_seq: next_ep_seq,
            ..PlayerState::default()
        };
    });
    next_ep_seq
}

#[must_use]
pub fn begin_episode_switch() -> u64 {
    loaded_seq_state().set(LoadedPlaybackSeq(0));
    progress_state().set(PlayerProgressState::default());
    let mut next = 0;
    state().update(|s| {
        next = s.episode_seq.wrapping_add(1).max(1);
        s.episode_seq = next;
        s.source = None;
        s.error = None;
    });
    next
}

pub fn hydrate_episode_progress(items: Vec<WatchProgress>) {
    let mut map = HashMap::with_capacity(items.len());
    for item in items {
        map.insert(item.key.ep_num, item);
    }
    episode_progress_state().set(EpisodeProgressMap(map));
}

pub fn record_episode_progress_local(progress: WatchProgress) {
    episode_progress_state().update(|m| {
        let prev_completed = m.0.get(&progress.key.ep_num).is_some_and(|p| p.completed);
        m.0.insert(
            progress.key.ep_num,
            WatchProgress {
                completed: progress.completed || prev_completed,
                ..progress
            },
        );
    });
}

#[must_use]
pub fn episode_progress_for_untracked(ep_num: EpisodeNumber) -> Option<WatchProgress> {
    episode_progress_state().with_untracked(|m| m.0.get(&ep_num).copied())
}

#[must_use]
pub fn is_episode_watched(ep_num: EpisodeNumber) -> bool {
    episode_progress_state().with(|m| m.0.contains_key(&ep_num))
}

#[must_use]
pub fn current_episode_seq() -> u64 {
    state().with_untracked(|s| s.episode_seq)
}

#[must_use]
pub fn is_playback_seq_active() -> bool {
    let seq = state().with_untracked(|s| s.episode_seq);
    let loaded = loaded_seq_state().with_untracked(|l| l.0);
    seq > 0 && seq == loaded
}

pub fn pause_and_cancel_loads() {
    controls_state().update(|c| {
        c.paused = true;
        c.controls_hovered = false;
        c.popover = ActivePopover::None;
    });
    state().update(|s| {
        s.episode_seq = s.episode_seq.wrapping_add(1).max(1);
    });
}

pub fn set_detail_if_seq(seq: u64, detail: SeriesDetail, bookmarked: bool) -> bool {
    let mut applied = false;
    state().update(|s| {
        if s.episode_seq != seq {
            return;
        }
        s.detail = Some(detail);
        s.bookmarked = bookmarked;
        s.error = None;
        applied = true;
    });
    applied
}

pub fn set_source_if_seq(seq: u64, ep_num: EpisodeNumber, source: PlaybackSource) -> bool {
    if state().with_untracked(|s| s.episode_seq != seq) {
        return false;
    }
    let duration_secs = source.duration_secs;
    let segment_index = (ep_num.get().saturating_sub(1) / 50) as usize;
    progress_state().set(PlayerProgressState {
        position_secs: 0,
        duration_secs,
        seek_seq: 0,
        scrub_preview_secs: None,
        last_reported_secs: None,
        pending_seek_target: None,
        pending_seek_guard_ticks: 0,
    });
    let mut source_slot = Some(source);
    let mut applied = false;
    state().update(|s| {
        if s.episode_seq != seq {
            return;
        }
        s.current_episode = Some(ep_num);
        s.episode_segment_index = segment_index;
        s.source = source_slot.take();
        s.error = None;
        applied = true;
    });
    if applied {
        loaded_seq_state().set(LoadedPlaybackSeq(seq));
        controls_state().update(|c| c.paused = false);
    }
    applied
}

pub fn begin_comments_load_if_seq(episode_seq: u64) -> Option<u64> {
    if current_episode_seq() != episode_seq {
        return None;
    }
    let mut req_seq = 0;
    comments_state().update(|c| {
        req_seq = c.request_seq.wrapping_add(1).max(1);
        *c = EpisodeCommentsState {
            items: Vec::new(),
            total_count: 0,
            next_page: 1,
            has_more: false,
            loading: true,
            loading_more: false,
            error: None,
            request_seq: req_seq,
        };
    });
    Some(req_seq)
}

pub fn begin_comments_load_more() -> Option<(u64, u32)> {
    let mut started = None;
    comments_state().update(|c| {
        if !c.has_more || c.loading || c.loading_more {
            return;
        }
        let req_seq = c.request_seq.wrapping_add(1).max(1);
        c.request_seq = req_seq;
        c.loading_more = true;
        c.error = None;
        started = Some((req_seq, c.next_page));
    });
    started
}

pub fn apply_comments_loaded(
    episode_seq: u64,
    req_seq: u64,
    items: Vec<EpisodeComment>,
    pagination: PageInfo,
) {
    if current_episode_seq() != episode_seq {
        return;
    }
    comments_state().update(|c| {
        if c.request_seq != req_seq {
            return;
        }
        c.items = items;
        c.total_count = pagination.total.max(c.items.len() as u64);
        c.has_more = pagination.page < pagination.pages;
        c.next_page = pagination.page.saturating_add(1);
        c.loading = false;
        c.loading_more = false;
        c.error = None;
    });
}

pub fn apply_comments_more(
    episode_seq: u64,
    req_seq: u64,
    incoming: Vec<EpisodeComment>,
    pagination: PageInfo,
) {
    if current_episode_seq() != episode_seq {
        return;
    }
    comments_state().update(|c| {
        if c.request_seq != req_seq {
            return;
        }
        for comment in incoming {
            if !c.items.iter().any(|existing| existing.id == comment.id) {
                c.items.push(comment);
            }
        }
        c.total_count = pagination.total.max(c.items.len() as u64);
        c.has_more = pagination.page < pagination.pages;
        c.next_page = pagination.page.saturating_add(1);
        c.loading_more = false;
        c.error = None;
    });
}

pub fn apply_comments_error(episode_seq: u64, req_seq: u64, initial: bool, message: String) {
    if current_episode_seq() != episode_seq {
        return;
    }
    comments_state().update(|c| {
        if c.request_seq != req_seq {
            return;
        }
        if initial {
            c.loading = false;
        } else {
            c.loading_more = false;
        }
        c.error = Some(message);
    });
}

pub fn set_scrub_preview(preview_secs: Option<u32>) {
    progress_state().update(|p| {
        if p.scrub_preview_secs != preview_secs {
            p.scrub_preview_secs = preview_secs;
        }
    });
}

pub fn seek_to(position_secs: u32) {
    progress_state().update(|p| {
        p.position_secs = position_secs;
        p.seek_seq = p.seek_seq.saturating_add(1);
        p.scrub_preview_secs = None;
        p.pending_seek_target = Some(position_secs);
        p.pending_seek_guard_ticks = 6;
    });
}

pub fn update_playback_progress(position_secs: u32, duration_secs: u32) -> bool {
    let mut accepted = true;
    progress_state().update(|p| {
        if duration_secs > 0 {
            p.duration_secs = duration_secs;
        }
        if let Some(target) = p.pending_seek_target {
            if position_secs.abs_diff(target) <= 3 {
                p.pending_seek_target = None;
                p.pending_seek_guard_ticks = 0;
                p.position_secs = position_secs;
            } else if p.pending_seek_guard_ticks > 0 {
                p.pending_seek_guard_ticks = p.pending_seek_guard_ticks.saturating_sub(1);
                accepted = false;
            } else {
                p.pending_seek_target = None;
                p.position_secs = position_secs;
            }
        } else {
            p.position_secs = position_secs;
        }
    });
    accepted
}

pub fn mark_progress_reported(position_secs: u32) {
    progress_state().update(|p| p.last_reported_secs = Some(position_secs));
}

pub fn set_bookmarked(bookmarked: bool) {
    state().update(|s| s.bookmarked = bookmarked);
}

pub fn set_mode(mode: PlaybackMode) {
    state().update(|s| s.mode = mode);
    controls_state().update(|c| c.popover = ActivePopover::None);
}

pub fn set_aspect(aspect: AspectPreset) {
    controls_state().update(|c| {
        c.aspect = aspect;
        c.status_hint = Some(aspect.label().to_owned());
    });
}

pub fn set_paused(paused: bool) {
    let currently_paused = controls_state().with_untracked(|c| c.paused);
    if currently_paused != paused {
        controls_state().update(|c| c.paused = paused);
    }
}

pub fn toggle_paused() {
    controls_state().update(|c| c.paused = !c.paused);
}

pub fn toggle_muted() {
    controls_state().update(|c| c.muted = !c.muted);
}

pub fn set_controls_hovered(hovered: bool) {
    let current = controls_state().with_untracked(|c| c.controls_hovered);
    if current != hovered {
        controls_state().update(|c| c.controls_hovered = hovered);
    }
}

#[must_use]
pub fn should_show_controls() -> bool {
    controls_state().with(|c| {
        c.controls_visible || c.paused || c.popover != ActivePopover::None || c.controls_hovered
    }) || progress_state().with(|p| p.scrub_preview_secs.is_some())
}

#[must_use]
pub fn should_keep_controls_visible() -> bool {
    controls_state().with(|c| c.paused || c.popover != ActivePopover::None || c.controls_hovered)
        || progress_state().with(|p| p.scrub_preview_secs.is_some())
}

#[must_use]
pub fn controls_activity_seq() -> u64 {
    controls_state().with(|c| c.controls_activity_seq)
}

pub fn reveal_controls() {
    controls_state().update(|c| {
        c.controls_visible = true;
        c.controls_activity_seq = c.controls_activity_seq.saturating_add(1);
    });
}

pub fn hide_controls() {
    let visible = controls_state().with_untracked(|c| c.controls_visible);
    if visible {
        controls_state().update(|c| c.controls_visible = false);
    }
}

pub fn leave_surface() {
    controls_state().update(|c| {
        c.controls_hovered = false;
        c.controls_visible = false;
    });
}

#[must_use]
pub fn toggle_fullscreen() -> bool {
    let mut next = false;
    controls_state().update(|c| {
        c.fullscreen = !c.fullscreen;
        c.popover = ActivePopover::None;
        c.controls_hovered = false;
        next = c.fullscreen;
    });
    next
}

pub fn toggle_auto_advance() {
    state().update(|s| s.auto_advance = !s.auto_advance);
}

pub fn toggle_loop_single() {
    state().update(|s| s.loop_single = !s.loop_single);
}

pub fn toggle_episodes_expanded() {
    state().update(|s| s.episodes_expanded = !s.episodes_expanded);
}

pub fn set_episode_segment(index: usize) {
    state().update(|s| s.episode_segment_index = index);
}

pub fn toggle_popover(target: ActivePopover) {
    controls_state().update(|c| {
        c.popover = if c.popover == target {
            ActivePopover::None
        } else {
            target
        };
    });
}

pub fn close_popover() {
    controls_state().update(|c| c.popover = ActivePopover::None);
}

pub fn open_comment_replies(comment_id: CommentId) {
    state().update(|s| s.active_replies_comment = Some(comment_id));
}

pub fn close_comment_replies() {
    state().update(|s| s.active_replies_comment = None);
}

#[must_use]
pub fn active_replies_target() -> Option<(SeriesId, CommentId)> {
    state().with(|s| {
        s.detail
            .as_ref()
            .map(|d| d.summary.id)
            .zip(s.active_replies_comment)
    })
}

pub fn set_batch_download_open(open: bool) {
    state().update(|s| s.batch_download_open = open);
}

#[must_use]
pub fn is_batch_download_ready() -> bool {
    state().with(|s| s.batch_download_open && s.detail.is_some())
}

pub fn set_volume(volume_percent: u8) {
    controls_state().update(|c| {
        c.volume_percent = volume_percent;
        c.status_hint = Some(format!("音量 {volume_percent}%"));
    });
}

pub fn set_rate(rate_hundredths: u16) {
    controls_state().update(|c| {
        c.rate_hundredths = rate_hundredths;
        c.popover = ActivePopover::None;
        c.status_hint = Some(format!("{:.2}×", f32::from(rate_hundredths) / 100.0));
    });
}

pub fn set_rate_silent(rate_hundredths: u16) {
    controls_state().update(|c| {
        c.rate_hundredths = rate_hundredths;
    });
}

pub fn set_hint(message: String) {
    controls_state().update(|c| c.status_hint = Some(message));
}

pub fn set_error_if_seq(seq: u64, message: String) {
    state().update(|s| {
        if s.episode_seq == seq {
            s.error = Some(message);
        }
    });
}
