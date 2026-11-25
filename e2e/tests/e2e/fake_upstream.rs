use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use aes_gcm::aead::{Aead as _, Payload};
use aes_gcm::{Aes256Gcm, KeyInit as _, Nonce};
use base64::Engine as _;
use base64::engine::general_purpose::{
    STANDARD as BASE64_STANDARD, URL_SAFE_NO_PAD as BASE64_URL_SAFE_NO_PAD,
};
use bytes::Bytes;
use hg_runtime::infra::clock::{Clock, SystemClock};
use hkdf::Hkdf;
use http_body_util::{BodyExt as _, Full};
use hyper::body::Incoming;
use hyper::header::{CONNECTION, CONTENT_TYPE};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use p256::PublicKey;
use p256::ecdh::EphemeralSecret;
use p256::elliptic_curve::Generate as _;
use p256::elliptic_curve::sec1::ToSec1Point as _;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::Sha256;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

const EDGE_PROTOCOL: &str = "dx150-edge-v1";
const SEGMENT_BYTES_LEN: usize = 163_840;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedUpstreamCall {
    pub scope: String,
    pub method: String,
    pub path: String,
}

#[derive(Debug, Clone)]
struct UpstreamSession {
    kid: String,
    scope: String,
    info: String,
    req_key: [u8; 32],
    res_key: [u8; 32],
    last_seq: u32,
}

struct UpstreamState {
    base_url: String,
    clock: SystemClock,
    offline: AtomicBool,
    next_id: AtomicU64,
    sessions: Mutex<HashMap<String, UpstreamSession>>,
    calls: Mutex<Vec<RecordedUpstreamCall>>,
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}

#[derive(Clone)]
pub struct FakeUpstreamServer {
    state: Arc<UpstreamState>,
}

#[derive(Debug, Deserialize)]
struct HandshakeRequest {
    v: u8,
    #[serde(rename = "type")]
    kind: String,
    client_pub: String,
}

#[derive(Debug, Serialize)]
struct HandshakeResponse {
    v: u8,
    #[serde(rename = "type")]
    kind: &'static str,
    kid: String,
    sid: String,
    site: &'static str,
    server_pub: String,
    salt: String,
    server_time: i64,
    expires_at: i64,
}

#[derive(Debug, Deserialize)]
struct EncryptedEnvelope {
    v: u8,
    #[serde(rename = "type")]
    kind: String,
    kid: String,
    sid: String,
    seq: u32,
    ts: u64,
    ciphertext: String,
}

#[derive(Debug, Serialize)]
struct EncryptedResponseEnvelope {
    v: u8,
    #[serde(rename = "type")]
    kind: &'static str,
    kid: String,
    sid: String,
    seq: u32,
    ts: u64,
    ciphertext: String,
}

#[derive(Debug, Deserialize)]
struct InnerRequestWrapper {
    request: InnerRequest,
}

#[derive(Debug, Deserialize)]
struct InnerRequest {
    method: String,
    path: String,
    #[serde(default)]
    body: String,
}

#[derive(Debug, Serialize)]
struct InnerResponse {
    status: u16,
    body: String,
}

impl FakeUpstreamServer {
    pub async fn start() -> Result<Self, String> {
        let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .await
            .map_err(|err| format!("绑定本地模拟上游端口失败: {err}"))?;
        let addr = listener
            .local_addr()
            .map_err(|err| format!("获取本地模拟上游地址失败: {err}"))?;
        let base_url = format!("http://{addr}");
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let state = Arc::new(UpstreamState {
            base_url,
            clock: SystemClock,
            offline: AtomicBool::new(false),
            next_id: AtomicU64::new(1),
            sessions: Mutex::new(HashMap::new()),
            calls: Mutex::new(Vec::new()),
            shutdown_tx: Mutex::new(Some(shutdown_tx)),
        });

        let accept_state = Arc::clone(&state);
        tokio::spawn(async move {
            run_accept_loop(listener, accept_state, shutdown_rx).await;
        });

        Ok(Self { state })
    }

    #[must_use]
    pub fn base_url(&self) -> String {
        self.state.base_url.clone()
    }

    pub fn set_offline(&self, offline: bool) {
        self.state.offline.store(offline, Ordering::SeqCst);
    }

    #[must_use]
    pub fn call_count(&self) -> usize {
        self.state.calls.lock().len()
    }

    #[must_use]
    pub fn calls(&self) -> Vec<RecordedUpstreamCall> {
        self.state.calls.lock().clone()
    }

    pub fn shutdown(&self) {
        if let Some(tx) = self.state.shutdown_tx.lock().take() {
            let _sent = tx.send(());
        }
        self.state.sessions.lock().clear();
    }
}

impl Drop for FakeUpstreamServer {
    fn drop(&mut self) {
        if Arc::strong_count(&self.state) <= 2 {
            self.shutdown();
        }
    }
}

async fn run_accept_loop(
    listener: TcpListener,
    state: Arc<UpstreamState>,
    mut shutdown_rx: oneshot::Receiver<()>,
) {
    loop {
        tokio::select! {
            _ = &mut shutdown_rx => break,
            accepted = listener.accept() => {
                let Ok((stream, _peer)) = accepted else {
                    break;
                };
                if state.offline.load(Ordering::SeqCst) {
                    drop(stream);
                    continue;
                }
                let conn_state = Arc::clone(&state);
                tokio::spawn(async move {
                    let io = TokioIo::new(stream);
                    let service = service_fn(move |req| {
                        let req_state = Arc::clone(&conn_state);
                        async move { Ok::<_, std::convert::Infallible>(handle_http(req, &req_state).await) }
                    });
                    drop(http1::Builder::new().serve_connection(io, service).await);
                });
            }
        }
    }
}

async fn handle_http(req: Request<Incoming>, state: &UpstreamState) -> Response<Full<Bytes>> {
    if state.offline.load(Ordering::SeqCst) {
        return Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .header(CONNECTION, "close")
            .body(Full::new(Bytes::from_static(b"upstream offline")))
            .unwrap_or_else(|_| empty_status(StatusCode::SERVICE_UNAVAILABLE));
    }

    let method = req.method().clone();
    let path = req.uri().path().to_owned();
    let is_public_json = req
        .headers()
        .get("X-HG-Public-JSON")
        .and_then(|v| v.to_str().ok())
        == Some("1");

    if method == Method::POST
        && let Some(scope) = parse_encrypted_scope(&path)
    {
        return handle_scope_post(req, state, scope, is_public_json).await;
    }

    if method == Method::GET || method == Method::HEAD {
        return handle_media_or_cover(&method, &path, state);
    }

    empty_status(StatusCode::NOT_FOUND)
}

async fn handle_scope_post(
    req: Request<Incoming>,
    state: &UpstreamState,
    scope: &str,
    is_public_json: bool,
) -> Response<Full<Bytes>> {
    let bytes = match read_body_bytes(req).await {
        Ok(b) => b,
        Err((status, msg)) => return json_response(status, &json!({ "error": msg })),
    };

    if scope == "public" && is_public_json {
        return handle_public_json_request(&bytes, state);
    }

    let kind = serde_json::from_slice::<Value>(&bytes)
        .ok()
        .and_then(|v| v.get("type").and_then(Value::as_str).map(str::to_owned));
    match kind.as_deref() {
        Some("hello") => handle_handshake(&bytes, state, scope),
        Some("call") => handle_encrypted_channel(&bytes, state, scope),
        _ => json_response(
            StatusCode::BAD_REQUEST,
            &json!({ "error": "invalid_request" }),
        ),
    }
}

fn handle_public_json_request(bytes: &[u8], state: &UpstreamState) -> Response<Full<Bytes>> {
    let Ok(inner_req) = serde_json::from_slice::<InnerRequest>(bytes) else {
        return json_response(
            StatusCode::BAD_REQUEST,
            &json!({ "error": "invalid_request" }),
        );
    };
    state.calls.lock().push(RecordedUpstreamCall {
        scope: "public".to_owned(),
        method: inner_req.method.clone(),
        path: inner_req.path.clone(),
    });
    let (status_code, body) =
        dispatch_api_route(&inner_req.method, &inner_req.path, &state.base_url);
    let status = StatusCode::from_u16(status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let encoded = serde_json::to_vec(&body).unwrap_or_default();
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json")
        .header("X-HG-Public-JSON", "1")
        .body(Full::new(Bytes::from(encoded)))
        .unwrap_or_else(|_| empty_status(StatusCode::INTERNAL_SERVER_ERROR))
}

fn parse_encrypted_scope(path: &str) -> Option<&'static str> {
    match path {
        "/api/public/request" => Some("public"),
        "/api/dynamic/request" => Some("dynamic"),
        "/api/playback/request" => Some("playback"),
        _ => None,
    }
}

async fn read_body_bytes(req: Request<Incoming>) -> Result<Bytes, (StatusCode, &'static str)> {
    req.into_body()
        .collect()
        .await
        .map(http_body_util::Collected::to_bytes)
        .map_err(|_| (StatusCode::BAD_REQUEST, "bad_body"))
}

fn handle_handshake(bytes: &[u8], state: &UpstreamState, scope: &str) -> Response<Full<Bytes>> {
    let Ok(hs_req) = serde_json::from_slice::<HandshakeRequest>(bytes) else {
        return json_response(
            StatusCode::BAD_REQUEST,
            &json!({ "error": "invalid_handshake" }),
        );
    };
    if hs_req.v != 1
        || hs_req.kind != "hello"
        || !matches!(scope, "public" | "dynamic" | "playback")
    {
        return json_response(
            StatusCode::BAD_REQUEST,
            &json!({ "error": "unsupported_protocol" }),
        );
    }
    let Ok(client_pub_bytes) = BASE64_URL_SAFE_NO_PAD.decode(&hs_req.client_pub) else {
        return json_response(
            StatusCode::BAD_REQUEST,
            &json!({ "error": "invalid_client_key_b64" }),
        );
    };
    let Ok(client_pub) = PublicKey::from_sec1_bytes(&client_pub_bytes) else {
        return json_response(
            StatusCode::BAD_REQUEST,
            &json!({ "error": "invalid_client_key" }),
        );
    };

    let Ok(response) = create_session_and_response(state, scope, &client_pub) else {
        return json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &json!({ "error": "key_derivation_failed" }),
        );
    };
    json_response(StatusCode::OK, &response)
}

fn create_session_and_response(
    state: &UpstreamState,
    scope: &str,
    client_pub: &PublicKey,
) -> Result<HandshakeResponse, String> {
    let seq_id = state.next_id.fetch_add(1, Ordering::SeqCst);
    let kid = format!("kid-{seq_id}");
    let sid = format!("sid-{seq_id}");
    let site = "hg-e2e";
    let salt = format!("salt-{seq_id:027}").into_bytes();

    let server_secret = EphemeralSecret::generate();
    let server_pub = server_secret.public_key();
    let server_pub_bytes = server_pub.to_sec1_point(false);
    let shared = server_secret.diffie_hellman(client_pub);

    let (req_key, res_key, info) = derive_keys(
        shared.raw_secret_bytes().as_slice(),
        &salt,
        scope,
        site,
        &kid,
        &sid,
    )?;

    state.sessions.lock().insert(
        sid.clone(),
        UpstreamSession {
            kid: kid.clone(),
            scope: scope.to_owned(),
            info,
            req_key,
            res_key,
            last_seq: 0,
        },
    );

    let server_time = i64::try_from(state.clock.now_epoch_millis() / 1_000).unwrap_or(0);
    let expires_at = server_time.saturating_add(300);
    Ok(HandshakeResponse {
        v: 1,
        kind: "hello",
        kid,
        sid,
        site,
        server_pub: BASE64_URL_SAFE_NO_PAD.encode(server_pub_bytes.as_bytes()),
        salt: BASE64_URL_SAFE_NO_PAD.encode(&salt),
        server_time,
        expires_at,
    })
}

fn handle_encrypted_channel(
    bytes: &[u8],
    state: &UpstreamState,
    scope: &str,
) -> Response<Full<Bytes>> {
    let Ok(env) = serde_json::from_slice::<EncryptedEnvelope>(bytes) else {
        return json_response(
            StatusCode::BAD_REQUEST,
            &json!({ "error": "invalid_envelope" }),
        );
    };
    if env.v != 1 || env.kind != "call" {
        return json_response(
            StatusCode::BAD_REQUEST,
            &json!({ "error": "invalid_protocol" }),
        );
    }

    let session = match validate_and_step_session(state, &env, scope) {
        Ok(sess) => sess,
        Err((status, msg)) => return json_response(status, &json!({ "error": msg })),
    };

    let Ok(inner_req) = decrypt_inner_request(&session, &env) else {
        return json_response(
            StatusCode::BAD_REQUEST,
            &json!({ "error": "decrypt_failed" }),
        );
    };

    state.calls.lock().push(RecordedUpstreamCall {
        scope: scope.to_owned(),
        method: inner_req.method.clone(),
        path: inner_req.path.clone(),
    });

    let _raw_req_body = BASE64_STANDARD.decode(&inner_req.body).unwrap_or_default();
    let (inner_status, inner_body) =
        dispatch_api_route(&inner_req.method, &inner_req.path, &state.base_url);

    match encrypt_inner_response(&session, &env, inner_status, &inner_body) {
        Ok(res_env) => json_response(StatusCode::OK, &res_env),
        Err(_) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &json!({ "error": "encrypt_failed" }),
        ),
    }
}

fn validate_and_step_session(
    state: &UpstreamState,
    env: &EncryptedEnvelope,
    scope: &str,
) -> Result<UpstreamSession, (StatusCode, &'static str)> {
    let mut guard = state.sessions.lock();
    let Some(sess) = guard.get_mut(&env.sid) else {
        return Err((StatusCode::UNAUTHORIZED, "session_expired"));
    };
    if sess.kid != env.kid || sess.scope != scope || env.seq <= sess.last_seq {
        return Err((StatusCode::UNAUTHORIZED, "session_expired"));
    }
    sess.last_seq = env.seq;
    Ok(sess.clone())
}

fn decrypt_inner_request(
    session: &UpstreamSession,
    env: &EncryptedEnvelope,
) -> Result<InnerRequest, String> {
    let cipher_bytes = BASE64_URL_SAFE_NO_PAD
        .decode(&env.ciphertext)
        .map_err(|err| err.to_string())?;
    let aad = format!("{}\nreq\n{}\n{}", session.info, env.seq, env.ts).into_bytes();
    let plain = aes_gcm_decrypt(&session.req_key, env.seq, &aad, &cipher_bytes)?;
    let wrapper: InnerRequestWrapper =
        serde_json::from_slice(&plain).map_err(|err| err.to_string())?;
    Ok(wrapper.request)
}

fn encrypt_inner_response(
    session: &UpstreamSession,
    env: &EncryptedEnvelope,
    status: u16,
    body: &Value,
) -> Result<EncryptedResponseEnvelope, String> {
    let body_bytes = serde_json::to_vec(body).map_err(|err| err.to_string())?;
    let inner = InnerResponse {
        status,
        body: BASE64_STANDARD.encode(&body_bytes),
    };
    let plain = serde_json::to_vec(&inner).map_err(|err| err.to_string())?;
    let aad = format!("{}\nres\n{}\n{}", session.info, env.seq, env.ts).into_bytes();
    let cipher = aes_gcm_encrypt(&session.res_key, env.seq, &aad, &plain)?;
    Ok(EncryptedResponseEnvelope {
        v: 1,
        kind: "result",
        kid: env.kid.clone(),
        sid: env.sid.clone(),
        seq: env.seq,
        ts: env.ts,
        ciphertext: BASE64_URL_SAFE_NO_PAD.encode(&cipher),
    })
}

fn derive_keys(
    shared_secret: &[u8],
    salt: &[u8],
    scope: &str,
    site: &str,
    kid: &str,
    sid: &str,
) -> Result<([u8; 32], [u8; 32], String), String> {
    let base_info = format!("{EDGE_PROTOCOL}\n{scope}\n{site}\n{kid}\n{sid}");
    let hkdf = Hkdf::<Sha256>::new(Some(salt), shared_secret);

    let mut req_key = [0_u8; 32];
    hkdf.expand(format!("{base_info}\nreq").as_bytes(), &mut req_key)
        .map_err(|_| "派生请求密钥失败".to_owned())?;

    let mut res_key = [0_u8; 32];
    hkdf.expand(format!("{base_info}\nres").as_bytes(), &mut res_key)
        .map_err(|_| "派生响应密钥失败".to_owned())?;

    Ok((req_key, res_key, base_info))
}

fn build_nonce(sequence: u32) -> [u8; 12] {
    let mut nonce = [0_u8; 12];
    let seq_bytes = sequence.to_be_bytes();
    if let Some(tail) = nonce.get_mut(8..12) {
        tail.copy_from_slice(&seq_bytes);
    }
    nonce
}

fn aes_gcm_encrypt(
    key: &[u8; 32],
    sequence: u32,
    aad: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new(key.into());
    let nonce = Nonce::from(build_nonce(sequence));
    cipher
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| "AES-256-GCM 加密失败".to_owned())
}

fn aes_gcm_decrypt(
    key: &[u8; 32],
    sequence: u32,
    aad: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new(key.into());
    let nonce = Nonce::from(build_nonce(sequence));
    cipher
        .decrypt(
            &nonce,
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| "AES-256-GCM 解密失败".to_owned())
}

fn handle_media_or_cover(
    method: &Method,
    path: &str,
    state: &UpstreamState,
) -> Response<Full<Bytes>> {
    if path.starts_with("/covers/") && path.ends_with(".png") {
        let mut png = Vec::from(b"\x89PNG\r\n\x1a\nfake-cover-data".as_slice());
        png.resize(256, b'C');
        let body = if *method == Method::HEAD {
            Bytes::new()
        } else {
            Bytes::from(png)
        };
        return bytes_response(StatusCode::OK, "image/png", body);
    }

    if let Some(rest) = path.strip_prefix("/hls/") {
        if rest.ends_with(".m3u8") {
            let playlist = "#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-TARGETDURATION:6\n#EXTINF:5.0,\nseg0.ts\n#EXT-X-ENDLIST\n";
            let body = if *method == Method::HEAD {
                Bytes::new()
            } else {
                Bytes::from_static(playlist.as_bytes())
            };
            return bytes_response(StatusCode::OK, "application/vnd.apple.mpegurl", body);
        }
        if rest.ends_with("/seg0.ts") {
            let mut ts_data = vec![0x47_u8; SEGMENT_BYTES_LEN];
            if let Some(tag_slice) = ts_data.get_mut(1..1 + state.base_url.len().min(32)) {
                let bytes = state.base_url.as_bytes();
                let count = tag_slice.len().min(bytes.len());
                if let (Some(dst), Some(src)) = (tag_slice.get_mut(..count), bytes.get(..count)) {
                    dst.copy_from_slice(src);
                }
            }
            let body = if *method == Method::HEAD {
                Bytes::new()
            } else {
                Bytes::from(ts_data)
            };
            return bytes_response(StatusCode::OK, "video/mp2t", body);
        }
    }

    empty_status(StatusCode::NOT_FOUND)
}

fn dispatch_api_route(method: &str, path_with_query: &str, base_url: &str) -> (u16, Value) {
    let (path, query) = path_with_query
        .split_once('?')
        .unwrap_or((path_with_query, ""));

    match (method, path) {
        ("GET", "/api/home") => (200, home_payload(base_url)),
        ("GET", "/api/home/latest") => (
            200,
            paged_videos_payload(base_url, "sort=new&page=1&size=20", None),
        ),
        ("GET", "/api/site/categories/video") => (200, categories_payload()),
        ("GET", "/api/tags") => (200, tags_payload()),
        ("GET", "/api/search/hot") => (200, hot_keywords_payload()),
        ("GET", "/api/search/tags/hot") => (200, hot_tags_payload()),
        ("GET", "/api/videos") => (200, paged_videos_payload(base_url, query, None)),
        ("GET", "/api/search") => {
            let keyword = extract_query_param(query, "q")
                .map(|raw| percent_decode(&raw))
                .unwrap_or_default();
            (200, paged_videos_payload(base_url, query, Some(&keyword)))
        }
        ("GET", "/api/videos/101") => (200, series_101_detail_payload(base_url)),
        ("GET", "/api/videos/102") => (200, series_102_detail_payload(base_url)),
        ("GET", "/api/videos/101/comments") => (200, comments_101_payload(query)),
        ("GET", "/api/videos/101/comments/5001/replies") => (200, replies_5001_payload()),
        ("GET", p) if p.starts_with("/api/ranks/") => (200, ranking_payload(base_url)),
        ("GET", p)
            if p.starts_with("/api/videos/category/") || p.starts_with("/api/videos/tag/") =>
        {
            (200, paged_videos_payload(base_url, query, None))
        }
        ("POST", p) if p.starts_with("/api/videos/") && p.ends_with("/playback") => {
            dispatch_playback_route(p, base_url)
        }
        _ => (404, json!({ "status": 0, "msg": "资源不存在" })),
    }
}

fn dispatch_playback_route(path: &str, base_url: &str) -> (u16, Value) {
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    let Some(series_str) = parts.get(2).copied() else {
        return (404, json!({ "status": 0, "msg": "剧目不存在" }));
    };
    let Some(ep_str) = parts.get(4).copied() else {
        return (404, json!({ "status": 0, "msg": "分集不存在" }));
    };
    let Ok(series_id) = series_str.parse::<u64>() else {
        return (404, json!({ "status": 0, "msg": "剧目不存在" }));
    };
    let Ok(ep_num) = ep_str.parse::<u32>() else {
        return (404, json!({ "status": 0, "msg": "分集不存在" }));
    };
    if series_id != 101 && series_id != 102 {
        return (404, json!({ "status": 0, "msg": "剧目不存在" }));
    }

    let payload = json!({
        "status": 1,
        "msg": "ok",
        "data": {
            "id": series_str,
            "ep": ep_num,
            "timestamp": 1_700_000_000_u64,
            "cdn": {
                "authKey": "x-cdn-auth",
                "authValue": format!("tok-{series_id}-{ep_num}"),
                "app": "150"
            },
            "streams": {
                "hevc": format!("{base_url}/hls/{series_id}/e{ep_num}/index.m3u8?auth_key=1700000000-r-0-a"),
                "main": format!("{base_url}/hls/{series_id}/e{ep_num}/main.m3u8?auth_key=1700000000-r-0-b")
            }
        }
    });
    (200, payload)
}

fn series_101_summary(base_url: &str) -> Value {
    json!({
        "id": 101,
        "title": "夜雨长灯",
        "cover": format!("{base_url}/covers/101.png"),
        "hot": 99000,
        "score": 9.3,
        "tags": ["悬疑", "古装"],
        "desc": "全流程端到端测试剧目，第一集揭开当年真相与长灯之谜，适合完整验证发现搜索、播放、评论回复、快进上报与追剧持久化。",
        "episode_count": 3,
        "is_finished": true
    })
}

fn series_102_summary(base_url: &str) -> Value {
    json!({
        "id": 102,
        "title": "星辰暗涌",
        "cover": format!("{base_url}/covers/102.png"),
        "hot": 88000,
        "score": 8.9,
        "tags": ["都市"],
        "desc": "第二部用于验证搜索过滤的对比剧目。",
        "episode_count": 2,
        "is_finished": false
    })
}

fn home_payload(base_url: &str) -> Value {
    let s1 = series_101_summary(base_url);
    let s2 = series_102_summary(base_url);
    json!({
        "status": 1,
        "msg": "ok",
        "data": {
            "banners": [],
            "featured": [s1.clone(), s2.clone()],
            "cate_rec": [{
                "cate_id": 1,
                "cate_name": "热门短剧",
                "videos": [s1, s2]
            }]
        }
    })
}

fn categories_payload() -> Value {
    json!({
        "status": 1,
        "msg": "ok",
        "data": [{ "id": 1, "name": "热门短剧", "route": "duanju" }]
    })
}

fn tags_payload() -> Value {
    json!({
        "status": 1,
        "msg": "ok",
        "data": {
            "categories": [{
                "id": 1,
                "code": "theme",
                "name": "题材",
                "tags": [{ "id": 10, "name": "悬疑", "slug": "xuan-yi", "is_hot": true }]
            }]
        }
    })
}

fn hot_keywords_payload() -> Value {
    json!({
        "status": 1,
        "msg": "ok",
        "data": {
            "items": [{ "word": "夜雨", "heat": 1000 }]
        }
    })
}

fn hot_tags_payload() -> Value {
    json!({
        "status": 1,
        "msg": "ok",
        "data": {
            "items": [{ "name": "悬疑", "heat": 900 }]
        }
    })
}

fn paged_videos_payload(base_url: &str, query: &str, keyword: Option<&str>) -> Value {
    let page = extract_query_param(query, "page")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1)
        .max(1);
    let size = extract_query_param(query, "size")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(24)
        .max(1);
    let sort = extract_query_param(query, "sort").unwrap_or_else(|| "hot".to_owned());

    let mut all = if sort == "new" {
        vec![series_102_summary(base_url), series_101_summary(base_url)]
    } else {
        vec![series_101_summary(base_url), series_102_summary(base_url)]
    };

    if keyword.is_none() {
        for idx in 3..=26_u64 {
            all.push(json!({
                "id": 100 + idx,
                "title": if idx <= 24 {
                    format!("第1页剧目-{idx}")
                } else {
                    format!("第2页剧目-{idx}")
                },
                "cover": format!("{base_url}/covers/101.png"),
                "hot": 50_000_u64.saturating_sub(idx * 100),
                "score": 8.5,
                "tags": ["悬疑"],
                "desc": "用于验证滚动分页的扩展剧目。",
                "episode_count": 10,
                "is_finished": true
            }));
        }
    }

    let filtered: Vec<Value> = match keyword {
        Some(q) if !q.trim().is_empty() => all
            .into_iter()
            .filter(|item| {
                let title = item.get("title").and_then(Value::as_str).unwrap_or("");
                title.contains(q.trim())
            })
            .collect(),
        _ => all,
    };

    let total = filtered.len();
    let pages = total.div_ceil(size).max(1);
    let start = page.saturating_sub(1).saturating_mul(size);
    let page_items: Vec<Value> = filtered.into_iter().skip(start).take(size).collect();

    json!({
        "status": 1,
        "msg": "ok",
        "data": {
            "items": page_items,
            "pagination": {
                "page": page,
                "pages": pages,
                "size": size,
                "total": total
            }
        }
    })
}

fn ranking_payload(base_url: &str) -> Value {
    let mut s1 = series_101_summary(base_url);
    if let Some(obj) = s1.as_object_mut() {
        obj.insert("rank".to_owned(), json!(1));
    }
    json!({
        "status": 1,
        "msg": "ok",
        "data": {
            "items": [s1],
            "pagination": {
                "page": 1,
                "pages": 1,
                "size": 6,
                "total": 1
            }
        }
    })
}

fn series_101_detail_payload(base_url: &str) -> Value {
    json!({
        "status": 1,
        "msg": "ok",
        "data": {
            "id": 101,
            "title": "夜雨长灯",
            "description": "全流程端到端测试剧目，第一集揭开当年真相与长灯之谜，适合完整验证发现搜索、播放、评论回复、快进上报与追剧持久化。",
            "cover": format!("{base_url}/covers/101.png"),
            "score": 9.3,
            "is_finished": true,
            "episode_count": 3,
            "total_episodes": 3,
            "tags": ["悬疑", "古装"],
            "counts": { "hot": 99000, "like": 120, "collect": 88, "comment": 1 },
            "episodes": [
                { "ep_num": 1, "title": "第1集", "duration": 90, "status": 1 },
                { "ep_num": 2, "title": "第2集", "duration": 95, "status": 1 },
                { "ep_num": 3, "title": "第3集", "duration": 100, "status": 1 }
            ]
        }
    })
}

fn series_102_detail_payload(base_url: &str) -> Value {
    json!({
        "status": 1,
        "msg": "ok",
        "data": {
            "id": 102,
            "title": "星辰暗涌",
            "description": "第二部用于验证搜索过滤与上游 total_episodes=0 兜底的对比剧目。",
            "cover": format!("{base_url}/covers/102.png"),
            "score": 8.9,
            "is_finished": false,
            "episode_count": 0,
            "total_episodes": 0,
            "tags": ["都市"],
            "counts": { "hot": 88000, "like": 60, "collect": 30, "comment": 0 },
            "episodes": [
                { "ep_num": 1, "title": "第1集", "duration": 95, "status": 1 },
                { "ep_num": 2, "title": "第2集", "duration": 95, "status": 1 }
            ]
        }
    })
}

fn comments_101_payload(query: &str) -> Value {
    let page = extract_query_param(query, "page")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(1)
        .max(1);

    let items: Vec<Value> = if page == 1 {
        let mut list = vec![json!({
            "id": 5001,
            "nickname": "林深见鹿",
            "avatar_color": "#3b82f6",
            "text": "第一集节奏很紧凑",
            "time": "5分钟前",
            "like_count": 12,
            "is_pinned": true,
            "ip_region": "浙江",
            "reply_count": 2,
            "preview_replies": [{
                "id": 6001,
                "nickname": "南风知意",
                "avatar_color": "#10b981",
                "text": "同感，伏笔很多",
                "time": "2分钟前",
                "like_count": 3
            }]
        })];
        for idx in 2..=20_u64 {
            list.push(json!({
                "id": 5000 + idx,
                "nickname": format!("首屏观众{idx}"),
                "avatar_color": "#64748b",
                "text": format!("第1页评论内容 {idx}"),
                "time": "10分钟前",
                "like_count": idx,
                "is_pinned": false,
                "reply_count": 0,
                "preview_replies": []
            }));
        }
        list
    } else if page == 2 {
        vec![
            json!({
                "id": 5021,
                "nickname": "二页评论作者",
                "avatar_color": "#ec4899",
                "text": "第二页滚动加载出的评论",
                "time": "1小时前",
                "like_count": 5,
                "is_pinned": false,
                "reply_count": 0,
                "preview_replies": []
            }),
            json!({
                "id": 5022,
                "nickname": "尾页观众",
                "avatar_color": "#8b5cf6",
                "text": "最后一条评论",
                "time": "2小时前",
                "like_count": 1,
                "is_pinned": false,
                "reply_count": 0,
                "preview_replies": []
            }),
        ]
    } else {
        Vec::new()
    };

    json!({
        "status": 1,
        "msg": "ok",
        "data": {
            "items": items,
            "pagination": {
                "page": page,
                "pages": 2,
                "size": 20,
                "total": 22
            }
        }
    })
}

fn replies_5001_payload() -> Value {
    json!({
        "status": 1,
        "msg": "ok",
        "data": {
            "items": [
                {
                    "id": 6001,
                    "nickname": "南风知意",
                    "avatar_color": "#10b981",
                    "text": "同感，伏笔很多",
                    "time": "2分钟前",
                    "like_count": 3
                },
                {
                    "id": 6002,
                    "nickname": "周游记",
                    "avatar_color": "#f59e0b",
                    "text": "期待下一集反转",
                    "time": "1分钟前",
                    "like_count": 1,
                    "reply_to_nickname": "南风知意"
                }
            ],
            "pagination": {
                "page": 1,
                "pages": 1,
                "size": 20,
                "total": 2
            }
        }
    })
}

fn extract_query_param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        if k == key { Some(v.to_owned()) } else { None }
    })
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut idx = 0;
    while idx < bytes.len() {
        let current = bytes.get(idx).copied().unwrap_or_default();
        if current == b'%'
            && let (Some(&hi), Some(&lo)) = (bytes.get(idx + 1), bytes.get(idx + 2))
            && let (Some(h), Some(l)) = (hex_val(hi), hex_val(lo))
        {
            out.push((h << 4) | l);
            idx += 3;
            continue;
        }
        if current == b'+' {
            out.push(b' ');
        } else {
            out.push(current);
        }
        idx += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

const fn hex_val(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn json_response<T: Serialize>(status: StatusCode, payload: &T) -> Response<Full<Bytes>> {
    let body = serde_json::to_vec(payload).unwrap_or_default();
    bytes_response(status, "application/json", Bytes::from(body))
}

fn bytes_response(status: StatusCode, content_type: &str, body: Bytes) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, content_type)
        .body(Full::new(body))
        .unwrap_or_else(|_| empty_status(StatusCode::INTERNAL_SERVER_ERROR))
}

fn empty_status(status: StatusCode) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .body(Full::new(Bytes::new()))
        .unwrap_or_else(|_| Response::new(Full::new(Bytes::new())))
}
