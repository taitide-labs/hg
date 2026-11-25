ALTER TABLE viewing_playback_preference
    ADD COLUMN window_mode TEXT
    CHECK (window_mode IS NULL OR window_mode IN ('normal', 'maximized', 'fullscreen'));

ALTER TABLE viewing_playback_preference
    ADD COLUMN window_x INTEGER;

ALTER TABLE viewing_playback_preference
    ADD COLUMN window_y INTEGER;

ALTER TABLE viewing_playback_preference
    ADD COLUMN window_width INTEGER
    CHECK (window_width IS NULL OR window_width > 0);

ALTER TABLE viewing_playback_preference
    ADD COLUMN window_height INTEGER
    CHECK (window_height IS NULL OR window_height > 0);
