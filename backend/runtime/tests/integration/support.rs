use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use hg_backend_contracts::catalog::CdnAuthHeaders;
use hg_backend_contracts::error::{PortError, PortFuture};
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId};
use hg_runtime::catalog::upstream::CatalogUpstream;
use hg_runtime::infra::clock::FixedClock;
use hg_runtime::infra::edge_crypto::EdgeChannelScope;
use hg_runtime::offline::fetcher::OfflineSegmentFetcher;
use hg_runtime::{AssembledContexts, BackendRuntime};
use parking_lot::Mutex;
use serde_json::{Value, json};
use tempfile::TempDir;

pub struct FakeUpstream {
    routes: HashMap<String, Value>,
    cover_bytes: Vec<u8>,
    offline: AtomicBool,
    calls: Mutex<Vec<(EdgeChannelScope, String)>>,
}

impl FakeUpstream {
    pub fn new(routes: HashMap<String, Value>, cover_bytes: Vec<u8>) -> Self {
        Self {
            routes,
            cover_bytes,
            offline: AtomicBool::new(false),
            calls: Mutex::new(Vec::new()),
        }
    }

    pub fn set_offline(&self, offline: bool) {
        self.offline.store(offline, Ordering::SeqCst);
    }

    pub fn call_count(&self) -> usize {
        self.calls.lock().len()
    }
}

impl CatalogUpstream for FakeUpstream {
    fn request_json<'a>(
        &'a self,
        scope: EdgeChannelScope,
        _method: &'a str,
        path_with_query: &'a str,
        _body: Option<&'a Value>,
    ) -> PortFuture<'a, Result<Value, PortError>> {
        self.calls.lock().push((scope, path_with_query.to_owned()));
        if self.offline.load(Ordering::SeqCst) {
            return Box::pin(async {
                Err(PortError::NetworkUnavailable {
                    detail: "模拟离线断网".to_owned(),
                })
            });
        }

        let found = self.routes.get(path_with_query).cloned();
        Box::pin(async move {
            found.ok_or_else(|| PortError::NotFound {
                resource: path_with_query.to_owned(),
            })
        })
    }

    fn fetch_raw_cover<'a>(
        &'a self,
        _cover_url: &'a str,
    ) -> PortFuture<'a, Result<Vec<u8>, PortError>> {
        if self.offline.load(Ordering::SeqCst) {
            return Box::pin(async {
                Err(PortError::NetworkUnavailable {
                    detail: "模拟离线断网".to_owned(),
                })
            });
        }
        let bytes = self.cover_bytes.clone();
        Box::pin(async move { Ok(bytes) })
    }
}

pub struct FakeFetcher {
    assets: HashMap<String, Vec<u8>>,
}

impl FakeFetcher {
    pub fn new(assets: HashMap<String, Vec<u8>>) -> Self {
        Self { assets }
    }
}

impl OfflineSegmentFetcher for FakeFetcher {
    fn fetch_bytes<'a>(
        &'a self,
        url: &'a str,
        _cdn: &'a CdnAuthHeaders,
    ) -> PortFuture<'a, Result<Vec<u8>, PortError>> {
        let found = self.assets.get(url).cloned();
        Box::pin(async move {
            found.ok_or_else(|| PortError::NotFound {
                resource: url.to_owned(),
            })
        })
    }
}

pub struct IntegrationHarness {
    pub _temp_dir: TempDir,
    pub runtime: BackendRuntime,
    pub contexts: AssembledContexts,
    pub upstream: Arc<FakeUpstream>,
    pub series_id: SeriesId,
    pub ep1: EpisodeKey,
    pub ep2: EpisodeKey,
}

impl IntegrationHarness {
    pub async fn start() -> Self {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let clock = Arc::new(FixedClock::new(1_700_000_000));
        let runtime = BackendRuntime::open_in_memory(
            &temp_dir.path().join("data"),
            &temp_dir.path().join("cache"),
            clock,
        )
        .await
        .expect("open runtime");

        let series_id = SeriesId::try_from(101).expect("series 101");
        let ep1 = EpisodeKey::new(series_id, EpisodeNumber::try_from(1).expect("ep 1"));
        let ep2 = EpisodeKey::new(series_id, EpisodeNumber::try_from(2).expect("ep 2"));

        let upstream = Arc::new(FakeUpstream::new(
            sample_upstream_routes(),
            b"\x89PNG\r\n\x1a\ncover-image".to_vec(),
        ));
        let fetcher = Arc::new(FakeFetcher::new(sample_segment_assets()));
        let contexts = runtime.assemble_contexts(Arc::clone(&upstream) as _, fetcher);

        Self {
            _temp_dir: temp_dir,
            runtime,
            contexts,
            upstream,
            series_id,
            ep1,
            ep2,
        }
    }

    pub async fn shutdown(self) {
        self.runtime.shutdown().await;
    }
}

fn sample_upstream_routes() -> HashMap<String, Value> {
    let mut routes = HashMap::new();
    routes.insert(
        "/api/home".to_owned(),
        json!({
            "status": 1,
            "msg": "ok",
            "data": {
                "banners": [],
                "featured": [{
                    "id": 101,
                    "title": "夜雨长灯",
                    "cover": "/covers/101.png",
                    "hot": 99000,
                    "score": 9.3,
                    "tags": ["悬疑"],
                    "desc": "全流程集成测试剧目",
                    "episode_count": 2,
                    "is_finished": true
                }],
                "cate_rec": []
            }
        }),
    );
    routes.insert(
        "/api/videos/101".to_owned(),
        json!({
            "status": 1,
            "msg": "ok",
            "data": {
                "id": 101,
                "title": "夜雨长灯",
                "description": "全流程集成测试剧目",
                "cover": "https://cdn.example.com/covers/101.png",
                "score": 9.3,
                "is_finished": true,
                "episode_count": 2,
                "total_episodes": 2,
                "tags": ["悬疑"],
                "counts": { "hot": 99000, "like": 120, "collect": 88, "comment": 16 },
                "episodes": [
                    { "ep_num": 1, "title": "第1集", "duration": 90, "status": 1 },
                    { "ep_num": 2, "title": "第2集", "duration": 95, "status": 1 }
                ]
            }
        }),
    );
    routes.insert(
        "/api/videos/101/episodes/1/playback".to_owned(),
        json!({
            "status": 1,
            "msg": "ok",
            "data": {
                "id": "101",
                "ep": 1,
                "timestamp": 1_700_000_000_u64,
                "cdn": {
                    "authKey": "x-cdn-auth",
                    "authValue": "tok-101-1",
                    "app": "150"
                },
                "streams": {
                    "hevc": "https://cdn.example.com/101/e1/index.m3u8?auth_key=1700000000-r-0-a"
                }
            }
        }),
    );
    routes.insert(
        "/api/videos/101/episodes/2/playback".to_owned(),
        json!({
            "status": 1,
            "msg": "ok",
            "data": {
                "id": "101",
                "ep": 2,
                "timestamp": 1_700_000_000_u64,
                "cdn": {
                    "authKey": "x-cdn-auth",
                    "authValue": "tok-101-2",
                    "app": "150"
                },
                "streams": {
                    "hevc": "https://cdn.example.com/101/e2/index.m3u8?auth_key=1700000000-r-0-b"
                }
            }
        }),
    );
    routes
}

fn sample_segment_assets() -> HashMap<String, Vec<u8>> {
    let mut assets = HashMap::new();
    for (ep, seg_payload) in [(1_u32, b"EP1-SEG-BYTES".as_slice()), (2, b"EP2-SEG-BYTES")] {
        assets.insert(
            format!(
                "https://cdn.example.com/101/e{ep}/index.m3u8?auth_key=1700000000-r-0-{}",
                if ep == 1 { "a" } else { "b" }
            ),
            b"#EXTM3U\n#EXT-X-VERSION:3\n#EXTINF:5.0,\nseg0.ts\n#EXT-X-ENDLIST\n".to_vec(),
        );
        assets.insert(
            format!("https://cdn.example.com/101/e{ep}/seg0.ts"),
            seg_payload.to_vec(),
        );
    }
    assets
}
