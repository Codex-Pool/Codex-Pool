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
use serde_json::{json, Value};
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
    let _env_guard = support::lock_env().await;
    dp_build_app_with_event_sink_and_allowed_keys(config, event_sink, allowed_keys).await
}

fn test_upstream_accounts() -> Vec<UpstreamAccount> {
    vec![
        UpstreamAccount {
            id: Uuid::new_v4(),
            label: "enabled-a".to_string(),
            mode: UpstreamMode::OpenAiApiKey,
            base_url: "https://api.openai.com/v1".to_string(),
            bearer_token: "tok-enabled".to_string(),
            chatgpt_account_id: None,
            enabled: true,
            priority: 100,
            created_at: Utc::now(),
        },
        UpstreamAccount {
            id: Uuid::new_v4(),
            label: "disabled-b".to_string(),
            mode: UpstreamMode::OpenAiApiKey,
            base_url: "https://api.openai.com/v1".to_string(),
            bearer_token: "tok-disabled".to_string(),
            chatgpt_account_id: None,
            enabled: false,
            priority: 100,
            created_at: Utc::now(),
        },
    ]
}

async fn build_test_app(
    enable_internal_debug_routes: bool,
    auth_validate_url: Option<String>,
    auth_fail_open: bool,
    allowed_api_keys: Vec<String>,
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
            auth_fail_open,
            enable_internal_debug_routes,
        },
        Arc::new(NoopEventSink),
        allowed_api_keys,
    )
    .await
    .expect("app should build")
}

#[tokio::test]
async fn internal_debug_state_route_returns_404_when_debug_routes_disabled() {
    let control_plane = MockServer::start().await;
    let app = build_test_app(
        false,
        Some(format!("{}/internal/v1/auth/validate", control_plane.uri())),
        false,
        Vec::new(),
    )
    .await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/internal/v1/debug/state")
                .header("authorization", "Bearer cp_disabled_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn internal_debug_state_route_returns_runtime_snapshot_when_enabled_and_token_valid() {
    let control_plane = MockServer::start().await;
    let tenant_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();

    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": tenant_id,
            "api_key_id": api_key_id,
            "enabled": true,
            "cache_ttl_sec": 30
        })))
        .mount(&control_plane)
        .await;

    let app = build_test_app(
        true,
        Some(format!("{}/internal/v1/auth/validate", control_plane.uri())),
        true,
        Vec::new(),
    )
    .await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/internal/v1/debug/state")
                .header("authorization", "Bearer cp_valid_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["snapshot_revision"], 0);
    assert_eq!(payload["account_total"], 2);
    assert_eq!(payload["active_account_total"], 1);
    assert_eq!(payload["auth_mode"], "online_validator");
    assert_eq!(payload["auth_fail_open"], true);
    assert_eq!(payload["allowlist_api_key_total"], 0);
    assert_eq!(payload["auth_validator_enabled"], true);
    assert_eq!(payload["sticky_session_total"], 0);
    assert_eq!(payload["sticky_hit_count"], 0);
    assert_eq!(payload["sticky_miss_count"], 0);
    assert_eq!(payload["sticky_rebind_count"], 0);
    assert_eq!(payload["sticky_mapping_total"], 0);
    assert_eq!(payload["sticky_hit_ratio"], 0.0);
    assert_eq!(payload["failover_enabled"], true);
    assert_eq!(payload["same_account_quick_retry_max"], 1);
    assert_eq!(payload["request_failover_wait_ms"], 2_000);
    assert_eq!(payload["retry_poll_interval_ms"], 100);
    assert_eq!(payload["invalid_request_guard_enabled"], true);
    assert_eq!(payload["invalid_request_guard_window_sec"], 30);
    assert_eq!(payload["invalid_request_guard_threshold"], 12);
    assert_eq!(payload["invalid_request_guard_block_ttl_sec"], 120);
    assert_eq!(payload["sticky_prefer_non_conflicting"], true);
    assert_eq!(payload["shared_routing_cache_enabled"], true);
    assert_eq!(payload["enable_metered_stream_billing"], true);
    assert_eq!(payload["billing_authorize_required_for_stream"], true);
    assert_eq!(payload["stream_billing_reserve_microcredits"], 2_000_000);
    assert_eq!(payload["billing_dynamic_preauth_enabled"], true);
    assert_eq!(payload["billing_preauth_expected_output_tokens"], 256);
    assert_eq!(payload["billing_preauth_safety_factor"], 1.3);
    assert_eq!(payload["billing_preauth_min_microcredits"], 1_000);
    assert_eq!(
        payload["billing_preauth_max_microcredits"],
        1_000_000_000_000i64
    );
    assert_eq!(payload["billing_preauth_unit_price_microcredits"], 10_000);
    assert_eq!(payload["failover_attempt_total"], 0);
    assert_eq!(payload["failover_success_total"], 0);
    assert_eq!(payload["failover_exhausted_total"], 0);
    assert_eq!(payload["same_account_retry_total"], 0);
    assert_eq!(payload["invalid_request_guard_block_total"], 0);
    assert_eq!(payload["billing_preauth_dynamic_total"], 0);
    assert_eq!(payload["billing_preauth_fallback_total"], 0);
    assert_eq!(payload["billing_preauth_amount_microcredits_sum"], 0);
    assert_eq!(payload["billing_preauth_error_ratio_count_total"], 0);
    assert_eq!(payload["billing_preauth_error_ratio_avg"], 0.0);
    assert_eq!(payload["billing_preauth_error_ratio_p50"], 0.0);
    assert_eq!(payload["billing_preauth_error_ratio_p95"], 0.0);
    assert_eq!(payload["billing_preauth_capture_missing_total"], 0);
    assert_eq!(payload["billing_settle_complete_total"], 0);
    assert_eq!(payload["billing_release_without_capture_total"], 0);
    assert_eq!(payload["billing_settle_complete_ratio"], 0.0);
    assert_eq!(payload["billing_release_without_capture_ratio"], 0.0);
    assert_eq!(
        payload["billing_preauth_model_error_stats"]
            .as_array()
            .map(|items| items.len()),
        Some(0)
    );
    assert_eq!(payload["stream_usage_estimated_total"], 0);
    assert_eq!(payload["stream_response_total"], 0);
    assert_eq!(payload["stream_protocol_sse_header_total"], 0);
    assert_eq!(payload["stream_protocol_header_missing_total"], 0);
    assert_eq!(payload["stream_usage_json_line_fallback_total"], 0);
    assert_eq!(payload["stream_protocol_sse_header_hit_ratio"], 0.0);
    assert_eq!(payload["stream_protocol_header_missing_hit_ratio"], 0.0);
    assert_eq!(payload["stream_usage_json_line_fallback_hit_ratio"], 0.0);
    assert_eq!(payload["routing_cache_local_sticky_hit_total"], 0);
    assert_eq!(payload["routing_cache_local_sticky_miss_total"], 0);
    assert_eq!(payload["routing_cache_shared_sticky_hit_total"], 0);
    assert_eq!(payload["routing_cache_shared_sticky_miss_total"], 0);
}

#[tokio::test]
async fn internal_debug_state_route_returns_allowlist_mode_details_when_validator_disabled() {
    let app = build_test_app(
        true,
        None,
        false,
        vec!["cp_allow_1".to_string(), "cp_allow_2".to_string()],
    )
    .await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/internal/v1/debug/state")
                .header("authorization", "Bearer cp_allow_1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["auth_mode"], "allowlist");
    assert_eq!(payload["auth_fail_open"], false);
    assert_eq!(payload["allowlist_api_key_total"], 2);
    assert_eq!(payload["auth_validator_enabled"], false);
    assert_eq!(payload["sticky_session_total"], 0);
    assert_eq!(payload["sticky_mapping_total"], 0);
}

#[tokio::test]
async fn internal_debug_state_route_requires_bearer_token() {
    let control_plane = MockServer::start().await;
    let app = build_test_app(
        true,
        Some(format!("{}/internal/v1/auth/validate", control_plane.uri())),
        false,
        Vec::new(),
    )
    .await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/internal/v1/debug/state")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
