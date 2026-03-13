use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use codex_pool_core::api::{
    AccountUsageLeaderboardItem, ApiKeyUsageLeaderboardItem, ApiKeyUsageLeaderboardResponse,
    HourlyAccountUsagePoint, HourlyTenantApiKeyUsagePoint, HourlyTenantUsageTotalPoint,
    HourlyUsageTotalPoint, TenantUsageLeaderboardItem, UsageSummaryQueryResponse,
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
struct ApiKeyLeaderboardCall {
    start_ts: i64,
    end_ts: i64,
    limit: u32,
    tenant_id: Option<Uuid>,
    api_key_id: Option<Uuid>,
}

#[derive(Clone, Default)]
struct FakeUsageRepo {
    api_key_leaderboard_items: Vec<ApiKeyUsageLeaderboardItem>,
    api_key_leaderboard_calls: Arc<Mutex<Vec<ApiKeyLeaderboardCall>>>,
}

impl FakeUsageRepo {
    fn api_key_leaderboard_calls(&self) -> Vec<ApiKeyLeaderboardCall> {
        self.api_key_leaderboard_calls.lock().unwrap().clone()
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
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        tenant_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<Vec<ApiKeyUsageLeaderboardItem>> {
        self.api_key_leaderboard_calls
            .lock()
            .unwrap()
            .push(ApiKeyLeaderboardCall {
                start_ts,
                end_ts,
                limit,
                tenant_id,
                api_key_id,
            });

        Ok(self.api_key_leaderboard_items.clone())
    }
}

#[tokio::test]
async fn api_key_leaderboard_endpoint_returns_503_when_usage_repo_unavailable() {
    let app = build_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/usage/leaderboard/api-keys?start_ts=1700000000&end_ts=1700003600")
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
async fn api_key_leaderboard_endpoint_returns_400_when_start_ts_greater_than_end_ts() {
    let app = build_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/usage/leaderboard/api-keys?start_ts=1700003600&end_ts=1700000000")
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
async fn api_key_leaderboard_endpoint_uses_default_limit_when_missing() {
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
                .uri("/api/v1/usage/leaderboard/api-keys?start_ts=1700000000&end_ts=1700003600")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let calls = usage_repo_handle.api_key_leaderboard_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].limit, 20);
    assert_eq!(calls[0].tenant_id, None);
    assert_eq!(calls[0].api_key_id, None);
}

#[tokio::test]
async fn api_key_leaderboard_endpoint_caps_limit_at_max() {
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
                    "/api/v1/usage/leaderboard/api-keys?start_ts=1700000000&end_ts=1700003600&limit=9999",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let calls = usage_repo_handle.api_key_leaderboard_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].limit, 200);
}

#[tokio::test]
async fn api_key_leaderboard_endpoint_passes_tenant_filter_to_repo() {
    let tenant_id = Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();
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
                        "/api/v1/usage/leaderboard/api-keys?start_ts=1700000000&end_ts=1700003600&tenant_id={tenant_id}"
                    )
                    .as_str(),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let calls = usage_repo_handle.api_key_leaderboard_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].tenant_id, Some(tenant_id));
    assert_eq!(calls[0].api_key_id, None);
}

#[tokio::test]
async fn api_key_leaderboard_endpoint_passes_api_key_filter_to_repo() {
    let api_key_id = Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap();
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
                        "/api/v1/usage/leaderboard/api-keys?start_ts=1700000000&end_ts=1700003600&api_key_id={api_key_id}"
                    )
                    .as_str(),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let calls = usage_repo_handle.api_key_leaderboard_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].tenant_id, None);
    assert_eq!(calls[0].api_key_id, Some(api_key_id));
}

#[tokio::test]
async fn api_key_leaderboard_endpoint_passes_tenant_and_api_key_filters_to_repo() {
    let tenant_id = Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();
    let api_key_id = Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap();
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
                        "/api/v1/usage/leaderboard/api-keys?start_ts=1700000000&end_ts=1700003600&tenant_id={tenant_id}&api_key_id={api_key_id}"
                    )
                    .as_str(),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let calls = usage_repo_handle.api_key_leaderboard_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].tenant_id, Some(tenant_id));
    assert_eq!(calls[0].api_key_id, Some(api_key_id));
}

#[tokio::test]
async fn api_key_leaderboard_endpoint_returns_items_when_usage_repo_available() {
    let expected_items = vec![
        ApiKeyUsageLeaderboardItem {
            tenant_id: Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
            api_key_id: Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap(),
            total_requests: 123,
        },
        ApiKeyUsageLeaderboardItem {
            tenant_id: Uuid::parse_str("cccccccc-cccc-cccc-cccc-cccccccccccc").unwrap(),
            api_key_id: Uuid::parse_str("dddddddd-dddd-dddd-dddd-dddddddddddd").unwrap(),
            total_requests: 45,
        },
    ];

    let usage_repo = FakeUsageRepo {
        api_key_leaderboard_items: expected_items.clone(),
        ..Default::default()
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
                    "/api/v1/usage/leaderboard/api-keys?start_ts=1700000000&end_ts=1700003600&limit=5",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: ApiKeyUsageLeaderboardResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload.start_ts, 1_700_000_000);
    assert_eq!(payload.end_ts, 1_700_003_600);
    assert_eq!(payload.items, expected_items);

    let calls = usage_repo_handle.api_key_leaderboard_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].limit, 5);
}
