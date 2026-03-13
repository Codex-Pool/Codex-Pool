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
use control_plane::store::{ControlPlaneStore, InMemoryStore};
use control_plane::usage::clickhouse_repo::UsageQueryRepository;
use serde_json::Value;
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

#[derive(Clone)]
struct FakeUsageRepo {
    summary: UsageSummaryQueryResponse,
    summary_calls: Arc<Mutex<Vec<SummaryCall>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SummaryCall {
    start_ts: i64,
    end_ts: i64,
    tenant_id: Option<Uuid>,
    account_id: Option<Uuid>,
    api_key_id: Option<Uuid>,
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
        start_ts: i64,
        end_ts: i64,
        tenant_id: Option<Uuid>,
        account_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<UsageSummaryQueryResponse> {
        self.summary_calls.lock().unwrap().push(SummaryCall {
            start_ts,
            end_ts,
            tenant_id,
            account_id,
            api_key_id,
        });
        Ok(self.summary.clone())
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
}

#[tokio::test]
async fn usage_summary_endpoint_returns_503_when_usage_repo_unavailable() {
    let app = build_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/usage/summary?start_ts=1700000000&end_ts=1700003600")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"]["code"], "service_unavailable");
    assert_eq!(json["error"]["message"], "usage repository is unavailable");
}

#[tokio::test]
async fn usage_summary_endpoint_returns_400_when_start_ts_greater_than_end_ts() {
    let app = build_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/usage/summary?start_ts=1700003600&end_ts=1700000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"]["code"], "invalid_request");
}

#[tokio::test]
async fn usage_summary_endpoint_returns_summary_when_usage_repo_available() {
    let summary = UsageSummaryQueryResponse {
        start_ts: 1_700_000_000,
        end_ts: 1_700_003_600,
        account_total_requests: 42,
        tenant_api_key_total_requests: 7,
        unique_account_count: 3,
        unique_tenant_api_key_count: 5,
        estimated_cost_microusd: None,
        dashboard_metrics: None,
    };
    let usage_repo = FakeUsageRepo {
        summary: summary.clone(),
        summary_calls: Arc::new(Mutex::new(Vec::new())),
    };

    let store: Arc<dyn ControlPlaneStore> = Arc::new(InMemoryStore::default());
    let app = build_app_with_store_ttl_and_usage_repo(
        store,
        DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC,
        Some(Arc::new(usage_repo)),
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/usage/summary?start_ts=1700000000&end_ts=1700003600")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: UsageSummaryQueryResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload, summary);
}

#[tokio::test]
async fn usage_summary_endpoint_forwards_filters_to_repo() {
    let tenant_id = Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();
    let account_id = Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap();
    let api_key_id = Uuid::parse_str("cccccccc-cccc-cccc-cccc-cccccccccccc").unwrap();
    let summary = UsageSummaryQueryResponse {
        start_ts: 1_700_000_000,
        end_ts: 1_700_003_600,
        account_total_requests: 42,
        tenant_api_key_total_requests: 7,
        unique_account_count: 3,
        unique_tenant_api_key_count: 5,
        estimated_cost_microusd: None,
        dashboard_metrics: None,
    };
    let usage_repo = FakeUsageRepo {
        summary,
        summary_calls: Arc::new(Mutex::new(Vec::new())),
    };
    let usage_repo_handle = usage_repo.clone();
    let store: Arc<dyn ControlPlaneStore> = Arc::new(InMemoryStore::default());
    let app = build_app_with_store_ttl_and_usage_repo(
        store,
        DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC,
        Some(Arc::new(usage_repo)),
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(
                    format!(
                        "/api/v1/usage/summary?start_ts=1700000000&end_ts=1700003600&tenant_id={tenant_id}&account_id={account_id}&api_key_id={api_key_id}"
                    )
                    .as_str(),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let calls = usage_repo_handle.summary_calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0],
        SummaryCall {
            start_ts: 1_700_000_000,
            end_ts: 1_700_003_600,
            tenant_id: Some(tenant_id),
            account_id: Some(account_id),
            api_key_id: Some(api_key_id),
        }
    );
}
