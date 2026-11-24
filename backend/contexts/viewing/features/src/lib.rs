use std::sync::Arc;

use hg_backend_contracts::catalog::{CatalogMetadataPort, CatalogPlaybackTicketPort};
use hg_backend_contracts::offline::OfflineAssetQueryPort;
use hg_infra::clock::Clock;
use hg_infra::db::LocalDatabase;
use hg_infra::loopback_proxy::LoopbackProxyHandle;
use hg_infra::storage::StorageLayout;
use sqlx::SqlitePool;

pub mod cmd_playback_session;
pub mod cmd_record_watch_progress;
pub mod cmd_watchlist_and_settings;
pub mod qry_watch_history;
pub mod qry_watchlist_and_settings;

pub const CONTEXT_NAME: &str = "viewing";

#[derive(Clone)]
pub struct ViewingDeps {
    pub db: LocalDatabase,
    pub storage: Arc<StorageLayout>,
    pub clock: Arc<dyn Clock>,
    pub catalog_metadata: Arc<dyn CatalogMetadataPort>,
    pub catalog_tickets: Arc<dyn CatalogPlaybackTicketPort>,
    pub offline_assets: Arc<dyn OfflineAssetQueryPort>,
    pub loopback: LoopbackProxyHandle,
}

impl ViewingDeps {
    #[must_use]
    pub const fn pool(&self) -> &SqlitePool {
        self.db.pool()
    }
}
