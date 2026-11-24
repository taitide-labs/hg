use hg_backend_contracts::catalog::{CdnAuthHeaders, PlaybackTicket, StreamEndpoints};
use hg_catalog_domain::error::CatalogError;
use hg_catalog_domain::playback::{compute_ticket_expiry, parse_auth_key_issued_at};
use hg_contracts::ids::EpisodeKey;
use serde_json::{Map, Value};

#[must_use]
pub fn resolve_ticket_expiry_epoch_secs(stream_url: &str, fallback_issued_at: u64) -> u64 {
    let issued_at = parse_auth_key_issued_at(stream_url).unwrap_or(fallback_issued_at);
    compute_ticket_expiry(issued_at)
}

#[must_use]
pub fn playback_request_path(key: EpisodeKey) -> String {
    format!(
        "/api/videos/{}/episodes/{}/playback",
        key.series_id, key.ep_num
    )
}

pub fn parse_playback_ticket(
    key: EpisodeKey,
    payload: &Value,
    fallback_now_epoch_secs: u64,
) -> Result<PlaybackTicket, CatalogError> {
    let data = payload
        .get("data")
        .and_then(Value::as_object)
        .ok_or_else(|| CatalogError::UpstreamPayloadMalformed {
            detail: "播放凭据响应缺少 data 对象".to_owned(),
        })?;

    let streams = parse_streams(data)?;
    let cdn = parse_cdn_headers(data.get("cdn").and_then(Value::as_object));
    let fallback_issued_at = data
        .get("timestamp")
        .and_then(Value::as_u64)
        .or_else(|| payload.get("req_time").and_then(Value::as_u64))
        .unwrap_or(fallback_now_epoch_secs);

    let primary_url = [&streams.hevc, &streams.main, &streams.preview]
        .into_iter()
        .flatten()
        .find(|url| parse_auth_key_issued_at(url).is_some())
        .map(String::as_str);

    let issued_at_epoch_secs = primary_url
        .and_then(parse_auth_key_issued_at)
        .unwrap_or(fallback_issued_at);
    let expires_at_epoch_secs = primary_url.map_or_else(
        || compute_ticket_expiry(issued_at_epoch_secs),
        |url| resolve_ticket_expiry_epoch_secs(url, fallback_issued_at),
    );

    Ok(PlaybackTicket {
        key,
        streams,
        cdn,
        issued_at_epoch_secs,
        expires_at_epoch_secs,
    })
}

fn parse_streams(data: &Map<String, Value>) -> Result<StreamEndpoints, CatalogError> {
    let streams_obj = data
        .get("streams")
        .and_then(Value::as_object)
        .ok_or_else(|| CatalogError::UpstreamPayloadMalformed {
            detail: "播放凭据响应缺少 streams 对象".to_owned(),
        })?;

    let streams = StreamEndpoints {
        hevc: read_non_empty_str(streams_obj, "hevc"),
        main: read_non_empty_str(streams_obj, "main"),
        preview: read_non_empty_str(streams_obj, "preview"),
    };

    if streams.hevc.is_none() && streams.main.is_none() && streams.preview.is_none() {
        return Err(CatalogError::EmptyStreamList);
    }

    Ok(streams)
}

fn parse_cdn_headers(cdn: Option<&Map<String, Value>>) -> CdnAuthHeaders {
    let Some(map) = cdn else {
        return CdnAuthHeaders {
            referer: String::new(),
            user_agent: String::new(),
            auth_header_name: None,
            auth_header_value: None,
            cdn_app: None,
        };
    };

    CdnAuthHeaders {
        referer: read_non_empty_str(map, "referer").unwrap_or_default(),
        user_agent: read_non_empty_str(map, "userAgent")
            .or_else(|| read_non_empty_str(map, "user_agent"))
            .unwrap_or_default(),
        auth_header_name: read_non_empty_str(map, "authKey")
            .or_else(|| read_non_empty_str(map, "auth_key")),
        auth_header_value: read_non_empty_str(map, "authValue")
            .or_else(|| read_non_empty_str(map, "auth_value")),
        cdn_app: read_string_or_u64(map, "app"),
    }
}

fn read_non_empty_str(map: &Map<String, Value>, key: &str) -> Option<String> {
    map.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn read_string_or_u64(map: &Map<String, Value>, key: &str) -> Option<String> {
    read_non_empty_str(map, key)
        .or_else(|| map.get(key).and_then(Value::as_u64).map(|v| v.to_string()))
}
