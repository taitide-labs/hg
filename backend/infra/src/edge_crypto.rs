use std::sync::Arc;
use std::time::Duration;

use aes::Aes128;
use aes_gcm::aead::{Aead as _, Payload};
use aes_gcm::{Aes256Gcm, KeyInit as _, Nonce};
use base64::Engine as _;
use base64::engine::general_purpose::{
    STANDARD as BASE64_STANDARD, URL_SAFE_NO_PAD as BASE64_URL_SAFE_NO_PAD,
};
use cbc::cipher::block_padding::Pkcs7;
use cbc::cipher::{BlockModeDecrypt as _, KeyIvInit as _};
use hkdf::Hkdf;
use p256::PublicKey;
use p256::ecdh::EphemeralSecret;
use p256::elliptic_curve::Generate as _;
use p256::elliptic_curve::sec1::ToSec1Point as _;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;
use tokio::sync::Mutex;
use url::Url;

use crate::clock::Clock;
use crate::error::InfraError;

pub const DEFAULT_IMAGE_ORIGIN: &str = "https://pic.qnlycc.cn";
pub const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/152.0.0.0 Safari/537.36";
pub const EDGE_PROTOCOL: &str = "dx150-edge-v1";
const MAX_SEQUENCE: u32 = 16_384;
const SESSION_REFRESH_MARGIN_SECS: i64 = 5;
const MAX_HANDSHAKE_TTL_SECS: i64 = 600;
const EXPECTED_SALT_LEN: usize = 32;
const COVER_AES_KEY: [u8; 16] = *b"f5d965df75336270";
const COVER_AES_IV: [u8; 16] = *b"97b60394abc2fbe1";
const JPEG_MAGIC: [u8; 3] = [0xFF, 0xD8, 0xFF];
const PNG_MAGIC: [u8; 4] = [0x89, 0x50, 0x4E, 0x47];
const RIFF_MAGIC: [u8; 4] = [0x52, 0x49, 0x46, 0x46];

type Aes128CbcDecryptor = cbc::Decryptor<Aes128>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeChannelScope {
    Public,
    Dynamic,
    Playback,
}

impl EdgeChannelScope {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Dynamic => "dynamic",
            Self::Playback => "playback",
        }
    }

    #[must_use]
    pub const fn endpoint_path(self) -> &'static str {
        match self {
            Self::Public => "/api/public/request",
            Self::Dynamic => "/api/dynamic/request",
            Self::Playback => "/api/playback/request",
        }
    }
}

#[derive(Debug, Clone)]
pub struct EdgeSessionKeys {
    pub kid: String,
    pub sid: String,
    pub site: String,
    pub scope: EdgeChannelScope,
    pub info: String,
    pub clock_offset_secs: i64,
    pub expires_at_secs: i64,
    pub req_key: [u8; 32],
    pub res_key: [u8; 32],
    pub sequence: u32,
}

#[derive(Debug, Clone)]
pub struct EdgeHttpResponse {
    pub status: u16,
    pub body: Value,
}

#[derive(Debug, Serialize)]
struct HandshakeRequest {
    v: u8,
    #[serde(rename = "type")]
    kind: &'static str,
    client_pub: String,
}

#[derive(Debug, Deserialize)]
struct HandshakeResponse {
    v: u8,
    #[serde(rename = "type")]
    kind: String,
    kid: String,
    sid: String,
    site: String,
    server_pub: String,
    salt: String,
    server_time: i64,
    expires_at: i64,
}

#[derive(Debug, Serialize)]
struct EncryptedEnvelope {
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
struct EncryptedResponseEnvelope {
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
struct InnerRequestWrapper<'a> {
    request: InnerRequest<'a>,
}

#[derive(Debug, Serialize)]
struct InnerRequest<'a> {
    method: &'a str,
    path: &'a str,
    body: String,
    content_type: &'a str,
}

#[derive(Debug, Deserialize)]
struct InnerResponse {
    status: u16,
    #[serde(default)]
    body: String,
}

#[derive(Debug, Deserialize)]
struct EdgeErrorPayload {
    error: Option<String>,
}

pub fn decrypt_cover_bytes(raw: &[u8]) -> Result<Vec<u8>, InfraError> {
    if raw.starts_with(&JPEG_MAGIC) || raw.starts_with(&PNG_MAGIC) || raw.starts_with(&RIFF_MAGIC) {
        return Ok(raw.to_vec());
    }

    Aes128CbcDecryptor::new(&COVER_AES_KEY.into(), &COVER_AES_IV.into())
        .decrypt_padded_vec::<Pkcs7>(raw)
        .map_err(|_| InfraError::Crypto {
            detail: "封面 AES-128-CBC 解密或 PKCS7 去填充失败".to_owned(),
        })
}

pub fn derive_session_keys(
    shared_secret: &[u8],
    salt: &[u8],
    scope: EdgeChannelScope,
    site: &str,
    kid: &str,
    sid: &str,
) -> Result<([u8; 32], [u8; 32], String), InfraError> {
    let base_info = format!("{EDGE_PROTOCOL}\n{}\n{site}\n{kid}\n{sid}", scope.as_str());
    let hkdf = Hkdf::<Sha256>::new(Some(salt), shared_secret);

    let mut req_key = [0_u8; 32];
    let req_info = format!("{base_info}\nreq");
    hkdf.expand(req_info.as_bytes(), &mut req_key)
        .map_err(|_| InfraError::Crypto {
            detail: "派生请求密钥失败".to_owned(),
        })?;

    let mut res_key = [0_u8; 32];
    let res_info = format!("{base_info}\nres");
    hkdf.expand(res_info.as_bytes(), &mut res_key)
        .map_err(|_| InfraError::Crypto {
            detail: "派生响应密钥失败".to_owned(),
        })?;

    Ok((req_key, res_key, base_info))
}

#[must_use]
pub fn build_nonce(sequence: u32) -> [u8; 12] {
    let mut nonce = [0_u8; 12];
    let seq_bytes = sequence.to_be_bytes();
    if let Some(tail) = nonce.get_mut(8..12) {
        tail.copy_from_slice(&seq_bytes);
    }
    nonce
}

#[must_use]
pub fn build_aad(session_info: &str, direction: &str, sequence: u32, timestamp_ms: u64) -> Vec<u8> {
    format!("{session_info}\n{direction}\n{sequence}\n{timestamp_ms}").into_bytes()
}

pub fn encrypt_envelope_payload(
    key: &[u8; 32],
    sequence: u32,
    aad: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>, InfraError> {
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
        .map_err(|_| InfraError::Crypto {
            detail: "AES-256-GCM 加密失败".to_owned(),
        })
}

pub fn decrypt_envelope_payload(
    key: &[u8; 32],
    sequence: u32,
    aad: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, InfraError> {
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
        .map_err(|_| InfraError::Crypto {
            detail: "AES-256-GCM 解密或 AAD 校验失败".to_owned(),
        })
}

#[must_use]
pub fn canonicalize_public_path(path_with_query: &str) -> String {
    let Some((path, query)) = path_with_query.split_once('?') else {
        return path_with_query.to_owned();
    };

    let mut pairs: Vec<(&str, &str)> = query
        .split('&')
        .filter(|part| !part.is_empty())
        .map(|part| part.split_once('=').unwrap_or((part, "")))
        .collect();
    pairs.sort_unstable();

    if pairs.is_empty() {
        return path.to_owned();
    }

    let sorted = pairs
        .into_iter()
        .map(|(k, v)| {
            if v.is_empty() {
                k.to_owned()
            } else {
                format!("{k}={v}")
            }
        })
        .collect::<Vec<_>>()
        .join("&");
    format!("{path}?{sorted}")
}

#[must_use]
pub fn resolve_cover_url(raw_url: &str) -> String {
    if raw_url.starts_with("http://") || raw_url.starts_with("https://") {
        return raw_url.to_owned();
    }
    let trimmed = raw_url.trim_start_matches('/');
    format!("{DEFAULT_IMAGE_ORIGIN}/{trimmed}")
}

pub struct EdgeHttpClient {
    http: reqwest::Client,
    base_url: Url,
    referer: String,
    clock: Arc<dyn Clock>,
    public_session: Mutex<Option<EdgeSessionKeys>>,
    dynamic_session: Mutex<Option<EdgeSessionKeys>>,
    playback_session: Mutex<Option<EdgeSessionKeys>>,
}

impl EdgeHttpClient {
    pub fn new(base_url: &str, clock: Arc<dyn Clock>) -> Result<Self, InfraError> {
        let parsed = Url::parse(base_url).map_err(|err| InfraError::Protocol {
            detail: format!("无效的上游基础地址 {base_url}: {err}"),
        })?;
        let referer = format!("{}/", parsed.as_str().trim_end_matches('/'));
        let http = reqwest::Client::builder()
            .user_agent(DEFAULT_USER_AGENT)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(20))
            .build()?;

        Ok(Self {
            http,
            base_url: parsed,
            referer,
            clock,
            public_session: Mutex::new(None),
            dynamic_session: Mutex::new(None),
            playback_session: Mutex::new(None),
        })
    }

    pub async fn request_json(
        &self,
        scope: EdgeChannelScope,
        method: &str,
        path_with_query: &str,
        body: Option<&Value>,
    ) -> Result<EdgeHttpResponse, InfraError> {
        if scope == EdgeChannelScope::Public
            && method.eq_ignore_ascii_case("GET")
            && body.is_none()
            && let Some(fast) = self.try_public_json_fast_path(path_with_query).await?
        {
            return Ok(fast);
        }

        self.request_encrypted_with_retry(scope, method, path_with_query, body)
            .await
    }

    pub async fn fetch_raw_bytes(
        &self,
        url: &str,
        headers: Option<HeaderMap>,
    ) -> Result<Vec<u8>, InfraError> {
        let mut req = self.http.get(url);
        if let Some(map) = headers {
            req = req.headers(map);
        }
        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(InfraError::Protocol {
                detail: format!("拉取二进制资源返回状态码 {status}"),
            });
        }
        Ok(resp.bytes().await?.to_vec())
    }

    pub async fn fetch_decrypted_cover(&self, cover_url: &str) -> Result<Vec<u8>, InfraError> {
        let resolved = resolve_cover_url(cover_url);
        let raw = self.fetch_raw_bytes(&resolved, None).await?;
        decrypt_cover_bytes(&raw)
    }

    async fn try_public_json_fast_path(
        &self,
        path_with_query: &str,
    ) -> Result<Option<EdgeHttpResponse>, InfraError> {
        let canonical = canonicalize_public_path(path_with_query);
        let url = self.join_url(EdgeChannelScope::Public.endpoint_path())?;
        let packet = InnerRequest {
            method: "GET",
            path: &canonical,
            body: String::new(),
            content_type: "",
        };
        let resp = self
            .http
            .post(url)
            .header("X-HG-Public-JSON", HeaderValue::from_static("1"))
            .header(reqwest::header::REFERER, &self.referer)
            .json(&packet)
            .send()
            .await?;

        let status = resp.status().as_u16();
        let has_public_header = resp
            .headers()
            .get("X-HG-Public-JSON")
            .and_then(|v| v.to_str().ok())
            == Some("1");
        let is_json = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|ct| ct.contains("application/json"));

        if !is_json || status == 426 || status == 403 {
            return Ok(None);
        }

        let raw_text = resp.text().await?;
        if status == 400
            && !has_public_header
            && let Ok(err_payload) = serde_json::from_str::<EdgeErrorPayload>(&raw_text)
            && matches!(
                err_payload.error.as_deref(),
                Some("invalid_request" | "unsupported_version")
            )
        {
            return Ok(None);
        }

        let body: Value = serde_json::from_str(&raw_text).map_err(|err| InfraError::Protocol {
            detail: format!("公共快路径 JSON 解析失败: {err}"),
        })?;
        Ok(Some(EdgeHttpResponse { status, body }))
    }

    async fn request_encrypted_with_retry(
        &self,
        scope: EdgeChannelScope,
        method: &str,
        path_with_query: &str,
        body: Option<&Value>,
    ) -> Result<EdgeHttpResponse, InfraError> {
        for attempt in 0..2_u8 {
            match self
                .request_encrypted_once(scope, method, path_with_query, body)
                .await
            {
                Ok(resp) => return Ok(resp),
                Err(InfraError::Protocol { detail })
                    if attempt == 0 && detail.contains("session_expired") =>
                {
                    self.invalidate_session(scope).await;
                }
                Err(err) => return Err(err),
            }
        }

        Err(InfraError::Protocol {
            detail: "边缘加密会话重试后仍失败".to_owned(),
        })
    }

    async fn request_encrypted_once(
        &self,
        scope: EdgeChannelScope,
        method: &str,
        path_with_query: &str,
        body: Option<&Value>,
    ) -> Result<EdgeHttpResponse, InfraError> {
        let (kid, sid, info, req_key, res_key, seq, clock_offset_secs) =
            self.next_session_params(scope).await?;
        let local_secs = i64::try_from(self.clock.now_epoch_millis() / 1_000).unwrap_or(0);
        let ts = u64::try_from(local_secs.saturating_add(clock_offset_secs).max(0)).unwrap_or(0);

        let body_bytes = match body {
            Some(val) => serde_json::to_vec(val).map_err(|err| InfraError::Protocol {
                detail: format!("序列化请求体失败: {err}"),
            })?,
            None => Vec::new(),
        };
        let inner = InnerRequestWrapper {
            request: InnerRequest {
                method,
                path: path_with_query,
                body: if body_bytes.is_empty() {
                    String::new()
                } else {
                    BASE64_STANDARD.encode(&body_bytes)
                },
                content_type: if body.is_some() {
                    "application/json"
                } else {
                    ""
                },
            },
        };
        let plaintext = serde_json::to_vec(&inner).map_err(|err| InfraError::Protocol {
            detail: format!("序列化内层信封失败: {err}"),
        })?;

        let req_aad = build_aad(&info, "req", seq, ts);
        let ciphertext = encrypt_envelope_payload(&req_key, seq, &req_aad, &plaintext)?;
        let envelope = EncryptedEnvelope {
            v: 1,
            kind: "call",
            kid: kid.clone(),
            sid: sid.clone(),
            seq,
            ts,
            ciphertext: BASE64_URL_SAFE_NO_PAD.encode(&ciphertext),
        };

        let url = self.join_url(scope.endpoint_path())?;
        let resp = self
            .http
            .post(url)
            .header(reqwest::header::REFERER, &self.referer)
            .json(&envelope)
            .send()
            .await?;
        let http_status = resp.status();
        let raw_text = resp.text().await?;

        if !http_status.is_success() {
            if let Ok(err_payload) = serde_json::from_str::<EdgeErrorPayload>(&raw_text)
                && err_payload.error.as_deref() == Some("session_expired")
            {
                return Err(InfraError::Protocol {
                    detail: "session_expired".to_owned(),
                });
            }
            return Err(InfraError::Protocol {
                detail: format!("边缘加密通道返回 {http_status}: {raw_text}"),
            });
        }

        let res_env: EncryptedResponseEnvelope =
            serde_json::from_str(&raw_text).map_err(|err| InfraError::Protocol {
                detail: format!("解析响应信封失败: {err}"),
            })?;
        decode_encrypted_response(&res_env, &res_key, &info, &kid, &sid, seq, ts)
    }

    async fn next_session_params(
        &self,
        scope: EdgeChannelScope,
    ) -> Result<(String, String, String, [u8; 32], [u8; 32], u32, i64), InfraError> {
        let mut guard = self.session_slot(scope).lock().await;
        let now_secs = i64::try_from(self.clock.now_epoch_millis() / 1_000).unwrap_or(0);
        let need_handshake = match guard.as_ref() {
            None => true,
            Some(sess) => {
                sess.sequence >= MAX_SEQUENCE
                    || now_secs.saturating_add(SESSION_REFRESH_MARGIN_SECS) >= sess.expires_at_secs
            }
        };

        if need_handshake {
            let fresh = self.perform_handshake(scope).await?;
            *guard = Some(fresh);
        }

        let Some(sess) = guard.as_mut() else {
            return Err(InfraError::Protocol {
                detail: "握手后会话为空".to_owned(),
            });
        };
        sess.sequence = sess.sequence.saturating_add(1);
        Ok((
            sess.kid.clone(),
            sess.sid.clone(),
            sess.info.clone(),
            sess.req_key,
            sess.res_key,
            sess.sequence,
            sess.clock_offset_secs,
        ))
    }

    async fn invalidate_session(&self, scope: EdgeChannelScope) {
        let mut guard = self.session_slot(scope).lock().await;
        *guard = None;
    }

    const fn session_slot(&self, scope: EdgeChannelScope) -> &Mutex<Option<EdgeSessionKeys>> {
        match scope {
            EdgeChannelScope::Public => &self.public_session,
            EdgeChannelScope::Dynamic => &self.dynamic_session,
            EdgeChannelScope::Playback => &self.playback_session,
        }
    }

    async fn perform_handshake(
        &self,
        scope: EdgeChannelScope,
    ) -> Result<EdgeSessionKeys, InfraError> {
        let secret = EphemeralSecret::generate();
        let client_pub = secret.public_key();
        let client_pub_bytes = client_pub.to_sec1_point(false);

        let payload = HandshakeRequest {
            v: 1,
            kind: "hello",
            client_pub: BASE64_URL_SAFE_NO_PAD.encode(client_pub_bytes.as_bytes()),
        };

        let url = self.join_url(scope.endpoint_path())?;
        let resp = self
            .http
            .post(url)
            .header(reqwest::header::REFERER, &self.referer)
            .json(&payload)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(InfraError::Protocol {
                detail: format!("边缘握手失败，状态码 {status}"),
            });
        }

        let hs: HandshakeResponse = resp.json().await.map_err(|err| InfraError::Protocol {
            detail: format!("边缘握手响应解析失败: {err}"),
        })?;
        let ttl = hs.expires_at.saturating_sub(hs.server_time);
        if hs.v != 1 || hs.kind != "hello" || ttl <= 0 || ttl > MAX_HANDSHAKE_TTL_SECS {
            return Err(InfraError::Protocol {
                detail: "边缘握手响应元数据非法".to_owned(),
            });
        }

        let server_pub_raw = BASE64_URL_SAFE_NO_PAD
            .decode(&hs.server_pub)
            .map_err(|err| InfraError::Crypto {
                detail: format!("服务端公钥 Base64Url 解码失败: {err}"),
            })?;
        let server_pub =
            PublicKey::from_sec1_bytes(&server_pub_raw).map_err(|_| InfraError::Crypto {
                detail: "服务端 P-256 公钥格式非法".to_owned(),
            })?;
        let salt = BASE64_URL_SAFE_NO_PAD
            .decode(&hs.salt)
            .map_err(|err| InfraError::Crypto {
                detail: format!("握手 salt Base64Url 解码失败: {err}"),
            })?;
        if salt.len() != EXPECTED_SALT_LEN {
            return Err(InfraError::Crypto {
                detail: "握手 salt 长度非法".to_owned(),
            });
        }

        let shared = secret.diffie_hellman(&server_pub);
        let (req_key, res_key, info) = derive_session_keys(
            shared.raw_secret_bytes().as_slice(),
            &salt,
            scope,
            &hs.site,
            &hs.kid,
            &hs.sid,
        )?;
        let local_now_secs = i64::try_from(self.clock.now_epoch_millis() / 1_000).unwrap_or(0);

        Ok(EdgeSessionKeys {
            kid: hs.kid,
            sid: hs.sid,
            site: hs.site,
            scope,
            info,
            clock_offset_secs: hs.server_time.saturating_sub(local_now_secs),
            expires_at_secs: local_now_secs.saturating_add(ttl),
            req_key,
            res_key,
            sequence: 0,
        })
    }

    fn join_url(&self, path_with_query: &str) -> Result<Url, InfraError> {
        self.base_url
            .join(path_with_query)
            .map_err(|err| InfraError::Protocol {
                detail: format!("拼接上游地址失败 {path_with_query}: {err}"),
            })
    }
}

fn decode_encrypted_response(
    res_env: &EncryptedResponseEnvelope,
    res_key: &[u8; 32],
    info: &str,
    kid: &str,
    sid: &str,
    seq: u32,
    ts: u64,
) -> Result<EdgeHttpResponse, InfraError> {
    if res_env.v != 1
        || res_env.kind != "result"
        || res_env.kid != kid
        || res_env.sid != sid
        || res_env.seq != seq
        || res_env.ts != ts
    {
        return Err(InfraError::Protocol {
            detail: "边缘响应信封元数据不匹配".to_owned(),
        });
    }

    let res_cipher = BASE64_URL_SAFE_NO_PAD
        .decode(&res_env.ciphertext)
        .map_err(|err| InfraError::Crypto {
            detail: format!("响应密文 Base64Url 解码失败: {err}"),
        })?;
    let res_aad = build_aad(info, "res", res_env.seq, res_env.ts);
    let res_plain = decrypt_envelope_payload(res_key, res_env.seq, &res_aad, &res_cipher)?;
    let inner_resp: InnerResponse =
        serde_json::from_slice(&res_plain).map_err(|err| InfraError::Protocol {
            detail: format!("解析内层响应失败: {err}"),
        })?;
    let decoded_body = if inner_resp.body.is_empty() {
        Vec::new()
    } else {
        BASE64_STANDARD
            .decode(&inner_resp.body)
            .map_err(|err| InfraError::Protocol {
                detail: format!("内层响应体 Base64 解码失败: {err}"),
            })?
    };
    let parsed_body = if decoded_body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&decoded_body).map_err(|err| InfraError::Protocol {
            detail: format!("内层响应体 JSON 解析失败: {err}"),
        })?
    };

    Ok(EdgeHttpResponse {
        status: inner_resp.status,
        body: parsed_body,
    })
}
