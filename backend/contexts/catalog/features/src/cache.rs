use moka::future::Cache;
use serde::Serialize;
use serde::de::DeserializeOwned;

const DEFAULT_CATALOG_MEMORY_CACHE_CAPACITY: u64 = 256;

#[derive(Debug, Clone)]
struct CachedPayload {
    payload_json: String,
    expires_at_epoch_secs: u64,
}

#[derive(Clone)]
pub struct CatalogMemoryCache {
    inner: Cache<String, CachedPayload>,
}

impl Default for CatalogMemoryCache {
    fn default() -> Self {
        Self::new(DEFAULT_CATALOG_MEMORY_CACHE_CAPACITY)
    }
}

impl CatalogMemoryCache {
    #[must_use]
    pub fn new(max_capacity: u64) -> Self {
        Self {
            inner: Cache::builder().max_capacity(max_capacity).build(),
        }
    }

    pub async fn read_json<T: DeserializeOwned>(
        &self,
        key: &str,
        now_epoch_secs: u64,
    ) -> Option<T> {
        let entry = self.inner.get(key).await?;
        if entry.expires_at_epoch_secs <= now_epoch_secs {
            self.inner.invalidate(key).await;
            return None;
        }
        serde_json::from_str::<T>(&entry.payload_json).ok()
    }

    pub async fn write_json<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        now_epoch_secs: u64,
        ttl_secs: u64,
    ) -> Result<(), serde_json::Error> {
        let payload_json = serde_json::to_string(value)?;
        let expires_at_epoch_secs = now_epoch_secs.saturating_add(ttl_secs);
        self.inner
            .insert(
                key.to_owned(),
                CachedPayload {
                    payload_json,
                    expires_at_epoch_secs,
                },
            )
            .await;
        Ok(())
    }

    pub async fn invalidate_all(&self) {
        self.inner.invalidate_all();
        self.inner.run_pending_tasks().await;
    }

    #[must_use]
    pub fn entry_count(&self) -> u64 {
        self.inner.entry_count()
    }

    pub async fn run_pending_tasks(&self) {
        self.inner.run_pending_tasks().await;
    }
}
