use anyhow::anyhow;
use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use base64::Engine;
use codex_pool_core::api::{
    CreateApiKeyRequest, CreateApiKeyResponse, CreateTenantRequest, CreateUpstreamAccountRequest,
    DataPlaneSnapshot, OAuthAccountStatusResponse, OAuthRateLimitRefreshJobStatus,
    OAuthRateLimitRefreshJobSummary, OAuthRateLimitSnapshot, OAuthRateLimitWindow,
};
use codex_pool_core::model::{ApiKey, Tenant, UpstreamAccount, UpstreamAuthProvider};
use control_plane::app::{
    build_app as cp_build_app, build_app_with_store as cp_build_app_with_store,
};
use control_plane::crypto::CredentialCipher;
use control_plane::oauth::{
    OAuthRefreshErrorCode, OAuthTokenClient, OAuthTokenClientError, OAuthTokenInfo,
};
use control_plane::store::{
    ControlPlaneStore, InMemoryStore, UpsertOneTimeSessionAccountRequest, ValidatedPrincipal,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use tokio::time::{sleep, Duration};
use tower::ServiceExt;
use uuid::Uuid;

fn build_app() -> axum::Router {
    support::ensure_test_security_env();
    cp_build_app()
}

fn build_app_with_store(store: Arc<dyn ControlPlaneStore>) -> axum::Router {
    support::ensure_test_security_env();
    cp_build_app_with_store(store)
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

fn internal_service_token() -> String {
    support::internal_service_token()
}

struct RateLimitApiTestStore {
    inner: InMemoryStore,
    statuses: Arc<Mutex<HashMap<Uuid, OAuthAccountStatusResponse>>>,
    jobs: Arc<Mutex<HashMap<Uuid, OAuthRateLimitRefreshJobSummary>>>,
    run_calls: Arc<AtomicUsize>,
    seen_ok_refresh_calls: Arc<AtomicUsize>,
}

impl Default for RateLimitApiTestStore {
    fn default() -> Self {
        Self {
            inner: InMemoryStore::default(),
            statuses: Arc::new(Mutex::new(HashMap::new())),
            jobs: Arc::new(Mutex::new(HashMap::new())),
            run_calls: Arc::new(AtomicUsize::new(0)),
            seen_ok_refresh_calls: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl RateLimitApiTestStore {
    fn with_status(status: OAuthAccountStatusResponse) -> Self {
        let mut statuses = HashMap::new();
        statuses.insert(status.account_id, status);
        Self {
            inner: InMemoryStore::default(),
            statuses: Arc::new(Mutex::new(statuses)),
            jobs: Arc::new(Mutex::new(HashMap::new())),
            run_calls: Arc::new(AtomicUsize::new(0)),
            seen_ok_refresh_calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn run_call_count(&self) -> usize {
        self.run_calls.load(Ordering::Relaxed)
    }

    fn seen_ok_refresh_call_count(&self) -> usize {
        self.seen_ok_refresh_calls.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl ControlPlaneStore for RateLimitApiTestStore {
    async fn create_tenant(&self, req: CreateTenantRequest) -> anyhow::Result<Tenant> {
        self.inner.create_tenant(req).await
    }

    async fn list_tenants(&self) -> anyhow::Result<Vec<Tenant>> {
        self.inner.list_tenants().await
    }

    async fn create_api_key(
        &self,
        req: CreateApiKeyRequest,
    ) -> anyhow::Result<CreateApiKeyResponse> {
        self.inner.create_api_key(req).await
    }

    async fn list_api_keys(&self) -> anyhow::Result<Vec<ApiKey>> {
        self.inner.list_api_keys().await
    }

    async fn validate_api_key(&self, token: &str) -> anyhow::Result<Option<ValidatedPrincipal>> {
        self.inner.validate_api_key(token).await
    }

    async fn create_upstream_account(
        &self,
        req: CreateUpstreamAccountRequest,
    ) -> anyhow::Result<UpstreamAccount> {
        self.inner.create_upstream_account(req).await
    }

    async fn list_upstream_accounts(&self) -> anyhow::Result<Vec<UpstreamAccount>> {
        self.inner.list_upstream_accounts().await
    }

    async fn upsert_one_time_session_account(
        &self,
        req: UpsertOneTimeSessionAccountRequest,
    ) -> anyhow::Result<control_plane::store::OAuthUpsertResult> {
        self.inner.upsert_one_time_session_account(req).await
    }

    async fn oauth_account_status(
        &self,
        account_id: Uuid,
    ) -> anyhow::Result<OAuthAccountStatusResponse> {
        if let Some(status) = self
            .statuses
            .lock()
            .expect("statuses lock")
            .get(&account_id)
            .cloned()
        {
            return Ok(status);
        }
        self.inner.oauth_account_status(account_id).await
    }

    async fn oauth_account_statuses(
        &self,
        account_ids: Vec<Uuid>,
    ) -> anyhow::Result<Vec<OAuthAccountStatusResponse>> {
        let mut items = Vec::with_capacity(account_ids.len());
        for account_id in account_ids {
            items.push(self.oauth_account_status(account_id).await?);
        }
        Ok(items)
    }

    async fn create_oauth_rate_limit_refresh_job(
        &self,
    ) -> anyhow::Result<OAuthRateLimitRefreshJobSummary> {
        let summary = OAuthRateLimitRefreshJobSummary {
            job_id: Uuid::new_v4(),
            status: OAuthRateLimitRefreshJobStatus::Queued,
            total: 3,
            processed: 0,
            success_count: 0,
            failed_count: 0,
            started_at: None,
            finished_at: None,
            created_at: chrono::Utc::now(),
            throughput_per_min: None,
            error_summary: Vec::new(),
        };
        self.jobs
            .lock()
            .expect("jobs lock")
            .insert(summary.job_id, summary.clone());
        Ok(summary)
    }

    async fn oauth_rate_limit_refresh_job(
        &self,
        job_id: Uuid,
    ) -> anyhow::Result<OAuthRateLimitRefreshJobSummary> {
        self.jobs
            .lock()
            .expect("jobs lock")
            .get(&job_id)
            .cloned()
            .ok_or_else(|| anyhow!("job not found"))
    }

    async fn run_oauth_rate_limit_refresh_job(&self, job_id: Uuid) -> anyhow::Result<()> {
        self.run_calls.fetch_add(1, Ordering::Relaxed);
        let mut jobs = self.jobs.lock().expect("jobs lock");
        let summary = jobs
            .get_mut(&job_id)
            .ok_or_else(|| anyhow!("job not found"))?;
        if summary.status == OAuthRateLimitRefreshJobStatus::Queued {
            let now = chrono::Utc::now();
            summary.status = OAuthRateLimitRefreshJobStatus::Completed;
            summary.started_at = Some(now);
            summary.finished_at = Some(now);
            summary.processed = summary.total;
            summary.success_count = summary.total;
            summary.failed_count = 0;
            summary.throughput_per_min = Some(60.0);
        }
        Ok(())
    }

    async fn maybe_refresh_oauth_rate_limit_cache_on_seen_ok(
        &self,
        _account_id: Uuid,
    ) -> anyhow::Result<()> {
        self.seen_ok_refresh_calls.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn mark_account_seen_ok(
        &self,
        account_id: Uuid,
        seen_ok_at: chrono::DateTime<chrono::Utc>,
        min_write_interval_sec: i64,
    ) -> anyhow::Result<bool> {
        self.inner
            .mark_account_seen_ok(account_id, seen_ok_at, min_write_interval_sec)
            .await
    }

    async fn snapshot(&self) -> anyhow::Result<DataPlaneSnapshot> {
        self.inner.snapshot().await
    }
}

fn sample_cached_oauth_status(account_id: Uuid) -> OAuthAccountStatusResponse {
    let now = chrono::Utc::now();
    OAuthAccountStatusResponse {
        account_id,
        auth_provider: UpstreamAuthProvider::OAuthRefreshToken,
        credential_kind: Some(codex_pool_core::api::SessionCredentialKind::RefreshRotatable),
        email: Some("cached@example.com".to_string()),
        oauth_subject: None,
        oauth_identity_provider: None,
        email_verified: None,
        chatgpt_plan_type: Some("pro".to_string()),
        chatgpt_user_id: None,
        chatgpt_subscription_active_start: None,
        chatgpt_subscription_active_until: None,
        chatgpt_subscription_last_checked: None,
        chatgpt_account_user_id: None,
        chatgpt_compute_residency: None,
        organizations: None,
        groups: None,
        source_type: Some("codex".to_string()),
        token_family_id: Some("family-1".to_string()),
        token_version: Some(2),
        token_expires_at: Some(now + chrono::Duration::minutes(30)),
        last_refresh_at: Some(now - chrono::Duration::minutes(1)),
        last_refresh_status: codex_pool_core::api::OAuthRefreshStatus::Ok,
        refresh_reused_detected: false,
        last_refresh_error_code: None,
        last_refresh_error: None,
        effective_enabled: true,
        rate_limits: vec![OAuthRateLimitSnapshot {
            limit_id: Some("five_hours".to_string()),
            limit_name: Some("5h".to_string()),
            primary: Some(OAuthRateLimitWindow {
                used_percent: 12.5,
                window_minutes: Some(300),
                resets_at: Some(now + chrono::Duration::minutes(10)),
            }),
            secondary: None,
        }],
        rate_limits_fetched_at: Some(now - chrono::Duration::seconds(15)),
        rate_limits_expires_at: Some(now + chrono::Duration::minutes(3)),
        rate_limits_last_error_code: None,
        rate_limits_last_error: None,
        next_refresh_at: Some(now + chrono::Duration::minutes(25)),
    }
}

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let app = build_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value, json!({"ok": true}));
}
#[tokio::test]
async fn tenant_routes_require_admin_auth() {
    let app = build_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/tenants")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn snapshot_route_requires_internal_service_token() {
    let app = build_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/data-plane/snapshot")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn create_and_list_tenants() {
    let app = build_app();
    let admin_token = login_admin_token(&app).await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tenants")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(r#"{"name":"team-a"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::OK);

    let list_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/tenants")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list_response.status(), StatusCode::OK);
    let body = to_bytes(list_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(value.as_array().unwrap().len(), 1);
    assert_eq!(value[0]["name"], "team-a");
}

#[tokio::test]
async fn create_and_list_api_keys() {
    let app = build_app();
    let admin_token = login_admin_token(&app).await;

    let tenant_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tenants")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(r#"{"name":"tenant-k"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    let tenant_body = to_bytes(tenant_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let tenant_json: Value = serde_json::from_slice(&tenant_body).unwrap();
    let tenant_id = tenant_json["id"].as_str().unwrap();

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/api-keys")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({"tenant_id": tenant_id, "name": "cli-key"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body = to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_json: Value = serde_json::from_slice(&create_body).unwrap();
    assert!(create_json["plaintext_key"]
        .as_str()
        .unwrap()
        .starts_with("cp_"));
    assert_eq!(create_json["record"]["name"], "cli-key");

    let list_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/api-keys")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list_response.status(), StatusCode::OK);
    let body = to_bytes(list_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(value.as_array().unwrap().len(), 1);
    assert_eq!(value[0]["tenant_id"], tenant_id);
}

#[tokio::test]
async fn create_and_list_upstream_accounts() {
    let app = build_app();
    let admin_token = login_admin_token(&app).await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "label": "acc-chatgpt",
                        "mode": "chat_gpt_session",
                        "base_url": "https://chatgpt.com/backend-api/codex",
                        "bearer_token": "tok-1",
                        "chatgpt_account_id": "acct_123",
                        "enabled": true,
                        "priority": 5
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::OK);

    let list_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/upstream-accounts")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list_response.status(), StatusCode::OK);
    let body = to_bytes(list_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(value.as_array().unwrap().len(), 1);
    assert_eq!(value[0]["label"], "acc-chatgpt");
    assert_eq!(value[0]["mode"], "chat_gpt_session");
}

#[tokio::test]
async fn data_plane_snapshot_reflects_account_changes() {
    let app = build_app();
    let admin_token = login_admin_token(&app).await;

    let initial = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/data-plane/snapshot")
                .header(
                    "authorization",
                    format!("Bearer {}", internal_service_token()),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(initial.status(), StatusCode::OK);
    let initial_body = to_bytes(initial.into_body(), usize::MAX).await.unwrap();
    let initial_json: Value = serde_json::from_slice(&initial_body).unwrap();
    assert_eq!(initial_json["accounts"].as_array().unwrap().len(), 0);
    let initial_revision = initial_json["revision"].as_u64().unwrap();

    let _create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/upstream-accounts")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({
                        "label": "acc-2",
                        "mode": "open_ai_api_key",
                        "base_url": "https://api.openai.com/v1",
                        "bearer_token": "tok-2",
                        "enabled": true,
                        "priority": 10
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let updated = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/data-plane/snapshot")
                .header(
                    "authorization",
                    format!("Bearer {}", internal_service_token()),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(updated.status(), StatusCode::OK);
    let updated_body = to_bytes(updated.into_body(), usize::MAX).await.unwrap();
    let updated_json: Value = serde_json::from_slice(&updated_body).unwrap();
    assert_eq!(updated_json["accounts"].as_array().unwrap().len(), 1);
    assert!(updated_json["revision"].as_u64().unwrap() > initial_revision);
}

#[tokio::test]
async fn validate_api_key_returns_principal() {
    let store = InMemoryStore::default();
    let tenant = store
        .create_tenant(CreateTenantRequest {
            name: "tenant-auth".to_string(),
        })
        .await
        .unwrap();
    let created = store
        .create_api_key(CreateApiKeyRequest {
            tenant_id: tenant.id,
            name: "auth-key".to_string(),
        })
        .await
        .unwrap();

    let principal = store
        .validate_api_key(&created.plaintext_key)
        .await
        .unwrap()
        .expect("api key should be valid");

    assert_eq!(principal.tenant_id, tenant.id);
    assert_eq!(principal.api_key_id, created.record.id);
    assert!(principal.enabled);
}

#[tokio::test]
async fn internal_auth_validate_endpoint() {
    let app = build_app();
    let admin_token = login_admin_token(&app).await;

    let tenant_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tenants")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(r#"{"name":"tenant-auth"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(tenant_response.status(), StatusCode::OK);
    let tenant_body = to_bytes(tenant_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let tenant_json: Value = serde_json::from_slice(&tenant_body).unwrap();
    let tenant_id = tenant_json["id"].as_str().unwrap();

    let key_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/api-keys")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({"tenant_id": tenant_id, "name": "auth-key"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(key_response.status(), StatusCode::OK);
    let key_body = to_bytes(key_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let key_json: Value = serde_json::from_slice(&key_body).unwrap();
    let token = key_json["plaintext_key"].as_str().unwrap().to_string();
    let api_key_id = key_json["record"]["id"].as_str().unwrap().to_string();

    let validate_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/internal/v1/auth/validate")
                .header("content-type", "application/json")
                .header(
                    "authorization",
                    format!("Bearer {}", internal_service_token()),
                )
                .body(Body::from(json!({ "token": token }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(validate_response.status(), StatusCode::OK);
    let validate_body = to_bytes(validate_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let validate_json: Value = serde_json::from_slice(&validate_body).unwrap();
    assert_eq!(validate_json["tenant_id"], tenant_id);
    assert_eq!(validate_json["api_key_id"], api_key_id);
    assert_eq!(validate_json["enabled"], true);
    assert!(validate_json["cache_ttl_sec"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn internal_auth_validate_route_requires_internal_service_token() {
    let app = build_app();
    let admin_token = login_admin_token(&app).await;

    let tenant_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tenants")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(r#"{"name":"tenant-auth-guard"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(tenant_response.status(), StatusCode::OK);
    let tenant_body = to_bytes(tenant_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let tenant_json: Value = serde_json::from_slice(&tenant_body).unwrap();
    let tenant_id = tenant_json["id"].as_str().unwrap();

    let key_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/api-keys")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::from(
                    json!({"tenant_id": tenant_id, "name": "auth-key"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(key_response.status(), StatusCode::OK);
    let key_body = to_bytes(key_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let key_json: Value = serde_json::from_slice(&key_body).unwrap();
    let token = key_json["plaintext_key"].as_str().unwrap().to_string();

    let validate_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/internal/v1/auth/validate")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "token": token }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(validate_response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn internal_auth_validate_endpoint_rejects_unknown_token() {
    let app = build_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/internal/v1/auth/validate")
                .header("content-type", "application/json")
                .header(
                    "authorization",
                    format!("Bearer {}", internal_service_token()),
                )
                .body(Body::from(r#"{"token":"cp_missing"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["error"]["code"], "unauthorized");
}

#[derive(Clone)]
struct StaticOAuthTokenClient;

#[async_trait]
impl OAuthTokenClient for StaticOAuthTokenClient {
    async fn refresh_token(
        &self,
        _refresh_token: &str,
        _base_url: Option<&str>,
    ) -> Result<OAuthTokenInfo, control_plane::oauth::OAuthTokenClientError> {
        Ok(OAuthTokenInfo {
            access_token: "access-from-oauth".to_string(),
            refresh_token: "refresh-from-oauth".to_string(),
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(3600),
            token_type: Some("Bearer".to_string()),
            scope: Some("model.read".to_string()),
            email: Some("oauth@example.com".to_string()),
            oauth_subject: Some("auth0|api".to_string()),
            oauth_identity_provider: Some("google-oauth2".to_string()),
            email_verified: Some(true),
            chatgpt_account_id: Some("acct_from_oauth".to_string()),
            chatgpt_user_id: Some("user_api".to_string()),
            chatgpt_plan_type: Some("pro".to_string()),
            chatgpt_subscription_active_start: None,
            chatgpt_subscription_active_until: None,
            chatgpt_subscription_last_checked: None,
            chatgpt_account_user_id: Some("acct_user_api".to_string()),
            chatgpt_compute_residency: Some("us".to_string()),
            organizations: None,
            groups: None,
        })
    }
}

#[derive(Clone)]
struct ReusedOAuthTokenClient;

#[async_trait]
impl OAuthTokenClient for ReusedOAuthTokenClient {
    async fn refresh_token(
        &self,
        _refresh_token: &str,
        _base_url: Option<&str>,
    ) -> Result<OAuthTokenInfo, OAuthTokenClientError> {
        Err(OAuthTokenClientError::InvalidRefreshToken {
            code: OAuthRefreshErrorCode::RefreshTokenReused,
            message: "refresh_token_reused".to_string(),
        })
    }
}
