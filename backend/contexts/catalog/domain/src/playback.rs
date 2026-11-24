pub const DEFAULT_IMAGE_ORIGIN: &str = "https://pic.qnlycc.cn";
pub const DEFAULT_TICKET_TTL_SECS: u64 = 1800;
pub const TICKET_REFRESH_LEAD_SECS: u64 = 120;

#[must_use]
pub fn parse_auth_key_issued_at(stream_url: &str) -> Option<u64> {
    let (_, query) = stream_url.split_once('?')?;
    for pair in query.split('&') {
        if let Some(("auth_key", value)) = pair.split_once('=') {
            let (ts_part, _) = value.split_once('-')?;
            if (9..=11).contains(&ts_part.len()) && ts_part.bytes().all(|b| b.is_ascii_digit()) {
                return ts_part.parse::<u64>().ok();
            }
        }
    }
    None
}

#[must_use]
pub const fn compute_ticket_expiry(issued_at_epoch_secs: u64) -> u64 {
    issued_at_epoch_secs.saturating_add(DEFAULT_TICKET_TTL_SECS)
}

#[must_use]
pub const fn is_ticket_near_expiry(expires_at_epoch_secs: u64, now_epoch_secs: u64) -> bool {
    now_epoch_secs.saturating_add(TICKET_REFRESH_LEAD_SECS) >= expires_at_epoch_secs
}

#[must_use]
pub fn resolve_cover_url(raw_url: &str, image_origin: &str) -> String {
    let trimmed = raw_url.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed.to_owned();
    }
    if trimmed.starts_with("//") {
        return format!("https:{trimmed}");
    }
    let origin = image_origin.trim_end_matches('/');
    let suffix = trimmed.trim_start_matches('/');
    format!("{origin}/{suffix}")
}
