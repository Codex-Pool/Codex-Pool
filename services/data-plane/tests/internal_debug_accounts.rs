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
    support::ensure_test_security_env().await;
    dp_build_app_with_event_sink_and_allowed_keys(config, event_sink, allowed_keys).await
}

fn test_upstream_accounts() -> Vec<UpstreamAccount> {
    vec![
        UpstreamAccount {
            id: Uuid::new_v4(),
            label: "openai-account".to_string(),
            mode: UpstreamMode::OpenAiApiKey,
            base_url: "https://api.openai.com/v1".to_string(),
            bearer_token: "tok-openai".to_string(),
            chatgpt_account_id: None,
            enabled: true,
            priority: 100,
            created_at: Utc::now(),
        },
        UpstreamAccount {
            id: Uuid::new_v4(),
            label: "chatgpt-account".to_string(),
            mode: UpstreamMode::ChatGptSession,
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            bearer_token: "tok-chatgpt".to_string(),
            chatgpt_account_id: Some("acct_debug_1".to_string()),
            enabled: false,
            priority: 80,
            created_at: Utc::now(),
        },
    ]
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
async fn internal_debug_accounts_route_returns_404_when_debug_routes_disabled() {
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
                .uri("/internal/v1/debug/accounts")
                .header("authorization", "Bearer cp_disabled_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn internal_debug_accounts_route_returns_accounts_when_enabled_and_token_valid() {
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
    )
    .await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/internal/v1/debug/accounts")
                .header("authorization", "Bearer cp_valid_token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&body).unwrap();

    let accounts = payload["accounts"].as_array().unwrap();
    assert_eq!(accounts.len(), 2);

    let first = &accounts[0];
    assert!(first["id"].is_string());
    assert_eq!(first["label"], "openai-account");
    assert_eq!(first["mode"], "open_ai_api_key");
    assert_eq!(first["enabled"], true);
    assert_eq!(first["priority"], 100);
    assert_eq!(first["base_url"], "https://api.openai.com/v1");
    assert!(first["chatgpt_account_id"].is_null());
    assert!(first["temporarily_unhealthy"].is_boolean());
}

#[tokio::test]
async fn internal_debug_accounts_route_requires_bearer_token() {
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
                .uri("/internal/v1/debug/accounts")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
