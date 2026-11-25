DROP TABLE IF EXISTS catalog_cache_entries;

CREATE TABLE IF NOT EXISTS viewing_disliked_series (
    series_id INTEGER PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    cover_url TEXT NOT NULL,
    episode_count INTEGER NOT NULL CHECK (episode_count >= 0),
    is_finished INTEGER NOT NULL CHECK (is_finished IN (0, 1)),
    score REAL NOT NULL,
    disliked_at_epoch_secs INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_viewing_disliked_series_at
    ON viewing_disliked_series (disliked_at_epoch_secs DESC);
