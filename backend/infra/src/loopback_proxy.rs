use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use bytes::Bytes;
use hg_backend_contracts::error::{PortError, PortFuture};
use hg_backend_contracts::viewing::PlaybackHaltPort;
use hg_contracts::ids::{EpisodeKey, SeriesId};
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::header::CONTENT_TYPE;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use parking_lot::Mutex;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use crate::error::InfraError;
use crate::storage::StorageLayout;

mod cover;
mod online;

use cover::{build_cover_proxy_url, proxy_cover_request};
use online::{OnlineStreamSession, build_online_session, proxy_online_request};

#[derive(Debug, Clone)]
enum SessionTarget {
    Offline { episode_dir: PathBuf },
    Online(OnlineStreamSession),
}

#[derive(Debug, Clone)]
struct RegisteredSession {
    key: EpisodeKey,
    target: SessionTarget,
}

#[derive(Debug, Default)]
struct SessionRegistry {
    sessions: HashMap<String, RegisteredSession>,
}

#[derive(Debug)]
struct LoopbackInner {
    addr: SocketAddr,
    storage: Arc<StorageLayout>,
    http_client: reqwest::Client,
    registry: Mutex<SessionRegistry>,
    next_token: AtomicU64,
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}

#[derive(Debug, Clone)]
pub struct LoopbackProxyHandle {
    inner: Arc<LoopbackInner>,
}

impl LoopbackProxyHandle {
    pub async fn start(storage: Arc<StorageLayout>) -> Result<Self, InfraError> {
        let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).await?;
        let addr = listener.local_addr()?;
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let http_client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|err| InfraError::Crypto {
                detail: format!("初始化回环代理客户端失败: {err}"),
            })?;

        let inner = Arc::new(LoopbackInner {
            addr,
            storage,
            http_client,
            registry: Mutex::new(SessionRegistry::default()),
            next_token: AtomicU64::new(1),
            shutdown_tx: Mutex::new(Some(shutdown_tx)),
        });

        let task_inner = Arc::clone(&inner);
        tokio::spawn(async move {
            run_accept_loop(listener, task_inner, shutdown_rx).await;
        });

        Ok(Self { inner })
    }

    #[must_use]
    pub fn base_url(&self) -> String {
        format!("http://{}", self.inner.addr)
    }

    #[must_use]
    pub fn cover_proxy_url(&self, raw_cover_url: &str) -> String {
        build_cover_proxy_url(&self.base_url(), raw_cover_url)
    }

    pub fn register_offline_episode(
        &self,
        key: EpisodeKey,
        playlist_rel_path: &str,
    ) -> Result<String, InfraError> {
        let playlist_full = self
            .inner
            .storage
            .resolve_offline_library_path(playlist_rel_path)?;
        let Some(episode_dir) = playlist_full.parent().map(Path::to_path_buf) else {
            return Err(InfraError::InvalidPath {
                detail: format!("播放列表缺少所属目录: {playlist_rel_path}"),
            });
        };
        let Some(file_name) = playlist_full.file_name().and_then(|name| name.to_str()) else {
            return Err(InfraError::InvalidPath {
                detail: format!("播放列表文件名非法: {playlist_rel_path}"),
            });
        };

        let token = format!(
            "ep-{}-{}-{}",
            key.series_id,
            key.ep_num,
            self.inner.next_token.fetch_add(1, Ordering::SeqCst)
        );
        self.inner.registry.lock().sessions.insert(
            token.clone(),
            RegisteredSession {
                key,
                target: SessionTarget::Offline { episode_dir },
            },
        );

        Ok(format!("{}/offline/{token}/{file_name}", self.base_url()))
    }

    pub fn register_online_stream(
        &self,
        key: EpisodeKey,
        stream_url: &str,
        headers: &[(String, String)],
    ) -> Result<String, InfraError> {
        let session = build_online_session(stream_url, headers)?;
        let token = format!(
            "online-{}-{}-{}",
            key.series_id,
            key.ep_num,
            self.inner.next_token.fetch_add(1, Ordering::SeqCst)
        );
        self.inner.registry.lock().sessions.insert(
            token.clone(),
            RegisteredSession {
                key,
                target: SessionTarget::Online(session),
            },
        );

        Ok(format!("{}/online/{token}/manifest.m3u8", self.base_url()))
    }

    #[must_use]
    pub fn active_session_count(&self) -> usize {
        self.inner.registry.lock().sessions.len()
    }

    pub fn shutdown(&self) {
        if let Some(tx) = self.inner.shutdown_tx.lock().take() {
            let _sent = tx.send(());
        }
        self.inner.registry.lock().sessions.clear();
    }
}

impl PlaybackHaltPort for LoopbackProxyHandle {
    fn halt_episode(&self, key: EpisodeKey) -> PortFuture<'_, Result<(), PortError>> {
        self.inner
            .registry
            .lock()
            .sessions
            .retain(|_, session| session.key != key);
        Box::pin(async { Ok(()) })
    }

    fn halt_series(&self, series_id: SeriesId) -> PortFuture<'_, Result<(), PortError>> {
        self.inner
            .registry
            .lock()
            .sessions
            .retain(|_, session| session.key.series_id != series_id);
        Box::pin(async { Ok(()) })
    }
}

async fn run_accept_loop(
    listener: TcpListener,
    inner: Arc<LoopbackInner>,
    mut shutdown_rx: oneshot::Receiver<()>,
) {
    loop {
        tokio::select! {
            _ = &mut shutdown_rx => break,
            accepted = listener.accept() => {
                let Ok((stream, _peer)) = accepted else {
                    break;
                };
                let conn_inner = Arc::clone(&inner);
                tokio::spawn(async move {
                    let io = TokioIo::new(stream);
                    let service = service_fn(move |req| {
                        let req_inner = Arc::clone(&conn_inner);
                        async move { Ok::<_, std::convert::Infallible>(handle_request(req, &req_inner).await) }
                    });
                    drop(http1::Builder::new().serve_connection(io, service).await);
                });
            }
        }
    }
}

async fn handle_request(req: Request<Incoming>, inner: &LoopbackInner) -> Response<Full<Bytes>> {
    if req.method() != Method::GET && req.method() != Method::HEAD {
        return empty_response(StatusCode::METHOD_NOT_ALLOWED);
    }

    if let Some(encoded) = req.uri().path().strip_prefix("/cover/")
        && !encoded.is_empty()
        && !encoded.contains('/')
    {
        return proxy_cover_request(&inner.http_client, &inner.storage, req.method(), encoded)
            .await;
    }

    if let Some((token, rel_file)) = parse_scoped_path(req.uri().path(), "/offline/") {
        return handle_offline_request(req.method(), token, rel_file, inner).await;
    }

    if let Some((token, subpath)) = parse_scoped_path(req.uri().path(), "/online/") {
        let session = {
            let guard = inner.registry.lock();
            match guard.sessions.get(token).map(|s| &s.target) {
                Some(SessionTarget::Online(online_session)) => Some(online_session.clone()),
                Some(SessionTarget::Offline { .. }) | None => None,
            }
        };
        let Some(online_session) = session else {
            return empty_response(StatusCode::GONE);
        };
        return proxy_online_request(
            &inner.http_client,
            req.method(),
            token,
            subpath,
            req.uri().query(),
            &online_session,
        )
        .await;
    }

    empty_response(StatusCode::BAD_REQUEST)
}

async fn handle_offline_request(
    method: &Method,
    token: &str,
    rel_file: &str,
    inner: &LoopbackInner,
) -> Response<Full<Bytes>> {
    let episode_dir = {
        let guard = inner.registry.lock();
        match guard.sessions.get(token).map(|s| &s.target) {
            Some(SessionTarget::Offline { episode_dir }) => Some(episode_dir.clone()),
            Some(SessionTarget::Online(_)) | None => None,
        }
    };

    let Some(base_dir) = episode_dir else {
        return empty_response(StatusCode::GONE);
    };

    let Some(clean_rel) = sanitize_asset_subpath(rel_file) else {
        return empty_response(StatusCode::BAD_REQUEST);
    };

    let target_file = base_dir.join(clean_rel);
    let bytes = match tokio::fs::read(&target_file).await {
        Ok(data) => data,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return empty_response(StatusCode::NOT_FOUND);
        }
        Err(_) => return empty_response(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let content_type = content_type_for_path(&target_file);
    let body = if *method == Method::HEAD {
        Bytes::new()
    } else {
        Bytes::from(bytes)
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, content_type)
        .body(Full::new(body))
        .unwrap_or_else(|_| empty_response(StatusCode::INTERNAL_SERVER_ERROR))
}

fn parse_scoped_path<'a>(path: &'a str, prefix: &str) -> Option<(&'a str, &'a str)> {
    let trimmed = path.strip_prefix(prefix)?;
    let (token, rel) = trimmed.split_once('/')?;
    if token.is_empty() || rel.is_empty() {
        return None;
    }
    Some((token, rel))
}

fn sanitize_asset_subpath(raw: &str) -> Option<PathBuf> {
    if raw.contains('\\') {
        return None;
    }
    let mut clean = PathBuf::new();
    for component in Path::new(raw).components() {
        match component {
            Component::Normal(part) => clean.push(part),
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => return None,
        }
    }
    if clean.as_os_str().is_empty() {
        return None;
    }
    Some(clean)
}

fn content_type_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("m3u8") => "application/vnd.apple.mpegurl",
        Some("ts") => "video/mp2t",
        Some("mp4" | "m4s") => "video/mp4",
        Some("key") => "application/octet-stream",
        Some(_) | None => "application/octet-stream",
    }
}

fn empty_response(status: StatusCode) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .body(Full::new(Bytes::new()))
        .unwrap_or_else(|_| Response::new(Full::new(Bytes::new())))
}
