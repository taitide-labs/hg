use std::collections::HashMap;

use hg_backend_contracts::error::PortError;
use hg_backend_contracts::offline::CommittedOfflineAsset;
use hg_contracts::error::ContractError;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId, StreamProfile};
use hg_contracts::offline::{OfflineEpisodeView, OfflineSeriesView};
use hg_infra::storage::StorageLayout;
use sqlx::{FromRow, SqlitePool};

pub const FEATURE_NAME: &str = "offline.library_and_outbox";

pub fn ensure_enabled(enabled: bool) -> Result<(), ContractError> {
    if enabled {
        Ok(())
    } else {
        Err(ContractError::Internal {
            detail: format!("{FEATURE_NAME} 特性开关未开启"),
        })
    }
}

#[derive(Debug, FromRow)]
struct OfflineSeriesRow {
    series_id: i64,
    title: String,
    cover_local_path: String,
}

#[derive(Debug, FromRow)]
struct OfflineEpisodeRow {
    series_id: i64,
    ep_num: i64,
    title: String,
    duration_secs: i64,
    profile: String,
    size_bytes: i64,
    downloaded_at_epoch_secs: i64,
}

#[derive(Debug, FromRow)]
struct CommittedAssetRow {
    profile: String,
    playlist_rel_path: String,
    duration_secs: i64,
    size_bytes: i64,
}

pub async fn fetch_offline_library(
    pool: &SqlitePool,
    storage: &StorageLayout,
) -> Result<Vec<OfflineSeriesView>, ContractError> {
    let series_rows = sqlx::query_as::<_, OfflineSeriesRow>(
        "SELECT series_id, title, cover_local_path
         FROM offline_series
         ORDER BY updated_at_epoch_secs DESC, series_id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("查询离线剧目失败: {err}"),
    })?;

    let episode_rows = sqlx::query_as::<_, OfflineEpisodeRow>(
        "SELECT
            series_id,
            ep_num,
            title,
            duration_secs,
            profile,
            size_bytes,
            downloaded_at_epoch_secs
         FROM offline_episodes
         ORDER BY series_id ASC, ep_num ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("查询离线分集失败: {err}"),
    })?;

    let mut episodes_by_series: HashMap<SeriesId, Vec<OfflineEpisodeView>> = HashMap::new();
    for row in episode_rows {
        let series_id = parse_series_id(row.series_id)?;
        let ep_view = row.try_into()?;
        episodes_by_series
            .entry(series_id)
            .or_default()
            .push(ep_view);
    }

    let mut result = Vec::with_capacity(series_rows.len());
    for row in series_rows {
        let series_id = parse_series_id(row.series_id)?;
        let Some(episodes) = episodes_by_series.remove(&series_id) else {
            continue;
        };
        if episodes.is_empty() {
            continue;
        }
        let total_size_bytes = episodes
            .iter()
            .fold(0_u64, |acc, ep| acc.saturating_add(ep.size_bytes));
        let cover_local_uri = resolve_cover_uri(storage, &row.cover_local_path)?;
        result.push(OfflineSeriesView {
            series_id,
            title: row.title,
            cover_local_uri,
            total_size_bytes,
            episodes,
        });
    }

    Ok(result)
}

pub async fn find_committed_episode(
    pool: &SqlitePool,
    storage: &StorageLayout,
    key: EpisodeKey,
) -> Result<Option<CommittedOfflineAsset>, PortError> {
    let series_id = i64::try_from(key.series_id.get()).map_err(|err| PortError::InvalidState {
        detail: format!("剧目标识超出 SQLite 整数范围: {err}"),
    })?;
    let ep_num = i64::from(key.ep_num.get());

    let row = sqlx::query_as::<_, CommittedAssetRow>(
        "SELECT profile, playlist_rel_path, duration_secs, size_bytes
         FROM offline_episodes
         WHERE series_id = ?1 AND ep_num = ?2",
    )
    .bind(series_id)
    .bind(ep_num)
    .fetch_optional(pool)
    .await
    .map_err(|err| PortError::StorageFailure {
        detail: format!("查询已提交离线分集失败: {err}"),
    })?;

    row.map(|entry| entry.into_asset(storage, key)).transpose()
}

pub async fn total_offline_library_bytes(pool: &SqlitePool) -> Result<u64, PortError> {
    let total =
        sqlx::query_scalar::<_, i64>("SELECT COALESCE(SUM(size_bytes), 0) FROM offline_episodes")
            .fetch_one(pool)
            .await
            .map_err(|err| PortError::StorageFailure {
                detail: format!("统计离线片库占用体积失败: {err}"),
            })?;

    u64::try_from(total).map_err(|err| PortError::InvalidState {
        detail: format!("离线片库总占用体积非法: {err}"),
    })
}

impl TryFrom<OfflineEpisodeRow> for OfflineEpisodeView {
    type Error = ContractError;

    fn try_from(row: OfflineEpisodeRow) -> Result<Self, Self::Error> {
        let ep_raw = u32::try_from(row.ep_num).map_err(|err| ContractError::Internal {
            detail: format!("离线分集序号非法: {err}"),
        })?;
        let ep_num = EpisodeNumber::try_from(ep_raw).map_err(|detail| ContractError::Internal {
            detail: format!("离线分集序号无效: {detail}"),
        })?;
        let duration_secs =
            u32::try_from(row.duration_secs).map_err(|err| ContractError::Internal {
                detail: format!("离线分集时长字段非法: {err}"),
            })?;
        let profile =
            parse_stream_profile(&row.profile).map_err(|detail| ContractError::Internal {
                detail: detail.to_owned(),
            })?;
        let size_bytes = u64::try_from(row.size_bytes).map_err(|err| ContractError::Internal {
            detail: format!("离线分集大小字段非法: {err}"),
        })?;
        let downloaded_at_epoch_secs =
            u64::try_from(row.downloaded_at_epoch_secs).map_err(|err| ContractError::Internal {
                detail: format!("离线分集下载时间戳字段非法: {err}"),
            })?;

        Ok(Self {
            ep_num,
            title: row.title,
            duration_secs,
            profile,
            size_bytes,
            downloaded_at_epoch_secs,
        })
    }
}

impl CommittedAssetRow {
    fn into_asset(
        self,
        storage: &StorageLayout,
        key: EpisodeKey,
    ) -> Result<CommittedOfflineAsset, PortError> {
        storage.resolve_offline_library_path(&self.playlist_rel_path)?;
        let profile =
            parse_stream_profile(&self.profile).map_err(|detail| PortError::InvalidState {
                detail: detail.to_owned(),
            })?;
        let duration_secs =
            u32::try_from(self.duration_secs).map_err(|err| PortError::InvalidState {
                detail: format!("离线分集时长字段非法: {err}"),
            })?;
        let size_bytes = u64::try_from(self.size_bytes).map_err(|err| PortError::InvalidState {
            detail: format!("离线分集大小字段非法: {err}"),
        })?;

        Ok(CommittedOfflineAsset {
            key,
            profile,
            playlist_rel_path: self.playlist_rel_path,
            duration_secs,
            size_bytes,
        })
    }
}

fn parse_series_id(raw: i64) -> Result<SeriesId, ContractError> {
    let raw_u64 = u64::try_from(raw).map_err(|err| ContractError::Internal {
        detail: format!("离线剧目标识非法: {err}"),
    })?;
    SeriesId::try_from(raw_u64).map_err(|detail| ContractError::Internal {
        detail: format!("离线剧目标识无效: {detail}"),
    })
}

fn parse_stream_profile(raw: &str) -> Result<StreamProfile, &'static str> {
    match raw {
        "hevc" => Ok(StreamProfile::Hevc),
        "main" => Ok(StreamProfile::Main),
        _ => Err("离线分集流档位字段非法"),
    }
}

fn resolve_cover_uri(
    storage: &StorageLayout,
    cover_local_path: &str,
) -> Result<String, ContractError> {
    if cover_local_path.is_empty() {
        return Ok(String::new());
    }
    if cover_local_path.contains("://") {
        return Ok(cover_local_path.to_owned());
    }
    let resolved = storage
        .resolve_offline_library_path(cover_local_path)
        .map_err(|err| ContractError::Internal {
            detail: format!("解析离线封面路径失败: {err}"),
        })?;
    let normalized = resolved.to_string_lossy().replace('\\', "/");
    Ok(format!("file:///{}", normalized.trim_start_matches('/')))
}
