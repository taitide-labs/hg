use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;

use hg_contracts::catalog::{
    Category, CommentReply, EpisodeComment, HomeFeed, HotKeyword, HotTag,
    ListCommentRepliesRequest, ListEpisodeCommentsRequest, ListRankingRequest, ListSeriesRequest,
    PagedResult, RankingEntry, SearchSeriesRequest, SearchSuggestion, SeriesDetail, SeriesSummary,
    TagGroup,
};
use hg_contracts::error::ContractError;
use hg_contracts::ids::{EpisodeKey, SeriesId};
use hg_contracts::offline::{
    ControlAllDownloadsAction, ControlDownloadRequest, DownloadPolicy, DownloadTaskView,
    EnqueueEpisodeDownloadsRequest, EnqueueEpisodeDownloadsResult, OfflineSeriesView,
};
use hg_contracts::viewing::{
    PlaybackPreference, PlaybackSource, ReportWatchProgressRequest, ReportWatchProgressResult,
    SeriesBookmark, StorageOverview, WatchHistoryEntry,
};
use hg_runtime::catalog::upstream::{CatalogUpstream, EdgeCatalogUpstream};
use hg_runtime::catalog::{cmd_playback_and_cache, qry_discovery, qry_series_detail};
use hg_runtime::infra::clock::{Clock, SystemClock};
use hg_runtime::infra::edge_crypto::EdgeHttpClient;
use hg_runtime::offline::fetcher::{HttpSegmentFetcher, OfflineSegmentFetcher};
use hg_runtime::offline::{
    cmd_download_queue, cmd_offline_library, qry_download_queue, qry_offline_library,
};
use hg_runtime::viewing::{
    cmd_playback_session, cmd_record_watch_progress, cmd_watchlist_and_settings, qry_watch_history,
    qry_watchlist_and_settings,
};
use hg_runtime::{AssembledContexts, BackendRuntime, BackendRuntimeConfig};
use tokio::runtime::{Handle, Runtime};

use crate::contract::{AppGateway, GatewayFuture};

pub const DEFAULT_UPSTREAM_BASE_URL: &str = "https://huangguoai.com";

#[derive(Debug, Clone)]
pub struct LocalGatewayConfig {
    pub db_path: PathBuf,
    pub data_root: PathBuf,
    pub cache_root: PathBuf,
    pub upstream_base_url: String,
}

impl LocalGatewayConfig {
    #[must_use]
    pub fn from_env() -> Self {
        let data_root = env_path("HG_DATA_DIR").unwrap_or_else(default_data_dir);
        let cache_root = env_path("HG_CACHE_DIR").unwrap_or_else(default_cache_dir);
        Self {
            db_path: data_root.join("hg.sqlite"),
            data_root,
            cache_root,
            upstream_base_url: resolve_upstream_base_url(),
        }
    }

    #[must_use]
    pub fn from_temp_fallback() -> Self {
        let base = std::env::temp_dir().join("com.taitidelabs.hg");
        let data_root = base.join("data");
        let cache_root = base.join("cache");
        Self {
            db_path: data_root.join("hg.sqlite"),
            data_root,
            cache_root,
            upstream_base_url: resolve_upstream_base_url(),
        }
    }
}

#[expect(
    clippy::disallowed_methods,
    reason = "客户端启动配置根读取可选上游地址覆盖变量"
)]
fn resolve_upstream_base_url() -> String {
    std::env::var("HG_UPSTREAM_BASE_URL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_UPSTREAM_BASE_URL.to_owned())
}

fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var_os(key)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

fn default_data_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    if let Some(home) = env_path("HOME") {
        return home
            .join("Library")
            .join("Application Support")
            .join("com.taitidelabs.hg");
    }

    #[cfg(target_os = "windows")]
    if let Some(local_app_data) = env_path("LOCALAPPDATA") {
        return local_app_data.join("com.taitidelabs.hg").join("data");
    }

    if let Some(xdg) = env_path("XDG_DATA_HOME") {
        return xdg.join("com.taitidelabs.hg");
    }
    if let Some(home) = env_path("HOME") {
        return home.join(".local").join("share").join("com.taitidelabs.hg");
    }
    std::env::temp_dir().join("com.taitidelabs.hg").join("data")
}

fn default_cache_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    if let Some(home) = env_path("HOME") {
        return home
            .join("Library")
            .join("Caches")
            .join("com.taitidelabs.hg");
    }

    #[cfg(target_os = "windows")]
    if let Some(local_app_data) = env_path("LOCALAPPDATA") {
        return local_app_data.join("com.taitidelabs.hg").join("cache");
    }

    if let Some(xdg) = env_path("XDG_CACHE_HOME") {
        return xdg.join("com.taitidelabs.hg");
    }
    if let Some(home) = env_path("HOME") {
        return home.join(".cache").join("com.taitidelabs.hg");
    }
    std::env::temp_dir()
        .join("com.taitidelabs.hg")
        .join("cache")
}

pub struct LocalGateway {
    runtime: BackendRuntime,
    contexts: AssembledContexts,
    tokio_handle: Handle,
    _owned_tokio: Option<Arc<Runtime>>,
}

impl std::fmt::Debug for LocalGateway {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalGateway")
            .field("loopback_base_url", &self.runtime.loopback().base_url())
            .finish()
    }
}

impl LocalGateway {
    pub fn open_default() -> Result<Self, ContractError> {
        let config = LocalGatewayConfig::from_env();
        Self::open_with_config(&config)
            .or_else(|_| Self::open_with_config(&LocalGatewayConfig::from_temp_fallback()))
    }

    pub fn open_with_config(config: &LocalGatewayConfig) -> Result<Self, ContractError> {
        let runtime_config = BackendRuntimeConfig {
            db_path: config.db_path.clone(),
            data_root: config.data_root.clone(),
            cache_root: config.cache_root.clone(),
        };
        Self::open(&runtime_config, &config.upstream_base_url)
    }

    pub fn open(
        config: &BackendRuntimeConfig,
        upstream_base_url: &str,
    ) -> Result<Self, ContractError> {
        let tokio_rt = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_name("hg-local-backend")
                .build()
                .map_err(|err| ContractError::Internal {
                    detail: format!("创建本地后台运行时失败: {err}"),
                })?,
        );
        let clock: Arc<dyn Clock> = Arc::new(SystemClock);
        let edge_client =
            EdgeHttpClient::new(upstream_base_url, Arc::clone(&clock)).map_err(|err| {
                ContractError::Internal {
                    detail: format!("初始化边缘加密客户端失败: {err}"),
                }
            })?;
        let upstream: Arc<dyn CatalogUpstream> = Arc::new(EdgeCatalogUpstream::new(edge_client));
        let segment_fetcher: Arc<dyn OfflineSegmentFetcher> =
            Arc::new(HttpSegmentFetcher::new().map_err(ContractError::from)?);

        let (runtime, contexts) = tokio_rt.block_on(async {
            let runtime = BackendRuntime::open(config, clock).await.map_err(|err| {
                ContractError::Internal {
                    detail: format!("启动本地后端运行时失败: {err}"),
                }
            })?;
            let contexts = runtime.assemble_contexts(upstream, segment_fetcher);
            let _dispatched =
                cmd_offline_library::dispatch_pending_outbox(&contexts.offline).await?;
            Ok::<_, ContractError>((runtime, contexts))
        })?;

        let tokio_handle = tokio_rt.handle().clone();
        Ok(Self {
            runtime,
            contexts,
            tokio_handle,
            _owned_tokio: Some(tokio_rt),
        })
    }

    #[must_use]
    pub const fn from_assembled(
        runtime: BackendRuntime,
        contexts: AssembledContexts,
        tokio_handle: Handle,
    ) -> Self {
        Self {
            runtime,
            contexts,
            tokio_handle,
            _owned_tokio: None,
        }
    }

    #[must_use]
    pub const fn runtime(&self) -> &BackendRuntime {
        &self.runtime
    }

    #[must_use]
    pub const fn contexts(&self) -> &AssembledContexts {
        &self.contexts
    }

    pub fn process_next_download(&self) -> GatewayFuture<Option<EpisodeKey>> {
        let offline = self.contexts.offline.clone();
        self.dispatch(async move { cmd_download_queue::execute_next_queued_task(&offline).await })
    }

    pub fn spawn_background_download_pump(&self) {
        let offline = self.contexts.offline.clone();
        self.tokio_handle.spawn(async move {
            loop {
                match cmd_download_queue::execute_next_queued_task(&offline).await {
                    Ok(Some(_)) => {}
                    Ok(None) | Err(_) => {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }
                }
            }
        });
    }

    fn dispatch<T, F>(&self, future: F) -> GatewayFuture<T>
    where
        T: Send + 'static,
        F: Future<Output = Result<T, ContractError>> + Send + 'static,
    {
        let join = self.tokio_handle.spawn(future);
        Box::pin(async move {
            match join.await {
                Ok(result) => result,
                Err(err) => Err(ContractError::Internal {
                    detail: format!("本地运行时任务执行中断: {err}"),
                }),
            }
        })
    }
}

impl Drop for LocalGateway {
    fn drop(&mut self) {
        if self._owned_tokio.is_some() {
            self.runtime.loopback().shutdown();
        }
    }
}

impl AppGateway for LocalGateway {
    fn home_feed(&self) -> GatewayFuture<HomeFeed> {
        let catalog = self.contexts.catalog.clone();
        self.dispatch(async move { qry_discovery::home_feed(&catalog).await })
    }

    fn categories(&self) -> GatewayFuture<Vec<Category>> {
        let catalog = self.contexts.catalog.clone();
        self.dispatch(async move { qry_discovery::categories(&catalog).await })
    }

    fn tag_groups(&self) -> GatewayFuture<Vec<TagGroup>> {
        let catalog = self.contexts.catalog.clone();
        self.dispatch(async move { qry_discovery::tag_groups(&catalog).await })
    }

    fn list_series(&self, request: ListSeriesRequest) -> GatewayFuture<PagedResult<SeriesSummary>> {
        let catalog = self.contexts.catalog.clone();
        self.dispatch(async move { qry_discovery::list_series(&catalog, request).await })
    }

    fn list_ranking(
        &self,
        request: ListRankingRequest,
    ) -> GatewayFuture<PagedResult<RankingEntry>> {
        let catalog = self.contexts.catalog.clone();
        self.dispatch(async move { qry_discovery::list_ranking(&catalog, request).await })
    }

    fn search_series(
        &self,
        request: SearchSeriesRequest,
    ) -> GatewayFuture<PagedResult<SeriesSummary>> {
        let catalog = self.contexts.catalog.clone();
        self.dispatch(async move { qry_discovery::search_series(&catalog, request).await })
    }

    fn search_suggestions(&self, keyword: String) -> GatewayFuture<SearchSuggestion> {
        let catalog = self.contexts.catalog.clone();
        self.dispatch(async move { qry_discovery::search_suggestions(&catalog, keyword).await })
    }

    fn hot_keywords(&self) -> GatewayFuture<Vec<HotKeyword>> {
        let catalog = self.contexts.catalog.clone();
        self.dispatch(async move { qry_discovery::hot_keywords(&catalog).await })
    }

    fn hot_tags(&self) -> GatewayFuture<Vec<HotTag>> {
        let catalog = self.contexts.catalog.clone();
        self.dispatch(async move { qry_discovery::hot_tags(&catalog).await })
    }

    fn series_detail(&self, series_id: SeriesId) -> GatewayFuture<SeriesDetail> {
        let catalog = self.contexts.catalog.clone();
        self.dispatch(async move { qry_series_detail::series_detail(&catalog, series_id).await })
    }

    fn list_episode_comments(
        &self,
        request: ListEpisodeCommentsRequest,
    ) -> GatewayFuture<PagedResult<EpisodeComment>> {
        let catalog = self.contexts.catalog.clone();
        self.dispatch(
            async move { qry_series_detail::list_episode_comments(&catalog, request).await },
        )
    }

    fn list_comment_replies(
        &self,
        request: ListCommentRepliesRequest,
    ) -> GatewayFuture<PagedResult<CommentReply>> {
        let catalog = self.contexts.catalog.clone();
        self.dispatch(
            async move { qry_series_detail::list_comment_replies(&catalog, request).await },
        )
    }

    fn clear_catalog_cache(&self) -> GatewayFuture<()> {
        let catalog = self.contexts.catalog.clone();
        self.dispatch(async move { cmd_playback_and_cache::clear_catalog_cache(&catalog).await })
    }

    fn prepare_episode_playback(&self, key: EpisodeKey) -> GatewayFuture<PlaybackSource> {
        let viewing = self.contexts.viewing.clone();
        self.dispatch(
            async move { cmd_playback_session::prepare_episode_playback(&viewing, key).await },
        )
    }

    fn refresh_online_playback(&self, key: EpisodeKey) -> GatewayFuture<PlaybackSource> {
        let viewing = self.contexts.viewing.clone();
        self.dispatch(
            async move { cmd_playback_session::refresh_online_playback(&viewing, key).await },
        )
    }

    fn report_watch_progress(
        &self,
        request: ReportWatchProgressRequest,
    ) -> GatewayFuture<ReportWatchProgressResult> {
        let viewing = self.contexts.viewing.clone();
        self.dispatch(async move { cmd_record_watch_progress::execute(&viewing, request).await })
    }

    fn watch_history(&self) -> GatewayFuture<Vec<WatchHistoryEntry>> {
        let viewing = self.contexts.viewing.clone();
        self.dispatch(async move { qry_watch_history::execute(&viewing).await })
    }

    fn remove_watch_history(&self, series_id: SeriesId) -> GatewayFuture<()> {
        let viewing = self.contexts.viewing.clone();
        self.dispatch(async move {
            cmd_watchlist_and_settings::remove_watch_history(&viewing, series_id).await
        })
    }

    fn clear_watch_history(&self) -> GatewayFuture<()> {
        let viewing = self.contexts.viewing.clone();
        self.dispatch(
            async move { cmd_watchlist_and_settings::clear_watch_history(&viewing).await },
        )
    }

    fn bookmarks(&self) -> GatewayFuture<Vec<SeriesBookmark>> {
        let viewing = self.contexts.viewing.clone();
        self.dispatch(async move { qry_watchlist_and_settings::bookmarks(&viewing).await })
    }

    fn is_bookmarked(&self, series_id: SeriesId) -> GatewayFuture<bool> {
        let viewing = self.contexts.viewing.clone();
        self.dispatch(async move {
            qry_watchlist_and_settings::is_bookmarked(&viewing, series_id).await
        })
    }

    fn bookmark_series(&self, series_id: SeriesId) -> GatewayFuture<()> {
        let viewing = self.contexts.viewing.clone();
        self.dispatch(async move {
            cmd_watchlist_and_settings::bookmark_series(&viewing, series_id).await
        })
    }

    fn unbookmark_series(&self, series_id: SeriesId) -> GatewayFuture<()> {
        let viewing = self.contexts.viewing.clone();
        self.dispatch(async move {
            cmd_watchlist_and_settings::unbookmark_series(&viewing, series_id).await
        })
    }

    fn playback_preference(&self) -> GatewayFuture<PlaybackPreference> {
        let viewing = self.contexts.viewing.clone();
        self.dispatch(
            async move { qry_watchlist_and_settings::playback_preference(&viewing).await },
        )
    }

    fn update_playback_preference(&self, preference: PlaybackPreference) -> GatewayFuture<()> {
        let viewing = self.contexts.viewing.clone();
        self.dispatch(async move {
            cmd_watchlist_and_settings::update_playback_preference(&viewing, preference).await
        })
    }

    fn clear_playback_cache(&self) -> GatewayFuture<()> {
        let viewing = self.contexts.viewing.clone();
        self.dispatch(
            async move { cmd_watchlist_and_settings::clear_playback_cache(&viewing).await },
        )
    }

    fn storage_overview(&self) -> GatewayFuture<StorageOverview> {
        let viewing = self.contexts.viewing.clone();
        self.dispatch(async move { qry_watchlist_and_settings::storage_overview(&viewing).await })
    }

    fn offline_library(&self) -> GatewayFuture<Vec<OfflineSeriesView>> {
        let offline = self.contexts.offline.clone();
        self.dispatch(async move { qry_offline_library::offline_library(&offline).await })
    }

    fn download_tasks(&self) -> GatewayFuture<Vec<DownloadTaskView>> {
        let offline = self.contexts.offline.clone();
        self.dispatch(async move { qry_download_queue::download_tasks(&offline).await })
    }

    fn enqueue_episode_downloads(
        &self,
        request: EnqueueEpisodeDownloadsRequest,
    ) -> GatewayFuture<EnqueueEpisodeDownloadsResult> {
        let offline = self.contexts.offline.clone();
        self.dispatch(async move {
            cmd_download_queue::enqueue_episode_downloads(&offline, request).await
        })
    }

    fn control_download(&self, request: ControlDownloadRequest) -> GatewayFuture<()> {
        let offline = self.contexts.offline.clone();
        self.dispatch(async move { cmd_download_queue::control_download(&offline, request).await })
    }

    fn control_all_downloads(&self, action: ControlAllDownloadsAction) -> GatewayFuture<()> {
        let offline = self.contexts.offline.clone();
        self.dispatch(
            async move { cmd_download_queue::control_all_downloads(&offline, action).await },
        )
    }

    fn delete_offline_episode(&self, key: EpisodeKey) -> GatewayFuture<()> {
        let offline = self.contexts.offline.clone();
        self.dispatch(
            async move { cmd_offline_library::delete_offline_episode(&offline, key).await },
        )
    }

    fn delete_offline_series(&self, series_id: SeriesId) -> GatewayFuture<()> {
        let offline = self.contexts.offline.clone();
        self.dispatch(async move {
            cmd_offline_library::delete_offline_series(&offline, series_id).await
        })
    }

    fn download_policy(&self) -> GatewayFuture<DownloadPolicy> {
        let offline = self.contexts.offline.clone();
        self.dispatch(async move { qry_download_queue::download_policy(&offline).await })
    }

    fn update_download_policy(&self, policy: DownloadPolicy) -> GatewayFuture<()> {
        let offline = self.contexts.offline.clone();
        self.dispatch(
            async move { cmd_download_queue::update_download_policy(&offline, policy).await },
        )
    }
}
