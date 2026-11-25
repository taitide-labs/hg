use hg_contracts::error::ContractError;
use hg_contracts::viewing::{ReportWatchProgressRequest, ReportWatchProgressResult};

use crate::ViewingDeps;

use super::command::RecordWatchProgressCommand;
use super::workflow::{WatchHistoryWrite, plan_record_watch_progress};

pub async fn execute(
    deps: &ViewingDeps,
    request: ReportWatchProgressRequest,
) -> Result<ReportWatchProgressResult, ContractError> {
    let command = RecordWatchProgressCommand::try_from(request)?;
    let series = deps
        .catalog_metadata
        .fetch_series_snapshot(command.key.series_id)
        .await
        .map_err(ContractError::from)?;

    let watched_at = deps.clock.now_epoch_secs();
    let outcome = plan_record_watch_progress(command, &series, watched_at);

    persist_watch_history(deps, &outcome.history_write).await?;
    Ok(outcome.result)
}

async fn persist_watch_history(
    deps: &ViewingDeps,
    row: &WatchHistoryWrite,
) -> Result<(), ContractError> {
    let series_id = i64::try_from(row.series_id.get()).map_err(|err| ContractError::Internal {
        detail: format!("剧目标识超出 SQLite 整数范围: {err}"),
    })?;
    let watched_at =
        i64::try_from(row.watched_at_epoch_secs).map_err(|err| ContractError::Internal {
            detail: format!("观看时间戳超出 SQLite 整数范围: {err}"),
        })?;
    let completed_flag = i64::from(row.episode_completed);

    let mut tx = deps
        .db
        .pool()
        .begin()
        .await
        .map_err(|err| ContractError::Internal {
            detail: format!("开启观看历史写入事务失败: {err}"),
        })?;

    sqlx::query(
        "INSERT INTO viewing_watch_history (
            series_id,
            title,
            cover_url,
            last_episode,
            position_secs,
            duration_secs,
            episode_completed,
            total_episodes,
            watched_at_epoch_secs
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ON CONFLICT(series_id) DO UPDATE SET
            title = excluded.title,
            cover_url = excluded.cover_url,
            last_episode = excluded.last_episode,
            position_secs = excluded.position_secs,
            duration_secs = excluded.duration_secs,
            episode_completed = excluded.episode_completed,
            total_episodes = excluded.total_episodes,
            watched_at_epoch_secs = excluded.watched_at_epoch_secs",
    )
    .bind(series_id)
    .bind(&row.title)
    .bind(&row.cover_url)
    .bind(i64::from(row.last_episode.get()))
    .bind(i64::from(row.position_secs))
    .bind(i64::from(row.duration_secs))
    .bind(completed_flag)
    .bind(i64::from(row.total_episodes))
    .bind(watched_at)
    .execute(&mut *tx)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("写入观看历史失败: {err}"),
    })?;

    sqlx::query(
        "INSERT INTO viewing_episode_progress (
            series_id,
            ep_num,
            position_secs,
            duration_secs,
            completed,
            watched_at_epoch_secs
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(series_id, ep_num) DO UPDATE SET
            position_secs = excluded.position_secs,
            duration_secs = excluded.duration_secs,
            completed = CASE
                WHEN viewing_episode_progress.completed = 1 THEN 1
                ELSE excluded.completed
            END,
            watched_at_epoch_secs = excluded.watched_at_epoch_secs",
    )
    .bind(series_id)
    .bind(i64::from(row.last_episode.get()))
    .bind(i64::from(row.position_secs))
    .bind(i64::from(row.duration_secs))
    .bind(completed_flag)
    .bind(watched_at)
    .execute(&mut *tx)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("写入分集观看进度失败: {err}"),
    })?;

    tx.commit().await.map_err(|err| ContractError::Internal {
        detail: format!("提交观看历史事务失败: {err}"),
    })
}
