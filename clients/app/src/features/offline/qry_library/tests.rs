use std::num::{NonZeroU32, NonZeroU64};

use hg_contracts::ids::{EpisodeNumber, SeriesId, StreamProfile};
use hg_contracts::offline::{OfflineEpisodeView, OfflineSeriesView};

use super::query::{
    episodes_preview_text, first_downloaded_episode, format_size_bytes, library_totals,
    series_stats_text, sort_library_recent_first,
};

fn sample_episode(ep: u32, size_bytes: u64, downloaded_at: u64) -> OfflineEpisodeView {
    OfflineEpisodeView {
        ep_num: EpisodeNumber::new(NonZeroU32::new(ep).unwrap_or(NonZeroU32::MIN)),
        title: format!("第 {ep} 集"),
        duration_secs: 120,
        profile: StreamProfile::Hevc,
        size_bytes,
        downloaded_at_epoch_secs: downloaded_at,
    }
}

fn sample_series(id: u64, episodes: Vec<OfflineEpisodeView>) -> OfflineSeriesView {
    let total_size_bytes = episodes.iter().map(|ep| ep.size_bytes).sum();
    let total_episodes = episodes
        .iter()
        .map(|ep| ep.ep_num.get())
        .max()
        .unwrap_or(1)
        .max(12);
    OfflineSeriesView {
        series_id: SeriesId::new(NonZeroU64::new(id).unwrap_or(NonZeroU64::MIN)),
        title: format!("剧目 {id}"),
        cover_local_uri: "poster://amber-noir".to_owned(),
        total_episodes,
        total_size_bytes,
        episodes,
    }
}

#[test]
fn sort_library_recent_first_orders_series_and_episodes() {
    let mut library = vec![
        sample_series(1002, vec![sample_episode(5, 1024, 100)]),
        sample_series(
            1001,
            vec![
                sample_episode(3, 2048, 250),
                sample_episode(1, 2048, 200),
                sample_episode(2, 2048, 220),
            ],
        ),
    ];

    sort_library_recent_first(&mut library);

    let first = library.first().expect("expected first series");
    assert_eq!(first.series_id.get(), 1001);
    let ordered_eps: Vec<u32> = first.episodes.iter().map(|e| e.ep_num.get()).collect();
    assert_eq!(ordered_eps, vec![1, 2, 3]);
    assert_eq!(
        first_downloaded_episode(first).map(EpisodeNumber::get),
        Some(1)
    );
}

#[test]
fn format_size_and_summary_cover_units_and_episode_counts() {
    assert_eq!(format_size_bytes(512), "512 B");
    assert_eq!(format_size_bytes(2048), "2.0 KB");
    assert_eq!(format_size_bytes(58_400_000), "55.7 MB");
    assert_eq!(format_size_bytes(2_147_483_648), "2.00 GB");

    let s = sample_series(
        1001,
        vec![
            sample_episode(1, 20_000_000, 10),
            sample_episode(2, 20_000_000, 20),
            sample_episode(3, 18_400_000, 30),
        ],
    );
    assert_eq!(series_stats_text(&s), "已下载 3 集 · 55.7 MB");
    assert_eq!(library_totals(std::slice::from_ref(&s)), (1, 3, 58_400_000));
    assert_eq!(
        episodes_preview_text(&s.episodes),
        "已缓存 第 1 集 · 第 2 集 · 第 3 集"
    );
}

#[test]
fn episodes_preview_truncates_when_more_than_four_episodes() {
    let episodes: Vec<OfflineEpisodeView> = (1..=6)
        .map(|n| sample_episode(n, 1_000_000, u64::from(n)))
        .collect();
    assert_eq!(
        episodes_preview_text(&episodes),
        "已缓存 第 1 集 · 第 2 集 · 第 3 集 · 第 4 集 等 6 集"
    );
    assert_eq!(episodes_preview_text(&[]), "暂无可用分集");
}
