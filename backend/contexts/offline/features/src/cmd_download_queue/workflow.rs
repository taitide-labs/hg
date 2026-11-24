use std::collections::{HashMap, HashSet};

use hg_backend_contracts::catalog::StreamEndpoints;
use hg_backend_contracts::error::PortError;
use hg_contracts::error::ContractError;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId, StreamProfile};
use hg_contracts::offline::{
    ControlAllDownloadsAction, ControlDownloadAction, DownloadFailureReason, DownloadPolicy,
    DownloadTaskState, PauseReason,
};
use hg_offline_domain::policy::validate_download_policy;
use hg_offline_domain::task::{
    EnqueuePlan, SelectedDownloadStream, plan_episode_enqueue, select_download_stream, state_label,
    transition_to_cancelled, transition_to_completed, transition_to_downloading,
    transition_to_failed, transition_to_fetching, transition_to_paused, transition_to_resumed,
    transition_to_retried,
};
use reqwest::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistAsset {
    pub remote_url: String,
    pub local_name: String,
    pub is_media_segment: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewrittenPlaylist {
    pub assets: Vec<PlaylistAsset>,
    pub rewritten_m3u8: String,
    pub segment_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskStateColumns {
    pub state: &'static str,
    pub pause_reason: Option<&'static str>,
    pub failure_reason: Option<&'static str>,
    pub completed_segments: u32,
    pub total_segments: u32,
    pub downloaded_bytes: u64,
}

pub fn check_download_policy(policy: &DownloadPolicy) -> Result<(), ContractError> {
    validate_download_policy(policy).map_err(ContractError::from)
}

pub fn plan_enqueue(
    series_id: SeriesId,
    requested: &[EpisodeNumber],
    existing_present: &HashSet<EpisodeKey>,
) -> Result<EnqueuePlan, ContractError> {
    plan_episode_enqueue(series_id, requested, existing_present).map_err(ContractError::from)
}

pub fn select_stream(
    streams: &StreamEndpoints,
    preferred: StreamProfile,
) -> Result<SelectedDownloadStream, ContractError> {
    select_download_stream(streams, preferred).map_err(ContractError::from)
}

pub fn apply_control_action(
    state: DownloadTaskState,
    action: ControlDownloadAction,
) -> Result<DownloadTaskState, ContractError> {
    let next = match action {
        ControlDownloadAction::Pause => transition_to_paused(state, PauseReason::UserRequested),
        ControlDownloadAction::Resume => transition_to_resumed(state),
        ControlDownloadAction::Retry => transition_to_retried(state),
        ControlDownloadAction::Cancel => transition_to_cancelled(state),
    };
    next.map_err(ContractError::from)
}

#[must_use]
pub fn apply_batch_control_action(
    state: DownloadTaskState,
    action: ControlAllDownloadsAction,
) -> Option<DownloadTaskState> {
    match action {
        ControlAllDownloadsAction::PauseAll => match state {
            DownloadTaskState::Queued
            | DownloadTaskState::FetchingSource
            | DownloadTaskState::Downloading { .. } => {
                transition_to_paused(state, PauseReason::UserRequested).ok()
            }
            DownloadTaskState::Paused { .. }
            | DownloadTaskState::Failed { .. }
            | DownloadTaskState::Completed
            | DownloadTaskState::Cancelled => None,
        },
        ControlAllDownloadsAction::ResumeAll => match state {
            DownloadTaskState::Paused { .. } => transition_to_resumed(state).ok(),
            DownloadTaskState::Queued
            | DownloadTaskState::FetchingSource
            | DownloadTaskState::Downloading { .. }
            | DownloadTaskState::Failed { .. }
            | DownloadTaskState::Completed
            | DownloadTaskState::Cancelled => None,
        },
    }
}

pub fn begin_fetching(state: DownloadTaskState) -> Result<DownloadTaskState, ContractError> {
    transition_to_fetching(state).map_err(ContractError::from)
}

pub fn begin_or_advance_downloading(
    state: DownloadTaskState,
    completed_segments: u32,
    total_segments: u32,
    downloaded_bytes: u64,
) -> Result<DownloadTaskState, ContractError> {
    transition_to_downloading(state, completed_segments, total_segments, downloaded_bytes)
        .map_err(ContractError::from)
}

pub fn mark_failed(
    state: DownloadTaskState,
    reason: DownloadFailureReason,
) -> Result<DownloadTaskState, ContractError> {
    transition_to_failed(state, reason).map_err(ContractError::from)
}

pub fn mark_completed(state: DownloadTaskState) -> Result<DownloadTaskState, ContractError> {
    transition_to_completed(state).map_err(ContractError::from)
}

#[must_use]
pub const fn profile_to_str(profile: StreamProfile) -> &'static str {
    match profile {
        StreamProfile::Hevc => "hevc",
        StreamProfile::Main => "main",
    }
}

pub fn parse_profile(raw: &str) -> Result<StreamProfile, ContractError> {
    match raw {
        "hevc" => Ok(StreamProfile::Hevc),
        "main" => Ok(StreamProfile::Main),
        other => Err(ContractError::Internal {
            detail: format!("未知的流档位值: {other}"),
        }),
    }
}

#[must_use]
pub const fn pause_reason_to_str(reason: PauseReason) -> &'static str {
    match reason {
        PauseReason::UserRequested => "user_requested",
        PauseReason::WifiOnlyBlocked => "wifi_only_blocked",
        PauseReason::AppInterrupted => "app_interrupted",
    }
}

#[must_use]
pub const fn failure_reason_to_str(reason: DownloadFailureReason) -> &'static str {
    match reason {
        DownloadFailureReason::PreviewStreamOnly => "preview_stream_only",
        DownloadFailureReason::NetworkError => "network_error",
        DownloadFailureReason::StorageFull => "storage_full",
        DownloadFailureReason::TicketExpired => "ticket_expired",
        DownloadFailureReason::CorruptedSegment => "corrupted_segment",
    }
}

#[must_use]
pub fn state_to_columns(
    state: DownloadTaskState,
    preserved_downloaded_bytes: u64,
) -> TaskStateColumns {
    let label = state_label(state);
    match state {
        DownloadTaskState::Queued
        | DownloadTaskState::FetchingSource
        | DownloadTaskState::Completed => TaskStateColumns {
            state: label,
            pause_reason: None,
            failure_reason: None,
            completed_segments: 0,
            total_segments: 0,
            downloaded_bytes: preserved_downloaded_bytes,
        },
        DownloadTaskState::Cancelled => TaskStateColumns {
            state: label,
            pause_reason: None,
            failure_reason: None,
            completed_segments: 0,
            total_segments: 0,
            downloaded_bytes: 0,
        },
        DownloadTaskState::Downloading {
            completed_segments,
            total_segments,
            downloaded_bytes,
        } => TaskStateColumns {
            state: label,
            pause_reason: None,
            failure_reason: None,
            completed_segments,
            total_segments,
            downloaded_bytes,
        },
        DownloadTaskState::Paused {
            reason,
            completed_segments,
            total_segments,
        } => TaskStateColumns {
            state: label,
            pause_reason: Some(pause_reason_to_str(reason)),
            failure_reason: None,
            completed_segments,
            total_segments,
            downloaded_bytes: preserved_downloaded_bytes,
        },
        DownloadTaskState::Failed {
            reason,
            completed_segments,
            total_segments,
        } => TaskStateColumns {
            state: label,
            pause_reason: None,
            failure_reason: Some(failure_reason_to_str(reason)),
            completed_segments,
            total_segments,
            downloaded_bytes: preserved_downloaded_bytes,
        },
    }
}

pub fn columns_to_state(
    state: &str,
    pause_reason: Option<&str>,
    failure_reason: Option<&str>,
    completed_segments: u32,
    total_segments: u32,
    downloaded_bytes: u64,
) -> Result<DownloadTaskState, ContractError> {
    match state {
        "queued" => Ok(DownloadTaskState::Queued),
        "fetching_source" => Ok(DownloadTaskState::FetchingSource),
        "downloading" => Ok(DownloadTaskState::Downloading {
            completed_segments,
            total_segments,
            downloaded_bytes,
        }),
        "paused" => Ok(DownloadTaskState::Paused {
            reason: parse_pause_reason(pause_reason)?,
            completed_segments,
            total_segments,
        }),
        "failed" => Ok(DownloadTaskState::Failed {
            reason: parse_failure_reason(failure_reason)?,
            completed_segments,
            total_segments,
        }),
        "completed" => Ok(DownloadTaskState::Completed),
        "cancelled" => Ok(DownloadTaskState::Cancelled),
        other => Err(ContractError::Internal {
            detail: format!("未知的下载任务状态: {other}"),
        }),
    }
}

#[must_use]
pub fn classify_port_error(err: &PortError) -> DownloadFailureReason {
    match err {
        PortError::UpstreamRejected { .. } => DownloadFailureReason::TicketExpired,
        PortError::StorageFailure { .. } => DownloadFailureReason::StorageFull,
        PortError::NotFound { .. }
        | PortError::NetworkUnavailable { .. }
        | PortError::InvalidState { .. } => DownloadFailureReason::NetworkError,
    }
}

pub fn select_master_playlist_variant(
    playlist_text: &str,
    base_url: &str,
) -> Result<Option<String>, ContractError> {
    let base = parse_base_url(base_url)?;
    let mut pending_bandwidth: Option<u64> = None;
    let mut best: Option<(u64, String)> = None;

    for raw_line in playlist_text.lines() {
        let line = raw_line.trim();
        if let Some(attrs) = line.strip_prefix("#EXT-X-STREAM-INF:") {
            pending_bandwidth = Some(parse_bandwidth_attr(attrs));
            continue;
        }
        if let Some(bw) = pending_bandwidth.take()
            && !line.is_empty()
            && !line.starts_with('#')
        {
            let resolved = resolve_url(&base, line)?;
            let should_replace = best.as_ref().is_none_or(|(best_bw, _)| bw > *best_bw);
            if should_replace {
                best = Some((bw, resolved));
            }
        }
    }

    Ok(best.map(|(_, uri)| uri))
}

pub fn rewrite_media_playlist(
    playlist_text: &str,
    base_url: &str,
) -> Result<RewrittenPlaylist, ContractError> {
    let trimmed = playlist_text.trim_start_matches('\u{feff}');
    if !trimmed.lines().any(|line| line.trim() == "#EXTM3U") {
        return Err(ContractError::InvalidInput {
            message: "播放列表缺少 #EXTM3U 头".to_owned(),
        });
    }

    let base = parse_base_url(base_url)?;
    let mut rewriter = PlaylistRewriter::new(base);
    let mut output_lines = Vec::new();

    for raw_line in trimmed.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        output_lines.push(rewriter.process_line(line)?);
    }

    if rewriter.segment_count == 0 {
        return Err(ContractError::InvalidInput {
            message: "播放列表不包含任何媒体分片".to_owned(),
        });
    }

    let mut rewritten_m3u8 = output_lines.join("\n");
    rewritten_m3u8.push('\n');

    Ok(RewrittenPlaylist {
        assets: rewriter.assets,
        rewritten_m3u8,
        segment_count: rewriter.segment_count,
    })
}

struct PlaylistRewriter {
    base: Url,
    assets: Vec<PlaylistAsset>,
    seen_aux: HashMap<String, String>,
    aux_index: u32,
    segment_count: u32,
}

impl PlaylistRewriter {
    fn new(base: Url) -> Self {
        Self {
            base,
            assets: Vec::new(),
            seen_aux: HashMap::new(),
            aux_index: 0,
            segment_count: 0,
        }
    }

    fn process_line(&mut self, line: &str) -> Result<String, ContractError> {
        if line.starts_with("#EXT-X-KEY:") {
            return self.rewrite_tag_uri(line, "key", "key");
        }
        if line.starts_with("#EXT-X-MAP:") {
            return self.rewrite_tag_uri(line, "map", "mp4");
        }
        if line.starts_with('#') {
            return Ok(line.to_owned());
        }

        let remote_url = resolve_url(&self.base, line)?;
        let ext = infer_extension(&remote_url, "ts");
        let local_name = format!("seg_{:05}.{ext}", self.segment_count);
        self.segment_count = self.segment_count.saturating_add(1);
        self.assets.push(PlaylistAsset {
            remote_url,
            local_name: local_name.clone(),
            is_media_segment: true,
        });
        Ok(local_name)
    }

    fn rewrite_tag_uri(
        &mut self,
        line: &str,
        prefix: &str,
        default_ext: &str,
    ) -> Result<String, ContractError> {
        let Some((before, uri, after)) = split_quoted_uri(line) else {
            return Ok(line.to_owned());
        };
        let remote_url = resolve_url(&self.base, uri)?;
        let local_name = if let Some(existing) = self.seen_aux.get(&remote_url) {
            existing.clone()
        } else {
            let ext = infer_extension(&remote_url, default_ext);
            let name = format!("{prefix}_{:05}.{ext}", self.aux_index);
            self.aux_index = self.aux_index.saturating_add(1);
            self.seen_aux.insert(remote_url.clone(), name.clone());
            self.assets.push(PlaylistAsset {
                remote_url,
                local_name: name.clone(),
                is_media_segment: false,
            });
            name
        };
        Ok(format!("{before}{local_name}{after}"))
    }
}

fn split_quoted_uri(line: &str) -> Option<(&str, &str, &str)> {
    let marker = "URI=\"";
    let start = line.find(marker)?;
    let prefix_end = start + marker.len();
    let rest = line.get(prefix_end..)?;
    let close = rest.find('"')?;
    let uri = rest.get(..close)?;
    let before = line.get(..prefix_end)?;
    let after = rest.get(close..)?;
    Some((before, uri, after))
}

fn parse_bandwidth_attr(attrs: &str) -> u64 {
    for part in attrs.split(',') {
        let trimmed = part.trim();
        if let Some(val) = trimmed.strip_prefix("BANDWIDTH=")
            && let Ok(parsed) = val.parse::<u64>()
        {
            return parsed;
        }
    }
    0
}

fn parse_base_url(base_url: &str) -> Result<Url, ContractError> {
    Url::parse(base_url).map_err(|err| ContractError::InvalidInput {
        message: format!("播放列表 URL 非法: {err}"),
    })
}

fn resolve_url(base: &Url, raw_ref: &str) -> Result<String, ContractError> {
    base.join(raw_ref)
        .map(Url::into)
        .map_err(|err| ContractError::InvalidInput {
            message: format!("分片 URL 解析失败: {err}"),
        })
}

fn infer_extension(remote_url: &str, default_ext: &str) -> String {
    let candidate = Url::parse(remote_url)
        .ok()
        .and_then(|parsed| {
            parsed
                .path_segments()
                .and_then(|mut segments| segments.next_back().map(str::to_owned))
        })
        .and_then(|file| {
            file.rsplit_once('.')
                .map(|(_, ext)| ext.to_ascii_lowercase())
        });

    match candidate.as_deref() {
        Some("ts" | "m4s" | "mp4" | "aac" | "key" | "vtt") => {
            candidate.unwrap_or_else(|| default_ext.to_owned())
        }
        _ => default_ext.to_owned(),
    }
}

fn parse_pause_reason(raw: Option<&str>) -> Result<PauseReason, ContractError> {
    match raw {
        Some("user_requested") => Ok(PauseReason::UserRequested),
        Some("wifi_only_blocked") => Ok(PauseReason::WifiOnlyBlocked),
        Some("app_interrupted") => Ok(PauseReason::AppInterrupted),
        Some(other) => Err(ContractError::Internal {
            detail: format!("未知的暂停原因: {other}"),
        }),
        None => Err(ContractError::Internal {
            detail: "暂停状态缺少 pause_reason 字段".to_owned(),
        }),
    }
}

fn parse_failure_reason(raw: Option<&str>) -> Result<DownloadFailureReason, ContractError> {
    match raw {
        Some("preview_stream_only") => Ok(DownloadFailureReason::PreviewStreamOnly),
        Some("network_error") => Ok(DownloadFailureReason::NetworkError),
        Some("storage_full") => Ok(DownloadFailureReason::StorageFull),
        Some("ticket_expired") => Ok(DownloadFailureReason::TicketExpired),
        Some("corrupted_segment") => Ok(DownloadFailureReason::CorruptedSegment),
        Some(other) => Err(ContractError::Internal {
            detail: format!("未知的失败原因: {other}"),
        }),
        None => Err(ContractError::Internal {
            detail: "失败状态缺少 failure_reason 字段".to_owned(),
        }),
    }
}
