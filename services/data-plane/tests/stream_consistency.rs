use std::sync::Arc;

use axum::body::Body;
use axum::Router;
use codex_pool_core::model::{RoutingStrategy, UpstreamAccount, UpstreamMode};
use data_plane::app::build_app_with_event_sink as dp_build_app_with_event_sink;
use data_plane::config::DataPlaneConfig;
use data_plane::event::NoopEventSink;
use futures_util::{SinkExt, StreamExt};
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::handshake::server::ErrorResponse;
use tokio_tungstenite::tungstenite::protocol::Message;
use tower::ServiceExt;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support;

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
        label: "acc-consistency".to_string(),
        mode: UpstreamMode::ChatGptSession,
        base_url,
        bearer_token: token.to_string(),
        chatgpt_account_id: Some("acct_consistency".to_string()),
        enabled: true,
        priority: 100,
        created_at: chrono::Utc::now(),
    }
}

async fn build_test_app(account: UpstreamAccount) -> Router {
    build_app_with_event_sink(
        DataPlaneConfig {
            listen_addr: "127.0.0.1:0".parse().unwrap(),
            routing_strategy: RoutingStrategy::RoundRobin,
            upstream_accounts: vec![account],
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
    )
    .await
    .expect("app should build")
}

async fn spawn_data_plane_server(account: UpstreamAccount) -> String {
    let app = build_test_app(account).await;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

async fn spawn_scripted_ws_upstream(scripted_frames: Vec<String>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let frames = Arc::new(scripted_frames);

    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(v) => v,
                Err(_) => break,
            };
            let frames = frames.clone();
            tokio::spawn(async move {
                let ws_stream = accept_hdr_async(
                    stream,
                    |_request: &tokio_tungstenite::tungstenite::handshake::server::Request,
                     response: tokio_tungstenite::tungstenite::handshake::server::Response|
                     -> Result<
                        tokio_tungstenite::tungstenite::handshake::server::Response,
                        ErrorResponse,
                    > { Ok(response) },
                )
                .await;
                let Ok(mut ws_stream) = ws_stream else {
                    return;
                };

                for frame in frames.iter() {
                    if ws_stream
                        .send(Message::Text(frame.clone().into()))
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
                let _ = ws_stream.close(None).await;
            });
        }
    });

    format!("http://{}", addr)
}

fn ws_url(http_base: &str, path_and_query: &str) -> String {
    format!(
        "{}{}",
        http_base.replacen("http://", "ws://", 1),
        path_and_query
    )
}

fn build_sse_payload(event_types: &[&str]) -> String {
    let mut payload = String::new();
    for event_type in event_types {
        payload.push_str(&format!(
            "event: {event_type}\ndata: {{\"type\":\"{event_type}\"}}\n\n"
        ));
    }
    payload
}

fn parse_sse_event_types(body: &str) -> Vec<String> {
    body.lines()
        .filter_map(|line| line.strip_prefix("event: ").map(str::trim))
        .map(ToString::to_string)
        .collect()
}

fn parse_ws_event_types(frames: &[String]) -> Vec<String> {
    frames
        .iter()
        .filter_map(|raw| serde_json::from_str::<Value>(raw).ok())
        .filter_map(|payload| {
            payload
                .get("type")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .collect()
}

fn parse_sse_error_code(body: &str) -> Option<String> {
    for line in body.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if let Ok(payload) = serde_json::from_str::<Value>(data) {
                if payload.get("type").and_then(Value::as_str) == Some("error") {
                    if let Some(code) = payload
                        .get("error")
                        .and_then(|value| value.get("code"))
                        .and_then(Value::as_str)
                    {
                        return Some(code.to_string());
                    }
                }
            }
        }
    }
    None
}

fn parse_ws_error_code(frames: &[String]) -> Option<String> {
    for frame in frames {
        if let Ok(payload) = serde_json::from_str::<Value>(frame) {
            if payload.get("type").and_then(Value::as_str) == Some("error") {
                if let Some(code) = payload
                    .get("error")
                    .and_then(|value| value.get("code"))
                    .and_then(Value::as_str)
                {
                    return Some(code.to_string());
                }
            }
        }
    }
    None
}

#[tokio::test]
async fn responses_sse_and_websocket_keep_same_event_order() {
    let expected = vec![
        "response.created",
        "response.in_progress",
        "response.output_item.added",
        "response.completed",
    ];

    let sse_upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/backend-api/codex/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(build_sse_payload(&expected), "text/event-stream"),
        )
        .mount(&sse_upstream)
        .await;

    let sse_account = test_account(
        format!("{}/backend-api/codex", sse_upstream.uri()),
        "upstream-token-sse",
    );
    let sse_app = build_test_app(sse_account).await;
    let sse_response = sse_app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(sse_response.status(), StatusCode::OK);
    let sse_body = sse_response.into_body().collect().await.unwrap().to_bytes();
    let sse_event_types = parse_sse_event_types(std::str::from_utf8(&sse_body).unwrap());

    let ws_frames = expected
        .iter()
        .map(|event| serde_json::json!({ "type": event }).to_string())
        .collect::<Vec<_>>();
    let ws_upstream = spawn_scripted_ws_upstream(ws_frames).await;
    let ws_account = test_account(
        format!("{}/backend-api/codex", ws_upstream),
        "upstream-token-ws",
    );
    let data_plane_base = spawn_data_plane_server(ws_account).await;

    let request = ws_url(&data_plane_base, "/v1/responses")
        .into_client_request()
        .unwrap();
    let (mut ws_client, response) = connect_async(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

    let mut received_ws_frames = Vec::new();
    while let Some(message) = ws_client.next().await {
        let message = message.unwrap();
        match message {
            Message::Text(text) => received_ws_frames.push(text.to_string()),
            Message::Close(_) => break,
            _ => {}
        }
    }
    let ws_event_types = parse_ws_event_types(&received_ws_frames);

    let expected_types = expected
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>();
    assert_eq!(sse_event_types, expected_types);
    assert_eq!(ws_event_types, expected_types);
}

#[tokio::test]
async fn responses_sse_and_websocket_keep_same_error_event_code() {
    let error_payload =
        serde_json::json!({"type":"error","error":{"code":"server_overloaded","message":"busy"}})
            .to_string();

    let sse_upstream = MockServer::start().await;
    let sse_body = format!(
        "event: response.created\ndata: {{\"type\":\"response.created\"}}\n\nevent: error\ndata: {error_payload}\n\n"
    );
    Mock::given(method("POST"))
        .and(path("/backend-api/codex/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(sse_body, "text/event-stream"))
        .mount(&sse_upstream)
        .await;

    let sse_account = test_account(
        format!("{}/backend-api/codex", sse_upstream.uri()),
        "upstream-token-sse",
    );
    let sse_app = build_test_app(sse_account).await;
    let sse_response = sse_app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(sse_response.status(), StatusCode::OK);
    let sse_bytes = sse_response.into_body().collect().await.unwrap().to_bytes();
    let sse_error_code = parse_sse_error_code(std::str::from_utf8(&sse_bytes).unwrap());

    let ws_upstream = spawn_scripted_ws_upstream(vec![
        serde_json::json!({"type":"response.created"}).to_string(),
        error_payload,
    ])
    .await;
    let ws_account = test_account(
        format!("{}/backend-api/codex", ws_upstream),
        "upstream-token-ws",
    );
    let data_plane_base = spawn_data_plane_server(ws_account).await;

    let request = ws_url(&data_plane_base, "/v1/responses")
        .into_client_request()
        .unwrap();
    let (mut ws_client, response) = connect_async(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

    let mut received_ws_frames = Vec::new();
    while let Some(message) = ws_client.next().await {
        let message = message.unwrap();
        match message {
            Message::Text(text) => received_ws_frames.push(text.to_string()),
            Message::Close(_) => break,
            _ => {}
        }
    }
    let ws_error_code = parse_ws_error_code(&received_ws_frames);

    assert_eq!(sse_error_code.as_deref(), Some("server_overloaded"));
    assert_eq!(ws_error_code.as_deref(), Some("server_overloaded"));
}
