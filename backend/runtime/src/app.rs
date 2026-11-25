use std::path::{Path, PathBuf};
use std::sync::Arc;

use hg_catalog_features::CatalogDeps;
use hg_catalog_features::cmd_playback_and_cache::CatalogPlaybackTicketAdapter;
use hg_catalog_features::qry_series_detail::CatalogMetadataAdapter;
use hg_catalog_features::upstream::CatalogUpstream;
use hg_infra::clock::Clock;
use hg_infra::db::LocalDatabase;
use hg_infra::error::InfraError;
use hg_infra::loopback_proxy::LoopbackProxyHandle;
use hg_infra::network::{NetworkMonitor, SystemNetworkMonitor};
use hg_infra::storage::StorageLayout;
use hg_offline_features::OfflineDeps;
use hg_offline_features::fetcher::OfflineSegmentFetcher;
use hg_offline_features::qry_offline_library::OfflineAssetQueryAdapter;
use hg_viewing_features::ViewingDeps;

pub const REGISTERED_CONTEXTS: [&str; 3] = [
    hg_catalog_features::CONTEXT_NAME,
    hg_offline_features::CONTEXT_NAME,
    hg_viewing_features::CONTEXT_NAME,
];

#[must_use]
pub const fn runtime_name() -> &'static str {
    "hg-runtime"
}

#[derive(Debug, Clone)]
pub struct BackendRuntimeConfig {
    pub db_path: PathBuf,
    pub data_root: PathBuf,
    pub cache_root: PathBuf,
}

#[derive(Clone)]
pub struct AssembledContexts {
    pub catalog: CatalogDeps,
    pub offline: OfflineDeps,
    pub viewing: ViewingDeps,
}

#[derive(Clone)]
pub struct BackendRuntime {
    db: LocalDatabase,
    storage: Arc<StorageLayout>,
    clock: Arc<dyn Clock>,
    loopback: LoopbackProxyHandle,
}

impl BackendRuntime {
    pub async fn open(
        config: &BackendRuntimeConfig,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, InfraError> {
        let storage = Arc::new(StorageLayout::new(&config.data_root, &config.cache_root)?);
        storage.ensure_dirs().await?;
        let db = LocalDatabase::open_file(&config.db_path).await?;
        let loopback = LoopbackProxyHandle::start_with_cache(
            Arc::clone(&storage),
            db.clone(),
            Arc::clone(&clock),
        )
        .await?;

        Ok(Self {
            db,
            storage,
            clock,
            loopback,
        })
    }

    pub async fn open_in_memory(
        data_root: &Path,
        cache_root: &Path,
        clock: Arc<dyn Clock>,
    ) -> Result<Self, InfraError> {
        let storage = Arc::new(StorageLayout::new(data_root, cache_root)?);
        storage.ensure_dirs().await?;
        let db = LocalDatabase::open_in_memory().await?;
        let loopback = LoopbackProxyHandle::start_with_cache(
            Arc::clone(&storage),
            db.clone(),
            Arc::clone(&clock),
        )
        .await?;

        Ok(Self {
            db,
            storage,
            clock,
            loopback,
        })
    }

    #[must_use]
    pub const fn db(&self) -> &LocalDatabase {
        &self.db
    }

    #[must_use]
    pub const fn storage(&self) -> &Arc<StorageLayout> {
        &self.storage
    }

    #[must_use]
    pub const fn clock(&self) -> &Arc<dyn Clock> {
        &self.clock
    }

    #[must_use]
    pub const fn loopback(&self) -> &LoopbackProxyHandle {
        &self.loopback
    }

    #[must_use]
    pub fn assemble_contexts(
        &self,
        upstream: Arc<dyn CatalogUpstream>,
        segment_fetcher: Arc<dyn OfflineSegmentFetcher>,
    ) -> AssembledContexts {
        let catalog = CatalogDeps {
            db: self.db.clone(),
            storage: Arc::clone(&self.storage),
            clock: Arc::clone(&self.clock),
            upstream,
            cache: hg_catalog_features::cache::CatalogMemoryCache::default(),
        };

        let catalog_metadata = Arc::new(CatalogMetadataAdapter::new(catalog.clone()));
        let catalog_tickets = Arc::new(CatalogPlaybackTicketAdapter::new(catalog.clone()));
        let playback_halt = Arc::new(self.loopback.clone());

        let offline = OfflineDeps {
            db: self.db.clone(),
            storage: Arc::clone(&self.storage),
            clock: Arc::clone(&self.clock),
            network: Arc::new(SystemNetworkMonitor) as Arc<dyn NetworkMonitor>,
            catalog_metadata: Arc::clone(&catalog_metadata) as _,
            catalog_tickets: Arc::clone(&catalog_tickets) as _,
            playback_halt,
            segment_fetcher,
        };

        let offline_assets = Arc::new(OfflineAssetQueryAdapter::new(offline.clone()));

        let viewing = ViewingDeps {
            db: self.db.clone(),
            storage: Arc::clone(&self.storage),
            clock: Arc::clone(&self.clock),
            catalog_metadata,
            catalog_tickets,
            offline_assets,
            loopback: self.loopback.clone(),
        };

        AssembledContexts {
            catalog,
            offline,
            viewing,
        }
    }

    pub async fn shutdown(self) {
        self.loopback.shutdown();
        self.db.close().await;
    }
}
