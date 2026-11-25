use hg_contracts::ids::StreamProfile;
use hg_contracts::viewing::{AppAppearance, PlaybackPreference};

use crate::error::ViewingError;

pub const MIN_PLAYBACK_CACHE_LIMIT_MB: u32 = 64;
pub const MIN_DEFAULT_RATE_HUNDREDTHS: u16 = 50;
pub const MAX_DEFAULT_RATE_HUNDREDTHS: u16 = 300;
pub const MIN_SIDEBAR_WIDTH: u16 = 160;
pub const MAX_SIDEBAR_WIDTH: u16 = 360;
pub const DEFAULT_SIDEBAR_WIDTH: u16 = 208;
pub const MAX_SEARCH_HISTORY_ENTRIES: usize = 20;

#[must_use]
pub const fn default_playback_preference() -> PlaybackPreference {
    PlaybackPreference {
        preferred_profile: StreamProfile::Hevc,
        playback_cache_limit_mb: 512,
        default_rate_hundredths: 100,
        appearance: AppAppearance::Dark,
        sidebar_collapsed: false,
        sidebar_width: DEFAULT_SIDEBAR_WIDTH,
        window_placement: None,
    }
}

pub fn validate_playback_preference(pref: &PlaybackPreference) -> Result<(), ViewingError> {
    if pref.playback_cache_limit_mb < MIN_PLAYBACK_CACHE_LIMIT_MB {
        return Err(ViewingError::InvalidPreference {
            message: format!("播放缓存上限不能低于 {MIN_PLAYBACK_CACHE_LIMIT_MB} MB"),
        });
    }
    if !(MIN_DEFAULT_RATE_HUNDREDTHS..=MAX_DEFAULT_RATE_HUNDREDTHS)
        .contains(&pref.default_rate_hundredths)
    {
        return Err(ViewingError::InvalidPreference {
            message: format!(
                "默认倍速必须在 {MIN_DEFAULT_RATE_HUNDREDTHS}..={MAX_DEFAULT_RATE_HUNDREDTHS} 之间"
            ),
        });
    }
    if !(MIN_SIDEBAR_WIDTH..=MAX_SIDEBAR_WIDTH).contains(&pref.sidebar_width) {
        return Err(ViewingError::InvalidPreference {
            message: format!("侧边栏宽度必须在 {MIN_SIDEBAR_WIDTH}..={MAX_SIDEBAR_WIDTH} 之间"),
        });
    }
    if let Some(placement) = pref.window_placement
        && (placement.width == 0 || placement.height == 0)
    {
        return Err(ViewingError::InvalidPreference {
            message: "窗口尺寸宽高必须大于 0".to_owned(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedSegmentMeta {
    pub segment_key: String,
    pub size_bytes: u64,
}

#[must_use]
pub fn plan_lru_eviction(
    segments_oldest_first: &[CachedSegmentMeta],
    limit_bytes: u64,
) -> Vec<String> {
    let mut total_bytes: u64 = segments_oldest_first
        .iter()
        .map(|s| s.size_bytes)
        .fold(0_u64, u64::saturating_add);
    let mut evict_keys = Vec::new();

    for segment in segments_oldest_first {
        if total_bytes <= limit_bytes {
            break;
        }
        total_bytes = total_bytes.saturating_sub(segment.size_bytes);
        evict_keys.push(segment.segment_key.clone());
    }

    evict_keys
}
