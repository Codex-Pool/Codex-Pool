use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{convert::Infallible, net::SocketAddr};

use async_trait::async_trait;
use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::routing::post;
use axum::Router;
use bytes::Bytes;
use codex_pool_core::api::UsageSummary;
use codex_pool_core::events::RequestLogEvent;
use codex_pool_core::model::{RoutingStrategy, UpstreamAccount, UpstreamMode};
use data_plane::app::{
    build_app_with_event_sink as dp_build_app_with_event_sink,
    build_app_with_event_sink_and_allowed_keys as dp_build_app_with_event_sink_and_allowed_keys,
};
use data_plane::config::DataPlaneConfig;
use data_plane::event::{EventSink, NoopEventSink};
use data_plane::router::RoundRobinRouter;
use http::Request;
use http::StatusCode;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tower::ServiceExt;
use uuid::Uuid;
use wiremock::matchers::{body_bytes, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support;

async fn build_app_with_event_sink(
    config: DataPlaneConfig,
    event_sink: Arc<dyn EventSink>,
) -> anyhow::Result<Router> {
    support::ensure_test_security_env().await;
    dp_build_app_with_event_sink(config, event_sink).await
}

async fn build_app_with_event_sink_and_allowed_keys(
    config: DataPlaneConfig,
    event_sink: Arc<dyn EventSink>,
    allowed_keys: Vec<String>,
) -> anyhow::Result<Router> {
    support::ensure_test_security_env().await;
    dp_build_app_with_event_sink_and_allowed_keys(config, event_sink, allowed_keys).await
}

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

async fn spawn_live_test_app(accounts: Vec<UpstreamAccount>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let cfg = DataPlaneConfig {
        listen_addr: addr,
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
    let app = build_app_with_event_sink(cfg, Arc::new(NoopEventSink))
        .await
        .expect("app should build");
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

async fn spawn_scripted_sse_upstream(chunks: Vec<&'static str>) -> String {
    async fn sse_handler(
        chunks: Arc<Vec<String>>,
    ) -> (
        StatusCode,
        [(axum::http::header::HeaderName, &'static str); 1],
        Body,
    ) {
        let (tx, rx) = mpsc::channel::<Result<Bytes, Infallible>>(8);
        tokio::spawn(async move {
            for chunk in chunks.iter() {
                if tx.send(Ok(Bytes::from(chunk.clone()))).await.is_err() {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });

        (
            StatusCode::OK,
            [(CONTENT_TYPE, "text/event-stream")],
            Body::from_stream(ReceiverStream::new(rx)),
        )
    }

    let chunks = Arc::new(chunks.into_iter().map(str::to_string).collect::<Vec<_>>());
    let app = Router::new().route(
        "/v1/responses",
        post({
            let chunks = chunks.clone();
            move || sse_handler(chunks.clone())
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

async fn spawn_previous_response_reset_http_upstream(
    route_path: &'static str,
    success_payload: Value,
) -> (String, Arc<Mutex<Vec<Value>>>) {
    async fn continuation_handler(
        body: Bytes,
        seen_bodies: Arc<Mutex<Vec<Value>>>,
        success_payload: Value,
    ) -> (
        StatusCode,
        [(axum::http::header::HeaderName, &'static str); 1],
        Body,
    ) {
        let value: Value = serde_json::from_slice(&body).unwrap_or_else(|_| json!({}));
        seen_bodies.lock().unwrap().push(value.clone());
        if value.get("previous_response_id").is_some() {
            (
                StatusCode::BAD_REQUEST,
                [(CONTENT_TYPE, "application/json")],
                Body::from(
                    json!({
                        "error": {
                            "code": "previous_response_not_found",
                            "message": "previous response was not found"
                        }
                    })
                    .to_string(),
                ),
            )
        } else {
            (
                StatusCode::OK,
                [(CONTENT_TYPE, "application/json")],
                Body::from(success_payload.to_string()),
            )
        }
    }

    let seen_bodies = Arc::new(Mutex::new(Vec::new()));
    let app = Router::new().route(
        route_path,
        post({
            let seen_bodies = seen_bodies.clone();
            let success_payload = success_payload.clone();
            move |body: Bytes| {
                let seen_bodies = seen_bodies.clone();
                let success_payload = success_payload.clone();
                async move { continuation_handler(body, seen_bodies, success_payload).await }
            }
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{}", addr), seen_bodies)
}

async fn spawn_storage_sensitive_continuation_upstream(
    route_path: &'static str,
) -> (String, Arc<Mutex<Vec<Value>>>) {
    async fn continuation_handler(
        body: Bytes,
        seen_bodies: Arc<Mutex<Vec<Value>>>,
    ) -> (
        StatusCode,
        [(axum::http::header::HeaderName, &'static str); 1],
        Body,
    ) {
        let value: Value = serde_json::from_slice(&body).unwrap_or_else(|_| json!({}));
        seen_bodies.lock().unwrap().push(value.clone());

        let previous_response_id = value
            .get("previous_response_id")
            .and_then(Value::as_str)
            .map(str::to_string);

        match previous_response_id.as_deref() {
            Some("resp_stored") => (
                StatusCode::OK,
                [(CONTENT_TYPE, "application/json")],
                Body::from(
                    json!({
                        "id": "resp_followup_ok",
                        "status": "completed",
                        "output": [{
                            "type": "message",
                            "role": "assistant",
                            "content": [{"type": "output_text", "text": "continued"}]
                        }]
                    })
                    .to_string(),
                ),
            ),
            Some(_) => (
                StatusCode::BAD_REQUEST,
                [(CONTENT_TYPE, "application/json")],
                Body::from(
                    json!({
                        "error": {
                            "code": "previous_response_not_found",
                            "message": "previous response was not found"
                        }
                    })
                    .to_string(),
                ),
            ),
            None => {
                let response_id = if value.get("store").and_then(Value::as_bool) == Some(false) {
                    "resp_unstored"
                } else {
                    "resp_stored"
                };
                (
                    StatusCode::OK,
                    [(CONTENT_TYPE, "application/json")],
                    Body::from(
                        json!({
                            "id": response_id,
                            "status": "completed",
                            "output": [{
                                "type": "message",
                                "role": "assistant",
                                "content": [{"type": "output_text", "text": "first-turn"}]
                            }]
                        })
                        .to_string(),
                    ),
                )
            }
        }
    }

    let seen_bodies = Arc::new(Mutex::new(Vec::new()));
    let app = Router::new().route(
        route_path,
        post({
            let seen_bodies = seen_bodies.clone();
            move |body: Bytes| {
                let seen_bodies = seen_bodies.clone();
                async move { continuation_handler(body, seen_bodies).await }
            }
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{}", addr), seen_bodies)
}

async fn spawn_background_delay_upstream(
    route_path: &'static str,
    delay: Duration,
    success_payload: Value,
) -> (String, Arc<AtomicUsize>) {
    async fn delayed_handler(
        body: Bytes,
        seen_total: Arc<AtomicUsize>,
        delay: Duration,
        success_payload: Value,
    ) -> (
        StatusCode,
        [(axum::http::header::HeaderName, &'static str); 1],
        Body,
    ) {
        let _value: Value = serde_json::from_slice(&body).unwrap_or_else(|_| json!({}));
        seen_total.fetch_add(1, Ordering::Relaxed);
        tokio::time::sleep(delay).await;
        (
            StatusCode::OK,
            [(CONTENT_TYPE, "application/json")],
            Body::from(success_payload.to_string()),
        )
    }

    let seen_total = Arc::new(AtomicUsize::new(0));
    let app = Router::new().route(
        route_path,
        post({
            let seen_total = seen_total.clone();
            let success_payload = success_payload.clone();
            move |body: Bytes| {
                let seen_total = seen_total.clone();
                let success_payload = success_payload.clone();
                async move { delayed_handler(body, seen_total, delay, success_payload).await }
            }
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{}", addr), seen_total)
}

async fn test_app_with_failover_wait_and_control_plane(
    accounts: Vec<UpstreamAccount>,
    request_failover_wait_ms: u64,
    control_plane_base_url: Option<String>,
) -> Router {
    test_app_with_failover_wait_and_control_plane_with_debug(
        accounts,
        request_failover_wait_ms,
        control_plane_base_url,
        false,
    )
    .await
}

async fn test_app_with_failover_wait_and_control_plane_with_debug(
    accounts: Vec<UpstreamAccount>,
    request_failover_wait_ms: u64,
    control_plane_base_url: Option<String>,
    enable_internal_debug_routes: bool,
) -> Router {
    test_app_with_failover_wait_and_control_plane_with_preauth(
        accounts,
        request_failover_wait_ms,
        control_plane_base_url,
        true,
        10_000,
        enable_internal_debug_routes,
    )
    .await
}

async fn test_app_with_failover_wait_and_control_plane_with_preauth(
    accounts: Vec<UpstreamAccount>,
    request_failover_wait_ms: u64,
    control_plane_base_url: Option<String>,
    billing_dynamic_preauth_enabled: bool,
    billing_preauth_unit_price_microcredits: i64,
    enable_internal_debug_routes: bool,
) -> Router {
    test_app_with_failover_wait_and_control_plane_with_preauth_limits(
        accounts,
        request_failover_wait_ms,
        control_plane_base_url,
        billing_dynamic_preauth_enabled,
        256,
        1.3,
        1_000,
        1_000_000_000_000,
        billing_preauth_unit_price_microcredits,
        enable_internal_debug_routes,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn test_app_with_failover_wait_and_control_plane_with_preauth_limits(
    accounts: Vec<UpstreamAccount>,
    request_failover_wait_ms: u64,
    control_plane_base_url: Option<String>,
    billing_dynamic_preauth_enabled: bool,
    billing_preauth_expected_output_tokens: i64,
    billing_preauth_safety_factor: f64,
    billing_preauth_min_microcredits: i64,
    billing_preauth_max_microcredits: i64,
    billing_preauth_unit_price_microcredits: i64,
    enable_internal_debug_routes: bool,
) -> Router {
    let auth_validate_url = control_plane_base_url
        .as_deref()
        .map(|base| format!("{}/internal/v1/auth/validate", base.trim_end_matches('/')));
    let cfg = DataPlaneConfig {
        listen_addr: "127.0.0.1:0".parse().unwrap(),
        routing_strategy: RoutingStrategy::RoundRobin,
        upstream_accounts: accounts,
        account_ejection_ttl_sec: 30,
        enable_request_failover: true,
        same_account_quick_retry_max: 1,
        request_failover_wait_ms,
        retry_poll_interval_ms: 100,
        sticky_prefer_non_conflicting: true,
        shared_routing_cache_enabled: true,
        enable_metered_stream_billing: true,
        billing_authorize_required_for_stream: true,
        stream_billing_reserve_microcredits: 2_000_000,
        billing_dynamic_preauth_enabled,
        billing_preauth_expected_output_tokens,
        billing_preauth_safety_factor,
        billing_preauth_min_microcredits,
        billing_preauth_max_microcredits,
        billing_preauth_unit_price_microcredits,
        stream_billing_drain_timeout_ms: 5_000,
        billing_capture_retry_max: 3,
        billing_capture_retry_backoff_ms: 200,
        redis_url: None,
        auth_validate_url,
        auth_validate_cache_ttl_sec: 30,
        auth_validate_negative_cache_ttl_sec: 5,
        auth_fail_open: false,
        enable_internal_debug_routes,
    };
    build_app_with_event_sink(cfg, Arc::new(NoopEventSink))
        .await
        .expect("app should build")
}

#[derive(Default)]
struct RecordingSink {
    events: Mutex<Vec<RequestLogEvent>>,
}

impl RecordingSink {
    fn events(&self) -> Vec<RequestLogEvent> {
        self.events.lock().unwrap().clone()
    }
}

#[async_trait]
impl EventSink for RecordingSink {
    async fn emit_request_log(&self, event: RequestLogEvent) {
        self.events.lock().unwrap().push(event);
    }
}

#[tokio::test]
async fn round_robin_router_cycles_accounts() {
    let a = test_account("http://upstream-a".to_string(), "token-a");
    let b = test_account("http://upstream-b".to_string(), "token-b");
    let router = RoundRobinRouter::new(vec![a.clone(), b.clone()]);

    assert_eq!(router.pick().unwrap().id, a.id);
    assert_eq!(router.pick().unwrap().id, b.id);
    assert_eq!(router.pick().unwrap().id, a.id);
}

#[tokio::test]
async fn forwards_headers_and_zstd_body_for_v1_responses() {
    let upstream = MockServer::start().await;
    let body = vec![0x28_u8, 0xb5, 0x2f, 0xfd, 0x00, 0x40, 0x78, 0x79, 0x7a];

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(header("authorization", "Bearer upstream-token"))
        .and(header("chatgpt-account-id", "acct_123"))
        .and(header("session_id", "session-abc"))
        .and(header("x-codex-turn-state", "turn-state-1"))
        .and(header("x-codex-turn-metadata", "meta-1"))
        .and(header("x-codex-beta-features", "responses_websockets"))
        .and(header("openai-beta", "responses_websockets=2026-02-04"))
        .and(header("x-openai-subagent", "review"))
        .and(header("content-encoding", "zstd"))
        .and(body_bytes(body.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .mount(&upstream)
        .await;

    let app = test_app(vec![test_account(upstream.uri(), "upstream-token")]).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer tenant-token")
                .header("session_id", "session-abc")
                .header("x-codex-turn-state", "turn-state-1")
                .header("x-codex-turn-metadata", "meta-1")
                .header("x-codex-beta-features", "responses_websockets")
                .header("openai-beta", "responses_websockets=2026-02-04")
                .header("x-openai-subagent", "review")
                .header("content-encoding", "zstd")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(payload["ok"], true);
}

#[tokio::test]
async fn maps_x_session_id_to_session_id_when_missing() {
    let upstream = MockServer::start().await;
    let sse_payload = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_x_session\",\"status\":\"in_progress\"}}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_x_session\",\"status\":\"completed\"}}\n\n",
        "data: [DONE]\n\n",
    );

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(header("authorization", "Bearer upstream-token"))
        .and(header("chatgpt-account-id", "acct_123"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(sse_payload, "text/event-stream"))
        .mount(&upstream)
        .await;

    let app = test_app(vec![test_account(upstream.uri(), "upstream-token")]).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer tenant-token")
                .header("x-session-id", "x-session-xyz")
                .body(Body::from(r#"{"model":"gpt-4.1-mini","stream":true}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(content_type.contains("text/event-stream"));

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_text = String::from_utf8_lossy(&body);
    assert!(body_text.contains("response.created"));
    assert!(body_text.contains("[DONE]"));

    let received = upstream.received_requests().await.unwrap();
    assert_eq!(received.len(), 1);
    let request = &received[0];
    assert_eq!(
        request
            .headers
            .get("x-session-id")
            .and_then(|value| value.to_str().ok()),
        Some("x-session-xyz")
    );
    assert_eq!(
        request
            .headers
            .get("session_id")
            .and_then(|value| value.to_str().ok()),
        Some("x-session-xyz")
    );
}

#[tokio::test]
async fn rejects_oversized_request_body() {
    let upstream = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .mount(&upstream)
        .await;

    let app = test_app(vec![test_account(upstream.uri(), "upstream-token")]).await;
    let oversized = vec![b'a'; 11 * 1024 * 1024];

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .body(Body::from(oversized))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn rejects_backend_api_codex_v1_responses_path() {
    let upstream = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/backend-api/codex/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"path_ok": true})))
        .mount(&upstream)
        .await;

    let app = test_app(vec![test_account(upstream.uri(), "upstream-token")]).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/backend-api/codex/v1/responses")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn rewrites_v1_responses_to_codex_responses_for_codex_base_profile() {
    let upstream = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/backend-api/codex/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"rewritten": true})))
        .mount(&upstream)
        .await;

    let codex_base = format!("{}/backend-api/codex", upstream.uri());
    let app = test_app(vec![test_account(codex_base, "upstream-token")]).await;

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

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(payload["rewritten"], true);
}

#[tokio::test]
async fn adapts_openai_non_stream_responses_request_for_codex_profile() {
    let upstream = MockServer::start().await;
    let sse_payload = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_compat\",\"status\":\"in_progress\"}}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_compat\",\"status\":\"completed\",\"usage\":{\"input_tokens\":3,\"output_tokens\":1},\"output\":[{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"OK\"}]}]}}\n\n",
        "data: [DONE]\n\n",
    );

    Mock::given(method("POST"))
        .and(path("/backend-api/codex/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(sse_payload, "text/event-stream"))
        .mount(&upstream)
        .await;

    let codex_base = format!("{}/backend-api/codex", upstream.uri());
    let app = test_app(vec![test_account(codex_base, "upstream-token")]).await;

    let request_body = json!({
        "model": "gpt-5.4",
        "input": "Reply with exactly OK.",
        "max_output_tokens": 16
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("content-type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("application/json")
    );
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(payload["id"], "resp_compat");
    assert_eq!(payload["status"], "completed");
    assert_eq!(payload["usage"]["input_tokens"], 3);
    assert_eq!(payload["output"][0]["content"][0]["text"], "OK");

    let requests = upstream.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let forwarded: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(forwarded["instructions"], "");
    assert_eq!(forwarded["store"], false);
    assert_eq!(forwarded["stream"], true);
    assert!(forwarded.get("max_output_tokens").is_none());
    assert_eq!(
        forwarded["input"],
        json!([{
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": "Reply with exactly OK."
            }]
        }])
    );
}

#[tokio::test]
async fn adapts_openai_streaming_responses_request_for_codex_profile() {
    let upstream = MockServer::start().await;
    let sse_payload = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_stream\",\"status\":\"in_progress\"}}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_stream\",\"status\":\"completed\"}}\n\n",
        "data: [DONE]\n\n",
    );

    Mock::given(method("POST"))
        .and(path("/backend-api/codex/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(sse_payload, "text/event-stream"))
        .mount(&upstream)
        .await;

    let codex_base = format!("{}/backend-api/codex", upstream.uri());
    let app = test_app(vec![test_account(codex_base, "upstream-token")]).await;

    let request_body = json!({
        "model": "gpt-5.4",
        "stream": true,
        "input": "Reply with exactly OK.",
        "max_output_tokens": 16
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("content-type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("text/event-stream")
    );
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = std::str::from_utf8(&bytes).unwrap();
    assert!(body.contains("response.completed"));

    let requests = upstream.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let forwarded: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(forwarded["instructions"], "");
    assert_eq!(forwarded["store"], false);
    assert_eq!(forwarded["stream"], true);
    assert!(forwarded.get("max_output_tokens").is_none());
    assert_eq!(
        forwarded["input"],
        json!([{
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": "Reply with exactly OK."
            }]
        }])
    );
}

#[tokio::test]
async fn rewrites_v1_responses_compact_to_codex_responses_compact_for_codex_base_profile() {
    let upstream = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/backend-api/codex/responses/compact"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "output": [
                {
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "output_text", "text": "compacted"}]
                }
            ]
        })))
        .mount(&upstream)
        .await;

    let codex_base = format!("{}/backend-api/codex", upstream.uri());
    let app = test_app(vec![test_account(codex_base, "upstream-token")]).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses/compact")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "model": "gpt-5.4",
                        "input": "compress this history",
                        "max_output_tokens": 16
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(payload["output"][0]["type"], "message");

    let requests = upstream.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let forwarded: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(forwarded["model"], "gpt-5.4");
    assert_eq!(forwarded["instructions"], "");
    assert!(forwarded.get("store").is_none());
    assert!(forwarded.get("stream").is_none());
    assert!(forwarded.get("max_output_tokens").is_none());
    assert_eq!(
        forwarded["input"],
        json!([{
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": "compress this history"
            }]
        }])
    );
}

#[tokio::test]
async fn keeps_previous_response_requests_continuation_aware_for_codex_profile() {
    let upstream = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/backend-api/codex/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_keep_prev",
            "status": "completed",
            "output": []
        })))
        .mount(&upstream)
        .await;

    let codex_base = format!("{}/backend-api/codex", upstream.uri());
    let app = test_app(vec![test_account(codex_base, "upstream-token")]).await;

    let request_body = json!({
        "model": "gpt-5.4",
        "previous_response_id": "resp_prev_123",
        "input": "Continue from before."
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("content-type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let requests = upstream.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let forwarded: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(forwarded["previous_response_id"], "resp_prev_123");
    assert!(forwarded.get("store").is_none());
}

#[tokio::test]
async fn recovers_http_responses_from_previous_response_not_found_without_reusing_stale_id() {
    let (upstream_base, seen_bodies) = spawn_previous_response_reset_http_upstream(
        "/backend-api/codex/responses",
        json!({
            "id": "resp_recovered_http",
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "recovered"}]
            }]
        }),
    )
    .await;

    let codex_base = format!("{upstream_base}/backend-api/codex");
    let app = test_app(vec![test_account(codex_base, "upstream-token")]).await;

    let request_body = json!({
        "model": "gpt-5.4",
        "previous_response_id": "resp_prev_123",
        "input": "Continue from before."
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("content-type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(payload["id"], "resp_recovered_http");

    let seen = seen_bodies.lock().unwrap().clone();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0]["previous_response_id"], "resp_prev_123");
    assert!(seen[1].get("previous_response_id").is_none());
}

#[tokio::test]
async fn preserves_first_turn_storage_so_previous_response_id_continues_without_retry_rewrite() {
    let (upstream_base, seen_bodies) =
        spawn_storage_sensitive_continuation_upstream("/backend-api/codex/responses").await;
    let codex_base = format!("{upstream_base}/backend-api/codex");
    let app = test_app(vec![test_account(codex_base, "upstream-token")]).await;

    let first_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "model": "gpt-5.4",
                        "input": "Remember the token BLUEBIRD."
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(first_response.status(), StatusCode::OK);
    let first_bytes = first_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let first_payload: Value = serde_json::from_slice(&first_bytes).unwrap();
    assert_eq!(first_payload["id"], "resp_stored");

    let second_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "model": "gpt-5.4",
                        "previous_response_id": first_payload["id"].as_str().unwrap(),
                        "input": "What token did I ask you to remember?"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(second_response.status(), StatusCode::OK);
    let second_bytes = second_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let second_payload: Value = serde_json::from_slice(&second_bytes).unwrap();
    assert_eq!(second_payload["id"], "resp_followup_ok");

    let seen = seen_bodies.lock().unwrap().clone();
    assert_eq!(seen.len(), 2);
    assert!(seen[0].get("store").is_none());
    assert_eq!(seen[1]["previous_response_id"], "resp_stored");
}

#[tokio::test]
async fn recovers_compact_responses_from_previous_response_not_found_without_reusing_stale_id() {
    let (upstream_base, seen_bodies) = spawn_previous_response_reset_http_upstream(
        "/backend-api/codex/responses/compact",
        json!({
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "compacted-recovered"}]
            }]
        }),
    )
    .await;

    let codex_base = format!("{upstream_base}/backend-api/codex");
    let app = test_app(vec![test_account(codex_base, "upstream-token")]).await;

    let request_body = json!({
        "model": "gpt-5.4",
        "previous_response_id": "resp_prev_456",
        "input": "compress this history"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses/compact")
                .header("content-type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        payload["output"][0]["content"][0]["text"],
        "compacted-recovered"
    );

    let seen = seen_bodies.lock().unwrap().clone();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0]["previous_response_id"], "resp_prev_456");
    assert!(seen[1].get("previous_response_id").is_none());
}

#[tokio::test]
async fn retrieves_stored_http_response_by_response_id() {
    let upstream = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/backend-api/codex/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_saved_http",
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "saved"}]
            }]
        })))
        .mount(&upstream)
        .await;

    let codex_base = format!("{}/backend-api/codex", upstream.uri());
    let base_url = spawn_live_test_app(vec![test_account(codex_base, "upstream-token")]).await;
    let client = reqwest::Client::new();

    let create = client
        .post(format!("{base_url}/v1/responses"))
        .header(CONTENT_TYPE.as_str(), "application/json")
        .body(
            json!({
                "model": "gpt-5.4",
                "input": "Store this response."
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(create.status(), reqwest::StatusCode::OK);
    let created: Value = create.json().await.unwrap();
    assert_eq!(created["id"], "resp_saved_http");

    let retrieve = client
        .get(format!(
            "{base_url}/v1/responses/{}",
            created["id"].as_str().unwrap()
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(retrieve.status(), reqwest::StatusCode::OK);
    let retrieved: Value = retrieve.json().await.unwrap();
    assert_eq!(retrieved["id"], "resp_saved_http");
    assert_eq!(
        retrieved["output"][0]["content"][0]["text"],
        Value::String("saved".to_string())
    );
}

#[tokio::test]
async fn does_not_retrieve_http_responses_when_store_is_false() {
    let upstream = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/backend-api/codex/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_not_stored",
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "volatile"}]
            }]
        })))
        .mount(&upstream)
        .await;

    let codex_base = format!("{}/backend-api/codex", upstream.uri());
    let base_url = spawn_live_test_app(vec![test_account(codex_base, "upstream-token")]).await;
    let client = reqwest::Client::new();

    let create = client
        .post(format!("{base_url}/v1/responses"))
        .header(CONTENT_TYPE.as_str(), "application/json")
        .body(
            json!({
                "model": "gpt-5.4",
                "store": false,
                "input": "Do not store this response."
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(create.status(), reqwest::StatusCode::OK);
    let created: Value = create.json().await.unwrap();
    assert_eq!(created["id"], "resp_not_stored");

    let retrieve = client
        .get(format!("{base_url}/v1/responses/resp_not_stored"))
        .send()
        .await
        .unwrap();
    assert_eq!(retrieve.status(), reqwest::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn completes_background_responses_and_makes_them_retrievable() {
    let (upstream_base, _) = spawn_background_delay_upstream(
        "/backend-api/codex/responses",
        Duration::from_millis(80),
        json!({
            "id": "resp_background_done",
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "background-ok"}]
            }]
        }),
    )
    .await;
    let codex_base = format!("{upstream_base}/backend-api/codex");
    let base_url = spawn_live_test_app(vec![test_account(codex_base, "upstream-token")]).await;
    let client = reqwest::Client::new();

    let queued = client
        .post(format!("{base_url}/v1/responses"))
        .header(CONTENT_TYPE.as_str(), "application/json")
        .body(
            json!({
                "model": "gpt-5.4",
                "background": true,
                "input": "run in background"
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(queued.status(), reqwest::StatusCode::ACCEPTED);
    let queued_payload: Value = queued.json().await.unwrap();
    let response_id = queued_payload["id"].as_str().unwrap().to_string();

    let mut final_payload = queued_payload.clone();
    for _ in 0..20 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let retrieve = client
            .get(format!("{base_url}/v1/responses/{response_id}"))
            .send()
            .await
            .unwrap();
        assert_eq!(retrieve.status(), reqwest::StatusCode::OK);
        let payload: Value = retrieve.json().await.unwrap();
        if payload["status"] == "completed" {
            final_payload = payload;
            break;
        }
        final_payload = payload;
    }

    assert_eq!(final_payload["status"], "completed");
    assert_eq!(
        final_payload["output"][0]["content"][0]["text"],
        Value::String("background-ok".to_string())
    );
}

#[tokio::test]
async fn cancels_background_responses_before_completion() {
    let (upstream_base, seen_total) = spawn_background_delay_upstream(
        "/backend-api/codex/responses",
        Duration::from_millis(250),
        json!({
            "id": "resp_background_late",
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "too-late"}]
            }]
        }),
    )
    .await;
    let codex_base = format!("{upstream_base}/backend-api/codex");
    let base_url = spawn_live_test_app(vec![test_account(codex_base, "upstream-token")]).await;
    let client = reqwest::Client::new();

    let queued = client
        .post(format!("{base_url}/v1/responses"))
        .header(CONTENT_TYPE.as_str(), "application/json")
        .body(
            json!({
                "model": "gpt-5.4",
                "background": true,
                "input": "cancel me"
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(queued.status(), reqwest::StatusCode::ACCEPTED);
    let queued_payload: Value = queued.json().await.unwrap();
    let response_id = queued_payload["id"].as_str().unwrap().to_string();

    let cancelled = client
        .post(format!("{base_url}/v1/responses/{response_id}/cancel"))
        .send()
        .await
        .unwrap();
    assert_eq!(cancelled.status(), reqwest::StatusCode::OK);
    let cancelled_payload: Value = cancelled.json().await.unwrap();
    assert_eq!(cancelled_payload["status"], "cancelled");

    tokio::time::sleep(Duration::from_millis(350)).await;
    let retrieve = client
        .get(format!("{base_url}/v1/responses/{response_id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(retrieve.status(), reqwest::StatusCode::OK);
    let retrieved: Value = retrieve.json().await.unwrap();
    assert_eq!(retrieved["status"], "cancelled");
    assert_eq!(seen_total.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn conversation_reuses_last_response_id_without_explicit_previous_response_id() {
    let (upstream_base, seen_bodies) =
        spawn_storage_sensitive_continuation_upstream("/backend-api/codex/responses").await;
    let codex_base = format!("{upstream_base}/backend-api/codex");
    let app = test_app(vec![test_account(codex_base, "upstream-token")]).await;

    let first_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "model": "gpt-5.4",
                        "conversation": "conv_bluebird",
                        "input": "Remember BLUEBIRD."
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(first_response.status(), StatusCode::OK);
    let first_payload: Value = serde_json::from_slice(
        &first_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes(),
    )
    .unwrap();
    assert_eq!(first_payload["id"], "resp_stored");

    let second_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "model": "gpt-5.4",
                        "conversation": "conv_bluebird",
                        "input": "What did I ask you to remember?"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(second_response.status(), StatusCode::OK);
    let second_payload: Value = serde_json::from_slice(
        &second_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes(),
    )
    .unwrap();
    assert_eq!(second_payload["id"], "resp_followup_ok");

    let seen = seen_bodies.lock().unwrap().clone();
    assert_eq!(seen.len(), 2);
    assert!(seen[0].get("conversation").is_none());
    assert!(seen[1].get("conversation").is_none());
    assert_eq!(seen[1]["previous_response_id"], "resp_stored");
}

#[tokio::test]
async fn counts_input_tokens_for_responses_payloads() {
    let app = test_app(Vec::new()).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses/input_tokens")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "model": "gpt-5.4",
                        "input": [{
                            "role": "user",
                            "content": [
                                {"type": "input_text", "text": "Count these prompt tokens."}
                            ]
                        }]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let payload: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(payload["object"], "response.input_tokens");
    assert!(payload["input_tokens"].as_i64().unwrap_or_default() > 0);
}

#[tokio::test]
async fn lists_input_items_for_stored_response() {
    let upstream = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/backend-api/codex/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_with_inputs",
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "stored"}]
            }]
        })))
        .mount(&upstream)
        .await;

    let codex_base = format!("{}/backend-api/codex", upstream.uri());
    let base_url = spawn_live_test_app(vec![test_account(codex_base, "upstream-token")]).await;
    let client = reqwest::Client::new();

    let create = client
        .post(format!("{base_url}/v1/responses"))
        .header(CONTENT_TYPE.as_str(), "application/json")
        .body(
            json!({
                "model": "gpt-5.4",
                "input": [{
                    "role": "user",
                    "content": [
                        {"type": "input_text", "text": "first"},
                        {"type": "input_text", "text": "second"}
                    ]
                }]
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(create.status(), reqwest::StatusCode::OK);

    let listed = client
        .get(format!(
            "{base_url}/v1/responses/resp_with_inputs/input_items?limit=1&order=asc"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(listed.status(), reqwest::StatusCode::OK);
    let payload: Value = listed.json().await.unwrap();
    assert_eq!(payload["object"], "list");
    assert_eq!(payload["data"].as_array().map(Vec::len), Some(1));
    assert_eq!(payload["has_more"], Value::Bool(false));
    assert_eq!(payload["data"][0]["type"], "message");
    assert_eq!(payload["data"][0]["role"], "user");
}

#[tokio::test]
async fn rejects_background_responses_when_store_is_false() {
    let upstream = MockServer::start().await;
    let codex_base = format!("{}/backend-api/codex", upstream.uri());
    let base_url = spawn_live_test_app(vec![test_account(codex_base, "upstream-token")]).await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{base_url}/v1/responses"))
        .header(CONTENT_TYPE.as_str(), "application/json")
        .body(
            json!({
                "model": "gpt-5.4",
                "background": true,
                "store": false,
                "input": "must fail"
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
    let payload: Value = response.json().await.unwrap();
    assert_eq!(payload["error"]["code"], "background_requires_store");
}

#[tokio::test]
async fn does_not_cancel_non_background_responses() {
    let upstream = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/backend-api/codex/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_sync_only",
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "sync"}]
            }]
        })))
        .mount(&upstream)
        .await;

    let codex_base = format!("{}/backend-api/codex", upstream.uri());
    let base_url = spawn_live_test_app(vec![test_account(codex_base, "upstream-token")]).await;
    let client = reqwest::Client::new();

    let create = client
        .post(format!("{base_url}/v1/responses"))
        .header(CONTENT_TYPE.as_str(), "application/json")
        .body(
            json!({
                "model": "gpt-5.4",
                "input": "sync only"
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(create.status(), reqwest::StatusCode::OK);

    let cancel = client
        .post(format!("{base_url}/v1/responses/resp_sync_only/cancel"))
        .send()
        .await
        .unwrap();
    assert_eq!(cancel.status(), reqwest::StatusCode::BAD_REQUEST);
    let payload: Value = cancel.json().await.unwrap();
    assert_eq!(payload["error"]["code"], "response_not_cancellable");
}

#[tokio::test]
async fn streams_background_responses_and_supports_starting_after_resume() {
    let sse_payload = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_bg_stream\",\"status\":\"in_progress\"}}\n\n",
        "event: response.output_text.delta\n",
        "data: {\"type\":\"response.output_text.delta\",\"delta\":\"hello\"}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_bg_stream\",\"status\":\"completed\",\"output\":[{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"hello\"}]}]}}\n\n"
    );
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/backend-api/codex/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(sse_payload, "text/event-stream"))
        .mount(&upstream)
        .await;

    let codex_base = format!("{}/backend-api/codex", upstream.uri());
    let base_url = spawn_live_test_app(vec![test_account(codex_base, "upstream-token")]).await;
    let client = reqwest::Client::new();

    let create = client
        .post(format!("{base_url}/v1/responses"))
        .header(CONTENT_TYPE.as_str(), "application/json")
        .body(
            json!({
                "model": "gpt-5.4",
                "background": true,
                "stream": true,
                "input": "stream me"
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(create.status(), reqwest::StatusCode::OK);
    let create_content_type = create
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(create_content_type.contains("text/event-stream"));
    let create_body = create.bytes().await.unwrap();
    let create_text = String::from_utf8_lossy(&create_body);
    assert!(create_text.contains("\"sequence_number\":1"));
    assert!(create_text.contains("\"sequence_number\":2"));
    assert!(create_text.contains("\"sequence_number\":3"));
    let response_id = create_text
        .split("\"response\":{\"id\":\"")
        .nth(1)
        .and_then(|tail| tail.split('"').next())
        .expect("stream should expose a response id")
        .to_string();

    let resumed = client
        .get(format!(
            "{base_url}/v1/responses/{response_id}?stream=true&starting_after=1"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resumed.status(), reqwest::StatusCode::OK);
    let resume_content_type = resumed
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(resume_content_type.contains("text/event-stream"));
    let resumed_body = resumed.bytes().await.unwrap();
    let resumed_text = String::from_utf8_lossy(&resumed_body);
    assert!(!resumed_text.contains("\"sequence_number\":1"));
    assert!(resumed_text.contains("\"sequence_number\":2"));
    assert!(resumed_text.contains("\"sequence_number\":3"));
}

#[tokio::test]
async fn passes_input_image_items_through_for_codex_profile() {
    let upstream = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/backend-api/codex/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_image_ok",
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "saw-image"}]
            }]
        })))
        .mount(&upstream)
        .await;

    let codex_base = format!("{}/backend-api/codex", upstream.uri());
    let app = test_app(vec![test_account(codex_base, "upstream-token")]).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "model": "gpt-5.4",
                        "input": [{
                            "role": "user",
                            "content": [
                                {"type": "input_text", "text": "Describe this image."},
                                {
                                    "type": "input_image",
                                    "image_url": "https://example.com/cat.png"
                                }
                            ]
                        }]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let requests = upstream.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let forwarded: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(
        forwarded["input"][0]["content"][1],
        json!({
            "type": "input_image",
            "image_url": "https://example.com/cat.png"
        })
    );
}

#[tokio::test]
async fn rewrites_v1_memories_trace_summarize_to_codex_memories_trace_summarize_for_codex_base_profile(
) {
    let upstream = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/backend-api/codex/memories/trace_summarize"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "output": [
                {
                    "trace_summary": "raw summary",
                    "memory_summary": "memory summary"
                }
            ]
        })))
        .mount(&upstream)
        .await;

    let codex_base = format!("{}/backend-api/codex", upstream.uri());
    let app = test_app(vec![test_account(codex_base, "upstream-token")]).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/memories/trace_summarize")
                .body(Body::from("{\"model\":\"gpt-test\",\"traces\":[]}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(payload["output"][0]["memory_summary"], "memory summary");
}

#[tokio::test]
async fn rewrites_v1_models_to_codex_models_and_appends_client_version() {
    let upstream = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/backend-api/codex/models"))
        .and(query_param("a", "1"))
        .and(query_param("client_version", "0.1.0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"models": []})))
        .mount(&upstream)
        .await;

    let codex_base = format!("{}/backend-api/codex", upstream.uri());
    let app = test_app(vec![test_account(codex_base, "upstream-token")]).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/models?a=1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(payload["models"], json!([]));
}

#[tokio::test]
async fn rewrites_v1_models_to_codex_models_and_uses_version_header_when_query_missing() {
    let upstream = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/backend-api/codex/models"))
        .and(query_param("a", "1"))
        .and(query_param("client_version", "2026.03.12"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"models": [{"id": "o3"}]})))
        .mount(&upstream)
        .await;

    let codex_base = format!("{}/backend-api/codex", upstream.uri());
    let app = test_app(vec![test_account(codex_base, "upstream-token")]).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/models?a=1")
                .header("version", "2026.03.12")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(payload["models"][0]["id"], "o3");
}

#[tokio::test]
async fn codex_models_passthrough_preserves_raw_body_headers_and_local_304_cache() {
    let upstream = MockServer::start().await;
    let raw_body = r#"{"models":[{"id":"o3","visibility":"list"}],"meta":{"source":"upstream"}}"#;

    Mock::given(method("GET"))
        .and(path("/backend-api/codex/models"))
        .and(query_param("client_version", "0.1.0"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/vnd.openai.codex-models+json")
                .insert_header("etag", "\"models-upstream-etag\"")
                .set_body_raw(raw_body, "application/vnd.openai.codex-models+json"),
        )
        .mount(&upstream)
        .await;

    let codex_base = format!("{}/backend-api/codex", upstream.uri());
    let app = test_app(vec![test_account(codex_base, "upstream-token")]).await;

    let first_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/models")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(first_response.status(), StatusCode::OK);
    assert_eq!(
        first_response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("application/vnd.openai.codex-models+json")
    );
    assert_eq!(
        first_response
            .headers()
            .get("etag")
            .and_then(|value| value.to_str().ok()),
        Some("\"models-upstream-etag\"")
    );
    let first_body = first_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    assert_eq!(std::str::from_utf8(&first_body).unwrap(), raw_body);

    let cached_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/models")
                .header("if-none-match", "\"models-upstream-etag\"")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(cached_response.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(
        cached_response
            .headers()
            .get("etag")
            .and_then(|value| value.to_str().ok()),
        Some("\"models-upstream-etag\"")
    );
    let cached_body = cached_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    assert!(cached_body.is_empty());
}

#[tokio::test]
async fn codex_models_cache_is_scoped_by_effective_client_version() {
    let upstream = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/backend-api/codex/models"))
        .and(query_param("client_version", "2026.03.12"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("etag", "\"models-v1\"")
                .set_body_json(json!({"models": [{"id": "o3"}]})),
        )
        .mount(&upstream)
        .await;

    Mock::given(method("GET"))
        .and(path("/backend-api/codex/models"))
        .and(query_param("client_version", "2026.03.13"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("etag", "\"models-v2\"")
                .set_body_json(json!({"models": [{"id": "o4-mini"}]})),
        )
        .mount(&upstream)
        .await;

    let codex_base = format!("{}/backend-api/codex", upstream.uri());
    let app = test_app(vec![test_account(codex_base, "upstream-token")]).await;

    let first_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/models")
                .header("version", "2026.03.12")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first_response.status(), StatusCode::OK);
    let first_body = first_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let first_payload: Value = serde_json::from_slice(&first_body).unwrap();
    assert_eq!(first_payload["models"][0]["id"], "o3");

    let second_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/models")
                .header("version", "2026.03.13")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second_response.status(), StatusCode::OK);
    let second_body = second_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let second_payload: Value = serde_json::from_slice(&second_body).unwrap();
    assert_eq!(second_payload["models"][0]["id"], "o4-mini");
}

#[tokio::test]
async fn keeps_session_sticky_for_session_and_conversation_id_headers() {
    let upstream_a = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"account": "a"})))
        .mount(&upstream_a)
        .await;

    let upstream_b = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"account": "b"})))
        .mount(&upstream_b)
        .await;

    let app = test_app(vec![
        test_account(upstream_a.uri(), "token-a"),
        test_account(upstream_b.uri(), "token-b"),
    ])
    .await;

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("session_id", "session-1")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    let first_body = first.into_body().collect().await.unwrap().to_bytes();
    let first_payload: Value = serde_json::from_slice(&first_body).unwrap();
    assert_eq!(first_payload["account"], "a");

    let second = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("session_id", "session-1")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    let second_body = second.into_body().collect().await.unwrap().to_bytes();
    let second_payload: Value = serde_json::from_slice(&second_body).unwrap();
    assert_eq!(second_payload["account"], "a");

    let third = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("conversation_id", "conversation-1")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    let third_body = third.into_body().collect().await.unwrap().to_bytes();
    let third_payload: Value = serde_json::from_slice(&third_body).unwrap();
    assert_eq!(third_payload["account"], "b");

    let fourth = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("conversation_id", "conversation-1")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    let fourth_body = fourth.into_body().collect().await.unwrap().to_bytes();
    let fourth_payload: Value = serde_json::from_slice(&fourth_body).unwrap();
    assert_eq!(fourth_payload["account"], "b");
}

#[tokio::test]
async fn forwards_openai_beta_and_subagent_headers() {
    let upstream = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/backend-api/codex/responses"))
        .and(header("openai-beta", "responses_websockets=2026-02-04"))
        .and(header("x-openai-subagent", "review"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .mount(&upstream)
        .await;

    let app = test_app(vec![test_account(upstream.uri(), "upstream-token")]).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/backend-api/codex/responses")
                .header("openai-beta", "responses_websockets=2026-02-04")
                .header("x-openai-subagent", "review")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(payload["ok"], true);
}

#[tokio::test]
async fn forwards_sse_stream_response_headers_and_body() {
    let upstream = MockServer::start().await;
    let sse_payload = "data: {\"type\":\"response.output_text.delta\",\"delta\":\"hello\"}\n\n";

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(sse_payload, "text/event-stream"))
        .mount(&upstream)
        .await;

    let app = test_app(vec![test_account(upstream.uri(), "upstream-token")]).await;

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

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    assert!(content_type.contains("text/event-stream"));

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(bytes.as_ref(), sse_payload.as_bytes());
}

#[tokio::test]
async fn stream_request_without_sse_content_type_still_uses_stream_billing_path() {
    let upstream = MockServer::start().await;
    let sse_payload = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"response\":{\"usage\":null}}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"usage\":{\"input_tokens\":14,\"output_tokens\":5}}}\n\n",
    );

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(sse_payload, "text/plain"))
        .mount(&upstream)
        .await;

    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": Uuid::new_v4(),
            "api_key_id": Uuid::new_v4(),
            "enabled": true,
            "tenant_status": "active",
            "balance_microcredits": 1_000_000,
            "cache_ttl_sec": 30
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/authorize"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorization_id": Uuid::new_v4(),
            "status": "authorized",
            "reserved_microcredits": 2_000_000
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/capture"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "captured"
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/release"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "released"
        })))
        .mount(&control_plane)
        .await;

    let app = test_app_with_failover_wait_and_control_plane(
        vec![test_account(upstream.uri(), "upstream-token")],
        2_000,
        Some(control_plane.uri()),
    )
    .await;

    let request_body = json!({
        "model": "gpt-5.3-codex",
        "stream": true,
        "instructions": "You are helpful",
        "input": [{
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": "ping"
            }]
        }]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer cp_identity")
                .header("content-type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = std::str::from_utf8(&bytes).unwrap();
    assert!(body.contains("event: response.completed"));
    assert!(!body.contains("billing_usage_missing"));
}

#[tokio::test]
async fn stream_request_with_plain_json_line_still_captures_billing_usage() {
    let upstream = MockServer::start().await;
    let payload = "{\"id\":\"resp_123\",\"usage\":{\"input_tokens\":9,\"output_tokens\":2}}\n";

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(payload, "text/plain"))
        .mount(&upstream)
        .await;

    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": Uuid::new_v4(),
            "api_key_id": Uuid::new_v4(),
            "enabled": true,
            "tenant_status": "active",
            "balance_microcredits": 1_000_000,
            "cache_ttl_sec": 30
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/authorize"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorization_id": Uuid::new_v4(),
            "status": "authorized",
            "reserved_microcredits": 2_000_000
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/capture"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "captured"
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/release"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "released"
        })))
        .mount(&control_plane)
        .await;

    let app = test_app_with_failover_wait_and_control_plane(
        vec![test_account(upstream.uri(), "upstream-token")],
        2_000,
        Some(control_plane.uri()),
    )
    .await;

    let request_body = json!({
        "model": "gpt-5.3-codex",
        "stream": true,
        "instructions": "You are helpful",
        "input": [{
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": "ping"
            }]
        }]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer cp_identity")
                .header("content-type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(bytes.as_ref(), payload.as_bytes());

    let mut capture_payload: Option<Value> = None;
    let mut last_paths: Vec<String> = Vec::new();
    for _ in 0..20 {
        let requests = control_plane.received_requests().await.unwrap();
        last_paths = requests
            .iter()
            .map(|request| request.url.path().to_string())
            .collect();
        if let Some(capture) = requests
            .iter()
            .find(|request| request.url.path() == "/internal/v1/billing/capture")
        {
            capture_payload = Some(serde_json::from_slice(&capture.body).unwrap());
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let capture_payload = capture_payload
        .unwrap_or_else(|| panic!("capture request should be sent, paths={last_paths:?}"));
    assert_eq!(capture_payload["input_tokens"], 9);
    assert_eq!(capture_payload["output_tokens"], 2);
}

#[tokio::test]
async fn stream_success_reports_seen_ok_to_control_plane() {
    let upstream = MockServer::start().await;
    let sse_payload = concat!(
        "event: response.output_text.delta\n",
        "data: {\"type\":\"response.output_text.delta\",\"delta\":\"OK\"}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"usage\":{\"input_tokens\":9,\"cached_input_tokens\":0,\"output_tokens\":2,\"reasoning_tokens\":0}}}\n\n",
    );

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(sse_payload, "text/event-stream"))
        .mount(&upstream)
        .await;

    let control_plane = MockServer::start().await;
    let account = test_account(upstream.uri(), "upstream-token");
    let seen_ok_path = format!(
        "/internal/v1/upstream-accounts/{}/health/seen-ok",
        account.id
    );
    let model_seen_ok_path = format!(
        "/internal/v1/upstream-accounts/{}/models/seen-ok",
        account.id
    );

    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": Uuid::new_v4(),
            "api_key_id": Uuid::new_v4(),
            "enabled": true,
            "tenant_status": "active",
            "balance_microcredits": 1_000_000,
            "cache_ttl_sec": 30
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/authorize"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorization_id": Uuid::new_v4(),
            "status": "authorized",
            "reserved_microcredits": 2_000_000
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/capture"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "captured"
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path(seen_ok_path.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "accepted": true,
            "account_id": account.id,
            "seen_ok_at": chrono::Utc::now(),
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path(model_seen_ok_path.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "accepted": true,
            "account_id": account.id,
            "model": "gpt-5.4",
            "seen_ok_at": chrono::Utc::now(),
        })))
        .mount(&control_plane)
        .await;

    let app = test_app_with_failover_wait_and_control_plane(
        vec![account.clone()],
        2_000,
        Some(control_plane.uri()),
    )
    .await;

    let request_body = json!({
        "model": "gpt-5.4",
        "stream": true,
        "instructions": "You are helpful",
        "input": [{
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": "ping"
            }]
        }]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer cp_identity")
                .header("content-type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(bytes.as_ref(), sse_payload.as_bytes());

    let mut seen_ok_count = 0;
    let mut model_seen_ok_count = 0;
    let mut last_paths = Vec::new();
    for _ in 0..100 {
        let requests = control_plane.received_requests().await.unwrap();
        last_paths = requests
            .iter()
            .map(|request| request.url.path().to_string())
            .collect();
        seen_ok_count = requests
            .iter()
            .filter(|request| request.url.path() == seen_ok_path)
            .count();
        model_seen_ok_count = requests
            .iter()
            .filter(|request| request.url.path() == model_seen_ok_path)
            .count();
        if seen_ok_count >= 1 && model_seen_ok_count >= 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    assert_eq!(seen_ok_count, 1, "paths={last_paths:?}");
    assert_eq!(model_seen_ok_count, 1, "paths={last_paths:?}");
}

#[tokio::test]
async fn stream_request_without_usage_estimates_tokens_for_billing_capture() {
    let upstream = MockServer::start().await;
    let sse_payload = concat!(
        "event: response.output_text.delta\n",
        "data: {\"type\":\"response.output_text.delta\",\"delta\":\"hello world\"}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"usage\":null}}\n\n",
    );

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(sse_payload, "text/event-stream"))
        .mount(&upstream)
        .await;

    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": Uuid::new_v4(),
            "api_key_id": Uuid::new_v4(),
            "enabled": true,
            "tenant_status": "active",
            "balance_microcredits": 1_000_000,
            "cache_ttl_sec": 30
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/authorize"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorization_id": Uuid::new_v4(),
            "status": "authorized",
            "reserved_microcredits": 2_000_000
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/capture"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "captured"
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/release"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "released"
        })))
        .mount(&control_plane)
        .await;

    let app = test_app_with_failover_wait_and_control_plane(
        vec![test_account(upstream.uri(), "upstream-token")],
        2_000,
        Some(control_plane.uri()),
    )
    .await;

    let request_body = json!({
        "model": "gpt-5.3-codex",
        "stream": true,
        "instructions": "You are helpful",
        "input": [{
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": "ping"
            }]
        }]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer cp_identity")
                .header("content-type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = std::str::from_utf8(&bytes).unwrap();
    assert!(body.contains("response.output_text.delta"));

    let mut capture_payload: Option<Value> = None;
    let mut last_paths: Vec<String> = Vec::new();
    for _ in 0..20 {
        let requests = control_plane.received_requests().await.unwrap();
        last_paths = requests
            .iter()
            .map(|request| request.url.path().to_string())
            .collect();
        if let Some(capture) = requests
            .iter()
            .find(|request| request.url.path() == "/internal/v1/billing/capture")
        {
            capture_payload = Some(serde_json::from_slice(&capture.body).unwrap());
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let capture_payload = capture_payload
        .unwrap_or_else(|| panic!("capture request should be sent, paths={last_paths:?}"));
    assert_eq!(capture_payload["input_tokens"], 5);
    assert_eq!(capture_payload["output_tokens"], 3);
}

#[tokio::test]
async fn dynamic_preauth_reserve_is_used_for_authorize_payload() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_dynamic_1",
            "usage": {"input_tokens": 11, "output_tokens": 7}
        })))
        .mount(&upstream)
        .await;

    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": Uuid::new_v4(),
            "api_key_id": Uuid::new_v4(),
            "enabled": true,
            "tenant_status": "active",
            "balance_microcredits": 1_000_000,
            "cache_ttl_sec": 30
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/authorize"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorization_id": Uuid::new_v4(),
            "status": "authorized",
            "reserved_microcredits": 1_000
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/capture"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "captured"
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/release"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "released"
        })))
        .mount(&control_plane)
        .await;

    let app = test_app_with_failover_wait_and_control_plane_with_preauth(
        vec![test_account(upstream.uri(), "upstream-token")],
        2_000,
        Some(control_plane.uri()),
        true,
        10_000,
        false,
    )
    .await;

    let request_body = json!({
        "model": "gpt-5.3-codex",
        "input": "abcdefgh"
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer cp_identity")
                .header("content-type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let expected_dynamic_reserve =
        ((((2_i64 + 256_i64) as f64) * 10_000_f64) / 1_000_000_f64 * 1.3_f64).ceil() as i64;
    let expected_dynamic_reserve = expected_dynamic_reserve.clamp(1_000, 1_000_000_000_000);
    let mut authorize_payload: Option<Value> = None;
    for _ in 0..20 {
        let requests = control_plane.received_requests().await.unwrap();
        if let Some(authorize) = requests
            .iter()
            .find(|request| request.url.path() == "/internal/v1/billing/authorize")
        {
            authorize_payload = Some(serde_json::from_slice(&authorize.body).unwrap());
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let authorize_payload = authorize_payload.expect("authorize request should be sent");
    assert_eq!(
        authorize_payload["reserved_microcredits"],
        expected_dynamic_reserve
    );
}

#[tokio::test]
async fn preauth_falls_back_to_fixed_reserve_when_dynamic_disabled() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_dynamic_2",
            "usage": {"input_tokens": 12, "output_tokens": 4}
        })))
        .mount(&upstream)
        .await;

    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": Uuid::new_v4(),
            "api_key_id": Uuid::new_v4(),
            "enabled": true,
            "tenant_status": "active",
            "balance_microcredits": 1_000_000,
            "cache_ttl_sec": 30
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/authorize"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorization_id": Uuid::new_v4(),
            "status": "authorized",
            "reserved_microcredits": 2_000_000
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/capture"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "captured"
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/release"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "released"
        })))
        .mount(&control_plane)
        .await;

    let app = test_app_with_failover_wait_and_control_plane_with_preauth(
        vec![test_account(upstream.uri(), "upstream-token")],
        2_000,
        Some(control_plane.uri()),
        false,
        10_000,
        false,
    )
    .await;

    let request_body = json!({
        "model": "gpt-5.3-codex",
        "input": "abcdefgh"
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer cp_identity")
                .header("content-type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let mut authorize_payload: Option<Value> = None;
    for _ in 0..20 {
        let requests = control_plane.received_requests().await.unwrap();
        if let Some(authorize) = requests
            .iter()
            .find(|request| request.url.path() == "/internal/v1/billing/authorize")
        {
            authorize_payload = Some(serde_json::from_slice(&authorize.body).unwrap());
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let authorize_payload = authorize_payload.expect("authorize request should be sent");
    assert_eq!(authorize_payload["reserved_microcredits"], 2_000_000);
}

#[tokio::test]
async fn free_priced_request_hits_minimum_preauth_reserve() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_dynamic_free",
            "usage": {"input_tokens": 6, "output_tokens": 2}
        })))
        .mount(&upstream)
        .await;

    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": Uuid::new_v4(),
            "api_key_id": Uuid::new_v4(),
            "enabled": true,
            "tenant_status": "active",
            "balance_microcredits": 1_000_000,
            "cache_ttl_sec": 30
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/authorize"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorization_id": Uuid::new_v4(),
            "status": "authorized",
            "reserved_microcredits": 1_000
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/capture"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "captured"
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/release"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "released"
        })))
        .mount(&control_plane)
        .await;

    let app = test_app_with_failover_wait_and_control_plane_with_preauth_limits(
        vec![test_account(upstream.uri(), "upstream-token")],
        2_000,
        Some(control_plane.uri()),
        true,
        256,
        1.3,
        1_000,
        1_000_000_000_000,
        0,
        false,
    )
    .await;

    let request_body = json!({
        "model": "gpt-5.3-codex",
        "input": "abcdefgh"
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer cp_identity")
                .header("content-type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let mut authorize_payload: Option<Value> = None;
    for _ in 0..20 {
        let requests = control_plane.received_requests().await.unwrap();
        if let Some(authorize) = requests
            .iter()
            .find(|request| request.url.path() == "/internal/v1/billing/authorize")
        {
            authorize_payload = Some(serde_json::from_slice(&authorize.body).unwrap());
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let authorize_payload = authorize_payload.expect("authorize request should be sent");
    assert_eq!(authorize_payload["reserved_microcredits"], 1_000);
}

#[tokio::test]
async fn high_price_request_reserve_is_clamped_by_maximum_limit() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_dynamic_high_price",
            "usage": {"input_tokens": 6, "output_tokens": 2}
        })))
        .mount(&upstream)
        .await;

    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": Uuid::new_v4(),
            "api_key_id": Uuid::new_v4(),
            "enabled": true,
            "tenant_status": "active",
            "balance_microcredits": 1_000_000,
            "cache_ttl_sec": 30
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/authorize"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorization_id": Uuid::new_v4(),
            "status": "authorized",
            "reserved_microcredits": 3_000_000
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/capture"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "captured"
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/release"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "released"
        })))
        .mount(&control_plane)
        .await;

    let app = test_app_with_failover_wait_and_control_plane_with_preauth_limits(
        vec![test_account(upstream.uri(), "upstream-token")],
        2_000,
        Some(control_plane.uri()),
        true,
        256,
        1.3,
        1_000,
        3_000_000,
        10_000_000_000,
        false,
    )
    .await;

    let request_body = json!({
        "model": "gpt-5.3-codex",
        "input": "abcdefgh"
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer cp_identity")
                .header("content-type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let mut authorize_payload: Option<Value> = None;
    for _ in 0..20 {
        let requests = control_plane.received_requests().await.unwrap();
        if let Some(authorize) = requests
            .iter()
            .find(|request| request.url.path() == "/internal/v1/billing/authorize")
        {
            authorize_payload = Some(serde_json::from_slice(&authorize.body).unwrap());
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let authorize_payload = authorize_payload.expect("authorize request should be sent");
    assert_eq!(authorize_payload["reserved_microcredits"], 3_000_000);
}

#[tokio::test]
async fn model_pricing_endpoint_drives_dynamic_preauth_and_cache_reuse() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_pricing_cache",
            "usage": {"input_tokens": 3, "output_tokens": 1}
        })))
        .mount(&upstream)
        .await;

    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": Uuid::new_v4(),
            "api_key_id": Uuid::new_v4(),
            "enabled": true,
            "tenant_status": "active",
            "balance_microcredits": 1_000_000,
            "cache_ttl_sec": 30
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/pricing"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "model": "gpt-5.3-codex",
            "input_price_microcredits": 20_000_000,
            "cached_input_price_microcredits": 2_000_000,
            "output_price_microcredits": 40_000_000,
            "source": "exact"
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/authorize"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorization_id": Uuid::new_v4(),
            "status": "authorized",
            "reserved_microcredits": 13_364
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/capture"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "captured"
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/release"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "released"
        })))
        .mount(&control_plane)
        .await;

    let app = test_app_with_failover_wait_and_control_plane_with_preauth(
        vec![test_account(upstream.uri(), "upstream-token")],
        2_000,
        Some(control_plane.uri()),
        true,
        10_000,
        false,
    )
    .await;

    let request_body = json!({
        "model": "gpt-5.3-codex",
        "input": "abcdefgh"
    });
    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/responses")
                    .header("authorization", "Bearer cp_identity")
                    .header("content-type", "application/json")
                    .body(Body::from(request_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    let expected_reserve =
        ((((2_i64 * 20_000_000_i64) + (256_i64 * 40_000_000_i64)) as f64) / 1_000_000_f64 * 1.3)
            .ceil() as i64;
    let mut authorize_payload: Option<Value> = None;
    let mut pricing_count = 0usize;
    let mut authorize_count = 0usize;
    for _ in 0..30 {
        let requests = control_plane.received_requests().await.unwrap();
        pricing_count = requests
            .iter()
            .filter(|request| request.url.path() == "/internal/v1/billing/pricing")
            .count();
        authorize_count = requests
            .iter()
            .filter(|request| request.url.path() == "/internal/v1/billing/authorize")
            .count();
        if let Some(authorize) = requests
            .iter()
            .find(|request| request.url.path() == "/internal/v1/billing/authorize")
        {
            authorize_payload = Some(serde_json::from_slice(&authorize.body).unwrap());
        }
        if authorize_payload.is_some() && authorize_count >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let authorize_payload = authorize_payload.expect("authorize request should be sent");
    assert_eq!(authorize_payload["reserved_microcredits"], expected_reserve);
    assert_eq!(authorize_count, 2);
    assert_eq!(pricing_count, 1);
}

#[tokio::test]
async fn codex_profile_fast_service_tier_flows_into_pricing_and_request_log() {
    let tenant_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();

    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/backend-api/codex/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_service_tier_http",
            "service_tier": "priority",
            "usage": {"input_tokens": 3, "output_tokens": 1}
        })))
        .mount(&upstream)
        .await;

    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": tenant_id,
            "api_key_id": api_key_id,
            "enabled": true,
            "tenant_status": "active",
            "balance_microcredits": 1_000_000,
            "cache_ttl_sec": 30
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/pricing"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "model": "gpt-5.3-codex",
            "input_price_microcredits": 20_000_000,
            "cached_input_price_microcredits": 2_000_000,
            "output_price_microcredits": 40_000_000,
            "source": "priority_exact"
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/authorize"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorization_id": Uuid::new_v4(),
            "status": "authorized",
            "reserved_microcredits": 13_364
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/capture"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "captured"
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/release"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "released"
        })))
        .mount(&control_plane)
        .await;

    let sink = Arc::new(RecordingSink::default());
    let cfg = DataPlaneConfig {
        listen_addr: "127.0.0.1:0".parse().unwrap(),
        routing_strategy: RoutingStrategy::RoundRobin,
        upstream_accounts: vec![test_account(
            format!("{}/backend-api/codex", upstream.uri()),
            "upstream-token",
        )],
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
        auth_validate_url: Some(format!("{}/internal/v1/auth/validate", control_plane.uri())),
        auth_validate_cache_ttl_sec: 30,
        auth_validate_negative_cache_ttl_sec: 5,
        auth_fail_open: false,
        enable_internal_debug_routes: false,
    };
    let app = build_app_with_event_sink(cfg, sink.clone())
        .await
        .expect("app should build");

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer cp_identity")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "model": "gpt-5.3-codex",
                        "input": "hello priority",
                        "service_tier": "fast"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let (pricing_payload, events) = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let requests = control_plane.received_requests().await.unwrap();
            let pricing_payload = requests
                .iter()
                .find(|request| request.url.path() == "/internal/v1/billing/pricing")
                .map(|request| serde_json::from_slice::<Value>(&request.body).unwrap());
            let events = sink.events();
            if pricing_payload.is_some() && !events.is_empty() {
                return (pricing_payload, events);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("pricing request and request log should arrive");

    let pricing_payload = pricing_payload.expect("pricing payload should exist");
    assert_eq!(pricing_payload["model"], "gpt-5.3-codex");
    assert_eq!(pricing_payload["service_tier"], "priority");

    let final_event = events.last().expect("expected final request log event");
    assert_eq!(final_event.model.as_deref(), Some("gpt-5.3-codex"));
    assert_eq!(final_event.service_tier.as_deref(), Some("priority"));
    assert_eq!(final_event.capture_status.as_deref(), Some("captured"));
}

#[tokio::test]
async fn non_stream_billing_settlement_uses_usage_tokens_and_releases_hold() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_non_stream_settle",
            "usage": {"input_tokens": 21, "output_tokens": 9}
        })))
        .mount(&upstream)
        .await;

    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": Uuid::new_v4(),
            "api_key_id": Uuid::new_v4(),
            "enabled": true,
            "tenant_status": "active",
            "balance_microcredits": 1_000_000,
            "cache_ttl_sec": 30
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/authorize"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorization_id": Uuid::new_v4(),
            "status": "authorized",
            "reserved_microcredits": 2_000_000
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/capture"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "captured"
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/release"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "released"
        })))
        .mount(&control_plane)
        .await;

    let app = test_app_with_failover_wait_and_control_plane(
        vec![test_account(upstream.uri(), "upstream-token")],
        2_000,
        Some(control_plane.uri()),
    )
    .await;

    let request_body = json!({
        "model": "gpt-5.3-codex",
        "input": "let us settle"
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer cp_identity")
                .header("content-type", "application/json")
                .body(Body::from(request_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let mut capture_payload: Option<Value> = None;
    let mut capture_count = 0usize;
    let mut release_count = 0usize;
    for _ in 0..20 {
        let requests = control_plane.received_requests().await.unwrap();
        capture_count = requests
            .iter()
            .filter(|request| request.url.path() == "/internal/v1/billing/capture")
            .count();
        release_count = requests
            .iter()
            .filter(|request| request.url.path() == "/internal/v1/billing/release")
            .count();
        if let Some(capture) = requests
            .iter()
            .find(|request| request.url.path() == "/internal/v1/billing/capture")
        {
            capture_payload = Some(serde_json::from_slice(&capture.body).unwrap());
        }
        if capture_payload.is_some() && release_count >= 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let capture_payload = capture_payload.expect("capture request should be sent");
    assert_eq!(capture_payload["input_tokens"], 21);
    assert_eq!(capture_payload["output_tokens"], 9);
    assert_eq!(capture_count, 1);
    assert_eq!(release_count, 1);
}

#[tokio::test]
async fn fails_over_before_first_sse_chunk_when_upstream_stream_is_empty() {
    let upstream_a = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("", "text/event-stream"))
        .mount(&upstream_a)
        .await;

    let upstream_b = MockServer::start().await;
    let sse_payload = "data: {\"type\":\"response.output_text.delta\",\"delta\":\"from-b\"}\n\n";
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(sse_payload, "text/event-stream"))
        .mount(&upstream_b)
        .await;

    let app = test_app(vec![
        test_account(upstream_a.uri(), "token-a"),
        test_account(upstream_b.uri(), "token-b"),
    ])
    .await;

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

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    assert!(content_type.contains("text/event-stream"));

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(bytes.as_ref(), sse_payload.as_bytes());
}

#[tokio::test]
async fn fails_over_on_stream_prelude_quota_error_event() {
    let upstream_a = MockServer::start().await;
    let quota_error_payload = "data: {\"type\":\"error\",\"error\":{\"code\":\"usage_limit\",\"message\":\"You've hit your usage limit. Start a free trial of Plus today.\"}}\n\n";
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(quota_error_payload, "text/event-stream"),
        )
        .mount(&upstream_a)
        .await;

    let upstream_b = MockServer::start().await;
    let success_payload =
        "data: {\"type\":\"response.output_text.delta\",\"delta\":\"from-b\"}\n\n";
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(success_payload, "text/event-stream"))
        .mount(&upstream_b)
        .await;

    let app = test_app(vec![
        test_account(upstream_a.uri(), "token-a"),
        test_account(upstream_b.uri(), "token-b"),
    ])
    .await;

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

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = tokio::time::timeout(Duration::from_secs(2), response.into_body().collect())
        .await
        .expect("stream body should complete after failover")
        .unwrap()
        .to_bytes();
    assert_eq!(bytes.as_ref(), success_payload.as_bytes());
}

#[tokio::test]
async fn fails_over_when_stream_fails_after_response_created_before_output() {
    let upstream_a = spawn_scripted_sse_upstream(vec![
        "event: response.created\ndata: {\"type\":\"response.created\",\"response\":{\"id\":\"resp-created-only\",\"model\":\"gpt-5.4\"}}\n\n",
        "event: response.failed\ndata: {\"type\":\"response.failed\",\"response\":{\"id\":\"resp-created-only\",\"status\":\"failed\",\"error\":{\"code\":\"rate_limit_exceeded\",\"message\":\"too many requests\"}}}\n\n",
    ])
    .await;

    let upstream_b = MockServer::start().await;
    let success_payload =
        "data: {\"type\":\"response.output_text.delta\",\"delta\":\"from-b\"}\n\n";
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(success_payload, "text/event-stream"))
        .mount(&upstream_b)
        .await;

    let app = test_app(vec![
        test_account(upstream_a, "token-a"),
        test_account(upstream_b.uri(), "token-b"),
    ])
    .await;

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

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = tokio::time::timeout(Duration::from_secs(2), response.into_body().collect())
        .await
        .expect("stream body should complete after failover")
        .unwrap()
        .to_bytes();
    assert_eq!(bytes.as_ref(), success_payload.as_bytes());
}

#[tokio::test]
async fn fails_over_when_stream_closes_after_response_created_before_output() {
    let upstream_a = spawn_scripted_sse_upstream(vec![
        "event: response.created\ndata: {\"type\":\"response.created\",\"response\":{\"id\":\"resp-created-then-eof\",\"model\":\"gpt-5.4\"}}\n\n",
    ])
    .await;

    let upstream_b = MockServer::start().await;
    let success_payload =
        "data: {\"type\":\"response.output_text.delta\",\"delta\":\"from-b\"}\n\n";
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(success_payload, "text/event-stream"))
        .mount(&upstream_b)
        .await;

    let app = test_app(vec![
        test_account(upstream_a, "token-a"),
        test_account(upstream_b.uri(), "token-b"),
    ])
    .await;

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

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(bytes.as_ref(), success_payload.as_bytes());
}

#[tokio::test]
async fn fails_over_on_http_402_payment_required_response() {
    let upstream_a = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(402).set_body_json(json!({
            "message": "Upgrade to Plus to continue using Codex"
        })))
        .mount(&upstream_a)
        .await;

    let upstream_b = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"account": "b"})))
        .mount(&upstream_b)
        .await;

    let app = test_app(vec![
        test_account(upstream_a.uri(), "token-a"),
        test_account(upstream_b.uri(), "token-b"),
    ])
    .await;

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

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["account"], "b");
}

#[tokio::test]
async fn fails_over_on_official_429_quota_response() {
    let upstream_a = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(429).set_body_json(json!({
            "error": {
                "message": "You exceeded your current quota, please check your plan and billing details"
            }
        })))
        .mount(&upstream_a)
        .await;

    let upstream_b = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"account": "b"})))
        .mount(&upstream_b)
        .await;

    let app = test_app(vec![
        test_account(upstream_a.uri(), "token-a"),
        test_account(upstream_b.uri(), "token-b"),
    ])
    .await;

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

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["account"], "b");
}

#[tokio::test]
async fn stream_prelude_auth_error_event_maps_to_consistent_error_envelope() {
    let upstream = MockServer::start().await;
    let auth_error_payload = "data: {\"type\":\"error\",\"error\":{\"message\":\"Your access token could not be refreshed because you have since logged out or signed in to another account. Please sign in again.\"}}\n\n";
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(auth_error_payload, "text/event-stream"),
        )
        .mount(&upstream)
        .await;

    let app = test_app(vec![test_account(upstream.uri(), "upstream-token")]).await;

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
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(payload["error"]["code"], "auth_expired");
    assert_eq!(
        payload["error"]["message"],
        "upstream account authentication expired; retry later with another account"
    );
}

#[tokio::test]
async fn auth_expired_single_account_recovers_immediately_after_fallback_refresh_success() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let upstream_router = {
        let attempts = attempts.clone();
        Router::new().route(
            "/v1/responses",
            post(move || {
                let attempts = attempts.clone();
                async move {
                    let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                    if attempt == 0 {
                        (
                            StatusCode::UNAUTHORIZED,
                            [(CONTENT_TYPE, "application/json")],
                            Body::from(
                                json!({
                                    "error": {
                                        "message": "Your access token could not be refreshed because you have since logged out."
                                    }
                                })
                                .to_string(),
                            ),
                        )
                    } else {
                        (
                            StatusCode::OK,
                            [(CONTENT_TYPE, "application/json")],
                            Body::from(json!({"account": "recovered"}).to_string()),
                        )
                    }
                }
            }),
        )
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, upstream_router).await.unwrap();
    });

    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": Uuid::new_v4(),
            "api_key_id": Uuid::new_v4(),
            "enabled": true,
            "cache_ttl_sec": 30
        })))
        .mount(&control_plane)
        .await;

    let account = test_account(format!("http://{}", addr), "token-a");
    Mock::given(method("POST"))
        .and(path(format!(
            "/internal/v1/upstream-accounts/{}/oauth/refresh",
            account.id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "last_refresh_status": "failed",
            "effective_enabled": true,
            "has_access_token_fallback": true,
            "refresh_credential_state": "terminal_invalid"
        })))
        .mount(&control_plane)
        .await;

    let app = test_app_with_failover_wait_and_control_plane(
        vec![account],
        150,
        Some(control_plane.uri()),
    )
    .await;

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer cp_identity")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::UNAUTHORIZED);

    let second = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer cp_identity")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::OK);

    let payload: Value =
        serde_json::from_slice(&second.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(payload["account"], "recovered");
}

#[tokio::test]
async fn auth_expired_cross_account_recovery_clears_unhealthy_after_async_refresh_success() {
    let upstream_a = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": {
                "message": "Your access token could not be refreshed because you have since logged out."
            }
        })))
        .mount(&upstream_a)
        .await;

    let upstream_b = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"account": "b"})))
        .mount(&upstream_b)
        .await;

    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": Uuid::new_v4(),
            "api_key_id": Uuid::new_v4(),
            "enabled": true,
            "cache_ttl_sec": 30
        })))
        .mount(&control_plane)
        .await;

    let account_a = test_account(upstream_a.uri(), "token-a");
    let account_b = test_account(upstream_b.uri(), "token-b");
    let account_a_id = account_a.id;
    Mock::given(method("POST"))
        .and(path(format!(
            "/internal/v1/upstream-accounts/{}/oauth/refresh",
            account_a_id
        )))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(30))
                .set_body_json(json!({
                    "last_refresh_status": "failed",
                    "effective_enabled": true,
                    "has_access_token_fallback": true,
                    "refresh_credential_state": "terminal_invalid"
                })),
        )
        .mount(&control_plane)
        .await;

    let app = test_app_with_failover_wait_and_control_plane_with_debug(
        vec![account_a, account_b],
        150,
        Some(control_plane.uri()),
        true,
    )
    .await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer cp_identity")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let payload: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(payload["account"], "b");

    let mut unhealthy_payload = None;
    for _ in 0..30 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/internal/v1/debug/accounts/unhealthy")
                    .header("authorization", "Bearer cp_identity")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let payload: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        let contains_account_a = payload["accounts"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .any(|item| item["id"] == account_a_id.to_string())
            })
            .unwrap_or(false);
        if !contains_account_a {
            unhealthy_payload = Some(payload);
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let unhealthy_payload =
        unhealthy_payload.expect("account A should be cleared from unhealthy list");
    assert!(unhealthy_payload["accounts"]
        .as_array()
        .map(|items| items
            .iter()
            .all(|item| item["id"] != account_a_id.to_string()))
        .unwrap_or(true));
}

#[tokio::test]
async fn maps_server_overloaded_to_consistent_error_envelope() {
    let upstream = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(503).set_body_json(
            json!({"error": {"code": "server_is_overloaded", "message": "slow_down"}}),
        ))
        .mount(&upstream)
        .await;

    let app = test_app(vec![test_account(upstream.uri(), "upstream-token")]).await;

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

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(payload["error"]["code"], "server_overloaded");
}

#[tokio::test]
async fn ejects_account_after_overloaded_503_response() {
    let upstream_a = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(503).set_body_json(
                json!({"error": {"code": "server_is_overloaded", "message": "busy"}}),
            ),
        )
        .mount(&upstream_a)
        .await;

    let upstream_b = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"account": "b"})))
        .mount(&upstream_b)
        .await;

    let app = test_app(vec![
        test_account(upstream_a.uri(), "token-a"),
        test_account(upstream_b.uri(), "token-b"),
    ])
    .await;

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let first_body = first.into_body().collect().await.unwrap().to_bytes();
    let first_payload: Value = serde_json::from_slice(&first_body).unwrap();
    assert_eq!(first_payload["account"], "b");

    let second = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::OK);

    let third = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(third.status(), StatusCode::OK);
}

#[tokio::test]
async fn ejects_account_after_generic_500_response() {
    let upstream_a = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(500)
                .set_body_json(json!({"error": {"code": "internal_error", "message": "boom"}})),
        )
        .mount(&upstream_a)
        .await;

    let upstream_b = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"account": "b"})))
        .mount(&upstream_b)
        .await;

    let app = test_app(vec![
        test_account(upstream_a.uri(), "token-a"),
        test_account(upstream_b.uri(), "token-b"),
    ])
    .await;

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let first_body = first.into_body().collect().await.unwrap().to_bytes();
    let first_payload: Value = serde_json::from_slice(&first_body).unwrap();
    assert_eq!(first_payload["account"], "b");

    let second = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::OK);

    let third = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(third.status(), StatusCode::OK);
}

#[tokio::test]
async fn single_request_failover_succeeds_when_next_account_is_healthy() {
    let upstream_a = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(500)
                .set_body_json(json!({"error": {"code": "internal_error", "message": "boom"}})),
        )
        .mount(&upstream_a)
        .await;

    let upstream_b = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"account": "b"})))
        .mount(&upstream_b)
        .await;

    let app = test_app(vec![
        test_account(upstream_a.uri(), "token-a"),
        test_account(upstream_b.uri(), "token-b"),
    ])
    .await;

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
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["account"], "b");
}

#[tokio::test]
async fn ejects_account_after_401_response() {
    let upstream_a = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_json(json!({"error": {"code": "unauthorized", "message": "invalid"}})),
        )
        .mount(&upstream_a)
        .await;

    let upstream_b = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"account": "b"})))
        .mount(&upstream_b)
        .await;

    let app = test_app(vec![
        test_account(upstream_a.uri(), "token-a"),
        test_account(upstream_b.uri(), "token-b"),
    ])
    .await;

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let first_body = first.into_body().collect().await.unwrap().to_bytes();
    let first_payload: Value = serde_json::from_slice(&first_body).unwrap();
    assert_eq!(first_payload["account"], "b");

    let second = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::OK);

    let third = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(third.status(), StatusCode::OK);
}

#[tokio::test]
async fn token_invalidated_should_failover_without_waiting_internal_refresh_completion() {
    let upstream_a = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(401).set_body_json(
                json!({"error": {"code": "token_invalidated", "message": "invalid"}}),
            ),
        )
        .mount(&upstream_a)
        .await;

    let upstream_b = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"account": "b"})))
        .mount(&upstream_b)
        .await;

    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": Uuid::new_v4(),
            "api_key_id": Uuid::new_v4(),
            "enabled": true,
            "cache_ttl_sec": 30
        })))
        .mount(&control_plane)
        .await;
    let account_a = test_account(upstream_a.uri(), "token-a");
    let account_b = test_account(upstream_b.uri(), "token-b");
    Mock::given(method("POST"))
        .and(path(format!(
            "/internal/v1/upstream-accounts/{}/oauth/refresh",
            account_a.id
        )))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(700))
                .set_body_json(json!({"last_refresh_status":"ok"})),
        )
        .mount(&control_plane)
        .await;

    let app = test_app_with_failover_wait_and_control_plane(
        vec![account_a, account_b],
        150,
        Some(control_plane.uri()),
    )
    .await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer cp_identity")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["account"], "b");
}

#[tokio::test]
async fn accountid_extraction_failure_should_failover_across_accounts() {
    let upstream_a = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(400).set_body_json(
                json!({"error": {"message": "Failed to extract accountId from token"}}),
            ),
        )
        .mount(&upstream_a)
        .await;

    let upstream_b = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"account": "b"})))
        .mount(&upstream_b)
        .await;

    let app = test_app(vec![
        test_account(upstream_a.uri(), "token-a"),
        test_account(upstream_b.uri(), "token-b"),
    ])
    .await;

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
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["account"], "b");
}

#[tokio::test]
async fn plain_text_accountid_extraction_failure_should_failover_across_accounts() {
    let upstream_a = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(400)
                .insert_header("content-type", "text/plain; charset=utf-8")
                .set_body_string("Failed to extract accountId from token"),
        )
        .mount(&upstream_a)
        .await;

    let upstream_b = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"account": "b"})))
        .mount(&upstream_b)
        .await;

    let app = test_app(vec![
        test_account(upstream_a.uri(), "token-a"),
        test_account(upstream_b.uri(), "token-b"),
    ])
    .await;

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
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["account"], "b");
}

#[tokio::test]
async fn plain_text_stream_accountid_extraction_failure_should_failover_across_accounts() {
    let upstream_a = MockServer::start().await;
    let auth_error_payload = "data: Failed to extract accountId from token\n\n";
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(auth_error_payload, "text/event-stream"),
        )
        .mount(&upstream_a)
        .await;

    let upstream_b = MockServer::start().await;
    let success_payload =
        "data: {\"type\":\"response.output_text.delta\",\"delta\":\"from-b\"}\n\n";
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(success_payload, "text/event-stream"))
        .mount(&upstream_b)
        .await;

    let app = test_app(vec![
        test_account(upstream_a.uri(), "token-a"),
        test_account(upstream_b.uri(), "token-b"),
    ])
    .await;

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

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(bytes.as_ref(), success_payload.as_bytes());
}

#[tokio::test]
async fn ejects_account_after_429_response() {
    let upstream_a = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_json(json!({"error": {"code": "rate_limited", "message": "slow"}})),
        )
        .mount(&upstream_a)
        .await;

    let upstream_b = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"account": "b"})))
        .mount(&upstream_b)
        .await;

    let app = test_app(vec![
        test_account(upstream_a.uri(), "token-a"),
        test_account(upstream_b.uri(), "token-b"),
    ])
    .await;

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let first_body = first.into_body().collect().await.unwrap().to_bytes();
    let first_payload: Value = serde_json::from_slice(&first_body).unwrap();
    assert_eq!(first_payload["account"], "b");

    let second = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::OK);

    let third = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(third.status(), StatusCode::OK);
}

#[tokio::test]
async fn blocks_high_frequency_non_retryable_client_errors_for_same_api_key() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_json(json!({"error":{"code":"model_not_found","message":"missing"}})),
        )
        .mount(&upstream)
        .await;

    let app = build_app_with_event_sink_and_allowed_keys(
        DataPlaneConfig {
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
        },
        Arc::new(NoopEventSink),
        vec!["tenant-attack-key".to_string()],
    )
    .await
    .unwrap();

    for _ in 0..12 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/responses")
                    .header("authorization", "Bearer tenant-attack-key")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    let blocked = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer tenant-attack-key")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(blocked.status(), StatusCode::TOO_MANY_REQUESTS);

    let blocked_body = blocked.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&blocked_body).unwrap();
    assert_eq!(payload["error"]["code"], "invalid_request_rate_limited");

    let received = upstream.received_requests().await.unwrap();
    assert_eq!(received.len(), 12);
}

#[tokio::test]
async fn usage_endpoint_reports_account_counts() {
    let app = test_app(vec![
        test_account("http://one".to_string(), "token-1"),
        test_account("http://two".to_string(), "token-2"),
    ])
    .await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/codex/usage")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let usage: UsageSummary = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(usage.account_total, 2);
    assert_eq!(usage.active_account_total, 2);
}

#[tokio::test]
async fn request_log_event_contains_identity_from_online_validation() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .mount(&upstream)
        .await;

    let auth_server = MockServer::start().await;
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
        .mount(&auth_server)
        .await;

    let sink = Arc::new(RecordingSink::default());
    let app = build_app_with_event_sink_and_allowed_keys(
        DataPlaneConfig {
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
            auth_validate_url: Some(format!("{}/internal/v1/auth/validate", auth_server.uri())),
            auth_validate_cache_ttl_sec: 30,
            auth_validate_negative_cache_ttl_sec: 5,
            auth_fail_open: false,
            enable_internal_debug_routes: false,
        },
        sink.clone(),
        Vec::new(),
    )
    .await
    .unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", "Bearer cp_identity")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let events = sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].tenant_id, Some(tenant_id));
    assert_eq!(events[0].api_key_id, Some(api_key_id));
    assert_eq!(events[0].event_version, 2);
}
