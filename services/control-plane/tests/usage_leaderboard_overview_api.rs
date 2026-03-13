use std::sync::{Arc, Mutex};

use anyhow::Result;
use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use codex_pool_core::api::{
    AccountUsageLeaderboardItem, ApiKeyUsageLeaderboardItem, HourlyAccountUsagePoint,
    HourlyTenantApiKeyUsagePoint, HourlyTenantUsageTotalPoint, HourlyUsageTotalPoint,
    TenantUsageLeaderboardItem, UsageLeaderboardOverviewResponse, UsageSummaryQueryResponse,
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
struct LeaderboardCall {
    start_ts: i64,
    end_ts: i64,
    limit: u32,
    tenant_id: Option<Uuid>,
    account_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ApiKeyLeaderboardCall {
    start_ts: i64,
    end_ts: i64,
    limit: u32,
    tenant_id: Option<Uuid>,
    api_key_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SummaryCall {
    start_ts: i64,
    end_ts: i64,
    tenant_id: Option<Uuid>,
    account_id: Option<Uuid>,
    api_key_id: Option<Uuid>,
}

#[derive(Clone, Default)]
struct FakeUsageRepo {
    tenant_items: Vec<TenantUsageLeaderboardItem>,
    account_items: Vec<AccountUsageLeaderboardItem>,
    api_key_items: Vec<ApiKeyUsageLeaderboardItem>,
    tenant_calls: Arc<Mutex<Vec<LeaderboardCall>>>,
    account_calls: Arc<Mutex<Vec<LeaderboardCall>>>,
    api_key_calls: Arc<Mutex<Vec<ApiKeyLeaderboardCall>>>,
    summary_calls: Arc<Mutex<Vec<SummaryCall>>>,
}

impl FakeUsageRepo {
    fn tenant_calls(&self) -> Vec<LeaderboardCall> {
        self.tenant_calls.lock().unwrap().clone()
    }

    fn account_calls(&self) -> Vec<LeaderboardCall> {
        self.account_calls.lock().unwrap().clone()
    }

    fn api_key_calls(&self) -> Vec<ApiKeyLeaderboardCall> {
        self.api_key_calls.lock().unwrap().clone()
    }

    fn summary_calls(&self) -> Vec<SummaryCall> {
        self.summary_calls.lock().unwrap().clone()
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
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        tenant_id: Option<Uuid>,
    ) -> Result<Vec<TenantUsageLeaderboardItem>> {
        self.tenant_calls.lock().unwrap().push(LeaderboardCall {
            start_ts,
            end_ts,
            limit,
            tenant_id,
            account_id: None,
        });

        Ok(self.tenant_items.clone())
    }

    async fn query_account_leaderboard(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        account_id: Option<Uuid>,
    ) -> Result<Vec<AccountUsageLeaderboardItem>> {
        self.account_calls.lock().unwrap().push(LeaderboardCall {
            start_ts,
            end_ts,
            limit,
            tenant_id: None,
            account_id,
        });

        Ok(self.account_items.clone())
    }

    async fn query_api_key_leaderboard(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        tenant_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<Vec<ApiKeyUsageLeaderboardItem>> {
        self.api_key_calls
            .lock()
            .unwrap()
            .push(ApiKeyLeaderboardCall {
                start_ts,
                end_ts,
                limit,
                tenant_id,
                api_key_id,
            });

        Ok(self.api_key_items.clone())
    }
}

#[tokio::test]
async fn overview_endpoint_returns_503_when_usage_repo_unavailable() {
    let app = build_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/usage/leaderboard/overview?start_ts=1700000000&end_ts=1700003600")
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
async fn overview_endpoint_returns_400_when_start_ts_greater_than_end_ts() {
    let app = build_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/usage/leaderboard/overview?start_ts=1700003600&end_ts=1700000000")
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
async fn overview_endpoint_uses_default_limit_when_missing() {
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
                .uri("/api/v1/usage/leaderboard/overview?start_ts=1700000000&end_ts=1700003600")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let tenant_calls = usage_repo_handle.tenant_calls();
    assert_eq!(tenant_calls.len(), 1);
    assert_eq!(tenant_calls[0].limit, 20);
    assert_eq!(tenant_calls[0].tenant_id, None);

    let account_calls = usage_repo_handle.account_calls();
    assert_eq!(account_calls.len(), 1);
    assert_eq!(account_calls[0].limit, 20);

    let api_key_calls = usage_repo_handle.api_key_calls();
    assert_eq!(api_key_calls.len(), 1);
    assert_eq!(api_key_calls[0].limit, 20);
    assert_eq!(api_key_calls[0].tenant_id, None);
    assert_eq!(api_key_calls[0].api_key_id, None);
}

#[tokio::test]
async fn overview_endpoint_passes_per_leaderboard_limits_to_each_repo() {
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
                    "/api/v1/usage/leaderboard/overview?start_ts=1700000000&end_ts=1700003600&limit=5&tenant_limit=7&account_limit=11&api_key_limit=13",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let tenant_calls = usage_repo_handle.tenant_calls();
    assert_eq!(tenant_calls.len(), 1);
    assert_eq!(tenant_calls[0].limit, 7);

    let account_calls = usage_repo_handle.account_calls();
    assert_eq!(account_calls.len(), 1);
    assert_eq!(account_calls[0].limit, 11);

    let api_key_calls = usage_repo_handle.api_key_calls();
    assert_eq!(api_key_calls.len(), 1);
    assert_eq!(api_key_calls[0].limit, 13);
}

#[tokio::test]
async fn overview_endpoint_caps_per_leaderboard_limits_at_max() {
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
                    "/api/v1/usage/leaderboard/overview?start_ts=1700000000&end_ts=1700003600&limit=1&tenant_limit=9999&account_limit=8888&api_key_limit=7777",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let tenant_calls = usage_repo_handle.tenant_calls();
    assert_eq!(tenant_calls.len(), 1);
    assert_eq!(tenant_calls[0].limit, 200);

    let account_calls = usage_repo_handle.account_calls();
    assert_eq!(account_calls.len(), 1);
    assert_eq!(account_calls[0].limit, 200);

    let api_key_calls = usage_repo_handle.api_key_calls();
    assert_eq!(api_key_calls.len(), 1);
    assert_eq!(api_key_calls[0].limit, 200);
}

#[tokio::test]
async fn overview_endpoint_passes_tenant_filter_only_to_tenant_leaderboard() {
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
                        "/api/v1/usage/leaderboard/overview?start_ts=1700000000&end_ts=1700003600&tenant_id={tenant_id}&api_key_id={api_key_id}"
                    )
                    .as_str(),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let tenant_calls = usage_repo_handle.tenant_calls();
    assert_eq!(tenant_calls.len(), 1);
    assert_eq!(tenant_calls[0].limit, 20);
    assert_eq!(tenant_calls[0].tenant_id, Some(tenant_id));

    let account_calls = usage_repo_handle.account_calls();
    assert_eq!(account_calls.len(), 1);
    assert_eq!(account_calls[0].limit, 20);
    assert_eq!(account_calls[0].tenant_id, None);
    assert_eq!(account_calls[0].account_id, None);

    let api_key_calls = usage_repo_handle.api_key_calls();
    assert_eq!(api_key_calls.len(), 1);
    assert_eq!(api_key_calls[0].tenant_id, None);
    assert_eq!(api_key_calls[0].api_key_id, Some(api_key_id));
}

#[tokio::test]
async fn overview_endpoint_passes_account_filter_only_to_account_leaderboard() {
    let account_id = Uuid::parse_str("cccccccc-cccc-cccc-cccc-cccccccccccc").unwrap();
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
                        "/api/v1/usage/leaderboard/overview?start_ts=1700000000&end_ts=1700003600&account_id={account_id}"
                    )
                    .as_str(),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let tenant_calls = usage_repo_handle.tenant_calls();
    assert_eq!(tenant_calls.len(), 1);
    assert_eq!(tenant_calls[0].tenant_id, None);
    assert_eq!(tenant_calls[0].account_id, None);

    let account_calls = usage_repo_handle.account_calls();
    assert_eq!(account_calls.len(), 1);
    assert_eq!(account_calls[0].tenant_id, None);
    assert_eq!(account_calls[0].account_id, Some(account_id));

    let api_key_calls = usage_repo_handle.api_key_calls();
    assert_eq!(api_key_calls.len(), 1);
    assert_eq!(api_key_calls[0].tenant_id, None);
    assert_eq!(api_key_calls[0].api_key_id, None);
}

#[tokio::test]
async fn overview_endpoint_passes_api_key_tenant_filter_only_to_api_key_leaderboard() {
    let tenant_id = Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();
    let api_key_tenant_id = Uuid::parse_str("dddddddd-dddd-dddd-dddd-dddddddddddd").unwrap();
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
                        "/api/v1/usage/leaderboard/overview?start_ts=1700000000&end_ts=1700003600&tenant_id={tenant_id}&api_key_tenant_id={api_key_tenant_id}&api_key_id={api_key_id}"
                    )
                    .as_str(),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let tenant_calls = usage_repo_handle.tenant_calls();
    assert_eq!(tenant_calls.len(), 1);
    assert_eq!(tenant_calls[0].tenant_id, Some(tenant_id));
    assert_eq!(tenant_calls[0].account_id, None);

    let account_calls = usage_repo_handle.account_calls();
    assert_eq!(account_calls.len(), 1);
    assert_eq!(account_calls[0].tenant_id, None);
    assert_eq!(account_calls[0].account_id, None);

    let api_key_calls = usage_repo_handle.api_key_calls();
    assert_eq!(api_key_calls.len(), 1);
    assert_eq!(api_key_calls[0].tenant_id, Some(api_key_tenant_id));
    assert_eq!(api_key_calls[0].api_key_id, Some(api_key_id));
}

#[tokio::test]
async fn overview_endpoint_returns_all_three_leaderboards() {
    let expected_tenants = vec![
        TenantUsageLeaderboardItem {
            tenant_id: Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
            total_requests: 123,
        },
        TenantUsageLeaderboardItem {
            tenant_id: Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap(),
            total_requests: 45,
        },
    ];

    let expected_accounts = vec![
        AccountUsageLeaderboardItem {
            account_id: Uuid::parse_str("cccccccc-cccc-cccc-cccc-cccccccccccc").unwrap(),
            total_requests: 88,
        },
        AccountUsageLeaderboardItem {
            account_id: Uuid::parse_str("dddddddd-dddd-dddd-dddd-dddddddddddd").unwrap(),
            total_requests: 33,
        },
    ];

    let expected_api_keys = vec![
        ApiKeyUsageLeaderboardItem {
            tenant_id: Uuid::parse_str("eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee").unwrap(),
            api_key_id: Uuid::parse_str("ffffffff-ffff-ffff-ffff-ffffffffffff").unwrap(),
            total_requests: 21,
        },
        ApiKeyUsageLeaderboardItem {
            tenant_id: Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
            api_key_id: Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
            total_requests: 13,
        },
    ];

    let usage_repo = FakeUsageRepo {
        tenant_items: expected_tenants.clone(),
        account_items: expected_accounts.clone(),
        api_key_items: expected_api_keys.clone(),
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
                    "/api/v1/usage/leaderboard/overview?start_ts=1700000000&end_ts=1700003600&limit=5",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: UsageLeaderboardOverviewResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload.start_ts, 1_700_000_000);
    assert_eq!(payload.end_ts, 1_700_003_600);
    assert_eq!(payload.tenants, expected_tenants);
    assert_eq!(payload.accounts, expected_accounts);
    assert_eq!(payload.api_keys, expected_api_keys);

    let tenant_calls = usage_repo_handle.tenant_calls();
    assert_eq!(tenant_calls.len(), 1);
    assert_eq!(tenant_calls[0].limit, 5);
    assert_eq!(tenant_calls[0].tenant_id, None);

    let account_calls = usage_repo_handle.account_calls();
    assert_eq!(account_calls.len(), 1);
    assert_eq!(account_calls[0].limit, 5);
    assert_eq!(account_calls[0].tenant_id, None);
    assert_eq!(account_calls[0].account_id, None);

    let api_key_calls = usage_repo_handle.api_key_calls();
    assert_eq!(api_key_calls.len(), 1);
    assert_eq!(api_key_calls[0].limit, 5);
}

#[tokio::test]
async fn overview_endpoint_includes_summary_when_requested() {
    let tenant_id = Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();
    let account_id = Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap();
    let api_key_id = Uuid::parse_str("cccccccc-cccc-cccc-cccc-cccccccccccc").unwrap();
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
                        "/api/v1/usage/leaderboard/overview?start_ts=1700000000&end_ts=1700003600&include_summary=true&tenant_id={tenant_id}&account_id={account_id}&api_key_id={api_key_id}",
                    )
                    .as_str(),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["summary"]["start_ts"], 1_700_000_000);
    assert_eq!(json["summary"]["end_ts"], 1_700_003_600);

    let summary_calls = usage_repo_handle.summary_calls();
    assert_eq!(summary_calls.len(), 1);
    assert_eq!(
        summary_calls[0],
        SummaryCall {
            start_ts: 1_700_000_000,
            end_ts: 1_700_003_600,
            tenant_id: Some(tenant_id),
            account_id: Some(account_id),
            api_key_id: Some(api_key_id),
        }
    );
}

#[tokio::test]
async fn overview_endpoint_omits_summary_when_include_summary_false_or_missing() {
    let usage_repo = FakeUsageRepo::default();
    let usage_repo_handle = usage_repo.clone();
    let store: Arc<dyn ControlPlaneStore> = Arc::new(InMemoryStore::default());
    let app = build_app_with_store_ttl_and_usage_repo(
        store,
        DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC,
        Some(Arc::new(usage_repo)),
    );

    let response_without_param = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/usage/leaderboard/overview?start_ts=1700000000&end_ts=1700003600")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response_without_param.status(), StatusCode::OK);
    let body = to_bytes(response_without_param.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("summary").is_none());

    let response_with_false = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(
                    "/api/v1/usage/leaderboard/overview?start_ts=1700000000&end_ts=1700003600&include_summary=false",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response_with_false.status(), StatusCode::OK);
    let body = to_bytes(response_with_false.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("summary").is_none());

    assert!(usage_repo_handle.summary_calls().is_empty());
}
