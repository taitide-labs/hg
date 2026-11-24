use hg_backend_contracts::error::PortError;
use serde_json::json;

use super::normalize_upstream_response;

#[test]
fn accepts_http_200_with_business_status_one() {
    let body = json!({ "status": 1, "msg": "ok", "data": { "id": 1 } });
    let normalized = normalize_upstream_response(200, body.clone(), "/api/videos/1")
        .expect("status 1 should succeed");
    assert_eq!(normalized, body);
}

#[test]
fn maps_http_200_business_not_found_to_port_not_found() {
    let body = json!({ "status": 0, "msg": "视频不存在", "req_time": 1_700_000_000_u64 });
    let err = normalize_upstream_response(200, body, "/api/videos/999")
        .expect_err("business not found should fail");
    assert!(matches!(err, PortError::NotFound { .. }));
}

#[test]
fn maps_http_200_business_rejection_to_upstream_rejected() {
    let body = json!({ "status": 0, "msg": "请求过于频繁" });
    let err = normalize_upstream_response(200, body, "/api/search")
        .expect_err("business rate limit should fail");
    assert!(matches!(err, PortError::UpstreamRejected { detail } if detail.contains("频繁")));
}

#[test]
fn maps_rfc9457_problem_details_and_gateway_errors() {
    let problem = json!({
        "type": "https://example.com/problems/not-found",
        "title": "Not Found",
        "status": 404,
        "detail": "资源未找到"
    });
    let err = normalize_upstream_response(404, problem, "/api/videos/404")
        .expect_err("rfc9457 404 should map to NotFound");
    assert!(matches!(err, PortError::NotFound { .. }));

    let gateway_err = json!({ "error": "invalid_request" });
    let err = normalize_upstream_response(400, gateway_err, "/api/public/request")
        .expect_err("gateway error should map to UpstreamRejected");
    assert!(matches!(err, PortError::UpstreamRejected { detail } if detail == "invalid_request"));
}
