CREATE TABLE IF NOT EXISTS catalog_snapshot_cache (
    cache_key TEXT PRIMARY KEY NOT NULL,
    payload_json TEXT NOT NULL,
    updated_at_epoch_secs INTEGER NOT NULL
);
