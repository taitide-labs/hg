use hg_backend_contracts::catalog::CatalogPlaybackTicketPort as _;
use hg_catalog_features::cmd_playback_and_cache::{self, CatalogPlaybackTicketAdapter};
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId};
use hg_infra::edge_crypto::EdgeChannelScope;
use serde_json::json;

use super::support::{CatalogTestHarness, FakeCatalogUpstream};

#[tokio::test]
async fn playback_ticket_parses_streams_and_clear_cache_preserves_offline_library() {
    let playback_payload = json!({
        "status": 1,
        "msg": "ok",
        "req_time": 1_700_000_000_u64,
        "data": {
            "mode": 1,
            "timestamp": 1_700_000_000_u64,
            "manifest_method": "GET",
            "id": "101",
            "ep": 1,
            "cdn": {
                "m3u8Enabled": true,
                "otherEnabled": true,
                "authKey": "x-cdn-auth",
                "authValue": "secret-token",
                "app": "150"
            },
            "streams": {
                "hevc": "https://stream.example.com/101/1/hevc.m3u8?auth_key=1700000000-rand-0-abcd",
                "main": "https://stream.example.com/101/1/main.m3u8?auth_key=1700000000-rand-0-ef01",
                "preview": "https://stream.example.com/101/1/preview.m3u8"
            }
        }
    });

    let upstream = FakeCatalogUpstream::default()
        .with_json("/api/videos/101/episodes/1/playback", playback_payload);
    let harness = CatalogTestHarness::new(upstream).await;

    let key = EpisodeKey::new(
        SeriesId::try_from(101).expect("series 101"),
        EpisodeNumber::try_from(1).expect("ep 1"),
    );

    let port = CatalogPlaybackTicketAdapter::new(harness.deps.clone());
    let ticket = port
        .fetch_playback_ticket(key)
        .await
        .expect("playback ticket should parse");
    assert!(ticket.streams.hevc.is_some());
    assert!(ticket.streams.main.is_some());
    assert_eq!(ticket.cdn.auth_header_name.as_deref(), Some("x-cdn-auth"));
    assert_eq!(ticket.cdn.cdn_app.as_deref(), Some("150"));
    assert_eq!(ticket.issued_at_epoch_secs, 1_700_000_000);
    assert_eq!(ticket.expires_at_epoch_secs, 1_700_001_800);
    assert_eq!(
        harness
            .upstream
            .recorded_calls()
            .first()
            .expect("call")
            .scope,
        EdgeChannelScope::Playback
    );

    let cache_file = harness.deps.storage.catalog_cache_dir().join("cover.bin");
    tokio::fs::write(&cache_file, b"cached-cover")
        .await
        .expect("write catalog cache");

    let offline_dir = harness.deps.storage.library_episode_dir(key);
    tokio::fs::create_dir_all(&offline_dir)
        .await
        .expect("create offline dir");
    let offline_file = offline_dir.join("index.m3u8");
    tokio::fs::write(&offline_file, b"#EXTM3U\n")
        .await
        .expect("write offline file");

    sqlx::query(
        "INSERT INTO catalog_cache_entries (cache_key, payload_json, size_bytes, expires_at_epoch_secs, updated_at_epoch_secs)
         VALUES ('home', '{}', 2, 1700003600, 1700000000)",
    )
    .execute(harness.deps.pool())
    .await
    .expect("seed catalog cache entry");

    cmd_playback_and_cache::clear_catalog_cache(&harness.deps)
        .await
        .expect("clear catalog cache should succeed");

    assert!(!cache_file.exists());
    assert!(offline_file.exists());
    let remaining: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM catalog_cache_entries")
        .fetch_one(harness.deps.pool())
        .await
        .expect("count catalog cache");
    assert_eq!(remaining.0, 0);
}
