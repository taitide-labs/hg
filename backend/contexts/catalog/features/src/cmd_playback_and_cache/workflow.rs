use hg_catalog_domain::playback::{compute_ticket_expiry, parse_auth_key_issued_at};

#[must_use]
pub fn resolve_ticket_expiry_epoch_secs(stream_url: &str, fallback_issued_at: u64) -> u64 {
    let issued_at = parse_auth_key_issued_at(stream_url).unwrap_or(fallback_issued_at);
    compute_ticket_expiry(issued_at)
}
