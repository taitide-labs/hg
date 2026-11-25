DROP INDEX IF EXISTS idx_offline_episodes_series;

DROP INDEX IF EXISTS idx_viewing_watch_history_watched_at;
CREATE INDEX IF NOT EXISTS idx_viewing_watch_history_watched_at
    ON viewing_watch_history (watched_at_epoch_secs DESC, series_id ASC);

DROP INDEX IF EXISTS idx_viewing_bookmarks_bookmarked_at;
CREATE INDEX IF NOT EXISTS idx_viewing_bookmarks_bookmarked_at
    ON viewing_bookmarks (bookmarked_at_epoch_secs DESC, series_id ASC);

DROP INDEX IF EXISTS idx_viewing_search_history_searched_at;
CREATE INDEX IF NOT EXISTS idx_viewing_search_history_searched_at
    ON viewing_search_history (searched_at_epoch_secs DESC, keyword ASC);

CREATE INDEX IF NOT EXISTS idx_offline_series_updated
    ON offline_series (updated_at_epoch_secs DESC, series_id ASC);

CREATE INDEX IF NOT EXISTS idx_offline_download_tasks_wifi_pause
    ON offline_download_tasks (state, pause_reason);

CREATE TABLE IF NOT EXISTS viewing_episode_progress (
    series_id INTEGER NOT NULL,
    ep_num INTEGER NOT NULL CHECK (ep_num >= 1),
    position_secs INTEGER NOT NULL CHECK (position_secs >= 0),
    duration_secs INTEGER NOT NULL CHECK (duration_secs >= 0),
    completed INTEGER NOT NULL CHECK (completed IN (0, 1)),
    watched_at_epoch_secs INTEGER NOT NULL,
    PRIMARY KEY (series_id, ep_num)
);

CREATE INDEX IF NOT EXISTS idx_viewing_episode_progress_watched
    ON viewing_episode_progress (series_id, watched_at_epoch_secs DESC);
