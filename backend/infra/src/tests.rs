use std::sync::Arc;

use aes::Aes128;
use cbc::cipher::block_padding::Pkcs7;
use cbc::cipher::{BlockModeEncrypt as _, KeyIvInit as _};
use hg_backend_contracts::viewing::PlaybackHaltPort as _;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId};
use tempfile::tempdir;

use crate::clock::{Clock as _, FixedClock};
use crate::db::LocalDatabase;
use crate::edge_crypto::{
    EdgeChannelScope, build_aad, canonicalize_public_path, decrypt_cover_bytes,
    decrypt_envelope_payload, derive_session_keys, encrypt_envelope_payload,
};
use crate::loopback_proxy::LoopbackProxyHandle;
use crate::storage::StorageLayout;

type Aes128CbcEncryptor = cbc::Encryptor<Aes128>;

#[tokio::test]
async fn sqlite_migration_creates_all_tables_and_seed_rows() {
    let db = LocalDatabase::open_in_memory()
        .await
        .expect("in-memory sqlite should open and migrate");

    let policy_row: (String, i64, i64) = sqlx::query_as(
        "SELECT preferred_profile, wifi_only, max_concurrent_tasks FROM offline_download_policy WHERE id = 1",
    )
    .fetch_one(db.pool())
    .await
    .expect("download policy seed row should exist");
    assert_eq!(policy_row, ("hevc".to_owned(), 1, 2));

    let pref_row: (String, i64, i64) = sqlx::query_as(
        "SELECT preferred_profile, playback_cache_limit_mb, default_rate_hundredths FROM viewing_playback_preference WHERE id = 1",
    )
    .fetch_one(db.pool())
    .await
    .expect("playback preference seed row should exist");
    assert_eq!(pref_row, ("hevc".to_owned(), 512, 100));

    db.close().await;
}

#[tokio::test]
async fn storage_layout_physically_isolates_cache_from_offline_library() {
    let root = tempdir().expect("tempdir should create");
    let data_root = root.path().join("data");
    let cache_root = root.path().join("cache");

    assert!(StorageLayout::new(&cache_root, &cache_root).is_err());

    let layout =
        StorageLayout::new(&data_root, &cache_root).expect("isolated roots should succeed");
    layout.ensure_dirs().await.expect("dirs should create");

    let key = EpisodeKey::new(
        SeriesId::try_from(7).expect("valid series id"),
        EpisodeNumber::try_from(2).expect("valid ep num"),
    );
    let staging_dir = layout.staging_episode_dir(key);
    tokio::fs::create_dir_all(&staging_dir)
        .await
        .expect("staging dir should create");
    tokio::fs::write(staging_dir.join("index.m3u8"), b"#EXTM3U\n")
        .await
        .expect("staging playlist should write");

    let committed = layout
        .commit_staging_episode(key)
        .await
        .expect("commit should rename atomically");
    assert!(committed.join("index.m3u8").exists());

    tokio::fs::write(layout.catalog_cache_dir().join("feed.json"), b"{}")
        .await
        .expect("catalog cache file should write");
    tokio::fs::write(layout.playback_cache_dir().join("seg0.ts"), b"12345")
        .await
        .expect("playback cache file should write");

    let freed_catalog = layout
        .clear_catalog_cache()
        .await
        .expect("clear catalog cache");
    let freed_playback = layout
        .clear_playback_cache()
        .await
        .expect("clear playback cache");
    assert_eq!(freed_catalog, 2);
    assert_eq!(freed_playback, 5);
    assert!(committed.join("index.m3u8").exists());

    assert!(layout.resolve_offline_library_path("../escape").is_err());
    layout
        .remove_offline_rel_dir_with_retry("s7/e2")
        .await
        .expect("remove offline dir should succeed");
    assert!(!committed.exists());
}

#[test]
fn cover_decryption_passes_plain_images_and_decrypts_aes_cbc() {
    let jpeg = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
    assert_eq!(
        decrypt_cover_bytes(&jpeg).expect("plain jpeg should pass through"),
        jpeg
    );

    let key: [u8; 16] = *b"f5d965df75336270";
    let iv: [u8; 16] = *b"97b60394abc2fbe1";
    let plain = b"\x89PNG\r\n\x1a\nfake-cover-bytes";
    let encrypted =
        Aes128CbcEncryptor::new(&key.into(), &iv.into()).encrypt_padded_vec::<Pkcs7>(plain);

    let decrypted = decrypt_cover_bytes(&encrypted).expect("encrypted cover should decrypt");
    assert_eq!(decrypted, plain);
}

#[test]
fn edge_crypto_roundtrips_and_rejects_tampered_aad() {
    let shared_secret = [0x42_u8; 32];
    let salt = [0x11_u8; 32];
    let (req_key, res_key, info) = derive_session_keys(
        &shared_secret,
        &salt,
        EdgeChannelScope::Playback,
        "site-a",
        "kid-1",
        "sid-9",
    )
    .expect("hkdf derivation should succeed");
    assert_ne!(req_key, res_key);

    let aad = build_aad(&info, "req", 1, 1_700_000_000_000);
    let ciphertext = encrypt_envelope_payload(&req_key, 1, &aad, b"hello-drama")
        .expect("encrypt should succeed");
    let plaintext =
        decrypt_envelope_payload(&req_key, 1, &aad, &ciphertext).expect("decrypt should succeed");
    assert_eq!(plaintext, b"hello-drama");

    let tampered_aad = build_aad(&info, "req", 2, 1_700_000_000_000);
    assert!(decrypt_envelope_payload(&req_key, 1, &tampered_aad, &ciphertext).is_err());

    assert_eq!(
        canonicalize_public_path("/api/home?b=2&a=1"),
        "/api/home?a=1&b=2"
    );
}

#[tokio::test]
async fn loopback_proxy_serves_offline_asset_and_halts_with_gone() {
    let root = tempdir().expect("tempdir should create");
    let layout = Arc::new(
        StorageLayout::new(&root.path().join("data"), &root.path().join("cache"))
            .expect("storage layout"),
    );
    layout.ensure_dirs().await.expect("ensure dirs");

    let key = EpisodeKey::new(
        SeriesId::try_from(42).expect("valid series id"),
        EpisodeNumber::try_from(3).expect("valid ep num"),
    );
    let ep_dir = layout.library_episode_dir(key);
    tokio::fs::create_dir_all(&ep_dir)
        .await
        .expect("episode dir");
    tokio::fs::write(ep_dir.join("index.m3u8"), b"#EXTM3U\n#EXT-X-ENDLIST\n")
        .await
        .expect("playlist write");

    let proxy = LoopbackProxyHandle::start(Arc::clone(&layout))
        .await
        .expect("loopback proxy should start");
    let url = proxy
        .register_offline_episode(key, "s42/e3/index.m3u8")
        .expect("register offline episode");

    let client = reqwest::Client::new();
    let ok_resp = client.get(&url).send().await.expect("GET playlist");
    assert_eq!(ok_resp.status(), reqwest::StatusCode::OK);
    let body = ok_resp.text().await.expect("playlist body");
    assert!(body.contains("#EXTM3U"));

    proxy
        .halt_episode(key)
        .await
        .expect("halt episode should succeed");
    let gone_resp = client.get(&url).send().await.expect("GET after halt");
    assert_eq!(gone_resp.status(), reqwest::StatusCode::GONE);

    proxy.shutdown();
}

async fn spawn_fake_cdn_server() -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    use bytes::Bytes;
    use http_body_util::Full;
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper::{Request, Response, StatusCode};
    use hyper_util::rt::TokioIo;
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind fake cdn");
    let cdn_addr = listener.local_addr().expect("cdn local addr");

    let handle = tokio::spawn(async move {
        for _ in 0..2 {
            let Ok((stream, _)) = listener.accept().await else {
                break;
            };
            tokio::spawn(async move {
                let io = TokioIo::new(stream);
                let svc = service_fn(|req: Request<hyper::body::Incoming>| async move {
                    let has_auth = req
                        .headers()
                        .get("x-cdn-auth")
                        .and_then(|v| v.to_str().ok())
                        == Some("token-123");
                    let has_app =
                        req.headers().get("x-cdn-app").and_then(|v| v.to_str().ok()) == Some("150");
                    if !has_auth || !has_app {
                        return Ok::<_, std::convert::Infallible>(
                            Response::builder()
                                .status(StatusCode::FORBIDDEN)
                                .body(Full::new(Bytes::new()))
                                .expect("403"),
                        );
                    }
                    let resp = match req.uri().path() {
                        "/v/index.m3u8" => Response::builder()
                            .status(StatusCode::OK)
                            .header("content-type", "application/vnd.apple.mpegurl")
                            .body(Full::new(Bytes::from(
                                "#EXTM3U\n#EXTINF:5.0,\nseg0.ts\n#EXT-X-ENDLIST\n",
                            )))
                            .expect("manifest"),
                        "/v/seg0.ts" => Response::builder()
                            .status(StatusCode::OK)
                            .header("content-type", "video/mp2t")
                            .body(Full::new(Bytes::from_static(b"TS-DATA")))
                            .expect("seg0"),
                        _ => Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .body(Full::new(Bytes::new()))
                            .expect("404"),
                    };
                    Ok(resp)
                });
                drop(http1::Builder::new().serve_connection(io, svc).await);
            });
        }
    });
    (cdn_addr, handle)
}

#[tokio::test]
async fn loopback_proxy_forwards_online_hls_with_cdn_headers_and_rewrites_uris() {
    let (cdn_addr, cdn_task) = spawn_fake_cdn_server().await;
    let root = tempdir().expect("tempdir should create");
    let layout = Arc::new(
        StorageLayout::new(&root.path().join("data"), &root.path().join("cache"))
            .expect("storage layout"),
    );
    layout.ensure_dirs().await.expect("ensure dirs");
    let proxy = LoopbackProxyHandle::start(layout)
        .await
        .expect("loopback proxy");
    let key = EpisodeKey::new(
        SeriesId::try_from(7).expect("series 7"),
        EpisodeNumber::try_from(1).expect("ep 1"),
    );
    let proxied = proxy
        .register_online_stream(
            key,
            &format!("http://{cdn_addr}/v/index.m3u8"),
            &[
                ("x-cdn-auth".to_owned(), "token-123".to_owned()),
                ("x-cdn-app".to_owned(), "150".to_owned()),
            ],
        )
        .expect("register online stream");

    let client = reqwest::Client::new();
    let manifest_body = client
        .get(&proxied)
        .send()
        .await
        .expect("get manifest")
        .text()
        .await
        .expect("manifest text");
    let seg_path = manifest_body
        .lines()
        .find(|line| line.starts_with("/online/"))
        .expect("rewritten segment line");
    let seg_url = format!("{}{seg_path}", proxy.base_url());
    let seg_bytes = client
        .get(&seg_url)
        .send()
        .await
        .expect("get seg0")
        .bytes()
        .await
        .expect("seg0 bytes");
    assert_eq!(seg_bytes.as_ref(), b"TS-DATA");

    proxy.halt_episode(key).await.expect("halt online");
    let after_halt = client.get(&proxied).send().await.expect("after halt");
    assert_eq!(after_halt.status(), reqwest::StatusCode::GONE);

    proxy.shutdown();
    drop(cdn_task);
}

#[test]
fn fixed_clock_advances_deterministically() {
    let clock = FixedClock::new(1_700_000_000);
    assert_eq!(clock.now_epoch_secs(), 1_700_000_000);
    clock.advance_secs(60);
    assert_eq!(clock.now_epoch_secs(), 1_700_000_060);
    assert_eq!(clock.now_epoch_millis(), 1_700_000_060_000);
}
