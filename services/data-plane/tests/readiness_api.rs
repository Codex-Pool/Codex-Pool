use std::sync::Arc;

use axum::body::Body;
use axum::Router;
use codex_pool_core::model::{RoutingStrategy, UpstreamAccount, UpstreamMode};
use data_plane::app::build_app_with_event_sink_and_allowed_keys as dp_build_app_with_event_sink_and_allowed_keys;
use data_plane::config::DataPlaneConfig;
use data_plane::event::NoopEventSink;
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

use crate::support;

async fn build_app_with_event_sink_and_allowed_keys(
    config: DataPlaneConfig,
    event_sink: Arc<NoopEventSink>,
    allowed_keys: Vec<String>,
) -> anyhow::Result<Router> {
    let _env_guard = support::lock_env().await;
    dp_build_app_with_event_sink_and_allowed_keys(config, event_sink, allowed_keys).await
}

fn test_account(label: &str, enabled: bool) -> UpstreamAccount {
    UpstreamAccount {
        id: Uuid::new_v4(),
        label: label.to_string(),
        mode: UpstreamMode::OpenAiApiKey,
        base_url: "https://api.openai.com/v1".to_string(),
        bearer_token: format!("tok-{label}"),
        chatgpt_account_id: None,
        enabled,
        priority: 100,
        created_at: chrono::Utc::now(),
    }
}

async fn build_test_app(
    accounts: Vec<UpstreamAccount>,
    auth_validate_url: Option<String>,
) -> Router {
    build_app_with_event_sink_and_allowed_keys(
        DataPlaneConfig {
            listen_addr: "127.0.0.1:0".parse().unwrap(),
            routing_strategy: RoutingStrategy::RoundRobin,
            upstream_accounts: accounts,
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
            enable_internal_debug_routes: false,
        },
        Arc::new(NoopEventSink),
        Vec::new(),
    )
    .await
    .expect("app should build")
}

#[tokio::test]
async fn readyz_returns_ok_when_active_accounts_exist() {
    let app = build_test_app(
        vec![
            test_account("enabled", true),
            test_account("disabled", false),
        ],
        Some("https://control-plane.test/internal/v1/auth/validate".to_string()),
    )
    .await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/readyz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["ok"], true);
    assert_eq!(payload["reason"], "ready");
    assert_eq!(payload["account_total"], 2);
    assert_eq!(payload["active_account_total"], 1);
    assert_eq!(payload["auth_validator_enabled"], true);
}

#[tokio::test]
async fn readyz_returns_503_error_envelope_when_no_active_accounts() {
    let app = build_test_app(
        vec![
            test_account("disabled-a", false),
            test_account("disabled-b", false),
        ],
        None,
    )
    .await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/readyz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["error"]["code"], "not_ready");
    assert_eq!(payload["error"]["message"], "no active upstream accounts");
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["reason"], "no_active_accounts");
    assert_eq!(payload["account_total"], 2);
    assert_eq!(payload["active_account_total"], 0);
    assert_eq!(payload["auth_validator_enabled"], false);
}
