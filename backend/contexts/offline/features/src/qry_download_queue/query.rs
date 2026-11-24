use hg_contracts::error::ContractError;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId, StreamProfile};
use hg_contracts::offline::{
    DownloadFailureReason, DownloadPolicy, DownloadTaskState, DownloadTaskView, PauseReason,
};
use sqlx::{FromRow, SqlitePool};

pub const FEATURE_NAME: &str = "offline.download_queue";

#[derive(Debug, FromRow)]
pub(crate) struct DownloadPolicyRow {
    pub(crate) preferred_profile: String,
    pub(crate) wifi_only: i64,
    pub(crate) max_concurrent_tasks: i64,
    pub(crate) storage_directory: Option<String>,
}

#[derive(Debug, FromRow)]
pub(crate) struct DownloadTaskRow {
    pub(crate) series_id: i64,
    pub(crate) ep_num: i64,
    pub(crate) series_title: String,
    pub(crate) episode_title: String,
    pub(crate) cover_url: String,
    pub(crate) profile: String,
    pub(crate) state: String,
    pub(crate) pause_reason: Option<String>,
    pub(crate) failure_reason: Option<String>,
    pub(crate) completed_segments: i64,
    pub(crate) total_segments: i64,
    pub(crate) downloaded_bytes: i64,
}

pub async fn fetch_download_policy(pool: &SqlitePool) -> Result<DownloadPolicy, ContractError> {
    let row = sqlx::query_as::<_, DownloadPolicyRow>(
        "SELECT
            preferred_profile,
            wifi_only,
            max_concurrent_tasks,
            storage_directory
        FROM offline_download_policy
        WHERE id = 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("查询离线下载策略失败: {err}"),
    })?
    .ok_or_else(|| ContractError::NotFound {
        resource: "offline_download_policy".to_owned(),
    })?;

    row.try_into()
}

pub async fn fetch_download_tasks(
    pool: &SqlitePool,
) -> Result<Vec<DownloadTaskView>, ContractError> {
    let rows = sqlx::query_as::<_, DownloadTaskRow>(
        "SELECT
            series_id,
            ep_num,
            series_title,
            episode_title,
            cover_url,
            profile,
            state,
            pause_reason,
            failure_reason,
            completed_segments,
            total_segments,
            downloaded_bytes
        FROM offline_download_tasks
        ORDER BY created_at_epoch_secs ASC, rowid ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("查询离线下载队列失败: {err}"),
    })?;

    rows.into_iter().map(TryInto::try_into).collect()
}

impl TryFrom<DownloadPolicyRow> for DownloadPolicy {
    type Error = ContractError;

    fn try_from(row: DownloadPolicyRow) -> Result<Self, Self::Error> {
        let preferred_profile = parse_stream_profile(&row.preferred_profile)?;
        let max_concurrent_tasks =
            u8::try_from(row.max_concurrent_tasks).map_err(|err| ContractError::Internal {
                detail: format!("并发任务上限字段非法: {err}"),
            })?;

        Ok(Self {
            preferred_profile,
            wifi_only: row.wifi_only != 0,
            max_concurrent_tasks,
            storage_directory: row.storage_directory,
        })
    }
}

impl TryFrom<DownloadTaskRow> for DownloadTaskView {
    type Error = ContractError;

    fn try_from(row: DownloadTaskRow) -> Result<Self, Self::Error> {
        let series_id_raw =
            u64::try_from(row.series_id).map_err(|err| ContractError::Internal {
                detail: format!("下载任务剧目标识非法: {err}"),
            })?;
        let series_id =
            SeriesId::try_from(series_id_raw).map_err(|detail| ContractError::Internal {
                detail: format!("下载任务剧目标识无效: {detail}"),
            })?;
        let ep_raw = u32::try_from(row.ep_num).map_err(|err| ContractError::Internal {
            detail: format!("下载任务分集序号非法: {err}"),
        })?;
        let ep_num = EpisodeNumber::try_from(ep_raw).map_err(|detail| ContractError::Internal {
            detail: format!("下载任务分集序号无效: {detail}"),
        })?;
        let profile = parse_stream_profile(&row.profile)?;
        let state = parse_task_state(
            &row.state,
            row.pause_reason.as_deref(),
            row.failure_reason.as_deref(),
            row.completed_segments,
            row.total_segments,
            row.downloaded_bytes,
        )?;

        Ok(Self {
            key: EpisodeKey::new(series_id, ep_num),
            series_title: row.series_title,
            episode_title: row.episode_title,
            cover_url: row.cover_url,
            profile,
            state,
        })
    }
}

fn parse_stream_profile(raw: &str) -> Result<StreamProfile, ContractError> {
    match raw {
        "hevc" => Ok(StreamProfile::Hevc),
        "main" => Ok(StreamProfile::Main),
        other => Err(ContractError::Internal {
            detail: format!("未知的流档位值: {other}"),
        }),
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

fn parse_task_state(
    state: &str,
    pause_reason: Option<&str>,
    failure_reason: Option<&str>,
    completed_segments_raw: i64,
    total_segments_raw: i64,
    downloaded_bytes_raw: i64,
) -> Result<DownloadTaskState, ContractError> {
    let completed_segments =
        u32::try_from(completed_segments_raw).map_err(|err| ContractError::Internal {
            detail: format!("已完成分片数字段非法: {err}"),
        })?;
    let total_segments =
        u32::try_from(total_segments_raw).map_err(|err| ContractError::Internal {
            detail: format!("总分片数字段非法: {err}"),
        })?;
    let downloaded_bytes =
        u64::try_from(downloaded_bytes_raw).map_err(|err| ContractError::Internal {
            detail: format!("已下载字节数字段非法: {err}"),
        })?;

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
