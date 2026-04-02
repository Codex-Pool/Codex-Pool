use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use axum::body::Body;
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use codex_pool_core::api::{
    DataPlaneSnapshot, DataPlaneSnapshotEvent, DataPlaneSnapshotEventType,
    DataPlaneSnapshotEventsResponse,
};
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
    dp_build_app_with_event_sink(config, event_sink).await
}

#[derive(Clone)]
struct SnapshotSource {
    revision: Arc<AtomicU64>,
    cursor: Arc<AtomicU64>,
    accounts: Arc<RwLock<Vec<UpstreamAccount>>>,
    events: Arc<RwLock<Vec<DataPlaneSnapshotEvent>>>,
    cursor_gone_once: Arc<std::sync::atomic::AtomicBool>,
}

impl SnapshotSource {
    fn new(revision: u64, accounts: Vec<UpstreamAccount>) -> Self {
        Self {
            revision: Arc::new(AtomicU64::new(revision)),
            cursor: Arc::new(AtomicU64::new(0)),
            accounts: Arc::new(RwLock::new(accounts)),
            events: Arc::new(RwLock::new(Vec::new())),
            cursor_gone_once: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    fn update(&self, revision: u64, accounts: Vec<UpstreamAccount>) {
        let previous_accounts = {
            let mut guard = self.accounts.write().unwrap();
            let previous = guard.clone();
            *guard = accounts.clone();
            previous
        };
        self.revision.store(revision, Ordering::Relaxed);

        let next_ids: HashSet<Uuid> = accounts.iter().map(|account| account.id).collect();
        let mut new_events = Vec::new();

        for old_account in previous_accounts {
            if !next_ids.contains(&old_account.id) {
                new_events.push(DataPlaneSnapshotEvent {
                    id: 0,
                    event_type: DataPlaneSnapshotEventType::AccountDelete,
                    account_id: old_account.id,
                    account: None,
                    compiled_routing_plan: None,
                    ai_error_learning_settings: None,
                    approved_upstream_error_templates: None,
                    builtin_error_templates: None,
                    outbound_proxy_pool_settings: None,
                    outbound_proxy_nodes: None,
                    claude_code_routing_settings: None,
                    created_at: chrono::Utc::now(),
                });
            }
        }

        for account in accounts {
            new_events.push(DataPlaneSnapshotEvent {
                id: 0,
                event_type: DataPlaneSnapshotEventType::AccountUpsert,
                account_id: account.id,
                account: Some(account),
                compiled_routing_plan: None,
                ai_error_learning_settings: None,
                approved_upstream_error_templates: None,
                builtin_error_templates: None,
                outbound_proxy_pool_settings: None,
                outbound_proxy_nodes: None,
                claude_code_routing_settings: None,
                created_at: chrono::Utc::now(),
            });
        }

        if !new_events.is_empty() {
            let mut events = self.events.write().unwrap();
            for mut event in new_events {
                let id = self
                    .cursor
                    .fetch_add(1, Ordering::Relaxed)
                    .saturating_add(1);
                event.id = id;
                events.push(event);
            }
        }
    }

    fn snapshot(&self) -> DataPlaneSnapshot {
        DataPlaneSnapshot {
            revision: self.revision.load(Ordering::Relaxed),
            cursor: self.cursor.load(Ordering::Relaxed),
            accounts: self.accounts.read().unwrap().clone(),
            account_traits: Vec::new(),
            compiled_routing_plan: None,
            ai_error_learning_settings: Default::default(),
            approved_upstream_error_templates: Vec::new(),
            builtin_error_templates: Vec::new(),
            outbound_proxy_pool_settings: Default::default(),
            outbound_proxy_nodes: Vec::new(),
            claude_code_routing_settings: Default::default(),
            issued_at: chrono::Utc::now(),
        }
    }

    fn events(&self, after: u64, limit: u32) -> DataPlaneSnapshotEventsResponse {
        let limit = limit.clamp(1, 5_000) as usize;
        let events = self
            .events
            .read()
            .unwrap()
            .iter()
            .filter(|event| event.id > after)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let cursor = events
            .last()
            .map(|event| event.id)
            .unwrap_or_else(|| self.cursor.load(Ordering::Relaxed).max(after));

        DataPlaneSnapshotEventsResponse {
            cursor,
            high_watermark: self.cursor.load(Ordering::Relaxed),
            events,
        }
    }

    fn trigger_cursor_gone_once(&self) {
        self.cursor_gone_once.store(true, Ordering::Relaxed);
    }
}

fn account(label: &str, base_url: String, token: &str) -> UpstreamAccount {
    UpstreamAccount {
        id: Uuid::new_v4(),
        label: label.to_string(),
        mode: UpstreamMode::OpenAiApiKey,
        base_url,
        bearer_token: token.to_string(),
        chatgpt_account_id: None,
        enabled: true,
        priority: 100,
        created_at: chrono::Utc::now(),
    }
}

fn internal_service_token() -> String {
    std::env::var("CONTROL_PLANE_INTERNAL_AUTH_TOKEN")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "cp-internal-dev-token-change-me".to_string())
}

async fn start_snapshot_server(source: SnapshotSource) -> String {
    #[derive(serde::Deserialize)]
    struct SnapshotEventsQuery {
        after: Option<u64>,
        limit: Option<u32>,
    }

    async fn snapshot_handler(
        State(source): State<SnapshotSource>,
        headers: axum::http::HeaderMap,
    ) -> Result<Json<DataPlaneSnapshot>, http::StatusCode> {
        let expected = format!("Bearer {}", internal_service_token());
        let Some(actual) = headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
        else {
            return Err(http::StatusCode::UNAUTHORIZED);
        };
        if actual != expected {
            return Err(http::StatusCode::UNAUTHORIZED);
        }
        Ok(Json(source.snapshot()))
    }

    async fn snapshot_events_handler(
        State(source): State<SnapshotSource>,
        headers: axum::http::HeaderMap,
        axum::extract::Query(query): axum::extract::Query<SnapshotEventsQuery>,
    ) -> Result<Json<DataPlaneSnapshotEventsResponse>, http::StatusCode> {
        let expected = format!("Bearer {}", internal_service_token());
        let Some(actual) = headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
        else {
            return Err(http::StatusCode::UNAUTHORIZED);
        };
        if actual != expected {
            return Err(http::StatusCode::UNAUTHORIZED);
        }
        if source.cursor_gone_once.swap(false, Ordering::Relaxed) {
            return Err(http::StatusCode::GONE);
        }

        Ok(Json(source.events(
            query.after.unwrap_or(0),
            query.limit.unwrap_or(500),
        )))
    }

    let app = Router::new()
        .route("/api/v1/data-plane/snapshot", get(snapshot_handler))
        .route(
            "/api/v1/data-plane/snapshot/events",
            get(snapshot_events_handler),
        )
        .with_state(source);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    format!("http://{addr}")
}

async fn send_proxy_request(app: &Router) -> Value {
    let response = app
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

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn data_plane_refreshes_snapshot_then_routes_to_new_account() {
    let _env_guard = support::lock_env().await;
    let upstream_a = MockServer::start().await;
    let upstream_b = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(header("authorization", "Bearer token-a"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"account": "a"})))
        .mount(&upstream_a)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(header("authorization", "Bearer token-b"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"account": "b"})))
        .mount(&upstream_b)
        .await;

    let account_a = account("account-a", upstream_a.uri(), "token-a");
    let account_b = account("account-b", upstream_b.uri(), "token-b");

    let snapshot_source = SnapshotSource::new(1, vec![account_a.clone()]);
    let control_plane_base_url = start_snapshot_server(snapshot_source.clone()).await;

    std::env::set_var("CONTROL_PLANE_BASE_URL", &control_plane_base_url);
    std::env::set_var("SNAPSHOT_POLL_INTERVAL_MS", "50");
    std::env::set_var("SNAPSHOT_EVENTS_WAIT_MS", "50");

    let app = build_app_with_event_sink(
        DataPlaneConfig {
            listen_addr: "127.0.0.1:0".parse().unwrap(),
            routing_strategy: RoutingStrategy::RoundRobin,
            upstream_accounts: vec![account_a],
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
    .unwrap();

    let first = send_proxy_request(&app).await;
    assert_eq!(first["account"], "a");

    snapshot_source.update(2, vec![account_b]);
    tokio::time::sleep(Duration::from_millis(200)).await;

    let second = send_proxy_request(&app).await;
    assert_eq!(second["account"], "b");

    std::env::remove_var("CONTROL_PLANE_BASE_URL");
    std::env::remove_var("SNAPSHOT_POLL_INTERVAL_MS");
    std::env::remove_var("SNAPSHOT_EVENTS_WAIT_MS");
}

#[tokio::test(flavor = "current_thread")]
async fn data_plane_fallbacks_to_full_snapshot_when_events_cursor_is_gone() {
    let _env_guard = support::lock_env().await;
    let upstream_a = MockServer::start().await;
    let upstream_b = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(header("authorization", "Bearer token-a"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"account": "a"})))
        .mount(&upstream_a)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .and(header("authorization", "Bearer token-b"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"account": "b"})))
        .mount(&upstream_b)
        .await;

    let account_a = account("account-a", upstream_a.uri(), "token-a");
    let account_b = account("account-b", upstream_b.uri(), "token-b");

    let snapshot_source = SnapshotSource::new(1, vec![account_a.clone()]);
    let control_plane_base_url = start_snapshot_server(snapshot_source.clone()).await;

    std::env::set_var("CONTROL_PLANE_BASE_URL", &control_plane_base_url);
    std::env::set_var("SNAPSHOT_POLL_INTERVAL_MS", "50");
    std::env::set_var("SNAPSHOT_EVENTS_WAIT_MS", "50");

    let app = build_app_with_event_sink(
        DataPlaneConfig {
            listen_addr: "127.0.0.1:0".parse().unwrap(),
            routing_strategy: RoutingStrategy::RoundRobin,
            upstream_accounts: vec![account_a],
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
    .unwrap();

    let first = send_proxy_request(&app).await;
    assert_eq!(first["account"], "a");

    snapshot_source.update(2, vec![account_b]);
    snapshot_source.trigger_cursor_gone_once();
    tokio::time::sleep(Duration::from_millis(250)).await;

    let second = send_proxy_request(&app).await;
    assert_eq!(second["account"], "b");

    std::env::remove_var("CONTROL_PLANE_BASE_URL");
    std::env::remove_var("SNAPSHOT_POLL_INTERVAL_MS");
    std::env::remove_var("SNAPSHOT_EVENTS_WAIT_MS");
}
