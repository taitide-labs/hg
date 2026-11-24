use hg_contracts::error::ContractError;
use hg_contracts::viewing::PlaybackPreference;
use hg_viewing_domain::preference::validate_playback_preference;

pub fn check_playback_preference(pref: &PlaybackPreference) -> Result<(), ContractError> {
    validate_playback_preference(pref).map_err(ContractError::from)
}
