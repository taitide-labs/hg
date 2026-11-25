use std::path::Path;

use hg_backend_contracts::catalog::{EpisodeSnapshot, PlaybackTicket, SeriesSnapshot};
use hg_backend_contracts::error::PortError;
use hg_contracts::error::ContractError;
use hg_contracts::ids::{EpisodeKey, StreamProfile};
use hg_contracts::offline::{
    ControlAllDownloadsAction, ControlDownloadAction, ControlDownloadRequest,
    DownloadFailureReason, DownloadPolicy, DownloadTaskState, EnqueueEpisodeDownloadsRequest,
    EnqueueEpisodeDownloadsResult,
};
use hg_infra::network::NetworkKind;
use hg_infra::storage::StorageLayout;

use crate::OfflineDeps;

use super::store::{
    ClaimedTask, CompletedEpisodeCommit, claim_next_queued_task, commit_completed_episode,
    fail_task, insert_queued_tasks, load_existing_episode_keys, load_policy_prefs,
    pause_tasks_for_wifi_only, read_task_state, resume_wifi_blocked_tasks, transition_all_tasks,
    transition_single_task, update_task_progress, upsert_policy,
};
use super::workflow::{
    PlaylistAsset, RewrittenPlaylist, begin_or_advance_downloading, check_download_policy,
    classify_port_error, plan_enqueue, rewrite_media_playlist, select_master_playlist_variant,
    select_stream,
};

struct EpisodeMetadata {
    series_title: String,
    episode_title: String,
    total_episodes: u32,
    duration_secs: u32,
    cover_bytes: Vec<u8>,
}

struct CommitInput<'a> {
    claimed: &'a ClaimedTask,
    profile: StreamProfile,
    rewritten_m3u8: &'a str,
    metadata: EpisodeMetadata,
    staging_dir: &'a Path,
    state: DownloadTaskState,
    downloaded_bytes: u64,
}

struct PreparedDownload {
    profile: StreamProfile,
    ticket: PlaybackTicket,
    rewritten: RewrittenPlaylist,
    metadata: EpisodeMetadata,
}

pub async fn enqueue_episode_downloads(
    deps: &OfflineDeps,
    request: EnqueueEpisodeDownloadsRequest,
) -> Result<EnqueueEpisodeDownloadsResult, ContractError> {
    let existing = load_existing_episode_keys(deps.pool(), request.series_id).await?;
    let plan = plan_enqueue(request.series_id, &request.episodes, &existing)?;

    if plan.to_queue.is_empty() {
        return Ok(EnqueueEpisodeDownloadsResult {
            queued: Vec::new(),
            skipped_already_present: plan.skipped_already_present,
        });
    }

    let prefs = load_policy_prefs(deps.pool()).await?;
    let series = deps
        .catalog_metadata
        .fetch_series_snapshot(request.series_id)
        .await
        .map_err(ContractError::from)?;

    let mut episodes = Vec::with_capacity(plan.to_queue.len());
    for &key in &plan.to_queue {
        let ep = deps
            .catalog_metadata
            .fetch_episode_snapshot(key)
            .await
            .map_err(ContractError::from)?;
        if !ep.playable {
            return Err(ContractError::InvalidInput {
                message: format!("分集 {} 暂不可播放", key.ep_num),
            });
        }
        episodes.push(ep);
    }

    let now = deps.clock.now_epoch_secs();
    insert_queued_tasks(
        deps.pool(),
        &series,
        &episodes,
        prefs.preferred_profile,
        now,
    )
    .await?;

    Ok(EnqueueEpisodeDownloadsResult {
        queued: plan.to_queue,
        skipped_already_present: plan.skipped_already_present,
    })
}

pub async fn control_download(
    deps: &OfflineDeps,
    request: ControlDownloadRequest,
) -> Result<(), ContractError> {
    let now = deps.clock.now_epoch_secs();
    transition_single_task(deps.pool(), request.key, request.action, now).await?;
    if request.action == ControlDownloadAction::Cancel {
        remove_dir_if_exists(&deps.storage.staging_episode_dir(request.key)).await?;
    }
    Ok(())
}

pub async fn control_all_downloads(
    deps: &OfflineDeps,
    action: ControlAllDownloadsAction,
) -> Result<(), ContractError> {
    let now = deps.clock.now_epoch_secs();
    transition_all_tasks(deps.pool(), action, now).await
}

pub async fn update_download_policy(
    deps: &OfflineDeps,
    policy: DownloadPolicy,
) -> Result<(), ContractError> {
    check_download_policy(&policy)?;
    let wifi_only = policy.wifi_only;
    upsert_policy(deps.pool(), &policy).await?;
    let now = deps.clock.now_epoch_secs();
    if wifi_only && deps.network.current_network() == NetworkKind::Metered {
        pause_tasks_for_wifi_only(deps.pool(), now).await?;
    } else if !wifi_only {
        resume_wifi_blocked_tasks(deps.pool(), now).await?;
    }
    Ok(())
}

pub async fn execute_next_queued_task(
    deps: &OfflineDeps,
) -> Result<Option<EpisodeKey>, ContractError> {
    let prefs = load_policy_prefs(deps.pool()).await?;
    let now = deps.clock.now_epoch_secs();
    if prefs.wifi_only && deps.network.current_network() == NetworkKind::Metered {
        pause_tasks_for_wifi_only(deps.pool(), now).await?;
        return Ok(None);
    }
    resume_wifi_blocked_tasks(deps.pool(), now).await?;

    let Some(claimed) =
        claim_next_queued_task(deps.pool(), prefs.max_concurrent_tasks, now).await?
    else {
        return Ok(None);
    };

    let key = claimed.key;
    let prepared = match prepare_download(deps, &claimed, prefs.preferred_profile).await {
        Ok(item) => item,
        Err(reason) => {
            let now_fail = deps.clock.now_epoch_secs();
            fail_task(
                deps.pool(),
                key,
                DownloadTaskState::FetchingSource,
                reason,
                0,
                now_fail,
            )
            .await?;
            return Ok(Some(key));
        }
    };

    run_download_and_commit(deps, &claimed, prepared).await?;
    Ok(Some(key))
}

async fn prepare_download(
    deps: &OfflineDeps,
    claimed: &ClaimedTask,
    preferred_profile: StreamProfile,
) -> Result<PreparedDownload, DownloadFailureReason> {
    let key = claimed.key;
    let mut ticket = deps
        .catalog_tickets
        .fetch_playback_ticket(key)
        .await
        .map_err(|err| classify_port_error(&err))?;

    let mut selected = select_stream(&ticket.streams, preferred_profile)
        .map_err(|_| DownloadFailureReason::PreviewStreamOnly)?;

    let metadata = fetch_metadata_for_task(deps, claimed).await?;

    let (playlist_text, playlist_url) =
        fetch_playlist_with_refresh(deps, key, preferred_profile, &mut ticket, &mut selected)
            .await?;

    let rewritten = rewrite_media_playlist(&playlist_text, &playlist_url)
        .map_err(|_| DownloadFailureReason::CorruptedSegment)?;

    Ok(PreparedDownload {
        profile: selected.profile,
        ticket,
        rewritten,
        metadata,
    })
}

async fn fetch_metadata_for_task(
    deps: &OfflineDeps,
    claimed: &ClaimedTask,
) -> Result<EpisodeMetadata, DownloadFailureReason> {
    let series = deps
        .catalog_metadata
        .fetch_series_snapshot(claimed.key.series_id)
        .await;
    let episode = deps
        .catalog_metadata
        .fetch_episode_snapshot(claimed.key)
        .await;

    let (series_title, cover_url, total_episodes) = match series {
        Ok(SeriesSnapshot {
            title,
            cover_url,
            total_episodes,
            ..
        }) => (title, cover_url, total_episodes),
        Err(PortError::NotFound { .. }) => {
            (claimed.series_title.clone(), claimed.cover_url.clone(), 0)
        }
        Err(err) => return Err(classify_port_error(&err)),
    };

    let (episode_title, duration_secs) = match episode {
        Ok(EpisodeSnapshot {
            title,
            duration_secs,
            ..
        }) => (title, duration_secs),
        Err(PortError::NotFound { .. }) => (claimed.episode_title.clone(), 0),
        Err(err) => return Err(classify_port_error(&err)),
    };

    let cover_bytes = if cover_url.trim().is_empty() {
        Vec::new()
    } else {
        deps.catalog_metadata
            .fetch_decrypted_cover(&cover_url)
            .await
            .map_err(|err| classify_port_error(&err))?
    };

    Ok(EpisodeMetadata {
        series_title,
        episode_title,
        total_episodes,
        duration_secs,
        cover_bytes,
    })
}

async fn fetch_playlist_with_refresh(
    deps: &OfflineDeps,
    key: EpisodeKey,
    preferred_profile: StreamProfile,
    ticket: &mut PlaybackTicket,
    selected: &mut hg_offline_domain::task::SelectedDownloadStream,
) -> Result<(String, String), DownloadFailureReason> {
    match fetch_resolved_media_playlist(deps, &selected.playlist_url, &ticket.cdn).await {
        Ok(res) => Ok(res),
        Err(DownloadFailureReason::TicketExpired) => {
            *ticket = deps
                .catalog_tickets
                .fetch_playback_ticket(key)
                .await
                .map_err(|err| classify_port_error(&err))?;
            *selected = select_stream(&ticket.streams, preferred_profile)
                .map_err(|_| DownloadFailureReason::PreviewStreamOnly)?;
            fetch_resolved_media_playlist(deps, &selected.playlist_url, &ticket.cdn).await
        }
        Err(other) => Err(other),
    }
}

async fn fetch_resolved_media_playlist(
    deps: &OfflineDeps,
    playlist_url: &str,
    cdn: &hg_backend_contracts::catalog::CdnAuthHeaders,
) -> Result<(String, String), DownloadFailureReason> {
    let bytes = deps
        .segment_fetcher
        .fetch_bytes(playlist_url, cdn)
        .await
        .map_err(|err| classify_port_error(&err))?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| DownloadFailureReason::CorruptedSegment)?
        .to_owned();

    if let Some(variant_url) = select_master_playlist_variant(&text, playlist_url)
        .map_err(|_| DownloadFailureReason::CorruptedSegment)?
    {
        let sub_bytes = deps
            .segment_fetcher
            .fetch_bytes(&variant_url, cdn)
            .await
            .map_err(|err| classify_port_error(&err))?;
        let sub_text = std::str::from_utf8(&sub_bytes)
            .map_err(|_| DownloadFailureReason::CorruptedSegment)?
            .to_owned();
        return Ok((sub_text, variant_url));
    }

    Ok((text, playlist_url.to_owned()))
}

async fn run_download_and_commit(
    deps: &OfflineDeps,
    claimed: &ClaimedTask,
    prepared: PreparedDownload,
) -> Result<(), ContractError> {
    let key = claimed.key;
    let staging_dir = deps.storage.staging_episode_dir(key);
    if claimed.previous_profile != prepared.profile {
        remove_dir_if_exists(&staging_dir).await?;
    }
    create_dir_all_mapped(&staging_dir).await?;

    let PreparedDownload {
        profile,
        ticket: mut active_ticket,
        rewritten,
        metadata,
    } = prepared;

    let total_segments = rewritten.segment_count;
    let mut state =
        begin_or_advance_downloading(DownloadTaskState::FetchingSource, 0, total_segments, 0)?;
    update_task_progress(
        deps.pool(),
        key,
        profile,
        state,
        deps.clock.now_epoch_secs(),
    )
    .await?;

    let mut completed_segments = 0_u32;
    let mut downloaded_bytes = 0_u64;

    for asset in &rewritten.assets {
        let prefs = load_policy_prefs(deps.pool()).await?;
        if prefs.wifi_only && deps.network.current_network() == NetworkKind::Metered {
            pause_tasks_for_wifi_only(deps.pool(), deps.clock.now_epoch_secs()).await?;
            return Ok(());
        }
        let Some(current) = read_task_state(deps.pool(), key).await? else {
            return Ok(());
        };
        if !matches!(current, DownloadTaskState::Downloading { .. }) {
            return Ok(());
        }

        let target_path = staging_dir.join(&asset.local_name);
        let asset_size =
            match ensure_asset_downloaded(deps, key, asset, &target_path, &mut active_ticket).await
            {
                Ok(size) => size,
                Err(reason) => {
                    fail_task(
                        deps.pool(),
                        key,
                        state,
                        reason,
                        downloaded_bytes,
                        deps.clock.now_epoch_secs(),
                    )
                    .await?;
                    return Ok(());
                }
            };

        downloaded_bytes = downloaded_bytes.saturating_add(asset_size);
        if asset.is_media_segment {
            completed_segments = completed_segments.saturating_add(1);
            state = begin_or_advance_downloading(
                state,
                completed_segments,
                total_segments,
                downloaded_bytes,
            )?;
            update_task_progress(
                deps.pool(),
                key,
                profile,
                state,
                deps.clock.now_epoch_secs(),
            )
            .await?;
        }
    }

    finalize_commit(
        deps,
        CommitInput {
            claimed,
            profile,
            rewritten_m3u8: &rewritten.rewritten_m3u8,
            metadata,
            staging_dir: &staging_dir,
            state,
            downloaded_bytes,
        },
    )
    .await
}

async fn ensure_asset_downloaded(
    deps: &OfflineDeps,
    key: EpisodeKey,
    asset: &PlaylistAsset,
    target_path: &Path,
    ticket: &mut PlaybackTicket,
) -> Result<u64, DownloadFailureReason> {
    if let Ok(meta) = tokio::fs::metadata(target_path).await
        && meta.is_file()
        && meta.len() > 0
    {
        return Ok(meta.len());
    }

    let bytes = match deps
        .segment_fetcher
        .fetch_bytes(&asset.remote_url, &ticket.cdn)
        .await
    {
        Ok(data) => data,
        Err(PortError::UpstreamRejected { .. }) => {
            *ticket = deps
                .catalog_tickets
                .fetch_playback_ticket(key)
                .await
                .map_err(|err| classify_port_error(&err))?;
            deps.segment_fetcher
                .fetch_bytes(&asset.remote_url, &ticket.cdn)
                .await
                .map_err(|err| classify_port_error(&err))?
        }
        Err(err) => return Err(classify_port_error(&err)),
    };

    if bytes.is_empty() {
        return Err(DownloadFailureReason::CorruptedSegment);
    }

    let len = u64::try_from(bytes.len()).map_err(|_| DownloadFailureReason::CorruptedSegment)?;
    write_atomic_file(target_path, &bytes)
        .await
        .map_err(|_| DownloadFailureReason::StorageFull)?;

    Ok(len)
}

async fn finalize_commit(deps: &OfflineDeps, input: CommitInput<'_>) -> Result<(), ContractError> {
    let key = input.claimed.key;
    let playlist_path = input.staging_dir.join("index.m3u8");
    if let Err(_err) = write_atomic_file(&playlist_path, input.rewritten_m3u8.as_bytes()).await {
        fail_task(
            deps.pool(),
            key,
            input.state,
            DownloadFailureReason::StorageFull,
            input.downloaded_bytes,
            deps.clock.now_epoch_secs(),
        )
        .await?;
        return Ok(());
    }

    let total_size = StorageLayout::dir_size_bytes(input.staging_dir)
        .await
        .map_err(|err| ContractError::Internal {
            detail: format!("计算暂存分集体积失败: {err}"),
        })?;

    let series_dir = deps.storage.library_series_dir(key.series_id);
    create_dir_all_mapped(&series_dir).await?;

    let cover_rel_path = format!("s{}/cover.png", key.series_id);
    if !input.metadata.cover_bytes.is_empty() {
        let cover_path = series_dir.join("cover.png");
        if let Err(_err) = write_atomic_file(&cover_path, &input.metadata.cover_bytes).await {
            fail_task(
                deps.pool(),
                key,
                input.state,
                DownloadFailureReason::StorageFull,
                input.downloaded_bytes,
                deps.clock.now_epoch_secs(),
            )
            .await?;
            return Ok(());
        }
    }

    let Some(latest_state) = read_task_state(deps.pool(), key).await? else {
        remove_dir_if_exists(input.staging_dir).await?;
        return Ok(());
    };
    if !matches!(latest_state, DownloadTaskState::Downloading { .. }) {
        if matches!(latest_state, DownloadTaskState::Cancelled) {
            remove_dir_if_exists(input.staging_dir).await?;
        }
        return Ok(());
    }

    if let Err(_err) = deps.storage.commit_staging_episode(key).await {
        fail_task(
            deps.pool(),
            key,
            input.state,
            DownloadFailureReason::StorageFull,
            input.downloaded_bytes,
            deps.clock.now_epoch_secs(),
        )
        .await?;
        return Ok(());
    }

    let commit = CompletedEpisodeCommit {
        key,
        series_title: input.metadata.series_title,
        cover_local_path: cover_rel_path,
        total_episodes: input.metadata.total_episodes,
        episode_title: input.metadata.episode_title,
        duration_secs: input.metadata.duration_secs,
        profile: input.profile,
        playlist_rel_path: format!("s{}/e{}/index.m3u8", key.series_id, key.ep_num),
        size_bytes: total_size,
        now_epoch_secs: deps.clock.now_epoch_secs(),
    };
    let _committed = commit_completed_episode(deps.pool(), &commit).await?;
    Ok(())
}

async fn write_atomic_file(target: &Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    let file_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("asset");
    let part_path = target.with_file_name(format!("{file_name}.part"));
    tokio::fs::write(&part_path, bytes).await?;
    tokio::fs::rename(&part_path, target).await
}

async fn create_dir_all_mapped(path: &Path) -> Result<(), ContractError> {
    tokio::fs::create_dir_all(path).await.map_err(|err| {
        if err.kind() == std::io::ErrorKind::StorageFull {
            ContractError::StorageFull
        } else {
            ContractError::Internal {
                detail: format!("创建离线目录失败: {err}"),
            }
        }
    })
}

async fn remove_dir_if_exists(path: &Path) -> Result<(), ContractError> {
    match tokio::fs::remove_dir_all(path).await {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(ContractError::Internal {
            detail: format!("清理暂存目录失败: {err}"),
        }),
    }
}
