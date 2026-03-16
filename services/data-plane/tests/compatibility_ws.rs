use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use axum::Router;
use codex_pool_core::events::RequestLogEvent;
use codex_pool_core::model::{RoutingStrategy, UpstreamAccount, UpstreamMode};
use data_plane::app::build_app_with_event_sink as dp_build_app_with_event_sink;
use data_plane::config::DataPlaneConfig;
use data_plane::event::{EventSink, NoopEventSink};
use futures_util::{SinkExt, StreamExt};
use http::StatusCode;
use reqwest::Client;
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
    event_sink: Arc<dyn EventSink>,
) -> anyhow::Result<Router> {
    support::ensure_test_security_env().await;
    dp_build_app_with_event_sink(config, event_sink).await
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

async fn spawn_data_plane_server_with_event_sink(
    accounts: Vec<UpstreamAccount>,
    event_sink: Arc<dyn EventSink>,
) -> String {
    let auth_validate_url = None;
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
        auth_validate_url,
        auth_validate_cache_ttl_sec: 30,
        auth_validate_negative_cache_ttl_sec: 5,
        auth_fail_open: false,
        enable_internal_debug_routes: false,
    };
    let app = build_app_with_event_sink(cfg, event_sink)
        .await
        .expect("app should build");
    spawn_data_plane_server_with_app(app).await
}

async fn spawn_data_plane_server_with_control_plane_and_event_sink(
    accounts: Vec<UpstreamAccount>,
    control_plane_base_url: Option<String>,
    event_sink: Arc<dyn EventSink>,
) -> String {
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
    };
    let app = build_app_with_event_sink(cfg, event_sink)
        .await
        .expect("app should build");
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

async fn spawn_ws_logical_usage_upstream() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(value) => value,
                Err(_) => break,
            };
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
                let Ok(ws_stream) = ws_stream else {
                    return;
                };

                let (mut writer, mut reader) = ws_stream.split();
                let mut response_index = 0usize;

                while let Some(message) = reader.next().await {
                    let Ok(message) = message else {
                        break;
                    };
                    match message {
                        Message::Text(text) => {
                            let Ok(value) = serde_json::from_str::<Value>(&text) else {
                                continue;
                            };
                            let is_create = value
                                .get("type")
                                .and_then(Value::as_str)
                                .map(|item| item == "response.create")
                                .unwrap_or(false);
                            if !is_create {
                                continue;
                            }

                            response_index += 1;
                            let response_id = format!("resp-{response_index}");
                            let model = value
                                .get("response")
                                .and_then(|item| item.get("model"))
                                .and_then(Value::as_str)
                                .unwrap_or("gpt-5.4");
                            let created = json!({
                                "type": "response.created",
                                "response": {
                                    "id": response_id,
                                    "model": model,
                                }
                            });
                            let completed = json!({
                                "type": "response.completed",
                                "response": {
                                    "id": response_id,
                                    "model": model,
                                    "usage": {
                                        "input_tokens": 17,
                                        "output_tokens": 9
                                    }
                                }
                            });
                            if writer
                                .send(Message::Text(created.to_string().into()))
                                .await
                                .is_err()
                            {
                                break;
                            }
                            if writer
                                .send(Message::Text(completed.to_string().into()))
                                .await
                                .is_err()
                            {
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
                        Message::Binary(_) | Message::Pong(_) | Message::Frame(_) => {}
                    }
                }
            });
        }
    });

    format!("http://{}", addr)
}

async fn spawn_ws_scripted_upstream(turns: Vec<Vec<Value>>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let turn_scripts = Arc::new(Mutex::new(VecDeque::from(turns)));

    tokio::spawn({
        let turn_scripts = turn_scripts.clone();
        async move {
            loop {
                let (stream, _) = match listener.accept().await {
                    Ok(value) => value,
                    Err(_) => break,
                };
                let turn_scripts = turn_scripts.clone();
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
                    let Ok(ws_stream) = ws_stream else {
                        return;
                    };

                    let (mut writer, mut reader) = ws_stream.split();
                    while let Some(message) = reader.next().await {
                        let Ok(message) = message else {
                            break;
                        };
                        match message {
                            Message::Text(text) => {
                                let Ok(value) = serde_json::from_str::<Value>(&text) else {
                                    continue;
                                };
                                let is_create = value
                                    .get("type")
                                    .and_then(Value::as_str)
                                    .map(|item| item == "response.create")
                                    .unwrap_or(false);
                                if !is_create {
                                    continue;
                                }

                                let scripted_events =
                                    turn_scripts.lock().unwrap().pop_front().unwrap_or_default();
                                for event in scripted_events {
                                    if writer
                                        .send(Message::Text(event.to_string().into()))
                                        .await
                                        .is_err()
                                    {
                                        return;
                                    }
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
                            Message::Binary(_) | Message::Pong(_) | Message::Frame(_) => {}
                        }
                    }
                });
            }
        }
    });

    format!("http://{}", addr)
}

async fn spawn_ws_text_scripted_upstream(turns: Vec<Vec<String>>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let turn_scripts = Arc::new(Mutex::new(VecDeque::from(turns)));

    tokio::spawn({
        let turn_scripts = turn_scripts.clone();
        async move {
            loop {
                let (stream, _) = match listener.accept().await {
                    Ok(value) => value,
                    Err(_) => break,
                };
                let turn_scripts = turn_scripts.clone();
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
                    let Ok(ws_stream) = ws_stream else {
                        return;
                    };

                    let (mut writer, mut reader) = ws_stream.split();
                    while let Some(message) = reader.next().await {
                        let Ok(message) = message else {
                            break;
                        };
                        match message {
                            Message::Text(text) => {
                                let Ok(value) = serde_json::from_str::<Value>(&text) else {
                                    continue;
                                };
                                let is_create = value
                                    .get("type")
                                    .and_then(Value::as_str)
                                    .map(|item| item == "response.create")
                                    .unwrap_or(false);
                                if !is_create {
                                    continue;
                                }

                                let scripted_events =
                                    turn_scripts.lock().unwrap().pop_front().unwrap_or_default();
                                for event in scripted_events {
                                    if writer.send(Message::Text(event.into())).await.is_err() {
                                        return;
                                    }
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
                            Message::Binary(_) | Message::Pong(_) | Message::Frame(_) => {}
                        }
                    }
                });
            }
        }
    });

    format!("http://{}", addr)
}

async fn spawn_ws_scripted_upstream_and_close(turns: Vec<Vec<Value>>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let turn_scripts = Arc::new(Mutex::new(VecDeque::from(turns)));

    tokio::spawn({
        let turn_scripts = turn_scripts.clone();
        async move {
            loop {
                let (stream, _) = match listener.accept().await {
                    Ok(value) => value,
                    Err(_) => break,
                };
                let turn_scripts = turn_scripts.clone();
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
                    let Ok(ws_stream) = ws_stream else {
                        return;
                    };

                    let (mut writer, mut reader) = ws_stream.split();
                    while let Some(message) = reader.next().await {
                        let Ok(message) = message else {
                            break;
                        };
                        match message {
                            Message::Text(text) => {
                                let Ok(value) = serde_json::from_str::<Value>(&text) else {
                                    continue;
                                };
                                let is_create = value
                                    .get("type")
                                    .and_then(Value::as_str)
                                    .map(|item| item == "response.create")
                                    .unwrap_or(false);
                                if !is_create {
                                    continue;
                                }

                                let scripted_events =
                                    turn_scripts.lock().unwrap().pop_front().unwrap_or_default();
                                for event in scripted_events {
                                    if writer
                                        .send(Message::Text(event.to_string().into()))
                                        .await
                                        .is_err()
                                    {
                                        return;
                                    }
                                }
                                let _ = writer.close().await;
                                break;
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
                            Message::Binary(_) | Message::Pong(_) | Message::Frame(_) => {}
                        }
                    }
                });
            }
        }
    });

    format!("http://{}", addr)
}

async fn spawn_ws_turn_state_required_upstream(expected_turn_text: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(value) => value,
                Err(_) => break,
            };
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
                let Ok(ws_stream) = ws_stream else {
                    return;
                };

                let (mut writer, mut reader) = ws_stream.split();
                let mut saw_expected_turn = false;

                while let Some(message) = reader.next().await {
                    let Ok(message) = message else {
                        break;
                    };
                    match message {
                        Message::Text(text) => {
                            let Ok(value) = serde_json::from_str::<Value>(&text) else {
                                continue;
                            };
                            let Some(event_type) = value.get("type").and_then(Value::as_str) else {
                                continue;
                            };

                            match event_type {
                                "conversation.item.create" => {
                                    saw_expected_turn = value
                                        .get("item")
                                        .and_then(|item| item.get("content"))
                                        .and_then(Value::as_array)
                                        .and_then(|items| items.first())
                                        .and_then(|item| item.get("text"))
                                        .and_then(Value::as_str)
                                        == Some(expected_turn_text);
                                }
                                "response.create" => {
                                    if !saw_expected_turn {
                                        let error = json!({
                                            "type": "error",
                                            "error": {
                                                "code": "missing_turn_state",
                                                "message": "missing latest turn state before response.create"
                                            }
                                        });
                                        let _ = writer
                                            .send(Message::Text(error.to_string().into()))
                                            .await;
                                        continue;
                                    }

                                    let created = json!({
                                        "type": "response.created",
                                        "response": { "id": "resp-turn-2", "model": "gpt-5.4" }
                                    });
                                    let completed = json!({
                                        "type": "response.completed",
                                        "response": {
                                            "id": "resp-turn-2",
                                            "model": "gpt-5.4",
                                            "usage": { "input_tokens": 8, "output_tokens": 4 }
                                        }
                                    });
                                    if writer
                                        .send(Message::Text(created.to_string().into()))
                                        .await
                                        .is_err()
                                    {
                                        return;
                                    }
                                    if writer
                                        .send(Message::Text(completed.to_string().into()))
                                        .await
                                        .is_err()
                                    {
                                        return;
                                    }
                                }
                                _ => {}
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
                        Message::Binary(_) | Message::Pong(_) | Message::Frame(_) => {}
                    }
                }
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
async fn ws_logical_usage_emits_one_event_per_completed_response() {
    let upstream_base = spawn_ws_logical_usage_upstream().await;
    let sink = Arc::new(RecordingSink::default());
    let data_plane_base = spawn_data_plane_server_with_event_sink(
        vec![test_account(upstream_base, "upstream-token")],
        sink.clone(),
    )
    .await;

    let request = ws_url(&data_plane_base, "/v1/responses")
        .into_client_request()
        .unwrap();
    let mut request = request;
    request
        .headers_mut()
        .insert("authorization", "Bearer cp_identity".parse().unwrap());
    let (mut ws_client, response) = connect_async(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

    ws_client
        .send(Message::Text(
            json!({
                "type": "response.create",
                "request_id": "req-1",
                "response": { "model": "gpt-5.4" }
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();
    ws_client
        .send(Message::Text(
            json!({
                "type": "response.create",
                "request_id": "req-2",
                "response": { "model": "gpt-5.4-mini" }
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();

    for _ in 0..4 {
        let message = ws_client.next().await.unwrap().unwrap();
        assert!(matches!(message, Message::Text(_)));
    }

    let events = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let events = sink.events();
            if events.len() == 2 {
                return events;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("events should arrive");

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].request_id.as_deref(), Some("req-1"));
    assert_eq!(events[0].model.as_deref(), Some("gpt-5.4"));
    assert_eq!(events[0].input_tokens, Some(17));
    assert_eq!(events[0].output_tokens, Some(9));
    assert_eq!(
        events[0].billing_phase.as_deref(),
        Some("ws_response_completed")
    );
    assert_eq!(events[1].request_id.as_deref(), Some("req-2"));
    assert_eq!(events[1].model.as_deref(), Some("gpt-5.4-mini"));

    ws_client.close(None).await.unwrap();
}

#[tokio::test]
async fn ws_billing_completed_response_authorizes_and_captures() {
    let authorization_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let upstream_base = spawn_ws_scripted_upstream(vec![vec![
        json!({
            "type": "response.created",
            "response": { "id": "resp-1", "model": "gpt-5.4" }
        }),
        json!({
            "type": "response.completed",
            "response": {
                "id": "resp-1",
                "model": "gpt-5.4",
                "usage": { "input_tokens": 17, "output_tokens": 9 }
            }
        }),
    ]])
    .await;

    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": tenant_id,
            "api_key_id": api_key_id,
            "enabled": true,
            "group": {
                "id": Uuid::new_v4(),
                "name": "default",
                "invalid": false
            },
            "tenant_status": "active",
            "balance_microcredits": 1_000_000,
            "cache_ttl_sec": 30,
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/authorize"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorization_id": authorization_id,
            "status": "authorized",
            "reserved_microcredits": 2_000_000,
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
    let data_plane_base = spawn_data_plane_server_with_control_plane_and_event_sink(
        vec![test_account(upstream_base, "upstream-token")],
        Some(control_plane.uri()),
        sink.clone(),
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

    ws_client
        .send(Message::Text(
            json!({
                "type": "response.create",
                "request_id": "req-1",
                "response": { "model": "gpt-5.4" }
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();

    for _ in 0..2 {
        let message = ws_client.next().await.unwrap().unwrap();
        assert!(matches!(message, Message::Text(_)));
    }

    let (authorize_count, capture_count, release_count, capture_payload) =
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let requests = control_plane.received_requests().await.unwrap();
                let authorize_count = requests
                    .iter()
                    .filter(|request| request.url.path() == "/internal/v1/billing/authorize")
                    .count();
                let capture_count = requests
                    .iter()
                    .filter(|request| request.url.path() == "/internal/v1/billing/capture")
                    .count();
                let release_count = requests
                    .iter()
                    .filter(|request| request.url.path() == "/internal/v1/billing/release")
                    .count();
                let capture_payload = requests
                    .iter()
                    .find(|request| request.url.path() == "/internal/v1/billing/capture")
                    .map(|request| serde_json::from_slice::<Value>(&request.body).unwrap());
                if authorize_count == 1 && capture_count == 1 {
                    return (
                        authorize_count,
                        capture_count,
                        release_count,
                        capture_payload,
                    );
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("billing requests should arrive");

    assert_eq!(authorize_count, 1);
    assert_eq!(capture_count, 1);
    assert_eq!(release_count, 1);
    let capture_payload = capture_payload.expect("capture payload should exist");
    assert!(capture_payload["request_id"].as_str().is_some());
    assert_eq!(capture_payload["model"], "gpt-5.4");
    assert_eq!(capture_payload["input_tokens"], 17);
    assert_eq!(capture_payload["output_tokens"], 9);
    assert_eq!(capture_payload["is_stream"], true);

    let events = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let events = sink.events();
            if events.len() == 1 {
                return events;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("billing event should arrive");

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].request_id.as_deref(), Some("req-1"));
    assert_eq!(events[0].authorization_id, Some(authorization_id));
    assert_eq!(events[0].capture_status.as_deref(), Some("captured"));
    assert_eq!(events[0].billing_phase.as_deref(), Some("released"));
    assert_eq!(events[0].input_tokens, Some(17));
    assert_eq!(events[0].output_tokens, Some(9));

    ws_client.close(None).await.unwrap();
}

#[tokio::test]
async fn ws_billing_uses_priority_service_tier_for_pricing_and_logs_effective_tier() {
    let authorization_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let upstream_base = spawn_ws_scripted_upstream(vec![vec![
        json!({
            "type": "response.created",
            "response": { "id": "resp-tier-1", "model": "gpt-5.4" }
        }),
        json!({
            "type": "response.completed",
            "response": {
                "id": "resp-tier-1",
                "model": "gpt-5.4",
                "service_tier": "priority",
                "usage": { "input_tokens": 17, "output_tokens": 9 }
            }
        }),
    ]])
    .await;

    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": tenant_id,
            "api_key_id": api_key_id,
            "enabled": true,
            "group": {
                "id": Uuid::new_v4(),
                "name": "default",
                "invalid": false
            },
            "tenant_status": "active",
            "balance_microcredits": 1_000_000,
            "cache_ttl_sec": 30,
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/pricing"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "model": "gpt-5.4",
            "input_price_microcredits": 2_000_000,
            "cached_input_price_microcredits": 200_000,
            "output_price_microcredits": 8_000_000,
            "source": "priority_exact"
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/authorize"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorization_id": authorization_id,
            "status": "authorized",
            "reserved_microcredits": 2_000_000,
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
    let codex_profile_base = format!("{}/backend-api/codex", upstream_base.trim_end_matches('/'));
    let data_plane_base = spawn_data_plane_server_with_control_plane_and_event_sink(
        vec![test_account(codex_profile_base, "upstream-token")],
        Some(control_plane.uri()),
        sink.clone(),
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

    ws_client
        .send(Message::Text(
            json!({
                "type": "response.create",
                "request_id": "req-tier-1",
                "response": {
                    "model": "gpt-5.4",
                    "service_tier": "fast"
                }
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();

    for _ in 0..2 {
        let message = ws_client.next().await.unwrap().unwrap();
        assert!(matches!(message, Message::Text(_)));
    }

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
    .expect("pricing request and final billing event should arrive");

    let pricing_payload = pricing_payload.expect("pricing payload should exist");
    assert_eq!(pricing_payload["model"], "gpt-5.4");
    assert_eq!(pricing_payload["service_tier"], "priority");

    let final_event = events
        .last()
        .expect("expected final stream request log event");
    assert_eq!(final_event.request_id.as_deref(), Some("req-tier-1"));
    assert_eq!(final_event.authorization_id, Some(authorization_id));
    assert_eq!(final_event.service_tier.as_deref(), Some("priority"));
    assert_eq!(final_event.capture_status.as_deref(), Some("captured"));

    ws_client.close(None).await.unwrap();
}

#[tokio::test]
async fn ws_billing_incomplete_response_releases_without_capture() {
    let tenant_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let upstream_base = spawn_ws_scripted_upstream(vec![vec![
        json!({
            "type": "response.created",
            "response": { "id": "resp-1", "model": "gpt-5.4" }
        }),
        json!({
            "type": "response.incomplete",
            "response": { "id": "resp-1", "model": "gpt-5.4" }
        }),
    ]])
    .await;

    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": tenant_id,
            "api_key_id": api_key_id,
            "enabled": true,
            "group": {
                "id": Uuid::new_v4(),
                "name": "default",
                "invalid": false
            },
            "tenant_status": "active",
            "balance_microcredits": 1_000_000,
            "cache_ttl_sec": 30,
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/authorize"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorization_id": Uuid::new_v4(),
            "status": "authorized",
            "reserved_microcredits": 2_000_000,
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
    let data_plane_base = spawn_data_plane_server_with_control_plane_and_event_sink(
        vec![test_account(upstream_base, "upstream-token")],
        Some(control_plane.uri()),
        sink,
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

    ws_client
        .send(Message::Text(
            json!({
                "type": "response.create",
                "request_id": "req-incomplete",
                "response": { "model": "gpt-5.4" }
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();

    for _ in 0..2 {
        let message = ws_client.next().await.unwrap().unwrap();
        assert!(matches!(message, Message::Text(_)));
    }

    let (authorize_count, capture_count, release_count, release_payload) =
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let requests = control_plane.received_requests().await.unwrap();
                let authorize_count = requests
                    .iter()
                    .filter(|request| request.url.path() == "/internal/v1/billing/authorize")
                    .count();
                let capture_count = requests
                    .iter()
                    .filter(|request| request.url.path() == "/internal/v1/billing/capture")
                    .count();
                let release_count = requests
                    .iter()
                    .filter(|request| request.url.path() == "/internal/v1/billing/release")
                    .count();
                let release_payload = requests
                    .iter()
                    .find(|request| request.url.path() == "/internal/v1/billing/release")
                    .map(|request| serde_json::from_slice::<Value>(&request.body).unwrap());
                if authorize_count == 1 && release_count == 1 {
                    return (
                        authorize_count,
                        capture_count,
                        release_count,
                        release_payload,
                    );
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("release request should arrive");

    assert_eq!(authorize_count, 1);
    assert_eq!(capture_count, 0);
    assert_eq!(release_count, 1);
    let release_payload = release_payload.expect("release payload should exist");
    assert!(release_payload["request_id"].as_str().is_some());
    assert_eq!(release_payload["is_stream"], true);

    ws_client.close(None).await.unwrap();
}

#[tokio::test]
async fn ws_billing_upstream_close_releases_pending_authorization() {
    let tenant_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let upstream_base = spawn_ws_scripted_upstream_and_close(vec![vec![json!({
        "type": "response.created",
        "response": { "id": "resp-close-1", "model": "gpt-5.4" }
    })]])
    .await;

    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": tenant_id,
            "api_key_id": api_key_id,
            "enabled": true,
            "group": {
                "id": Uuid::new_v4(),
                "name": "default",
                "invalid": false
            },
            "tenant_status": "active",
            "balance_microcredits": 1_000_000,
            "cache_ttl_sec": 30,
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/authorize"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorization_id": Uuid::new_v4(),
            "status": "authorized",
            "reserved_microcredits": 2_000_000,
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

    let data_plane_base = spawn_data_plane_server_with_control_plane(
        vec![test_account(upstream_base, "upstream-token")],
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

    ws_client
        .send(Message::Text(
            json!({
                "type": "response.create",
                "request_id": "req-close-1",
                "response": { "model": "gpt-5.4" }
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();

    let first = tokio::time::timeout(Duration::from_secs(2), ws_client.next())
        .await
        .expect("first upstream message should arrive")
        .unwrap()
        .unwrap();
    assert!(matches!(first, Message::Text(_)));

    let (authorize_count, capture_count, release_count, release_payload) =
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let requests = control_plane.received_requests().await.unwrap();
                let authorize_count = requests
                    .iter()
                    .filter(|request| request.url.path() == "/internal/v1/billing/authorize")
                    .count();
                let capture_count = requests
                    .iter()
                    .filter(|request| request.url.path() == "/internal/v1/billing/capture")
                    .count();
                let release_count = requests
                    .iter()
                    .filter(|request| request.url.path() == "/internal/v1/billing/release")
                    .count();
                let release_payload = requests
                    .iter()
                    .find(|request| request.url.path() == "/internal/v1/billing/release")
                    .map(|request| serde_json::from_slice::<Value>(&request.body).unwrap());
                if authorize_count == 1 && release_count == 1 {
                    return (
                        authorize_count,
                        capture_count,
                        release_count,
                        release_payload,
                    );
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("release request should arrive after upstream close");

    assert_eq!(authorize_count, 1);
    assert_eq!(capture_count, 0);
    assert_eq!(release_count, 1);
    let release_payload = release_payload.expect("release payload should exist");
    assert!(release_payload["request_id"].as_str().is_some());
    assert_eq!(release_payload["release_reason"], "ws_upstream_closed");
}

#[tokio::test]
async fn ws_billing_response_failed_releases_without_capture() {
    let tenant_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let upstream_base = spawn_ws_scripted_upstream_and_close(vec![vec![
        json!({
            "type": "response.created",
            "response": { "id": "resp-failed-1", "model": "gpt-5.4" }
        }),
        json!({
            "type": "response.failed",
            "response": {
                "id": "resp-failed-1",
                "model": "gpt-5.4",
                "error": { "code": "invalid_request_error", "message": "synthetic ws failure" }
            }
        }),
    ]])
    .await;

    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": tenant_id,
            "api_key_id": api_key_id,
            "enabled": true,
            "group": {
                "id": Uuid::new_v4(),
                "name": "default",
                "invalid": false
            },
            "tenant_status": "active",
            "balance_microcredits": 1_000_000,
            "cache_ttl_sec": 30,
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/authorize"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorization_id": Uuid::new_v4(),
            "status": "authorized",
            "reserved_microcredits": 2_000_000,
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

    let data_plane_base = spawn_data_plane_server_with_control_plane(
        vec![test_account(upstream_base, "upstream-token")],
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

    ws_client
        .send(Message::Text(
            json!({
                "type": "response.create",
                "request_id": "req-failed-1",
                "response": { "model": "gpt-5.4" }
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();

    for _ in 0..2 {
        let message = tokio::time::timeout(Duration::from_secs(2), ws_client.next())
            .await
            .expect("failed response messages should arrive")
            .unwrap()
            .unwrap();
        assert!(matches!(message, Message::Text(_)));
    }
    let (authorize_count, capture_count, release_count, release_payload) =
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let requests = control_plane.received_requests().await.unwrap();
                let authorize_count = requests
                    .iter()
                    .filter(|request| request.url.path() == "/internal/v1/billing/authorize")
                    .count();
                let capture_count = requests
                    .iter()
                    .filter(|request| request.url.path() == "/internal/v1/billing/capture")
                    .count();
                let release_count = requests
                    .iter()
                    .filter(|request| request.url.path() == "/internal/v1/billing/release")
                    .count();
                let release_payload = requests
                    .iter()
                    .find(|request| request.url.path() == "/internal/v1/billing/release")
                    .map(|request| serde_json::from_slice::<Value>(&request.body).unwrap());
                if authorize_count == 1 && release_count == 1 {
                    return (
                        authorize_count,
                        capture_count,
                        release_count,
                        release_payload,
                    );
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("release request should arrive after response.failed");

    assert_eq!(authorize_count, 1);
    assert_eq!(capture_count, 0);
    assert_eq!(release_count, 1);
    let release_payload = release_payload.expect("release payload should exist");
    assert!(release_payload["request_id"].as_str().is_some());
    assert_eq!(release_payload["release_reason"], "ws_response_failed");
}

#[tokio::test]
async fn ws_billing_multiple_completed_responses_capture_twice() {
    let tenant_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let upstream_base = spawn_ws_scripted_upstream(vec![
        vec![
            json!({
                "type": "response.created",
                "response": { "id": "resp-1", "model": "gpt-5.4" }
            }),
            json!({
                "type": "response.completed",
                "response": {
                    "id": "resp-1",
                    "model": "gpt-5.4",
                    "usage": { "input_tokens": 11, "output_tokens": 7 }
                }
            }),
        ],
        vec![
            json!({
                "type": "response.created",
                "response": { "id": "resp-2", "model": "gpt-5.4-mini" }
            }),
            json!({
                "type": "response.completed",
                "response": {
                    "id": "resp-2",
                    "model": "gpt-5.4-mini",
                    "usage": { "input_tokens": 5, "output_tokens": 3 }
                }
            }),
        ],
    ])
    .await;

    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": tenant_id,
            "api_key_id": api_key_id,
            "enabled": true,
            "group": {
                "id": Uuid::new_v4(),
                "name": "default",
                "invalid": false
            },
            "tenant_status": "active",
            "balance_microcredits": 1_000_000,
            "cache_ttl_sec": 30,
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/authorize"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorization_id": Uuid::new_v4(),
            "status": "authorized",
            "reserved_microcredits": 2_000_000,
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
    let data_plane_base = spawn_data_plane_server_with_control_plane_and_event_sink(
        vec![test_account(upstream_base, "upstream-token")],
        Some(control_plane.uri()),
        sink.clone(),
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

    ws_client
        .send(Message::Text(
            json!({
                "type": "response.create",
                "request_id": "req-1",
                "response": { "model": "gpt-5.4" }
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();
    ws_client
        .send(Message::Text(
            json!({
                "type": "response.create",
                "request_id": "req-2",
                "response": { "model": "gpt-5.4-mini" }
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();

    for _ in 0..4 {
        let message = ws_client.next().await.unwrap().unwrap();
        assert!(matches!(message, Message::Text(_)));
    }

    let (authorize_count, capture_count, release_count) =
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let requests = control_plane.received_requests().await.unwrap();
                let authorize_count = requests
                    .iter()
                    .filter(|request| request.url.path() == "/internal/v1/billing/authorize")
                    .count();
                let capture_count = requests
                    .iter()
                    .filter(|request| request.url.path() == "/internal/v1/billing/capture")
                    .count();
                let release_count = requests
                    .iter()
                    .filter(|request| request.url.path() == "/internal/v1/billing/release")
                    .count();
                if authorize_count == 2 && capture_count == 2 {
                    return (authorize_count, capture_count, release_count);
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("two captures should arrive");

    assert_eq!(authorize_count, 2);
    assert_eq!(capture_count, 2);
    assert_eq!(release_count, 2);

    let events = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let events = sink.events();
            if events.len() == 2 {
                return events;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("billing events should arrive");

    assert_eq!(events.len(), 2);
    assert!(events.iter().all(|event| event.authorization_id.is_some()));
    assert!(events
        .iter()
        .all(|event| event.capture_status.as_deref() == Some("captured")));

    ws_client.close(None).await.unwrap();
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
async fn ws_upgrade_v1_responses_maps_fast_service_tier_to_priority_for_codex_profile() {
    let (upstream_base, _records) = spawn_ws_upstream().await;
    let codex_profile_base = format!("{}/backend-api/codex", upstream_base.trim_end_matches('/'));
    let data_plane_base =
        spawn_data_plane_server(vec![test_account(codex_profile_base, "upstream-token")]).await;

    let request = ws_url(&data_plane_base, "/v1/responses")
        .into_client_request()
        .unwrap();
    let (mut ws_client, response) = connect_async(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

    let first = ws_client.next().await.unwrap().unwrap();
    assert_eq!(first, Message::Text("upstream-ready".to_string().into()));

    ws_client
        .send(Message::Text(
            json!({
                "type": "response.create",
                "model": "gpt-5.4",
                "service_tier": "fast",
                "input": []
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();

    let echoed_text = ws_client.next().await.unwrap().unwrap();
    let echoed_payload = match echoed_text {
        Message::Text(text) => serde_json::from_str::<Value>(text.as_ref()).unwrap(),
        other => panic!("expected echoed text frame, got {other:?}"),
    };

    assert_eq!(echoed_payload["service_tier"], "priority");

    ws_client.close(None).await.unwrap();
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
async fn ws_session_retries_same_request_on_new_account_before_any_output() {
    let quota_upstream_base = spawn_ws_scripted_upstream(vec![vec![json!({
        "type": "error",
        "request_id": "req-retry",
        "error": {
            "code": "usage_limit",
            "message": "You've hit your usage limit. Upgrade to Plus to continue using Codex"
        }
    })]])
    .await;
    let healthy_upstream_base = spawn_ws_scripted_upstream(vec![vec![
        json!({
            "type": "response.created",
            "response": { "id": "resp-retry", "model": "gpt-5.4" }
        }),
        json!({
            "type": "response.completed",
            "response": {
                "id": "resp-retry",
                "model": "gpt-5.4",
                "usage": { "input_tokens": 8, "output_tokens": 4 }
            }
        }),
    ]])
    .await;

    let data_plane_base = spawn_data_plane_server(vec![
        test_account(quota_upstream_base, "upstream-token-a"),
        test_account(healthy_upstream_base, "upstream-token-b"),
    ])
    .await;

    let request = ws_url(&data_plane_base, "/v1/responses")
        .into_client_request()
        .unwrap();
    let mut request = request;
    request
        .headers_mut()
        .insert("authorization", "Bearer cp_identity".parse().unwrap());
    let (mut ws_client, response) = connect_async(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

    ws_client
        .send(Message::Text(
            json!({
                "type": "response.create",
                "request_id": "req-retry",
                "response": { "model": "gpt-5.4" }
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();

    let (first, second) = tokio::time::timeout(Duration::from_secs(2), async {
        let first = ws_client.next().await.unwrap().unwrap();
        let second = ws_client.next().await.unwrap().unwrap();
        (first, second)
    })
    .await
    .expect("same request should be retried on a new account");

    let first_text = match first {
        Message::Text(text) => text.to_string(),
        other => panic!("expected first text message, got {other:?}"),
    };
    let second_text = match second {
        Message::Text(text) => text.to_string(),
        other => panic!("expected second text message, got {other:?}"),
    };
    let first_payload: Value = serde_json::from_str(&first_text).unwrap();
    let second_payload: Value = serde_json::from_str(&second_text).unwrap();

    assert_eq!(first_payload["type"], "response.created");
    assert_eq!(second_payload["type"], "response.completed");
    assert_eq!(second_payload["response"]["id"], "resp-retry");

    ws_client.close(None).await.unwrap();
}

#[tokio::test]
async fn ws_session_rebind_uses_new_billing_request_id_after_releasing_old_authorization() {
    let tenant_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let quota_upstream_base = spawn_ws_scripted_upstream(vec![vec![json!({
        "type": "error",
        "request_id": "req-rebind-billing",
        "error": {
            "code": "usage_limit",
            "message": "You have hit your usage limit. Upgrade to Plus to continue using Codex"
        }
    })]])
    .await;
    let healthy_upstream_base = spawn_ws_scripted_upstream(vec![vec![
        json!({
            "type": "response.created",
            "response": { "id": "resp-rebind-billing", "model": "gpt-5.4" }
        }),
        json!({
            "type": "response.completed",
            "response": {
                "id": "resp-rebind-billing",
                "model": "gpt-5.4",
                "usage": { "input_tokens": 8, "output_tokens": 4 }
            }
        }),
    ]])
    .await;

    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": tenant_id,
            "api_key_id": api_key_id,
            "enabled": true,
            "group": {
                "id": Uuid::new_v4(),
                "name": "default",
                "invalid": false
            },
            "tenant_status": "active",
            "balance_microcredits": 1_000_000,
            "cache_ttl_sec": 30,
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/authorize"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorization_id": Uuid::new_v4(),
            "status": "authorized",
            "reserved_microcredits": 2_000_000,
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

    let data_plane_base = spawn_data_plane_server_with_control_plane(
        vec![
            test_account(quota_upstream_base, "upstream-token-a"),
            test_account(healthy_upstream_base, "upstream-token-b"),
        ],
        Some(control_plane.uri()),
    )
    .await;

    let request = ws_url(&data_plane_base, "/v1/responses")
        .into_client_request()
        .unwrap();
    let mut request = request;
    request
        .headers_mut()
        .insert("authorization", "Bearer cp_identity".parse().unwrap());
    let (mut ws_client, response) = connect_async(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

    ws_client
        .send(Message::Text(
            json!({
                "type": "response.create",
                "request_id": "req-rebind-billing",
                "response": { "model": "gpt-5.4" }
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();

    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        let first = ws_client.next().await.unwrap().unwrap();
        let second = ws_client.next().await.unwrap().unwrap();
        (first, second)
    })
    .await
    .expect("rebind flow should complete");

    let (authorize_payloads, capture_payloads, release_payloads) =
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let requests = control_plane.received_requests().await.unwrap();
                let authorize_payloads = requests
                    .iter()
                    .filter(|request| request.url.path() == "/internal/v1/billing/authorize")
                    .map(|request| serde_json::from_slice::<Value>(&request.body).unwrap())
                    .collect::<Vec<_>>();
                let capture_payloads = requests
                    .iter()
                    .filter(|request| request.url.path() == "/internal/v1/billing/capture")
                    .map(|request| serde_json::from_slice::<Value>(&request.body).unwrap())
                    .collect::<Vec<_>>();
                let release_payloads = requests
                    .iter()
                    .filter(|request| request.url.path() == "/internal/v1/billing/release")
                    .map(|request| serde_json::from_slice::<Value>(&request.body).unwrap())
                    .collect::<Vec<_>>();
                if authorize_payloads.len() == 2
                    && capture_payloads.len() == 1
                    && release_payloads.len() == 2
                {
                    return (authorize_payloads, capture_payloads, release_payloads);
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("billing requests should reflect old release plus new capture/release");

    assert_ne!(
        authorize_payloads[0]["request_id"],
        authorize_payloads[1]["request_id"]
    );
    assert_eq!(
        capture_payloads[0]["request_id"],
        authorize_payloads[1]["request_id"]
    );
    assert!(release_payloads
        .iter()
        .any(|payload| payload["release_reason"] == "ws_retry_rebind"));
    assert!(release_payloads
        .iter()
        .any(|payload| payload["release_reason"].is_null()));

    ws_client.close(None).await.unwrap();
}

#[tokio::test]
async fn ws_session_retries_plain_text_auth_error_on_new_account_before_any_output() {
    let auth_upstream_base = spawn_ws_text_scripted_upstream(vec![vec![
        "Failed to extract accountId from token".to_string(),
    ]])
    .await;
    let healthy_upstream_base = spawn_ws_scripted_upstream(vec![vec![
        json!({
            "type": "response.created",
            "response": { "id": "resp-retry-text", "model": "gpt-5.4" }
        }),
        json!({
            "type": "response.completed",
            "response": {
                "id": "resp-retry-text",
                "model": "gpt-5.4",
                "usage": { "input_tokens": 8, "output_tokens": 4 }
            }
        }),
    ]])
    .await;

    let data_plane_base = spawn_data_plane_server(vec![
        test_account(auth_upstream_base, "upstream-token-a"),
        test_account(healthy_upstream_base, "upstream-token-b"),
    ])
    .await;

    let request = ws_url(&data_plane_base, "/v1/responses")
        .into_client_request()
        .unwrap();
    let mut request = request;
    request
        .headers_mut()
        .insert("authorization", "Bearer cp_identity".parse().unwrap());
    let (mut ws_client, response) = connect_async(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

    ws_client
        .send(Message::Text(
            json!({
                "type": "response.create",
                "request_id": "req-retry-text",
                "response": { "model": "gpt-5.4" }
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();

    let (first, second) = tokio::time::timeout(Duration::from_secs(2), async {
        let first = ws_client.next().await.unwrap().unwrap();
        let second = ws_client.next().await.unwrap().unwrap();
        (first, second)
    })
    .await
    .expect("same request should be retried on a new account");

    let first_text = match first {
        Message::Text(text) => text.to_string(),
        other => panic!("expected first text message, got {other:?}"),
    };
    let second_text = match second {
        Message::Text(text) => text.to_string(),
        other => panic!("expected second text message, got {other:?}"),
    };
    let first_payload: Value = serde_json::from_str(&first_text).unwrap();
    let second_payload: Value = serde_json::from_str(&second_text).unwrap();

    assert_eq!(first_payload["type"], "response.created");
    assert_eq!(second_payload["type"], "response.completed");
    assert_eq!(second_payload["response"]["id"], "resp-retry-text");

    ws_client.close(None).await.unwrap();
}

#[tokio::test]
async fn ws_session_retries_same_request_without_request_id_on_new_account_before_any_output() {
    let quota_upstream_base = spawn_ws_scripted_upstream(vec![vec![json!({
        "type": "error",
        "error": {
            "code": "usage_limit",
            "message": "You've hit your usage limit. Upgrade to Plus to continue using Codex"
        }
    })]])
    .await;
    let healthy_upstream_base = spawn_ws_scripted_upstream(vec![vec![
        json!({
            "type": "response.created",
            "response": { "id": "resp-retry-no-id", "model": "gpt-5.4" }
        }),
        json!({
            "type": "response.completed",
            "response": {
                "id": "resp-retry-no-id",
                "model": "gpt-5.4",
                "usage": { "input_tokens": 8, "output_tokens": 4 }
            }
        }),
    ]])
    .await;

    let data_plane_base = spawn_data_plane_server(vec![
        test_account(quota_upstream_base, "upstream-token-a"),
        test_account(healthy_upstream_base, "upstream-token-b"),
    ])
    .await;

    let request = ws_url(&data_plane_base, "/v1/responses")
        .into_client_request()
        .unwrap();
    let (mut ws_client, response) = connect_async(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

    ws_client
        .send(Message::Text(
            json!({
                "type": "response.create",
                "response": { "model": "gpt-5.4" }
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();

    let (first, second) = tokio::time::timeout(Duration::from_secs(2), async {
        let first = ws_client.next().await.unwrap().unwrap();
        let second = ws_client.next().await.unwrap().unwrap();
        (first, second)
    })
    .await
    .expect("same request should retry on a new account even without request_id in error");

    let first_text = match first {
        Message::Text(text) => text.to_string(),
        other => panic!("expected first text message, got {other:?}"),
    };
    let second_text = match second {
        Message::Text(text) => text.to_string(),
        other => panic!("expected second text message, got {other:?}"),
    };
    let first_payload: Value = serde_json::from_str(&first_text).unwrap();
    let second_payload: Value = serde_json::from_str(&second_text).unwrap();

    assert_eq!(first_payload["type"], "response.created");
    assert_eq!(second_payload["type"], "response.completed");
    assert_eq!(second_payload["response"]["id"], "resp-retry-no-id");

    ws_client.close(None).await.unwrap();
}

#[tokio::test]
async fn ws_session_replays_required_turn_frames_before_retrying_response_create() {
    let quota_upstream_base = spawn_ws_scripted_upstream(vec![vec![json!({
        "type": "error",
        "request_id": "req-turn-2",
        "error": {
            "code": "usage_limit",
            "message": "You've hit your usage limit. Upgrade to Plus to continue using Codex"
        }
    })]])
    .await;
    let healthy_upstream_base = spawn_ws_turn_state_required_upstream("turn-2").await;

    let data_plane_base = spawn_data_plane_server(vec![
        test_account(quota_upstream_base, "upstream-token-a"),
        test_account(healthy_upstream_base, "upstream-token-b"),
    ])
    .await;

    let request = ws_url(&data_plane_base, "/v1/responses")
        .into_client_request()
        .unwrap();
    let (mut ws_client, response) = connect_async(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

    ws_client
        .send(Message::Text(
            json!({
                "type": "conversation.item.create",
                "item": {
                    "type": "message",
                    "role": "user",
                    "content": [{ "type": "input_text", "text": "turn-2" }]
                }
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();
    ws_client
        .send(Message::Text(
            json!({
                "type": "response.create",
                "request_id": "req-turn-2",
                "response": { "model": "gpt-5.4" }
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();

    let (first, second) = tokio::time::timeout(Duration::from_secs(2), async {
        let first = ws_client.next().await.unwrap().unwrap();
        let second = ws_client.next().await.unwrap().unwrap();
        (first, second)
    })
    .await
    .expect("retry should replay required turn frames before response.create");

    let first_text = match first {
        Message::Text(text) => text.to_string(),
        other => panic!("expected first text message, got {other:?}"),
    };
    let second_text = match second {
        Message::Text(text) => text.to_string(),
        other => panic!("expected second text message, got {other:?}"),
    };
    let first_payload: Value = serde_json::from_str(&first_text).unwrap();
    let second_payload: Value = serde_json::from_str(&second_text).unwrap();

    assert_eq!(first_payload["type"], "response.created");
    assert_eq!(second_payload["type"], "response.completed");
    assert_eq!(second_payload["response"]["id"], "resp-turn-2");

    ws_client.close(None).await.unwrap();
}

#[tokio::test]
async fn ws_session_rotates_account_after_usage_limit_error_for_followup_requests() {
    let quota_upstream_base = spawn_ws_scripted_upstream(vec![vec![json!({
        "type": "error",
        "request_id": "req-1",
        "error": {
            "code": "usage_limit",
            "message": "You've hit your usage limit. Upgrade to Plus to continue using Codex"
        }
    })]])
    .await;
    let healthy_upstream_base = spawn_ws_scripted_upstream(vec![
        vec![
            json!({
                "type": "response.created",
                "response": { "id": "resp-1", "model": "gpt-5.4" }
            }),
            json!({
                "type": "response.completed",
                "response": {
                    "id": "resp-1",
                    "model": "gpt-5.4",
                    "usage": { "input_tokens": 5, "output_tokens": 3 }
                }
            }),
        ],
        vec![
            json!({
                "type": "response.created",
                "response": { "id": "resp-2", "model": "gpt-5.4" }
            }),
            json!({
                "type": "response.completed",
                "response": {
                    "id": "resp-2",
                    "model": "gpt-5.4",
                    "usage": { "input_tokens": 7, "output_tokens": 4 }
                }
            }),
        ],
    ])
    .await;

    let data_plane_base = spawn_data_plane_server(vec![
        test_account(quota_upstream_base, "upstream-token-a"),
        test_account(healthy_upstream_base, "upstream-token-b"),
    ])
    .await;

    let request = ws_url(&data_plane_base, "/v1/responses")
        .into_client_request()
        .unwrap();
    let (mut ws_client, response) = connect_async(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

    ws_client
        .send(Message::Text(
            json!({
                "type": "response.create",
                "request_id": "req-1",
                "response": { "model": "gpt-5.4" }
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();

    let (first, second) = tokio::time::timeout(Duration::from_secs(2), async {
        let first = ws_client.next().await.unwrap().unwrap();
        let second = ws_client.next().await.unwrap().unwrap();
        (first, second)
    })
    .await
    .expect("first request should retry successfully on rotated account");
    let first_text = match first {
        Message::Text(text) => text.to_string(),
        other => panic!("expected first text message, got {other:?}"),
    };
    let second_text = match second {
        Message::Text(text) => text.to_string(),
        other => panic!("expected second text message, got {other:?}"),
    };
    let first_payload: Value = serde_json::from_str(&first_text).unwrap();
    let second_payload: Value = serde_json::from_str(&second_text).unwrap();
    assert_eq!(first_payload["type"], "response.created");
    assert_eq!(second_payload["type"], "response.completed");
    assert_eq!(second_payload["response"]["id"], "resp-1");

    ws_client
        .send(Message::Text(
            json!({
                "type": "response.create",
                "request_id": "req-2",
                "response": { "model": "gpt-5.4" }
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();

    let (third, fourth) = tokio::time::timeout(Duration::from_secs(2), async {
        let third = ws_client.next().await.unwrap().unwrap();
        let fourth = ws_client.next().await.unwrap().unwrap();
        (third, fourth)
    })
    .await
    .expect("follow-up request should stay on rotated account");

    let third_text = match third {
        Message::Text(text) => text.to_string(),
        other => panic!("expected third text message, got {other:?}"),
    };
    let fourth_text = match fourth {
        Message::Text(text) => text.to_string(),
        other => panic!("expected fourth text message, got {other:?}"),
    };
    let third_payload: Value = serde_json::from_str(&third_text).unwrap();
    let fourth_payload: Value = serde_json::from_str(&fourth_text).unwrap();
    assert_eq!(third_payload["type"], "response.created");
    assert_eq!(fourth_payload["type"], "response.completed");
    assert_eq!(fourth_payload["response"]["id"], "resp-2");

    ws_client.close(None).await.unwrap();
}

#[tokio::test]
async fn ws_session_does_not_rotate_on_previous_response_not_found_error() {
    let first_upstream_base = spawn_ws_scripted_upstream(vec![
        vec![json!({
            "type": "error",
            "request_id": "req-1",
            "error": {
                "code": "previous_response_not_found",
                "message": "previous response was not found"
            }
        })],
        vec![
            json!({
                "type": "response.created",
                "response": { "id": "resp-still-a", "model": "gpt-5.4" }
            }),
            json!({
                "type": "response.completed",
                "response": {
                    "id": "resp-still-a",
                    "model": "gpt-5.4",
                    "usage": { "input_tokens": 6, "output_tokens": 2 }
                }
            }),
        ],
    ])
    .await;
    let second_upstream_base = spawn_ws_scripted_upstream(vec![vec![
        json!({
            "type": "response.created",
            "response": { "id": "resp-rotated", "model": "gpt-5.4" }
        }),
        json!({
            "type": "response.completed",
            "response": {
                "id": "resp-rotated",
                "model": "gpt-5.4",
                "usage": { "input_tokens": 3, "output_tokens": 1 }
            }
        }),
    ]])
    .await;

    let data_plane_base = spawn_data_plane_server(vec![
        test_account(first_upstream_base, "upstream-token-a"),
        test_account(second_upstream_base, "upstream-token-b"),
    ])
    .await;

    let request = ws_url(&data_plane_base, "/v1/responses")
        .into_client_request()
        .unwrap();
    let (mut ws_client, response) = connect_async(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

    ws_client
        .send(Message::Text(
            json!({
                "type": "response.create",
                "request_id": "req-1",
                "response": { "model": "gpt-5.4" }
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();

    let first = tokio::time::timeout(Duration::from_secs(2), async {
        ws_client.next().await.unwrap().unwrap()
    })
    .await
    .expect("first request should surface the original upstream error");
    let first_text = match first {
        Message::Text(text) => text.to_string(),
        other => panic!("expected first text message, got {other:?}"),
    };
    let first_payload: Value = serde_json::from_str(&first_text).unwrap();
    assert_eq!(first_payload["type"], "error");
    assert_eq!(
        first_payload["error"]["code"],
        "previous_response_not_found"
    );

    ws_client
        .send(Message::Text(
            json!({
                "type": "response.create",
                "request_id": "req-2",
                "response": { "model": "gpt-5.4" }
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();

    let (second, third) = tokio::time::timeout(Duration::from_secs(2), async {
        let second = ws_client.next().await.unwrap().unwrap();
        let third = ws_client.next().await.unwrap().unwrap();
        (second, third)
    })
    .await
    .expect("follow-up request should continue on the original websocket session");

    let second_text = match second {
        Message::Text(text) => text.to_string(),
        other => panic!("expected second text message, got {other:?}"),
    };
    let third_text = match third {
        Message::Text(text) => text.to_string(),
        other => panic!("expected third text message, got {other:?}"),
    };
    let second_payload: Value = serde_json::from_str(&second_text).unwrap();
    let third_payload: Value = serde_json::from_str(&third_text).unwrap();
    assert_eq!(second_payload["type"], "response.created");
    assert_eq!(third_payload["type"], "response.completed");
    assert_eq!(third_payload["response"]["id"], "resp-still-a");

    ws_client.close(None).await.unwrap();
}

#[tokio::test]
async fn stream_failover_reauthorizes_and_attributes_final_event_to_second_account() {
    let first_upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/backend-api/codex/responses"))
        .respond_with(ResponseTemplate::new(503).set_body_json(json!({
            "error": { "code": "server_overloaded", "message": "busy" }
        })))
        .mount(&first_upstream)
        .await;

    let second_upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/backend-api/codex/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            concat!(
                "event: response.created\n",
                "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp-stream-failover\",\"model\":\"gpt-5.4\"}}\n\n",
                "event: response.completed\n",
                "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp-stream-failover\",\"model\":\"gpt-5.4\",\"usage\":{\"input_tokens\":17,\"output_tokens\":9}}}\n\n"
            ),
            "text/event-stream",
        ))
        .mount(&second_upstream)
        .await;

    let tenant_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": tenant_id,
            "api_key_id": api_key_id,
            "enabled": true,
            "group": {
                "id": Uuid::new_v4(),
                "name": "default",
                "invalid": false
            },
            "tenant_status": "active",
            "balance_microcredits": 1_000_000,
            "cache_ttl_sec": 30,
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/authorize"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorization_id": Uuid::new_v4(),
            "status": "authorized",
            "reserved_microcredits": 2_000_000,
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

    let first_account = test_account(
        format!("{}/backend-api/codex", first_upstream.uri()),
        "upstream-token-a",
    );
    let second_account = test_account(
        format!("{}/backend-api/codex", second_upstream.uri()),
        "upstream-token-b",
    );
    let second_account_id = second_account.id;
    let sink = Arc::new(RecordingSink::default());
    let data_plane_base = spawn_data_plane_server_with_control_plane_and_event_sink(
        vec![first_account, second_account],
        Some(control_plane.uri()),
        sink.clone(),
    )
    .await;

    let response = Client::new()
        .post(format!("{}/v1/responses", data_plane_base))
        .bearer_auth("cp_identity")
        .json(&json!({
            "model": "gpt-5.4",
            "stream": true,
            "input": "hello stream failover"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await.unwrap();
    assert!(body.contains("response.completed"));

    let (authorize_count, release_count, events) =
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let requests = control_plane.received_requests().await.unwrap();
                let authorize_count = requests
                    .iter()
                    .filter(|request| request.url.path() == "/internal/v1/billing/authorize")
                    .count();
                let release_count = requests
                    .iter()
                    .filter(|request| request.url.path() == "/internal/v1/billing/release")
                    .count();
                let events = sink.events();
                if authorize_count == 2 && !events.is_empty() {
                    return (authorize_count, release_count, events);
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("stream failover billing events should arrive");

    assert_eq!(authorize_count, 2);
    assert_eq!(release_count, 2);
    let final_event = events.last().expect("expected final stream event");
    assert_eq!(final_event.account_id, second_account_id);
    assert_eq!(final_event.billing_phase.as_deref(), Some("released"));
    assert_eq!(final_event.capture_status.as_deref(), Some("captured"));
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

#[tokio::test]
async fn ws_session_rebind_authorizes_fresh_billing_operation() {
    let tenant_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let quota_upstream_base = spawn_ws_scripted_upstream(vec![vec![json!({
        "type": "error",
        "request_id": "req-rebind-billing",
        "error": {
            "code": "usage_limit",
            "message": "You have hit your usage limit. Upgrade to Plus to continue using Codex"
        }
    })]])
    .await;
    let healthy_upstream_base = spawn_ws_scripted_upstream(vec![vec![
        json!({
            "type": "response.created",
            "response": { "id": "resp-rebind-billing", "model": "gpt-5.4" }
        }),
        json!({
            "type": "response.completed",
            "response": {
                "id": "resp-rebind-billing",
                "model": "gpt-5.4",
                "usage": { "input_tokens": 8, "output_tokens": 4 }
            }
        }),
    ]])
    .await;

    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": tenant_id,
            "api_key_id": api_key_id,
            "enabled": true,
            "group": {
                "id": Uuid::new_v4(),
                "name": "default",
                "invalid": false
            },
            "tenant_status": "active",
            "balance_microcredits": 1_000_000,
            "cache_ttl_sec": 30,
        })))
        .mount(&control_plane)
        .await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/billing/authorize"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorization_id": Uuid::new_v4(),
            "status": "authorized",
            "reserved_microcredits": 2_000_000,
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

    let data_plane_base = spawn_data_plane_server_with_control_plane(
        vec![
            test_account(quota_upstream_base, "upstream-token-a"),
            test_account(healthy_upstream_base, "upstream-token-b"),
        ],
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

    ws_client
        .send(Message::Text(
            json!({
                "type": "response.create",
                "request_id": "req-rebind-billing",
                "response": { "model": "gpt-5.4" }
            })
            .to_string()
            .into(),
        ))
        .await
        .unwrap();

    let (_first, _second) = tokio::time::timeout(Duration::from_secs(2), async {
        let first = ws_client.next().await.unwrap().unwrap();
        let second = ws_client.next().await.unwrap().unwrap();
        (first, second)
    })
    .await
    .expect("same request should be retried on a new account with billing enabled");

    let (authorize_payloads, capture_count, release_count) =
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let requests = control_plane.received_requests().await.unwrap();
                let authorize_payloads = requests
                    .iter()
                    .filter(|request| request.url.path() == "/internal/v1/billing/authorize")
                    .map(|request| serde_json::from_slice::<Value>(&request.body).unwrap())
                    .collect::<Vec<_>>();
                let capture_count = requests
                    .iter()
                    .filter(|request| request.url.path() == "/internal/v1/billing/capture")
                    .count();
                let release_count = requests
                    .iter()
                    .filter(|request| request.url.path() == "/internal/v1/billing/release")
                    .count();
                if authorize_payloads.len() == 2 && capture_count == 1 && release_count >= 2 {
                    return (authorize_payloads, capture_count, release_count);
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("billing replay requests should arrive");

    assert_eq!(capture_count, 1);
    assert!(release_count >= 2);
    assert_ne!(
        authorize_payloads[0]["request_id"],
        Value::String("req-rebind-billing".to_string())
    );
    assert_ne!(
        authorize_payloads[1]["request_id"],
        Value::String("req-rebind-billing".to_string())
    );
    assert_ne!(
        authorize_payloads[0]["request_id"],
        authorize_payloads[1]["request_id"]
    );

    ws_client.close(None).await.unwrap();
}
