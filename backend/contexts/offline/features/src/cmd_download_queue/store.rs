use std::collections::HashSet;

use hg_backend_contracts::catalog::{EpisodeSnapshot, SeriesSnapshot};
use hg_contracts::error::ContractError;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId, StreamProfile};
use hg_contracts::offline::{
    ControlAllDownloadsAction, ControlDownloadAction, DownloadFailureReason, DownloadPolicy,
    DownloadTaskState,
};
use sqlx::{FromRow, SqlitePool};

use super::workflow::{
    apply_batch_control_action, apply_control_action, begin_fetching, columns_to_state,
    mark_completed, mark_failed, parse_profile, pause_for_wifi_only, profile_to_str,
    resume_from_wifi_only, state_to_columns,
};

#[derive(Debug, Clone)]
pub(crate) struct ClaimedTask {
    pub(crate) key: EpisodeKey,
    pub(crate) previous_profile: StreamProfile,
    pub(crate) series_title: String,
    pub(crate) episode_title: String,
    pub(crate) cover_url: String,
}

#[derive(Debug, Clone)]
pub(crate) struct CompletedEpisodeCommit {
    pub(crate) key: EpisodeKey,
    pub(crate) series_title: String,
    pub(crate) cover_local_path: String,
    pub(crate) total_episodes: u32,
    pub(crate) episode_title: String,
    pub(crate) duration_secs: u32,
    pub(crate) profile: StreamProfile,
    pub(crate) playlist_rel_path: String,
    pub(crate) size_bytes: u64,
    pub(crate) now_epoch_secs: u64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PolicyPrefs {
    pub(crate) preferred_profile: StreamProfile,
    pub(crate) wifi_only: bool,
    pub(crate) max_concurrent_tasks: u8,
}

#[derive(Debug, FromRow)]
struct PolicyPrefsRow {
    preferred_profile: String,
    wifi_only: i64,
    max_concurrent_tasks: i64,
}

#[derive(Debug, FromRow)]
struct EpisodeNumRow {
    ep_num: i64,
}

#[derive(Debug, FromRow)]
struct TaskStateRow {
    series_id: i64,
    ep_num: i64,
    series_title: String,
    episode_title: String,
    cover_url: String,
    profile: String,
    state: String,
    pause_reason: Option<String>,
    failure_reason: Option<String>,
    completed_segments: i64,
    total_segments: i64,
    downloaded_bytes: i64,
}

pub(crate) async fn load_policy_prefs(pool: &SqlitePool) -> Result<PolicyPrefs, ContractError> {
    let row = sqlx::query_as::<_, PolicyPrefsRow>(
        "SELECT preferred_profile, wifi_only, max_concurrent_tasks
         FROM offline_download_policy
         WHERE id = 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("读取下载策略失败: {err}"),
    })?;

    match row {
        Some(prefs) => {
            let preferred_profile = parse_profile(&prefs.preferred_profile)?;
            let max_concurrent_tasks = u8::try_from(prefs.max_concurrent_tasks).map_err(|err| {
                ContractError::Internal {
                    detail: format!("并发任务上限字段非法: {err}"),
                }
            })?;
            Ok(PolicyPrefs {
                preferred_profile,
                wifi_only: prefs.wifi_only != 0,
                max_concurrent_tasks,
            })
        }
        None => Ok(PolicyPrefs {
            preferred_profile: StreamProfile::Hevc,
            wifi_only: true,
            max_concurrent_tasks: 2,
        }),
    }
}

pub(crate) async fn pause_tasks_for_wifi_only(
    pool: &SqlitePool,
    now_epoch_secs: u64,
) -> Result<(), ContractError> {
    let now = to_i64_u64(now_epoch_secs, "时间戳")?;
    let mut tx = pool.begin().await.map_err(|err| ContractError::Internal {
        detail: format!("开启仅 Wi-Fi 挂起事务失败: {err}"),
    })?;

    let rows = sqlx::query_as::<_, TaskStateRow>(
        "SELECT
            series_id, ep_num, series_title, episode_title, cover_url, profile,
            state, pause_reason, failure_reason, completed_segments, total_segments,
            downloaded_bytes
         FROM offline_download_tasks
         WHERE state IN ('queued', 'fetching_source', 'downloading')",
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("查询待挂起下载任务失败: {err}"),
    })?;

    for row in rows {
        let key = decode_row_key(&row)?;
        let (current_state, downloaded_bytes) = decode_row_state(&row)?;
        if let Some(next_state) = pause_for_wifi_only(current_state) {
            let cols = state_to_columns(next_state, downloaded_bytes);
            update_task_columns(&mut tx, key, &cols, None, now).await?;
        }
    }

    tx.commit().await.map_err(|err| ContractError::Internal {
        detail: format!("提交仅 Wi-Fi 挂起事务失败: {err}"),
    })
}

pub(crate) async fn resume_wifi_blocked_tasks(
    pool: &SqlitePool,
    now_epoch_secs: u64,
) -> Result<(), ContractError> {
    let now = to_i64_u64(now_epoch_secs, "时间戳")?;
    let mut tx = pool.begin().await.map_err(|err| ContractError::Internal {
        detail: format!("开启恢复 Wi-Fi 挂起任务事务失败: {err}"),
    })?;

    let rows = sqlx::query_as::<_, TaskStateRow>(
        "SELECT
            series_id, ep_num, series_title, episode_title, cover_url, profile,
            state, pause_reason, failure_reason, completed_segments, total_segments,
            downloaded_bytes
         FROM offline_download_tasks
         WHERE state = 'paused' AND pause_reason = 'wifi_only_blocked'",
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("查询 Wi-Fi 挂起下载任务失败: {err}"),
    })?;

    for row in rows {
        let key = decode_row_key(&row)?;
        let (current_state, downloaded_bytes) = decode_row_state(&row)?;
        if let Some(next_state) = resume_from_wifi_only(current_state) {
            let cols = state_to_columns(next_state, downloaded_bytes);
            update_task_columns(&mut tx, key, &cols, None, now).await?;
        }
    }

    tx.commit().await.map_err(|err| ContractError::Internal {
        detail: format!("提交恢复 Wi-Fi 挂起任务事务失败: {err}"),
    })
}

pub(crate) async fn upsert_policy(
    pool: &SqlitePool,
    policy: &DownloadPolicy,
) -> Result<(), ContractError> {
    let mut tx = pool.begin().await.map_err(|err| ContractError::Internal {
        detail: format!("开启更新下载策略事务失败: {err}"),
    })?;

    sqlx::query(
        "INSERT INTO offline_download_policy (
            id, preferred_profile, wifi_only, max_concurrent_tasks, storage_directory
        ) VALUES (1, ?1, ?2, ?3, ?4)
        ON CONFLICT(id) DO UPDATE SET
            preferred_profile = excluded.preferred_profile,
            wifi_only = excluded.wifi_only,
            max_concurrent_tasks = excluded.max_concurrent_tasks,
            storage_directory = excluded.storage_directory",
    )
    .bind(profile_to_str(policy.preferred_profile))
    .bind(i64::from(policy.wifi_only))
    .bind(i64::from(policy.max_concurrent_tasks))
    .bind(&policy.storage_directory)
    .execute(&mut *tx)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("保存下载策略失败: {err}"),
    })?;

    tx.commit().await.map_err(|err| ContractError::Internal {
        detail: format!("提交下载策略事务失败: {err}"),
    })
}

pub(crate) async fn load_existing_episode_keys(
    pool: &SqlitePool,
    series_id: SeriesId,
) -> Result<HashSet<EpisodeKey>, ContractError> {
    let sid = to_i64_u64(series_id.get(), "剧目标识")?;
    let mut keys = HashSet::new();

    let committed_rows = sqlx::query_as::<_, EpisodeNumRow>(
        "SELECT ep_num FROM offline_episodes WHERE series_id = ?1",
    )
    .bind(sid)
    .fetch_all(pool)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("查询已下载分集失败: {err}"),
    })?;

    for row in committed_rows {
        keys.insert(EpisodeKey::new(series_id, parse_ep_num(row.ep_num)?));
    }

    let active_rows = sqlx::query_as::<_, EpisodeNumRow>(
        "SELECT ep_num
         FROM offline_download_tasks
         WHERE series_id = ?1
           AND state NOT IN ('completed', 'cancelled')",
    )
    .bind(sid)
    .fetch_all(pool)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("查询进行中下载任务失败: {err}"),
    })?;

    for row in active_rows {
        keys.insert(EpisodeKey::new(series_id, parse_ep_num(row.ep_num)?));
    }

    Ok(keys)
}

pub(crate) async fn insert_queued_tasks(
    pool: &SqlitePool,
    series: &SeriesSnapshot,
    episodes: &[EpisodeSnapshot],
    preferred_profile: StreamProfile,
    now_epoch_secs: u64,
) -> Result<(), ContractError> {
    let sid = to_i64_u64(series.series_id.get(), "剧目标识")?;
    let now = to_i64_u64(now_epoch_secs, "时间戳")?;
    let profile_str = profile_to_str(preferred_profile);

    let mut tx = pool.begin().await.map_err(|err| ContractError::Internal {
        detail: format!("开启入队事务失败: {err}"),
    })?;

    for ep in episodes {
        let ep_num = i64::from(ep.key.ep_num.get());
        sqlx::query(
            "DELETE FROM offline_download_tasks
             WHERE series_id = ?1
               AND ep_num = ?2
               AND state IN ('completed', 'cancelled')",
        )
        .bind(sid)
        .bind(ep_num)
        .execute(&mut *tx)
        .await
        .map_err(|err| ContractError::Internal {
            detail: format!("清理旧下载任务记录失败: {err}"),
        })?;

        let staging_rel_dir = format!("s{}_e{}", series.series_id, ep.key.ep_num);
        sqlx::query(
            "INSERT INTO offline_download_tasks (
                series_id, ep_num, series_title, episode_title, cover_url, profile,
                state, pause_reason, failure_reason, completed_segments, total_segments,
                downloaded_bytes, staging_rel_dir, created_at_epoch_secs, updated_at_epoch_secs
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'queued', NULL, NULL, 0, 0, 0, ?7, ?8, ?8)",
        )
        .bind(sid)
        .bind(ep_num)
        .bind(&series.title)
        .bind(&ep.title)
        .bind(&series.cover_url)
        .bind(profile_str)
        .bind(&staging_rel_dir)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|err| ContractError::Internal {
            detail: format!("写入下载任务失败: {err}"),
        })?;
    }

    tx.commit().await.map_err(|err| ContractError::Internal {
        detail: format!("提交入队事务失败: {err}"),
    })
}

pub(crate) async fn transition_single_task(
    pool: &SqlitePool,
    key: EpisodeKey,
    action: ControlDownloadAction,
    now_epoch_secs: u64,
) -> Result<(), ContractError> {
    let sid = to_i64_u64(key.series_id.get(), "剧目标识")?;
    let ep = i64::from(key.ep_num.get());
    let now = to_i64_u64(now_epoch_secs, "时间戳")?;

    let mut tx = pool.begin().await.map_err(|err| ContractError::Internal {
        detail: format!("开启任务控制事务失败: {err}"),
    })?;

    let row = sqlx::query_as::<_, TaskStateRow>(
        "SELECT
            series_id, ep_num, series_title, episode_title, cover_url, profile,
            state, pause_reason, failure_reason, completed_segments, total_segments,
            downloaded_bytes
         FROM offline_download_tasks
         WHERE series_id = ?1 AND ep_num = ?2",
    )
    .bind(sid)
    .bind(ep)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("查询下载任务失败: {err}"),
    })?
    .ok_or_else(|| ContractError::NotFound {
        resource: format!("download:{}:{}", key.series_id, key.ep_num),
    })?;

    let (current_state, downloaded_bytes) = decode_row_state(&row)?;
    let next_state = apply_control_action(current_state, action)?;
    let cols = state_to_columns(next_state, downloaded_bytes);

    update_task_columns(&mut tx, key, &cols, None, now).await?;

    tx.commit().await.map_err(|err| ContractError::Internal {
        detail: format!("提交任务控制事务失败: {err}"),
    })
}

pub(crate) async fn transition_all_tasks(
    pool: &SqlitePool,
    action: ControlAllDownloadsAction,
    now_epoch_secs: u64,
) -> Result<(), ContractError> {
    let now = to_i64_u64(now_epoch_secs, "时间戳")?;
    let mut tx = pool.begin().await.map_err(|err| ContractError::Internal {
        detail: format!("开启批量任务控制事务失败: {err}"),
    })?;

    let rows = sqlx::query_as::<_, TaskStateRow>(
        "SELECT
            series_id, ep_num, series_title, episode_title, cover_url, profile,
            state, pause_reason, failure_reason, completed_segments, total_segments,
            downloaded_bytes
         FROM offline_download_tasks",
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("查询下载任务列表失败: {err}"),
    })?;

    for row in rows {
        let key = decode_row_key(&row)?;
        let (current_state, downloaded_bytes) = decode_row_state(&row)?;
        if let Some(next_state) = apply_batch_control_action(current_state, action) {
            let cols = state_to_columns(next_state, downloaded_bytes);
            update_task_columns(&mut tx, key, &cols, None, now).await?;
        }
    }

    tx.commit().await.map_err(|err| ContractError::Internal {
        detail: format!("提交批量任务控制事务失败: {err}"),
    })
}

pub(crate) async fn claim_next_queued_task(
    pool: &SqlitePool,
    max_concurrent_tasks: u8,
    now_epoch_secs: u64,
) -> Result<Option<ClaimedTask>, ContractError> {
    let now = to_i64_u64(now_epoch_secs, "时间戳")?;
    let mut tx = pool.begin().await.map_err(|err| ContractError::Internal {
        detail: format!("开启认领下载任务事务失败: {err}"),
    })?;

    let active_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM offline_download_tasks
         WHERE state IN ('fetching_source', 'downloading')",
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("统计活跃下载任务数失败: {err}"),
    })?;

    if active_count.0 >= i64::from(max_concurrent_tasks) {
        tx.commit().await.map_err(|err| ContractError::Internal {
            detail: format!("提交认领检查事务失败: {err}"),
        })?;
        return Ok(None);
    }

    let next_row = sqlx::query_as::<_, TaskStateRow>(
        "SELECT
            series_id, ep_num, series_title, episode_title, cover_url, profile,
            state, pause_reason, failure_reason, completed_segments, total_segments,
            downloaded_bytes
         FROM offline_download_tasks
         WHERE state = 'queued'
         ORDER BY created_at_epoch_secs ASC, rowid ASC
         LIMIT 1",
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("查询下一个排队任务失败: {err}"),
    })?;

    let Some(row) = next_row else {
        tx.commit().await.map_err(|err| ContractError::Internal {
            detail: format!("提交空队检查事务失败: {err}"),
        })?;
        return Ok(None);
    };

    let key = decode_row_key(&row)?;
    let previous_profile = parse_profile(&row.profile)?;
    let (current_state, downloaded_bytes) = decode_row_state(&row)?;
    let fetching_state = begin_fetching(current_state)?;
    let cols = state_to_columns(fetching_state, downloaded_bytes);
    update_task_columns(&mut tx, key, &cols, None, now).await?;

    tx.commit().await.map_err(|err| ContractError::Internal {
        detail: format!("提交认领任务事务失败: {err}"),
    })?;

    Ok(Some(ClaimedTask {
        key,
        previous_profile,
        series_title: row.series_title,
        episode_title: row.episode_title,
        cover_url: row.cover_url,
    }))
}

pub(crate) async fn read_task_state(
    pool: &SqlitePool,
    key: EpisodeKey,
) -> Result<Option<DownloadTaskState>, ContractError> {
    let sid = to_i64_u64(key.series_id.get(), "剧目标识")?;
    let ep = i64::from(key.ep_num.get());
    let row = sqlx::query_as::<_, TaskStateRow>(
        "SELECT
            series_id, ep_num, series_title, episode_title, cover_url, profile,
            state, pause_reason, failure_reason, completed_segments, total_segments,
            downloaded_bytes
         FROM offline_download_tasks
         WHERE series_id = ?1 AND ep_num = ?2",
    )
    .bind(sid)
    .bind(ep)
    .fetch_optional(pool)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("读取下载任务状态失败: {err}"),
    })?;

    row.map(|r| decode_row_state(&r).map(|(s, _)| s))
        .transpose()
}

pub(crate) async fn update_task_progress(
    pool: &SqlitePool,
    key: EpisodeKey,
    profile: StreamProfile,
    state: DownloadTaskState,
    now_epoch_secs: u64,
) -> Result<(), ContractError> {
    let now = to_i64_u64(now_epoch_secs, "时间戳")?;
    let cols = state_to_columns(state, 0);
    let mut tx = pool.begin().await.map_err(|err| ContractError::Internal {
        detail: format!("开启更新进度事务失败: {err}"),
    })?;
    update_task_columns(&mut tx, key, &cols, Some(profile), now).await?;
    tx.commit().await.map_err(|err| ContractError::Internal {
        detail: format!("提交更新进度事务失败: {err}"),
    })
}

pub(crate) async fn fail_task(
    pool: &SqlitePool,
    key: EpisodeKey,
    current_state: DownloadTaskState,
    reason: DownloadFailureReason,
    preserved_bytes: u64,
    now_epoch_secs: u64,
) -> Result<(), ContractError> {
    let now = to_i64_u64(now_epoch_secs, "时间戳")?;
    let next_state = mark_failed(current_state, reason)?;
    let cols = state_to_columns(next_state, preserved_bytes);
    let mut tx = pool.begin().await.map_err(|err| ContractError::Internal {
        detail: format!("开启失败记录事务失败: {err}"),
    })?;
    update_task_columns(&mut tx, key, &cols, None, now).await?;
    tx.commit().await.map_err(|err| ContractError::Internal {
        detail: format!("提交失败记录事务失败: {err}"),
    })
}

pub(crate) async fn commit_completed_episode(
    pool: &SqlitePool,
    commit: &CompletedEpisodeCommit,
) -> Result<bool, ContractError> {
    let sid = to_i64_u64(commit.key.series_id.get(), "剧目标识")?;
    let ep = i64::from(commit.key.ep_num.get());
    let now = to_i64_u64(commit.now_epoch_secs, "时间戳")?;
    let size_bytes = to_i64_u64(commit.size_bytes, "分集体积")?;

    let mut tx = pool.begin().await.map_err(|err| ContractError::Internal {
        detail: format!("开启完成提交事务失败: {err}"),
    })?;

    let row = sqlx::query_as::<_, TaskStateRow>(
        "SELECT
            series_id, ep_num, series_title, episode_title, cover_url, profile,
            state, pause_reason, failure_reason, completed_segments, total_segments,
            downloaded_bytes
         FROM offline_download_tasks
         WHERE series_id = ?1 AND ep_num = ?2",
    )
    .bind(sid)
    .bind(ep)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("读取待提交任务状态失败: {err}"),
    })?;

    let Some(row) = row else {
        tx.commit().await.map_err(|err| ContractError::Internal {
            detail: format!("提交检查事务失败: {err}"),
        })?;
        return Ok(false);
    };

    let (current_state, _) = decode_row_state(&row)?;
    if !matches!(current_state, DownloadTaskState::Downloading { .. }) {
        tx.commit().await.map_err(|err| ContractError::Internal {
            detail: format!("提交非下载态检查事务失败: {err}"),
        })?;
        return Ok(false);
    }

    sqlx::query(
        "INSERT INTO offline_series (
            series_id, title, cover_local_path, total_episodes, updated_at_epoch_secs
        ) VALUES (?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(series_id) DO UPDATE SET
            title = excluded.title,
            cover_local_path = excluded.cover_local_path,
            total_episodes = excluded.total_episodes,
            updated_at_epoch_secs = excluded.updated_at_epoch_secs",
    )
    .bind(sid)
    .bind(&commit.series_title)
    .bind(&commit.cover_local_path)
    .bind(i64::from(commit.total_episodes))
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("持久化离线剧目失败: {err}"),
    })?;

    let profile_str = profile_to_str(commit.profile);
    sqlx::query(
        "INSERT INTO offline_episodes (
            series_id, ep_num, title, duration_secs, profile,
            playlist_rel_path, size_bytes, downloaded_at_epoch_secs
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ON CONFLICT(series_id, ep_num) DO UPDATE SET
            title = excluded.title,
            duration_secs = excluded.duration_secs,
            profile = excluded.profile,
            playlist_rel_path = excluded.playlist_rel_path,
            size_bytes = excluded.size_bytes,
            downloaded_at_epoch_secs = excluded.downloaded_at_epoch_secs",
    )
    .bind(sid)
    .bind(ep)
    .bind(&commit.episode_title)
    .bind(i64::from(commit.duration_secs))
    .bind(profile_str)
    .bind(&commit.playlist_rel_path)
    .bind(size_bytes)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("持久化离线分集失败: {err}"),
    })?;

    let completed_state = mark_completed(current_state)?;
    let cols = state_to_columns(completed_state, commit.size_bytes);
    update_task_columns(&mut tx, commit.key, &cols, Some(commit.profile), now).await?;

    tx.commit().await.map_err(|err| ContractError::Internal {
        detail: format!("提交离线资产事务失败: {err}"),
    })?;

    Ok(true)
}

async fn update_task_columns(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    key: EpisodeKey,
    cols: &super::workflow::TaskStateColumns,
    profile: Option<StreamProfile>,
    now: i64,
) -> Result<(), ContractError> {
    let sid = to_i64_u64(key.series_id.get(), "剧目标识")?;
    let ep = i64::from(key.ep_num.get());
    let bytes = to_i64_u64(cols.downloaded_bytes, "已下载字节数")?;

    if let Some(selected_profile) = profile {
        sqlx::query(
            "UPDATE offline_download_tasks
             SET profile = ?3,
                 state = ?4,
                 pause_reason = ?5,
                 failure_reason = ?6,
                 completed_segments = ?7,
                 total_segments = ?8,
                 downloaded_bytes = ?9,
                 updated_at_epoch_secs = ?10
             WHERE series_id = ?1 AND ep_num = ?2",
        )
        .bind(sid)
        .bind(ep)
        .bind(profile_to_str(selected_profile))
        .bind(cols.state)
        .bind(cols.pause_reason)
        .bind(cols.failure_reason)
        .bind(i64::from(cols.completed_segments))
        .bind(i64::from(cols.total_segments))
        .bind(bytes)
        .bind(now)
        .execute(&mut **tx)
        .await
    } else {
        sqlx::query(
            "UPDATE offline_download_tasks
             SET state = ?3,
                 pause_reason = ?4,
                 failure_reason = ?5,
                 completed_segments = ?6,
                 total_segments = ?7,
                 downloaded_bytes = ?8,
                 updated_at_epoch_secs = ?9
             WHERE series_id = ?1 AND ep_num = ?2",
        )
        .bind(sid)
        .bind(ep)
        .bind(cols.state)
        .bind(cols.pause_reason)
        .bind(cols.failure_reason)
        .bind(i64::from(cols.completed_segments))
        .bind(i64::from(cols.total_segments))
        .bind(bytes)
        .bind(now)
        .execute(&mut **tx)
        .await
    }
    .map_err(|err| ContractError::Internal {
        detail: format!("更新下载任务状态失败: {err}"),
    })?;

    Ok(())
}

fn decode_row_key(row: &TaskStateRow) -> Result<EpisodeKey, ContractError> {
    let sid_raw = u64::try_from(row.series_id).map_err(|err| ContractError::Internal {
        detail: format!("下载任务剧目标识非法: {err}"),
    })?;
    let series_id = SeriesId::try_from(sid_raw).map_err(|detail| ContractError::Internal {
        detail: format!("下载任务剧目标识无效: {detail}"),
    })?;
    let ep_num = parse_ep_num(row.ep_num)?;
    Ok(EpisodeKey::new(series_id, ep_num))
}

fn decode_row_state(row: &TaskStateRow) -> Result<(DownloadTaskState, u64), ContractError> {
    let completed =
        u32::try_from(row.completed_segments).map_err(|err| ContractError::Internal {
            detail: format!("已完成分片数非法: {err}"),
        })?;
    let total = u32::try_from(row.total_segments).map_err(|err| ContractError::Internal {
        detail: format!("总分片数非法: {err}"),
    })?;
    let bytes = u64::try_from(row.downloaded_bytes).map_err(|err| ContractError::Internal {
        detail: format!("已下载字节数非法: {err}"),
    })?;
    let state = columns_to_state(
        &row.state,
        row.pause_reason.as_deref(),
        row.failure_reason.as_deref(),
        completed,
        total,
        bytes,
    )?;
    Ok((state, bytes))
}

fn parse_ep_num(raw: i64) -> Result<EpisodeNumber, ContractError> {
    let u = u32::try_from(raw).map_err(|err| ContractError::Internal {
        detail: format!("分集序号非法: {err}"),
    })?;
    EpisodeNumber::try_from(u).map_err(|detail| ContractError::Internal {
        detail: format!("分集序号无效: {detail}"),
    })
}

fn to_i64_u64(val: u64, label: &str) -> Result<i64, ContractError> {
    i64::try_from(val).map_err(|err| ContractError::Internal {
        detail: format!("{label}超出 SQLite 整数范围: {err}"),
    })
}
