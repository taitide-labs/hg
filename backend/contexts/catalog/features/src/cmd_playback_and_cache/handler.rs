use hg_backend_contracts::catalog::PlaybackTicket;
use hg_backend_contracts::error::PortError;
use hg_contracts::error::ContractError;
use hg_contracts::ids::EpisodeKey;
use hg_infra::edge_crypto::EdgeChannelScope;

use crate::CatalogDeps;

use super::workflow::{parse_playback_ticket, playback_request_path};

pub async fn clear_catalog_cache(deps: &CatalogDeps) -> Result<(), ContractError> {
    deps.cache.invalidate_all().await;

    let mut tx = deps
        .pool()
        .begin()
        .await
        .map_err(|err| ContractError::Internal {
            detail: format!("开启清理目录缓存事务失败: {err}"),
        })?;

    sqlx::query("DELETE FROM catalog_cover_cache")
        .execute(&mut *tx)
        .await
        .map_err(|err| ContractError::Internal {
            detail: format!("清理封面缓存表失败: {err}"),
        })?;

    sqlx::query("DELETE FROM catalog_snapshot_cache")
        .execute(&mut *tx)
        .await
        .map_err(|err| ContractError::Internal {
            detail: format!("清理目录快照缓存表失败: {err}"),
        })?;

    tx.commit().await.map_err(|err| ContractError::Internal {
        detail: format!("提交清理目录缓存事务失败: {err}"),
    })?;

    deps.storage
        .clear_catalog_cache()
        .await
        .map_err(|err| ContractError::Internal {
            detail: format!("清理目录缓存目录失败: {err}"),
        })?;

    Ok(())
}

pub(crate) async fn fetch_playback_ticket(
    deps: &CatalogDeps,
    key: EpisodeKey,
) -> Result<PlaybackTicket, PortError> {
    let path = playback_request_path(key);
    let empty_body = serde_json::json!({});
    let payload = deps
        .upstream
        .request_json(EdgeChannelScope::Playback, "POST", &path, Some(&empty_body))
        .await?;
    let fallback_now = deps.clock.now_epoch_secs();

    parse_playback_ticket(key, &payload, fallback_now).map_err(|err| PortError::UpstreamRejected {
        detail: err.to_string(),
    })
}
