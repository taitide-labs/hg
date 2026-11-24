use std::collections::HashMap;
use std::sync::Arc;

use hg_backend_contracts::catalog::{
    CatalogMetadataPort, CatalogPlaybackTicketPort, CdnAuthHeaders, EpisodeSnapshot,
    PlaybackTicket, SeriesSnapshot, StreamEndpoints,
};
use hg_backend_contracts::error::{PortError, PortFuture};
use hg_backend_contracts::viewing::PlaybackHaltPort;
use hg_contracts::ids::{EpisodeKey, SeriesId};
use hg_infra::clock::FixedClock;
use hg_infra::db::LocalDatabase;
use hg_infra::storage::StorageLayout;
use hg_offline_features::OfflineDeps;
use hg_offline_features::fetcher::OfflineSegmentFetcher;
use parking_lot::Mutex;
use tempfile::TempDir;

#[derive(Default)]
pub struct StubCatalogPorts {
    series: HashMap<SeriesId, SeriesSnapshot>,
    episodes: HashMap<EpisodeKey, EpisodeSnapshot>,
    covers: HashMap<String, Vec<u8>>,
    tickets: HashMap<EpisodeKey, PlaybackTicket>,
}

impl StubCatalogPorts {
    pub fn with_series(mut self, snap: SeriesSnapshot) -> Self {
        self.series.insert(snap.series_id, snap);
        self
    }

    pub fn with_episode(mut self, snap: EpisodeSnapshot) -> Self {
        self.episodes.insert(snap.key, snap);
        self
    }

    pub fn with_cover(mut self, url: impl Into<String>, bytes: Vec<u8>) -> Self {
        self.covers.insert(url.into(), bytes);
        self
    }

    pub fn with_ticket(mut self, ticket: PlaybackTicket) -> Self {
        self.tickets.insert(ticket.key, ticket);
        self
    }
}

impl CatalogMetadataPort for StubCatalogPorts {
    fn fetch_series_snapshot(
        &self,
        series_id: SeriesId,
    ) -> PortFuture<'_, Result<SeriesSnapshot, PortError>> {
        let found = self.series.get(&series_id).cloned();
        Box::pin(async move {
            found.ok_or_else(|| PortError::NotFound {
                resource: format!("series:{series_id}"),
            })
        })
    }

    fn fetch_episode_snapshot(
        &self,
        key: EpisodeKey,
    ) -> PortFuture<'_, Result<EpisodeSnapshot, PortError>> {
        let found = self.episodes.get(&key).cloned();
        Box::pin(async move {
            found.ok_or_else(|| PortError::NotFound {
                resource: format!("episode:{}:{}", key.series_id, key.ep_num),
            })
        })
    }

    fn fetch_decrypted_cover<'a>(
        &'a self,
        cover_url: &'a str,
    ) -> PortFuture<'a, Result<Vec<u8>, PortError>> {
        let found = self.covers.get(cover_url).cloned();
        Box::pin(async move {
            found.ok_or_else(|| PortError::NotFound {
                resource: cover_url.to_owned(),
            })
        })
    }
}

impl CatalogPlaybackTicketPort for StubCatalogPorts {
    fn fetch_playback_ticket(
        &self,
        key: EpisodeKey,
    ) -> PortFuture<'_, Result<PlaybackTicket, PortError>> {
        let found = self.tickets.get(&key).cloned();
        Box::pin(async move {
            found.ok_or_else(|| PortError::NotFound {
                resource: format!("ticket:{}:{}", key.series_id, key.ep_num),
            })
        })
    }
}

#[derive(Default)]
pub struct StubPlaybackHalt {
    halted_episodes: Mutex<Vec<EpisodeKey>>,
    halted_series: Mutex<Vec<SeriesId>>,
}

impl StubPlaybackHalt {
    pub fn halted_episodes(&self) -> Vec<EpisodeKey> {
        self.halted_episodes.lock().clone()
    }

    pub fn halted_series(&self) -> Vec<SeriesId> {
        self.halted_series.lock().clone()
    }
}

impl PlaybackHaltPort for StubPlaybackHalt {
    fn halt_episode(&self, key: EpisodeKey) -> PortFuture<'_, Result<(), PortError>> {
        self.halted_episodes.lock().push(key);
        Box::pin(async { Ok(()) })
    }

    fn halt_series(&self, series_id: SeriesId) -> PortFuture<'_, Result<(), PortError>> {
        self.halted_series.lock().push(series_id);
        Box::pin(async { Ok(()) })
    }
}

#[derive(Default)]
pub struct FakeSegmentFetcher {
    files: HashMap<String, Vec<u8>>,
}

impl FakeSegmentFetcher {
    pub fn with_file(mut self, url: impl Into<String>, data: Vec<u8>) -> Self {
        self.files.insert(url.into(), data);
        self
    }
}

impl OfflineSegmentFetcher for FakeSegmentFetcher {
    fn fetch_bytes<'a>(
        &'a self,
        url: &'a str,
        _cdn: &'a CdnAuthHeaders,
    ) -> PortFuture<'a, Result<Vec<u8>, PortError>> {
        let found = self.files.get(url).cloned();
        Box::pin(async move {
            found.ok_or_else(|| PortError::NotFound {
                resource: url.to_owned(),
            })
        })
    }
}

pub fn sample_cdn_headers() -> CdnAuthHeaders {
    CdnAuthHeaders {
        referer: "https://example.com/".to_owned(),
        user_agent: "hg-test/1.0".to_owned(),
        auth_header_name: Some("x-cdn-auth".to_owned()),
        auth_header_value: Some("tok".to_owned()),
        cdn_app: Some("150".to_owned()),
    }
}

pub fn sample_ticket(key: EpisodeKey, hevc: Option<&str>, preview: Option<&str>) -> PlaybackTicket {
    PlaybackTicket {
        key,
        streams: StreamEndpoints {
            hevc: hevc.map(str::to_owned),
            main: None,
            preview: preview.map(str::to_owned),
        },
        cdn: sample_cdn_headers(),
        issued_at_epoch_secs: 1_700_000_000,
        expires_at_epoch_secs: 1_700_001_800,
    }
}

pub struct OfflineTestHarness {
    pub _temp_dir: TempDir,
    pub _clock: Arc<FixedClock>,
    pub halt: Arc<StubPlaybackHalt>,
    pub deps: OfflineDeps,
}

impl OfflineTestHarness {
    pub async fn new(catalog: StubCatalogPorts, fetcher: FakeSegmentFetcher) -> Self {
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
        let catalog = Arc::new(catalog);
        let halt = Arc::new(StubPlaybackHalt::default());
        let fetcher = Arc::new(fetcher);

        let deps = OfflineDeps {
            db,
            storage,
            clock: Arc::clone(&clock) as _,
            catalog_metadata: Arc::clone(&catalog) as _,
            catalog_tickets: catalog,
            playback_halt: Arc::clone(&halt) as _,
            segment_fetcher: fetcher,
        };
        Self {
            _temp_dir: temp_dir,
            _clock: clock,
            halt,
            deps,
        }
    }
}
