use hg_contracts::error::ContractError;
use hg_contracts::ids::{SeriesId, StreamProfile};
use hg_contracts::viewing::{PlaybackPreference, SeriesBookmark};
use sqlx::{FromRow, SqlitePool};

pub const FEATURE_NAME: &str = "viewing.watchlist_and_settings";

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
struct BookmarkRow {
    series_id: i64,
    title: String,
    cover_url: String,
    episode_count: i64,
    is_finished: i64,
    score: f64,
    bookmarked_at_epoch_secs: i64,
}

#[derive(Debug, FromRow)]
struct PlaybackPreferenceRow {
    preferred_profile: String,
    playback_cache_limit_mb: i64,
    default_rate_hundredths: i64,
}

pub async fn fetch_bookmarks(pool: &SqlitePool) -> Result<Vec<SeriesBookmark>, ContractError> {
    let rows = sqlx::query_as::<_, BookmarkRow>(
        "SELECT
            series_id,
            title,
            cover_url,
            episode_count,
            is_finished,
            score,
            bookmarked_at_epoch_secs
        FROM viewing_bookmarks
        ORDER BY bookmarked_at_epoch_secs DESC, series_id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("查询追剧收藏失败: {err}"),
    })?;

    rows.into_iter().map(TryInto::try_into).collect()
}

pub async fn fetch_is_bookmarked(
    pool: &SqlitePool,
    series_id: SeriesId,
) -> Result<bool, ContractError> {
    let series_id = i64::try_from(series_id.get()).map_err(|err| ContractError::Internal {
        detail: format!("剧目标识超出 SQLite 整数范围: {err}"),
    })?;

    let found: Option<(i64,)> =
        sqlx::query_as("SELECT 1 FROM viewing_bookmarks WHERE series_id = ?1 LIMIT 1")
            .bind(series_id)
            .fetch_optional(pool)
            .await
            .map_err(|err| ContractError::Internal {
                detail: format!("查询追剧收藏状态失败: {err}"),
            })?;

    Ok(found.is_some())
}

pub async fn fetch_playback_preference(
    pool: &SqlitePool,
) -> Result<PlaybackPreference, ContractError> {
    let row = sqlx::query_as::<_, PlaybackPreferenceRow>(
        "SELECT
            preferred_profile,
            playback_cache_limit_mb,
            default_rate_hundredths
        FROM viewing_playback_preference
        WHERE id = 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("查询播放偏好失败: {err}"),
    })?;

    match row {
        Some(row) => row.try_into(),
        None => Ok(PlaybackPreference {
            preferred_profile: StreamProfile::Hevc,
            playback_cache_limit_mb: 512,
            default_rate_hundredths: 100,
        }),
    }
}

impl TryFrom<BookmarkRow> for SeriesBookmark {
    type Error = ContractError;

    fn try_from(row: BookmarkRow) -> Result<Self, Self::Error> {
        let series_id_raw =
            u64::try_from(row.series_id).map_err(|err| ContractError::Internal {
                detail: format!("追剧收藏剧目标识非法: {err}"),
            })?;
        let series_id =
            SeriesId::try_from(series_id_raw).map_err(|detail| ContractError::Internal {
                detail: format!("追剧收藏剧目标识无效: {detail}"),
            })?;
        let episode_count =
            u32::try_from(row.episode_count).map_err(|err| ContractError::Internal {
                detail: format!("追剧收藏集数字段非法: {err}"),
            })?;
        let bookmarked_at_epoch_secs =
            u64::try_from(row.bookmarked_at_epoch_secs).map_err(|err| ContractError::Internal {
                detail: format!("追剧收藏时间戳字段非法: {err}"),
            })?;

        Ok(Self {
            series_id,
            title: row.title,
            cover_url: row.cover_url,
            episode_count,
            is_finished: row.is_finished != 0,
            score: row.score as f32,
            bookmarked_at_epoch_secs,
        })
    }
}

impl TryFrom<PlaybackPreferenceRow> for PlaybackPreference {
    type Error = ContractError;

    fn try_from(row: PlaybackPreferenceRow) -> Result<Self, Self::Error> {
        let preferred_profile = match row.preferred_profile.as_str() {
            "hevc" => StreamProfile::Hevc,
            "main" => StreamProfile::Main,
            other => {
                return Err(ContractError::Internal {
                    detail: format!("播放偏好流档位字段非法: {other}"),
                });
            }
        };
        let playback_cache_limit_mb =
            u32::try_from(row.playback_cache_limit_mb).map_err(|err| ContractError::Internal {
                detail: format!("播放缓存上限字段非法: {err}"),
            })?;
        let default_rate_hundredths =
            u16::try_from(row.default_rate_hundredths).map_err(|err| ContractError::Internal {
                detail: format!("默认倍速字段非法: {err}"),
            })?;

        Ok(Self {
            preferred_profile,
            playback_cache_limit_mb,
            default_rate_hundredths,
        })
    }
}
