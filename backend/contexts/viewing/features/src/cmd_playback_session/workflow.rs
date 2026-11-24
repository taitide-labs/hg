use hg_backend_contracts::catalog::PlaybackTicket;
use hg_backend_contracts::offline::CommittedOfflineAsset;
use hg_contracts::error::ContractError;
use hg_contracts::ids::StreamProfile;
use hg_viewing_domain::playback::{ResolvedPlaybackPlan, resolve_playback_plan};

pub fn select_playback_plan(
    offline_asset: Option<&CommittedOfflineAsset>,
    online_ticket: Option<&PlaybackTicket>,
    preferred_profile: StreamProfile,
) -> Result<ResolvedPlaybackPlan, ContractError> {
    resolve_playback_plan(offline_asset, online_ticket, preferred_profile)
        .map_err(ContractError::from)
}
