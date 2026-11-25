use std::sync::Arc;

use hg_backend_contracts::error::PortError;
use hg_infra::cover_cache::CoverCacheStore;
use hg_infra::edge_crypto::decrypt_cover_bytes;

use crate::CatalogDeps;

#[cfg(test)]
pub fn build_cover_rel_path(cover_url: &str) -> String {
    hg_infra::cover_cache::build_cover_rel_path(cover_url)
}

pub async fn fetch_or_cache_cover(
    deps: &CatalogDeps,
    cover_url: &str,
) -> Result<Vec<u8>, PortError> {
    let trimmed_url = cover_url.trim();
    if trimmed_url.is_empty() {
        return Err(PortError::InvalidState {
            detail: "封面地址不能为空".to_owned(),
        });
    }

    let store = CoverCacheStore::new(
        deps.db.clone(),
        Arc::clone(&deps.storage),
        Arc::clone(&deps.clock),
    );
    if let Some(cached) = store.read(trimmed_url).await.map_err(PortError::from)? {
        return Ok(cached);
    }

    let raw_bytes = deps.upstream.fetch_raw_cover(trimmed_url).await?;
    let decrypted = decrypt_cover_bytes(&raw_bytes).map_err(PortError::from)?;
    store
        .store_decrypted(trimmed_url, &decrypted)
        .await
        .map_err(PortError::from)
}
