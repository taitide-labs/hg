use hg_backend_contracts::offline::OfflineDomainEvent;
use hg_contracts::error::ContractError;
use hg_contracts::ids::{EpisodeKey, SeriesId};
use sqlx::{Sqlite, Transaction};

use crate::OfflineDeps;

use super::outbox::dispatch_pending_outbox;
use super::workflow::{build_episode_removed_event, build_series_removed_event, outbox_event_type};

pub async fn delete_offline_episode(
    deps: &OfflineDeps,
    key: EpisodeKey,
) -> Result<(), ContractError> {
    let series_id = to_sqlite_series_id(key.series_id)?;
    let ep_num = i64::from(key.ep_num.get());
    let now = to_sqlite_timestamp(deps.clock.now_epoch_secs())?;

    let mut tx = begin_tx(deps).await?;
    let deleted = sqlx::query("DELETE FROM offline_episodes WHERE series_id = ?1 AND ep_num = ?2")
        .bind(series_id)
        .bind(ep_num)
        .execute(&mut *tx)
        .await
        .map_err(|err| ContractError::Internal {
            detail: format!("删除离线分集元数据失败: {err}"),
        })?;

    if deleted.rows_affected() > 0 {
        let remaining: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM offline_episodes WHERE series_id = ?1")
                .bind(series_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(|err| ContractError::Internal {
                    detail: format!("统计离线剧目剩余分集失败: {err}"),
                })?;

        if remaining == 0 {
            sqlx::query("DELETE FROM offline_series WHERE series_id = ?1")
                .bind(series_id)
                .execute(&mut *tx)
                .await
                .map_err(|err| ContractError::Internal {
                    detail: format!("清理空离线剧目元数据失败: {err}"),
                })?;
        }

        let event = build_episode_removed_event(key);
        append_outbox_event(&mut tx, &event, now).await?;
    }

    tx.commit().await.map_err(|err| ContractError::Internal {
        detail: format!("提交离线分集删档事务失败: {err}"),
    })?;

    let _dispatched = dispatch_pending_outbox(deps).await?;
    Ok(())
}

pub async fn delete_offline_series(
    deps: &OfflineDeps,
    series_id: SeriesId,
) -> Result<(), ContractError> {
    let series_id_i64 = to_sqlite_series_id(series_id)?;
    let now = to_sqlite_timestamp(deps.clock.now_epoch_secs())?;

    let mut tx = begin_tx(deps).await?;
    let episodes_deleted = sqlx::query("DELETE FROM offline_episodes WHERE series_id = ?1")
        .bind(series_id_i64)
        .execute(&mut *tx)
        .await
        .map_err(|err| ContractError::Internal {
            detail: format!("删除离线剧目分集元数据失败: {err}"),
        })?;
    let series_deleted = sqlx::query("DELETE FROM offline_series WHERE series_id = ?1")
        .bind(series_id_i64)
        .execute(&mut *tx)
        .await
        .map_err(|err| ContractError::Internal {
            detail: format!("删除离线剧目元数据失败: {err}"),
        })?;

    if episodes_deleted.rows_affected() > 0 || series_deleted.rows_affected() > 0 {
        let event = build_series_removed_event(series_id);
        append_outbox_event(&mut tx, &event, now).await?;
    }

    tx.commit().await.map_err(|err| ContractError::Internal {
        detail: format!("提交离线剧目删档事务失败: {err}"),
    })?;

    let _dispatched = dispatch_pending_outbox(deps).await?;
    Ok(())
}

async fn begin_tx(deps: &OfflineDeps) -> Result<Transaction<'_, Sqlite>, ContractError> {
    deps.pool()
        .begin()
        .await
        .map_err(|err| ContractError::Internal {
            detail: format!("开启离线片库删档事务失败: {err}"),
        })
}

async fn append_outbox_event(
    tx: &mut Transaction<'_, Sqlite>,
    event: &OfflineDomainEvent,
    created_at_epoch_secs: i64,
) -> Result<(), ContractError> {
    let payload_json = serde_json::to_string(event).map_err(|err| ContractError::Internal {
        detail: format!("序列化离线 Outbox 事件失败: {err}"),
    })?;

    sqlx::query(
        "INSERT INTO offline_outbox_events (
            event_type,
            payload_json,
            created_at_epoch_secs,
            dispatched_at_epoch_secs
        ) VALUES (?1, ?2, ?3, NULL)",
    )
    .bind(outbox_event_type(event))
    .bind(payload_json)
    .bind(created_at_epoch_secs)
    .execute(&mut **tx)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("写入离线 Outbox 事件失败: {err}"),
    })?;

    Ok(())
}

fn to_sqlite_series_id(series_id: SeriesId) -> Result<i64, ContractError> {
    i64::try_from(series_id.get()).map_err(|err| ContractError::Internal {
        detail: format!("剧目标识超出 SQLite 整数范围: {err}"),
    })
}

fn to_sqlite_timestamp(epoch_secs: u64) -> Result<i64, ContractError> {
    i64::try_from(epoch_secs).map_err(|err| ContractError::Internal {
        detail: format!("时间戳超出 SQLite 整数范围: {err}"),
    })
}
