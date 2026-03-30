use std::sync::Arc;

use axum::body::Body;
use axum::Router;
use codex_pool_core::model::{RoutingStrategy, UpstreamAccount, UpstreamMode};
use data_plane::app::build_app_with_event_sink as dp_build_app_with_event_sink;
use data_plane::config::DataPlaneConfig;
use data_plane::event::NoopEventSink;
use http::Request;
use http::StatusCode;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support;

async fn build_app_with_event_sink(
    config: DataPlaneConfig,
    event_sink: Arc<NoopEventSink>,
) -> anyhow::Result<Router> {
    let _env_guard = support::lock_env().await;
    dp_build_app_with_event_sink(config, event_sink).await
}

const OPENAI_BETA: &str = "responses_websockets=2026-02-04";
const X_OPENAI_SUBAGENT: &str = "review";
const X_CODEX_TURN_STATE: &str = "turn-state-contract";
const X_CODEX_TURN_METADATA: &str = "turn-meta-contract";
const X_CODEX_BETA_FEATURES: &str = "responses_websockets";
const SESSION_ID: &str = "session-contract";

fn test_account(base_url: String, token: &str) -> UpstreamAccount {
    UpstreamAccount {
        id: Uuid::new_v4(),
        label: "compat-contract-account".to_string(),
        mode: UpstreamMode::ChatGptSession,
        base_url,
        bearer_token: token.to_string(),
        chatgpt_account_id: Some("acct_compat_contract".to_string()),
        enabled: true,
        priority: 100,
        created_at: chrono::Utc::now(),
    }
}

async fn test_app(accounts: Vec<UpstreamAccount>) -> Router {
    let cfg = DataPlaneConfig {
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
    };

    build_app_with_event_sink(cfg, Arc::new(NoopEventSink))
        .await
        .expect("app should build")
}

fn compat_request(route: &str, body: Body) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(route)
        .header("authorization", "Bearer tenant-token")
        .header("openai-beta", OPENAI_BETA)
        .header("x-openai-subagent", X_OPENAI_SUBAGENT)
        .header("x-codex-turn-state", X_CODEX_TURN_STATE)
        .header("x-codex-turn-metadata", X_CODEX_TURN_METADATA)
        .header("x-codex-beta-features", X_CODEX_BETA_FEATURES)
        .header("session_id", SESSION_ID)
        .body(body)
        .unwrap()
}

#[tokio::test]
async fn routes_and_headers_contract_minimal_set() {
    let upstream = MockServer::start().await;
    let routes = [
        "/v1/responses",
        "/backend-api/codex/responses",
        "/v1/chat/completions",
    ];

    for route in &routes {
        Mock::given(method("POST"))
            .and(path(*route))
            .and(header("authorization", "Bearer upstream-token"))
            .and(header("chatgpt-account-id", "acct_compat_contract"))
            .and(header("openai-beta", OPENAI_BETA))
            .and(header("x-openai-subagent", X_OPENAI_SUBAGENT))
            .and(header("x-codex-turn-state", X_CODEX_TURN_STATE))
            .and(header("x-codex-turn-metadata", X_CODEX_TURN_METADATA))
            .and(header("x-codex-beta-features", X_CODEX_BETA_FEATURES))
            .and(header("session_id", SESSION_ID))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"route": route})))
            .mount(&upstream)
            .await;
    }

    let app = test_app(vec![test_account(upstream.uri(), "upstream-token")]).await;

    for route in &routes {
        let response = app
            .clone()
            .oneshot(compat_request(route, Body::from("{}")))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK, "route: {route}");
        let payload: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(payload["route"], *route, "route: {route}");
    }
}

#[tokio::test]
async fn sse_passthrough_keeps_event_stream_intact() {
    let upstream = MockServer::start().await;
    let sse_payload = "event: response.output_text.delta\ndata: {\"delta\":\"hello\"}\n\n";

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(sse_payload, "text/event-stream"))
        .mount(&upstream)
        .await;

    let app = test_app(vec![test_account(upstream.uri(), "upstream-token")]).await;

    let response = app
        .oneshot(compat_request(
            "/v1/responses",
            Body::from("{\"stream\":true}"),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get(http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap();
    assert!(
        content_type.contains("text/event-stream"),
        "actual content-type: {content_type}"
    );

    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body.as_ref(), sse_payload.as_bytes());
}
