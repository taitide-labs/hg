use std::sync::Arc;

use hg_backend_contracts::catalog::{CatalogMetadataPort, CatalogPlaybackTicketPort};
use hg_backend_contracts::viewing::PlaybackHaltPort;
use hg_infra::clock::Clock;
use hg_infra::db::LocalDatabase;
use hg_infra::storage::StorageLayout;
use sqlx::SqlitePool;

use crate::fetcher::OfflineSegmentFetcher;
use crate::flags::OfflineFeatureFlags;

pub mod cmd_download_queue;
pub mod cmd_offline_library;
pub mod fetcher;
pub mod flags;
pub mod qry_download_queue;
pub mod qry_offline_library;

pub const CONTEXT_NAME: &str = "offline";

#[derive(Clone)]
pub struct OfflineDeps {
    pub db: LocalDatabase,
    pub storage: Arc<StorageLayout>,
    pub clock: Arc<dyn Clock>,
    pub catalog_metadata: Arc<dyn CatalogMetadataPort>,
    pub catalog_tickets: Arc<dyn CatalogPlaybackTicketPort>,
    pub playback_halt: Arc<dyn PlaybackHaltPort>,
    pub segment_fetcher: Arc<dyn OfflineSegmentFetcher>,
    pub flags: OfflineFeatureFlags,
}

impl OfflineDeps {
    #[must_use]
    pub const fn pool(&self) -> &SqlitePool {
        self.db.pool()
    }
}
