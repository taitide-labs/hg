use hg_contracts::error::ContractError;
use hg_contracts::ids::SeriesId;
use hg_contracts::viewing::PlaybackPreference;

use crate::ViewingDeps;

use super::command::ensure_enabled;
use super::workflow::{
    BookmarkWrite, check_playback_preference, plan_bookmark_series, profile_as_str,
};

pub async fn remove_watch_history(
    deps: &ViewingDeps,
    series_id: SeriesId,
) -> Result<(), ContractError> {
    ensure_enabled(deps.flags.watchlist_and_settings)?;
    let series_id = sqlite_i64(series_id.get(), "剧目标识")?;
    let mut tx = begin_write_tx(deps, "移除观看历史").await?;

    sqlx::query("DELETE FROM viewing_watch_history WHERE series_id = ?1")
        .bind(series_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| ContractError::Internal {
            detail: format!("移除观看历史失败: {err}"),
        })?;

    commit_write_tx(tx, "移除观看历史").await
}

pub async fn clear_watch_history(deps: &ViewingDeps) -> Result<(), ContractError> {
    ensure_enabled(deps.flags.watchlist_and_settings)?;
    let mut tx = begin_write_tx(deps, "清空观看历史").await?;

    sqlx::query("DELETE FROM viewing_watch_history")
        .execute(&mut *tx)
        .await
        .map_err(|err| ContractError::Internal {
            detail: format!("清空观看历史失败: {err}"),
        })?;

    commit_write_tx(tx, "清空观看历史").await
}

pub async fn bookmark_series(deps: &ViewingDeps, series_id: SeriesId) -> Result<(), ContractError> {
    ensure_enabled(deps.flags.watchlist_and_settings)?;
    let series = deps
        .catalog_metadata
        .fetch_series_snapshot(series_id)
        .await
        .map_err(ContractError::from)?;

    let bookmarked_at = deps.clock.now_epoch_secs();
    let row = plan_bookmark_series(&series, bookmarked_at);
    persist_bookmark(deps, &row).await
}

pub async fn unbookmark_series(
    deps: &ViewingDeps,
    series_id: SeriesId,
) -> Result<(), ContractError> {
    ensure_enabled(deps.flags.watchlist_and_settings)?;
    let series_id = sqlite_i64(series_id.get(), "剧目标识")?;
    let mut tx = begin_write_tx(deps, "取消追剧收藏").await?;

    sqlx::query("DELETE FROM viewing_bookmarks WHERE series_id = ?1")
        .bind(series_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| ContractError::Internal {
            detail: format!("取消追剧收藏失败: {err}"),
        })?;

    commit_write_tx(tx, "取消追剧收藏").await
}

pub async fn update_playback_preference(
    deps: &ViewingDeps,
    preference: PlaybackPreference,
) -> Result<(), ContractError> {
    ensure_enabled(deps.flags.watchlist_and_settings)?;
    check_playback_preference(&preference)?;

    let profile = profile_as_str(preference.preferred_profile);
    let mut tx = begin_write_tx(deps, "更新播放偏好").await?;

    sqlx::query(
        "INSERT INTO viewing_playback_preference (
            id,
            preferred_profile,
            playback_cache_limit_mb,
            default_rate_hundredths
        ) VALUES (1, ?1, ?2, ?3)
        ON CONFLICT(id) DO UPDATE SET
            preferred_profile = excluded.preferred_profile,
            playback_cache_limit_mb = excluded.playback_cache_limit_mb,
            default_rate_hundredths = excluded.default_rate_hundredths",
    )
    .bind(profile)
    .bind(i64::from(preference.playback_cache_limit_mb))
    .bind(i64::from(preference.default_rate_hundredths))
    .execute(&mut *tx)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("更新播放偏好失败: {err}"),
    })?;

    commit_write_tx(tx, "更新播放偏好").await
}

pub async fn clear_playback_cache(deps: &ViewingDeps) -> Result<(), ContractError> {
    ensure_enabled(deps.flags.watchlist_and_settings)?;
    let mut tx = begin_write_tx(deps, "清理播放缓存").await?;

    sqlx::query("DELETE FROM viewing_playback_cache_segments")
        .execute(&mut *tx)
        .await
        .map_err(|err| ContractError::Internal {
            detail: format!("清理播放缓存索引失败: {err}"),
        })?;

    deps.storage
        .clear_playback_cache()
        .await
        .map_err(|err| ContractError::Internal {
            detail: format!("清理播放缓存目录失败: {err}"),
        })?;

    commit_write_tx(tx, "清理播放缓存").await
}

async fn persist_bookmark(deps: &ViewingDeps, row: &BookmarkWrite) -> Result<(), ContractError> {
    let series_id = sqlite_i64(row.series_id.get(), "剧目标识")?;
    let bookmarked_at = sqlite_i64(row.bookmarked_at_epoch_secs, "收藏时间戳")?;
    let mut tx = begin_write_tx(deps, "写入追剧收藏").await?;

    sqlx::query(
        "INSERT INTO viewing_bookmarks (
            series_id,
            title,
            cover_url,
            episode_count,
            is_finished,
            score,
            bookmarked_at_epoch_secs
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ON CONFLICT(series_id) DO UPDATE SET
            title = excluded.title,
            cover_url = excluded.cover_url,
            episode_count = excluded.episode_count,
            is_finished = excluded.is_finished,
            score = excluded.score,
            bookmarked_at_epoch_secs = excluded.bookmarked_at_epoch_secs",
    )
    .bind(series_id)
    .bind(&row.title)
    .bind(&row.cover_url)
    .bind(i64::from(row.episode_count))
    .bind(i64::from(row.is_finished))
    .bind(f64::from(row.score))
    .bind(bookmarked_at)
    .execute(&mut *tx)
    .await
    .map_err(|err| ContractError::Internal {
        detail: format!("写入追剧收藏失败: {err}"),
    })?;

    commit_write_tx(tx, "写入追剧收藏").await
}

fn sqlite_i64(value: u64, label: &str) -> Result<i64, ContractError> {
    i64::try_from(value).map_err(|err| ContractError::Internal {
        detail: format!("{label}超出 SQLite 整数范围: {err}"),
    })
}

async fn begin_write_tx<'a>(
    deps: &'a ViewingDeps,
    op: &str,
) -> Result<sqlx::Transaction<'a, sqlx::Sqlite>, ContractError> {
    deps.db
        .pool()
        .begin()
        .await
        .map_err(|err| ContractError::Internal {
            detail: format!("开启{op}事务失败: {err}"),
        })
}

async fn commit_write_tx(
    tx: sqlx::Transaction<'_, sqlx::Sqlite>,
    op: &str,
) -> Result<(), ContractError> {
    tx.commit().await.map_err(|err| ContractError::Internal {
        detail: format!("提交{op}事务失败: {err}"),
    })
}
