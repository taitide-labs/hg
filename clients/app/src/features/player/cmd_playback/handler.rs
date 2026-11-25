use std::num::{NonZeroU16, NonZeroU32};

use hg_contracts::catalog::ListEpisodeCommentsRequest;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, PageRequest, SeriesId};
use hg_contracts::viewing::{PlaybackMode, PlaybackOrigin, ReportWatchProgressRequest};
use nsh_core::prelude::*;

use super::super::store;
use super::command::{adjacent_episode, clamp_seek, resume_plan, step_rate, step_volume};
use crate::gateway;

const COMMENTS_PAGE_SIZE: u16 = 20;

pub fn open_series(
    series_id: SeriesId,
    initial_episode: Option<EpisodeNumber>,
    default_mode: PlaybackMode,
) {
    let seq = store::reset_for_open(default_mode);
    let gw = gateway::current();

    spawn(async move {
        let pref_res = gw.playback_preference().await;
        if store::current_episode_seq() != seq {
            return;
        }
        if let Ok(pref) = pref_res {
            let rate = pref.default_rate_hundredths.clamp(25, 400);
            store::set_rate_silent(rate);
        }

        let detail_res = gw.series_detail(series_id).await;
        let is_bookmarked = gw.is_bookmarked(series_id).await.unwrap_or(false);
        let last_watched = gw.watch_history().await.ok().and_then(|entries| {
            entries
                .into_iter()
                .find(|entry| entry.series_id == series_id)
                .map(|entry| {
                    (
                        entry.last_episode,
                        entry.position_secs,
                        entry.episode_completed,
                    )
                })
        });
        if store::current_episode_seq() != seq {
            return;
        }
        match detail_res {
            Ok(detail) => {
                let plan = resume_plan(initial_episode, last_watched, detail.total_episodes);
                if !store::set_detail_if_seq(seq, detail, is_bookmarked) {
                    return;
                }
                load_episode_internal(series_id, plan.episode, seq).await;
                if store::current_episode_seq() == seq
                    && let Some(position) = plan.position_secs
                {
                    store::seek_to(position);
                    store::mark_progress_reported(position);
                }
            }
            Err(err) => store::set_error_if_seq(seq, err.to_string()),
        }
    });
}

pub fn switch_episode(ep_num: EpisodeNumber) {
    let Some(series_id) = store::snapshot_untracked().detail.map(|d| d.summary.id) else {
        return;
    };
    let seq = store::bump_episode_seq();
    spawn(async move {
        load_episode_internal(series_id, ep_num, seq).await;
    });
}

pub fn step_episode(forward: bool) {
    let snap = store::snapshot_untracked();
    let (Some(detail), Some(current)) = (snap.detail, snap.current_episode) else {
        return;
    };
    if let Some(next_ep) = adjacent_episode(current, detail.total_episodes, forward) {
        switch_episode(next_ep);
    }
}

pub fn seek_to(position_secs: u32) {
    let snap = store::snapshot_untracked();
    let (Some(detail), Some(ep_num)) = (snap.detail, snap.current_episode) else {
        return;
    };
    let duration = store::progress_untracked().duration_secs;
    let clamped = position_secs.min(duration);
    store::seek_to(clamped);
    store::mark_progress_reported(clamped);
    report_progress_async(detail.summary.id, ep_num, clamped, duration);
}

pub fn seek_by(delta_secs: i32) {
    let snap = store::snapshot_untracked();
    let (Some(detail), Some(ep_num)) = (snap.detail, snap.current_episode) else {
        return;
    };
    let progress = store::progress_untracked();
    let next_pos = clamp_seek(progress.position_secs, delta_secs, progress.duration_secs);
    store::seek_to(next_pos);
    store::mark_progress_reported(next_pos);
    report_progress_async(detail.summary.id, ep_num, next_pos, progress.duration_secs);
}

pub fn on_playback_progress(position_secs: u32, duration_secs: u32) {
    if !store::is_playback_seq_active() {
        return;
    }
    store::update_playback_progress(position_secs, duration_secs);
    let snap = store::snapshot_untracked();
    let (Some(detail), Some(ep_num)) = (snap.detail, snap.current_episode) else {
        return;
    };
    let progress = store::progress_untracked();
    let should_report = progress
        .last_reported_secs
        .is_none_or(|prev| position_secs.abs_diff(prev) >= 5);
    if should_report {
        store::mark_progress_reported(position_secs);
        report_progress_async(
            detail.summary.id,
            ep_num,
            position_secs,
            progress.duration_secs.max(duration_secs),
        );
    }
}

pub fn on_playback_ended() {
    if !store::is_playback_seq_active() {
        return;
    }
    let snap = store::snapshot_untracked();
    let (Some(detail), Some(ep_num)) = (snap.detail, snap.current_episode) else {
        return;
    };
    let progress = store::progress_untracked();
    let final_pos = progress.duration_secs.max(progress.position_secs);
    store::update_playback_progress(final_pos, final_pos);
    store::mark_progress_reported(final_pos);
    report_progress_async(detail.summary.id, ep_num, final_pos, final_pos);

    if snap.loop_single {
        store::seek_to(0);
    } else if snap.auto_advance
        && let Some(next_ep) = adjacent_episode(ep_num, detail.total_episodes, true)
    {
        let next_seq = store::bump_episode_seq();
        spawn(async move {
            load_episode_internal(detail.summary.id, next_ep, next_seq).await;
        });
    }
}

pub fn on_playback_error(message: String) {
    if !store::is_playback_seq_active() {
        return;
    }
    let snap = store::snapshot_untracked();
    let seq = snap.episode_seq;
    if let (Some(detail), Some(ep_num), Some(source)) =
        (snap.detail, snap.current_episode, snap.source)
        && matches!(source.origin, PlaybackOrigin::OnlineStream { .. })
    {
        let gw = gateway::current();
        let key = EpisodeKey::new(detail.summary.id, ep_num);
        spawn(async move {
            match gw.refresh_online_playback(key).await {
                Ok(refreshed) => {
                    let _ = store::set_source_if_seq(seq, ep_num, refreshed);
                }
                Err(_) => {
                    if store::current_episode_seq() == seq {
                        store::set_hint(message);
                    }
                }
            }
        });
        return;
    }
    store::set_hint(message);
}

pub fn load_more_comments() {
    let snap = store::snapshot_untracked();
    let (Some(detail), Some(ep_num)) = (snap.detail, snap.current_episode) else {
        return;
    };
    let episode_seq = snap.episode_seq;
    let Some((req_seq, page)) = store::begin_comments_load_more() else {
        return;
    };
    let gw = gateway::current();
    let req = comments_request(detail.summary.id, ep_num, page);
    spawn(async move {
        match gw.list_episode_comments(req).await {
            Ok(paged) => {
                store::apply_comments_more(episode_seq, req_seq, paged.items, paged.pagination)
            }
            Err(err) => store::apply_comments_error(episode_seq, req_seq, false, err.to_string()),
        }
    });
}

pub fn toggle_bookmark() {
    let snap = store::snapshot_untracked();
    let Some(series_id) = snap.detail.map(|d| d.summary.id) else {
        return;
    };
    let currently_bookmarked = snap.bookmarked;
    let gw = gateway::current();
    spawn(async move {
        let res = if currently_bookmarked {
            gw.unbookmark_series(series_id).await
        } else {
            gw.bookmark_series(series_id).await
        };
        if res.is_ok() {
            let next = !currently_bookmarked;
            store::set_bookmarked(next);
            store::set_hint(if next {
                "已加入追剧".to_owned()
            } else {
                "已取消追剧".to_owned()
            });
        }
    });
}

pub fn adjust_rate(faster: bool) {
    let next = step_rate(store::controls_untracked().rate_hundredths, faster);
    store::set_rate(next);
}

pub fn reset_rate() {
    store::set_rate(100);
}

pub fn adjust_volume_or_episode(up: bool) {
    let snap = store::snapshot_untracked();
    match snap.mode {
        PlaybackMode::ShortDrama => step_episode(!up),
        PlaybackMode::Cinema => {
            let next = step_volume(store::controls_untracked().volume_percent, up);
            store::set_volume(next);
        }
    }
}

pub fn launch_external_player_hint() {
    let snap = store::snapshot_untracked();
    store::close_popover();
    if let Some(src) = snap.source {
        store::set_hint(format!("已准备外部播放地址：{}", src.playable_uri));
    }
}

pub fn cleanup_on_exit() {
    store::pause_and_cancel_loads();
}

pub fn pause_on_background() {
    store::set_paused(true);
}

fn report_progress_async(
    series_id: SeriesId,
    ep_num: EpisodeNumber,
    position_secs: u32,
    duration_secs: u32,
) {
    let gw = gateway::current();
    let req = ReportWatchProgressRequest {
        key: EpisodeKey::new(series_id, ep_num),
        position_secs,
        duration_secs,
    };
    spawn(async move {
        drop(gw.report_watch_progress(req).await);
    });
}

async fn load_episode_internal(series_id: SeriesId, ep_num: EpisodeNumber, seq: u64) {
    if store::current_episode_seq() != seq {
        return;
    }
    let gw = gateway::current();
    let key = EpisodeKey::new(series_id, ep_num);
    match gw.prepare_episode_playback(key).await {
        Ok(source) => {
            if !store::set_source_if_seq(seq, ep_num, source) {
                return;
            }
        }
        Err(err) => {
            store::set_error_if_seq(seq, err.to_string());
            return;
        }
    }
    let req_seq = store::begin_comments_load();
    let req = comments_request(series_id, ep_num, 1);
    match gw.list_episode_comments(req).await {
        Ok(comments) => {
            store::apply_comments_loaded(seq, req_seq, comments.items, comments.pagination);
        }
        Err(err) => store::apply_comments_error(seq, req_seq, true, err.to_string()),
    }
}

fn comments_request(
    series_id: SeriesId,
    ep_num: EpisodeNumber,
    page: u32,
) -> ListEpisodeCommentsRequest {
    ListEpisodeCommentsRequest {
        series_id,
        ep_num,
        page: PageRequest::new(
            NonZeroU32::new(page.max(1)).unwrap_or(NonZeroU32::MIN),
            NonZeroU16::new(COMMENTS_PAGE_SIZE).unwrap_or(NonZeroU16::MIN),
        ),
    }
}
