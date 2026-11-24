use std::sync::Arc;

use hg_backend_contracts::catalog::CatalogMetadataPort;
use hg_infra::clock::Clock;
use hg_infra::db::LocalDatabase;

pub mod cmd_record_watch_progress;
pub mod qry_watch_history;

pub const CONTEXT_NAME: &str = "viewing";

#[derive(Clone)]
pub struct ViewingDeps {
    pub db: LocalDatabase,
    pub clock: Arc<dyn Clock>,
    pub catalog_metadata: Arc<dyn CatalogMetadataPort>,
}
