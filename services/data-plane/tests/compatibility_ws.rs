use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::Router;
use codex_pool_core::model::{RoutingStrategy, UpstreamAccount, UpstreamMode};
use data_plane::app::build_app_with_event_sink as dp_build_app_with_event_sink;
use data_plane::config::DataPlaneConfig;
use data_plane::event::NoopEventSink;
use futures_util::{SinkExt, StreamExt};
use http::StatusCode;
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::handshake::server::ErrorResponse;
use tokio_tungstenite::tungstenite::protocol::Message;
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

#[derive(Clone, Debug)]
struct HandshakeRecord {
    path_and_query: String,
    headers: HashMap<String, String>,
}

impl HandshakeRecord {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }
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
    test_app_with_control_plane(accounts, None).await
}

async fn test_app_with_control_plane(
    accounts: Vec<UpstreamAccount>,
    control_plane_base_url: Option<String>,
) -> Router {
    test_app_with_control_plane_and_failover_wait(accounts, control_plane_base_url, 2_000).await
}

async fn test_app_with_control_plane_and_failover_wait(
    accounts: Vec<UpstreamAccount>,
    control_plane_base_url: Option<String>,
    request_failover_wait_ms: u64,
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
    };
    build_app_with_event_sink(cfg, Arc::new(NoopEventSink))
        .await
        .expect("app should build")
}

async fn spawn_data_plane_server(accounts: Vec<UpstreamAccount>) -> String {
    let app = test_app(accounts).await;
    spawn_data_plane_server_with_app(app).await
}

async fn spawn_data_plane_server_with_control_plane(
    accounts: Vec<UpstreamAccount>,
    control_plane_base_url: Option<String>,
) -> String {
    spawn_data_plane_server_with_control_plane_and_failover_wait(
        accounts,
        control_plane_base_url,
        2_000,
    )
    .await
}

async fn spawn_data_plane_server_with_control_plane_and_failover_wait(
    accounts: Vec<UpstreamAccount>,
    control_plane_base_url: Option<String>,
    request_failover_wait_ms: u64,
) -> String {
    let app = test_app_with_control_plane_and_failover_wait(
        accounts,
        control_plane_base_url,
        request_failover_wait_ms,
    )
    .await;
    spawn_data_plane_server_with_app(app).await
}

async fn spawn_data_plane_server_with_app(app: Router) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

async fn spawn_ws_upstream() -> (String, Arc<Mutex<Vec<HandshakeRecord>>>) {
    spawn_ws_upstream_with_subprotocol_echo(true).await
}

async fn spawn_ws_upstream_with_subprotocol_echo(
    echo_subprotocol: bool,
) -> (String, Arc<Mutex<Vec<HandshakeRecord>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let records = Arc::new(Mutex::new(Vec::<HandshakeRecord>::new()));
    let records_for_task = records.clone();

    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(value) => value,
                Err(_) => break,
            };
            let records = records_for_task.clone();
            let echo_subprotocol = echo_subprotocol;
            tokio::spawn(async move {
                let ws_stream = accept_hdr_async(
                    stream,
                    |request: &tokio_tungstenite::tungstenite::handshake::server::Request,
                     mut response: tokio_tungstenite::tungstenite::handshake::server::Response|
                     -> Result<
                        tokio_tungstenite::tungstenite::handshake::server::Response,
                        ErrorResponse,
                    > {
                        let headers = request
                            .headers()
                            .iter()
                            .filter_map(|(name, value)| {
                                value.to_str().ok().map(|value| {
                                    (name.as_str().to_ascii_lowercase(), value.to_string())
                                })
                            })
                            .collect::<HashMap<_, _>>();
                        let path_and_query = request
                            .uri()
                            .path_and_query()
                            .map(|value| value.as_str().to_string())
                            .unwrap_or_else(|| request.uri().path().to_string());
                        if echo_subprotocol {
                            if let Some(protocol) = request
                                .headers()
                                .get("sec-websocket-protocol")
                                .and_then(|value| value.to_str().ok())
                                .map(|value| value.split(',').next().unwrap_or("").trim())
                                .filter(|value| !value.is_empty())
                            {
                                response.headers_mut().insert(
                                    "sec-websocket-protocol",
                                    protocol.parse().expect("valid subprotocol"),
                                );
                            }
                        }
                        records.lock().unwrap().push(HandshakeRecord {
                            path_and_query,
                            headers,
                        });
                        Ok(response)
                    },
                )
                .await;
                let Ok(ws_stream) = ws_stream else {
                    return;
                };

                let (mut writer, mut reader) = ws_stream.split();
                if writer
                    .send(Message::Text("upstream-ready".to_string().into()))
                    .await
                    .is_err()
                {
                    return;
                }

                while let Some(message) = reader.next().await {
                    let Ok(message) = message else {
                        break;
                    };
                    match message {
                        Message::Text(text) => {
                            if writer.send(Message::Text(text)).await.is_err() {
                                break;
                            }
                        }
                        Message::Binary(bytes) => {
                            if writer.send(Message::Binary(bytes)).await.is_err() {
                                break;
                            }
                        }
                        Message::Close(frame) => {
                            let _ = writer.send(Message::Close(frame)).await;
                            break;
                        }
                        Message::Ping(payload) => {
                            if writer.send(Message::Pong(payload)).await.is_err() {
                                break;
                            }
                        }
                        Message::Pong(_) => {}
                        Message::Frame(_) => {}
                    }
                }
            });
        }
    });

    (format!("http://{}", addr), records)
}

async fn spawn_rejecting_ws_upstream(
    status: StatusCode,
    error_code: &str,
    error_message: &str,
) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let payload = serde_json::json!({
        "error": {
            "code": error_code,
            "message": error_message,
        }
    })
    .to_string();

    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(value) => value,
                Err(_) => break,
            };
            let payload = payload.clone();
            tokio::spawn(async move {
                let _ = accept_hdr_async(
                    stream,
                    |_request: &tokio_tungstenite::tungstenite::handshake::server::Request,
                     _response: tokio_tungstenite::tungstenite::handshake::server::Response|
                     -> Result<
                        tokio_tungstenite::tungstenite::handshake::server::Response,
                        ErrorResponse,
                    > {
                        let response = http::Response::builder()
                            .status(status)
                            .header("content-type", "application/json")
                            .body(Some(payload.clone()))
                            .expect("reject websocket handshake");
                        Err(response)
                    },
                )
                .await;
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

#[tokio::test]
async fn ws_upgrade_v2_responses_forwards_beta_header() {
    let (upstream_base, records) = spawn_ws_upstream().await;
    let data_plane_base =
        spawn_data_plane_server(vec![test_account(upstream_base, "upstream-token")]).await;

    let mut request = ws_url(&data_plane_base, "/v1/responses")
        .into_client_request()
        .unwrap();
    request.headers_mut().insert(
        "openai-beta",
        "responses_websockets=2026-02-06".parse().unwrap(),
    );

    let (mut ws_client, response) = connect_async(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
    ws_client.close(None).await.unwrap();

    let records = records.lock().unwrap().clone();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].header("openai-beta"),
        Some("responses_websockets=2026-02-06")
    );
}

#[tokio::test]
async fn ws_upgrade_v1_responses_forwards_headers_and_frames() {
    let (upstream_base, records) = spawn_ws_upstream().await;
    let data_plane_base =
        spawn_data_plane_server(vec![test_account(upstream_base, "upstream-token")]).await;

    let mut request = ws_url(&data_plane_base, "/v1/responses?trace=1")
        .into_client_request()
        .unwrap();
    request
        .headers_mut()
        .insert("authorization", "Bearer tenant-token".parse().unwrap());
    request
        .headers_mut()
        .insert("session_id", "session-abc".parse().unwrap());
    request
        .headers_mut()
        .insert("x-codex-turn-state", "turn-state-1".parse().unwrap());
    request
        .headers_mut()
        .insert("x-codex-turn-metadata", "meta-1".parse().unwrap());
    request.headers_mut().insert(
        "openai-beta",
        "responses_websockets=2026-02-04".parse().unwrap(),
    );
    request
        .headers_mut()
        .insert("x-openai-subagent", "review".parse().unwrap());

    let (mut ws_client, response) = connect_async(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

    let first = ws_client.next().await.unwrap().unwrap();
    assert_eq!(first, Message::Text("upstream-ready".to_string().into()));

    ws_client
        .send(Message::Text("hello-from-client".to_string().into()))
        .await
        .unwrap();
    let echoed_text = ws_client.next().await.unwrap().unwrap();
    assert_eq!(
        echoed_text,
        Message::Text("hello-from-client".to_string().into())
    );

    ws_client
        .send(Message::Binary(vec![1, 2, 3, 4].into()))
        .await
        .unwrap();
    let echoed_binary = ws_client.next().await.unwrap().unwrap();
    assert_eq!(echoed_binary, Message::Binary(vec![1, 2, 3, 4].into()));

    ws_client.close(None).await.unwrap();
    if let Some(message) = ws_client.next().await {
        assert!(matches!(message.unwrap(), Message::Close(_)));
    }

    let records = records.lock().unwrap().clone();
    assert_eq!(records.len(), 1);
    let first_record = &records[0];
    assert_eq!(first_record.path_and_query, "/v1/responses?trace=1");
    assert_eq!(
        first_record.header("authorization"),
        Some("Bearer upstream-token")
    );
    assert_eq!(first_record.header("chatgpt-account-id"), Some("acct_123"));
    assert_eq!(
        first_record.header("openai-beta"),
        Some("responses_websockets=2026-02-04")
    );
    assert_eq!(first_record.header("x-openai-subagent"), Some("review"));
    assert_eq!(first_record.header("session_id"), Some("session-abc"));
    assert_eq!(
        first_record.header("x-codex-turn-state"),
        Some("turn-state-1")
    );
    assert_eq!(first_record.header("x-codex-turn-metadata"), Some("meta-1"));
}

#[tokio::test]
async fn ws_upgrade_supports_backend_api_codex_responses_path() {
    let (upstream_base, records) = spawn_ws_upstream().await;
    let data_plane_base =
        spawn_data_plane_server(vec![test_account(upstream_base, "upstream-token")]).await;

    let mut request = ws_url(&data_plane_base, "/backend-api/codex/responses")
        .into_client_request()
        .unwrap();
    request.headers_mut().insert(
        "openai-beta",
        "responses_websockets=2026-02-04".parse().unwrap(),
    );

    let (mut ws_client, response) = connect_async(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

    let first = ws_client.next().await.unwrap().unwrap();
    assert_eq!(first, Message::Text("upstream-ready".to_string().into()));
    ws_client.close(None).await.unwrap();

    let records = records.lock().unwrap().clone();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].path_and_query, "/backend-api/codex/responses");
}

#[tokio::test]
async fn ws_upgrade_propagates_selected_subprotocol() {
    let (upstream_base, _records) = spawn_ws_upstream().await;
    let data_plane_base =
        spawn_data_plane_server(vec![test_account(upstream_base, "upstream-token")]).await;

    let mut request = ws_url(&data_plane_base, "/v1/responses")
        .into_client_request()
        .unwrap();
    request.headers_mut().insert(
        "sec-websocket-protocol",
        "responses-stream-v2".parse().unwrap(),
    );

    let (mut ws_client, response) = connect_async(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
    assert_eq!(
        response
            .headers()
            .get("sec-websocket-protocol")
            .and_then(|value| value.to_str().ok()),
        Some("responses-stream-v2")
    );
    ws_client.close(None).await.unwrap();
}

#[tokio::test]
async fn ws_upgrade_falls_back_when_upstream_omits_subprotocol() {
    let (upstream_base, records) = spawn_ws_upstream_with_subprotocol_echo(false).await;
    let data_plane_base =
        spawn_data_plane_server(vec![test_account(upstream_base, "upstream-token")]).await;

    let mut request = ws_url(&data_plane_base, "/v1/responses")
        .into_client_request()
        .unwrap();
    request.headers_mut().insert(
        "sec-websocket-protocol",
        "responses-stream-v2".parse().unwrap(),
    );

    let (mut ws_client, response) = connect_async(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
    assert_eq!(
        response
            .headers()
            .get("sec-websocket-protocol")
            .and_then(|value| value.to_str().ok()),
        Some("responses-stream-v2")
    );
    let first = ws_client.next().await.unwrap().unwrap();
    assert_eq!(first, Message::Text("upstream-ready".to_string().into()));
    ws_client.close(None).await.unwrap();

    let records = records.lock().unwrap().clone();
    assert_eq!(records.len(), 2);
    assert_eq!(
        records[0].header("sec-websocket-protocol"),
        Some("responses-stream-v2")
    );
    assert!(records[1].header("sec-websocket-protocol").is_none());
}

#[tokio::test]
async fn ws_upgrade_returns_structured_error_when_upstream_connect_fails() {
    let data_plane_base = spawn_data_plane_server(vec![test_account(
        "http://127.0.0.1:1".to_string(),
        "upstream-token",
    )])
    .await;

    let request = ws_url(&data_plane_base, "/v1/responses")
        .into_client_request()
        .unwrap();

    let err = connect_async(request)
        .await
        .expect_err("handshake should fail");
    let response = match err {
        tokio_tungstenite::tungstenite::Error::Http(response) => response,
        other => panic!("expected http response error, got {other:?}"),
    };

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let payload: Value = serde_json::from_slice(response.body().as_deref().unwrap()).unwrap();
    assert_eq!(payload["error"]["code"], "upstream_websocket_connect_error");
}

#[tokio::test]
async fn ws_upgrade_propagates_upgrade_required_for_http_fallback() {
    let upstream_base = spawn_rejecting_ws_upstream(
        StatusCode::UPGRADE_REQUIRED,
        "websocket_upgrade_required",
        "upstream requires websocket upgrade",
    )
    .await;
    let data_plane_base =
        spawn_data_plane_server(vec![test_account(upstream_base, "upstream-token")]).await;

    let request = ws_url(&data_plane_base, "/v1/responses")
        .into_client_request()
        .unwrap();

    let err = connect_async(request)
        .await
        .expect_err("handshake should fail");
    let response = match err {
        tokio_tungstenite::tungstenite::Error::Http(response) => response,
        other => panic!("expected http response error, got {other:?}"),
    };

    assert_eq!(response.status(), StatusCode::UPGRADE_REQUIRED);
    let payload: Value = serde_json::from_slice(response.body().as_deref().unwrap()).unwrap();
    assert_eq!(payload["error"]["code"], "websocket_upgrade_required");
}

#[tokio::test]
async fn ws_upgrade_normalizes_upstream_auth_handshake_error() {
    let upstream_base = spawn_rejecting_ws_upstream(
        StatusCode::UNAUTHORIZED,
        "token_expired",
        "access token expired",
    )
    .await;
    let data_plane_base =
        spawn_data_plane_server(vec![test_account(upstream_base, "upstream-token")]).await;

    let request = ws_url(&data_plane_base, "/v1/responses")
        .into_client_request()
        .unwrap();

    let err = connect_async(request)
        .await
        .expect_err("handshake should fail");
    let response = match err {
        tokio_tungstenite::tungstenite::Error::Http(response) => response,
        other => panic!("expected http response error, got {other:?}"),
    };

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let payload: Value = serde_json::from_slice(response.body().as_deref().unwrap()).unwrap();
    assert_eq!(payload["error"]["code"], "auth_expired");
}

#[tokio::test]
async fn ws_upgrade_fails_over_to_next_account_before_handshake_success() {
    let (upstream_base, records) = spawn_ws_upstream().await;
    let data_plane_base = spawn_data_plane_server(vec![
        test_account("http://127.0.0.1:1".to_string(), "upstream-token-a"),
        test_account(upstream_base, "upstream-token-b"),
    ])
    .await;

    let request = ws_url(&data_plane_base, "/v1/responses")
        .into_client_request()
        .unwrap();

    let (mut ws_client, response) = connect_async(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

    let first = ws_client.next().await.unwrap().unwrap();
    assert_eq!(first, Message::Text("upstream-ready".to_string().into()));

    ws_client
        .send(Message::Text("ping".to_string().into()))
        .await
        .unwrap();
    let echoed = ws_client.next().await.unwrap().unwrap();
    assert_eq!(echoed, Message::Text("ping".to_string().into()));

    ws_client.close(None).await.unwrap();

    let records = records.lock().unwrap().clone();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].header("authorization"),
        Some("Bearer upstream-token-b")
    );
}

#[tokio::test]
async fn ws_upgrade_account_deactivated_triggers_disable_and_failover() {
    let failing_upstream_base = spawn_rejecting_ws_upstream(
        StatusCode::UNAUTHORIZED,
        "account_deactivated",
        "account is deactivated",
    )
    .await;
    let (healthy_upstream_base, records) = spawn_ws_upstream().await;
    let account_a = test_account(failing_upstream_base, "upstream-token-a");
    let account_b = test_account(healthy_upstream_base, "upstream-token-b");

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
    let disable_path = format!("/internal/v1/upstream-accounts/{}/disable", account_a.id);
    Mock::given(method("POST"))
        .and(path(disable_path.clone()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status":"disabled"})))
        .mount(&control_plane)
        .await;

    let data_plane_base = spawn_data_plane_server_with_control_plane(
        vec![account_a, account_b],
        Some(control_plane.uri()),
    )
    .await;

    let mut request = ws_url(&data_plane_base, "/v1/responses")
        .into_client_request()
        .unwrap();
    request
        .headers_mut()
        .insert("authorization", "Bearer cp_identity".parse().unwrap());

    let (mut ws_client, response) = connect_async(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
    let first = ws_client.next().await.unwrap().unwrap();
    assert_eq!(first, Message::Text("upstream-ready".to_string().into()));
    ws_client.close(None).await.unwrap();

    let records = records.lock().unwrap().clone();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].header("authorization"),
        Some("Bearer upstream-token-b")
    );

    let mut disable_called = false;
    for _ in 0..30 {
        let requests = control_plane.received_requests().await.unwrap();
        if requests
            .iter()
            .any(|req| req.method.as_str() == "POST" && req.url.path() == disable_path)
        {
            disable_called = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    assert!(
        disable_called,
        "expected disable endpoint to be called for deactivated account"
    );
}

#[tokio::test]
async fn ws_upgrade_continues_failover_until_untried_candidates_exhausted() {
    let failing_upstream_a = spawn_rejecting_ws_upstream(
        StatusCode::UNAUTHORIZED,
        "account_deactivated",
        "account A is deactivated",
    )
    .await;
    let failing_upstream_b = spawn_rejecting_ws_upstream(
        StatusCode::UNAUTHORIZED,
        "account_deactivated",
        "account B is deactivated",
    )
    .await;
    let (healthy_upstream_base, records) = spawn_ws_upstream().await;

    let data_plane_base = spawn_data_plane_server_with_control_plane_and_failover_wait(
        vec![
            test_account(failing_upstream_a, "upstream-token-a"),
            test_account(failing_upstream_b, "upstream-token-b"),
            test_account(healthy_upstream_base, "upstream-token-c"),
        ],
        None,
        1,
    )
    .await;

    let request = ws_url(&data_plane_base, "/v1/responses")
        .into_client_request()
        .unwrap();
    let (mut ws_client, response) = connect_async(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
    let first = ws_client.next().await.unwrap().unwrap();
    assert_eq!(first, Message::Text("upstream-ready".to_string().into()));
    ws_client.close(None).await.unwrap();

    let records = records.lock().unwrap().clone();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].header("authorization"),
        Some("Bearer upstream-token-c")
    );
}
