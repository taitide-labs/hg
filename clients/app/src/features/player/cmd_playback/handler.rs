use std::num::{NonZeroU16, NonZeroU32};

use hg_contracts::catalog::ListEpisodeCommentsRequest;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, PageRequest, SeriesId};
use hg_contracts::viewing::{PlaybackMode, ReportWatchProgressRequest};
use nsh_core::prelude::*;

use super::super::store;
use super::command::{adjacent_episode, clamp_seek, step_rate, step_volume};
use crate::gateway;

pub fn open_series(
    series_id: SeriesId,
    initial_episode: Option<EpisodeNumber>,
    default_mode: PlaybackMode,
) {
    store::reset_for_open(default_mode);
    let gw = gateway::current();
    let target_ep = initial_episode
        .unwrap_or_else(|| EpisodeNumber::new(NonZeroU32::new(1).unwrap_or(NonZeroU32::MIN)));

    spawn(async move {
        let detail_res = gw.series_detail(series_id).await;
        let is_bookmarked = gw.is_bookmarked(series_id).await.unwrap_or(false);
        match detail_res {
            Ok(detail) => {
                store::set_detail(detail, is_bookmarked);
                load_episode_internal(series_id, target_ep).await;
            }
            Err(err) => store::set_error(err.to_string()),
        }
    });
}

pub fn switch_episode(ep_num: EpisodeNumber) {
    let Some(series_id) = store::snapshot().detail.map(|d| d.summary.id) else {
        return;
    };
    spawn(async move {
        load_episode_internal(series_id, ep_num).await;
    });
}

pub fn step_episode(forward: bool) {
    let snap = store::snapshot();
    let (Some(detail), Some(current)) = (snap.detail, snap.current_episode) else {
        return;
    };
    if let Some(next_ep) = adjacent_episode(current, detail.total_episodes, forward) {
        switch_episode(next_ep);
    }
}

pub fn seek_by(delta_secs: i32) {
    let snap = store::snapshot();
    let (Some(detail), Some(ep_num)) = (snap.detail, snap.current_episode) else {
        return;
    };
    let next_pos = clamp_seek(snap.position_secs, delta_secs, snap.duration_secs);
    store::set_position(next_pos);
    report_progress_async(
        detail.summary.id,
        ep_num,
        next_pos,
        snap.duration_secs,
        snap.auto_advance,
        snap.loop_single,
    );
}

pub fn toggle_bookmark() {
    let snap = store::snapshot();
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
    let next = step_rate(store::snapshot().rate_hundredths, faster);
    store::set_rate(next);
}

pub fn reset_rate() {
    store::set_rate(100);
}

pub fn adjust_volume_or_episode(up: bool) {
    let snap = store::snapshot();
    match snap.mode {
        PlaybackMode::ShortDrama => step_episode(!up),
        PlaybackMode::Cinema => {
            let next = step_volume(snap.volume_percent, up);
            store::set_volume(next);
        }
    }
}

pub fn launch_external_player_hint() {
    let snap = store::snapshot();
    store::close_popover();
    if let Some(src) = snap.source {
        store::set_hint(format!("已准备外部播放地址：{}", src.playable_uri));
    }
}

fn report_progress_async(
    series_id: SeriesId,
    ep_num: EpisodeNumber,
    position_secs: u32,
    duration_secs: u32,
    auto_advance: bool,
    loop_single: bool,
) {
    let gw = gateway::current();
    let req = ReportWatchProgressRequest {
        key: EpisodeKey::new(series_id, ep_num),
        position_secs,
        duration_secs,
    };
    spawn(async move {
        if let Ok(result) = gw.report_watch_progress(req).await
            && result.progress.completed
        {
            if loop_single {
                store::set_position(0);
            } else if auto_advance && let Some(next_ep) = result.next_episode {
                load_episode_internal(series_id, next_ep).await;
            }
        }
    });
}

async fn load_episode_internal(series_id: SeriesId, ep_num: EpisodeNumber) {
    let gw = gateway::current();
    let key = EpisodeKey::new(series_id, ep_num);
    match gw.prepare_episode_playback(key).await {
        Ok(source) => store::set_source(ep_num, source),
        Err(err) => {
            store::set_error(err.to_string());
            return;
        }
    }
    let req = ListEpisodeCommentsRequest {
        series_id,
        ep_num,
        page: PageRequest::new(
            NonZeroU32::new(1).unwrap_or(NonZeroU32::MIN),
            NonZeroU16::new(20).unwrap_or(NonZeroU16::MIN),
        ),
    };
    if let Ok(comments) = gw.list_episode_comments(req).await {
        store::set_comments(comments.items);
    }
}
