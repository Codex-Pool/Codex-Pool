use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::Router;
use codex_pool_core::model::{RoutingStrategy, UpstreamAccount, UpstreamMode};
use data_plane::app::build_app_with_event_sink_and_allowed_keys;
use data_plane::config::DataPlaneConfig;
use data_plane::event::NoopEventSink;
use http::Request;
use http::StatusCode;
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support;

fn test_account(base_url: String, token: &str) -> UpstreamAccount {
    UpstreamAccount {
        id: Uuid::new_v4(),
        label: "acc-1".to_string(),
        mode: UpstreamMode::ChatGptSession,
        base_url,
        bearer_token: token.to_string(),
        chatgpt_account_id: Some("acct_123".to_string()),
        enabled: true,
        priority: 100,
        created_at: chrono::Utc::now(),
    }
}

async fn build_test_app_with_allowed_keys(allowed_keys: Vec<String>) -> (Router, MockServer) {
    support::ensure_test_security_env().await;
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("{}", "application/json"))
        .mount(&upstream)
        .await;

    let cfg = DataPlaneConfig {
        listen_addr: "127.0.0.1:0".parse().unwrap(),
        routing_strategy: RoutingStrategy::RoundRobin,
        upstream_accounts: vec![test_account(upstream.uri(), "upstream-token")],
        account_ejection_ttl_sec: 30,
        enable_request_failover: true,
        same_account_quick_retry_max: 1,
        request_failover_wait_ms: 2_000,
        retry_poll_interval_ms: 100,
        sticky_prefer_non_conflicting: true,
        shared_routing_cache_enabled: true,
        enable_metered_stream_billing: true,
        billing_authorize_required_for_stream: true,
        stream_billing_reserve_microcredits: 2_000_000,
        billing_dynamic_preauth_enabled: true,
        billing_preauth_expected_output_tokens: 256,
        billing_preauth_safety_factor: 1.3,
        billing_preauth_min_microcredits: 1_000,
        billing_preauth_max_microcredits: 1_000_000_000_000,
        billing_preauth_unit_price_microcredits: 10_000,
        stream_billing_drain_timeout_ms: 5_000,
        billing_capture_retry_max: 3,
        billing_capture_retry_backoff_ms: 200,
        redis_url: None,
        auth_validate_url: None,
        auth_validate_cache_ttl_sec: 30,
        auth_validate_negative_cache_ttl_sec: 5,
        auth_fail_open: false,
        enable_internal_debug_routes: false,
    };

    let app =
        build_app_with_event_sink_and_allowed_keys(cfg, Arc::new(NoopEventSink), allowed_keys)
            .await
            .expect("app should build");
    (app, upstream)
}

#[tokio::test]
async fn rejects_request_without_bearer_api_key() {
    let (app, _upstream) = build_test_app_with_allowed_keys(vec!["cp_test_key".to_string()]).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(!request_id.is_empty());
    let payload: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(payload["error"]["code"], "unauthorized");
}

#[tokio::test]
async fn keeps_client_request_id_in_response_header() {
    let (app, _upstream) = build_test_app_with_allowed_keys(vec!["cp_test_key".to_string()]).await;
    let request_id = "req-client-header-001";

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer cp_test_key")
                .header("x-request-id", request_id)
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok()),
        Some(request_id)
    );
}
