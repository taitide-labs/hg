use std::sync::Arc;

use hg_backend_contracts::catalog::CatalogPlaybackTicketPort as _;
use hg_backend_contracts::error::{PortError, PortFuture};
use hg_catalog_domain::error::CatalogError;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId};
use hg_infra::clock::FixedClock;
use hg_infra::db::LocalDatabase;
use hg_infra::edge_crypto::EdgeChannelScope;
use hg_infra::storage::StorageLayout;
use parking_lot::Mutex;
use serde_json::{Value, json};

use crate::CatalogDeps;
use crate::flags::CatalogFeatureFlags;
use crate::upstream::CatalogUpstream;

use super::handler::clear_catalog_cache;
use super::port::CatalogPlaybackTicketAdapter;
use super::workflow::{
    parse_playback_ticket, playback_request_path, resolve_ticket_expiry_epoch_secs,
};

#[derive(Default)]
struct RecordingUpstream {
    bodies: Mutex<Vec<Option<Value>>>,
}

impl CatalogUpstream for RecordingUpstream {
    fn request_json<'a>(
        &'a self,
        _scope: EdgeChannelScope,
        _method: &'a str,
        _path_with_query: &'a str,
        body: Option<&'a Value>,
    ) -> PortFuture<'a, Result<Value, PortError>> {
        self.bodies.lock().push(body.cloned());
        let response = json!({
            "status": 1,
            "data": {
                "timestamp": 1_700_000_000_u64,
                "streams": {
                    "main": "https://stream.example.com/101/2/main.m3u8?auth_key=1700000000-r-0-a"
                }
            }
        });
        Box::pin(async move { Ok(response) })
    }

    fn fetch_raw_cover<'a>(
        &'a self,
        cover_url: &'a str,
    ) -> PortFuture<'a, Result<Vec<u8>, PortError>> {
        let missing = cover_url.to_owned();
        Box::pin(async move { Err(PortError::NotFound { resource: missing }) })
    }
}

fn sample_key() -> EpisodeKey {
    EpisodeKey::new(
        SeriesId::try_from(101).expect("series 101"),
        EpisodeNumber::try_from(2).expect("ep 2"),
    )
}

#[test]
fn builds_playback_request_path() {
    assert_eq!(
        playback_request_path(sample_key()),
        "/api/videos/101/episodes/2/playback"
    );
}

#[test]
fn parses_full_playback_ticket_with_auth_key_and_cdn_headers() {
    let payload = json!({
        "status": 1,
        "req_time": 1_700_000_050_u64,
        "data": {
            "timestamp": 1_700_000_040_u64,
            "cdn": {
                "referer": "https://cdn-ref.example.com/",
                "userAgent": "hg-player/1.0",
                "authKey": "x-cdn-auth",
                "authValue": "secret-token",
                "app": 150
            },
            "streams": {
                "hevc": "https://stream.example.com/101/2/hevc.m3u8?auth_key=1700000000-rand-0-abcd",
                "main": "https://stream.example.com/101/2/main.m3u8?auth_key=1700000000-rand-0-ef01",
                "preview": ""
            }
        }
    });

    let ticket = parse_playback_ticket(sample_key(), &payload, 1_700_000_999)
        .expect("full ticket should parse");

    assert_eq!(ticket.key, sample_key());
    assert_eq!(
        ticket.streams.hevc.as_deref(),
        Some("https://stream.example.com/101/2/hevc.m3u8?auth_key=1700000000-rand-0-abcd")
    );
    assert_eq!(
        ticket.streams.main.as_deref(),
        Some("https://stream.example.com/101/2/main.m3u8?auth_key=1700000000-rand-0-ef01")
    );
    assert!(ticket.streams.preview.is_none());
    assert_eq!(ticket.cdn.referer, "https://cdn-ref.example.com/");
    assert_eq!(ticket.cdn.user_agent, "hg-player/1.0");
    assert_eq!(ticket.cdn.auth_header_name.as_deref(), Some("x-cdn-auth"));
    assert_eq!(
        ticket.cdn.auth_header_value.as_deref(),
        Some("secret-token")
    );
    assert_eq!(ticket.cdn.cdn_app.as_deref(), Some("150"));
    assert_eq!(ticket.issued_at_epoch_secs, 1_700_000_000);
    assert_eq!(ticket.expires_at_epoch_secs, 1_700_001_800);
}

#[test]
fn falls_back_to_payload_timestamp_when_stream_lacks_auth_key() {
    let payload = json!({
        "status": 1,
        "data": {
            "timestamp": 1_700_000_123_u64,
            "streams": {
                "preview": "https://stream.example.com/101/2/preview.m3u8"
            }
        }
    });

    let ticket = parse_playback_ticket(sample_key(), &payload, 1_700_000_999)
        .expect("preview ticket should parse");

    assert!(ticket.streams.hevc.is_none());
    assert!(ticket.streams.main.is_none());
    assert_eq!(
        ticket.streams.preview.as_deref(),
        Some("https://stream.example.com/101/2/preview.m3u8")
    );
    assert_eq!(ticket.issued_at_epoch_secs, 1_700_000_123);
    assert_eq!(ticket.expires_at_epoch_secs, 1_700_001_923);
    assert_eq!(
        resolve_ticket_expiry_epoch_secs("https://stream.example.com/preview.m3u8", 1_700_000_123),
        1_700_001_923
    );
}

#[test]
fn rejects_empty_stream_list_and_malformed_payloads() {
    let empty_streams = json!({
        "status": 1,
        "data": {
            "streams": {
                "hevc": "  ",
                "main": null
            }
        }
    });
    let err = parse_playback_ticket(sample_key(), &empty_streams, 1_700_000_000)
        .expect_err("empty streams should fail");
    assert_eq!(err, CatalogError::EmptyStreamList);

    let missing_data = json!({ "status": 1 });
    let err = parse_playback_ticket(sample_key(), &missing_data, 1_700_000_000)
        .expect_err("missing data should fail");
    assert!(matches!(err, CatalogError::UpstreamPayloadMalformed { .. }));
}

#[tokio::test]
async fn sends_empty_json_object_body_and_clears_cover_cache_table() {
    let temp = tempfile::tempdir().expect("tempdir");
    let storage = Arc::new(
        StorageLayout::new(&temp.path().join("data"), &temp.path().join("cache"))
            .expect("storage layout"),
    );
    storage.ensure_dirs().await.expect("ensure dirs");
    let db = LocalDatabase::open_in_memory().await.expect("sqlite");
    let upstream = Arc::new(RecordingUpstream::default());
    let deps = CatalogDeps {
        db,
        storage: Arc::clone(&storage),
        clock: Arc::new(FixedClock::new(1_700_000_000)),
        upstream: Arc::clone(&upstream) as _,
        flags: CatalogFeatureFlags {
            playback_and_cache: true,
            ..CatalogFeatureFlags::default()
        },
    };

    let port = CatalogPlaybackTicketAdapter::new(deps.clone());
    let _ticket = port
        .fetch_playback_ticket(sample_key())
        .await
        .expect("ticket should succeed");
    assert_eq!(upstream.bodies.lock().as_slice(), &[Some(json!({}))]);

    sqlx::query(
        "INSERT INTO catalog_cover_cache (cover_url, local_rel_path, size_bytes, accessed_at_epoch_secs)
         VALUES ('/covers/101.enc', '101.png', 16, 1700000000)",
    )
    .execute(deps.pool())
    .await
    .expect("seed cover cache");

    clear_catalog_cache(&deps)
        .await
        .expect("clear catalog cache should succeed");

    let remaining_covers: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM catalog_cover_cache")
        .fetch_one(deps.pool())
        .await
        .expect("count cover cache");
    assert_eq!(remaining_covers.0, 0);
}
