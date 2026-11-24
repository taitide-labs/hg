use std::collections::HashMap;
use std::sync::Arc;

use hg_backend_contracts::catalog::{
    CatalogMetadataPort, CatalogPlaybackTicketPort, CdnAuthHeaders, EpisodeSnapshot,
    PlaybackTicket, SeriesSnapshot, StreamEndpoints,
};
use hg_backend_contracts::error::{PortError, PortFuture};
use hg_backend_contracts::offline::{CommittedOfflineAsset, OfflineAssetQueryPort};
use hg_contracts::ids::{EpisodeKey, SeriesId};
use hg_infra::clock::FixedClock;
use hg_infra::db::LocalDatabase;
use hg_infra::loopback_proxy::LoopbackProxyHandle;
use hg_infra::storage::StorageLayout;
use hg_viewing_features::ViewingDeps;
use hg_viewing_features::flags::ViewingFeatureFlags;
use parking_lot::Mutex;
use tempfile::TempDir;

#[derive(Default)]
pub struct StubViewingCatalog {
    series: HashMap<SeriesId, SeriesSnapshot>,
    tickets: HashMap<EpisodeKey, PlaybackTicket>,
    ticket_calls: Mutex<Vec<EpisodeKey>>,
}

impl StubViewingCatalog {
    pub fn with_series(mut self, items: impl IntoIterator<Item = SeriesSnapshot>) -> Self {
        for item in items {
            self.series.insert(item.series_id, item);
        }
        self
    }

    pub fn with_ticket(mut self, ticket: PlaybackTicket) -> Self {
        self.tickets.insert(ticket.key, ticket);
        self
    }

    pub fn recorded_ticket_calls(&self) -> Vec<EpisodeKey> {
        self.ticket_calls.lock().clone()
    }
}

impl CatalogMetadataPort for StubViewingCatalog {
    fn fetch_series_snapshot(
        &self,
        series_id: SeriesId,
    ) -> PortFuture<'_, Result<SeriesSnapshot, PortError>> {
        let result = self
            .series
            .get(&series_id)
            .cloned()
            .ok_or_else(|| PortError::NotFound {
                resource: format!("series:{series_id}"),
            });
        Box::pin(async move { result })
    }

    fn fetch_episode_snapshot(
        &self,
        key: EpisodeKey,
    ) -> PortFuture<'_, Result<EpisodeSnapshot, PortError>> {
        Box::pin(async move {
            Err(PortError::NotFound {
                resource: format!("episode:{}:{}", key.series_id, key.ep_num),
            })
        })
    }

    fn fetch_decrypted_cover<'a>(
        &'a self,
        cover_url: &'a str,
    ) -> PortFuture<'a, Result<Vec<u8>, PortError>> {
        Box::pin(async move {
            Err(PortError::NotFound {
                resource: cover_url.to_owned(),
            })
        })
    }
}

impl CatalogPlaybackTicketPort for StubViewingCatalog {
    fn fetch_playback_ticket(
        &self,
        key: EpisodeKey,
    ) -> PortFuture<'_, Result<PlaybackTicket, PortError>> {
        self.ticket_calls.lock().push(key);
        let found = self.tickets.get(&key).cloned();
        Box::pin(async move {
            found.ok_or_else(|| PortError::NotFound {
                resource: format!("ticket:{}:{}", key.series_id, key.ep_num),
            })
        })
    }
}

#[derive(Default)]
pub struct StubOfflineAssets {
    assets: HashMap<EpisodeKey, CommittedOfflineAsset>,
    total_bytes: u64,
}

impl StubOfflineAssets {
    pub fn with_asset(mut self, asset: CommittedOfflineAsset) -> Self {
        self.total_bytes = self.total_bytes.saturating_add(asset.size_bytes);
        self.assets.insert(asset.key, asset);
        self
    }
}

impl OfflineAssetQueryPort for StubOfflineAssets {
    fn find_committed_episode(
        &self,
        key: EpisodeKey,
    ) -> PortFuture<'_, Result<Option<CommittedOfflineAsset>, PortError>> {
        let found = self.assets.get(&key).cloned();
        Box::pin(async move { Ok(found) })
    }

    fn total_offline_library_bytes(&self) -> PortFuture<'_, Result<u64, PortError>> {
        let total = self.total_bytes;
        Box::pin(async move { Ok(total) })
    }
}

pub fn sample_series(id: u64, title: &str, total_episodes: u32) -> SeriesSnapshot {
    SeriesSnapshot {
        series_id: SeriesId::try_from(id).expect("valid series id"),
        title: title.to_owned(),
        cover_url: format!("https://cdn.example.com/covers/{id}.jpg"),
        total_episodes,
        is_finished: true,
        score: 9.2,
    }
}

pub fn sample_online_ticket(key: EpisodeKey) -> PlaybackTicket {
    PlaybackTicket {
        key,
        streams: StreamEndpoints {
            hevc: Some(format!(
                "https://cdn.example.com/{}/{}/hevc.m3u8",
                key.series_id, key.ep_num
            )),
            main: Some(format!(
                "https://cdn.example.com/{}/{}/main.m3u8",
                key.series_id, key.ep_num
            )),
            preview: None,
        },
        cdn: CdnAuthHeaders {
            referer: "https://example.com/".to_owned(),
            user_agent: "hg-test/1.0".to_owned(),
            auth_header_name: None,
            auth_header_value: None,
            cdn_app: None,
        },
        issued_at_epoch_secs: 1_700_000_000,
        expires_at_epoch_secs: 1_700_001_800,
    }
}

pub struct ViewingTestHarness {
    pub _temp_dir: TempDir,
    pub clock: Arc<FixedClock>,
    pub catalog: Arc<StubViewingCatalog>,
    pub deps: ViewingDeps,
}

impl ViewingTestHarness {
    pub async fn new(
        flags: ViewingFeatureFlags,
        catalog: StubViewingCatalog,
        offline_assets: StubOfflineAssets,
    ) -> Self {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let storage = Arc::new(
            StorageLayout::new(
                &temp_dir.path().join("data"),
                &temp_dir.path().join("cache"),
            )
            .expect("storage layout"),
        );
        storage.ensure_dirs().await.expect("ensure dirs");
        let db = LocalDatabase::open_in_memory().await.expect("sqlite");
        let clock = Arc::new(FixedClock::new(1_700_000_000));
        let loopback = LoopbackProxyHandle::start(Arc::clone(&storage))
            .await
            .expect("loopback");
        let catalog = Arc::new(catalog);
        let offline_assets = Arc::new(offline_assets);

        let deps = ViewingDeps {
            db,
            storage,
            clock: Arc::clone(&clock) as _,
            catalog_metadata: Arc::clone(&catalog) as _,
            catalog_tickets: Arc::clone(&catalog) as _,
            offline_assets,
            loopback,
            flags,
        };
        Self {
            _temp_dir: temp_dir,
            clock,
            catalog,
            deps,
        }
    }

    pub async fn shutdown(self) {
        self.deps.loopback.shutdown();
        self.deps.db.close().await;
    }
}
