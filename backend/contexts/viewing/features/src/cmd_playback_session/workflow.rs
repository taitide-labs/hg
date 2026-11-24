use hg_backend_contracts::catalog::PlaybackTicket;
use hg_backend_contracts::offline::CommittedOfflineAsset;
use hg_contracts::error::ContractError;
use hg_contracts::ids::{EpisodeKey, StreamProfile};
use hg_contracts::viewing::{PlaybackOrigin, PlaybackSource};
use hg_viewing_domain::playback::{ResolvedPlaybackPlan, resolve_playback_plan};
use hg_viewing_domain::preference::default_playback_preference;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfflineRegistrationPlan {
    pub playlist_rel_path: String,
    pub origin: PlaybackOrigin,
    pub duration_secs: u32,
}

pub fn select_playback_plan(
    offline_asset: Option<&CommittedOfflineAsset>,
    online_ticket: Option<&PlaybackTicket>,
    preferred_profile: StreamProfile,
) -> Result<ResolvedPlaybackPlan, ContractError> {
    resolve_playback_plan(offline_asset, online_ticket, preferred_profile)
        .map_err(ContractError::from)
}

pub fn parse_preferred_profile(raw: &str) -> Result<StreamProfile, ContractError> {
    match raw {
        "hevc" => Ok(StreamProfile::Hevc),
        "main" => Ok(StreamProfile::Main),
        _ => Err(ContractError::Internal {
            detail: format!("未知播放清晰度偏好: {raw}"),
        }),
    }
}

pub fn plan_offline_registration(
    asset: &CommittedOfflineAsset,
) -> Result<OfflineRegistrationPlan, ContractError> {
    let plan = select_playback_plan(
        Some(asset),
        None,
        default_playback_preference().preferred_profile,
    )?;
    let origin = plan.origin();
    match plan {
        ResolvedPlaybackPlan::OfflineLocal {
            playlist_rel_path,
            duration_secs,
            ..
        } => Ok(OfflineRegistrationPlan {
            playlist_rel_path,
            origin,
            duration_secs,
        }),
        ResolvedPlaybackPlan::OnlineStream { .. } => Err(ContractError::Internal {
            detail: "离线选源意外返回在线计划".to_owned(),
        }),
    }
}

pub fn plan_online_source(
    key: EpisodeKey,
    ticket: &PlaybackTicket,
    preferred_profile: StreamProfile,
) -> Result<PlaybackSource, ContractError> {
    let plan = select_playback_plan(None, Some(ticket), preferred_profile)?;
    let origin = plan.origin();
    match plan {
        ResolvedPlaybackPlan::OnlineStream { stream_url, .. } => Ok(PlaybackSource {
            key,
            playable_uri: stream_url,
            origin,
            duration_secs: 0,
        }),
        ResolvedPlaybackPlan::OfflineLocal { .. } => Err(ContractError::Internal {
            detail: "在线选源意外返回离线计划".to_owned(),
        }),
    }
}
