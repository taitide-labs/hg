use std::collections::HashMap;
use std::sync::Arc;

use hg_backend_contracts::error::{PortError, PortFuture};
use hg_catalog_features::CatalogDeps;
use hg_catalog_features::upstream::CatalogUpstream;
use hg_infra::clock::FixedClock;
use hg_infra::db::LocalDatabase;
use hg_infra::edge_crypto::EdgeChannelScope;
use hg_infra::storage::StorageLayout;
use parking_lot::Mutex;
use serde_json::Value;
use tempfile::TempDir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedUpstreamCall {
    pub scope: EdgeChannelScope,
    pub method: String,
    pub path: String,
}

#[derive(Default)]
pub struct FakeCatalogUpstream {
    json_routes: HashMap<String, Value>,
    cover_routes: HashMap<String, Vec<u8>>,
    calls: Mutex<Vec<RecordedUpstreamCall>>,
    cover_calls: Mutex<Vec<String>>,
}

impl FakeCatalogUpstream {
    pub fn with_json(mut self, path_prefix: impl Into<String>, response: Value) -> Self {
        self.json_routes.insert(path_prefix.into(), response);
        self
    }

    pub fn with_cover(mut self, cover_url: impl Into<String>, raw_bytes: Vec<u8>) -> Self {
        self.cover_routes.insert(cover_url.into(), raw_bytes);
        self
    }

    pub fn recorded_calls(&self) -> Vec<RecordedUpstreamCall> {
        self.calls.lock().clone()
    }

    pub fn recorded_cover_calls(&self) -> Vec<String> {
        self.cover_calls.lock().clone()
    }
}

impl CatalogUpstream for FakeCatalogUpstream {
    fn request_json<'a>(
        &'a self,
        scope: EdgeChannelScope,
        method: &'a str,
        path_with_query: &'a str,
        _body: Option<&'a Value>,
    ) -> PortFuture<'a, Result<Value, PortError>> {
        self.calls.lock().push(RecordedUpstreamCall {
            scope,
            method: method.to_owned(),
            path: path_with_query.to_owned(),
        });

        let found = self
            .json_routes
            .get(path_with_query)
            .or_else(|| {
                let path_only = path_with_query
                    .split_once('?')
                    .map_or(path_with_query, |(p, _)| p);
                self.json_routes.get(path_only)
            })
            .cloned();

        Box::pin(async move {
            found.ok_or_else(|| PortError::NotFound {
                resource: path_with_query.to_owned(),
            })
        })
    }

    fn fetch_raw_cover<'a>(
        &'a self,
        cover_url: &'a str,
    ) -> PortFuture<'a, Result<Vec<u8>, PortError>> {
        self.cover_calls.lock().push(cover_url.to_owned());
        let found = self.cover_routes.get(cover_url).cloned();
        Box::pin(async move {
            found.ok_or_else(|| PortError::NotFound {
                resource: cover_url.to_owned(),
            })
        })
    }
}

pub struct CatalogTestHarness {
    pub _temp_dir: TempDir,
    pub _clock: Arc<FixedClock>,
    pub upstream: Arc<FakeCatalogUpstream>,
    pub deps: CatalogDeps,
}

impl CatalogTestHarness {
    pub async fn new(upstream: FakeCatalogUpstream) -> Self {
        let temp_dir = tempfile::tempdir().expect("tempdir should create");
        let storage = Arc::new(
            StorageLayout::new(
                &temp_dir.path().join("data"),
                &temp_dir.path().join("cache"),
            )
            .expect("storage layout"),
        );
        storage.ensure_dirs().await.expect("ensure dirs");
        let db = LocalDatabase::open_in_memory()
            .await
            .expect("in-memory sqlite");
        let clock = Arc::new(FixedClock::new(1_700_000_000));
        let upstream = Arc::new(upstream);
        let deps = CatalogDeps {
            db,
            storage,
            clock: Arc::clone(&clock) as _,
            upstream: Arc::clone(&upstream) as _,
        };
        Self {
            _temp_dir: temp_dir,
            _clock: clock,
            upstream,
            deps,
        }
    }
}
