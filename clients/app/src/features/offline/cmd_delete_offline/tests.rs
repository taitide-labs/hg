use std::num::{NonZeroU32, NonZeroU64};

use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId, StreamProfile};
use hg_contracts::offline::{OfflineEpisodeView, OfflineSeriesView};

use super::command::{
    delete_episode_a11y_label, delete_series_a11y_label, manage_episodes_a11y_label,
    remove_episode_from_library, remove_series_from_library,
};

fn ep(raw: u32) -> EpisodeNumber {
    EpisodeNumber::new(NonZeroU32::new(raw).unwrap_or(NonZeroU32::MIN))
}

fn series_id(raw: u64) -> SeriesId {
    SeriesId::new(NonZeroU64::new(raw).unwrap_or(NonZeroU64::MIN))
}

fn sample_episode(num: u32, size_bytes: u64) -> OfflineEpisodeView {
    OfflineEpisodeView {
        ep_num: ep(num),
        title: format!("第 {num} 集"),
        duration_secs: 120,
        profile: StreamProfile::Hevc,
        size_bytes,
        downloaded_at_epoch_secs: u64::from(num) * 100,
    }
}

fn sample_series(id: u64, episodes: Vec<OfflineEpisodeView>) -> OfflineSeriesView {
    let total_size_bytes = episodes.iter().map(|e| e.size_bytes).sum();
    OfflineSeriesView {
        series_id: series_id(id),
        title: format!("剧目 {id}"),
        cover_local_uri: "poster://amber-noir".to_owned(),
        total_size_bytes,
        episodes,
    }
}

#[test]
fn remove_series_from_library_drops_matching_series() {
    let mut library = vec![
        sample_series(1001, vec![sample_episode(1, 10_000)]),
        sample_series(1002, vec![sample_episode(1, 20_000)]),
    ];

    remove_series_from_library(&mut library, series_id(1001));

    let remaining_ids: Vec<u64> = library.iter().map(|s| s.series_id.get()).collect();
    assert_eq!(remaining_ids, vec![1002]);
}

#[test]
fn remove_episode_from_library_updates_size_and_prunes_empty_series() {
    let mut library = vec![
        sample_series(
            1001,
            vec![sample_episode(1, 10_000), sample_episode(2, 15_000)],
        ),
        sample_series(1002, vec![sample_episode(1, 20_000)]),
    ];

    remove_episode_from_library(&mut library, EpisodeKey::new(series_id(1001), ep(1)));
    let first = library.first().expect("expected 1001 to remain");
    assert_eq!(first.series_id.get(), 1001);
    assert_eq!(first.episodes.len(), 1);
    assert_eq!(first.total_size_bytes, 15_000);

    remove_episode_from_library(&mut library, EpisodeKey::new(series_id(1001), ep(2)));
    let remaining_ids: Vec<u64> = library.iter().map(|s| s.series_id.get()).collect();
    assert_eq!(remaining_ids, vec![1002]);
}

#[test]
fn a11y_labels_format_series_and_episode_targets() {
    assert_eq!(
        delete_series_a11y_label("云端之上：千金归来"),
        "删除 云端之上：千金归来 离线剧集"
    );
    assert_eq!(
        manage_episodes_a11y_label("云端之上：千金归来"),
        "管理 云端之上：千金归来 离线分集"
    );
    assert_eq!(
        delete_episode_a11y_label("云端之上：千金归来", ep(2)),
        "删除 云端之上：千金归来 第 2 集离线分集"
    );
}
