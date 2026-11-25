ALTER TABLE viewing_playback_preference
    ADD COLUMN appearance TEXT NOT NULL DEFAULT 'system'
    CHECK (appearance IN ('system', 'light', 'dark'));

ALTER TABLE viewing_playback_preference
    ADD COLUMN sidebar_collapsed INTEGER NOT NULL DEFAULT 0
    CHECK (sidebar_collapsed IN (0, 1));

ALTER TABLE viewing_playback_preference
    ADD COLUMN sidebar_width INTEGER NOT NULL DEFAULT 208
    CHECK (sidebar_width BETWEEN 160 AND 360);

CREATE TABLE IF NOT EXISTS viewing_search_history (
    keyword TEXT PRIMARY KEY NOT NULL,
    searched_at_epoch_secs INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_viewing_search_history_searched_at
    ON viewing_search_history (searched_at_epoch_secs DESC);
