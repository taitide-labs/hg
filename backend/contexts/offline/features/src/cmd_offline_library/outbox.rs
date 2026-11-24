use hg_backend_contracts::offline::OfflineDomainEvent;
use hg_contracts::error::ContractError;
use sqlx::FromRow;

use crate::OfflineDeps;

#[derive(Debug, FromRow)]
struct PendingOutboxRow {
    event_id: i64,
    payload_json: String,
}

pub fn decode_outbox_event(payload_json: &str) -> Result<OfflineDomainEvent, ContractError> {
    serde_json::from_str(payload_json).map_err(|err| ContractError::Internal {
        detail: format!("反序列化离线 Outbox 事件失败: {err}"),
    })
}

pub async fn dispatch_pending_outbox(deps: &OfflineDeps) -> Result<usize, ContractError> {
    let pending = sqlx::query_as::<_, PendingOutboxRow>(
        "SELECT event_id, payload_json
         FROM offline_outbox_events
         WHERE dispatched_at_epoch_secs IS NULL
         ORDER BY event_id ASC",
    )
    .fetch_all(deps.pool())
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("查询待派发离线 Outbox 事件失败: {err}"),
    })?;

    let mut dispatched = 0_usize;
    for row in pending {
        if dispatch_single_event(deps, row).await? {
            dispatched = dispatched.saturating_add(1);
        }
    }

    Ok(dispatched)
}

async fn dispatch_single_event(
    deps: &OfflineDeps,
    row: PendingOutboxRow,
) -> Result<bool, ContractError> {
    let event = decode_outbox_event(&row.payload_json)?;
    let asset_rel_dir = match &event {
        OfflineDomainEvent::OfflineEpisodeRemoved { key, asset_rel_dir } => {
            deps.playback_halt
                .halt_episode(*key)
                .await
                .map_err(ContractError::from)?;
            asset_rel_dir.as_str()
        }
        OfflineDomainEvent::OfflineSeriesRemoved {
            series_id,
            asset_rel_dir,
        } => {
            deps.playback_halt
                .halt_series(*series_id)
                .await
                .map_err(ContractError::from)?;
            asset_rel_dir.as_str()
        }
    };

    deps.storage
        .remove_offline_rel_dir_with_retry(asset_rel_dir)
        .await
        .map_err(|err| ContractError::Internal {
            detail: format!("回收离线资产目录失败: {err}"),
        })?;

    let dispatched_at =
        i64::try_from(deps.clock.now_epoch_secs()).map_err(|err| ContractError::Internal {
            detail: format!("派发时间戳超出 SQLite 整数范围: {err}"),
        })?;

    let updated = sqlx::query(
        "UPDATE offline_outbox_events
         SET dispatched_at_epoch_secs = ?1
         WHERE event_id = ?2 AND dispatched_at_epoch_secs IS NULL",
    )
    .bind(dispatched_at)
    .bind(row.event_id)
    .execute(deps.pool())
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("标记离线 Outbox 事件已派发失败: {err}"),
    })?;

    Ok(updated.rows_affected() > 0)
}
