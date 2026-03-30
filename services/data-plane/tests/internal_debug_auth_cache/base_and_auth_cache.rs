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
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn build_app_with_event_sink_and_allowed_keys(
    config: DataPlaneConfig,
    event_sink: Arc<NoopEventSink>,
    allowed_keys: Vec<String>,
) -> anyhow::Result<Router> {
    let _env_guard = support::lock_env().await;
    dp_build_app_with_event_sink_and_allowed_keys(config, event_sink, allowed_keys).await
}

fn test_upstream_accounts() -> Vec<UpstreamAccount> {
    vec![UpstreamAccount {
        id: Uuid::new_v4(),
        label: "openai-account".to_string(),
        mode: UpstreamMode::OpenAiApiKey,
        base_url: "https://api.openai.com/v1".to_string(),
        bearer_token: "tok-openai".to_string(),
        chatgpt_account_id: None,
        enabled: true,
        priority: 100,
        created_at: Utc::now(),
    }]
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
async fn internal_debug_auth_cache_route_returns_404_when_debug_routes_disabled() {
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
                .uri("/internal/v1/debug/auth-cache")
                .header("authorization", "Bearer cp_disabled_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn internal_debug_auth_cache_route_requires_bearer_token() {
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
                .uri("/internal/v1/debug/auth-cache")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn internal_debug_auth_cache_route_returns_validator_cache_diagnostics_when_enabled() {
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
                .uri("/internal/v1/debug/auth-cache")
                .header("authorization", "Bearer cp_valid_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["auth_validator_enabled"], true);
    assert_eq!(payload["cached_principal_total"], 1);
    assert_eq!(payload["negative_cached_token_total"], 0);
    assert_eq!(payload["auth_fail_open"], true);
    assert_eq!(payload["allowlist_api_key_total"], 0);
}

#[tokio::test]
async fn internal_debug_auth_cache_route_returns_zero_cache_when_validator_disabled() {
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
                .uri("/internal/v1/debug/auth-cache")
                .header("authorization", "Bearer cp_allow_1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["auth_validator_enabled"], false);
    assert_eq!(payload["cached_principal_total"], 0);
    assert_eq!(payload["negative_cached_token_total"], 0);
    assert_eq!(payload["auth_fail_open"], false);
    assert_eq!(payload["allowlist_api_key_total"], 2);
}

#[tokio::test]
async fn internal_debug_auth_cache_route_reports_negative_cache_total_when_entries_exist() {
    let control_plane = MockServer::start().await;
    let tenant_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();

    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .and(body_json(json!({"token":"cp_admin_token"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": tenant_id,
            "api_key_id": api_key_id,
            "enabled": true,
            "cache_ttl_sec": 30
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&control_plane)
        .await;

    let app = build_test_app(
        true,
        Some(format!("{}/internal/v1/auth/validate", control_plane.uri())),
        false,
        Vec::new(),
    )
    .await;

    let unauthorized_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer cp_negative_token")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauthorized_response.status(), StatusCode::UNAUTHORIZED);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/internal/v1/debug/auth-cache")
                .header("authorization", "Bearer cp_admin_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["auth_validator_enabled"], true);
    assert_eq!(payload["cached_principal_total"], 1);
    assert_eq!(payload["negative_cached_token_total"], 1);
}

#[tokio::test]
async fn internal_debug_auth_cache_stats_route_returns_404_when_debug_routes_disabled() {
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
                .uri("/internal/v1/debug/auth-cache/stats")
                .header("authorization", "Bearer cp_disabled_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn internal_debug_auth_cache_stats_route_requires_bearer_token() {
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
                .uri("/internal/v1/debug/auth-cache/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn internal_debug_auth_cache_stats_route_returns_hit_miss_remote_counters_when_validator_enabled(
) {
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
        false,
        Vec::new(),
    )
    .await;

    let miss_response = app
        .clone()
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
    assert_eq!(miss_response.status(), StatusCode::OK);

    let stats_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/internal/v1/debug/auth-cache/stats")
                .header("authorization", "Bearer cp_valid_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(stats_response.status(), StatusCode::OK);

    let body = stats_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let payload: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["auth_validator_enabled"], true);
    assert_eq!(payload["cached_principal_total"], 1);
    assert_eq!(payload["cache_hit_count"], 1);
    assert_eq!(payload["cache_miss_count"], 1);
    assert_eq!(payload["remote_validate_count"], 1);
    assert_eq!(payload["negative_cache_hit_count"], 0);
    assert_eq!(payload["negative_cache_store_count"], 0);
}

#[tokio::test]
async fn internal_debug_auth_cache_stats_route_returns_zero_stats_when_validator_disabled() {
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
                .uri("/internal/v1/debug/auth-cache/stats")
                .header("authorization", "Bearer cp_allow_1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["auth_validator_enabled"], false);
    assert_eq!(payload["cached_principal_total"], 0);
    assert_eq!(payload["cache_hit_count"], 0);
    assert_eq!(payload["cache_miss_count"], 0);
    assert_eq!(payload["remote_validate_count"], 0);
    assert_eq!(payload["negative_cache_hit_count"], 0);
    assert_eq!(payload["negative_cache_store_count"], 0);
}

#[tokio::test]
async fn internal_debug_auth_cache_stats_reset_route_returns_404_when_debug_routes_disabled() {
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
                .method("POST")
                .uri("/internal/v1/debug/auth-cache/stats/reset")
                .header("authorization", "Bearer cp_disabled_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn internal_debug_auth_cache_stats_reset_route_requires_bearer_token() {
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
                .method("POST")
                .uri("/internal/v1/debug/auth-cache/stats/reset")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn internal_debug_auth_cache_stats_reset_route_resets_counters_when_validator_enabled() {
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
        false,
        Vec::new(),
    )
    .await;

    let miss_response = app
        .clone()
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
    assert_eq!(miss_response.status(), StatusCode::OK);

    let hit_response = app
        .clone()
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
    assert_eq!(hit_response.status(), StatusCode::OK);

    let reset_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/internal/v1/debug/auth-cache/stats/reset")
                .header("authorization", "Bearer cp_valid_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(reset_response.status(), StatusCode::OK);
    let reset_body = reset_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let payload: Value = serde_json::from_slice(&reset_body).unwrap();

    assert_eq!(payload["auth_validator_enabled"], true);
    assert!(payload["cache_hit_count_before"].as_u64().unwrap() > 0);
    assert!(payload["cache_miss_count_before"].as_u64().unwrap() > 0);
    assert!(payload["remote_validate_count_before"].as_u64().unwrap() > 0);
    assert_eq!(payload["negative_cache_hit_count_before"], 0);
    assert_eq!(payload["negative_cache_store_count_before"], 0);
    assert_eq!(payload["cache_hit_count_after"], 0);
    assert_eq!(payload["cache_miss_count_after"], 0);
    assert_eq!(payload["remote_validate_count_after"], 0);
    assert_eq!(payload["negative_cache_hit_count_after"], 0);
    assert_eq!(payload["negative_cache_store_count_after"], 0);
}
