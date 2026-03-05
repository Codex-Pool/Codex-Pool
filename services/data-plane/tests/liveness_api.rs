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
    support::ensure_test_security_env().await;
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

async fn build_test_app(accounts: Vec<UpstreamAccount>) -> Router {
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
            auth_validate_url: None,
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
async fn livez_returns_200_ok_json() {
    let app = build_test_app(vec![
        test_account("disabled-a", false),
        test_account("disabled-b", false),
    ])
    .await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/livez")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload, serde_json::json!({ "ok": true }));
}

#[tokio::test]
async fn readyz_returns_503_when_no_active_accounts_and_differs_from_livez() {
    let app = build_test_app(vec![
        test_account("disabled-a", false),
        test_account("disabled-b", false),
    ])
    .await;

    let readyz_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/readyz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(readyz_response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let readyz_body = readyz_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let readyz_payload: Value = serde_json::from_slice(&readyz_body).unwrap();
    assert_eq!(readyz_payload["ok"], false);
    assert_eq!(readyz_payload["reason"], "no_active_accounts");

    let livez_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/livez")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(livez_response.status(), StatusCode::OK);
}
