use std::sync::Arc;
use std::time::Duration;

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

async fn test_app_with_control_plane(
    accounts: Vec<UpstreamAccount>,
    control_plane_base_url: String,
) -> Router {
    let _env_guard = support::lock_env().await;
    std::env::set_var("CONTROL_PLANE_BASE_URL", &control_plane_base_url);
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
        auth_validate_url: Some(format!(
            "{}/internal/v1/auth/validate",
            control_plane_base_url.trim_end_matches('/'),
        )),
        auth_validate_cache_ttl_sec: 30,
        auth_validate_negative_cache_ttl_sec: 5,
        auth_fail_open: false,
        enable_internal_debug_routes: false,
    };
    dp_build_app_with_event_sink(cfg, Arc::new(NoopEventSink))
        .await
        .expect("app should build")
}

#[tokio::test]
async fn standalone_compact_billing_payload_includes_session_key_and_request_kind() {
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
            ],
            "usage": {"input_tokens": 18, "cached_input_tokens": 0, "output_tokens": 7, "reasoning_tokens": 3}
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
            "group": {
                "id": Uuid::new_v4(),
                "name": "default",
                "invalid": false
            },
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

    let codex_base = format!("{}/backend-api/codex", upstream.uri());
    let app = test_app_with_control_plane(
        vec![test_account(codex_base, "upstream-token")],
        control_plane.uri(),
    )
    .await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses/compact")
                .header("authorization", "Bearer cp_identity")
                .header("content-type", "application/json")
                .header("session_id", "session-compact-1")
                .body(Body::from(
                    json!({
                        "model": "gpt-5.4",
                        "input": "compress this history"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["output"][0]["type"], "message");

    let mut authorize_payload = None;
    let mut capture_payload = None;
    for _ in 0..30 {
        let requests = control_plane.received_requests().await.unwrap();
        authorize_payload = requests
            .iter()
            .find(|request| request.url.path() == "/internal/v1/billing/authorize")
            .map(|request| serde_json::from_slice::<Value>(&request.body).unwrap());
        capture_payload = requests
            .iter()
            .find(|request| request.url.path() == "/internal/v1/billing/capture")
            .map(|request| serde_json::from_slice::<Value>(&request.body).unwrap());
        if authorize_payload.is_some() && capture_payload.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let authorize_payload = authorize_payload.expect("authorize request should be sent");
    let capture_payload = capture_payload.expect("capture request should be sent");
    assert_eq!(authorize_payload["session_key"], "session-compact-1");
    assert_eq!(authorize_payload["request_kind"], "compact");
    assert_eq!(capture_payload["session_key"], "session-compact-1");
    assert_eq!(capture_payload["request_kind"], "compact");
}

#[tokio::test]
async fn standalone_compact_billing_session_key_falls_back_to_previous_response_id() {
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
            ],
            "usage": {"input_tokens": 10, "cached_input_tokens": 0, "output_tokens": 4, "reasoning_tokens": 1}
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
            "group": {
                "id": Uuid::new_v4(),
                "name": "default",
                "invalid": false
            },
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

    let codex_base = format!("{}/backend-api/codex", upstream.uri());
    let app = test_app_with_control_plane(
        vec![test_account(codex_base, "upstream-token")],
        control_plane.uri(),
    )
    .await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses/compact")
                .header("authorization", "Bearer cp_identity")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "model": "gpt-5.4",
                        "previous_response_id": "resp_prev_123",
                        "input": "compress this history"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let mut authorize_payload = None;
    for _ in 0..30 {
        let requests = control_plane.received_requests().await.unwrap();
        authorize_payload = requests
            .iter()
            .find(|request| request.url.path() == "/internal/v1/billing/authorize")
            .map(|request| serde_json::from_slice::<Value>(&request.body).unwrap());
        if authorize_payload.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let authorize_payload = authorize_payload.expect("authorize request should be sent");
    assert_eq!(authorize_payload["session_key"], "resp_prev_123");
}

#[tokio::test]
async fn standalone_compact_billing_request_id_does_not_reuse_client_x_request_id() {
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
            ],
            "usage": {"input_tokens": 10, "cached_input_tokens": 0, "output_tokens": 4, "reasoning_tokens": 1}
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
            "group": {
                "id": Uuid::new_v4(),
                "name": "default",
                "invalid": false
            },
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

    for input in ["compress this history", "compress this history again"] {
        let codex_base = format!("{}/backend-api/codex", upstream.uri());
        let app = test_app_with_control_plane(
            vec![test_account(codex_base, "upstream-token")],
            control_plane.uri(),
        )
        .await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/responses/compact")
                    .header("authorization", "Bearer cp_identity")
                    .header("content-type", "application/json")
                    .header("x-request-id", "client-req-1")
                    .body(Body::from(
                        json!({
                            "model": "gpt-5.4",
                            "input": input
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            status,
            StatusCode::OK,
            "unexpected response body: {}",
            String::from_utf8_lossy(&body)
        );
    }

    let mut authorize_payloads = Vec::new();
    for _ in 0..30 {
        let requests = control_plane.received_requests().await.unwrap();
        authorize_payloads = requests
            .iter()
            .filter(|request| request.url.path() == "/internal/v1/billing/authorize")
            .map(|request| serde_json::from_slice::<Value>(&request.body).unwrap())
            .collect::<Vec<_>>();
        if authorize_payloads.len() == 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    assert_eq!(authorize_payloads.len(), 2);
    assert_eq!(
        authorize_payloads[0]["trace_request_id"],
        Value::String("client-req-1".to_string())
    );
    assert_eq!(
        authorize_payloads[1]["trace_request_id"],
        Value::String("client-req-1".to_string())
    );
    assert_ne!(
        authorize_payloads[0]["request_id"],
        Value::String("client-req-1".to_string())
    );
    assert_ne!(
        authorize_payloads[1]["request_id"],
        Value::String("client-req-1".to_string())
    );
    assert_ne!(
        authorize_payloads[0]["request_id"],
        authorize_payloads[1]["request_id"]
    );
}

#[tokio::test]
async fn standalone_compact_preserves_upstream_success_when_billing_capture_fails() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/backend-api/codex/responses/compact"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "output": [
                {
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "output_text", "text": "compacted despite billing error"}]
                }
            ],
            "usage": {"input_tokens": 18, "cached_input_tokens": 0, "output_tokens": 7, "reasoning_tokens": 3}
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
            "group": {
                "id": Uuid::new_v4(),
                "name": "default",
                "invalid": false
            },
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
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "error": { "code": "capture_failed", "message": "synthetic capture failure" }
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

    let codex_base = format!("{}/backend-api/codex", upstream.uri());
    let app = test_app_with_control_plane(
        vec![test_account(codex_base, "upstream-token")],
        control_plane.uri(),
    )
    .await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses/compact")
                .header("authorization", "Bearer cp_identity")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "model": "gpt-5.4",
                        "input": "compress this history"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        payload["output"][0]["content"][0]["text"],
        "compacted despite billing error"
    );

    let (capture_count, release_count) = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let requests = control_plane.received_requests().await.unwrap();
            let capture_count = requests
                .iter()
                .filter(|request| request.url.path() == "/internal/v1/billing/capture")
                .count();
            let release_count = requests
                .iter()
                .filter(|request| request.url.path() == "/internal/v1/billing/release")
                .count();
            if capture_count >= 1 && release_count == 1 {
                return (capture_count, release_count);
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("capture failure path should still release the hold");

    assert!(capture_count >= 1);
    assert_eq!(release_count, 1);
}

#[tokio::test]
async fn standalone_compact_preserves_billing_model_missing_error_code() {
    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": Uuid::new_v4(),
            "api_key_id": Uuid::new_v4(),
            "enabled": true,
            "group": {
                "id": Uuid::new_v4(),
                "name": "default",
                "invalid": false
            },
            "tenant_status": "active",
            "balance_microcredits": 1_000_000,
            "cache_ttl_sec": 30
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/authorize"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": {
                "code": "billing_model_missing",
                "message": "billing model missing"
            }
        })))
        .mount(&control_plane)
        .await;

    let upstream = MockServer::start().await;
    let codex_base = format!("{}/backend-api/codex", upstream.uri());
    let app = test_app_with_control_plane(
        vec![test_account(codex_base, "upstream-token")],
        control_plane.uri(),
    )
    .await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses/compact")
                .header("authorization", "Bearer cp_identity")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "model": "missing-model",
                        "input": "compress this history"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["error"]["code"], "billing_model_missing");
    assert_eq!(payload["error"]["message"], "billing model missing");
}
