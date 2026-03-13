use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use codex_pool_core::api::{
    AccountUsageLeaderboardItem, ApiKeyUsageLeaderboardItem, HourlyAccountUsagePoint,
    HourlyTenantApiKeyUsagePoint, HourlyTenantUsageTotalPoint, HourlyUsageTotalPoint,
    TenantUsageLeaderboardItem, UsageSummaryQueryResponse,
};
use control_plane::app::{
    build_app as cp_build_app,
    build_app_with_store_ttl_and_usage_repo as cp_build_app_with_store_ttl_and_usage_repo,
    DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC,
};
use control_plane::store::postgres::PostgresStore;
use control_plane::store::{ControlPlaneStore, InMemoryStore};
use control_plane::usage::clickhouse_repo::UsageQueryRepository;
use control_plane::usage::{RequestLogQuery, RequestLogRow};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use crate::support;

fn build_app() -> axum::Router {
    support::ensure_test_security_env();
    cp_build_app()
}

fn build_app_with_store_ttl_and_usage_repo(
    store: Arc<dyn ControlPlaneStore>,
    auth_validate_cache_ttl_sec: u64,
    usage_repo: Option<Arc<dyn UsageQueryRepository>>,
) -> axum::Router {
    support::ensure_test_security_env();
    cp_build_app_with_store_ttl_and_usage_repo(store, auth_validate_cache_ttl_sec, usage_repo)
}

fn test_db_url() -> Option<String> {
    std::env::var("CONTROL_PLANE_DATABASE_URL")
        .ok()
        .or_else(|| std::env::var("DATABASE_URL").ok())
}

async fn login_admin_token(app: &axum::Router) -> String {
    let login_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "username": "admin",
                        "password": "admin123456"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login_response.status(), StatusCode::OK);
    let login_body = to_bytes(login_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let login_json: Value = serde_json::from_slice(&login_body).unwrap();
    login_json["access_token"].as_str().unwrap().to_string()
}

async fn register_verified_tenant_token(app: &axum::Router) -> (Uuid, String) {
    let suffix = Uuid::new_v4().simple().to_string();
    let email = format!("tenant-{suffix}@example.com");
    let password = "Password123!";
    let tenant_name = format!("tenant-{suffix}");

    let register_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tenant/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "tenant_name": tenant_name,
                        "email": email,
                        "password": password,
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(register_response.status(), StatusCode::OK);
    let register_body = to_bytes(register_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let register_json: Value = serde_json::from_slice(&register_body).unwrap();
    let tenant_id = Uuid::parse_str(register_json["tenant_id"].as_str().unwrap()).unwrap();
    let debug_code = register_json["debug_code"]
        .as_str()
        .expect("tenant auth debug code should be exposed in tests");

    let verify_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tenant/auth/verify-email")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "email": email,
                        "code": debug_code,
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(verify_response.status(), StatusCode::NO_CONTENT);

    let login_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tenant/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "email": email,
                        "password": password,
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login_response.status(), StatusCode::OK);
    let login_body = to_bytes(login_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let login_json: Value = serde_json::from_slice(&login_body).unwrap();
    let access_token = login_json["access_token"]
        .as_str()
        .expect("tenant login should return access token")
        .to_string();
    (tenant_id, access_token)
}

#[derive(Clone)]
struct FakeUsageRepo {
    request_logs: Vec<RequestLogRow>,
    request_log_calls: Arc<Mutex<Vec<RequestLogQuery>>>,
}

#[async_trait]
impl UsageQueryRepository for FakeUsageRepo {
    async fn query_hourly_accounts(
        &self,
        _start_ts: i64,
        _end_ts: i64,
        _limit: u32,
        _account_id: Option<Uuid>,
    ) -> Result<Vec<HourlyAccountUsagePoint>> {
        Ok(Vec::new())
    }

    async fn query_hourly_tenant_api_keys(
        &self,
        _start_ts: i64,
        _end_ts: i64,
        _limit: u32,
        _tenant_id: Option<Uuid>,
        _api_key_id: Option<Uuid>,
    ) -> Result<Vec<HourlyTenantApiKeyUsagePoint>> {
        Ok(Vec::new())
    }

    async fn query_hourly_account_totals(
        &self,
        _start_ts: i64,
        _end_ts: i64,
        _limit: u32,
        _account_id: Option<Uuid>,
    ) -> Result<Vec<HourlyUsageTotalPoint>> {
        Ok(Vec::new())
    }

    async fn query_hourly_tenant_api_key_totals(
        &self,
        _start_ts: i64,
        _end_ts: i64,
        _limit: u32,
        _tenant_id: Option<Uuid>,
        _api_key_id: Option<Uuid>,
    ) -> Result<Vec<HourlyUsageTotalPoint>> {
        Ok(Vec::new())
    }

    async fn query_hourly_tenant_totals(
        &self,
        _start_ts: i64,
        _end_ts: i64,
        _limit: u32,
        _tenant_id: Option<Uuid>,
        _api_key_id: Option<Uuid>,
    ) -> Result<Vec<HourlyTenantUsageTotalPoint>> {
        Ok(Vec::new())
    }

    async fn query_summary(
        &self,
        _start_ts: i64,
        _end_ts: i64,
        _tenant_id: Option<Uuid>,
        _account_id: Option<Uuid>,
        _api_key_id: Option<Uuid>,
    ) -> Result<UsageSummaryQueryResponse> {
        Ok(UsageSummaryQueryResponse {
            start_ts: 0,
            end_ts: 0,
            account_total_requests: 0,
            tenant_api_key_total_requests: 0,
            unique_account_count: 0,
            unique_tenant_api_key_count: 0,
            estimated_cost_microusd: None,
            dashboard_metrics: None,
        })
    }

    async fn query_tenant_leaderboard(
        &self,
        _start_ts: i64,
        _end_ts: i64,
        _limit: u32,
        _tenant_id: Option<Uuid>,
    ) -> Result<Vec<TenantUsageLeaderboardItem>> {
        Ok(Vec::new())
    }

    async fn query_account_leaderboard(
        &self,
        _start_ts: i64,
        _end_ts: i64,
        _limit: u32,
        _account_id: Option<Uuid>,
    ) -> Result<Vec<AccountUsageLeaderboardItem>> {
        Ok(Vec::new())
    }

    async fn query_api_key_leaderboard(
        &self,
        _start_ts: i64,
        _end_ts: i64,
        _limit: u32,
        _tenant_id: Option<Uuid>,
        _api_key_id: Option<Uuid>,
    ) -> Result<Vec<ApiKeyUsageLeaderboardItem>> {
        Ok(Vec::new())
    }

    async fn query_request_logs(&self, query: RequestLogQuery) -> Result<Vec<RequestLogRow>> {
        self.request_log_calls.lock().unwrap().push(query);
        Ok(self.request_logs.clone())
    }
}

#[tokio::test]
async fn admin_request_logs_returns_503_when_usage_repo_unavailable() {
    let app = build_app();
    let admin_token = login_admin_token(&app).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/request-logs?start_ts=1700000000&end_ts=1700003600")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn admin_request_logs_forwards_filters_and_returns_rows() {
    let tenant_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let account_id = Uuid::new_v4();
    let row = RequestLogRow {
        id: Uuid::new_v4(),
        account_id,
        tenant_id: Some(tenant_id),
        api_key_id: Some(api_key_id),
        request_id: Some("req-1".to_string()),
        path: "/v1/responses".to_string(),
        method: "POST".to_string(),
        model: Some("gpt-5.3-codex".to_string()),
        service_tier: Some("priority".to_string()),
        input_tokens: Some(123),
        cached_input_tokens: Some(12),
        output_tokens: Some(456),
        reasoning_tokens: Some(30),
        first_token_latency_ms: Some(91),
        status_code: 500,
        latency_ms: 123,
        is_stream: false,
        error_code: Some("500".to_string()),
        billing_phase: Some("released".to_string()),
        authorization_id: Some(Uuid::new_v4()),
        capture_status: Some("captured".to_string()),
        estimated_cost_microusd: None,
        created_at: chrono::Utc::now(),
        event_version: 2,
    };
    let usage_repo = FakeUsageRepo {
        request_logs: vec![row.clone()],
        request_log_calls: Arc::new(Mutex::new(Vec::new())),
    };

    let store: Arc<dyn ControlPlaneStore> = Arc::new(InMemoryStore::default());
    let app = build_app_with_store_ttl_and_usage_repo(
        store,
        DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC,
        Some(Arc::new(usage_repo.clone())),
    );
    let admin_token = login_admin_token(&app).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/admin/request-logs?start_ts=1700000000&end_ts=1700003600&limit=20&tenant_id={tenant_id}&api_key_id={api_key_id}&status_code=500&request_id=req-1&keyword=codex"
                ))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["items"][0]["account_id"], account_id.to_string());
    assert_eq!(payload["items"][0]["tenant_id"], tenant_id.to_string());
    assert_eq!(payload["items"][0]["api_key_id"], api_key_id.to_string());
    assert_eq!(payload["items"][0]["request_id"], "req-1");
    assert_eq!(payload["items"][0]["status_code"], 500);
    assert_eq!(payload["items"][0]["latency_ms"], 123);
    assert_eq!(payload["items"][0]["input_tokens"], 123);
    assert_eq!(payload["items"][0]["output_tokens"], 456);
    assert_eq!(payload["items"][0]["service_tier"], "priority");
    assert_eq!(payload["items"][0]["event_version"], 2);

    let calls = usage_repo.request_log_calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 1);
    let call = &calls[0];
    assert_eq!(call.start_ts, 1_700_000_000);
    assert_eq!(call.end_ts, 1_700_003_600);
    assert_eq!(call.limit, 20);
    assert_eq!(call.tenant_id, Some(tenant_id));
    assert_eq!(call.api_key_id, Some(api_key_id));
    assert_eq!(call.status_code, Some(500));
    assert_eq!(call.request_id.as_deref(), Some("req-1"));
    assert_eq!(call.keyword.as_deref(), Some("codex"));
}

#[tokio::test]
async fn admin_request_correlation_returns_request_and_audit_context() {
    let tenant_id = Uuid::new_v4();
    let account_id = Uuid::new_v4();
    let row = RequestLogRow {
        id: Uuid::new_v4(),
        account_id,
        tenant_id: Some(tenant_id),
        api_key_id: None,
        request_id: Some("req-correlation-1".to_string()),
        path: "/v1/responses".to_string(),
        method: "POST".to_string(),
        model: Some("gpt-5.3-codex".to_string()),
        service_tier: Some("default".to_string()),
        input_tokens: Some(64),
        cached_input_tokens: Some(0),
        output_tokens: Some(32),
        reasoning_tokens: Some(4),
        first_token_latency_ms: Some(40),
        status_code: 429,
        latency_ms: 88,
        is_stream: false,
        error_code: Some("rate_limited".to_string()),
        billing_phase: Some("released".to_string()),
        authorization_id: None,
        capture_status: Some("captured".to_string()),
        estimated_cost_microusd: None,
        created_at: chrono::Utc::now(),
        event_version: 2,
    };
    let usage_repo = FakeUsageRepo {
        request_logs: vec![row],
        request_log_calls: Arc::new(Mutex::new(Vec::new())),
    };

    let store: Arc<dyn ControlPlaneStore> = Arc::new(InMemoryStore::default());
    let app = build_app_with_store_ttl_and_usage_repo(
        store,
        DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC,
        Some(Arc::new(usage_repo.clone())),
    );
    let admin_token = login_admin_token(&app).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/admin/request-correlation/req-correlation-1?start_ts=1700000000&end_ts=1700003600&limit=20&tenant_id={tenant_id}"
                ))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| !value.is_empty()));
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["request_id"], "req-correlation-1");
    assert_eq!(
        payload["request_logs"][0]["request_id"],
        "req-correlation-1"
    );
    assert_eq!(payload["audit_logs_available"], false);
    assert_eq!(
        payload["audit_logs"]
            .as_array()
            .map(|items| items.len())
            .unwrap_or_default(),
        0
    );

    let calls = usage_repo.request_log_calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 1);
    let call = &calls[0];
    assert_eq!(call.start_ts, 1_700_000_000);
    assert_eq!(call.end_ts, 1_700_003_600);
    assert_eq!(call.limit, 20);
    assert_eq!(call.tenant_id, Some(tenant_id));
    assert_eq!(call.request_id.as_deref(), Some("req-correlation-1"));
    assert_eq!(call.api_key_id, None);
    assert_eq!(call.status_code, None);
    assert_eq!(call.keyword, None);
}

#[tokio::test]
async fn tenant_request_logs_returns_503_when_usage_repo_unavailable() {
    let Some(db_url) = test_db_url() else {
        eprintln!(
            "skip tenant_request_logs_returns_503_when_usage_repo_unavailable: set CONTROL_PLANE_DATABASE_URL"
        );
        return;
    };

    std::env::set_var("TENANT_AUTH_DEBUG_EXPOSE_CODE", "true");
    let store: Arc<dyn ControlPlaneStore> =
        Arc::new(PostgresStore::connect(&db_url).await.unwrap());
    let app =
        build_app_with_store_ttl_and_usage_repo(store, DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC, None);
    let (_tenant_id, tenant_token) = register_verified_tenant_token(&app).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/tenant/request-logs?start_ts=1700000000&end_ts=1700003600")
                .header("authorization", format!("Bearer {tenant_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn tenant_request_logs_overrides_tenant_scope_from_principal() {
    let Some(db_url) = test_db_url() else {
        eprintln!(
            "skip tenant_request_logs_overrides_tenant_scope_from_principal: set CONTROL_PLANE_DATABASE_URL"
        );
        return;
    };

    std::env::set_var("TENANT_AUTH_DEBUG_EXPOSE_CODE", "true");
    let usage_repo = FakeUsageRepo {
        request_logs: vec![RequestLogRow {
            id: Uuid::new_v4(),
            account_id: Uuid::new_v4(),
            tenant_id: None,
            api_key_id: None,
            request_id: Some("req-tenant".to_string()),
            path: "/v1/responses".to_string(),
            method: "POST".to_string(),
            model: Some("gpt-5.3-codex".to_string()),
            service_tier: Some("flex".to_string()),
            input_tokens: None,
            cached_input_tokens: None,
            output_tokens: None,
            reasoning_tokens: None,
            first_token_latency_ms: None,
            status_code: 200,
            latency_ms: 45,
            is_stream: false,
            error_code: None,
            billing_phase: None,
            authorization_id: None,
            capture_status: None,
            estimated_cost_microusd: None,
            created_at: chrono::Utc::now(),
            event_version: 2,
        }],
        request_log_calls: Arc::new(Mutex::new(Vec::new())),
    };
    let store: Arc<dyn ControlPlaneStore> =
        Arc::new(PostgresStore::connect(&db_url).await.unwrap());
    let app = build_app_with_store_ttl_and_usage_repo(
        store,
        DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC,
        Some(Arc::new(usage_repo.clone())),
    );
    let (tenant_id, tenant_token) = register_verified_tenant_token(&app).await;
    let fake_tenant_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/tenant/request-logs?start_ts=1700000000&end_ts=1700003600&limit=10&tenant_id={fake_tenant_id}&api_key_id={api_key_id}&status_code=200&request_id=req-tenant&keyword=codex"
                ))
                .header("authorization", format!("Bearer {tenant_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let calls = usage_repo.request_log_calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 1);
    let call = &calls[0];
    assert_eq!(call.start_ts, 1_700_000_000);
    assert_eq!(call.end_ts, 1_700_003_600);
    assert_eq!(call.limit, 10);
    assert_eq!(call.tenant_id, Some(tenant_id));
    assert_ne!(call.tenant_id, Some(fake_tenant_id));
    assert_eq!(call.api_key_id, Some(api_key_id));
    assert_eq!(call.status_code, Some(200));
    assert_eq!(call.request_id.as_deref(), Some("req-tenant"));
    assert_eq!(call.keyword.as_deref(), Some("codex"));
}
