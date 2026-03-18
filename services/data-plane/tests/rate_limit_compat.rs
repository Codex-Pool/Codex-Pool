use std::sync::Arc;

use axum::body::Body;
use axum::Router;
use codex_pool_core::model::{RoutingStrategy, UpstreamAccount, UpstreamMode};
use data_plane::app::build_app_with_event_sink as dp_build_app_with_event_sink;
use data_plane::config::DataPlaneConfig;
use data_plane::event::NoopEventSink;
use http::Request;
use http::StatusCode;
use tower::ServiceExt;
use uuid::Uuid;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

mod support;

const OPENAI_BETA: &str = "responses_websockets=2026-02-04";
const X_CODEX_TURN_STATE: &str = "turn-state-rate-limit";
const X_CODEX_TURN_METADATA: &str = "turn-meta-rate-limit";
const X_CODEX_BETA_FEATURES: &str = "responses_websockets";

async fn build_app_with_event_sink(
    config: DataPlaneConfig,
    event_sink: Arc<NoopEventSink>,
) -> anyhow::Result<Router> {
    support::ensure_test_security_env().await;
    dp_build_app_with_event_sink(config, event_sink).await
}

fn test_account(base_url: String, token: &str) -> UpstreamAccount {
    UpstreamAccount {
        id: Uuid::new_v4(),
        label: "rate-limit-compat-account".to_string(),
        mode: UpstreamMode::ChatGptSession,
        base_url,
        bearer_token: token.to_string(),
        chatgpt_account_id: Some("acct_rate_limit_compat".to_string()),
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

fn plain_api_request(route: &str, body: Body) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(route)
        .header("authorization", "Bearer tenant-token")
        .body(body)
        .unwrap()
}

fn compat_request(route: &str, body: Body) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(route)
        .header("authorization", "Bearer tenant-token")
        .header("openai-beta", OPENAI_BETA)
        .header("x-codex-turn-state", X_CODEX_TURN_STATE)
        .header("x-codex-turn-metadata", X_CODEX_TURN_METADATA)
        .header("x-codex-beta-features", X_CODEX_BETA_FEATURES)
        .body(body)
        .unwrap()
}

#[tokio::test]
async fn plain_api_stream_response_does_not_expose_upstream_codex_rate_limit_headers() {
    let upstream = MockServer::start().await;
    let sse_payload = "event: response.output_text.delta\ndata: {\"delta\":\"hello\"}\n\n";

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(header("authorization", "Bearer upstream-token"))
        .respond_with(
            ResponseTemplate::new(200)
                .append_header("content-type", "text/event-stream")
                .append_header("x-codex-primary-used-percent", "91.5")
                .append_header("x-codex-primary-window-minutes", "300")
                .append_header("x-codex-primary-reset-at", "1777777777")
                .append_header("x-codex-limit-name", "Codex")
                .append_header("x-codex-promo-message", "raw upstream promo should not leak")
                .set_body_raw(sse_payload, "text/event-stream"),
        )
        .mount(&upstream)
        .await;

    let app = test_app(vec![test_account(upstream.uri(), "upstream-token")]).await;

    let response = app
        .oneshot(plain_api_request(
            "/v1/responses",
            Body::from("{\"stream\":true}"),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response
            .headers()
            .get("x-codex-primary-used-percent")
            .is_none(),
        "plain API clients should not receive upstream raw x-codex rate limit headers"
    );
}

#[tokio::test]
async fn codex_compat_stream_response_preserves_codex_rate_limit_headers() {
    let upstream = MockServer::start().await;
    let sse_payload = "event: response.output_text.delta\ndata: {\"delta\":\"hello\"}\n\n";

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(header("authorization", "Bearer upstream-token"))
        .and(header("openai-beta", OPENAI_BETA))
        .and(header("x-codex-turn-state", X_CODEX_TURN_STATE))
        .and(header("x-codex-turn-metadata", X_CODEX_TURN_METADATA))
        .and(header("x-codex-beta-features", X_CODEX_BETA_FEATURES))
        .respond_with(
            ResponseTemplate::new(200)
                .append_header("content-type", "text/event-stream")
                .append_header("x-codex-primary-used-percent", "91.5")
                .append_header("x-codex-primary-window-minutes", "300")
                .append_header("x-codex-primary-reset-at", "1777777777")
                .set_body_raw(sse_payload, "text/event-stream"),
        )
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
    assert_eq!(
        response
            .headers()
            .get("x-codex-primary-used-percent")
            .and_then(|value| value.to_str().ok()),
        Some("91.5"),
        "Codex-compatible clients should continue receiving x-codex rate limit headers"
    );
    assert!(
        response.headers().get("x-codex-promo-message").is_none(),
        "rewritten SSE headers should drop upstream raw promo payloads"
    );
}
