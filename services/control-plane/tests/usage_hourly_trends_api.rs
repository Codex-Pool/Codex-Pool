use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use codex_pool_core::api::{
    AccountUsageLeaderboardItem, ApiKeyUsageLeaderboardItem, HourlyAccountUsagePoint,
    HourlyTenantApiKeyUsagePoint, HourlyTenantUsageTotalPoint, HourlyUsageTotalPoint,
    TenantUsageLeaderboardItem, UsageHourlyTrendsResponse, UsageSummaryQueryResponse,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrendsQueryCall {
    start_ts: i64,
    end_ts: i64,
    limit: u32,
    account_id: Option<Uuid>,
    tenant_id: Option<Uuid>,
    api_key_id: Option<Uuid>,
}

#[derive(Clone, Default)]
struct FakeUsageRepo {
    account_totals: Vec<HourlyUsageTotalPoint>,
    tenant_api_key_totals: Vec<HourlyUsageTotalPoint>,
    account_total_calls: Arc<Mutex<Vec<TrendsQueryCall>>>,
    tenant_api_key_total_calls: Arc<Mutex<Vec<TrendsQueryCall>>>,
}

impl FakeUsageRepo {
    fn account_total_calls(&self) -> Vec<TrendsQueryCall> {
        self.account_total_calls.lock().unwrap().clone()
    }

    fn tenant_api_key_total_calls(&self) -> Vec<TrendsQueryCall> {
        self.tenant_api_key_total_calls.lock().unwrap().clone()
    }
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
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        account_id: Option<Uuid>,
    ) -> Result<Vec<HourlyUsageTotalPoint>> {
        self.account_total_calls
            .lock()
            .unwrap()
            .push(TrendsQueryCall {
                start_ts,
                end_ts,
                limit,
                account_id,
                tenant_id: None,
                api_key_id: None,
            });

        Ok(self.account_totals.clone())
    }

    async fn query_hourly_tenant_api_key_totals(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        tenant_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<Vec<HourlyUsageTotalPoint>> {
        self.tenant_api_key_total_calls
            .lock()
            .unwrap()
            .push(TrendsQueryCall {
                start_ts,
                end_ts,
                limit,
                account_id: None,
                tenant_id,
                api_key_id,
            });

        Ok(self.tenant_api_key_totals.clone())
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
}

#[tokio::test]
async fn usage_hourly_trends_endpoint_returns_503_when_usage_repo_unavailable() {
    let app = build_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/usage/trends/hourly?start_ts=1700000000&end_ts=1700003600")
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
async fn usage_hourly_trends_endpoint_returns_400_when_start_ts_greater_than_end_ts() {
    let app = build_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/usage/trends/hourly?start_ts=1700003600&end_ts=1700000000")
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
async fn usage_hourly_trends_endpoint_uses_default_limit_when_missing() {
    let usage_repo = FakeUsageRepo::default();
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
                .uri("/api/v1/usage/trends/hourly?start_ts=1700000000&end_ts=1700003600")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let account_calls = usage_repo_handle.account_total_calls();
    assert_eq!(account_calls.len(), 1);
    assert_eq!(account_calls[0].limit, 200);

    let tenant_api_key_calls = usage_repo_handle.tenant_api_key_total_calls();
    assert_eq!(tenant_api_key_calls.len(), 1);
    assert_eq!(tenant_api_key_calls[0].limit, 200);
}

#[tokio::test]
async fn usage_hourly_trends_endpoint_clamps_limit_to_max() {
    let usage_repo = FakeUsageRepo::default();
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
                .uri("/api/v1/usage/trends/hourly?start_ts=1700000000&end_ts=1700003600&limit=5001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let account_calls = usage_repo_handle.account_total_calls();
    assert_eq!(account_calls.len(), 1);
    assert_eq!(account_calls[0].limit, 1000);

    let tenant_api_key_calls = usage_repo_handle.tenant_api_key_total_calls();
    assert_eq!(tenant_api_key_calls.len(), 1);
    assert_eq!(tenant_api_key_calls[0].limit, 1000);
}

#[tokio::test]
async fn usage_hourly_trends_endpoint_passes_filters_to_repo() {
    let account_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
    let tenant_id = Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap();
    let api_key_id = Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap();
    let usage_repo = FakeUsageRepo::default();
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
                        "/api/v1/usage/trends/hourly?start_ts=1700000000&end_ts=1700003600&account_id={account_id}&tenant_id={tenant_id}&api_key_id={api_key_id}",
                    )
                    .as_str(),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let account_calls = usage_repo_handle.account_total_calls();
    assert_eq!(account_calls.len(), 1);
    assert_eq!(account_calls[0].account_id, Some(account_id));

    let tenant_api_key_calls = usage_repo_handle.tenant_api_key_total_calls();
    assert_eq!(tenant_api_key_calls.len(), 1);
    assert_eq!(tenant_api_key_calls[0].tenant_id, Some(tenant_id));
    assert_eq!(tenant_api_key_calls[0].api_key_id, Some(api_key_id));
}

#[tokio::test]
async fn usage_hourly_trends_endpoint_returns_totals_when_usage_repo_available() {
    let account_totals = vec![
        HourlyUsageTotalPoint {
            hour_start: 1_700_000_000,
            request_count: 42,
        },
        HourlyUsageTotalPoint {
            hour_start: 1_700_003_600,
            request_count: 17,
        },
    ];
    let tenant_api_key_totals = vec![
        HourlyUsageTotalPoint {
            hour_start: 1_700_000_000,
            request_count: 33,
        },
        HourlyUsageTotalPoint {
            hour_start: 1_700_003_600,
            request_count: 9,
        },
    ];

    let usage_repo = FakeUsageRepo {
        account_totals: account_totals.clone(),
        tenant_api_key_totals: tenant_api_key_totals.clone(),
        ..Default::default()
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
                .uri("/api/v1/usage/trends/hourly?start_ts=1700000000&end_ts=1700007200&limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: UsageHourlyTrendsResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload.start_ts, 1_700_000_000);
    assert_eq!(payload.end_ts, 1_700_007_200);
    assert_eq!(payload.account_totals, account_totals);
    assert_eq!(payload.tenant_api_key_totals, tenant_api_key_totals);
}
