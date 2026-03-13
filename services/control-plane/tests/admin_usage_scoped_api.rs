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
use control_plane::store::{ControlPlaneStore, InMemoryStore};
use control_plane::usage::clickhouse_repo::UsageQueryRepository;
use control_plane::usage::{RequestLogQuery, RequestLogRow};
use serde_json::{json, Value};
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
        Ok(vec![HourlyUsageTotalPoint {
            hour_start: 1_700_000_000,
            request_count: 3,
        }])
    }

    async fn query_hourly_tenant_api_key_totals(
        &self,
        _start_ts: i64,
        _end_ts: i64,
        _limit: u32,
        _tenant_id: Option<Uuid>,
        _api_key_id: Option<Uuid>,
    ) -> Result<Vec<HourlyUsageTotalPoint>> {
        Ok(vec![HourlyUsageTotalPoint {
            hour_start: 1_700_000_000,
            request_count: 2,
        }])
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
            account_total_requests: 9,
            tenant_api_key_total_requests: 7,
            unique_account_count: 2,
            unique_tenant_api_key_count: 2,
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

fn build_app() -> axum::Router {
    support::ensure_test_security_env();
    let store: Arc<dyn ControlPlaneStore> = Arc::new(InMemoryStore::default());
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

#[tokio::test]
async fn admin_usage_summary_requires_admin_auth() {
    let app = build_app();

    let unauthorized = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/usage/summary?start_ts=1700000000&end_ts=1700003600")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let admin_token = login_admin_token(&app).await;
    let authorized = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/usage/summary?start_ts=1700000000&end_ts=1700003600")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(authorized.status(), StatusCode::OK);
}

#[tokio::test]
async fn admin_usage_hourly_trends_requires_admin_auth() {
    let app = build_app();

    let unauthorized = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/usage/trends/hourly?start_ts=1700000000&end_ts=1700003600")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let admin_token = login_admin_token(&app).await;
    let authorized = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/usage/trends/hourly?start_ts=1700000000&end_ts=1700003600")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(authorized.status(), StatusCode::OK);
}
