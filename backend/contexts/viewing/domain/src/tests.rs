use hg_backend_contracts::catalog::{CdnAuthHeaders, PlaybackTicket, StreamEndpoints};
use hg_backend_contracts::offline::CommittedOfflineAsset;
use hg_contracts::ids::{EpisodeKey, EpisodeNumber, SeriesId, StreamProfile};
use hg_contracts::viewing::{OnlineStreamKind, PlaybackPreference};

use crate::playback::{ResolvedPlaybackPlan, resolve_playback_plan};
use crate::preference::{
    CachedSegmentMeta, default_playback_preference, plan_lru_eviction, validate_playback_preference,
};
use crate::progress::evaluate_watch_progress;

fn sample_key(series: u64, ep: u32) -> EpisodeKey {
    EpisodeKey::new(
        SeriesId::try_from(series).expect("valid series"),
        EpisodeNumber::try_from(ep).expect("valid ep"),
    )
}

#[test]
fn prioritizes_committed_offline_asset_over_online_ticket() {
    let key = sample_key(14288, 1);
    let offline = CommittedOfflineAsset {
        key,
        profile: StreamProfile::Hevc,
        playlist_rel_path: "14288/ep-1/index.m3u8".to_owned(),
        duration_secs: 120,
        size_bytes: 2_048_000,
    };
    let ticket = PlaybackTicket {
        key,
        streams: StreamEndpoints {
            hevc: Some("https://cdn.example.com/hevc.m3u8".to_owned()),
            main: Some("https://cdn.example.com/main.m3u8".to_owned()),
            preview: None,
        },
        cdn: CdnAuthHeaders {
            referer: "https://example.com/".to_owned(),
            user_agent: "hg-test".to_owned(),
            auth_header_name: None,
            auth_header_value: None,
            cdn_app: None,
        },
        issued_at_epoch_secs: 1_730_000_000,
        expires_at_epoch_secs: 1_730_001_800,
    };

    let plan = resolve_playback_plan(Some(&offline), Some(&ticket), StreamProfile::Main)
        .expect("should resolve");
    assert_eq!(
        plan,
        ResolvedPlaybackPlan::OfflineLocal {
            playlist_rel_path: "14288/ep-1/index.m3u8".to_owned(),
            profile: StreamProfile::Hevc,
            duration_secs: 120,
        }
    );
}

#[test]
fn falls_back_to_preview_for_online_playback_when_needed() {
    let key = sample_key(14288, 2);
    let ticket = PlaybackTicket {
        key,
        streams: StreamEndpoints {
            hevc: None,
            main: None,
            preview: Some("https://cdn.example.com/preview.m3u8".to_owned()),
        },
        cdn: CdnAuthHeaders {
            referer: "https://example.com/".to_owned(),
            user_agent: "hg-test".to_owned(),
            auth_header_name: None,
            auth_header_value: None,
            cdn_app: None,
        },
        issued_at_epoch_secs: 1_730_000_000,
        expires_at_epoch_secs: 1_730_001_800,
    };

    let plan = resolve_playback_plan(None, Some(&ticket), StreamProfile::Hevc)
        .expect("should resolve preview");
    assert_eq!(
        plan,
        ResolvedPlaybackPlan::OnlineStream {
            stream_url: "https://cdn.example.com/preview.m3u8".to_owned(),
            kind: OnlineStreamKind::Preview,
        }
    );
}

#[test]
fn marks_tail_completed_and_returns_next_episode() {
    let key = sample_key(14288, 3);
    let mid = evaluate_watch_progress(key, 45, 100, 10);
    assert!(!mid.progress.completed);
    assert_eq!(mid.next_episode, None);

    let near_end_percent = evaluate_watch_progress(key, 95, 100, 10);
    assert!(near_end_percent.progress.completed);
    assert_eq!(
        near_end_percent.next_episode,
        Some(EpisodeNumber::try_from(4).expect("ep4"))
    );

    let last_ep = evaluate_watch_progress(sample_key(14288, 10), 118, 120, 10);
    assert!(last_ep.progress.completed);
    assert_eq!(last_ep.next_episode, None);
}

#[test]
fn validates_playback_preference_and_plans_lru_eviction() {
    assert!(validate_playback_preference(&default_playback_preference()).is_ok());

    let bad = PlaybackPreference {
        playback_cache_limit_mb: 32,
        ..PlaybackPreference::default()
    };
    assert!(validate_playback_preference(&bad).is_err());

    let bad_sidebar = PlaybackPreference {
        sidebar_width: 80,
        ..PlaybackPreference::default()
    };
    assert!(validate_playback_preference(&bad_sidebar).is_err());

    let segments = vec![
        CachedSegmentMeta {
            segment_key: "seg-1".to_owned(),
            size_bytes: 400,
        },
        CachedSegmentMeta {
            segment_key: "seg-2".to_owned(),
            size_bytes: 400,
        },
        CachedSegmentMeta {
            segment_key: "seg-3".to_owned(),
            size_bytes: 400,
        },
    ];
    assert_eq!(plan_lru_eviction(&segments, 800), vec!["seg-1".to_owned()]);
}
