mod seed;

use std::future;
use std::num::NonZeroU32;
use std::sync::Arc;

use hg_contracts::catalog::{
    Category, CommentReply, EpisodeComment, HomeFeed, HotKeyword, HotTag,
    ListCommentRepliesRequest, ListEpisodeCommentsRequest, ListRankingRequest, ListSeriesRequest,
    PagedResult, RankingEntry, RankingKind, SearchSeriesRequest, SearchSuggestion, SeriesDetail,
    SeriesFilter, SeriesSort, SeriesSummary, TagGroup,
};
use hg_contracts::error::ContractError;
use hg_contracts::ids::{
    EpisodeKey, EpisodeNumber, PageInfo, PageRequest, SeriesId, StreamProfile,
};
use hg_contracts::offline::{
    ControlAllDownloadsAction, ControlDownloadAction, ControlDownloadRequest, DownloadPolicy,
    DownloadTaskState, DownloadTaskView, EnqueueEpisodeDownloadsRequest,
    EnqueueEpisodeDownloadsResult, OfflineSeriesView, PauseReason,
};
use hg_contracts::viewing::{
    OnlineStreamKind, PlaybackOrigin, PlaybackPreference, PlaybackSource,
    ReportWatchProgressRequest, ReportWatchProgressResult, SearchHistoryEntry, SeriesBookmark,
    StorageOverview, WatchHistoryEntry, WatchProgress,
};
use parking_lot::Mutex;

use crate::contract::{AppGateway, GatewayFuture};
use crate::mock::seed::{MockState, build_home_feed, build_initial_state};

#[derive(Clone)]
pub struct MockGateway {
    state: Arc<Mutex<MockState>>,
    injected_error: Arc<Mutex<Option<ContractError>>>,
}

impl Default for MockGateway {
    fn default() -> Self {
        Self::new()
    }
}

impl MockGateway {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(build_initial_state())),
            injected_error: Arc::new(Mutex::new(None)),
        }
    }

    pub fn inject_error(&self, error: Option<ContractError>) {
        *self.injected_error.lock() = error;
    }

    fn check_error(&self) -> Result<(), ContractError> {
        if let Some(error) = self.injected_error.lock().clone() {
            return Err(error);
        }
        Ok(())
    }

    fn paginate<T: Clone>(items: &[T], request: PageRequest) -> PagedResult<T> {
        let size = usize::from(request.size.get());
        let page = usize::try_from(request.page.get()).unwrap_or(1);
        let total = u64::try_from(items.len()).unwrap_or(u64::MAX);
        let pages_usize = if items.is_empty() {
            0
        } else {
            items.len().div_ceil(size)
        };
        let start = page.saturating_sub(1).saturating_mul(size);
        let slice = items
            .get(start..)
            .map(|rest| rest.iter().take(size).cloned().collect())
            .unwrap_or_default();
        PagedResult {
            items: slice,
            pagination: PageInfo {
                page: request.page.get(),
                pages: u32::try_from(pages_usize).unwrap_or(u32::MAX),
                size: u32::from(request.size.get()),
                total,
            },
        }
    }

    fn has_offline_episode(state: &MockState, key: EpisodeKey) -> Option<StreamProfile> {
        state
            .offline_library
            .iter()
            .find(|s| s.series_id == key.series_id)
            .and_then(|s| s.episodes.iter().find(|e| e.ep_num == key.ep_num))
            .map(|e| e.profile)
    }

    fn episode_completed(position_secs: u32, duration_secs: u32) -> bool {
        if duration_secs == 0 {
            return false;
        }
        let remaining = duration_secs.saturating_sub(position_secs);
        let percent = u64::from(position_secs).saturating_mul(100) / u64::from(duration_secs);
        remaining <= 3 || percent >= 95
    }

    fn ready<T: Send + 'static>(result: Result<T, ContractError>) -> GatewayFuture<T> {
        Box::pin(future::ready(result))
    }
}

impl AppGateway for MockGateway {
    fn home_feed(&self) -> GatewayFuture<HomeFeed> {
        let result = self.check_error().map(|()| {
            let state = self.state.lock();
            build_home_feed(&state)
        });
        Self::ready(result)
    }

    fn categories(&self) -> GatewayFuture<Vec<Category>> {
        let result = self.check_error().map(|()| {
            let state = self.state.lock();
            state.categories.clone()
        });
        Self::ready(result)
    }

    fn tag_groups(&self) -> GatewayFuture<Vec<TagGroup>> {
        let result = self.check_error().map(|()| {
            let state = self.state.lock();
            state.tag_groups.clone()
        });
        Self::ready(result)
    }

    fn list_series(&self, request: ListSeriesRequest) -> GatewayFuture<PagedResult<SeriesSummary>> {
        let result = self.check_error().map(|()| {
            let state = self.state.lock();
            let mut items: Vec<SeriesSummary> = state
                .details
                .values()
                .map(|d| d.summary.clone())
                .filter(|summary| match &request.filter {
                    SeriesFilter::All { category_id: None } => true,
                    SeriesFilter::All {
                        category_id: Some(25),
                    } => summary.tags.iter().any(|t| t == "漫剧"),
                    SeriesFilter::All {
                        category_id: Some(26),
                    } => summary.tags.iter().any(|t| t == "悬疑"),
                    SeriesFilter::All {
                        category_id: Some(27),
                    } => summary.tags.iter().any(|t| t == "古风"),
                    SeriesFilter::All {
                        category_id: Some(_),
                    } => true,
                    SeriesFilter::CategoryRoute { route } => match route.as_str() {
                        "manju" => summary.tags.iter().any(|t| t == "漫剧"),
                        "xuanyi" => summary.tags.iter().any(|t| t == "悬疑"),
                        "gufeng" => summary.tags.iter().any(|t| t == "古风"),
                        _ => true,
                    },
                    SeriesFilter::TagSlug { slug } => {
                        let target = match slug.as_str() {
                            "dushi" => "都市",
                            "gufeng" => "古风",
                            "xuanyi" => "悬疑",
                            "kehuan" => "科幻",
                            "nixi" => "逆袭",
                            "fanzhuan" => "反转",
                            "tianchong" => "甜宠",
                            other => other,
                        };
                        summary.tags.iter().any(|t| t == target)
                    }
                })
                .collect();

            match request.sort {
                SeriesSort::Hot | SeriesSort::HotWindow => {
                    items.sort_by_key(|a| std::cmp::Reverse(a.hot));
                }
                SeriesSort::New => items.sort_by_key(|a| std::cmp::Reverse(a.id)),
                SeriesSort::Random => items.reverse(),
            }

            Self::paginate(&items, request.page)
        });
        Self::ready(result)
    }

    fn list_ranking(
        &self,
        request: ListRankingRequest,
    ) -> GatewayFuture<PagedResult<RankingEntry>> {
        let result = self.check_error().map(|()| {
            let state = self.state.lock();
            let mut items: Vec<SeriesSummary> =
                state.details.values().map(|d| d.summary.clone()).collect();
            match request.kind {
                RankingKind::Hot => items.sort_by_key(|a| std::cmp::Reverse(a.hot)),
                RankingKind::Potential => {
                    items.sort_by_key(|a| std::cmp::Reverse(a.episode_count));
                }
                RankingKind::Recommend => {
                    items.sort_by(|a, b| b.score.total_cmp(&a.score));
                }
            }
            let metric_label = match request.kind {
                RankingKind::Hot => "热度",
                RankingKind::Potential => "飙升指数",
                RankingKind::Recommend => "口碑推荐",
            };
            let entries: Vec<RankingEntry> = items
                .into_iter()
                .enumerate()
                .map(|(idx, series)| RankingEntry {
                    rank: u32::try_from(idx + 1).unwrap_or(u32::MAX),
                    metric_value: series.hot,
                    metric_label: metric_label.to_owned(),
                    series,
                })
                .collect();
            Self::paginate(&entries, request.page)
        });
        Self::ready(result)
    }

    fn search_series(
        &self,
        request: SearchSeriesRequest,
    ) -> GatewayFuture<PagedResult<SeriesSummary>> {
        let result = self.check_error().map(|()| {
            let state = self.state.lock();
            let needle = request.keyword.trim().to_lowercase();
            let items: Vec<SeriesSummary> = state
                .details
                .values()
                .map(|d| d.summary.clone())
                .filter(|s| {
                    needle.is_empty()
                        || s.title.to_lowercase().contains(&needle)
                        || s.description.to_lowercase().contains(&needle)
                        || s.tags.iter().any(|t| t.to_lowercase().contains(&needle))
                })
                .collect();
            Self::paginate(&items, request.page)
        });
        Self::ready(result)
    }

    fn search_suggestions(&self, keyword: String) -> GatewayFuture<SearchSuggestion> {
        let result = self.check_error().map(|()| {
            let state = self.state.lock();
            let needle = keyword.trim().to_lowercase();
            let series: Vec<SeriesSummary> = state
                .details
                .values()
                .map(|d| d.summary.clone())
                .filter(|s| needle.is_empty() || s.title.to_lowercase().contains(&needle))
                .take(4)
                .collect();
            let phrases = series.iter().map(|s| s.title.clone()).collect();
            let tags = state
                .hot_tags
                .iter()
                .map(|t| t.name.clone())
                .filter(|t| needle.is_empty() || t.to_lowercase().contains(&needle))
                .collect();
            SearchSuggestion {
                series,
                phrases,
                tags,
            }
        });
        Self::ready(result)
    }

    fn hot_keywords(&self) -> GatewayFuture<Vec<HotKeyword>> {
        let result = self
            .check_error()
            .map(|()| self.state.lock().hot_keywords.clone());
        Self::ready(result)
    }

    fn hot_tags(&self) -> GatewayFuture<Vec<HotTag>> {
        let result = self
            .check_error()
            .map(|()| self.state.lock().hot_tags.clone());
        Self::ready(result)
    }

    fn series_detail(&self, series_id: SeriesId) -> GatewayFuture<SeriesDetail> {
        let result = self.check_error().and_then(|()| {
            let state = self.state.lock();
            state
                .details
                .get(&series_id)
                .cloned()
                .ok_or_else(|| ContractError::NotFound {
                    resource: format!("series/{series_id}"),
                })
        });
        Self::ready(result)
    }

    fn list_episode_comments(
        &self,
        request: ListEpisodeCommentsRequest,
    ) -> GatewayFuture<PagedResult<EpisodeComment>> {
        let result = self.check_error().map(|()| {
            let state = self.state.lock();
            let key = EpisodeKey::new(request.series_id, request.ep_num);
            let items = state.comments.get(&key).cloned().unwrap_or_default();
            Self::paginate(&items, request.page)
        });
        Self::ready(result)
    }

    fn list_comment_replies(
        &self,
        request: ListCommentRepliesRequest,
    ) -> GatewayFuture<PagedResult<CommentReply>> {
        let result = self.check_error().map(|()| {
            let state = self.state.lock();
            let replies: Vec<CommentReply> = state
                .comments
                .iter()
                .filter(|(k, _)| k.series_id == request.series_id)
                .flat_map(|(_, list)| list.iter())
                .find(|c| c.id == request.comment_id)
                .map(|c| c.preview_replies.clone())
                .unwrap_or_default();
            Self::paginate(&replies, request.page)
        });
        Self::ready(result)
    }

    fn clear_catalog_cache(&self) -> GatewayFuture<()> {
        let result = self.check_error().map(|()| {
            let mut state = self.state.lock();
            state.catalog_cache_bytes = 0;
        });
        Self::ready(result)
    }

    fn prepare_episode_playback(&self, key: EpisodeKey) -> GatewayFuture<PlaybackSource> {
        let result = self.check_error().and_then(|()| {
            let state = self.state.lock();
            let detail =
                state
                    .details
                    .get(&key.series_id)
                    .ok_or_else(|| ContractError::NotFound {
                        resource: format!("series/{}", key.series_id),
                    })?;
            let episode = detail
                .episodes
                .iter()
                .find(|e| e.ep_num == key.ep_num)
                .ok_or_else(|| ContractError::NotFound {
                    resource: format!("series/{}/ep/{}", key.series_id, key.ep_num),
                })?;

            if let Some(profile) = Self::has_offline_episode(&state, key) {
                return Ok(PlaybackSource {
                    key,
                    playable_uri: format!(
                        "http://127.0.0.1:19800/offline/{}/{}/master.m3u8",
                        key.series_id, key.ep_num
                    ),
                    origin: PlaybackOrigin::OfflineLocal { profile },
                    duration_secs: episode.duration_secs,
                });
            }

            let kind = match state.playback_preference.preferred_profile {
                StreamProfile::Hevc => OnlineStreamKind::Hevc,
                StreamProfile::Main => OnlineStreamKind::Main,
            };
            Ok(PlaybackSource {
                key,
                playable_uri: format!(
                    "http://127.0.0.1:19800/online/{}/{}/master.m3u8",
                    key.series_id, key.ep_num
                ),
                origin: PlaybackOrigin::OnlineStream { kind },
                duration_secs: episode.duration_secs,
            })
        });
        Self::ready(result)
    }

    fn refresh_online_playback(&self, key: EpisodeKey) -> GatewayFuture<PlaybackSource> {
        self.prepare_episode_playback(key)
    }

    fn report_watch_progress(
        &self,
        request: ReportWatchProgressRequest,
    ) -> GatewayFuture<ReportWatchProgressResult> {
        let result = self.check_error().and_then(|()| {
            let mut state = self.state.lock();
            let detail = state
                .details
                .get(&request.key.series_id)
                .cloned()
                .ok_or_else(|| ContractError::NotFound {
                    resource: format!("series/{}", request.key.series_id),
                })?;

            let completed = Self::episode_completed(request.position_secs, request.duration_secs);
            let next_episode = if completed && request.key.ep_num.get() < detail.total_episodes {
                NonZeroU32::new(request.key.ep_num.get() + 1).map(EpisodeNumber::new)
            } else {
                None
            };

            state.next_timestamp += 1;
            let watched_at = state.next_timestamp;
            state
                .watch_history
                .retain(|entry| entry.series_id != request.key.series_id);
            state.watch_history.insert(
                0,
                WatchHistoryEntry {
                    series_id: request.key.series_id,
                    title: detail.summary.title,
                    cover_url: detail.summary.cover_url,
                    last_episode: request.key.ep_num,
                    position_secs: request.position_secs,
                    duration_secs: request.duration_secs,
                    episode_completed: completed,
                    total_episodes: detail.total_episodes,
                    watched_at_epoch_secs: watched_at,
                },
            );

            Ok(ReportWatchProgressResult {
                progress: WatchProgress {
                    key: request.key,
                    position_secs: request.position_secs,
                    duration_secs: request.duration_secs,
                    completed,
                },
                next_episode,
            })
        });
        Self::ready(result)
    }

    fn watch_history(&self) -> GatewayFuture<Vec<WatchHistoryEntry>> {
        let result = self
            .check_error()
            .map(|()| self.state.lock().watch_history.clone());
        Self::ready(result)
    }

    fn remove_watch_history(&self, series_id: SeriesId) -> GatewayFuture<()> {
        let result = self.check_error().map(|()| {
            self.state
                .lock()
                .watch_history
                .retain(|e| e.series_id != series_id);
        });
        Self::ready(result)
    }

    fn clear_watch_history(&self) -> GatewayFuture<()> {
        let result = self.check_error().map(|()| {
            self.state.lock().watch_history.clear();
        });
        Self::ready(result)
    }

    fn bookmarks(&self) -> GatewayFuture<Vec<SeriesBookmark>> {
        let result = self
            .check_error()
            .map(|()| self.state.lock().bookmarks.clone());
        Self::ready(result)
    }

    fn is_bookmarked(&self, series_id: SeriesId) -> GatewayFuture<bool> {
        let result = self.check_error().map(|()| {
            self.state
                .lock()
                .bookmarks
                .iter()
                .any(|b| b.series_id == series_id)
        });
        Self::ready(result)
    }

    fn bookmark_series(&self, series_id: SeriesId) -> GatewayFuture<()> {
        let result =
            self.check_error().and_then(|()| {
                let mut state = self.state.lock();
                if state.bookmarks.iter().any(|b| b.series_id == series_id) {
                    return Ok(());
                }
                let detail = state.details.get(&series_id).cloned().ok_or_else(|| {
                    ContractError::NotFound {
                        resource: format!("series/{series_id}"),
                    }
                })?;
                state.next_timestamp += 1;
                let ts = state.next_timestamp;
                state.bookmarks.insert(
                    0,
                    SeriesBookmark {
                        series_id,
                        title: detail.summary.title,
                        cover_url: detail.summary.cover_url,
                        episode_count: detail.summary.episode_count,
                        is_finished: detail.summary.is_finished,
                        score: detail.summary.score,
                        bookmarked_at_epoch_secs: ts,
                    },
                );
                Ok(())
            });
        Self::ready(result)
    }

    fn unbookmark_series(&self, series_id: SeriesId) -> GatewayFuture<()> {
        let result = self.check_error().map(|()| {
            self.state
                .lock()
                .bookmarks
                .retain(|b| b.series_id != series_id);
        });
        Self::ready(result)
    }

    fn playback_preference(&self) -> GatewayFuture<PlaybackPreference> {
        let result = self
            .check_error()
            .map(|()| self.state.lock().playback_preference);
        Self::ready(result)
    }

    fn update_playback_preference(&self, preference: PlaybackPreference) -> GatewayFuture<()> {
        let result = self.check_error().map(|()| {
            self.state.lock().playback_preference = preference;
        });
        Self::ready(result)
    }

    fn search_history(&self) -> GatewayFuture<Vec<SearchHistoryEntry>> {
        let result = self
            .check_error()
            .map(|()| self.state.lock().search_history.clone());
        Self::ready(result)
    }

    fn record_search_history(&self, keyword: String) -> GatewayFuture<()> {
        let result = self.check_error().map(|()| {
            let trimmed = keyword.trim();
            if trimmed.is_empty() {
                return;
            }
            let mut state = self.state.lock();
            state.next_timestamp += 1;
            let ts = state.next_timestamp;
            state
                .search_history
                .retain(|entry| entry.keyword != trimmed);
            state.search_history.insert(
                0,
                SearchHistoryEntry {
                    keyword: trimmed.to_owned(),
                    searched_at_epoch_secs: ts,
                },
            );
            state.search_history.truncate(20);
        });
        Self::ready(result)
    }

    fn remove_search_history(&self, keyword: String) -> GatewayFuture<()> {
        let result = self.check_error().map(|()| {
            let trimmed = keyword.trim();
            self.state
                .lock()
                .search_history
                .retain(|entry| entry.keyword != trimmed);
        });
        Self::ready(result)
    }

    fn clear_search_history(&self) -> GatewayFuture<()> {
        let result = self.check_error().map(|()| {
            self.state.lock().search_history.clear();
        });
        Self::ready(result)
    }

    fn clear_playback_cache(&self) -> GatewayFuture<()> {
        let result = self.check_error().map(|()| {
            self.state.lock().playback_cache_bytes = 0;
        });
        Self::ready(result)
    }

    fn storage_overview(&self) -> GatewayFuture<StorageOverview> {
        let result = self.check_error().map(|()| {
            let state = self.state.lock();
            let offline_library_bytes: u64 = state
                .offline_library
                .iter()
                .map(|s| s.total_size_bytes)
                .sum();
            StorageOverview {
                offline_library_bytes,
                playback_cache_bytes: state.playback_cache_bytes,
                catalog_cache_bytes: state.catalog_cache_bytes,
            }
        });
        Self::ready(result)
    }

    fn offline_library(&self) -> GatewayFuture<Vec<OfflineSeriesView>> {
        let result = self
            .check_error()
            .map(|()| self.state.lock().offline_library.clone());
        Self::ready(result)
    }

    fn download_tasks(&self) -> GatewayFuture<Vec<DownloadTaskView>> {
        let result = self
            .check_error()
            .map(|()| self.state.lock().download_tasks.clone());
        Self::ready(result)
    }

    fn enqueue_episode_downloads(
        &self,
        request: EnqueueEpisodeDownloadsRequest,
    ) -> GatewayFuture<EnqueueEpisodeDownloadsResult> {
        let result = self.check_error().and_then(|()| {
            let mut state = self.state.lock();
            let detail = state
                .details
                .get(&request.series_id)
                .cloned()
                .ok_or_else(|| ContractError::NotFound {
                    resource: format!("series/{}", request.series_id),
                })?;
            let profile = state.download_policy.preferred_profile;

            let mut queued = Vec::new();
            let mut skipped_already_present = Vec::new();

            for ep in request.episodes {
                let key = EpisodeKey::new(request.series_id, ep);
                let already_offline = Self::has_offline_episode(&state, key).is_some();
                let already_queued = state.download_tasks.iter().any(|t| {
                    t.key == key
                        && !matches!(
                            t.state,
                            DownloadTaskState::Cancelled | DownloadTaskState::Completed
                        )
                });
                if already_offline || already_queued {
                    skipped_already_present.push(key);
                    continue;
                }
                let ep_title = detail
                    .episodes
                    .iter()
                    .find(|e| e.ep_num == ep)
                    .map_or_else(|| format!("第 {ep} 集"), |e| e.title.clone());
                state.download_tasks.push(DownloadTaskView {
                    key,
                    series_title: detail.summary.title.clone(),
                    episode_title: ep_title,
                    cover_url: detail.summary.cover_url.clone(),
                    profile,
                    state: DownloadTaskState::Queued,
                });
                queued.push(key);
            }

            Ok(EnqueueEpisodeDownloadsResult {
                queued,
                skipped_already_present,
            })
        });
        Self::ready(result)
    }

    fn control_download(&self, request: ControlDownloadRequest) -> GatewayFuture<()> {
        let result = self.check_error().and_then(|()| {
            let mut state = self.state.lock();
            let task = state
                .download_tasks
                .iter_mut()
                .find(|t| t.key == request.key)
                .ok_or_else(|| ContractError::NotFound {
                    resource: format!("download/{}/{}", request.key.series_id, request.key.ep_num),
                })?;

            match request.action {
                ControlDownloadAction::Pause => {
                    let (completed_segments, total_segments) = match task.state {
                        DownloadTaskState::Downloading {
                            completed_segments,
                            total_segments,
                            ..
                        }
                        | DownloadTaskState::Paused {
                            completed_segments,
                            total_segments,
                            ..
                        }
                        | DownloadTaskState::Failed {
                            completed_segments,
                            total_segments,
                            ..
                        } => (completed_segments, total_segments),
                        DownloadTaskState::Queued
                        | DownloadTaskState::FetchingSource
                        | DownloadTaskState::Completed
                        | DownloadTaskState::Cancelled => (0, 20),
                    };
                    task.state = DownloadTaskState::Paused {
                        reason: PauseReason::UserRequested,
                        completed_segments,
                        total_segments,
                    };
                }
                ControlDownloadAction::Resume | ControlDownloadAction::Retry => {
                    task.state = DownloadTaskState::Queued;
                }
                ControlDownloadAction::Cancel => {
                    task.state = DownloadTaskState::Cancelled;
                }
            }
            state
                .download_tasks
                .retain(|t| !matches!(t.state, DownloadTaskState::Cancelled));
            Ok(())
        });
        Self::ready(result)
    }

    fn control_all_downloads(&self, action: ControlAllDownloadsAction) -> GatewayFuture<()> {
        let result = self.check_error().map(|()| {
            let mut state = self.state.lock();
            for task in &mut state.download_tasks {
                match action {
                    ControlAllDownloadsAction::PauseAll => {
                        if let DownloadTaskState::Downloading {
                            completed_segments,
                            total_segments,
                            ..
                        } = task.state
                        {
                            task.state = DownloadTaskState::Paused {
                                reason: PauseReason::UserRequested,
                                completed_segments,
                                total_segments,
                            };
                        } else if matches!(
                            task.state,
                            DownloadTaskState::Queued | DownloadTaskState::FetchingSource
                        ) {
                            task.state = DownloadTaskState::Paused {
                                reason: PauseReason::UserRequested,
                                completed_segments: 0,
                                total_segments: 20,
                            };
                        }
                    }
                    ControlAllDownloadsAction::ResumeAll => {
                        if matches!(task.state, DownloadTaskState::Paused { .. }) {
                            task.state = DownloadTaskState::Queued;
                        }
                    }
                }
            }
        });
        Self::ready(result)
    }

    fn delete_offline_episode(&self, key: EpisodeKey) -> GatewayFuture<()> {
        let result = self.check_error().map(|()| {
            let mut state = self.state.lock();
            for series in &mut state.offline_library {
                if series.series_id == key.series_id {
                    series.episodes.retain(|e| e.ep_num != key.ep_num);
                    series.total_size_bytes = series.episodes.iter().map(|e| e.size_bytes).sum();
                }
            }
            state.offline_library.retain(|s| !s.episodes.is_empty());
        });
        Self::ready(result)
    }

    fn delete_offline_series(&self, series_id: SeriesId) -> GatewayFuture<()> {
        let result = self.check_error().map(|()| {
            self.state
                .lock()
                .offline_library
                .retain(|s| s.series_id != series_id);
        });
        Self::ready(result)
    }

    fn download_policy(&self) -> GatewayFuture<DownloadPolicy> {
        let result = self
            .check_error()
            .map(|()| self.state.lock().download_policy.clone());
        Self::ready(result)
    }

    fn update_download_policy(&self, policy: DownloadPolicy) -> GatewayFuture<()> {
        let result = self.check_error().map(|()| {
            self.state.lock().download_policy = policy;
        });
        Self::ready(result)
    }
}
