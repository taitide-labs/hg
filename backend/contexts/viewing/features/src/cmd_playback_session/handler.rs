use hg_backend_contracts::error::PortError;
use hg_backend_contracts::viewing::PlaybackHaltPort as _;
use hg_contracts::error::ContractError;
use hg_contracts::ids::{EpisodeKey, SeriesId, StreamProfile};
use hg_contracts::viewing::PlaybackSource;
use hg_viewing_domain::preference::default_playback_preference;

use crate::ViewingDeps;

use super::workflow::{parse_preferred_profile, plan_offline_registration, plan_online_source};

pub async fn prepare_episode_playback(
    deps: &ViewingDeps,
    key: EpisodeKey,
) -> Result<PlaybackSource, ContractError> {
    if let Some(offline_asset) = deps
        .offline_assets
        .find_committed_episode(key)
        .await
        .map_err(ContractError::from)?
    {
        let registration = plan_offline_registration(&offline_asset)?;
        let playable_uri = deps
            .loopback
            .register_offline_episode(key, &registration.playlist_rel_path)
            .map_err(|err| ContractError::from(PortError::from(err)))?;
        return Ok(PlaybackSource {
            key,
            playable_uri,
            origin: registration.origin,
            duration_secs: registration.duration_secs,
        });
    }

    resolve_online_playback_source(deps, key).await
}

pub async fn refresh_online_playback(
    deps: &ViewingDeps,
    key: EpisodeKey,
) -> Result<PlaybackSource, ContractError> {
    resolve_online_playback_source(deps, key).await
}

pub async fn stop_deleted_offline_episode(
    deps: &ViewingDeps,
    key: EpisodeKey,
) -> Result<(), ContractError> {
    deps.loopback
        .halt_episode(key)
        .await
        .map_err(ContractError::from)
}

pub async fn stop_deleted_offline_series(
    deps: &ViewingDeps,
    series_id: SeriesId,
) -> Result<(), ContractError> {
    deps.loopback
        .halt_series(series_id)
        .await
        .map_err(ContractError::from)
}

async fn resolve_online_playback_source(
    deps: &ViewingDeps,
    key: EpisodeKey,
) -> Result<PlaybackSource, ContractError> {
    let preferred_profile = load_preferred_profile(deps).await?;
    let ticket = deps
        .catalog_tickets
        .fetch_playback_ticket(key)
        .await
        .map_err(ContractError::from)?;
    let mut source = plan_online_source(key, &ticket, preferred_profile)?;
    let headers = collect_cdn_headers(&ticket.cdn);
    source.playable_uri = deps
        .loopback
        .register_online_stream(key, &source.playable_uri, &headers)
        .map_err(|err| ContractError::from(PortError::from(err)))?;
    Ok(source)
}

fn collect_cdn_headers(
    cdn: &hg_backend_contracts::catalog::CdnAuthHeaders,
) -> Vec<(String, String)> {
    let mut headers = Vec::new();
    if !cdn.referer.trim().is_empty() {
        headers.push(("Referer".to_owned(), cdn.referer.clone()));
    }
    if !cdn.user_agent.trim().is_empty() {
        headers.push(("User-Agent".to_owned(), cdn.user_agent.clone()));
    }
    if let (Some(name), Some(value)) = (&cdn.auth_header_name, &cdn.auth_header_value)
        && !name.trim().is_empty()
        && !value.trim().is_empty()
    {
        headers.push((name.clone(), value.clone()));
    }
    if let Some(app) = &cdn.cdn_app
        && !app.trim().is_empty()
    {
        headers.push(("x-cdn-app".to_owned(), app.clone()));
    }
    headers
}

async fn load_preferred_profile(deps: &ViewingDeps) -> Result<StreamProfile, ContractError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT preferred_profile FROM viewing_playback_preference WHERE id = 1")
            .fetch_optional(deps.pool())
            .await
            .map_err(|err| ContractError::Internal {
                detail: format!("读取播放清晰度偏好失败: {err}"),
            })?;

    match row {
        Some((raw_profile,)) => parse_preferred_profile(&raw_profile),
        None => Ok(default_playback_preference().preferred_profile),
    }
}
