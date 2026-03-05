use std::sync::Arc;

use axum::body::Body;
use axum::Router;
use chrono::Utc;
use codex_pool_core::model::{RoutingStrategy, UpstreamAccount, UpstreamMode};
use data_plane::app::build_app_with_event_sink_and_allowed_keys as dp_build_app_with_event_sink_and_allowed_keys;
use data_plane::config::DataPlaneConfig;
use data_plane::event::NoopEventSink;
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support;

async fn build_app_with_event_sink_and_allowed_keys(
    config: DataPlaneConfig,
    event_sink: Arc<NoopEventSink>,
    allowed_keys: Vec<String>,
) -> anyhow::Result<Router> {
    support::ensure_test_security_env().await;
    dp_build_app_with_event_sink_and_allowed_keys(config, event_sink, allowed_keys).await
}

fn test_upstream_accounts() -> Vec<UpstreamAccount> {
    vec![UpstreamAccount {
        id: Uuid::new_v4(),
        label: "enabled-a".to_string(),
        mode: UpstreamMode::OpenAiApiKey,
        base_url: "https://api.openai.com/v1".to_string(),
        bearer_token: "tok-enabled".to_string(),
        chatgpt_account_id: None,
        enabled: true,
        priority: 100,
        created_at: Utc::now(),
    }]
}

async fn build_test_app(
    enable_internal_debug_routes: bool,
    auth_validate_url: Option<String>,
) -> Router {
    build_app_with_event_sink_and_allowed_keys(
        DataPlaneConfig {
            listen_addr: "127.0.0.1:0".parse().unwrap(),
            routing_strategy: RoutingStrategy::RoundRobin,
            upstream_accounts: test_upstream_accounts(),
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
            auth_validate_url,
            auth_validate_cache_ttl_sec: 30,
            auth_validate_negative_cache_ttl_sec: 5,
            auth_fail_open: false,
            enable_internal_debug_routes,
        },
        Arc::new(NoopEventSink),
        Vec::new(),
    )
    .await
    .expect("app should build")
}

#[tokio::test]
async fn internal_metrics_route_returns_404_when_debug_routes_disabled() {
    let control_plane = MockServer::start().await;
    let app = build_test_app(
        false,
        Some(format!("{}/internal/v1/auth/validate", control_plane.uri())),
    )
    .await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/internal/v1/metrics")
                .header("authorization", "Bearer cp_disabled_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn internal_metrics_route_requires_bearer_token() {
    let control_plane = MockServer::start().await;
    let app = build_test_app(
        true,
        Some(format!("{}/internal/v1/auth/validate", control_plane.uri())),
    )
    .await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/internal/v1/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn internal_metrics_route_returns_prometheus_payload_when_enabled_and_token_valid() {
    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "tenant_id": Uuid::new_v4(),
            "api_key_id": Uuid::new_v4(),
            "enabled": true,
            "cache_ttl_sec": 30
        })))
        .mount(&control_plane)
        .await;

    let app = build_test_app(
        true,
        Some(format!("{}/internal/v1/auth/validate", control_plane.uri())),
    )
    .await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/internal/v1/metrics")
                .header("authorization", "Bearer cp_metrics_valid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    assert!(content_type.contains("text/plain"));

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let payload = std::str::from_utf8(&bytes).unwrap();
    assert!(payload.contains("codex_data_plane_failover_enabled 1"));
    assert!(payload.contains("codex_data_plane_failover_attempt_total 0"));
    assert!(payload.contains("codex_data_plane_invalid_request_guard_enabled 1"));
    assert!(payload.contains("codex_data_plane_invalid_request_guard_window_sec 30"));
    assert!(payload.contains("codex_data_plane_invalid_request_guard_threshold 12"));
    assert!(payload.contains("codex_data_plane_invalid_request_guard_block_ttl_sec 120"));
    assert!(payload.contains("codex_data_plane_invalid_request_guard_block_total 0"));
    assert!(payload.contains("codex_data_plane_billing_dynamic_preauth_enabled 1"));
    assert!(payload.contains("codex_data_plane_billing_preauth_expected_output_tokens 256"));
    assert!(payload.contains("codex_data_plane_billing_preauth_safety_factor 1.3"));
    assert!(payload.contains("codex_data_plane_billing_preauth_min_microcredits 1000"));
    assert!(payload.contains("codex_data_plane_billing_preauth_max_microcredits 1000000000000"));
    assert!(payload.contains("codex_data_plane_billing_preauth_unit_price_microcredits 10000"));
    assert!(payload.contains("codex_data_plane_billing_preauth_dynamic_total 0"));
    assert!(payload.contains("codex_data_plane_billing_preauth_fallback_total 0"));
    assert!(payload.contains("codex_data_plane_billing_preauth_amount_microcredits_sum 0"));
    assert!(payload.contains("codex_data_plane_billing_preauth_error_ratio_count_total 0"));
    assert!(payload.contains("codex_data_plane_billing_preauth_error_ratio_avg 0"));
    assert!(payload.contains("codex_data_plane_billing_preauth_error_ratio_p50 0"));
    assert!(payload.contains("codex_data_plane_billing_preauth_error_ratio_p95 0"));
    assert!(payload.contains("codex_data_plane_billing_preauth_capture_missing_total 0"));
    assert!(payload.contains("codex_data_plane_billing_settle_complete_total 0"));
    assert!(payload.contains("codex_data_plane_billing_release_without_capture_total 0"));
    assert!(payload.contains("codex_data_plane_billing_settle_complete_ratio 0"));
    assert!(payload.contains("codex_data_plane_billing_release_without_capture_ratio 0"));
    assert!(payload.contains("codex_data_plane_stream_usage_estimated_total 0"));
    assert!(payload.contains("codex_data_plane_stream_response_total 0"));
    assert!(payload.contains("codex_data_plane_stream_protocol_sse_header_total 0"));
    assert!(payload.contains("codex_data_plane_stream_protocol_header_missing_total 0"));
    assert!(payload.contains("codex_data_plane_stream_usage_json_line_fallback_total 0"));
    assert!(payload.contains("codex_data_plane_stream_protocol_sse_header_hit_ratio 0"));
    assert!(payload.contains("codex_data_plane_stream_protocol_header_missing_hit_ratio 0"));
    assert!(payload.contains("codex_data_plane_stream_usage_json_line_fallback_hit_ratio 0"));
    assert!(payload.contains("codex_data_plane_sticky_hit_total 0"));
    assert!(payload.contains("codex_data_plane_routing_cache_local_sticky_hit_total 0"));
}
