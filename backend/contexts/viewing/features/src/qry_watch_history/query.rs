use hg_contracts::error::ContractError;
use hg_contracts::ids::{EpisodeNumber, SeriesId};
use hg_contracts::viewing::WatchHistoryEntry;
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, FromRow)]
struct WatchHistoryRow {
    series_id: i64,
    title: String,
    cover_url: String,
    last_episode: i64,
    position_secs: i64,
    duration_secs: i64,
    episode_completed: i64,
    total_episodes: i64,
    watched_at_epoch_secs: i64,
}

pub async fn fetch_watch_history(
    pool: &SqlitePool,
) -> Result<Vec<WatchHistoryEntry>, ContractError> {
    let rows = sqlx::query_as::<_, WatchHistoryRow>(
        "SELECT
            series_id,
            title,
            cover_url,
            last_episode,
            position_secs,
            duration_secs,
            episode_completed,
            total_episodes,
            watched_at_epoch_secs
        FROM viewing_watch_history
        ORDER BY watched_at_epoch_secs DESC, series_id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("查询观看历史失败: {err}"),
    })?;

    rows.into_iter().map(TryInto::try_into).collect()
}

impl TryFrom<WatchHistoryRow> for WatchHistoryEntry {
    type Error = ContractError;

    fn try_from(row: WatchHistoryRow) -> Result<Self, Self::Error> {
        let series_id_raw =
            u64::try_from(row.series_id).map_err(|err| ContractError::Internal {
                detail: format!("观看历史剧目标识非法: {err}"),
            })?;
        let series_id =
            SeriesId::try_from(series_id_raw).map_err(|detail| ContractError::Internal {
                detail: format!("观看历史剧目标识无效: {detail}"),
            })?;
        let ep_raw = u32::try_from(row.last_episode).map_err(|err| ContractError::Internal {
            detail: format!("观看历史分集序号非法: {err}"),
        })?;
        let last_episode =
            EpisodeNumber::try_from(ep_raw).map_err(|detail| ContractError::Internal {
                detail: format!("观看历史分集序号无效: {detail}"),
            })?;
        let position_secs =
            u32::try_from(row.position_secs).map_err(|err| ContractError::Internal {
                detail: format!("观看位置字段非法: {err}"),
            })?;
        let duration_secs =
            u32::try_from(row.duration_secs).map_err(|err| ContractError::Internal {
                detail: format!("分集时长字段非法: {err}"),
            })?;
        let total_episodes =
            u32::try_from(row.total_episodes).map_err(|err| ContractError::Internal {
                detail: format!("总集数字段非法: {err}"),
            })?;
        let watched_at_epoch_secs =
            u64::try_from(row.watched_at_epoch_secs).map_err(|err| ContractError::Internal {
                detail: format!("观看时间戳字段非法: {err}"),
            })?;

        Ok(Self {
            series_id,
            title: row.title,
            cover_url: row.cover_url,
            last_episode,
            position_secs,
            duration_secs,
            episode_completed: row.episode_completed != 0,
            total_episodes,
            watched_at_epoch_secs,
        })
    }
}
