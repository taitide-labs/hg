use crate::playback::{
    DEFAULT_IMAGE_ORIGIN, compute_ticket_expiry, is_ticket_near_expiry, parse_auth_key_issued_at,
    resolve_cover_url,
};
use crate::scope::{EdgeScope, resolve_edge_scope};

#[test]
fn routes_public_dynamic_and_playback_scopes_correctly() {
    assert_eq!(resolve_edge_scope("GET", "/api/home"), EdgeScope::Public);
    assert_eq!(
        resolve_edge_scope("GET", "/api/videos?sort=hot&page=1"),
        EdgeScope::Public
    );
    assert_eq!(
        resolve_edge_scope("GET", "/api/videos?sort=random&page=1"),
        EdgeScope::Dynamic
    );
    assert_eq!(
        resolve_edge_scope("GET", "/api/videos?sort=rand&page=1"),
        EdgeScope::Dynamic
    );
    assert_eq!(
        resolve_edge_scope("GET", "/api/search/suggest?q=hello"),
        EdgeScope::Dynamic
    );
    assert_eq!(
        resolve_edge_scope("GET", "/api/videos/14288/episodes/1/comments"),
        EdgeScope::Dynamic
    );
    assert_eq!(
        resolve_edge_scope("POST", "/api/videos/14288/episodes/1/playback"),
        EdgeScope::Playback
    );
}

#[test]
fn parses_auth_key_timestamp_and_evaluates_expiry() {
    let url = "https://cdn.example.com/v/1.m3u8?auth_key=1730000000-abc-0-deadbeef";
    let issued = parse_auth_key_issued_at(url).expect("should parse auth_key");
    assert_eq!(issued, 1_730_000_000);

    let expires = compute_ticket_expiry(issued);
    assert!(!is_ticket_near_expiry(expires, issued + 100));
    assert!(is_ticket_near_expiry(expires, expires - 120));
    assert!(is_ticket_near_expiry(expires, expires + 10));
}

#[test]
fn normalizes_relative_and_protocol_relative_cover_urls() {
    assert_eq!(
        resolve_cover_url("/img/a.jpg", DEFAULT_IMAGE_ORIGIN),
        "https://pic.qnlycc.cn/img/a.jpg"
    );
    assert_eq!(
        resolve_cover_url("//img.example.com/b.jpg", DEFAULT_IMAGE_ORIGIN),
        "https://img.example.com/b.jpg"
    );
    assert_eq!(
        resolve_cover_url("https://img.example.com/c.jpg", DEFAULT_IMAGE_ORIGIN),
        "https://img.example.com/c.jpg"
    );
}
