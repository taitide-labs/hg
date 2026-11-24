use hg_backend_contracts::offline::OfflineDomainEvent;
use hg_contracts::ids::{EpisodeKey, SeriesId};

#[must_use]
pub fn build_episode_removed_event(key: EpisodeKey) -> OfflineDomainEvent {
    OfflineDomainEvent::OfflineEpisodeRemoved {
        key,
        asset_rel_dir: format!("s{}/e{}", key.series_id, key.ep_num),
    }
}

#[must_use]
pub fn build_series_removed_event(series_id: SeriesId) -> OfflineDomainEvent {
    OfflineDomainEvent::OfflineSeriesRemoved {
        series_id,
        asset_rel_dir: format!("s{series_id}"),
    }
}
