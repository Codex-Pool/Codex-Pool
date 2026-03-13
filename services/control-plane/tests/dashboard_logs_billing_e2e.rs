use std::sync::Arc;

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
    build_app_with_store_ttl_and_usage_repo as cp_build_app_with_store_ttl_and_usage_repo,
    DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC,
};
use control_plane::store::postgres::PostgresStore;
use control_plane::store::ControlPlaneStore;
use control_plane::usage::clickhouse_repo::UsageQueryRepository;
use control_plane::usage::{RequestLogQuery, RequestLogRow};
use serde_json::{json, Value};
use sqlx_core::query_scalar::query_scalar;
use tower::ServiceExt;
use uuid::Uuid;

use crate::support;

#[derive(Clone)]
struct FakeUsageRepo;

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
        start_ts: i64,
        end_ts: i64,
        _tenant_id: Option<Uuid>,
        _account_id: Option<Uuid>,
        _api_key_id: Option<Uuid>,
    ) -> Result<UsageSummaryQueryResponse> {
        Ok(UsageSummaryQueryResponse {
            start_ts,
            end_ts,
            account_total_requests: 12,
            tenant_api_key_total_requests: 8,
            unique_account_count: 2,
            unique_tenant_api_key_count: 3,
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

    async fn query_request_logs(&self, _query: RequestLogQuery) -> Result<Vec<RequestLogRow>> {
        Ok(Vec::new())
    }
}

fn test_db_url() -> Option<String> {
    std::env::var("CONTROL_PLANE_DATABASE_URL")
        .ok()
        .or_else(|| std::env::var("DATABASE_URL").ok())
}

fn build_app_with_usage_repo(store: Arc<dyn ControlPlaneStore>) -> axum::Router {
    support::ensure_test_security_env();
    cp_build_app_with_store_ttl_and_usage_repo(
        store,
        DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC,
        Some(Arc::new(FakeUsageRepo)),
    )
}

async fn login_admin_token(app: &axum::Router) -> String {
    let response = app
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
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    payload["access_token"].as_str().unwrap().to_string()
}

async fn ensure_default_tenant_id(app: &axum::Router, admin_token: &str) -> Uuid {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/tenants/ensure-default")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    Uuid::parse_str(payload["id"].as_str().unwrap()).unwrap()
}

async fn login_default_admin_tenant_token(app: &axum::Router) -> (Uuid, String) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tenant/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "email": "admin@tenant.local",
                        "password": "admin123456"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    let tenant_id = Uuid::parse_str(payload["tenant_id"].as_str().unwrap()).unwrap();
    let access_token = payload["access_token"].as_str().unwrap().to_string();
    (tenant_id, access_token)
}

async fn register_verified_tenant_token(app: &axum::Router) -> (Uuid, String) {
    let suffix = Uuid::new_v4().simple().to_string();
    let email = format!("tenant-e2e-{suffix}@example.com");
    let password = "Password123!";
    let tenant_name = format!("tenant-e2e-{suffix}");

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
    let access_token = login_json["access_token"].as_str().unwrap().to_string();
    (tenant_id, access_token)
}

#[tokio::test]
async fn admin_dashboard_logs_billing_flow_records_audit_and_enforces_rbac() {
    let Some(db_url) = test_db_url() else {
        eprintln!(
            "skip admin_dashboard_logs_billing_flow_records_audit_and_enforces_rbac: set CONTROL_PLANE_DATABASE_URL"
        );
        return;
    };
    std::env::set_var("TENANT_AUTH_DEBUG_EXPOSE_CODE", "true");

    let store = Arc::new(PostgresStore::connect(&db_url).await.unwrap());
    let app = build_app_with_usage_repo(store.clone());

    let admin_token = login_admin_token(&app).await;
    let tenant_id = ensure_default_tenant_id(&app, &admin_token).await;
    let (default_tenant_id_from_login, _default_tenant_token) =
        login_default_admin_tenant_token(&app).await;
    assert_eq!(default_tenant_id_from_login, tenant_id);
    let (_other_tenant_id, tenant_token) = register_verified_tenant_token(&app).await;

    let requests = vec![
        ("/api/v1/admin/system/state".to_string(), StatusCode::OK),
        (
            "/api/v1/admin/usage/summary?start_ts=1700000000&end_ts=1700003600".to_string(),
            StatusCode::OK,
        ),
        (
            "/api/v1/admin/usage/trends/hourly?start_ts=1700000000&end_ts=1700003600".to_string(),
            StatusCode::OK,
        ),
        (
            "/api/v1/admin/request-logs?start_ts=1700000000&end_ts=1700003600".to_string(),
            StatusCode::OK,
        ),
        (
            "/api/v1/admin/audit-logs?start_ts=1700000000&end_ts=1700003600".to_string(),
            StatusCode::OK,
        ),
        (
            format!("/api/v1/admin/tenants/{tenant_id}/credits/balance"),
            StatusCode::OK,
        ),
        (
            format!("/api/v1/admin/tenants/{tenant_id}/credits/ledger"),
            StatusCode::OK,
        ),
    ];

    for (uri, expected_status) in requests {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(uri)
                    .header("authorization", format!("Bearer {admin_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected_status);
    }

    let tenant_on_admin_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/request-logs?start_ts=1700000000&end_ts=1700003600")
                .header("authorization", format!("Bearer {tenant_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(tenant_on_admin_response.status(), StatusCode::UNAUTHORIZED);

    let admin_on_tenant_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/tenant/request-logs?start_ts=1700000000&end_ts=1700003600")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(admin_on_tenant_response.status(), StatusCode::UNAUTHORIZED);

    let audit_count: i64 = query_scalar(
        r#"
        SELECT COUNT(*)::BIGINT
        FROM audit_logs
        WHERE action IN (
            'admin.usage.summary.get',
            'admin.usage.trends.hourly.get',
            'admin.request_logs.list',
            'admin.audit_logs.list',
            'admin.tenant.credits.balance.get',
            'admin.tenant.credits.ledger.list'
        )
        "#,
    )
    .fetch_one(&store.clone_pool())
    .await
    .unwrap_or(0);
    assert!(
        audit_count >= 5,
        "expected at least 5 admin audit rows, got {audit_count}"
    );
}

#[tokio::test]
async fn tenant_dashboard_logs_billing_flow_records_audit() {
    let Some(db_url) = test_db_url() else {
        eprintln!(
            "skip tenant_dashboard_logs_billing_flow_records_audit: set CONTROL_PLANE_DATABASE_URL"
        );
        return;
    };
    std::env::set_var("TENANT_AUTH_DEBUG_EXPOSE_CODE", "true");

    let store = Arc::new(PostgresStore::connect(&db_url).await.unwrap());
    let app = build_app_with_usage_repo(store.clone());
    let (tenant_id, tenant_token) = register_verified_tenant_token(&app).await;

    let requests = vec![
        (
            "/api/v1/tenant/usage/summary?start_ts=1700000000&end_ts=1700003600".to_string(),
            StatusCode::OK,
        ),
        (
            "/api/v1/tenant/usage/trends/hourly?start_ts=1700000000&end_ts=1700003600".to_string(),
            StatusCode::OK,
        ),
        (
            "/api/v1/tenant/request-logs?start_ts=1700000000&end_ts=1700003600".to_string(),
            StatusCode::OK,
        ),
        (
            "/api/v1/tenant/audit-logs?start_ts=1700000000&end_ts=1700003600".to_string(),
            StatusCode::OK,
        ),
        ("/api/v1/tenant/credits/balance".to_string(), StatusCode::OK),
        ("/api/v1/tenant/credits/ledger".to_string(), StatusCode::OK),
    ];

    for (uri, expected_status) in requests {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(uri)
                    .header("authorization", format!("Bearer {tenant_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected_status);
    }

    let audit_count: i64 = query_scalar(
        r#"
        SELECT COUNT(*)::BIGINT
        FROM audit_logs
        WHERE tenant_id = $1
          AND action IN (
            'tenant.usage.summary.get',
            'tenant.usage.trends.hourly.get',
            'tenant.request_logs.list',
            'tenant.audit_logs.list',
            'tenant.credits.balance.get',
            'tenant.credits.ledger.list'
          )
        "#,
    )
    .bind(tenant_id)
    .fetch_one(&store.clone_pool())
    .await
    .unwrap_or(0);
    assert!(
        audit_count >= 5,
        "expected at least 5 tenant audit rows, got {audit_count}"
    );
}
