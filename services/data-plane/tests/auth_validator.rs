use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::Router;
use codex_pool_core::model::{RoutingStrategy, UpstreamAccount, UpstreamMode};
use data_plane::app::build_app_with_event_sink_and_allowed_keys as dp_build_app_with_event_sink_and_allowed_keys;
use data_plane::config::DataPlaneConfig;
use data_plane::event::NoopEventSink;
use http::Request;
use http::StatusCode;
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support;

async fn build_app_with_event_sink_and_allowed_keys(
    config: DataPlaneConfig,
    event_sink: Arc<NoopEventSink>,
    allowed_keys: Vec<String>,
) -> anyhow::Result<Router> {
    support::ensure_test_security_env().await;
    dp_build_app_with_event_sink_and_allowed_keys(config, event_sink, allowed_keys).await
}

fn test_account(base_url: String, token: &str) -> UpstreamAccount {
    UpstreamAccount {
        id: Uuid::new_v4(),
        label: "acc-validator".to_string(),
        mode: UpstreamMode::OpenAiApiKey,
        base_url,
        bearer_token: token.to_string(),
        chatgpt_account_id: None,
        enabled: true,
        priority: 100,
        created_at: chrono::Utc::now(),
    }
}

async fn send_authorized_request(app: &Router, token: &str) -> StatusCode {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

async fn build_test_app_with_auth_validator(
    upstream_url: String,
    auth_validate_url: String,
    auth_fail_open: bool,
    allowed_keys: Vec<String>,
    auth_validate_negative_cache_ttl_sec: u64,
) -> Router {
    build_app_with_event_sink_and_allowed_keys(
        DataPlaneConfig {
            listen_addr: "127.0.0.1:0".parse().unwrap(),
            routing_strategy: RoutingStrategy::RoundRobin,
            upstream_accounts: vec![test_account(upstream_url, "upstream-token")],
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
            auth_validate_url: Some(auth_validate_url),
            auth_validate_cache_ttl_sec: 1,
            auth_validate_negative_cache_ttl_sec,
            auth_fail_open,
            enable_internal_debug_routes: false,
        },
        Arc::new(NoopEventSink),
        allowed_keys,
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn auth_validator_caches_principal_within_ttl() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("{}", "application/json"))
        .mount(&upstream)
        .await;

    let control_plane = MockServer::start().await;
    let tenant_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": tenant_id,
            "api_key_id": api_key_id,
            "enabled": true,
            "cache_ttl_sec": 1
        })))
        .mount(&control_plane)
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
            auth_validate_url: Some(format!("{}/internal/v1/auth/validate", control_plane.uri())),
            auth_validate_cache_ttl_sec: 1,
            auth_validate_negative_cache_ttl_sec: 5,
            auth_fail_open: false,
            enable_internal_debug_routes: false,
        },
        Arc::new(NoopEventSink),
        Vec::new(),
    )
    .await
    .unwrap();

    assert_eq!(
        send_authorized_request(&app, "cp_cache").await,
        StatusCode::OK
    );
    assert_eq!(
        send_authorized_request(&app, "cp_cache").await,
        StatusCode::OK
    );

    let validate_calls = control_plane
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|req| req.url.path() == "/internal/v1/auth/validate")
        .count();
    assert_eq!(validate_calls, 1);
}

#[tokio::test]
async fn auth_validator_revalidates_after_ttl_expiry() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("{}", "application/json"))
        .mount(&upstream)
        .await;

    let control_plane = MockServer::start().await;
    let tenant_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": tenant_id,
            "api_key_id": api_key_id,
            "enabled": true,
            "cache_ttl_sec": 1
        })))
        .mount(&control_plane)
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
            auth_validate_url: Some(format!("{}/internal/v1/auth/validate", control_plane.uri())),
            auth_validate_cache_ttl_sec: 1,
            auth_validate_negative_cache_ttl_sec: 5,
            auth_fail_open: false,
            enable_internal_debug_routes: false,
        },
        Arc::new(NoopEventSink),
        Vec::new(),
    )
    .await
    .unwrap();

    assert_eq!(
        send_authorized_request(&app, "cp_expire").await,
        StatusCode::OK
    );
    tokio::time::sleep(Duration::from_millis(1100)).await;
    assert_eq!(
        send_authorized_request(&app, "cp_expire").await,
        StatusCode::OK
    );

    let validate_calls = control_plane
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|req| req.url.path() == "/internal/v1/auth/validate")
        .count();
    assert_eq!(validate_calls, 2);
}

#[tokio::test]
async fn auth_validator_5xx_returns_503_when_fail_open_disabled() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("{}", "application/json"))
        .mount(&upstream)
        .await;

    let auth_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(503).set_body_raw("validator down", "text/plain"))
        .mount(&auth_server)
        .await;

    let app = build_test_app_with_auth_validator(
        upstream.uri(),
        format!("{}/internal/v1/auth/validate", auth_server.uri()),
        false,
        vec!["cp_allow".to_string()],
        5,
    )
    .await;

    assert_eq!(
        send_authorized_request(&app, "cp_allow").await,
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn auth_validator_5xx_falls_back_to_allowlist_when_fail_open_enabled() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("{}", "application/json"))
        .mount(&upstream)
        .await;

    let auth_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(503).set_body_raw("validator down", "text/plain"))
        .mount(&auth_server)
        .await;

    let app = build_test_app_with_auth_validator(
        upstream.uri(),
        format!("{}/internal/v1/auth/validate", auth_server.uri()),
        true,
        vec!["cp_allow".to_string()],
        5,
    )
    .await;

    assert_eq!(
        send_authorized_request(&app, "cp_allow").await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn auth_validator_5xx_fallback_denies_when_allowlist_misses() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("{}", "application/json"))
        .mount(&upstream)
        .await;

    let auth_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(503).set_body_raw("validator down", "text/plain"))
        .mount(&auth_server)
        .await;

    let app = build_test_app_with_auth_validator(
        upstream.uri(),
        format!("{}/internal/v1/auth/validate", auth_server.uri()),
        true,
        vec!["cp_allow".to_string()],
        5,
    )
    .await;

    assert_eq!(
        send_authorized_request(&app, "cp_deny").await,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn auth_validator_5xx_fallback_allows_when_allowlist_empty() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("{}", "application/json"))
        .mount(&upstream)
        .await;

    let auth_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(503).set_body_raw("validator down", "text/plain"))
        .mount(&auth_server)
        .await;

    let app = build_test_app_with_auth_validator(
        upstream.uri(),
        format!("{}/internal/v1/auth/validate", auth_server.uri()),
        true,
        Vec::new(),
        5,
    )
    .await;

    assert_eq!(
        send_authorized_request(&app, "cp_any").await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn auth_validator_unauthorized_still_returns_401_even_when_fail_open_enabled() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("{}", "application/json"))
        .mount(&upstream)
        .await;

    let auth_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&auth_server)
        .await;

    let app = build_test_app_with_auth_validator(
        upstream.uri(),
        format!("{}/internal/v1/auth/validate", auth_server.uri()),
        true,
        Vec::new(),
        5,
    )
    .await;

    assert_eq!(
        send_authorized_request(&app, "cp_unauthorized").await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn auth_validator_caches_unauthorized_within_negative_ttl() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("{}", "application/json"))
        .mount(&upstream)
        .await;

    let auth_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&auth_server)
        .await;

    let app = build_test_app_with_auth_validator(
        upstream.uri(),
        format!("{}/internal/v1/auth/validate", auth_server.uri()),
        false,
        Vec::new(),
        1,
    )
    .await;

    assert_eq!(
        send_authorized_request(&app, "cp_neg_cache").await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        send_authorized_request(&app, "cp_neg_cache").await,
        StatusCode::UNAUTHORIZED
    );

    let validate_calls = auth_server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|req| req.url.path() == "/internal/v1/auth/validate")
        .count();
    assert_eq!(validate_calls, 1);
}

#[tokio::test]
async fn auth_validator_revalidates_unauthorized_after_negative_ttl_expiry() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("{}", "application/json"))
        .mount(&upstream)
        .await;

    let auth_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&auth_server)
        .await;

    let app = build_test_app_with_auth_validator(
        upstream.uri(),
        format!("{}/internal/v1/auth/validate", auth_server.uri()),
        false,
        Vec::new(),
        1,
    )
    .await;

    assert_eq!(
        send_authorized_request(&app, "cp_neg_expire").await,
        StatusCode::UNAUTHORIZED
    );
    tokio::time::sleep(Duration::from_millis(1100)).await;
    assert_eq!(
        send_authorized_request(&app, "cp_neg_expire").await,
        StatusCode::UNAUTHORIZED
    );

    let validate_calls = auth_server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|req| req.url.path() == "/internal/v1/auth/validate")
        .count();
    assert_eq!(validate_calls, 2);
}

#[tokio::test]
async fn auth_validator_5xx_is_not_negative_cached() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("{}", "application/json"))
        .mount(&upstream)
        .await;

    let auth_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(503).set_body_raw("validator down", "text/plain"))
        .mount(&auth_server)
        .await;

    let app = build_test_app_with_auth_validator(
        upstream.uri(),
        format!("{}/internal/v1/auth/validate", auth_server.uri()),
        false,
        Vec::new(),
        30,
    )
    .await;

    assert_eq!(
        send_authorized_request(&app, "cp_5xx").await,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        send_authorized_request(&app, "cp_5xx").await,
        StatusCode::SERVICE_UNAVAILABLE
    );

    let validate_calls = auth_server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|req| req.url.path() == "/internal/v1/auth/validate")
        .count();
    assert_eq!(validate_calls, 2);
}

#[tokio::test]
async fn auth_validator_network_error_is_not_negative_cached() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("{}", "application/json"))
        .mount(&upstream)
        .await;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let auth_server_addr = listener.local_addr().unwrap();
    drop(listener);

    let app = build_test_app_with_auth_validator(
        upstream.uri(),
        format!("http://{auth_server_addr}/internal/v1/auth/validate"),
        false,
        Vec::new(),
        30,
    )
    .await;

    assert_eq!(
        send_authorized_request(&app, "cp_network_error").await,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        send_authorized_request(&app, "cp_network_error").await,
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn auth_validator_rejects_disabled_api_key() {
    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("{}", "application/json"))
        .mount(&upstream)
        .await;

    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": Uuid::new_v4(),
            "api_key_id": Uuid::new_v4(),
            "enabled": false,
            "cache_ttl_sec": 30
        })))
        .mount(&control_plane)
        .await;

    let app = build_test_app_with_auth_validator(
        upstream.uri(),
        format!("{}/internal/v1/auth/validate", control_plane.uri()),
        false,
        Vec::new(),
        5,
    )
    .await;

    assert_eq!(
        send_authorized_request(&app, "cp_disabled").await,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn auth_validator_sends_internal_service_token_header() {
    support::ensure_test_security_env().await;
    let internal_token =
        std::env::var("CONTROL_PLANE_INTERNAL_AUTH_TOKEN").expect("internal auth token env");
    let expected_auth_header = format!("Bearer {internal_token}");

    let upstream = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("{}", "application/json"))
        .mount(&upstream)
        .await;

    let control_plane = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/internal/v1/auth/validate"))
        .and(header("authorization", expected_auth_header))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tenant_id": Uuid::new_v4(),
            "api_key_id": Uuid::new_v4(),
            "enabled": true,
            "cache_ttl_sec": 30
        })))
        .mount(&control_plane)
        .await;

    let app = build_test_app_with_auth_validator(
        upstream.uri(),
        format!("{}/internal/v1/auth/validate", control_plane.uri()),
        false,
        Vec::new(),
        5,
    )
    .await;

    assert_eq!(
        send_authorized_request(&app, "cp_internal").await,
        StatusCode::OK
    );
}
