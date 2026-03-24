use base64::Engine;
use chrono::{Duration, Utc};
use codex_pool_core::api::DataPlaneSnapshotEventType;
use codex_pool_core::model::{
    LocalizedErrorTemplates, RoutingStrategy, UpstreamErrorAction, UpstreamErrorRetryScope,
    UpstreamErrorTemplateRecord, UpstreamErrorTemplateStatus, UpstreamMode,
};
use control_plane::contracts::{
    CreateApiKeyRequest, CreateTenantRequest, CreateUpstreamAccountRequest,
    ImportOAuthRefreshTokenRequest, OAuthImportItemStatus, OAuthImportJobItem,
    OAuthImportJobStatus, OAuthImportJobSummary, OAuthInventorySummaryResponse,
    OAuthRateLimitSnapshot, OAuthRateLimitWindow, OAuthRefreshStatus, OAuthVaultRecordStatus,
    UpdateAiErrorLearningSettingsRequest, UpsertRetryPolicyRequest, UpsertRoutingPolicyRequest,
    UpsertStreamRetryPolicyRequest,
};
use control_plane::crypto::CredentialCipher;
use control_plane::import_jobs::{
    ImportTaskRequest, OAuthImportJobStore, PersistedImportItem, PostgresOAuthImportJobStore,
};
use control_plane::oauth::{
    OAuthRefreshErrorCode, OAuthTokenClient, OAuthTokenClientError, OAuthTokenInfo,
};
use control_plane::store::postgres::PostgresStore;
use control_plane::store::ControlPlaneStore;
use control_plane::tenant::{AdminImpersonateRequest, TenantAuthService};
use serde_json::json;
use sqlx_core::executor::Executor;
use sqlx_core::query::query;
use sqlx_core::query_scalar::query_scalar;
use sqlx_postgres::PgPoolOptions;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, Mutex as StdMutex};
use std::time::Instant;
use tokio::sync::Mutex as AsyncMutex;
use uuid::Uuid;

fn test_db_url() -> Option<String> {
    std::env::var("CONTROL_PLANE_DATABASE_URL")
        .ok()
        .or_else(|| std::env::var("DATABASE_URL").ok())
}

static ENV_LOCK: LazyLock<AsyncMutex<()>> = LazyLock::new(|| AsyncMutex::new(()));

fn admin_db_url(database_url: &str) -> String {
    let mut parsed = reqwest::Url::parse(database_url).expect("valid postgres database url");
    parsed.set_path("/postgres");
    parsed.to_string()
}

fn quoted_database_identifier(database_name: &str) -> String {
    assert!(
        database_name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_'),
        "database name must stay ASCII-safe for quoted SQL identifiers"
    );
    format!("\"{database_name}\"")
}

fn child_database_url(database_url: &str, database_name: &str) -> String {
    let mut parsed = reqwest::Url::parse(database_url).expect("valid postgres database url");
    parsed.set_path(&format!("/{database_name}"));
    parsed.to_string()
}

async fn create_temp_database(database_url: &str, database_name: &str) {
    let admin_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&admin_db_url(database_url))
        .await
        .expect("connect admin postgres");
    let quoted_name = quoted_database_identifier(database_name);
    query(&format!("DROP DATABASE IF EXISTS {quoted_name}"))
        .execute(&admin_pool)
        .await
        .expect("drop leftover temp database");
    query(&format!("CREATE DATABASE {quoted_name}"))
        .execute(&admin_pool)
        .await
        .expect("create temp database");
    admin_pool.close().await;
}

async fn drop_temp_database(database_url: &str, database_name: &str) {
    let admin_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&admin_db_url(database_url))
        .await
        .expect("connect admin postgres");
    query(
        r#"
        SELECT pg_terminate_backend(pid)
        FROM pg_stat_activity
        WHERE datname = $1
          AND pid <> pg_backend_pid()
        "#,
    )
    .bind(database_name)
    .execute(&admin_pool)
    .await
    .expect("terminate temp database connections");
    let quoted_name = quoted_database_identifier(database_name);
    query(&format!("DROP DATABASE IF EXISTS {quoted_name}"))
        .execute(&admin_pool)
        .await
        .expect("drop temp database");
    admin_pool.close().await;
}

#[derive(Default)]
struct RefreshThenRateLimitOAuthClient {
    refresh_calls: AtomicUsize,
    fetch_calls: AtomicUsize,
}

#[async_trait::async_trait]
impl OAuthTokenClient for RefreshThenRateLimitOAuthClient {
    async fn refresh_token(
        &self,
        _refresh_token: &str,
        _base_url: Option<&str>,
    ) -> Result<OAuthTokenInfo, OAuthTokenClientError> {
        let call_no = self.refresh_calls.fetch_add(1, Ordering::SeqCst);
        let (access_token, refresh_token) = if call_no == 0 {
            ("stale-access", "stale-refresh")
        } else {
            ("fresh-access", "fresh-refresh")
        };
        Ok(OAuthTokenInfo {
            access_token: access_token.to_string(),
            refresh_token: refresh_token.to_string(),
            expires_at: Utc::now() + Duration::hours(1),
            token_type: Some("Bearer".to_string()),
            scope: Some("model.read".to_string()),
            email: Some("rate-limit@example.com".to_string()),
            oauth_subject: Some("auth0|rate-limit".to_string()),
            oauth_identity_provider: Some("google-oauth2".to_string()),
            email_verified: Some(true),
            chatgpt_account_id: Some("acct-rate-limit-refresh".to_string()),
            chatgpt_user_id: Some("user-rate-limit".to_string()),
            chatgpt_plan_type: Some("team".to_string()),
            chatgpt_subscription_active_start: Some(
                chrono::DateTime::parse_from_rfc3339("2026-03-01T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
            chatgpt_subscription_active_until: Some(
                chrono::DateTime::parse_from_rfc3339("2026-04-01T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
            chatgpt_subscription_last_checked: Some(
                chrono::DateTime::parse_from_rfc3339("2026-03-11T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
            chatgpt_account_user_id: Some("acct_user_rate_limit".to_string()),
            chatgpt_compute_residency: Some("us".to_string()),
            workspace_name: Some("OAI-03.09".to_string()),
            organizations: Some(vec![json!({
                "id": "org_rate_limit",
                "title": "Personal",
            })]),
            groups: Some(vec![json!({
                "id": "grp_rate_limit",
                "name": "Rate Limit Team",
            })]),
        })
    }

    async fn fetch_rate_limits(
        &self,
        access_token: &str,
        _base_url: Option<&str>,
        _chatgpt_account_id: Option<&str>,
    ) -> Result<Vec<OAuthRateLimitSnapshot>, OAuthTokenClientError> {
        self.fetch_calls.fetch_add(1, Ordering::SeqCst);
        match access_token {
            "stale-access" => Err(OAuthTokenClientError::Upstream {
                code: OAuthRefreshErrorCode::InvalidRefreshToken,
                message: "stale access token".to_string(),
            }),
            "fresh-access" => Ok(vec![OAuthRateLimitSnapshot {
                limit_id: Some("codex".to_string()),
                limit_name: Some("Codex".to_string()),
                primary: Some(OAuthRateLimitWindow {
                    used_percent: 12.5,
                    window_minutes: Some(5),
                    resets_at: Some(Utc::now() + Duration::minutes(5)),
                }),
                secondary: None,
            }]),
            _ => Err(OAuthTokenClientError::Upstream {
                code: OAuthRefreshErrorCode::Unknown,
                message: "unexpected token".to_string(),
            }),
        }
    }
}

#[derive(Default)]
struct SuccessThenQuotaOAuthClient {
    refresh_calls: AtomicUsize,
    fetch_calls: AtomicUsize,
}

#[async_trait::async_trait]
impl OAuthTokenClient for SuccessThenQuotaOAuthClient {
    async fn refresh_token(
        &self,
        _refresh_token: &str,
        _base_url: Option<&str>,
    ) -> Result<OAuthTokenInfo, OAuthTokenClientError> {
        let call_no = self.refresh_calls.fetch_add(1, Ordering::SeqCst);
        Ok(OAuthTokenInfo {
            access_token: format!("test-outbox-token-{call_no}"),
            refresh_token: format!("test-outbox-refresh-{call_no}"),
            expires_at: Utc::now() + Duration::hours(1),
            token_type: Some("Bearer".to_string()),
            scope: Some("model.read".to_string()),
            email: Some("outbox@example.com".to_string()),
            oauth_subject: Some("auth0|outbox".to_string()),
            oauth_identity_provider: Some("google-oauth2".to_string()),
            email_verified: Some(true),
            chatgpt_account_id: Some("acct-outbox".to_string()),
            chatgpt_user_id: Some("user-outbox".to_string()),
            chatgpt_plan_type: Some("plus".to_string()),
            chatgpt_subscription_active_start: None,
            chatgpt_subscription_active_until: None,
            chatgpt_subscription_last_checked: None,
            chatgpt_account_user_id: Some("acct_user_outbox".to_string()),
            chatgpt_compute_residency: Some("eu".to_string()),
            workspace_name: None,
            organizations: Some(vec![json!({
                "id": "org_outbox",
                "title": "Personal",
            })]),
            groups: Some(vec![]),
        })
    }

    async fn fetch_rate_limits(
        &self,
        access_token: &str,
        _base_url: Option<&str>,
        _chatgpt_account_id: Option<&str>,
    ) -> Result<Vec<OAuthRateLimitSnapshot>, OAuthTokenClientError> {
        let call_no = self.fetch_calls.fetch_add(1, Ordering::SeqCst);
        if access_token.starts_with("test-outbox-token-") && call_no > 0 {
            return Err(OAuthTokenClientError::Upstream {
                code: OAuthRefreshErrorCode::RateLimited,
                message: "usage limit reached".to_string(),
            });
        }

        Ok(vec![OAuthRateLimitSnapshot {
            limit_id: Some("codex".to_string()),
            limit_name: Some("Codex".to_string()),
            primary: Some(OAuthRateLimitWindow {
                used_percent: 8.0,
                window_minutes: Some(5),
                resets_at: Some(Utc::now() + Duration::minutes(5)),
            }),
            secondary: None,
        }])
    }
}

#[derive(Default)]
struct RecordingOAuthClient {
    refresh_calls: AtomicUsize,
    record_fetches: AtomicBool,
    fetch_started_at: StdMutex<Vec<Instant>>,
}

impl RecordingOAuthClient {
    fn enable_recording(&self) {
        self.record_fetches.store(true, Ordering::SeqCst);
        self.fetch_started_at
            .lock()
            .expect("recording mutex poisoned")
            .clear();
    }

    fn fetch_starts(&self) -> Vec<Instant> {
        self.fetch_started_at
            .lock()
            .expect("recording mutex poisoned")
            .clone()
    }
}

#[derive(Default)]
struct SharedAccountIdRecordingOAuthClient;

#[async_trait::async_trait]
impl OAuthTokenClient for SharedAccountIdRecordingOAuthClient {
    async fn refresh_token(
        &self,
        refresh_token: &str,
        _base_url: Option<&str>,
    ) -> Result<OAuthTokenInfo, OAuthTokenClientError> {
        let (email, account_user_id) = if refresh_token.contains("workspace-b") {
            (
                "shared-workspace-b@example.com",
                "acct_user_shared_workspace_b",
            )
        } else {
            (
                "shared-workspace-a@example.com",
                "acct_user_shared_workspace_a",
            )
        };
        Ok(OAuthTokenInfo {
            access_token: format!("shared-access-{refresh_token}"),
            refresh_token: refresh_token.to_string(),
            expires_at: Utc::now() + Duration::hours(1),
            token_type: Some("Bearer".to_string()),
            scope: Some("model.read".to_string()),
            email: Some(email.to_string()),
            oauth_subject: Some("auth0|shared".to_string()),
            oauth_identity_provider: Some("google-oauth2".to_string()),
            email_verified: Some(true),
            chatgpt_account_id: Some("acct-shared-workspace".to_string()),
            chatgpt_user_id: Some("user-shared".to_string()),
            chatgpt_plan_type: Some("team".to_string()),
            chatgpt_subscription_active_start: None,
            chatgpt_subscription_active_until: None,
            chatgpt_subscription_last_checked: None,
            chatgpt_account_user_id: Some(account_user_id.to_string()),
            chatgpt_compute_residency: Some("us".to_string()),
            workspace_name: Some(
                if refresh_token.contains("workspace-b") {
                    "OAI-07.11"
                } else {
                    "OAI-03.09"
                }
                .to_string(),
            ),
            organizations: Some(vec![json!({
                "id": "org_shared",
                "title": "Personal",
            })]),
            groups: Some(vec![]),
        })
    }
}

#[derive(Default)]
struct WorkspaceProbeOAuthClient {
    fetch_workspace_name_calls: AtomicUsize,
}

#[async_trait::async_trait]
impl OAuthTokenClient for WorkspaceProbeOAuthClient {
    async fn refresh_token(
        &self,
        _refresh_token: &str,
        _base_url: Option<&str>,
    ) -> Result<OAuthTokenInfo, OAuthTokenClientError> {
        Ok(OAuthTokenInfo {
            access_token: "workspace-probe-access".to_string(),
            refresh_token: "workspace-probe-refresh".to_string(),
            expires_at: Utc::now() + Duration::hours(1),
            token_type: Some("Bearer".to_string()),
            scope: Some("model.read".to_string()),
            email: Some("workspace-probe@example.com".to_string()),
            oauth_subject: Some("auth0|workspace-probe".to_string()),
            oauth_identity_provider: Some("google-oauth2".to_string()),
            email_verified: Some(true),
            chatgpt_account_id: Some("acct-workspace-probe".to_string()),
            chatgpt_user_id: Some("user-workspace-probe".to_string()),
            chatgpt_plan_type: Some("team".to_string()),
            chatgpt_subscription_active_start: None,
            chatgpt_subscription_active_until: None,
            chatgpt_subscription_last_checked: None,
            chatgpt_account_user_id: Some("acct_user_workspace_probe".to_string()),
            chatgpt_compute_residency: Some("us".to_string()),
            workspace_name: None,
            organizations: Some(vec![json!({
                "id": "org_workspace_probe",
                "title": "Personal",
            })]),
            groups: Some(vec![]),
        })
    }

    async fn fetch_workspace_name(
        &self,
        access_token: &str,
        _base_url: Option<&str>,
        chatgpt_account_id: Option<&str>,
    ) -> Result<Option<String>, OAuthTokenClientError> {
        self.fetch_workspace_name_calls
            .fetch_add(1, Ordering::SeqCst);
        if access_token == "workspace-probe-access"
            && chatgpt_account_id == Some("acct-workspace-probe")
        {
            return Ok(Some("OAI-03.09".to_string()));
        }

        Ok(None)
    }
}

#[async_trait::async_trait]
impl OAuthTokenClient for RecordingOAuthClient {
    async fn refresh_token(
        &self,
        _refresh_token: &str,
        _base_url: Option<&str>,
    ) -> Result<OAuthTokenInfo, OAuthTokenClientError> {
        let call_no = self.refresh_calls.fetch_add(1, Ordering::SeqCst);
        Ok(OAuthTokenInfo {
            access_token: format!("test-recording-token-{call_no}"),
            refresh_token: format!("test-recording-refresh-{call_no}"),
            expires_at: Utc::now() + Duration::hours(1),
            token_type: Some("Bearer".to_string()),
            scope: Some("model.read".to_string()),
            email: Some("recording@example.com".to_string()),
            oauth_subject: Some("auth0|recording".to_string()),
            oauth_identity_provider: Some("google-oauth2".to_string()),
            email_verified: Some(true),
            chatgpt_account_id: Some("acct-recording".to_string()),
            chatgpt_user_id: Some("user-recording".to_string()),
            chatgpt_plan_type: Some("team".to_string()),
            chatgpt_subscription_active_start: None,
            chatgpt_subscription_active_until: None,
            chatgpt_subscription_last_checked: None,
            chatgpt_account_user_id: Some("acct_user_recording".to_string()),
            chatgpt_compute_residency: Some("us".to_string()),
            workspace_name: Some("OAI-REC".to_string()),
            organizations: Some(vec![json!({
                "id": "org_recording",
                "title": "Personal",
            })]),
            groups: Some(vec![]),
        })
    }

    async fn fetch_rate_limits(
        &self,
        _access_token: &str,
        _base_url: Option<&str>,
        _chatgpt_account_id: Option<&str>,
    ) -> Result<Vec<OAuthRateLimitSnapshot>, OAuthTokenClientError> {
        if self.record_fetches.load(Ordering::SeqCst) {
            self.fetch_started_at
                .lock()
                .expect("recording mutex poisoned")
                .push(Instant::now());
        }

        Ok(vec![OAuthRateLimitSnapshot {
            limit_id: Some("codex".to_string()),
            limit_name: Some("Codex".to_string()),
            primary: Some(OAuthRateLimitWindow {
                used_percent: 5.0,
                window_minutes: Some(5),
                resets_at: Some(Utc::now() + Duration::minutes(5)),
            }),
            secondary: None,
        }])
    }
}

#[derive(Default)]
struct AdmissionProbeOAuthClient {
    refresh_calls: AtomicUsize,
    fetch_calls: AtomicUsize,
}

#[async_trait::async_trait]
impl OAuthTokenClient for AdmissionProbeOAuthClient {
    async fn refresh_token(
        &self,
        refresh_token: &str,
        _base_url: Option<&str>,
    ) -> Result<OAuthTokenInfo, OAuthTokenClientError> {
        self.refresh_calls.fetch_add(1, Ordering::SeqCst);
        let account_suffix = refresh_token
            .rsplit('-')
            .next()
            .filter(|value| !value.is_empty())
            .unwrap_or("default");
        Ok(OAuthTokenInfo {
            access_token: format!("activation-access-{account_suffix}"),
            refresh_token: refresh_token.to_string(),
            expires_at: Utc::now() + Duration::hours(1),
            token_type: Some("Bearer".to_string()),
            scope: Some("model.read".to_string()),
            email: Some(format!("activation-{account_suffix}@example.com")),
            oauth_subject: Some(format!("auth0|activation-{account_suffix}")),
            oauth_identity_provider: Some("google-oauth2".to_string()),
            email_verified: Some(true),
            chatgpt_account_id: Some(format!("acct-activation-{account_suffix}")),
            chatgpt_user_id: Some(format!("user-activation-{account_suffix}")),
            chatgpt_plan_type: Some("team".to_string()),
            chatgpt_subscription_active_start: None,
            chatgpt_subscription_active_until: None,
            chatgpt_subscription_last_checked: None,
            chatgpt_account_user_id: Some(format!("acct_user_activation_{account_suffix}")),
            chatgpt_compute_residency: Some("us".to_string()),
            workspace_name: Some("OAI-ACTIVE".to_string()),
            organizations: Some(vec![json!({
                "id": format!("org_activation_{account_suffix}"),
                "title": "Personal",
            })]),
            groups: Some(vec![]),
        })
    }

    async fn fetch_rate_limits(
        &self,
        access_token: &str,
        _base_url: Option<&str>,
        _chatgpt_account_id: Option<&str>,
    ) -> Result<Vec<OAuthRateLimitSnapshot>, OAuthTokenClientError> {
        self.fetch_calls.fetch_add(1, Ordering::SeqCst);
        if access_token.starts_with("fallback-error") {
            return Err(OAuthTokenClientError::Upstream {
                code: OAuthRefreshErrorCode::InvalidRefreshToken,
                message: "fallback access token rejected".to_string(),
            });
        }

        let used_percent = if access_token.starts_with("fallback-quota") {
            100.0
        } else {
            12.5
        };
        Ok(vec![OAuthRateLimitSnapshot {
            limit_id: Some("codex".to_string()),
            limit_name: Some("Codex".to_string()),
            primary: Some(OAuthRateLimitWindow {
                used_percent,
                window_minutes: Some(5),
                resets_at: Some(Utc::now() + Duration::minutes(5)),
            }),
            secondary: None,
        }])
    }
}

async fn expire_rate_limit_snapshot(repo: &PostgresStore, account_id: Uuid) {
    let pool = repo.clone_pool();
    query(
        r#"
        UPDATE upstream_account_rate_limit_snapshots
        SET expires_at = now() - interval '1 second',
            updated_at = now()
        WHERE account_id = $1
        "#,
    )
    .bind(account_id)
    .execute(&pool)
    .await
    .expect("expire rate-limit snapshot");
}

async fn create_temp_repo_with_oauth(
    database_url: &str,
    name_prefix: &str,
    oauth_client: Arc<dyn OAuthTokenClient>,
    cipher: CredentialCipher,
) -> (PostgresStore, String) {
    let database_name = format!("{name_prefix}_{}", Uuid::new_v4().simple());
    create_temp_database(database_url, &database_name).await;
    let temp_db_url = child_database_url(database_url, &database_name);
    let repo = PostgresStore::connect_with_oauth(&temp_db_url, oauth_client, Some(cipher))
        .await
        .expect("connect temp postgres repo");
    (repo, database_name)
}

#[tokio::test]
async fn postgres_repo_queue_oauth_refresh_token_marks_ready_inventory_without_refresh() {
    let Some(db_url) = test_db_url() else {
        eprintln!(
            "skip postgres_repo_queue_oauth_refresh_token_marks_ready_inventory_without_refresh: set CONTROL_PLANE_DATABASE_URL"
        );
        return;
    };

    let cipher_key = base64::engine::general_purpose::STANDARD.encode([21_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let oauth_client = Arc::new(AdmissionProbeOAuthClient::default());
    let (repo, database_name) = create_temp_repo_with_oauth(
        &db_url,
        "cp_pg_ready_inventory",
        oauth_client.clone(),
        cipher,
    )
    .await;

    let inserted = repo
        .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
            label: format!("oauth-ready-{}", Uuid::new_v4().simple()),
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            refresh_token: format!("rt-ready-{}", Uuid::new_v4().simple()),
            fallback_access_token: Some("fallback-ready-token".to_string()),
            fallback_token_expires_at: Some(Utc::now() + Duration::hours(1)),
            chatgpt_account_id: Some(format!("acct-ready-{}", Uuid::new_v4().simple())),
            mode: Some(UpstreamMode::CodexOauth),
            enabled: Some(true),
            priority: Some(100),
            chatgpt_plan_type: Some("team".to_string()),
            source_type: Some("codex".to_string()),
        })
        .await
        .unwrap();

    let summary = repo.oauth_inventory_summary().await.unwrap();
    let records = repo.oauth_inventory_records().await.unwrap();
    let snapshot = repo.snapshot().await.unwrap();
    drop_temp_database(&db_url, &database_name).await;

    assert!(inserted);
    assert_eq!(
        summary,
        OAuthInventorySummaryResponse {
            total: 1,
            queued: 0,
            ready: 1,
            needs_refresh: 0,
            no_quota: 0,
            failed: 0,
        }
    );
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].vault_status, OAuthVaultRecordStatus::Ready);
    assert!(records[0].admission_checked_at.is_some());
    assert!(records[0].admission_rate_limits_expires_at.is_some());
    assert_eq!(snapshot.accounts.len(), 0);
    assert_eq!(oauth_client.refresh_calls.load(Ordering::SeqCst), 0);
    assert_eq!(oauth_client.fetch_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn postgres_repo_queue_oauth_refresh_token_marks_no_quota_inventory() {
    let Some(db_url) = test_db_url() else {
        eprintln!(
            "skip postgres_repo_queue_oauth_refresh_token_marks_no_quota_inventory: set CONTROL_PLANE_DATABASE_URL"
        );
        return;
    };

    let cipher_key = base64::engine::general_purpose::STANDARD.encode([22_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let oauth_client = Arc::new(AdmissionProbeOAuthClient::default());
    let (repo, database_name) = create_temp_repo_with_oauth(
        &db_url,
        "cp_pg_no_quota_inventory",
        oauth_client.clone(),
        cipher,
    )
    .await;

    repo.queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
        label: format!("oauth-no-quota-{}", Uuid::new_v4().simple()),
        base_url: "https://chatgpt.com/backend-api/codex".to_string(),
        refresh_token: format!("rt-no-quota-{}", Uuid::new_v4().simple()),
        fallback_access_token: Some("fallback-quota-token".to_string()),
        fallback_token_expires_at: Some(Utc::now() + Duration::hours(1)),
        chatgpt_account_id: Some(format!("acct-no-quota-{}", Uuid::new_v4().simple())),
        mode: Some(UpstreamMode::CodexOauth),
        enabled: Some(true),
        priority: Some(100),
        chatgpt_plan_type: Some("team".to_string()),
        source_type: Some("codex".to_string()),
    })
    .await
    .unwrap();

    let records = repo.oauth_inventory_records().await.unwrap();
    drop_temp_database(&db_url, &database_name).await;

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].vault_status, OAuthVaultRecordStatus::NoQuota);
    assert!(records[0].admission_retry_after.is_some());
    assert_eq!(
        records[0].admission_error_code.as_deref(),
        Some("primary_window_exhausted")
    );
    assert_eq!(oauth_client.refresh_calls.load(Ordering::SeqCst), 0);
    assert_eq!(oauth_client.fetch_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn postgres_repo_activation_uses_ready_without_refresh_and_needs_refresh_with_single_refresh()
{
    let Some(db_url) = test_db_url() else {
        eprintln!(
            "skip postgres_repo_activation_uses_ready_without_refresh_and_needs_refresh_with_single_refresh: set CONTROL_PLANE_DATABASE_URL"
        );
        return;
    };

    let _env_guard = ENV_LOCK.lock().await;
    let old_target = std::env::var("CONTROL_PLANE_ACTIVE_POOL_TARGET").ok();
    let old_min = std::env::var("CONTROL_PLANE_ACTIVE_POOL_MIN").ok();
    let old_batch = std::env::var("CONTROL_PLANE_VAULT_ACTIVATE_BATCH_SIZE").ok();
    let old_concurrency = std::env::var("CONTROL_PLANE_VAULT_ACTIVATE_CONCURRENCY").ok();
    let old_max_rps = std::env::var("CONTROL_PLANE_VAULT_ACTIVATE_MAX_RPS").ok();
    std::env::set_var("CONTROL_PLANE_ACTIVE_POOL_TARGET", "2");
    std::env::set_var("CONTROL_PLANE_ACTIVE_POOL_MIN", "0");
    std::env::set_var("CONTROL_PLANE_VAULT_ACTIVATE_BATCH_SIZE", "4");
    std::env::set_var("CONTROL_PLANE_VAULT_ACTIVATE_CONCURRENCY", "1");
    std::env::set_var("CONTROL_PLANE_VAULT_ACTIVATE_MAX_RPS", "10");

    let cipher_key = base64::engine::general_purpose::STANDARD.encode([23_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let oauth_client = Arc::new(AdmissionProbeOAuthClient::default());
    let (repo, database_name) = create_temp_repo_with_oauth(
        &db_url,
        "cp_pg_activate_inventory",
        oauth_client.clone(),
        cipher,
    )
    .await;

    repo.queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
        label: format!("oauth-ready-activate-{}", Uuid::new_v4().simple()),
        base_url: "https://chatgpt.com/backend-api/codex".to_string(),
        refresh_token: format!("rt-ready-activate-{}", Uuid::new_v4().simple()),
        fallback_access_token: Some("fallback-ready-activate".to_string()),
        fallback_token_expires_at: Some(Utc::now() + Duration::hours(1)),
        chatgpt_account_id: Some(format!("acct-ready-activate-{}", Uuid::new_v4().simple())),
        mode: Some(UpstreamMode::CodexOauth),
        enabled: Some(true),
        priority: Some(100),
        chatgpt_plan_type: Some("team".to_string()),
        source_type: Some("codex".to_string()),
    })
    .await
    .unwrap();
    repo.queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
        label: format!("oauth-needs-refresh-{}", Uuid::new_v4().simple()),
        base_url: "https://chatgpt.com/backend-api/codex".to_string(),
        refresh_token: format!("rt-needs-refresh-{}", Uuid::new_v4().simple()),
        fallback_access_token: Some("fallback-needs-refresh".to_string()),
        fallback_token_expires_at: None,
        chatgpt_account_id: Some(format!("acct-needs-refresh-{}", Uuid::new_v4().simple())),
        mode: Some(UpstreamMode::CodexOauth),
        enabled: Some(true),
        priority: Some(100),
        chatgpt_plan_type: Some("team".to_string()),
        source_type: Some("codex".to_string()),
    })
    .await
    .unwrap();

    let activated = repo.activate_oauth_refresh_token_vault().await.unwrap();
    let inventory = repo.oauth_inventory_records().await.unwrap();
    let snapshot = repo.snapshot().await.unwrap();

    if let Some(value) = old_target.as_deref() {
        std::env::set_var("CONTROL_PLANE_ACTIVE_POOL_TARGET", value);
    } else {
        std::env::remove_var("CONTROL_PLANE_ACTIVE_POOL_TARGET");
    }
    if let Some(value) = old_min.as_deref() {
        std::env::set_var("CONTROL_PLANE_ACTIVE_POOL_MIN", value);
    } else {
        std::env::remove_var("CONTROL_PLANE_ACTIVE_POOL_MIN");
    }
    if let Some(value) = old_batch.as_deref() {
        std::env::set_var("CONTROL_PLANE_VAULT_ACTIVATE_BATCH_SIZE", value);
    } else {
        std::env::remove_var("CONTROL_PLANE_VAULT_ACTIVATE_BATCH_SIZE");
    }
    if let Some(value) = old_concurrency.as_deref() {
        std::env::set_var("CONTROL_PLANE_VAULT_ACTIVATE_CONCURRENCY", value);
    } else {
        std::env::remove_var("CONTROL_PLANE_VAULT_ACTIVATE_CONCURRENCY");
    }
    if let Some(value) = old_max_rps.as_deref() {
        std::env::set_var("CONTROL_PLANE_VAULT_ACTIVATE_MAX_RPS", value);
    } else {
        std::env::remove_var("CONTROL_PLANE_VAULT_ACTIVATE_MAX_RPS");
    }

    drop_temp_database(&db_url, &database_name).await;

    assert_eq!(activated, 2);
    assert!(inventory.is_empty());
    assert_eq!(snapshot.accounts.len(), 2);
    assert_eq!(oauth_client.refresh_calls.load(Ordering::SeqCst), 1);
    assert_eq!(oauth_client.fetch_calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn postgres_repo_refetches_rate_limits_after_forced_oauth_refresh() {
    let Some(db_url) = test_db_url() else {
        eprintln!(
            "skip postgres_repo_refetches_rate_limits_after_forced_oauth_refresh: set CONTROL_PLANE_DATABASE_URL"
        );
        return;
    };

    let cipher_key = base64::engine::general_purpose::STANDARD.encode([3_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let oauth_client = Arc::new(RefreshThenRateLimitOAuthClient::default());
    let repo = PostgresStore::connect_with_oauth(&db_url, oauth_client.clone(), Some(cipher))
        .await
        .unwrap();

    let account = repo
        .import_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
            label: format!("oauth-rate-limit-refresh-{}", Uuid::new_v4().simple()),
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            refresh_token: format!("rt-{}", Uuid::new_v4().simple()),
            chatgpt_account_id: Some(format!("acct-{}", Uuid::new_v4().simple())),
            mode: Some(UpstreamMode::ChatGptSession),
            enabled: Some(true),
            priority: Some(100),
            chatgpt_plan_type: None,
            source_type: None,
        })
        .await
        .unwrap();

    let refreshed = repo.refresh_due_oauth_rate_limit_caches().await.unwrap();
    assert!(refreshed >= 1);

    let status = repo.oauth_account_status(account.id).await.unwrap();
    assert_eq!(status.last_refresh_status, OAuthRefreshStatus::Ok);
    assert!(status.rate_limits_fetched_at.is_some());
    assert!(status.rate_limits_last_error_code.is_none());
    assert!(!status.rate_limits.is_empty());
    assert_eq!(status.rate_limits[0].limit_id.as_deref(), Some("codex"));

    assert!(oauth_client.refresh_calls.load(Ordering::SeqCst) >= 2);
    assert!(oauth_client.fetch_calls.load(Ordering::SeqCst) >= 2);
}

#[tokio::test]
async fn postgres_repo_oauth_status_exposes_email() {
    let Some(db_url) = test_db_url() else {
        eprintln!("skip postgres_repo_oauth_status_exposes_email: set CONTROL_PLANE_DATABASE_URL");
        return;
    };

    let cipher_key = base64::engine::general_purpose::STANDARD.encode([13_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let oauth_client = Arc::new(RefreshThenRateLimitOAuthClient::default());
    let repo = PostgresStore::connect_with_oauth(&db_url, oauth_client, Some(cipher))
        .await
        .unwrap();

    let account = repo
        .import_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
            label: format!("oauth-email-{}", Uuid::new_v4().simple()),
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            refresh_token: format!("rt-email-{}", Uuid::new_v4().simple()),
            chatgpt_account_id: None,
            mode: Some(UpstreamMode::CodexOauth),
            enabled: Some(true),
            priority: Some(100),
            chatgpt_plan_type: None,
            source_type: Some("codex".to_string()),
        })
        .await
        .unwrap();

    let status = repo.oauth_account_status(account.id).await.unwrap();
    assert_eq!(status.email.as_deref(), Some("rate-limit@example.com"));
    assert_eq!(status.oauth_subject.as_deref(), Some("auth0|rate-limit"));
    assert_eq!(
        status.oauth_identity_provider.as_deref(),
        Some("google-oauth2")
    );
    assert_eq!(status.email_verified, Some(true));
    assert_eq!(status.chatgpt_user_id.as_deref(), Some("user-rate-limit"));
    assert_eq!(
        status.chatgpt_account_user_id.as_deref(),
        Some("acct_user_rate_limit")
    );
    assert_eq!(status.chatgpt_compute_residency.as_deref(), Some("us"));
    assert_eq!(status.workspace_name.as_deref(), Some("OAI-03.09"));
    assert_eq!(status.organizations.as_ref().map(Vec::len), Some(1));
    assert_eq!(status.groups.as_ref().map(Vec::len), Some(1));
}

#[tokio::test]
async fn postgres_repo_oauth_statuses_backfill_team_workspace_name_from_probe() {
    let Some(db_url) = test_db_url() else {
        eprintln!(
            "skip postgres_repo_oauth_statuses_backfill_team_workspace_name_from_probe: set CONTROL_PLANE_DATABASE_URL"
        );
        return;
    };

    let cipher_key = base64::engine::general_purpose::STANDARD.encode([14_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let oauth_client = Arc::new(WorkspaceProbeOAuthClient::default());
    let repo = PostgresStore::connect_with_oauth(&db_url, oauth_client.clone(), Some(cipher))
        .await
        .unwrap();

    let account = repo
        .import_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
            label: format!("oauth-workspace-probe-{}", Uuid::new_v4().simple()),
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            refresh_token: format!("rt-workspace-probe-{}", Uuid::new_v4().simple()),
            chatgpt_account_id: None,
            mode: Some(UpstreamMode::CodexOauth),
            enabled: Some(true),
            priority: Some(100),
            chatgpt_plan_type: None,
            source_type: Some("codex".to_string()),
        })
        .await
        .unwrap();

    let statuses = repo.oauth_account_statuses(vec![account.id]).await.unwrap();
    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0].chatgpt_plan_type.as_deref(), Some("team"));
    assert_eq!(statuses[0].workspace_name.as_deref(), Some("OAI-03.09"));
    assert_eq!(
        oauth_client
            .fetch_workspace_name_calls
            .load(Ordering::SeqCst),
        1
    );
}

#[tokio::test]
async fn postgres_repo_bootstraps_empty_database() {
    let Some(db_url) = test_db_url() else {
        eprintln!("skip postgres_repo_bootstraps_empty_database: set CONTROL_PLANE_DATABASE_URL");
        return;
    };

    let database_name = format!("cp_empty_bootstrap_{}", Uuid::new_v4().simple());
    create_temp_database(&db_url, &database_name).await;
    let empty_db_url = child_database_url(&db_url, &database_name);

    let result = PostgresStore::connect(&empty_db_url).await;
    let connect_ok = result.is_ok();
    let error_text = result
        .as_ref()
        .err()
        .map(std::string::ToString::to_string)
        .unwrap_or_default();
    drop(result);
    drop_temp_database(&db_url, &database_name).await;

    assert!(
        connect_ok,
        "expected connect() to bootstrap a brand-new database, got {error_text}"
    );
}

#[tokio::test]
async fn postgres_repo_emits_snapshot_event_after_rate_limit_cache_failure() {
    let Some(db_url) = test_db_url() else {
        eprintln!(
            "skip postgres_repo_emits_snapshot_event_after_rate_limit_cache_failure: set CONTROL_PLANE_DATABASE_URL"
        );
        return;
    };

    let cipher_key = base64::engine::general_purpose::STANDARD.encode([7_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let oauth_client = Arc::new(SuccessThenQuotaOAuthClient::default());
    let repo = PostgresStore::connect_with_oauth(&db_url, oauth_client, Some(cipher))
        .await
        .unwrap();

    let account = repo
        .import_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
            label: format!("oauth-rate-limit-outbox-{}", Uuid::new_v4().simple()),
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            refresh_token: format!("rt-{}", Uuid::new_v4().simple()),
            chatgpt_account_id: Some(format!("acct-{}", Uuid::new_v4().simple())),
            mode: Some(UpstreamMode::ChatGptSession),
            enabled: Some(true),
            priority: Some(100),
            chatgpt_plan_type: None,
            source_type: None,
        })
        .await
        .unwrap();

    expire_rate_limit_snapshot(&repo, account.id).await;

    let snapshot_before = repo.snapshot().await.unwrap();
    let cursor_before = snapshot_before.cursor;

    let refreshed = repo.refresh_due_oauth_rate_limit_caches().await.unwrap();
    assert!(refreshed >= 1);

    let status = repo.oauth_account_status(account.id).await.unwrap();
    assert_eq!(
        status.rate_limits_last_error_code.as_deref(),
        Some("rate_limited")
    );
    assert!(status
        .rate_limits_last_error
        .as_deref()
        .unwrap_or_default()
        .contains("usage limit"));

    let snapshot_after = repo.snapshot().await.unwrap();
    let refreshed_account = snapshot_after
        .accounts
        .iter()
        .find(|item| item.id == account.id)
        .expect("account present in snapshot");
    assert!(!refreshed_account.enabled);

    let events = repo
        .data_plane_snapshot_events(cursor_before, 50)
        .await
        .unwrap();
    assert!(
        events.events.iter().any(|event| {
            event.account_id == account.id
                && matches!(event.event_type, DataPlaneSnapshotEventType::AccountUpsert)
        }),
        "expected account upsert event for refreshed rate-limit snapshot"
    );
}

#[tokio::test]
async fn postgres_repo_rate_limit_refresh_respects_global_max_rps() {
    let Some(db_url) = test_db_url() else {
        eprintln!(
            "skip postgres_repo_rate_limit_refresh_respects_global_max_rps: set CONTROL_PLANE_DATABASE_URL"
        );
        return;
    };

    let _env_guard = ENV_LOCK.lock().await;
    let old_batch = std::env::var("CONTROL_PLANE_RATE_LIMIT_REFRESH_BATCH_SIZE").ok();
    let old_concurrency = std::env::var("CONTROL_PLANE_RATE_LIMIT_REFRESH_CONCURRENCY").ok();
    let old_max_rps = std::env::var("CONTROL_PLANE_RATE_LIMIT_REFRESH_MAX_RPS").ok();
    std::env::set_var("CONTROL_PLANE_RATE_LIMIT_REFRESH_BATCH_SIZE", "2");
    std::env::set_var("CONTROL_PLANE_RATE_LIMIT_REFRESH_CONCURRENCY", "2");
    std::env::set_var("CONTROL_PLANE_RATE_LIMIT_REFRESH_MAX_RPS", "1");

    let cipher_key = base64::engine::general_purpose::STANDARD.encode([9_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let oauth_client = Arc::new(RecordingOAuthClient::default());
    let repo = PostgresStore::connect_with_oauth(&db_url, oauth_client.clone(), Some(cipher))
        .await
        .unwrap();

    let first_account = repo
        .import_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
            label: format!("oauth-rate-limit-throttle-a-{}", Uuid::new_v4().simple()),
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            refresh_token: format!("rt-a-{}", Uuid::new_v4().simple()),
            chatgpt_account_id: Some(format!("acct-a-{}", Uuid::new_v4().simple())),
            mode: Some(UpstreamMode::ChatGptSession),
            enabled: Some(true),
            priority: Some(100),
            chatgpt_plan_type: None,
            source_type: None,
        })
        .await
        .unwrap();
    let second_account = repo
        .import_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
            label: format!("oauth-rate-limit-throttle-b-{}", Uuid::new_v4().simple()),
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            refresh_token: format!("rt-b-{}", Uuid::new_v4().simple()),
            chatgpt_account_id: Some(format!("acct-b-{}", Uuid::new_v4().simple())),
            mode: Some(UpstreamMode::ChatGptSession),
            enabled: Some(true),
            priority: Some(100),
            chatgpt_plan_type: None,
            source_type: None,
        })
        .await
        .unwrap();

    expire_rate_limit_snapshot(&repo, first_account.id).await;
    expire_rate_limit_snapshot(&repo, second_account.id).await;
    oauth_client.enable_recording();

    let started_at = Instant::now();
    let refreshed = repo.refresh_due_oauth_rate_limit_caches().await.unwrap();
    let elapsed = started_at.elapsed();

    if let Some(value) = old_batch.as_deref() {
        std::env::set_var("CONTROL_PLANE_RATE_LIMIT_REFRESH_BATCH_SIZE", value);
    } else {
        std::env::remove_var("CONTROL_PLANE_RATE_LIMIT_REFRESH_BATCH_SIZE");
    }
    if let Some(value) = old_concurrency.as_deref() {
        std::env::set_var("CONTROL_PLANE_RATE_LIMIT_REFRESH_CONCURRENCY", value);
    } else {
        std::env::remove_var("CONTROL_PLANE_RATE_LIMIT_REFRESH_CONCURRENCY");
    }
    if let Some(value) = old_max_rps.as_deref() {
        std::env::set_var("CONTROL_PLANE_RATE_LIMIT_REFRESH_MAX_RPS", value);
    } else {
        std::env::remove_var("CONTROL_PLANE_RATE_LIMIT_REFRESH_MAX_RPS");
    }

    assert!(refreshed >= 2);

    let mut starts = oauth_client.fetch_starts();
    starts.sort();
    assert!(starts.len() >= 2, "expected at least two recorded fetches");
    assert!(
        starts[1].duration_since(starts[0]) >= std::time::Duration::from_millis(900),
        "expected fetch launches to be spaced by rate-limit max RPS, got {:?}",
        starts[1].duration_since(starts[0])
    );
    assert!(
        elapsed >= std::time::Duration::from_millis(900),
        "expected refresh batch to take at least one second with max_rps=1, got {:?}",
        elapsed
    );
}

#[tokio::test]
async fn postgres_repo_oauth_upsert_dedupes_by_chatgpt_account_user_id() {
    let Some(db_url) = test_db_url() else {
        eprintln!(
            "skip postgres_repo_oauth_upsert_dedupes_by_chatgpt_account_user_id: set CONTROL_PLANE_DATABASE_URL"
        );
        return;
    };

    let cipher_key = base64::engine::general_purpose::STANDARD.encode([13_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let oauth_client = Arc::new(SharedAccountIdRecordingOAuthClient);
    let repo = PostgresStore::connect_with_oauth(&db_url, oauth_client, Some(cipher))
        .await
        .unwrap();

    let first = repo
        .upsert_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
            label: format!("oauth-shared-a-{}", Uuid::new_v4().simple()),
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            refresh_token: format!("rt-shared-workspace-a-{}", Uuid::new_v4().simple()),
            chatgpt_account_id: None,
            mode: Some(UpstreamMode::CodexOauth),
            enabled: Some(true),
            priority: Some(100),
            chatgpt_plan_type: None,
            source_type: Some("codex".to_string()),
        })
        .await
        .unwrap();
    let second = repo
        .upsert_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
            label: format!("oauth-shared-b-{}", Uuid::new_v4().simple()),
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            refresh_token: format!("rt-shared-workspace-a-{}", Uuid::new_v4().simple()),
            chatgpt_account_id: None,
            mode: Some(UpstreamMode::CodexOauth),
            enabled: Some(true),
            priority: Some(100),
            chatgpt_plan_type: None,
            source_type: Some("codex".to_string()),
        })
        .await
        .unwrap();

    assert!(first.created);
    assert!(!second.created);
    assert_eq!(first.account.id, second.account.id);

    let snapshot = repo.snapshot().await.unwrap();
    let shared_accounts = snapshot
        .accounts
        .into_iter()
        .filter(|account| account.chatgpt_account_id.as_deref() == Some("acct-shared-workspace"))
        .collect::<Vec<_>>();

    assert_eq!(shared_accounts.len(), 1);
}

#[tokio::test]
async fn postgres_repo_oauth_upsert_keeps_distinct_accounts_with_shared_chatgpt_account_id_but_different_account_user_id(
) {
    let Some(db_url) = test_db_url() else {
        eprintln!(
            "skip postgres_repo_oauth_upsert_keeps_distinct_accounts_with_shared_chatgpt_account_id_but_different_account_user_id: set CONTROL_PLANE_DATABASE_URL"
        );
        return;
    };

    let cipher_key = base64::engine::general_purpose::STANDARD.encode([14_u8; 32]);
    let cipher = CredentialCipher::from_base64_key(&cipher_key).unwrap();
    let oauth_client = Arc::new(SharedAccountIdRecordingOAuthClient);
    let repo = PostgresStore::connect_with_oauth(&db_url, oauth_client, Some(cipher))
        .await
        .unwrap();

    let first = repo
        .upsert_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
            label: format!("oauth-dup-a-{}", Uuid::new_v4().simple()),
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            refresh_token: format!("rt-shared-workspace-a-{}", Uuid::new_v4().simple()),
            chatgpt_account_id: None,
            mode: Some(UpstreamMode::CodexOauth),
            enabled: Some(true),
            priority: Some(100),
            chatgpt_plan_type: None,
            source_type: Some("codex".to_string()),
        })
        .await
        .unwrap();
    let second = repo
        .upsert_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
            label: format!("oauth-dup-b-{}", Uuid::new_v4().simple()),
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            refresh_token: format!("rt-shared-workspace-b-{}", Uuid::new_v4().simple()),
            chatgpt_account_id: None,
            mode: Some(UpstreamMode::CodexOauth),
            enabled: Some(true),
            priority: Some(100),
            chatgpt_plan_type: None,
            source_type: Some("codex".to_string()),
        })
        .await
        .unwrap();

    assert!(first.created);
    assert!(second.created);
    assert_ne!(first.account.id, second.account.id);

    let snapshot = repo.snapshot().await.unwrap();
    let shared_accounts = snapshot
        .accounts
        .into_iter()
        .filter(|account| account.chatgpt_account_id.as_deref() == Some("acct-shared-workspace"))
        .collect::<Vec<_>>();

    assert_eq!(shared_accounts.len(), 2);
}

#[tokio::test]
async fn postgres_repo_inserts_and_lists_tenants() {
    let Some(db_url) = test_db_url() else {
        eprintln!("skip postgres_repo_inserts_and_lists_tenants: set CONTROL_PLANE_DATABASE_URL");
        return;
    };

    let repo = PostgresStore::connect(&db_url).await.unwrap();
    let tenant_name = format!("team-a-{}", Uuid::new_v4().simple());
    repo.create_tenant(CreateTenantRequest {
        name: tenant_name.clone(),
    })
    .await
    .unwrap();

    let tenants = repo.list_tenants().await.unwrap();
    assert!(tenants.iter().any(|tenant| tenant.name == tenant_name));
}

#[tokio::test]
async fn postgres_repo_supports_snapshot_and_policy_paths() {
    let Some(db_url) = test_db_url() else {
        eprintln!(
            "skip postgres_repo_supports_snapshot_and_policy_paths: set CONTROL_PLANE_DATABASE_URL"
        );
        return;
    };

    let repo = PostgresStore::connect(&db_url).await.unwrap();
    let tenant_name = format!("team-policy-{}", Uuid::new_v4().simple());
    let tenant = repo
        .create_tenant(CreateTenantRequest { name: tenant_name })
        .await
        .unwrap();

    let created_key = repo
        .create_api_key(CreateApiKeyRequest {
            tenant_id: tenant.id,
            name: "primary".to_string(),
        })
        .await
        .unwrap();
    let api_keys = repo.list_api_keys().await.unwrap();
    assert!(api_keys.iter().any(|key| key.id == created_key.record.id));

    let account_label = format!("acct-{}", Uuid::new_v4().simple());
    let created_account = repo
        .create_upstream_account(CreateUpstreamAccountRequest {
            label: account_label.clone(),
            mode: UpstreamMode::OpenAiApiKey,
            base_url: "https://api.openai.com/v1".to_string(),
            bearer_token: "tok".to_string(),
            chatgpt_account_id: None,
            auth_provider: None,
            enabled: Some(true),
            priority: Some(100),
        })
        .await
        .unwrap();
    assert_eq!(created_account.label, account_label);

    let snapshot_before_policy = repo.snapshot().await.unwrap();
    assert!(snapshot_before_policy.revision >= 2);
    assert!(snapshot_before_policy
        .accounts
        .iter()
        .any(|account| account.id == created_account.id));

    let routing_policy = repo
        .upsert_routing_policy(UpsertRoutingPolicyRequest {
            tenant_id: tenant.id,
            strategy: RoutingStrategy::FillFirst,
            max_retries: 2,
            stream_max_retries: 4,
        })
        .await
        .unwrap();
    assert_eq!(routing_policy.strategy, RoutingStrategy::FillFirst);

    let retry_policy = repo
        .upsert_retry_policy(UpsertRetryPolicyRequest {
            tenant_id: tenant.id,
            max_retries: 7,
        })
        .await
        .unwrap();
    assert_eq!(retry_policy.max_retries, 7);
    assert_eq!(retry_policy.strategy, RoutingStrategy::FillFirst);

    let stream_retry_policy = repo
        .upsert_stream_retry_policy(UpsertStreamRetryPolicyRequest {
            tenant_id: tenant.id,
            stream_max_retries: 9,
        })
        .await
        .unwrap();
    assert_eq!(stream_retry_policy.stream_max_retries, 9);
    assert_eq!(stream_retry_policy.max_retries, 7);

    let snapshot_after_policy = repo.snapshot().await.unwrap();
    assert!(snapshot_after_policy.revision > snapshot_before_policy.revision);
}

#[tokio::test]
async fn postgres_repo_ai_error_learning_changes_are_emitted_to_snapshot_events() {
    let Some(db_url) = test_db_url() else {
        eprintln!(
            "skip postgres_repo_ai_error_learning_changes_are_emitted_to_snapshot_events: set CONTROL_PLANE_DATABASE_URL"
        );
        return;
    };

    let repo = PostgresStore::connect(&db_url).await.unwrap();
    let cursor_before = repo.snapshot().await.unwrap().cursor;

    repo.update_upstream_error_learning_settings(UpdateAiErrorLearningSettingsRequest {
        enabled: true,
        first_seen_timeout_ms: 2_000,
        review_hit_threshold: 10,
    })
    .await
    .unwrap();

    let settings_events = repo
        .data_plane_snapshot_events(cursor_before, 50)
        .await
        .unwrap();
    let settings_refresh = settings_events
        .events
        .iter()
        .find(|event| {
            matches!(
                event.event_type,
                DataPlaneSnapshotEventType::RoutingPlanRefresh
            )
        })
        .expect("expected routing refresh event after ai settings update");
    let settings = settings_refresh
        .ai_error_learning_settings
        .as_ref()
        .expect("routing refresh event should include ai error learning settings");
    assert!(settings.enabled);
    assert_eq!(settings.first_seen_timeout_ms, 2_000);

    let approved_template = UpstreamErrorTemplateRecord {
        id: Uuid::new_v4(),
        fingerprint: format!(
            "openai_compatible:400:unsupported-model-{}",
            Uuid::new_v4().simple()
        ),
        provider: "openai_compatible".to_string(),
        normalized_status_code: 400,
        semantic_error_code: "unsupported_model".to_string(),
        action: UpstreamErrorAction::ReturnFailure,
        retry_scope: UpstreamErrorRetryScope::None,
        status: UpstreamErrorTemplateStatus::Approved,
        templates: LocalizedErrorTemplates {
            en: Some("The requested model is not available.".to_string()),
            ..LocalizedErrorTemplates::default()
        },
        representative_samples: vec!["The requested model is not available.".to_string()],
        hit_count: 11,
        first_seen_at: Utc::now(),
        last_seen_at: Utc::now(),
        updated_at: Utc::now(),
    };

    repo.save_upstream_error_template(approved_template.clone())
        .await
        .unwrap();

    let template_events = repo
        .data_plane_snapshot_events(settings_events.cursor, 50)
        .await
        .unwrap();
    let template_refresh = template_events
        .events
        .iter()
        .find(|event| {
            matches!(
                event.event_type,
                DataPlaneSnapshotEventType::RoutingPlanRefresh
            )
        })
        .expect("expected routing refresh event after approved template save");
    let templates = template_refresh
        .approved_upstream_error_templates
        .as_ref()
        .expect("routing refresh event should include approved templates");
    assert!(templates
        .iter()
        .any(|template| template.fingerprint == approved_template.fingerprint));
}

#[tokio::test]
async fn postgres_repo_validates_api_key() {
    let Some(db_url) = test_db_url() else {
        eprintln!("skip postgres_repo_validates_api_key: set CONTROL_PLANE_DATABASE_URL");
        return;
    };

    let repo = PostgresStore::connect(&db_url).await.unwrap();
    let tenant = repo
        .create_tenant(CreateTenantRequest {
            name: format!("team-auth-{}", Uuid::new_v4().simple()),
        })
        .await
        .unwrap();
    let created = repo
        .create_api_key(CreateApiKeyRequest {
            tenant_id: tenant.id,
            name: "auth".to_string(),
        })
        .await
        .unwrap();

    let principal = repo
        .validate_api_key(&created.plaintext_key)
        .await
        .unwrap()
        .expect("api key principal should exist");
    assert_eq!(principal.tenant_id, tenant.id);
    assert_eq!(principal.api_key_id, created.record.id);
    assert!(principal.enabled);
}

#[tokio::test]
async fn postgres_repo_stores_hashed_api_key_token() {
    let Some(db_url) = test_db_url() else {
        eprintln!("skip postgres_repo_stores_hashed_api_key_token: set CONTROL_PLANE_DATABASE_URL");
        return;
    };

    let repo = PostgresStore::connect(&db_url).await.unwrap();
    let tenant = repo
        .create_tenant(CreateTenantRequest {
            name: format!("team-auth-hash-{}", Uuid::new_v4().simple()),
        })
        .await
        .unwrap();
    let created = repo
        .create_api_key(CreateApiKeyRequest {
            tenant_id: tenant.id,
            name: "auth-hash".to_string(),
        })
        .await
        .unwrap();
    let stored_token: String =
        query_scalar("SELECT token FROM api_key_tokens WHERE api_key_id = $1")
            .bind(created.record.id)
            .fetch_one(&repo.clone_pool())
            .await
            .unwrap();

    assert_ne!(stored_token, created.plaintext_key);
    assert!(stored_token.starts_with("hmac-sha256:"));
    assert!(!created.record.key_hash.starts_with("plaintext:"));
}

#[tokio::test]
async fn postgres_repo_upgrades_legacy_plaintext_api_key_token_hash_on_validate() {
    let Some(db_url) = test_db_url() else {
        eprintln!(
            "skip postgres_repo_upgrades_legacy_plaintext_api_key_token_hash_on_validate: set CONTROL_PLANE_DATABASE_URL"
        );
        return;
    };

    let repo = PostgresStore::connect(&db_url).await.unwrap();
    let tenant = repo
        .create_tenant(CreateTenantRequest {
            name: format!("team-auth-upgrade-{}", Uuid::new_v4().simple()),
        })
        .await
        .unwrap();
    let created = repo
        .create_api_key(CreateApiKeyRequest {
            tenant_id: tenant.id,
            name: "auth-upgrade".to_string(),
        })
        .await
        .unwrap();

    query("UPDATE api_key_tokens SET token = $1 WHERE api_key_id = $2")
        .bind(&created.plaintext_key)
        .bind(created.record.id)
        .execute(&repo.clone_pool())
        .await
        .unwrap();
    query("UPDATE api_keys SET key_hash = $1 WHERE id = $2")
        .bind(format!("plaintext:{}", created.plaintext_key))
        .bind(created.record.id)
        .execute(&repo.clone_pool())
        .await
        .unwrap();

    let principal = repo
        .validate_api_key(&created.plaintext_key)
        .await
        .unwrap()
        .expect("api key principal should exist");
    assert_eq!(principal.api_key_id, created.record.id);

    let migrated_token: String =
        query_scalar("SELECT token FROM api_key_tokens WHERE api_key_id = $1")
            .bind(created.record.id)
            .fetch_one(&repo.clone_pool())
            .await
            .unwrap();
    let migrated_key_hash: String = query_scalar("SELECT key_hash FROM api_keys WHERE id = $1")
        .bind(created.record.id)
        .fetch_one(&repo.clone_pool())
        .await
        .unwrap();

    assert!(migrated_token.starts_with("hmac-sha256:"));
    assert!(migrated_key_hash.starts_with("hmac-sha256:"));
}

#[tokio::test]
async fn postgres_import_store_recovers_running_processing_items() {
    let Some(db_url) = test_db_url() else {
        eprintln!(
            "skip postgres_import_store_recovers_running_processing_items: set CONTROL_PLANE_DATABASE_URL"
        );
        return;
    };

    let repo = PostgresStore::connect(&db_url).await.unwrap();
    let pool = repo.clone_pool();
    let import_store = PostgresOAuthImportJobStore::new(pool.clone())
        .await
        .unwrap();

    let job_id = Uuid::new_v4();
    let summary = OAuthImportJobSummary {
        job_id,
        status: OAuthImportJobStatus::Queued,
        total: 1,
        processed: 0,
        created_count: 0,
        updated_count: 0,
        failed_count: 0,
        skipped_count: 0,
        started_at: None,
        finished_at: None,
        created_at: chrono::Utc::now(),
        throughput_per_min: None,
        error_summary: Vec::new(),
    };

    let request = ImportOAuthRefreshTokenRequest {
        label: "recover-test".to_string(),
        base_url: "https://chatgpt.com/backend-api/codex".to_string(),
        refresh_token: "rt_recover_test".to_string(),
        chatgpt_account_id: Some("acct-recover".to_string()),
        mode: Some(UpstreamMode::ChatGptSession),
        enabled: Some(true),
        priority: Some(100),
        chatgpt_plan_type: None,
        source_type: None,
    };
    let item = PersistedImportItem {
        item: OAuthImportJobItem {
            item_id: 1,
            source_file: "recover.jsonl".to_string(),
            line_no: 1,
            status: OAuthImportItemStatus::Pending,
            label: "recover-test".to_string(),
            email: None,
            chatgpt_account_id: Some("acct-recover".to_string()),
            account_id: None,
            error_code: None,
            error_message: None,
        },
        request: Some(ImportTaskRequest::OAuthRefresh(request)),
        raw_record: None,
        normalized_record: None,
        retry_count: 0,
    };
    import_store.create_job(summary, vec![item]).await.unwrap();

    pool.execute(
        query("UPDATE oauth_import_jobs SET status = 'running' WHERE id = $1").bind(job_id),
    )
    .await
    .unwrap();
    pool.execute(
        query(
            "UPDATE oauth_import_job_items SET status = 'processing' WHERE job_id = $1 AND item_id = 1",
        )
        .bind(job_id),
    )
    .await
    .unwrap();

    let tasks = import_store.start_job(job_id, 50).await.unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].item_id, 1);
}

#[tokio::test]
async fn postgres_impersonation_revoke_invalidates_principal_immediately() {
    let Some(db_url) = test_db_url() else {
        eprintln!(
            "skip postgres_impersonation_revoke_invalidates_principal_immediately: set CONTROL_PLANE_DATABASE_URL"
        );
        return;
    };

    std::env::set_var("TENANT_JWT_SECRET", "tenant-test-jwt-secret");

    let repo = PostgresStore::connect(&db_url).await.unwrap();
    let pool = repo.clone_pool();
    let tenant = repo
        .create_tenant(CreateTenantRequest {
            name: format!("team-impersonation-{}", Uuid::new_v4().simple()),
        })
        .await
        .unwrap();
    let admin_user_id = Uuid::new_v4();
    let now = chrono::Utc::now();
    query(
        r#"
        INSERT INTO admin_users (id, username, password_hash, enabled, created_at, updated_at)
        VALUES ($1, $2, $3, true, $4, $4)
        "#,
    )
    .bind(admin_user_id)
    .bind(format!("impersonation-admin-{}", Uuid::new_v4().simple()))
    .bind("not-used")
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    let tenant_auth = TenantAuthService::from_pool(pool).expect("tenant auth service");
    let response = tenant_auth
        .admin_impersonate(
            admin_user_id,
            AdminImpersonateRequest {
                tenant_id: tenant.id,
                reason: "test revoke".to_string(),
            },
        )
        .await
        .unwrap();
    let principal = tenant_auth.verify_token(&response.access_token).unwrap();
    tenant_auth
        .ensure_principal_active(&principal)
        .await
        .unwrap();
    tenant_auth
        .admin_revoke_impersonation(response.session_id)
        .await
        .unwrap();
    assert!(tenant_auth
        .ensure_principal_active(&principal)
        .await
        .is_err());
}
