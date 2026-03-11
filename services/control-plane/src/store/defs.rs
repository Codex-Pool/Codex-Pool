use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use codex_pool_core::api::{
    CreateApiKeyRequest, CreateApiKeyResponse, CreateTenantRequest, CreateUpstreamAccountRequest,
    DataPlaneSnapshot, DataPlaneSnapshotEventsResponse, ImportOAuthRefreshTokenRequest,
    OAuthAccountStatusResponse, OAuthFamilyActionResponse, OAuthRateLimitRefreshJobSummary,
    OAuthRefreshStatus, SessionCredentialKind, UpsertModelRoutingPolicyRequest,
    UpsertRetryPolicyRequest, UpsertRoutingPolicyRequest, UpsertRoutingProfileRequest,
    UpsertStreamRetryPolicyRequest, UpdateModelRoutingSettingsRequest,
    ValidateOAuthRefreshTokenRequest,
    ValidateOAuthRefreshTokenResponse,
};
use codex_pool_core::model::{
    AccountRoutingTraits, ApiKey, ModelRoutingSettings, ModelRoutingTriggerMode,
    CompiledModelRoutingPolicy, CompiledRoutingPlan, CompiledRoutingProfile, ModelRoutingPolicy,
    RoutingPlanVersion, RoutingPolicy, RoutingProfile, RoutingStrategy, Tenant, UpstreamAccount,
    UpstreamAuthProvider, UpstreamMode,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx_postgres::PgPool;
use uuid::Uuid;

use crate::crypto::CredentialCipher;
use crate::oauth::{OAuthTokenClient, OAuthTokenInfo, OpenAiOAuthClient};

pub mod postgres;

const OAUTH_MANAGED_BEARER_SENTINEL: &str = "__managed_oauth__";
const OAUTH_REFRESH_WINDOW_SEC: i64 = 300;
const OAUTH_MIN_VALID_SEC: i64 = 60;
const OAUTH_REFRESH_BATCH_SIZE_ENV: &str = "CONTROL_PLANE_OAUTH_REFRESH_BATCH_SIZE";
const DEFAULT_OAUTH_REFRESH_BATCH_SIZE: usize = 200;
const MIN_OAUTH_REFRESH_BATCH_SIZE: usize = 1;
const MAX_OAUTH_REFRESH_BATCH_SIZE: usize = 2000;
const OAUTH_REFRESH_CONCURRENCY_ENV: &str = "CONTROL_PLANE_OAUTH_REFRESH_CONCURRENCY";
const DEFAULT_OAUTH_REFRESH_CONCURRENCY: usize = 8;
const MIN_OAUTH_REFRESH_CONCURRENCY: usize = 1;
const MAX_OAUTH_REFRESH_CONCURRENCY: usize = 64;

fn oauth_refresh_batch_size_from_env() -> usize {
    std::env::var(OAUTH_REFRESH_BATCH_SIZE_ENV)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(DEFAULT_OAUTH_REFRESH_BATCH_SIZE)
        .clamp(MIN_OAUTH_REFRESH_BATCH_SIZE, MAX_OAUTH_REFRESH_BATCH_SIZE)
}

fn oauth_refresh_concurrency_from_env() -> usize {
    std::env::var(OAUTH_REFRESH_CONCURRENCY_ENV)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(DEFAULT_OAUTH_REFRESH_CONCURRENCY)
        .clamp(MIN_OAUTH_REFRESH_CONCURRENCY, MAX_OAUTH_REFRESH_CONCURRENCY)
}

fn resolve_oauth_import_mode(
    mode: Option<UpstreamMode>,
    source_type: Option<&str>,
) -> UpstreamMode {
    if let Some(mode) = mode {
        return mode;
    }

    if source_type.is_some_and(|raw| raw.trim().eq_ignore_ascii_case("codex")) {
        return UpstreamMode::CodexOauth;
    }

    UpstreamMode::ChatGptSession
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedPrincipal {
    pub tenant_id: Uuid,
    pub api_key_id: Uuid,
    pub api_key_group_id: Uuid,
    pub api_key_group_name: String,
    pub api_key_group_invalid: bool,
    pub enabled: bool,
    pub key_ip_allowlist: Vec<String>,
    pub key_model_allowlist: Vec<String>,
    pub tenant_status: Option<String>,
    pub tenant_expires_at: Option<DateTime<Utc>>,
    pub balance_microcredits: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct OAuthUpsertResult {
    pub account: UpstreamAccount,
    pub created: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertOneTimeSessionAccountRequest {
    pub label: String,
    pub mode: UpstreamMode,
    pub base_url: String,
    pub access_token: String,
    pub chatgpt_account_id: Option<String>,
    pub enabled: Option<bool>,
    pub priority: Option<i32>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub chatgpt_plan_type: Option<String>,
    pub source_type: Option<String>,
}

#[async_trait]
pub trait ControlPlaneStore: Send + Sync {
    fn postgres_pool(&self) -> Option<PgPool> {
        None
    }
    async fn create_tenant(&self, req: CreateTenantRequest) -> Result<Tenant>;
    async fn list_tenants(&self) -> Result<Vec<Tenant>>;
    async fn create_api_key(&self, req: CreateApiKeyRequest) -> Result<CreateApiKeyResponse>;
    async fn list_api_keys(&self) -> Result<Vec<ApiKey>>;
    async fn set_api_key_enabled(&self, _api_key_id: Uuid, _enabled: bool) -> Result<ApiKey> {
        Err(anyhow!("api key update is not implemented"))
    }
    async fn validate_api_key(&self, token: &str) -> Result<Option<ValidatedPrincipal>>;
    async fn create_upstream_account(
        &self,
        req: CreateUpstreamAccountRequest,
    ) -> Result<UpstreamAccount>;
    async fn list_upstream_accounts(&self) -> Result<Vec<UpstreamAccount>>;
    async fn set_upstream_account_enabled(
        &self,
        _account_id: Uuid,
        _enabled: bool,
    ) -> Result<UpstreamAccount> {
        Err(anyhow!("upstream account update is not implemented"))
    }
    async fn delete_upstream_account(&self, _account_id: Uuid) -> Result<()> {
        Err(anyhow!("upstream account delete is not implemented"))
    }
    async fn validate_oauth_refresh_token(
        &self,
        _req: ValidateOAuthRefreshTokenRequest,
    ) -> Result<ValidateOAuthRefreshTokenResponse> {
        Err(anyhow!("oauth refresh-token validation is not implemented"))
    }
    async fn import_oauth_refresh_token(
        &self,
        _req: ImportOAuthRefreshTokenRequest,
    ) -> Result<UpstreamAccount> {
        Err(anyhow!("oauth account import is not implemented"))
    }
    async fn upsert_oauth_refresh_token(
        &self,
        _req: ImportOAuthRefreshTokenRequest,
    ) -> Result<OAuthUpsertResult> {
        Err(anyhow!("oauth account upsert is not implemented"))
    }
    async fn queue_oauth_refresh_token(
        &self,
        req: ImportOAuthRefreshTokenRequest,
    ) -> Result<bool> {
        let upserted = self.upsert_oauth_refresh_token(req).await?;
        Ok(upserted.created)
    }
    async fn dedupe_oauth_accounts_by_identity(&self) -> Result<u64> {
        Ok(0)
    }
    async fn upsert_one_time_session_account(
        &self,
        _req: UpsertOneTimeSessionAccountRequest,
    ) -> Result<OAuthUpsertResult> {
        Err(anyhow!(
            "one-time session account upsert is not implemented"
        ))
    }
    async fn refresh_oauth_account(&self, _account_id: Uuid) -> Result<OAuthAccountStatusResponse> {
        Err(anyhow!("oauth account refresh is not implemented"))
    }
    async fn oauth_account_status(&self, _account_id: Uuid) -> Result<OAuthAccountStatusResponse> {
        Err(anyhow!("oauth account status query is not implemented"))
    }
    async fn oauth_account_statuses(
        &self,
        account_ids: Vec<Uuid>,
    ) -> Result<Vec<OAuthAccountStatusResponse>> {
        let mut items = Vec::with_capacity(account_ids.len());
        for account_id in account_ids {
            items.push(self.oauth_account_status(account_id).await?);
        }
        Ok(items)
    }
    async fn upsert_routing_policy(
        &self,
        _req: UpsertRoutingPolicyRequest,
    ) -> Result<RoutingPolicy> {
        Err(anyhow!("routing policy repository is not implemented"))
    }
    async fn upsert_retry_policy(&self, _req: UpsertRetryPolicyRequest) -> Result<RoutingPolicy> {
        Err(anyhow!("retry policy repository is not implemented"))
    }
    async fn upsert_stream_retry_policy(
        &self,
        _req: UpsertStreamRetryPolicyRequest,
    ) -> Result<RoutingPolicy> {
        Err(anyhow!("stream retry policy repository is not implemented"))
    }
    async fn list_routing_profiles(&self) -> Result<Vec<RoutingProfile>> {
        Err(anyhow!("routing profile repository is not implemented"))
    }
    async fn upsert_routing_profile(
        &self,
        _req: UpsertRoutingProfileRequest,
    ) -> Result<RoutingProfile> {
        Err(anyhow!("routing profile repository is not implemented"))
    }
    async fn delete_routing_profile(&self, _profile_id: Uuid) -> Result<()> {
        Err(anyhow!("routing profile repository is not implemented"))
    }
    async fn list_model_routing_policies(&self) -> Result<Vec<ModelRoutingPolicy>> {
        Err(anyhow!("model routing policy repository is not implemented"))
    }
    async fn upsert_model_routing_policy(
        &self,
        _req: UpsertModelRoutingPolicyRequest,
    ) -> Result<ModelRoutingPolicy> {
        Err(anyhow!("model routing policy repository is not implemented"))
    }
    async fn delete_model_routing_policy(&self, _policy_id: Uuid) -> Result<()> {
        Err(anyhow!("model routing policy repository is not implemented"))
    }
    async fn model_routing_settings(&self) -> Result<ModelRoutingSettings> {
        Err(anyhow!("model routing settings repository is not implemented"))
    }
    async fn update_model_routing_settings(
        &self,
        _req: UpdateModelRoutingSettingsRequest,
    ) -> Result<ModelRoutingSettings> {
        Err(anyhow!("model routing settings repository is not implemented"))
    }
    async fn list_routing_plan_versions(&self) -> Result<Vec<RoutingPlanVersion>> {
        Err(anyhow!("routing plan version repository is not implemented"))
    }
    async fn record_account_model_support(
        &self,
        _account_id: Uuid,
        _supported_models: Vec<String>,
        _checked_at: DateTime<Utc>,
    ) -> Result<()> {
        Ok(())
    }
    async fn refresh_expiring_oauth_accounts(&self) -> Result<()> {
        Ok(())
    }
    async fn activate_oauth_refresh_token_vault(&self) -> Result<u64> {
        Ok(0)
    }
    async fn refresh_due_oauth_rate_limit_caches(&self) -> Result<u64> {
        Ok(0)
    }
    async fn recover_oauth_rate_limit_refresh_jobs(&self) -> Result<u64> {
        Ok(0)
    }
    async fn create_oauth_rate_limit_refresh_job(&self) -> Result<OAuthRateLimitRefreshJobSummary> {
        Err(anyhow!(
            "oauth rate-limit refresh job creation is not implemented"
        ))
    }
    async fn oauth_rate_limit_refresh_job(
        &self,
        _job_id: Uuid,
    ) -> Result<OAuthRateLimitRefreshJobSummary> {
        Err(anyhow!(
            "oauth rate-limit refresh job query is not implemented"
        ))
    }
    async fn run_oauth_rate_limit_refresh_job(&self, _job_id: Uuid) -> Result<()> {
        Err(anyhow!(
            "oauth rate-limit refresh job execution is not implemented"
        ))
    }
    async fn flush_snapshot_revision(&self, _max_batch: usize) -> Result<u32> {
        Ok(0)
    }
    async fn set_oauth_family_enabled(
        &self,
        _account_id: Uuid,
        _enabled: bool,
    ) -> Result<OAuthFamilyActionResponse> {
        Err(anyhow!("oauth family action is not implemented"))
    }
    async fn snapshot(&self) -> Result<DataPlaneSnapshot>;
    async fn cleanup_data_plane_outbox(&self, _retention: Duration) -> Result<u64> {
        Ok(0)
    }
    async fn data_plane_snapshot_events(
        &self,
        _after: u64,
        _limit: u32,
    ) -> Result<DataPlaneSnapshotEventsResponse> {
        Ok(DataPlaneSnapshotEventsResponse {
            cursor: 0,
            high_watermark: 0,
            events: Vec::new(),
        })
    }
    async fn mark_account_seen_ok(
        &self,
        _account_id: Uuid,
        _seen_ok_at: DateTime<Utc>,
        _min_write_interval_sec: i64,
    ) -> Result<bool> {
        Ok(false)
    }
    async fn maybe_refresh_oauth_rate_limit_cache_on_seen_ok(
        &self,
        _account_id: Uuid,
    ) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct OAuthCredentialRecord {
    access_token_enc: String,
    refresh_token_enc: String,
    refresh_token_sha256: String,
    token_family_id: String,
    token_version: u64,
    token_expires_at: DateTime<Utc>,
    last_refresh_at: Option<DateTime<Utc>>,
    last_refresh_status: OAuthRefreshStatus,
    refresh_reused_detected: bool,
    last_refresh_error_code: Option<String>,
    last_refresh_error: Option<String>,
    refresh_failure_count: u32,
    refresh_backoff_until: Option<DateTime<Utc>>,
}

impl OAuthCredentialRecord {
    fn from_token_info(cipher: &CredentialCipher, token_info: &OAuthTokenInfo) -> Result<Self> {
        Ok(Self {
            access_token_enc: cipher.encrypt(&token_info.access_token)?,
            refresh_token_enc: cipher.encrypt(&token_info.refresh_token)?,
            refresh_token_sha256: refresh_token_sha256(&token_info.refresh_token),
            token_family_id: Uuid::new_v4().to_string(),
            token_version: 1,
            token_expires_at: token_info.expires_at,
            last_refresh_at: Some(Utc::now()),
            last_refresh_status: OAuthRefreshStatus::Ok,
            refresh_reused_detected: false,
            last_refresh_error_code: None,
            last_refresh_error: None,
            refresh_failure_count: 0,
            refresh_backoff_until: None,
        })
    }

    fn backoff_duration(&self) -> Duration {
        match self.refresh_failure_count {
            0 => Duration::seconds(0),
            1 => Duration::seconds(30),
            2 => Duration::seconds(60),
            3 => Duration::seconds(120),
            _ => Duration::seconds(300),
        }
    }
}

#[derive(Debug, Clone)]
struct SessionProfileRecord {
    credential_kind: SessionCredentialKind,
    token_expires_at: Option<DateTime<Utc>>,
    email: Option<String>,
    oauth_subject: Option<String>,
    oauth_identity_provider: Option<String>,
    email_verified: Option<bool>,
    chatgpt_plan_type: Option<String>,
    chatgpt_user_id: Option<String>,
    chatgpt_subscription_active_start: Option<DateTime<Utc>>,
    chatgpt_subscription_active_until: Option<DateTime<Utc>>,
    chatgpt_subscription_last_checked: Option<DateTime<Utc>>,
    chatgpt_account_user_id: Option<String>,
    chatgpt_compute_residency: Option<String>,
    workspace_name: Option<String>,
    organizations: Option<Vec<Value>>,
    groups: Option<Vec<Value>>,
    source_type: Option<String>,
}

impl SessionProfileRecord {
    fn from_oauth_token_info(
        token_info: &OAuthTokenInfo,
        credential_kind: SessionCredentialKind,
        chatgpt_plan_type: Option<String>,
        source_type: Option<String>,
    ) -> Self {
        Self {
            credential_kind,
            token_expires_at: Some(token_info.expires_at),
            email: token_info.email.clone(),
            oauth_subject: token_info.oauth_subject.clone(),
            oauth_identity_provider: token_info.oauth_identity_provider.clone(),
            email_verified: token_info.email_verified,
            chatgpt_plan_type: chatgpt_plan_type.or(token_info.chatgpt_plan_type.clone()),
            chatgpt_user_id: token_info.chatgpt_user_id.clone(),
            chatgpt_subscription_active_start: token_info
                .chatgpt_subscription_active_start
                .as_ref()
                .cloned(),
            chatgpt_subscription_active_until: token_info
                .chatgpt_subscription_active_until
                .as_ref()
                .cloned(),
            chatgpt_subscription_last_checked: token_info
                .chatgpt_subscription_last_checked
                .as_ref()
                .cloned(),
            chatgpt_account_user_id: token_info.chatgpt_account_user_id.clone(),
            chatgpt_compute_residency: token_info.chatgpt_compute_residency.clone(),
            workspace_name: token_info.workspace_name.clone(),
            organizations: token_info.organizations.clone(),
            groups: token_info.groups.clone(),
            source_type,
        }
    }

    fn one_time_access_token(
        token_expires_at: Option<DateTime<Utc>>,
        chatgpt_plan_type: Option<String>,
        source_type: Option<String>,
    ) -> Self {
        Self {
            credential_kind: SessionCredentialKind::OneTimeAccessToken,
            token_expires_at,
            email: None,
            oauth_subject: None,
            oauth_identity_provider: None,
            email_verified: None,
            chatgpt_plan_type,
            chatgpt_user_id: None,
            chatgpt_subscription_active_start: None,
            chatgpt_subscription_active_until: None,
            chatgpt_subscription_last_checked: None,
            chatgpt_account_user_id: None,
            chatgpt_compute_residency: None,
            workspace_name: None,
            organizations: None,
            groups: None,
            source_type,
        }
    }

    fn merge_oauth_token_info(
        mut self,
        token_info: &OAuthTokenInfo,
        credential_kind: SessionCredentialKind,
        chatgpt_plan_type: Option<String>,
        source_type: Option<String>,
    ) -> Self {
        self.credential_kind = credential_kind;
        self.token_expires_at = Some(token_info.expires_at);
        self.email = token_info.email.clone().or(self.email);
        self.oauth_subject = token_info.oauth_subject.clone().or(self.oauth_subject);
        self.oauth_identity_provider = token_info
            .oauth_identity_provider
            .clone()
            .or(self.oauth_identity_provider);
        self.email_verified = token_info.email_verified.or(self.email_verified);
        self.chatgpt_plan_type = chatgpt_plan_type
            .or(token_info.chatgpt_plan_type.clone())
            .or(self.chatgpt_plan_type);
        self.chatgpt_user_id = token_info.chatgpt_user_id.clone().or(self.chatgpt_user_id);
        self.chatgpt_subscription_active_start = token_info
            .chatgpt_subscription_active_start
            .as_ref()
            .cloned()
            .or(self.chatgpt_subscription_active_start);
        self.chatgpt_subscription_active_until = token_info
            .chatgpt_subscription_active_until
            .as_ref()
            .cloned()
            .or(self.chatgpt_subscription_active_until);
        self.chatgpt_subscription_last_checked = token_info
            .chatgpt_subscription_last_checked
            .as_ref()
            .cloned()
            .or(self.chatgpt_subscription_last_checked);
        self.chatgpt_account_user_id = token_info
            .chatgpt_account_user_id
            .clone()
            .or(self.chatgpt_account_user_id);
        self.chatgpt_compute_residency = token_info
            .chatgpt_compute_residency
            .clone()
            .or(self.chatgpt_compute_residency);
        self.workspace_name = token_info.workspace_name.clone().or(self.workspace_name);
        self.organizations = token_info.organizations.clone().or(self.organizations);
        self.groups = token_info.groups.clone().or(self.groups);
        self.source_type = source_type.or(self.source_type);
        self
    }
}

#[derive(Debug, Clone, Default)]
struct UpstreamAccountHealthStateRecord {
    seen_ok_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default)]
struct AccountModelSupportRecord {
    supported_models: Vec<String>,
}

pub struct InMemoryStore {
    tenants: Arc<RwLock<HashMap<Uuid, Tenant>>>,
    api_keys: Arc<RwLock<HashMap<Uuid, ApiKey>>>,
    api_key_tokens: Arc<RwLock<HashMap<String, Uuid>>>,
    accounts: Arc<RwLock<HashMap<Uuid, UpstreamAccount>>>,
    account_auth_providers: Arc<RwLock<HashMap<Uuid, UpstreamAuthProvider>>>,
    oauth_credentials: Arc<RwLock<HashMap<Uuid, OAuthCredentialRecord>>>,
    session_profiles: Arc<RwLock<HashMap<Uuid, SessionProfileRecord>>>,
    account_health_states: Arc<RwLock<HashMap<Uuid, UpstreamAccountHealthStateRecord>>>,
    account_model_support: Arc<RwLock<HashMap<Uuid, AccountModelSupportRecord>>>,
    policies: Arc<RwLock<HashMap<Uuid, RoutingPolicy>>>,
    routing_profiles: Arc<RwLock<HashMap<Uuid, RoutingProfile>>>,
    model_routing_policies: Arc<RwLock<HashMap<Uuid, ModelRoutingPolicy>>>,
    model_routing_settings: Arc<RwLock<ModelRoutingSettings>>,
    routing_plan_versions: Arc<RwLock<Vec<RoutingPlanVersion>>>,
    revision: Arc<AtomicU64>,
    oauth_client: Arc<dyn OAuthTokenClient>,
    credential_cipher: Option<CredentialCipher>,
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new_with_oauth(
            Arc::new(OpenAiOAuthClient::from_env()),
            CredentialCipher::from_env().unwrap_or(None),
        )
    }
}
