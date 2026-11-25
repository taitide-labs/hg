mod seed;

use std::collections::{HashMap, VecDeque};
use std::future;
use std::num::{NonZeroU32, NonZeroU64};
use std::sync::Arc;

use hg_contracts::catalog::{
    Category, CommentReply, EpisodeComment, EpisodeSummary, HomeFeed, HotKeyword, HotTag,
    ListCommentRepliesRequest, ListEpisodeCommentsRequest, ListRankingRequest, ListSeriesRequest,
    PagedResult, RankingEntry, RankingKind, SearchSeriesRequest, SearchSuggestion, SeriesDetail,
    SeriesFilter, SeriesSort, SeriesStats, SeriesSummary, TagGroup,
};
use hg_contracts::error::ContractError;
use hg_contracts::ids::{
    CommentId, EpisodeKey, EpisodeNumber, PageInfo, PageRequest, SeriesId, StreamProfile,
};
use hg_contracts::offline::{
    ControlAllDownloadsAction, ControlDownloadAction, ControlDownloadRequest,
    DownloadFailureReason, DownloadPolicy, DownloadTaskState, DownloadTaskView,
    EnqueueEpisodeDownloadsRequest, EnqueueEpisodeDownloadsResult, OfflineEpisodeView,
    OfflineSeriesView, PauseReason,
};
use hg_contracts::viewing::{
    DislikedSeriesEntry, OnlineStreamKind, PlaybackOrigin, PlaybackPreference, PlaybackSource,
    ReportWatchProgressRequest, ReportWatchProgressResult, SearchHistoryEntry, SeriesBookmark,
    StorageOverview, WatchHistoryEntry, WatchProgress,
};
use parking_lot::Mutex;
use tokio::sync::oneshot;

use crate::contract::{AppGateway, GatewayFuture};
use crate::mock::seed::{MockState, build_home_feed, build_initial_state};

pub struct MethodGate {
    tx: Option<oneshot::Sender<()>>,
}

impl MethodGate {
    pub fn release(mut self) {
        if let Some(tx) = self.tx.take()
            && tx.send(()).is_err()
        {}
    }
}

type MethodGateMap = HashMap<&'static str, VecDeque<oneshot::Receiver<()>>>;

#[derive(Clone)]
pub struct MockGateway {
    state: Arc<Mutex<MockState>>,
    injected_error: Arc<Mutex<Option<ContractError>>>,
    method_errors: Arc<Mutex<HashMap<&'static str, ContractError>>>,
    method_gates: Arc<Mutex<MethodGateMap>>,
    home_feed_cap: Option<usize>,
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
            method_errors: Arc::new(Mutex::new(HashMap::new())),
            method_gates: Arc::new(Mutex::new(HashMap::new())),
            home_feed_cap: None,
        }
    }

    #[must_use]
    pub const fn with_capped_home_feed(mut self, cap: usize) -> Self {
        self.home_feed_cap = Some(cap);
        self
    }

    #[must_use]
    pub fn with_tag_groups(self, groups: Vec<TagGroup>) -> Self {
        self.state.lock().tag_groups = groups;
        self
    }

    pub fn set_series_episode_count(&self, series_id: SeriesId, episode_count: u32) {
        let mut state = self.state.lock();
        if let Some(detail) = state.details.get_mut(&series_id) {
            detail.summary.episode_count = episode_count;
            detail.total_episodes = episode_count;
            detail.episodes.truncate(episode_count as usize);
        }
    }

    pub fn set_download_task_state(&self, key: EpisodeKey, next_state: DownloadTaskState) {
        let mut state = self.state.lock();
        if let Some(task) = state.download_tasks.iter_mut().find(|t| t.key == key) {
            task.state = next_state;
        }
    }

    pub fn complete_download_task(&self, key: EpisodeKey) {
        let mut state = self.state.lock();
        let Some(pos) = state.download_tasks.iter().position(|t| t.key == key) else {
            return;
        };
        let task = state.download_tasks.remove(pos);
        state.next_timestamp = state.next_timestamp.saturating_add(1);
        let downloaded_at = state.next_timestamp;
        let ep_view = OfflineEpisodeView {
            ep_num: key.ep_num,
            title: task.episode_title,
            duration_secs: 120,
            profile: task.profile,
            size_bytes: 18_500_000,
            downloaded_at_epoch_secs: downloaded_at,
        };
        if let Some(series) = state
            .offline_library
            .iter_mut()
            .find(|s| s.series_id == key.series_id)
        {
            series.episodes.retain(|ep| ep.ep_num != key.ep_num);
            series.episodes.push(ep_view);
            series.episodes.sort_by_key(|ep| ep.ep_num.get());
            series.total_size_bytes = series.episodes.iter().map(|ep| ep.size_bytes).sum();
        } else {
            let total_episodes = state
                .details
                .get(&key.series_id)
                .map_or(key.ep_num.get(), |d| d.total_episodes.max(key.ep_num.get()));
            state.offline_library.push(OfflineSeriesView {
                series_id: key.series_id,
                title: task.series_title,
                cover_local_uri: task.cover_url,
                total_episodes,
                total_size_bytes: ep_view.size_bytes,
                episodes: vec![ep_view],
            });
        }
    }

    #[must_use]
    pub fn with_paged_catalog(self, count: u64) -> Self {
        let mut state = self.state.lock();
        for idx in 1..=count {
            let id = SeriesId::new(NonZeroU64::new(2000 + idx).unwrap_or(NonZeroU64::MIN));
            let title = format!("分页剧目 {idx:02}");
            let hot = 100_000_u64.saturating_sub(idx);
            let episodes = (1..=12_u32)
                .map(|n| EpisodeSummary {
                    ep_num: EpisodeNumber::new(NonZeroU32::new(n).unwrap_or(NonZeroU32::MIN)),
                    title: format!("第 {n} 集 · 分页剧情 {n}"),
                    duration_secs: 95,
                    playable: true,
                })
                .collect();
            state.details.insert(
                id,
                SeriesDetail {
                    summary: SeriesSummary {
                        id,
                        title,
                        cover_url: "poster://amber-noir".to_owned(),
                        score: 9.0,
                        tags: vec!["都市".to_owned(), "悬疑".to_owned()],
                        description: "分页测试剧目".to_owned(),
                        episode_count: 12,
                        is_finished: true,
                        hot,
                        is_original: false,
                        is_hot: false,
                        is_new: false,
                    },
                    total_episodes: 12,
                    stats: SeriesStats {
                        play_heat: hot,
                        likes: 1000,
                        bookmarks: 500,
                        comments: 200,
                    },
                    episodes,
                },
            );
        }
        drop(state);
        self
    }

    #[must_use]
    pub fn with_paged_comments(
        self,
        series_id: SeriesId,
        ep_num: EpisodeNumber,
        total: u64,
    ) -> Self {
        let comments = (1..=total)
            .map(|idx| {
                let preview_replies = if idx == 1 {
                    (1..=3_u64)
                        .map(|r| CommentReply {
                            id: CommentId::new(
                                NonZeroU64::new(7000 + r).unwrap_or(NonZeroU64::MIN),
                            ),
                            nickname: format!("回复用户{r}"),
                            avatar_color: "#0F766E".to_owned(),
                            text: format!("分页回复内容 #{r}"),
                            relative_time: "2 分钟前".to_owned(),
                            like_count: 12,
                            reply_to_nickname: Some("评论用户1".to_owned()),
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                EpisodeComment {
                    id: CommentId::new(NonZeroU64::new(6000 + idx).unwrap_or(NonZeroU64::MIN)),
                    nickname: format!("评论用户{idx}"),
                    avatar_color: "#0F766E".to_owned(),
                    text: format!("分页评论内容 #{idx}"),
                    relative_time: "5 分钟前".to_owned(),
                    like_count: 10,
                    is_pinned: idx == 1,
                    ip_region: None,
                    reply_count: if idx == 1 { 3 } else { 0 },
                    preview_replies,
                }
            })
            .collect();
        self.state
            .lock()
            .comments
            .insert(EpisodeKey::new(series_id, ep_num), comments);
        self
    }

    #[must_use]
    pub fn with_empty_download_queue(self) -> Self {
        self.state.lock().download_tasks.clear();
        self
    }

    #[must_use]
    pub fn with_empty_offline_library(self) -> Self {
        self.state.lock().offline_library.clear();
        self
    }

    #[must_use]
    pub fn with_empty_bookmarks(self) -> Self {
        self.state.lock().bookmarks.clear();
        self
    }

    #[must_use]
    pub fn with_empty_watch_history(self) -> Self {
        self.state.lock().watch_history.clear();
        self
    }

    #[must_use]
    pub fn with_empty_search_history(self) -> Self {
        self.state.lock().search_history.clear();
        self
    }

    #[must_use]
    pub fn with_playback_preference(self, preference: PlaybackPreference) -> Self {
        self.state.lock().playback_preference = preference;
        self
    }

    #[must_use]
    pub fn with_download_policy(self, policy: DownloadPolicy) -> Self {
        self.state.lock().download_policy = policy;
        self
    }

    #[must_use]
    pub fn with_failed_download_task(self) -> Self {
        let series_id = SeriesId::new(NonZeroU64::new(1003).unwrap_or(NonZeroU64::MIN));
        let ep_num = EpisodeNumber::new(NonZeroU32::MIN);
        self.state.lock().download_tasks.push(DownloadTaskView {
            key: EpisodeKey::new(series_id, ep_num),
            series_title: "重返 1998：科技大亨".to_owned(),
            episode_title: "第 1 集 · 时代浪潮转折 1".to_owned(),
            cover_url: "poster://slate-gold".to_owned(),
            profile: StreamProfile::Hevc,
            state: DownloadTaskState::Failed {
                reason: DownloadFailureReason::NetworkError,
                completed_segments: 4,
                total_segments: 20,
            },
        });
        self
    }

    #[must_use]
    pub fn with_dirty_zero_total_episodes(self, series_id: SeriesId) -> Self {
        if let Some(detail) = self.state.lock().details.get_mut(&series_id) {
            detail.total_episodes = 0;
        }
        self
    }

    pub fn inject_error(&self, error: Option<ContractError>) {
        *self.injected_error.lock() = error;
    }

    pub fn inject_method_error(&self, method: &'static str, error: Option<ContractError>) {
        let mut errors = self.method_errors.lock();
        if let Some(err) = error {
            errors.insert(method, err);
        } else {
            errors.remove(method);
        }
    }

    #[must_use]
    pub fn queue_method_gate(&self, method: &'static str) -> MethodGate {
        let (tx, rx) = oneshot::channel();
        self.method_gates
            .lock()
            .entry(method)
            .or_default()
            .push_back(rx);
        MethodGate { tx: Some(tx) }
    }

    #[must_use]
    pub fn snapshot_preference(&self) -> PlaybackPreference {
        self.state.lock().playback_preference
    }

    #[must_use]
    pub fn snapshot_download_policy(&self) -> DownloadPolicy {
        self.state.lock().download_policy.clone()
    }

    #[must_use]
    pub fn snapshot_bookmarks(&self) -> Vec<SeriesBookmark> {
        self.state.lock().bookmarks.clone()
    }

    #[must_use]
    pub fn snapshot_disliked(&self) -> Vec<DislikedSeriesEntry> {
        self.state.lock().disliked.clone()
    }

    #[must_use]
    pub fn snapshot_watch_history(&self) -> Vec<WatchHistoryEntry> {
        self.state.lock().watch_history.clone()
    }

    #[must_use]
    pub fn snapshot_search_history(&self) -> Vec<SearchHistoryEntry> {
        self.state.lock().search_history.clone()
    }

    #[must_use]
    pub fn snapshot_download_tasks(&self) -> Vec<DownloadTaskView> {
        self.state.lock().download_tasks.clone()
    }

    #[must_use]
    pub fn snapshot_offline_library(&self) -> Vec<OfflineSeriesView> {
        self.state.lock().offline_library.clone()
    }

    #[must_use]
    pub fn snapshot_storage(&self) -> StorageOverview {
        let state = self.state.lock();
        let offline_library_bytes = state
            .offline_library
            .iter()
            .map(|s| s.total_size_bytes)
            .sum();
        StorageOverview {
            offline_library_bytes,
            playback_cache_bytes: state.playback_cache_bytes,
            catalog_cache_bytes: state.catalog_cache_bytes,
        }
    }

    fn check_error(&self, method: &'static str) -> Result<(), ContractError> {
        if let Some(error) = self.method_errors.lock().get(method).cloned() {
            return Err(error);
        }
        if let Some(error) = self.injected_error.lock().clone() {
            return Err(error);
        }
        Ok(())
    }

    fn finish<T: Send + 'static>(
        &self,
        method: &'static str,
        result: Result<T, ContractError>,
    ) -> GatewayFuture<T> {
        let gate = self
            .method_gates
            .lock()
            .get_mut(method)
            .and_then(VecDeque::pop_front);
        match gate {
            Some(rx) => Box::pin(async move {
                drop(rx.await);
                result
            }),
            None => Box::pin(future::ready(result)),
        }
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
}

impl AppGateway for MockGateway {
    fn home_feed(&self) -> GatewayFuture<HomeFeed> {
        let cap = self.home_feed_cap;
        let result = self.check_error("home_feed").map(|()| {
            let state = self.state.lock();
            let mut feed = build_home_feed(&state);
            if let Some(limit) = cap {
                feed.featured.truncate(limit);
                feed.latest.truncate(limit);
                for rec in &mut feed.category_recommendations {
                    rec.items.truncate(limit);
                }
            }
            feed
        });
        self.finish("home_feed", result)
    }

    fn categories(&self) -> GatewayFuture<Vec<Category>> {
        let result = self.check_error("categories").map(|()| {
            let state = self.state.lock();
            state.categories.clone()
        });
        self.finish("categories", result)
    }

    fn tag_groups(&self) -> GatewayFuture<Vec<TagGroup>> {
        let result = self.check_error("tag_groups").map(|()| {
            let state = self.state.lock();
            state.tag_groups.clone()
        });
        self.finish("tag_groups", result)
    }

    fn list_series(&self, request: ListSeriesRequest) -> GatewayFuture<PagedResult<SeriesSummary>> {
        let result = self.check_error("list_series").map(|()| {
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
        self.finish("list_series", result)
    }

    fn list_ranking(
        &self,
        request: ListRankingRequest,
    ) -> GatewayFuture<PagedResult<RankingEntry>> {
        let result = self.check_error("list_ranking").map(|()| {
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
        self.finish("list_ranking", result)
    }

    fn search_series(
        &self,
        request: SearchSeriesRequest,
    ) -> GatewayFuture<PagedResult<SeriesSummary>> {
        let result = self.check_error("search_series").map(|()| {
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
        self.finish("search_series", result)
    }

    fn search_suggestions(&self, keyword: String) -> GatewayFuture<SearchSuggestion> {
        let result = self.check_error("search_suggestions").map(|()| {
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
        self.finish("search_suggestions", result)
    }

    fn hot_keywords(&self) -> GatewayFuture<Vec<HotKeyword>> {
        let result = self
            .check_error("hot_keywords")
            .map(|()| self.state.lock().hot_keywords.clone());
        self.finish("hot_keywords", result)
    }

    fn hot_tags(&self) -> GatewayFuture<Vec<HotTag>> {
        let result = self
            .check_error("hot_tags")
            .map(|()| self.state.lock().hot_tags.clone());
        self.finish("hot_tags", result)
    }

    fn series_detail(&self, series_id: SeriesId) -> GatewayFuture<SeriesDetail> {
        let result = self.check_error("series_detail").and_then(|()| {
            let state = self.state.lock();
            state
                .details
                .get(&series_id)
                .cloned()
                .ok_or_else(|| ContractError::NotFound {
                    resource: format!("series/{series_id}"),
                })
        });
        self.finish("series_detail", result)
    }

    fn list_episode_comments(
        &self,
        request: ListEpisodeCommentsRequest,
    ) -> GatewayFuture<PagedResult<EpisodeComment>> {
        let result = self.check_error("list_episode_comments").map(|()| {
            let state = self.state.lock();
            let key = EpisodeKey::new(request.series_id, request.ep_num);
            let items = state.comments.get(&key).cloned().unwrap_or_default();
            Self::paginate(&items, request.page)
        });
        self.finish("list_episode_comments", result)
    }

    fn list_comment_replies(
        &self,
        request: ListCommentRepliesRequest,
    ) -> GatewayFuture<PagedResult<CommentReply>> {
        let result = self.check_error("list_comment_replies").map(|()| {
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
        self.finish("list_comment_replies", result)
    }

    fn clear_catalog_cache(&self) -> GatewayFuture<()> {
        let result = self.check_error("clear_catalog_cache").map(|()| {
            let mut state = self.state.lock();
            state.catalog_cache_bytes = 0;
        });
        self.finish("clear_catalog_cache", result)
    }

    fn prepare_episode_playback(&self, key: EpisodeKey) -> GatewayFuture<PlaybackSource> {
        let result = self.check_error("prepare_episode_playback").and_then(|()| {
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
        self.finish("prepare_episode_playback", result)
    }

    fn refresh_online_playback(&self, key: EpisodeKey) -> GatewayFuture<PlaybackSource> {
        self.prepare_episode_playback(key)
    }

    fn report_watch_progress(
        &self,
        request: ReportWatchProgressRequest,
    ) -> GatewayFuture<ReportWatchProgressResult> {
        let result = self.check_error("report_watch_progress").and_then(|()| {
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

            let progress = WatchProgress {
                key: request.key,
                position_secs: request.position_secs,
                duration_secs: request.duration_secs,
                completed,
            };
            let prev_completed = state
                .episode_progress
                .get(&request.key)
                .is_some_and(|p| p.completed);
            state.episode_progress.insert(
                request.key,
                WatchProgress {
                    completed: completed || prev_completed,
                    ..progress
                },
            );

            Ok(ReportWatchProgressResult {
                progress,
                next_episode,
            })
        });
        self.finish("report_watch_progress", result)
    }

    fn watch_history(&self) -> GatewayFuture<Vec<WatchHistoryEntry>> {
        let result = self
            .check_error("watch_history")
            .map(|()| self.state.lock().watch_history.clone());
        self.finish("watch_history", result)
    }

    fn episode_progress(&self, series_id: SeriesId) -> GatewayFuture<Vec<WatchProgress>> {
        let result = self.check_error("episode_progress").map(|()| {
            let mut items: Vec<WatchProgress> = self
                .state
                .lock()
                .episode_progress
                .values()
                .filter(|p| p.key.series_id == series_id)
                .copied()
                .collect();
            items.sort_by_key(|p| p.key.ep_num.get());
            items
        });
        self.finish("episode_progress", result)
    }

    fn remove_watch_history(&self, series_id: SeriesId) -> GatewayFuture<()> {
        let result = self.check_error("remove_watch_history").map(|()| {
            let mut state = self.state.lock();
            state.watch_history.retain(|e| e.series_id != series_id);
            state
                .episode_progress
                .retain(|k, _| k.series_id != series_id);
        });
        self.finish("remove_watch_history", result)
    }

    fn clear_watch_history(&self) -> GatewayFuture<()> {
        let result = self.check_error("clear_watch_history").map(|()| {
            let mut state = self.state.lock();
            state.watch_history.clear();
            state.episode_progress.clear();
        });
        self.finish("clear_watch_history", result)
    }

    fn bookmarks(&self) -> GatewayFuture<Vec<SeriesBookmark>> {
        let result = self
            .check_error("bookmarks")
            .map(|()| self.state.lock().bookmarks.clone());
        self.finish("bookmarks", result)
    }

    fn is_bookmarked(&self, series_id: SeriesId) -> GatewayFuture<bool> {
        let result = self.check_error("is_bookmarked").map(|()| {
            self.state
                .lock()
                .bookmarks
                .iter()
                .any(|b| b.series_id == series_id)
        });
        self.finish("is_bookmarked", result)
    }

    fn bookmark_series(&self, series_id: SeriesId) -> GatewayFuture<()> {
        let result =
            self.check_error("bookmark_series").and_then(|()| {
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
        self.finish("bookmark_series", result)
    }

    fn unbookmark_series(&self, series_id: SeriesId) -> GatewayFuture<()> {
        let result = self.check_error("unbookmark_series").map(|()| {
            self.state
                .lock()
                .bookmarks
                .retain(|b| b.series_id != series_id);
        });
        self.finish("unbookmark_series", result)
    }

    fn disliked_series(&self) -> GatewayFuture<Vec<DislikedSeriesEntry>> {
        let result = self
            .check_error("disliked_series")
            .map(|()| self.state.lock().disliked.clone());
        self.finish("disliked_series", result)
    }

    fn is_disliked(&self, series_id: SeriesId) -> GatewayFuture<bool> {
        let result = self.check_error("is_disliked").map(|()| {
            self.state
                .lock()
                .disliked
                .iter()
                .any(|d| d.series_id == series_id)
        });
        self.finish("is_disliked", result)
    }

    fn dislike_series(&self, series_id: SeriesId) -> GatewayFuture<()> {
        let result =
            self.check_error("dislike_series").and_then(|()| {
                let mut state = self.state.lock();
                if state.disliked.iter().any(|d| d.series_id == series_id) {
                    return Ok(());
                }
                let detail = state.details.get(&series_id).cloned().ok_or_else(|| {
                    ContractError::NotFound {
                        resource: format!("series/{series_id}"),
                    }
                })?;
                state.next_timestamp += 1;
                let ts = state.next_timestamp;
                state.disliked.insert(
                    0,
                    DislikedSeriesEntry {
                        series_id,
                        title: detail.summary.title,
                        cover_url: detail.summary.cover_url,
                        episode_count: detail.summary.episode_count,
                        is_finished: detail.summary.is_finished,
                        score: detail.summary.score,
                        disliked_at_epoch_secs: ts,
                    },
                );
                Ok(())
            });
        self.finish("dislike_series", result)
    }

    fn undislike_series(&self, series_id: SeriesId) -> GatewayFuture<()> {
        let result = self.check_error("undislike_series").map(|()| {
            self.state
                .lock()
                .disliked
                .retain(|d| d.series_id != series_id);
        });
        self.finish("undislike_series", result)
    }

    fn clear_disliked_series(&self) -> GatewayFuture<()> {
        let result = self.check_error("clear_disliked_series").map(|()| {
            self.state.lock().disliked.clear();
        });
        self.finish("clear_disliked_series", result)
    }

    fn playback_preference(&self) -> GatewayFuture<PlaybackPreference> {
        let result = self
            .check_error("playback_preference")
            .map(|()| self.state.lock().playback_preference);
        self.finish("playback_preference", result)
    }

    fn update_playback_preference(&self, preference: PlaybackPreference) -> GatewayFuture<()> {
        let result = self.check_error("update_playback_preference").map(|()| {
            self.state.lock().playback_preference = preference;
        });
        self.finish("update_playback_preference", result)
    }

    fn search_history(&self) -> GatewayFuture<Vec<SearchHistoryEntry>> {
        let result = self
            .check_error("search_history")
            .map(|()| self.state.lock().search_history.clone());
        self.finish("search_history", result)
    }

    fn record_search_history(&self, keyword: String) -> GatewayFuture<()> {
        let result = self.check_error("record_search_history").map(|()| {
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
        self.finish("record_search_history", result)
    }

    fn remove_search_history(&self, keyword: String) -> GatewayFuture<()> {
        let result = self.check_error("remove_search_history").map(|()| {
            let trimmed = keyword.trim();
            self.state
                .lock()
                .search_history
                .retain(|entry| entry.keyword != trimmed);
        });
        self.finish("remove_search_history", result)
    }

    fn clear_search_history(&self) -> GatewayFuture<()> {
        let result = self.check_error("clear_search_history").map(|()| {
            self.state.lock().search_history.clear();
        });
        self.finish("clear_search_history", result)
    }

    fn clear_playback_cache(&self) -> GatewayFuture<()> {
        let result = self.check_error("clear_playback_cache").map(|()| {
            self.state.lock().playback_cache_bytes = 0;
        });
        self.finish("clear_playback_cache", result)
    }

    fn storage_overview(&self) -> GatewayFuture<StorageOverview> {
        let result = self.check_error("storage_overview").map(|()| {
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
        self.finish("storage_overview", result)
    }

    fn offline_library(&self) -> GatewayFuture<Vec<OfflineSeriesView>> {
        let result = self
            .check_error("offline_library")
            .map(|()| self.state.lock().offline_library.clone());
        self.finish("offline_library", result)
    }

    fn download_tasks(&self) -> GatewayFuture<Vec<DownloadTaskView>> {
        let result = self
            .check_error("download_tasks")
            .map(|()| self.state.lock().download_tasks.clone());
        self.finish("download_tasks", result)
    }

    fn enqueue_episode_downloads(
        &self,
        request: EnqueueEpisodeDownloadsRequest,
    ) -> GatewayFuture<EnqueueEpisodeDownloadsResult> {
        let result = self
            .check_error("enqueue_episode_downloads")
            .and_then(|()| {
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
        self.finish("enqueue_episode_downloads", result)
    }

    fn control_download(&self, request: ControlDownloadRequest) -> GatewayFuture<()> {
        let result = self.check_error("control_download").and_then(|()| {
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
        self.finish("control_download", result)
    }

    fn control_all_downloads(&self, action: ControlAllDownloadsAction) -> GatewayFuture<()> {
        let result = self.check_error("control_all_downloads").map(|()| {
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
        self.finish("control_all_downloads", result)
    }

    fn delete_offline_episode(&self, key: EpisodeKey) -> GatewayFuture<()> {
        let result = self.check_error("delete_offline_episode").map(|()| {
            let mut state = self.state.lock();
            for series in &mut state.offline_library {
                if series.series_id == key.series_id {
                    series.episodes.retain(|e| e.ep_num != key.ep_num);
                    series.total_size_bytes = series.episodes.iter().map(|e| e.size_bytes).sum();
                }
            }
            state.offline_library.retain(|s| !s.episodes.is_empty());
        });
        self.finish("delete_offline_episode", result)
    }

    fn delete_offline_series(&self, series_id: SeriesId) -> GatewayFuture<()> {
        let result = self.check_error("delete_offline_series").map(|()| {
            self.state
                .lock()
                .offline_library
                .retain(|s| s.series_id != series_id);
        });
        self.finish("delete_offline_series", result)
    }

    fn download_policy(&self) -> GatewayFuture<DownloadPolicy> {
        let result = self
            .check_error("download_policy")
            .map(|()| self.state.lock().download_policy.clone());
        self.finish("download_policy", result)
    }

    fn update_download_policy(&self, policy: DownloadPolicy) -> GatewayFuture<()> {
        let result = self.check_error("update_download_policy").map(|()| {
            self.state.lock().download_policy = policy;
        });
        self.finish("update_download_policy", result)
    }
}
