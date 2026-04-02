use std::sync::Arc;

use axum::body::Body;
use axum::Router;
use codex_pool_core::model::{
    ClaudeCodeRoutingSettings, RoutingStrategy, UpstreamAccount, UpstreamMode,
};
use data_plane::app::{
    build_embedded_app_with_event_sink_without_status_routes as dp_build_embedded_app_with_event_sink_without_status_routes,
    AppState,
};
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

async fn build_embedded_app(config: DataPlaneConfig) -> anyhow::Result<(Router, Arc<AppState>)> {
    let _env_guard = support::lock_env().await;
    dp_build_embedded_app_with_event_sink_without_status_routes(config, Arc::new(NoopEventSink))
        .await
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

async fn embedded_test_app(accounts: Vec<UpstreamAccount>) -> (Router, Arc<AppState>) {
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
    build_embedded_app(cfg).await.expect("app should build")
}

fn set_claude_code_targets(
    state: &AppState,
    opus_target: Option<&str>,
    sonnet_target: Option<&str>,
    haiku_target: Option<&str>,
) {
    state.replace_claude_code_routing_settings(ClaudeCodeRoutingSettings {
        enabled: true,
        opus_target_model: opus_target.map(ToString::to_string),
        sonnet_target_model: sonnet_target.map(ToString::to_string),
        haiku_target_model: haiku_target.map(ToString::to_string),
        updated_at: chrono::Utc::now(),
    });
}

#[tokio::test]
async fn anthropic_non_stream_json_errors_map_to_expected_error_types() {
    for (status, code, upstream_message, expected_type, expected_message_fragment) in [
        (
            StatusCode::UNAUTHORIZED,
            "invalid_api_key",
            "bad api key",
            "authentication_error",
            "authentication expired",
        ),
        (
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limit_exceeded",
            "too many requests",
            "rate_limit_error",
            "rate limited",
        ),
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "upstream_unavailable",
            "service unavailable",
            "api_error",
            "service is unavailable",
        ),
    ] {
        let upstream = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/responses"))
            .respond_with(ResponseTemplate::new(status.as_u16()).set_body_json(json!({
                "error": {
                    "code": code,
                    "message": upstream_message,
                }
            })))
            .mount(&upstream)
            .await;

        let (app, state) =
            embedded_test_app(vec![test_account(upstream.uri(), "upstream-token")]).await;
        set_claude_code_targets(
            &state,
            Some("target-opus"),
            Some("target-sonnet"),
            Some("target-haiku"),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/messages")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "model": "claude-sonnet-4-6",
                            "max_tokens": 32,
                            "messages": [{"role": "user", "content": "Reply with exactly OK."}]
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), status);
        let payload: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(payload["type"], "error");
        assert_eq!(payload["error"]["type"], expected_type);
        let message = payload["error"]["message"].as_str().unwrap_or_default();
        assert!(
            message.contains(expected_message_fragment),
            "expected `{message}` to contain `{expected_message_fragment}`",
        );
    }
}

#[tokio::test]
async fn anthropic_stream_non_2xx_json_errors_become_sse_error_events() {
    for (status, code, upstream_message, expected_type, expected_message_fragment) in [
        (
            StatusCode::UNAUTHORIZED,
            "invalid_api_key",
            "bad api key",
            "authentication_error",
            "authentication expired",
        ),
        (
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limit_exceeded",
            "too many requests",
            "rate_limit_error",
            "rate limited",
        ),
        (
            StatusCode::BAD_GATEWAY,
            "upstream_failure",
            "upstream failed",
            "api_error",
            "upstream request failed",
        ),
    ] {
        let upstream = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/responses"))
            .respond_with(ResponseTemplate::new(status.as_u16()).set_body_json(json!({
                "error": {
                    "code": code,
                    "message": upstream_message,
                }
            })))
            .mount(&upstream)
            .await;

        let (app, state) =
            embedded_test_app(vec![test_account(upstream.uri(), "upstream-token")]).await;
        set_claude_code_targets(
            &state,
            Some("target-opus"),
            Some("target-sonnet"),
            Some("target-haiku"),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/messages")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "model": "claude-sonnet-4-6",
                            "max_tokens": 32,
                            "stream": true,
                            "messages": [{"role": "user", "content": "Reply with exactly OK."}]
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let body = String::from_utf8(
            response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
        .unwrap();

        assert!(content_type.contains("text/event-stream"));
        assert!(body.contains("event: error"), "{body}");
        assert!(body.contains("\"type\":\"error\""), "{body}");
        assert!(
            body.contains(&format!("\"type\":\"{expected_type}\"")),
            "{body}"
        );
        assert!(body.contains(expected_message_fragment), "{body}");
    }
}

#[tokio::test]
async fn anthropic_claude_code_family_mappings_forward_all_three_targets() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "resp_family_smoke",
            "status": "completed",
            "usage": {"input_tokens": 5, "output_tokens": 2},
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "OK"}]
            }]
        })))
        .mount(&upstream)
        .await;

    let (app, state) =
        embedded_test_app(vec![test_account(upstream.uri(), "upstream-token")]).await;
    set_claude_code_targets(
        &state,
        Some("target-opus"),
        Some("target-sonnet"),
        Some("target-haiku"),
    );

    for model in ["claude-opus-4-1", "claude-sonnet-4-6", "claude-haiku-4-5"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/messages")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "model": model,
                            "max_tokens": 32,
                            "messages": [{"role": "user", "content": "Reply with exactly OK."}]
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK, "{model}");
    }

    let requests = upstream.received_requests().await.unwrap();
    let forwarded_models = requests
        .iter()
        .map(|request| {
            serde_json::from_slice::<Value>(&request.body).unwrap()["model"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect::<Vec<_>>();

    assert_eq!(
        forwarded_models,
        vec![
            "target-opus".to_string(),
            "target-sonnet".to_string(),
            "target-haiku".to_string(),
        ]
    );
}
