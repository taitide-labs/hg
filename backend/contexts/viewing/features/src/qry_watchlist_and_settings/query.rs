use hg_contracts::error::ContractError;
use hg_contracts::ids::{SeriesId, StreamProfile};
use hg_contracts::viewing::{
    AppAppearance, PlaybackPreference, SearchHistoryEntry, SeriesBookmark, WindowPlacementMode,
    WindowPlacementPref,
};
use sqlx::{FromRow, SqlitePool};

pub const FEATURE_NAME: &str = "viewing.watchlist_and_settings";

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
    appearance: String,
    sidebar_collapsed: i64,
    sidebar_width: i64,
    window_mode: Option<String>,
    window_x: Option<i64>,
    window_y: Option<i64>,
    window_width: Option<i64>,
    window_height: Option<i64>,
}

#[derive(Debug, FromRow)]
struct SearchHistoryRow {
    keyword: String,
    searched_at_epoch_secs: i64,
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
            default_rate_hundredths,
            appearance,
            sidebar_collapsed,
            sidebar_width,
            window_mode,
            window_x,
            window_y,
            window_width,
            window_height
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
        None => Ok(PlaybackPreference::default()),
    }
}

pub async fn fetch_search_history(
    pool: &SqlitePool,
) -> Result<Vec<SearchHistoryEntry>, ContractError> {
    let rows = sqlx::query_as::<_, SearchHistoryRow>(
        "SELECT
            keyword,
            searched_at_epoch_secs
        FROM viewing_search_history
        ORDER BY searched_at_epoch_secs DESC, keyword ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("查询搜索历史失败: {err}"),
    })?;

    rows.into_iter().map(TryInto::try_into).collect()
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
        let appearance = match row.appearance.as_str() {
            "system" => AppAppearance::System,
            "light" => AppAppearance::Light,
            "dark" => AppAppearance::Dark,
            other => {
                return Err(ContractError::Internal {
                    detail: format!("外观模式字段非法: {other}"),
                });
            }
        };
        let sidebar_width =
            u16::try_from(row.sidebar_width).map_err(|err| ContractError::Internal {
                detail: format!("侧边栏宽度字段非法: {err}"),
            })?;
        let window_placement = decode_window_placement(&row)?;

        Ok(Self {
            preferred_profile,
            playback_cache_limit_mb,
            default_rate_hundredths,
            appearance,
            sidebar_collapsed: row.sidebar_collapsed != 0,
            sidebar_width,
            window_placement,
        })
    }
}

fn decode_window_placement(
    row: &PlaybackPreferenceRow,
) -> Result<Option<WindowPlacementPref>, ContractError> {
    let (Some(mode), Some(x), Some(y), Some(width), Some(height)) = (
        row.window_mode.as_deref(),
        row.window_x,
        row.window_y,
        row.window_width,
        row.window_height,
    ) else {
        let any_present = row.window_mode.is_some()
            || row.window_x.is_some()
            || row.window_y.is_some()
            || row.window_width.is_some()
            || row.window_height.is_some();
        if any_present {
            return Err(ContractError::Internal {
                detail: "窗口状态字段不完整".to_owned(),
            });
        }
        return Ok(None);
    };

    let mode = match mode {
        "normal" => WindowPlacementMode::Normal,
        "maximized" => WindowPlacementMode::Maximized,
        "fullscreen" => WindowPlacementMode::FullScreen,
        other => {
            return Err(ContractError::Internal {
                detail: format!("窗口模式字段非法: {other}"),
            });
        }
    };
    let x = i32::try_from(x).map_err(|err| ContractError::Internal {
        detail: format!("窗口横坐标字段非法: {err}"),
    })?;
    let y = i32::try_from(y).map_err(|err| ContractError::Internal {
        detail: format!("窗口纵坐标字段非法: {err}"),
    })?;
    let width = u16::try_from(width).map_err(|err| ContractError::Internal {
        detail: format!("窗口宽度字段非法: {err}"),
    })?;
    let height = u16::try_from(height).map_err(|err| ContractError::Internal {
        detail: format!("窗口高度字段非法: {err}"),
    })?;

    Ok(Some(WindowPlacementPref {
        mode,
        x,
        y,
        width,
        height,
    }))
}

impl TryFrom<SearchHistoryRow> for SearchHistoryEntry {
    type Error = ContractError;

    fn try_from(row: SearchHistoryRow) -> Result<Self, Self::Error> {
        let searched_at_epoch_secs =
            u64::try_from(row.searched_at_epoch_secs).map_err(|err| ContractError::Internal {
                detail: format!("搜索历史时间戳字段非法: {err}"),
            })?;
        Ok(Self {
            keyword: row.keyword,
            searched_at_epoch_secs,
        })
    }
}
