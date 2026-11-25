use std::sync::Arc;

use hg_infra::clock::Clock;
use hg_infra::db::LocalDatabase;
use hg_infra::storage::StorageLayout;
use sqlx::SqlitePool;

use crate::cache::CatalogMemoryCache;
use crate::upstream::CatalogUpstream;

pub mod cache;
pub mod cmd_playback_and_cache;
pub mod qry_discovery;
pub mod qry_series_detail;
pub mod upstream;

pub const CONTEXT_NAME: &str = "catalog";

#[derive(Clone)]
pub struct CatalogDeps {
    pub db: LocalDatabase,
    pub storage: Arc<StorageLayout>,
    pub clock: Arc<dyn Clock>,
    pub upstream: Arc<dyn CatalogUpstream>,
    pub cache: CatalogMemoryCache,
}

impl CatalogDeps {
    #[must_use]
    pub const fn pool(&self) -> &SqlitePool {
        self.db.pool()
    }
}
