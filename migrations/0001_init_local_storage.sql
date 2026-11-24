PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS catalog_cache_entries (
    cache_key TEXT PRIMARY KEY NOT NULL,
    payload_json TEXT NOT NULL,
    size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0),
    expires_at_epoch_secs INTEGER NOT NULL,
    updated_at_epoch_secs INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_catalog_cache_entries_expires
    ON catalog_cache_entries (expires_at_epoch_secs);

CREATE TABLE IF NOT EXISTS catalog_cover_cache (
    cover_url TEXT PRIMARY KEY NOT NULL,
    local_rel_path TEXT NOT NULL,
    size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0),
    accessed_at_epoch_secs INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_catalog_cover_cache_accessed
    ON catalog_cover_cache (accessed_at_epoch_secs);

CREATE TABLE IF NOT EXISTS offline_series (
    series_id INTEGER PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    cover_local_path TEXT NOT NULL,
    total_episodes INTEGER NOT NULL CHECK (total_episodes >= 0),
    updated_at_epoch_secs INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS offline_episodes (
    series_id INTEGER NOT NULL,
    ep_num INTEGER NOT NULL CHECK (ep_num >= 1),
    title TEXT NOT NULL,
    duration_secs INTEGER NOT NULL CHECK (duration_secs >= 0),
    profile TEXT NOT NULL CHECK (profile IN ('hevc', 'main')),
    playlist_rel_path TEXT NOT NULL,
    size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0),
    downloaded_at_epoch_secs INTEGER NOT NULL,
    PRIMARY KEY (series_id, ep_num)
);

CREATE INDEX IF NOT EXISTS idx_offline_episodes_series
    ON offline_episodes (series_id, ep_num);

CREATE TABLE IF NOT EXISTS offline_download_tasks (
    series_id INTEGER NOT NULL,
    ep_num INTEGER NOT NULL CHECK (ep_num >= 1),
    series_title TEXT NOT NULL,
    episode_title TEXT NOT NULL,
    cover_url TEXT NOT NULL,
    profile TEXT NOT NULL CHECK (profile IN ('hevc', 'main')),
    state TEXT NOT NULL CHECK (
        state IN (
            'queued',
            'fetching_source',
            'downloading',
            'paused',
            'failed',
            'completed',
            'cancelled'
        )
    ),
    pause_reason TEXT CHECK (
        pause_reason IS NULL
        OR pause_reason IN ('user_requested', 'wifi_only_blocked', 'app_interrupted')
    ),
    failure_reason TEXT CHECK (
        failure_reason IS NULL
        OR failure_reason IN (
            'preview_stream_only',
            'network_error',
            'storage_full',
            'ticket_expired',
            'corrupted_segment'
        )
    ),
    completed_segments INTEGER NOT NULL DEFAULT 0 CHECK (completed_segments >= 0),
    total_segments INTEGER NOT NULL DEFAULT 0 CHECK (total_segments >= 0),
    downloaded_bytes INTEGER NOT NULL DEFAULT 0 CHECK (downloaded_bytes >= 0),
    staging_rel_dir TEXT NOT NULL,
    created_at_epoch_secs INTEGER NOT NULL,
    updated_at_epoch_secs INTEGER NOT NULL,
    PRIMARY KEY (series_id, ep_num)
);

CREATE INDEX IF NOT EXISTS idx_offline_download_tasks_state
    ON offline_download_tasks (state, created_at_epoch_secs);

CREATE TABLE IF NOT EXISTS offline_download_policy (
    id INTEGER PRIMARY KEY NOT NULL CHECK (id = 1),
    preferred_profile TEXT NOT NULL CHECK (preferred_profile IN ('hevc', 'main')),
    wifi_only INTEGER NOT NULL CHECK (wifi_only IN (0, 1)),
    max_concurrent_tasks INTEGER NOT NULL CHECK (max_concurrent_tasks BETWEEN 1 AND 5),
    storage_directory TEXT
);

INSERT OR IGNORE INTO offline_download_policy (
    id,
    preferred_profile,
    wifi_only,
    max_concurrent_tasks,
    storage_directory
) VALUES (1, 'hevc', 1, 2, NULL);

CREATE TABLE IF NOT EXISTS offline_outbox_events (
    event_id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL CHECK (
        event_type IN ('offline_episode_removed', 'offline_series_removed')
    ),
    payload_json TEXT NOT NULL,
    created_at_epoch_secs INTEGER NOT NULL,
    dispatched_at_epoch_secs INTEGER
);

CREATE INDEX IF NOT EXISTS idx_offline_outbox_pending
    ON offline_outbox_events (dispatched_at_epoch_secs, event_id);

CREATE TABLE IF NOT EXISTS viewing_watch_history (
    series_id INTEGER PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    cover_url TEXT NOT NULL,
    last_episode INTEGER NOT NULL CHECK (last_episode >= 1),
    position_secs INTEGER NOT NULL CHECK (position_secs >= 0),
    duration_secs INTEGER NOT NULL CHECK (duration_secs >= 0),
    episode_completed INTEGER NOT NULL CHECK (episode_completed IN (0, 1)),
    total_episodes INTEGER NOT NULL CHECK (total_episodes >= 0),
    watched_at_epoch_secs INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_viewing_watch_history_watched_at
    ON viewing_watch_history (watched_at_epoch_secs DESC);

CREATE TABLE IF NOT EXISTS viewing_bookmarks (
    series_id INTEGER PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    cover_url TEXT NOT NULL,
    episode_count INTEGER NOT NULL CHECK (episode_count >= 0),
    is_finished INTEGER NOT NULL CHECK (is_finished IN (0, 1)),
    score REAL NOT NULL,
    bookmarked_at_epoch_secs INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_viewing_bookmarks_bookmarked_at
    ON viewing_bookmarks (bookmarked_at_epoch_secs DESC);

CREATE TABLE IF NOT EXISTS viewing_playback_preference (
    id INTEGER PRIMARY KEY NOT NULL CHECK (id = 1),
    preferred_profile TEXT NOT NULL CHECK (preferred_profile IN ('hevc', 'main')),
    playback_cache_limit_mb INTEGER NOT NULL CHECK (playback_cache_limit_mb >= 64),
    default_rate_hundredths INTEGER NOT NULL CHECK (default_rate_hundredths BETWEEN 50 AND 300)
);

INSERT OR IGNORE INTO viewing_playback_preference (
    id,
    preferred_profile,
    playback_cache_limit_mb,
    default_rate_hundredths
) VALUES (1, 'hevc', 512, 100);

CREATE TABLE IF NOT EXISTS viewing_playback_cache_segments (
    segment_key TEXT PRIMARY KEY NOT NULL,
    local_rel_path TEXT NOT NULL,
    size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0),
    accessed_at_epoch_secs INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_viewing_playback_cache_accessed
    ON viewing_playback_cache_segments (accessed_at_epoch_secs);
