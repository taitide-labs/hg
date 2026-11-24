use std::path::{Path, PathBuf};
use std::sync::Arc;

use hg_backend_contracts::catalog::CatalogMetadataPort;
use hg_infra::clock::Clock;
use hg_infra::db::LocalDatabase;
use hg_infra::error::InfraError;
use hg_infra::loopback_proxy::LoopbackProxyHandle;
use hg_infra::storage::StorageLayout;
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
        let loopback = LoopbackProxyHandle::start(Arc::clone(&storage)).await?;

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
        let loopback = LoopbackProxyHandle::start(Arc::clone(&storage)).await?;

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
    pub fn viewing_deps(&self, catalog_metadata: Arc<dyn CatalogMetadataPort>) -> ViewingDeps {
        ViewingDeps {
            db: self.db.clone(),
            clock: Arc::clone(&self.clock),
            catalog_metadata,
        }
    }

    pub async fn shutdown(self) {
        self.loopback.shutdown();
        self.db.close().await;
    }
}
