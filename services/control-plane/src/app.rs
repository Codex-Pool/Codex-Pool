use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context};
use axum::body::Body;
use axum::extract::{rejection::JsonRejection, DefaultBodyLimit, Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::http::{HeaderMap, HeaderName, HeaderValue, Request};
use axum::middleware;
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::{delete, get, patch, post, put};
use axum::Json;
use axum::{response::IntoResponse, Router};
use chrono::{DateTime, Utc};
use codex_pool_core::api::{
    AccountUsageLeaderboardResponse, AdminLoginRequest, AdminMeResponse,
    AiErrorLearningSettingsResponse, ApiKeyUsageLeaderboardResponse, BuiltinErrorTemplateResponse,
    BuiltinErrorTemplatesResponse, CreateApiKeyRequest, CreateTenantRequest,
    CreateUpstreamAccountRequest, ErrorEnvelope, HourlyAccountUsagePoint,
    HourlyTenantApiKeyUsagePoint, ImportOAuthRefreshTokenRequest, ModelRoutingPoliciesResponse,
    ModelRoutingSettingsResponse, OAuthAccountStatusResponse, OAuthFamilyActionResponse,
    OAuthImportItemStatus, OAuthImportJobActionResponse, OAuthImportJobItemsResponse,
    OAuthImportJobSummary, OAuthRateLimitRefreshJobStatus, OAuthRateLimitRefreshJobSummary,
    PolicyResponse, ProductEdition, ResolveUpstreamErrorTemplateRequest,
    ResolveUpstreamErrorTemplateResponse, RoutingPlanVersionsResponse, RoutingProfilesResponse,
    SystemCapabilitiesResponse, TenantUsageLeaderboardResponse,
    UpdateAiErrorLearningSettingsRequest, UpdateBuiltinErrorTemplateRequest,
    UpdateModelRoutingSettingsRequest, UpdateUpstreamErrorTemplateRequest,
    UpsertModelRoutingPolicyRequest, UpsertRetryPolicyRequest, UpsertRoutingPolicyRequest,
    UpsertRoutingProfileRequest, UpsertStreamRetryPolicyRequest, UpstreamErrorTemplateResponse,
    UpstreamErrorTemplatesResponse, UsageHourlyTenantTrendsResponse, UsageHourlyTrendsResponse,
    UsageLeaderboardOverviewResponse, UsageQueryResponse, UsageSummaryQueryResponse,
    ValidateApiKeyRequest, ValidateApiKeyResponse, ValidateOAuthRefreshTokenRequest,
    ValidateOAuthRefreshTokenResponse,
};
use codex_pool_core::model::{
    ApiKey, BuiltinErrorTemplateKind, BuiltinErrorTemplateOverrideRecord,
    BuiltinErrorTemplateRecord, LocalizedErrorTemplates, ModelRoutingPolicy, RoutingProfile,
    UpstreamAccount, UpstreamErrorTemplateRecord, UpstreamErrorTemplateStatus,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::admin_auth::{AdminAuthService, AdminPrincipal};
use crate::import_jobs::{
    CreateOAuthImportJobOptions, ImportUploadFile, InMemoryOAuthImportJobStore,
    OAuthImportJobManager, OAuthImportJobStore,
};
use crate::store::{ControlPlaneStore, InMemoryStore};
use crate::tenant::TenantAuthService;
use crate::usage::{
    clickhouse_repo::UsageQueryRepository, sqlite_repo::SqliteUsageRepo, UsageIngestRepository,
};

#[path = "i18n.rs"]
mod i18n;

pub const DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC: u64 = 30;
const DEFAULT_USAGE_QUERY_LIMIT: u32 = 200;
const MAX_USAGE_QUERY_LIMIT: u32 = 1000;
const DEFAULT_USAGE_LEADERBOARD_LIMIT: u32 = 20;
const MAX_USAGE_LEADERBOARD_LIMIT: u32 = 200;
const DEFAULT_DATA_PLANE_BASE_URL: &str = "http://127.0.0.1:8091";
const DEFAULT_AUTH_VALIDATE_URL: &str = "http://127.0.0.1:8090/internal/v1/auth/validate";
const ADMIN_LOG_CAPACITY: usize = 500;
const MODEL_PROBE_CACHE_TTL_SEC: i64 = 3600;
const MODEL_PROBE_DEFAULT_INTERVAL_SEC: u64 = 3600;
const MODEL_PROBE_REQUEST_TIMEOUT_SEC: u64 = 8;
const MODEL_PROBE_ACCOUNT_FETCH_CONCURRENCY: usize = 64;
const MODEL_PROBE_INTERVAL_ENV: &str = "CONTROL_PLANE_MODEL_PROBE_INTERVAL_SEC";
const INTERNAL_AUTH_TOKEN_ENV: &str = "CONTROL_PLANE_INTERNAL_AUTH_TOKEN";
const OAUTH_IMPORT_MULTIPART_MAX_MB_ENV: &str = "CONTROL_PLANE_OAUTH_IMPORT_MULTIPART_MAX_MB";
const DEFAULT_OAUTH_IMPORT_MULTIPART_MAX_MB: usize = 256;
const MIN_OAUTH_IMPORT_MULTIPART_MAX_MB: usize = 8;
const MAX_OAUTH_IMPORT_MULTIPART_MAX_MB: usize = 1024;
const IMPORT_JOB_CONCURRENCY_ENV: &str = "CONTROL_PLANE_IMPORT_JOB_CONCURRENCY";
const DEFAULT_IMPORT_JOB_CONCURRENCY: usize = 8;
const MIN_IMPORT_JOB_CONCURRENCY: usize = 1;
const MAX_IMPORT_JOB_CONCURRENCY: usize = 64;
const IMPORT_JOB_CLAIM_BATCH_SIZE_ENV: &str = "CONTROL_PLANE_IMPORT_JOB_CLAIM_BATCH_SIZE";
const DEFAULT_IMPORT_JOB_CLAIM_BATCH_SIZE: usize = 200;
const MIN_IMPORT_JOB_CLAIM_BATCH_SIZE: usize = 1;
const MAX_IMPORT_JOB_CLAIM_BATCH_SIZE: usize = 2000;
const OAUTH_LOGIN_SESSION_TTL_SEC: i64 = 10 * 60;
const OAUTH_LOGIN_SESSION_RETENTION_SEC: i64 = 30 * 60;
const CODEX_OAUTH_CALLBACK_LISTEN_MODE_ENV: &str = "CODEX_OAUTH_CALLBACK_LISTEN_MODE";
const CODEX_OAUTH_CALLBACK_LISTEN_ENV: &str = "CODEX_OAUTH_CALLBACK_LISTEN";
const DEFAULT_CODEX_OAUTH_CALLBACK_LISTEN_ADDR: &str = "127.0.0.1:1455";
const UPSTREAM_ACCOUNT_BATCH_ACTION_MAX_ITEMS: usize = 20_000;
const UPSTREAM_ACCOUNT_BATCH_ACTION_CONCURRENCY_ENV: &str =
    "CONTROL_PLANE_UPSTREAM_ACCOUNT_BATCH_ACTION_CONCURRENCY";
const DEFAULT_UPSTREAM_ACCOUNT_BATCH_ACTION_CONCURRENCY: usize = 16;
const MIN_UPSTREAM_ACCOUNT_BATCH_ACTION_CONCURRENCY: usize = 1;
const MAX_UPSTREAM_ACCOUNT_BATCH_ACTION_CONCURRENCY: usize = 64;
const REQUEST_ID_HEADER: &str = "x-request-id";
const CODEX_POOL_EDITION_ENV: &str = "CODEX_POOL_EDITION";

fn read_request_id_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn ensure_request_id(headers: &mut HeaderMap) -> String {
    let request_id =
        read_request_id_from_headers(headers).unwrap_or_else(|| Uuid::new_v4().to_string());
    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        headers.insert(HeaderName::from_static(REQUEST_ID_HEADER), header_value);
    }
    request_id
}

async fn request_id_middleware(mut req: Request<Body>, next: Next) -> Response {
    let request_id = ensure_request_id(req.headers_mut());
    let mut response = next.run(req).await;
    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        response
            .headers_mut()
            .insert(HeaderName::from_static(REQUEST_ID_HEADER), header_value);
    }
    response
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeConfigSnapshot {
    control_plane_listen: String,
    data_plane_base_url: String,
    auth_validate_url: String,
    oauth_refresh_enabled: bool,
    oauth_refresh_interval_sec: u64,
    database_url: Option<String>,
    redis_url: Option<String>,
    clickhouse_url: Option<String>,
    notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeConfigUpdateRequest {
    data_plane_base_url: Option<String>,
    auth_validate_url: Option<String>,
    oauth_refresh_enabled: Option<bool>,
    oauth_refresh_interval_sec: Option<u64>,
    notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AdminLogEntry {
    id: u64,
    ts: DateTime<Utc>,
    level: String,
    action: String,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AdminLogsResponse {
    items: Vec<AdminLogEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct AdminProxyTestRequest {
    proxy_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct AdminSystemCounts {
    total_accounts: usize,
    enabled_accounts: usize,
    oauth_accounts: usize,
    api_keys: usize,
    tenants: usize,
}

#[derive(Debug, Clone, Serialize)]
struct AdminSystemStateResponse {
    generated_at: DateTime<Utc>,
    started_at: DateTime<Utc>,
    uptime_sec: i64,
    usage_repo_available: bool,
    config: RuntimeConfigSnapshot,
    counts: AdminSystemCounts,
    control_plane_debug: crate::tenant::BillingReconcileRuntimeSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    data_plane_debug: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data_plane_error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct AdminLogsQuery {
    limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
struct AdminUsageOverviewQuery {
    start_ts: i64,
    end_ts: i64,
    #[serde(default = "default_usage_leaderboard_limit")]
    limit: u32,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct AdminModelsProbeRequest {
    #[serde(default = "default_probe_force")]
    force: bool,
    #[serde(default)]
    models: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct AdminKeyCreateRequest {
    name: String,
    tenant_id: Option<Uuid>,
    tenant_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct AdminKeyPatchRequest {
    enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct UpstreamAccountPatchRequest {
    enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
struct AdminUsageOverviewResponse {
    start_ts: i64,
    end_ts: i64,
    summary: UsageSummaryQueryResponse,
    tenants: Vec<codex_pool_core::api::TenantUsageLeaderboardItem>,
    accounts: Vec<codex_pool_core::api::AccountUsageLeaderboardItem>,
    api_keys: Vec<codex_pool_core::api::ApiKeyUsageLeaderboardItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum AdminModelAvailabilityStatus {
    Unknown,
    Available,
    Unavailable,
}

#[derive(Debug, Clone, Serialize)]
struct AdminModelPricingView {
    input_price_microcredits: Option<i64>,
    cached_input_price_microcredits: Option<i64>,
    output_price_microcredits: Option<i64>,
    source: String,
}

#[derive(Debug, Clone, Serialize)]
struct AdminModelOfficialInfo {
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    context_window_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    knowledge_cutoff: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_token_support: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pricing_notes: Option<String>,
    input_modalities: Vec<String>,
    output_modalities: Vec<String>,
    endpoints: Vec<String>,
    source_url: String,
    synced_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    raw_text: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct AdminModelItem {
    id: String,
    owned_by: String,
    availability_status: AdminModelAvailabilityStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    availability_checked_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    availability_http_status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    availability_error: Option<String>,
    official: AdminModelOfficialInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    override_pricing: Option<crate::tenant::ModelPricingItem>,
    effective_pricing: AdminModelPricingView,
}

#[derive(Debug, Clone, Serialize)]
struct AdminModelsMeta {
    probe_cache_ttl_sec: i64,
    probe_cache_stale: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    probe_cache_updated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    probe_source_account_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    catalog_synced_at: Option<DateTime<Utc>>,
    catalog_sync_required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    catalog_last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct AdminModelsResponse {
    object: String,
    data: Vec<AdminModelItem>,
    meta: AdminModelsMeta,
}

#[derive(Debug, Clone)]
struct ModelProbeCacheEntry {
    status: AdminModelAvailabilityStatus,
    checked_at: DateTime<Utc>,
    http_status: Option<u16>,
    error: Option<String>,
    owned_by: Option<String>,
}

#[derive(Debug, Default)]
struct ModelProbeCache {
    updated_at: Option<DateTime<Utc>>,
    source_account_label: Option<String>,
    entries: HashMap<String, ModelProbeCacheEntry>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum CodexOAuthLoginSessionStatus {
    WaitingCallback,
    Exchanging,
    Importing,
    Completed,
    Failed,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodexOAuthLoginSessionError {
    code: String,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodexOAuthLoginSessionResult {
    created: bool,
    account: UpstreamAccount,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chatgpt_account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chatgpt_plan_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodexOAuthProbeSessionResult {
    base_url: String,
    access_token_present: bool,
    refresh_token_present: bool,
    id_token_present: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    access_token_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    refresh_token_preview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
    expires_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chatgpt_account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chatgpt_plan_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    raw_exchange_payload: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id_token_claims_raw: Option<Value>,
}

#[derive(Debug, Clone)]
struct CodexOAuthLoginSessionRecord {
    session_id: String,
    state: String,
    code_verifier: String,
    authorize_url: String,
    callback_url: String,
    base_url: String,
    label: Option<String>,
    enabled: bool,
    priority: i32,
    status: CodexOAuthLoginSessionStatus,
    error: Option<CodexOAuthLoginSessionError>,
    result: Option<CodexOAuthLoginSessionResult>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct CodexOAuthProbeSessionRecord {
    session_id: String,
    state: String,
    code_verifier: String,
    authorize_url: String,
    callback_url: String,
    base_url: String,
    status: CodexOAuthLoginSessionStatus,
    error: Option<CodexOAuthLoginSessionError>,
    result: Option<CodexOAuthProbeSessionResult>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

struct CodexOAuthCallbackListenerRuntime {
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
    join_handle: tokio::task::JoinHandle<()>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexOAuthCallbackListenMode {
    Always,
    OnDemand,
    Off,
}

pub fn codex_oauth_callback_listen_mode_from_env() -> CodexOAuthCallbackListenMode {
    std::env::var(CODEX_OAUTH_CALLBACK_LISTEN_MODE_ENV)
        .ok()
        .map(|raw| raw.trim().to_ascii_lowercase())
        .and_then(|normalized| match normalized.as_str() {
            "always" => Some(CodexOAuthCallbackListenMode::Always),
            "on_demand" | "ondemand" | "lazy" => Some(CodexOAuthCallbackListenMode::OnDemand),
            "off" | "none" | "disabled" => Some(CodexOAuthCallbackListenMode::Off),
            _ => None,
        })
        .unwrap_or(CodexOAuthCallbackListenMode::Always)
}

fn codex_oauth_callback_listen_addr_from_env() -> Option<SocketAddr> {
    let raw = std::env::var(CODEX_OAUTH_CALLBACK_LISTEN_ENV)
        .unwrap_or_else(|_| DEFAULT_CODEX_OAUTH_CALLBACK_LISTEN_ADDR.to_string());
    let normalized = raw.trim();
    if normalized.is_empty() {
        return None;
    }
    let lowered = normalized.to_ascii_lowercase();
    if lowered == "off" || lowered == "none" || lowered == "disabled" {
        return None;
    }
    match normalized.parse::<SocketAddr>() {
        Ok(addr) => Some(addr),
        Err(err) => {
            tracing::warn!(
                env = CODEX_OAUTH_CALLBACK_LISTEN_ENV,
                value = normalized,
                error = %err,
                "invalid oauth callback listen address; on-demand callback listener is disabled"
            );
            None
        }
    }
}

async fn ensure_codex_oauth_callback_listener_started(state: &AppState) -> anyhow::Result<()> {
    if state.codex_oauth_callback_listen_mode != CodexOAuthCallbackListenMode::OnDemand {
        return Ok(());
    }
    let listen_addr = state
        .codex_oauth_callback_listen_addr
        .ok_or_else(|| anyhow!("oauth callback listen address is not configured"))?;
    let mut guard = state.codex_oauth_callback_listener.lock().await;
    if guard.is_some() {
        return Ok(());
    }

    let listener = tokio::net::TcpListener::bind(listen_addr)
        .await
        .with_context(|| format!("failed to bind oauth callback listener at {listen_addr}"))?;
    let callback_app = Router::new()
        .route("/auth/callback", get(handle_codex_oauth_callback))
        .route(
            "/auth/probe-callback",
            get(handle_codex_oauth_probe_callback),
        )
        .route(
            "/api/v1/upstream-accounts/oauth/codex/callback",
            get(handle_codex_oauth_callback),
        )
        .route(
            "/api/v1/upstream-accounts/oauth/codex/probe/callback",
            get(handle_codex_oauth_probe_callback),
        )
        .with_state(state.clone());

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let join_handle = tokio::spawn(async move {
        let shutdown = async move {
            let _ = shutdown_rx.await;
        };
        if let Err(err) = axum::serve(listener, callback_app)
            .with_graceful_shutdown(shutdown)
            .await
        {
            tracing::warn!(
                listen_addr = %listen_addr,
                error = %err,
                "codex oauth on-demand callback listener exited with error"
            );
        }
    });
    *guard = Some(CodexOAuthCallbackListenerRuntime {
        shutdown_tx,
        join_handle,
    });
    tracing::info!(
        listen_addr = %listen_addr,
        "codex oauth on-demand callback listener started"
    );
    Ok(())
}

async fn stop_codex_oauth_callback_listener_if_idle(state: &AppState) {
    if state.codex_oauth_callback_listen_mode != CodexOAuthCallbackListenMode::OnDemand {
        return;
    }
    let has_pending_sessions = {
        let mut sessions = state
            .oauth_login_sessions
            .write()
            .expect("oauth login session lock poisoned");
        cleanup_codex_oauth_login_sessions(&mut sessions);
        sessions.values().any(|session| {
            matches!(
                session.status,
                CodexOAuthLoginSessionStatus::WaitingCallback
                    | CodexOAuthLoginSessionStatus::Exchanging
                    | CodexOAuthLoginSessionStatus::Importing
            )
        })
    };
    let has_pending_probe_sessions = {
        let mut sessions = state
            .oauth_probe_sessions
            .write()
            .expect("oauth probe session lock poisoned");
        cleanup_codex_oauth_probe_sessions(&mut sessions);
        sessions.values().any(|session| {
            matches!(
                session.status,
                CodexOAuthLoginSessionStatus::WaitingCallback
                    | CodexOAuthLoginSessionStatus::Exchanging
                    | CodexOAuthLoginSessionStatus::Importing
            )
        })
    };
    if has_pending_sessions || has_pending_probe_sessions {
        return;
    }

    let runtime = {
        let mut guard = state.codex_oauth_callback_listener.lock().await;
        guard.take()
    };
    if let Some(runtime) = runtime {
        let _ = runtime.shutdown_tx.send(());
        let _ = runtime.join_handle.await;
        tracing::info!("codex oauth on-demand callback listener stopped (idle)");
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct AppState {
    pub store: Arc<dyn ControlPlaneStore>,
    pub usage_repo: Option<Arc<dyn UsageQueryRepository>>,
    pub usage_ingest_repo: Option<Arc<dyn UsageIngestRepository>>,
    pub tenant_auth_service: Option<Arc<TenantAuthService>>,
    pub sqlite_usage_repo: Option<Arc<SqliteUsageRepo>>,
    pub auth_validate_cache_ttl_sec: u64,
    pub system_capabilities: SystemCapabilitiesResponse,
    pub admin_auth: AdminAuthService,
    pub internal_auth_token: Arc<str>,
    pub import_job_manager: OAuthImportJobManager,
    pub started_at: DateTime<Utc>,
    runtime_config: Arc<std::sync::RwLock<RuntimeConfigSnapshot>>,
    admin_logs: Arc<std::sync::RwLock<VecDeque<AdminLogEntry>>>,
    model_catalog_last_error: Arc<std::sync::RwLock<Option<String>>>,
    model_probe_cache: Arc<std::sync::RwLock<ModelProbeCache>>,
    oauth_login_sessions: Arc<std::sync::RwLock<HashMap<String, CodexOAuthLoginSessionRecord>>>,
    oauth_probe_sessions: Arc<std::sync::RwLock<HashMap<String, CodexOAuthProbeSessionRecord>>>,
    codex_oauth_callback_listen_mode: CodexOAuthCallbackListenMode,
    codex_oauth_callback_listen_addr: Option<SocketAddr>,
    codex_oauth_callback_listener:
        Arc<tokio::sync::Mutex<Option<CodexOAuthCallbackListenerRuntime>>>,
    outbound_proxy_runtime: Arc<crate::outbound_proxy_runtime::OutboundProxyRuntime>,
    upstream_error_learning_runtime:
        Arc<crate::upstream_error_learning::UpstreamErrorLearningRuntime>,
    #[cfg_attr(test, allow(dead_code))]
    model_probe_interval_sec: u64,
}

fn build_runtime_config_from_env(auth_validate_cache_ttl_sec: u64) -> RuntimeConfigSnapshot {
    RuntimeConfigSnapshot {
        control_plane_listen: std::env::var("CONTROL_PLANE_LISTEN")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "0.0.0.0:8090".to_string()),
        data_plane_base_url: std::env::var("DATA_PLANE_BASE_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_DATA_PLANE_BASE_URL.to_string()),
        auth_validate_url: std::env::var("AUTH_VALIDATE_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_AUTH_VALIDATE_URL.to_string()),
        oauth_refresh_enabled: std::env::var("CONTROL_PLANE_OAUTH_REFRESH_ENABLED")
            .ok()
            .and_then(|raw| parse_bool_flag(&raw))
            .unwrap_or(true),
        oauth_refresh_interval_sec: std::env::var("CONTROL_PLANE_OAUTH_REFRESH_INTERVAL_SEC")
            .ok()
            .and_then(|raw| raw.parse::<u64>().ok())
            .unwrap_or(15),
        database_url: std::env::var("CONTROL_PLANE_DATABASE_URL").ok(),
        redis_url: std::env::var("REDIS_URL").ok(),
        clickhouse_url: std::env::var("CLICKHOUSE_URL").ok(),
        notes: Some(format!(
            "auth_validate_cache_ttl_sec={auth_validate_cache_ttl_sec}"
        )),
    }
}

fn product_edition_from_env() -> ProductEdition {
    ProductEdition::from_env_var(CODEX_POOL_EDITION_ENV)
}

fn system_capabilities_from_env() -> SystemCapabilitiesResponse {
    SystemCapabilitiesResponse::for_edition(product_edition_from_env())
}

fn resolve_internal_auth_token() -> anyhow::Result<Arc<str>> {
    let token = std::env::var(INTERNAL_AUTH_TOKEN_ENV)
        .with_context(|| format!("{INTERNAL_AUTH_TOKEN_ENV} is required and must be set"))?;
    let token = token.trim();
    if token.is_empty() {
        return Err(anyhow!(
            "{INTERNAL_AUTH_TOKEN_ENV} is required and must not be empty"
        ));
    }
    Ok(token.to_string().into())
}

fn push_admin_log(state: &AppState, level: &str, action: &str, message: impl Into<String>) {
    let mut logs = state.admin_logs.write().expect("admin_logs lock poisoned");
    let next_id = logs.back().map(|entry| entry.id + 1).unwrap_or(1);
    logs.push_back(AdminLogEntry {
        id: next_id,
        ts: Utc::now(),
        level: level.to_string(),
        action: action.to_string(),
        message: message.into(),
    });
    while logs.len() > ADMIN_LOG_CAPACITY {
        let _ = logs.pop_front();
    }
}

fn parse_model_probe_interval_sec() -> u64 {
    std::env::var(MODEL_PROBE_INTERVAL_ENV)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(MODEL_PROBE_DEFAULT_INTERVAL_SEC)
}

fn default_probe_force() -> bool {
    true
}

fn oauth_import_multipart_max_bytes() -> usize {
    let mb = std::env::var(OAUTH_IMPORT_MULTIPART_MAX_MB_ENV)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(DEFAULT_OAUTH_IMPORT_MULTIPART_MAX_MB)
        .clamp(
            MIN_OAUTH_IMPORT_MULTIPART_MAX_MB,
            MAX_OAUTH_IMPORT_MULTIPART_MAX_MB,
        );
    mb.saturating_mul(1024 * 1024)
}

fn import_job_concurrency_from_env() -> usize {
    std::env::var(IMPORT_JOB_CONCURRENCY_ENV)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(DEFAULT_IMPORT_JOB_CONCURRENCY)
        .clamp(MIN_IMPORT_JOB_CONCURRENCY, MAX_IMPORT_JOB_CONCURRENCY)
}

fn import_job_claim_batch_size_from_env() -> usize {
    std::env::var(IMPORT_JOB_CLAIM_BATCH_SIZE_ENV)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(DEFAULT_IMPORT_JOB_CLAIM_BATCH_SIZE)
        .clamp(
            MIN_IMPORT_JOB_CLAIM_BATCH_SIZE,
            MAX_IMPORT_JOB_CLAIM_BATCH_SIZE,
        )
}

fn upstream_account_batch_action_concurrency_from_env() -> usize {
    std::env::var(UPSTREAM_ACCOUNT_BATCH_ACTION_CONCURRENCY_ENV)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(DEFAULT_UPSTREAM_ACCOUNT_BATCH_ACTION_CONCURRENCY)
        .clamp(
            MIN_UPSTREAM_ACCOUNT_BATCH_ACTION_CONCURRENCY,
            MAX_UPSTREAM_ACCOUNT_BATCH_ACTION_CONCURRENCY,
        )
}

#[cfg(test)]
mod app_env_tests {
    use super::{
        import_job_claim_batch_size_from_env, import_job_concurrency_from_env,
        oauth_import_multipart_max_bytes, resolve_internal_auth_token,
    };
    use crate::test_support::{set_env, ENV_LOCK};

    #[test]
    fn resolve_internal_auth_token_fails_when_missing() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old_internal = set_env("CONTROL_PLANE_INTERNAL_AUTH_TOKEN", None);

        let result = resolve_internal_auth_token();
        assert!(result.is_err());

        set_env("CONTROL_PLANE_INTERNAL_AUTH_TOKEN", old_internal.as_deref());
    }

    #[test]
    fn oauth_import_multipart_max_bytes_has_safe_default() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old = set_env("CONTROL_PLANE_OAUTH_IMPORT_MULTIPART_MAX_MB", None);

        assert_eq!(oauth_import_multipart_max_bytes(), 256 * 1024 * 1024);

        set_env(
            "CONTROL_PLANE_OAUTH_IMPORT_MULTIPART_MAX_MB",
            old.as_deref(),
        );
    }

    #[test]
    fn oauth_import_multipart_max_bytes_clamps_invalid_low_values() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old = set_env("CONTROL_PLANE_OAUTH_IMPORT_MULTIPART_MAX_MB", Some("0"));

        assert_eq!(oauth_import_multipart_max_bytes(), 8 * 1024 * 1024);

        set_env(
            "CONTROL_PLANE_OAUTH_IMPORT_MULTIPART_MAX_MB",
            old.as_deref(),
        );
    }

    #[test]
    fn import_job_concurrency_uses_safe_default() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old = set_env("CONTROL_PLANE_IMPORT_JOB_CONCURRENCY", None);

        assert_eq!(import_job_concurrency_from_env(), 8);

        set_env("CONTROL_PLANE_IMPORT_JOB_CONCURRENCY", old.as_deref());
    }

    #[test]
    fn import_job_concurrency_clamps_invalid_high_values() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old = set_env("CONTROL_PLANE_IMPORT_JOB_CONCURRENCY", Some("999"));

        assert_eq!(import_job_concurrency_from_env(), 64);

        set_env("CONTROL_PLANE_IMPORT_JOB_CONCURRENCY", old.as_deref());
    }

    #[test]
    fn import_job_claim_batch_size_uses_safe_default() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old = set_env("CONTROL_PLANE_IMPORT_JOB_CLAIM_BATCH_SIZE", None);

        assert_eq!(import_job_claim_batch_size_from_env(), 200);

        set_env("CONTROL_PLANE_IMPORT_JOB_CLAIM_BATCH_SIZE", old.as_deref());
    }

    #[test]
    fn import_job_claim_batch_size_clamps_invalid_high_values() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old = set_env("CONTROL_PLANE_IMPORT_JOB_CLAIM_BATCH_SIZE", Some("99999"));

        assert_eq!(import_job_claim_batch_size_from_env(), 2000);

        set_env("CONTROL_PLANE_IMPORT_JOB_CLAIM_BATCH_SIZE", old.as_deref());
    }
}

#[cfg(test)]
mod capabilities_tests {
    use super::{
        build_app_with_store,
        build_app_with_store_ttl_usage_repos_import_store_admin_auth_and_sqlite_repo,
        DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC,
    };
    use crate::admin_auth::AdminAuthService;
    use crate::import_jobs::InMemoryOAuthImportJobStore;
    use crate::store::{
        normalize_sqlite_database_url, ControlPlaneStore, InMemoryStore, SqliteBackedStore,
    };
    use crate::test_support::{set_env, ENV_LOCK};
    use crate::usage::sqlite_repo::SqliteUsageRepo;
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use serde_json::Value;
    use sqlx_core::pool::PoolOptions;
    use sqlx_sqlite::Sqlite;
    use std::sync::Arc;
    use tower::ServiceExt;
    use uuid::Uuid;

    const TEST_ADMIN_USERNAME: &str = "admin";
    const TEST_ADMIN_PASSWORD: &str = "admin123456";
    const TEST_ADMIN_JWT_SECRET: &str = "control-plane-test-jwt-secret";
    const TEST_INTERNAL_AUTH_TOKEN: &str = "control-plane-test-internal-auth-token";

    fn build_test_app() -> axum::Router {
        let store: Arc<dyn ControlPlaneStore> = Arc::new(InMemoryStore::default());
        build_app_with_store(store)
    }

    async fn build_personal_models_test_app() -> axum::Router {
        let path = std::env::temp_dir().join(format!(
            "codex-pool-personal-models-{}.sqlite3",
            Uuid::new_v4()
        ));
        let database_url = normalize_sqlite_database_url(&path.display().to_string());
        let pool = PoolOptions::<Sqlite>::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .expect("connect personal sqlite models db");
        let sqlite_usage_repo = Arc::new(
            SqliteUsageRepo::new(pool)
                .await
                .expect("create personal sqlite usage repo"),
        );
        sqlite_usage_repo
            .apply_openai_model_catalog_items(
                chrono::Utc::now(),
                &[crate::tenant::OpenAiModelCatalogItem {
                    model_id: "gpt-5.4".to_string(),
                    owned_by: "openai".to_string(),
                    title: "GPT-5.4".to_string(),
                    description: Some("Latest reasoning model".to_string()),
                    context_window_tokens: Some(400_000),
                    max_output_tokens: Some(128_000),
                    knowledge_cutoff: Some("Mar 1, 2025".to_string()),
                    reasoning_token_support: Some(true),
                    input_price_microcredits: Some(1_250_000),
                    cached_input_price_microcredits: Some(125_000),
                    output_price_microcredits: Some(10_000_000),
                    pricing_notes: None,
                    input_modalities: vec!["text".to_string()],
                    output_modalities: vec!["text".to_string()],
                    endpoints: vec!["v1/responses".to_string()],
                    source_url: "https://developers.openai.com/api/docs/models/gpt-5.4".to_string(),
                    raw_text: Some("model page".to_string()),
                    synced_at: chrono::Utc::now(),
                }],
            )
            .await
            .expect("seed personal sqlite catalog");
        let store: Arc<dyn ControlPlaneStore> = Arc::new(
            SqliteBackedStore::connect(&database_url)
                .await
                .expect("connect personal sqlite store"),
        );
        let admin_auth = AdminAuthService::from_env().expect("admin auth");
        build_app_with_store_ttl_usage_repos_import_store_admin_auth_and_sqlite_repo(
            store,
            DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC,
            Some(sqlite_usage_repo.clone()),
            Some(sqlite_usage_repo.clone()),
            Arc::new(InMemoryOAuthImportJobStore::default()),
            admin_auth,
            Some(sqlite_usage_repo),
        )
    }

    fn configure_test_env(edition: Option<&str>) -> [Option<String>; 5] {
        [
            set_env("ADMIN_USERNAME", Some(TEST_ADMIN_USERNAME)),
            set_env("ADMIN_PASSWORD", Some(TEST_ADMIN_PASSWORD)),
            set_env("ADMIN_JWT_SECRET", Some(TEST_ADMIN_JWT_SECRET)),
            set_env(
                "CONTROL_PLANE_INTERNAL_AUTH_TOKEN",
                Some(TEST_INTERNAL_AUTH_TOKEN),
            ),
            set_env("CODEX_POOL_EDITION", edition),
        ]
    }

    fn restore_test_env(old_values: [Option<String>; 5]) {
        let [old_username, old_password, old_secret, old_internal, old_edition] = old_values;
        set_env("ADMIN_USERNAME", old_username.as_deref());
        set_env("ADMIN_PASSWORD", old_password.as_deref());
        set_env("ADMIN_JWT_SECRET", old_secret.as_deref());
        set_env("CONTROL_PLANE_INTERNAL_AUTH_TOKEN", old_internal.as_deref());
        set_env("CODEX_POOL_EDITION", old_edition.as_deref());
    }

    fn json_request(method: &str, uri: &str, body: &str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_owned()))
            .unwrap()
    }

    async fn login_and_get_admin_token(app: &axum::Router) -> String {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "username": TEST_ADMIN_USERNAME,
                            "password": TEST_ADMIN_PASSWORD,
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .expect("admin login response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        payload["access_token"]
            .as_str()
            .expect("access token present")
            .to_string()
    }

    #[tokio::test]
    async fn system_capabilities_endpoint_defaults_to_business() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old_values = configure_test_env(None);

        let response = build_test_app()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/system/capabilities")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        restore_test_env(old_values);

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["edition"], "business");
        assert_eq!(payload["billing_mode"], "credit_enforced");
        assert_eq!(payload["features"]["multi_tenant"], true);
        assert_eq!(payload["features"]["tenant_portal"], true);
        assert_eq!(payload["features"]["tenant_self_service"], true);
        assert_eq!(payload["features"]["tenant_recharge"], true);
        assert_eq!(payload["features"]["cost_reports"], true);
    }

    #[tokio::test]
    async fn system_capabilities_endpoint_reflects_personal_edition() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old_values = configure_test_env(Some("personal"));

        let response = build_test_app()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/system/capabilities")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        restore_test_env(old_values);

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["edition"], "personal");
        assert_eq!(payload["billing_mode"], "cost_report_only");
        assert_eq!(payload["features"]["multi_tenant"], false);
        assert_eq!(payload["features"]["tenant_portal"], false);
        assert_eq!(payload["features"]["tenant_self_service"], false);
        assert_eq!(payload["features"]["tenant_recharge"], false);
        assert_eq!(payload["features"]["credit_billing"], false);
        assert_eq!(payload["features"]["cost_reports"], true);
    }

    #[tokio::test]
    async fn system_capabilities_endpoint_reflects_team_edition() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old_values = configure_test_env(Some("team"));

        let response = build_test_app()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/system/capabilities")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        restore_test_env(old_values);

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["edition"], "team");
        assert_eq!(payload["billing_mode"], "cost_report_only");
        assert_eq!(payload["features"]["multi_tenant"], true);
        assert_eq!(payload["features"]["tenant_portal"], true);
        assert_eq!(payload["features"]["tenant_self_service"], false);
        assert_eq!(payload["features"]["tenant_recharge"], false);
        assert_eq!(payload["features"]["credit_billing"], false);
        assert_eq!(payload["features"]["cost_reports"], true);
    }

    #[tokio::test]
    async fn personal_edition_hides_tenant_and_credit_routes() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old_values = configure_test_env(Some("personal"));
        let tenant_id = Uuid::new_v4();

        let login_response = build_test_app()
            .oneshot(json_request("POST", "/api/v1/tenant/auth/login", "{}"))
            .await
            .unwrap();
        let recharge_response = build_test_app()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/admin/tenants/{tenant_id}/credits/recharge"),
                "{}",
            ))
            .await
            .unwrap();
        let authorize_response = build_test_app()
            .oneshot(json_request("POST", "/internal/v1/billing/authorize", "{}"))
            .await
            .unwrap();

        restore_test_env(old_values);

        assert_eq!(login_response.status(), StatusCode::NOT_FOUND);
        assert_eq!(recharge_response.status(), StatusCode::NOT_FOUND);
        assert_eq!(authorize_response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn team_edition_keeps_login_but_hides_self_service_and_credit_routes() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old_values = configure_test_env(Some("team"));
        let tenant_id = Uuid::new_v4();

        let login_response = build_test_app()
            .oneshot(json_request("POST", "/api/v1/tenant/auth/login", "{}"))
            .await
            .unwrap();
        let register_response = build_test_app()
            .oneshot(json_request("POST", "/api/v1/tenant/auth/register", "{}"))
            .await
            .unwrap();
        let forgot_response = build_test_app()
            .oneshot(json_request(
                "POST",
                "/api/v1/tenant/auth/password/forgot",
                "{}",
            ))
            .await
            .unwrap();
        let tenant_credit_response = build_test_app()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/tenant/credits/balance")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let admin_recharge_response = build_test_app()
            .oneshot(json_request(
                "POST",
                &format!("/api/v1/admin/tenants/{tenant_id}/credits/recharge"),
                "{}",
            ))
            .await
            .unwrap();
        let authorize_response = build_test_app()
            .oneshot(json_request("POST", "/internal/v1/billing/authorize", "{}"))
            .await
            .unwrap();

        restore_test_env(old_values);

        assert_ne!(login_response.status(), StatusCode::NOT_FOUND);
        assert_eq!(register_response.status(), StatusCode::NOT_FOUND);
        assert_eq!(forgot_response.status(), StatusCode::NOT_FOUND);
        assert_eq!(tenant_credit_response.status(), StatusCode::NOT_FOUND);
        assert_eq!(admin_recharge_response.status(), StatusCode::NOT_FOUND);
        assert_eq!(authorize_response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn business_edition_keeps_internal_billing_routes_registered() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old_values = configure_test_env(Some("business"));

        let authorize_response = build_test_app()
            .oneshot(json_request("POST", "/internal/v1/billing/authorize", "{}"))
            .await
            .unwrap();

        restore_test_env(old_values);

        assert_ne!(authorize_response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn personal_edition_models_and_pricing_routes_use_sqlite_repo_instead_of_503() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old_values = configure_test_env(Some("personal"));
        let app = build_personal_models_test_app().await;
        let access_token = login_and_get_admin_token(&app).await;

        let list_models_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/admin/models")
                    .header("authorization", format!("Bearer {access_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let upsert_pricing_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/model-pricing")
                    .header("authorization", format!("Bearer {access_token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "model": "gpt-5.4",
                            "input_price_microcredits": 2000000,
                            "cached_input_price_microcredits": 200000,
                            "output_price_microcredits": 8000000,
                            "enabled": true,
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        restore_test_env(old_values);

        assert_eq!(list_models_response.status(), StatusCode::OK);
        let list_models_body = to_bytes(list_models_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let list_models_payload: Value = serde_json::from_slice(&list_models_body).unwrap();
        assert_eq!(list_models_payload["data"][0]["id"], "gpt-5.4");
        assert_eq!(upsert_pricing_response.status(), StatusCode::OK);
    }
}

#[cfg(test)]
mod usage_ingest_tests {
    use super::build_app_with_store_ttl_usage_repos_import_store_and_admin_auth;
    use crate::admin_auth::AdminAuthService;
    use crate::import_jobs::InMemoryOAuthImportJobStore;
    use crate::store::{ControlPlaneStore, InMemoryStore};
    use crate::test_support::{set_env, ENV_LOCK};
    use crate::usage::UsageIngestRepository;
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use codex_pool_core::events::RequestLogEvent;
    use serde_json::Value;
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;
    use uuid::Uuid;

    const TEST_ADMIN_USERNAME: &str = "admin";
    const TEST_ADMIN_PASSWORD: &str = "admin123456";
    const TEST_ADMIN_JWT_SECRET: &str = "control-plane-test-jwt-secret";
    const TEST_INTERNAL_AUTH_TOKEN: &str = "control-plane-test-internal-auth-token";

    #[derive(Clone, Default)]
    struct RecordingUsageIngestRepo {
        events: Arc<Mutex<Vec<RequestLogEvent>>>,
    }

    #[async_trait::async_trait]
    impl UsageIngestRepository for RecordingUsageIngestRepo {
        async fn ingest_request_log(&self, event: RequestLogEvent) -> anyhow::Result<()> {
            self.events.lock().expect("usage ingest lock").push(event);
            Ok(())
        }
    }

    fn configure_test_env() -> [Option<String>; 4] {
        [
            set_env("ADMIN_USERNAME", Some(TEST_ADMIN_USERNAME)),
            set_env("ADMIN_PASSWORD", Some(TEST_ADMIN_PASSWORD)),
            set_env("ADMIN_JWT_SECRET", Some(TEST_ADMIN_JWT_SECRET)),
            set_env(
                "CONTROL_PLANE_INTERNAL_AUTH_TOKEN",
                Some(TEST_INTERNAL_AUTH_TOKEN),
            ),
        ]
    }

    fn restore_test_env(old_values: [Option<String>; 4]) {
        let [old_username, old_password, old_secret, old_internal] = old_values;
        set_env("ADMIN_USERNAME", old_username.as_deref());
        set_env("ADMIN_PASSWORD", old_password.as_deref());
        set_env("ADMIN_JWT_SECRET", old_secret.as_deref());
        set_env("CONTROL_PLANE_INTERNAL_AUTH_TOKEN", old_internal.as_deref());
    }

    fn build_test_app(usage_ingest_repo: Option<Arc<dyn UsageIngestRepository>>) -> axum::Router {
        let store: Arc<dyn ControlPlaneStore> = Arc::new(InMemoryStore::default());
        let admin_auth = AdminAuthService::from_env().expect("admin auth");
        build_app_with_store_ttl_usage_repos_import_store_and_admin_auth(
            store,
            super::DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC,
            None,
            usage_ingest_repo,
            Arc::new(InMemoryOAuthImportJobStore::default()),
            admin_auth,
        )
    }

    fn request_log_event() -> RequestLogEvent {
        RequestLogEvent {
            id: Uuid::new_v4(),
            account_id: Uuid::new_v4(),
            tenant_id: Some(Uuid::new_v4()),
            api_key_id: Some(Uuid::new_v4()),
            event_version: 1,
            path: "/v1/chat/completions".to_string(),
            method: "POST".to_string(),
            status_code: 200,
            latency_ms: 42,
            is_stream: false,
            error_code: None,
            request_id: Some("req_usage_ingest".to_string()),
            model: Some("gpt-5.3-codex".to_string()),
            service_tier: Some("default".to_string()),
            input_tokens: Some(120),
            cached_input_tokens: Some(10),
            output_tokens: Some(60),
            reasoning_tokens: Some(5),
            first_token_latency_ms: Some(12),
            billing_phase: Some("captured".to_string()),
            authorization_id: Some(Uuid::new_v4()),
            capture_status: Some("captured".to_string()),
            created_at: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn internal_usage_ingest_returns_service_unavailable_without_repo() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old_values = configure_test_env();
        let payload = serde_json::to_string(&request_log_event()).expect("serialize event");

        let response = build_test_app(None)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/internal/v1/usage/request-logs")
                    .header(
                        "authorization",
                        format!("Bearer {TEST_INTERNAL_AUTH_TOKEN}"),
                    )
                    .header("content-type", "application/json")
                    .body(Body::from(payload))
                    .unwrap(),
            )
            .await
            .unwrap();

        restore_test_env(old_values);

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["error"]["code"], "service_unavailable");
    }

    #[tokio::test]
    async fn internal_usage_ingest_persists_event_when_authorized() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old_values = configure_test_env();
        let repo = RecordingUsageIngestRepo::default();
        let payload_event = request_log_event();
        let payload = serde_json::to_string(&payload_event).expect("serialize event");

        let response = build_test_app(Some(Arc::new(repo.clone())))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/internal/v1/usage/request-logs")
                    .header(
                        "authorization",
                        format!("Bearer {TEST_INTERNAL_AUTH_TOKEN}"),
                    )
                    .header("content-type", "application/json")
                    .body(Body::from(payload))
                    .unwrap(),
            )
            .await
            .unwrap();

        restore_test_env(old_values);

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let events = repo.events.lock().expect("usage ingest lock");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, payload_event.id);
        assert_eq!(events[0].request_id, payload_event.request_id);
    }
}

#[cfg(test)]
mod usage_cost_surface_tests {
    use super::build_app_with_store_ttl_usage_repo_import_store_and_admin_auth;
    use crate::admin_auth::AdminAuthService;
    use crate::import_jobs::InMemoryOAuthImportJobStore;
    use crate::store::{ControlPlaneStore, InMemoryStore};
    use crate::test_support::{set_env, ENV_LOCK};
    use crate::usage::clickhouse_repo::UsageQueryRepository;
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use chrono::Utc;
    use codex_pool_core::api::{
        UsageDashboardMetrics, UsageDashboardModelDistributionItem, UsageDashboardTokenBreakdown,
        UsageDashboardTokenTrendPoint, UsageSummaryQueryResponse,
    };
    use serde_json::{json, Value};
    use std::sync::Arc;
    use tower::ServiceExt;
    use uuid::Uuid;

    const TEST_ADMIN_USERNAME: &str = "admin";
    const TEST_ADMIN_PASSWORD: &str = "admin123456";
    const TEST_ADMIN_JWT_SECRET: &str = "control-plane-test-jwt-secret";
    const TEST_INTERNAL_AUTH_TOKEN: &str = "control-plane-test-internal-auth-token";

    #[derive(Clone)]
    struct CostSurfaceUsageRepo {
        rows: Vec<crate::usage::RequestLogRow>,
        summary: UsageSummaryQueryResponse,
    }

    #[async_trait::async_trait]
    impl UsageQueryRepository for CostSurfaceUsageRepo {
        async fn query_hourly_accounts(
            &self,
            _start_ts: i64,
            _end_ts: i64,
            _limit: u32,
            _account_id: Option<Uuid>,
        ) -> anyhow::Result<Vec<codex_pool_core::api::HourlyAccountUsagePoint>> {
            Ok(Vec::new())
        }

        async fn query_hourly_tenant_api_keys(
            &self,
            _start_ts: i64,
            _end_ts: i64,
            _limit: u32,
            _tenant_id: Option<Uuid>,
            _api_key_id: Option<Uuid>,
        ) -> anyhow::Result<Vec<codex_pool_core::api::HourlyTenantApiKeyUsagePoint>> {
            Ok(Vec::new())
        }

        async fn query_hourly_account_totals(
            &self,
            _start_ts: i64,
            _end_ts: i64,
            _limit: u32,
            _account_id: Option<Uuid>,
        ) -> anyhow::Result<Vec<codex_pool_core::api::HourlyUsageTotalPoint>> {
            Ok(Vec::new())
        }

        async fn query_hourly_tenant_api_key_totals(
            &self,
            _start_ts: i64,
            _end_ts: i64,
            _limit: u32,
            _tenant_id: Option<Uuid>,
            _api_key_id: Option<Uuid>,
        ) -> anyhow::Result<Vec<codex_pool_core::api::HourlyUsageTotalPoint>> {
            Ok(Vec::new())
        }

        async fn query_hourly_tenant_totals(
            &self,
            _start_ts: i64,
            _end_ts: i64,
            _limit: u32,
            _tenant_id: Option<Uuid>,
            _api_key_id: Option<Uuid>,
        ) -> anyhow::Result<Vec<codex_pool_core::api::HourlyTenantUsageTotalPoint>> {
            Ok(Vec::new())
        }

        async fn query_summary(
            &self,
            _start_ts: i64,
            _end_ts: i64,
            _tenant_id: Option<Uuid>,
            _account_id: Option<Uuid>,
            _api_key_id: Option<Uuid>,
        ) -> anyhow::Result<UsageSummaryQueryResponse> {
            Ok(self.summary.clone())
        }

        async fn query_tenant_leaderboard(
            &self,
            _start_ts: i64,
            _end_ts: i64,
            _limit: u32,
            _tenant_id: Option<Uuid>,
        ) -> anyhow::Result<Vec<codex_pool_core::api::TenantUsageLeaderboardItem>> {
            Ok(Vec::new())
        }

        async fn query_account_leaderboard(
            &self,
            _start_ts: i64,
            _end_ts: i64,
            _limit: u32,
            _account_id: Option<Uuid>,
        ) -> anyhow::Result<Vec<codex_pool_core::api::AccountUsageLeaderboardItem>> {
            Ok(Vec::new())
        }

        async fn query_api_key_leaderboard(
            &self,
            _start_ts: i64,
            _end_ts: i64,
            _limit: u32,
            _tenant_id: Option<Uuid>,
            _api_key_id: Option<Uuid>,
        ) -> anyhow::Result<Vec<codex_pool_core::api::ApiKeyUsageLeaderboardItem>> {
            Ok(Vec::new())
        }

        async fn query_request_logs(
            &self,
            _query: crate::usage::RequestLogQuery,
        ) -> anyhow::Result<Vec<crate::usage::RequestLogRow>> {
            Ok(self.rows.clone())
        }
    }

    fn configure_test_env() -> [Option<String>; 4] {
        [
            set_env("ADMIN_USERNAME", Some(TEST_ADMIN_USERNAME)),
            set_env("ADMIN_PASSWORD", Some(TEST_ADMIN_PASSWORD)),
            set_env("ADMIN_JWT_SECRET", Some(TEST_ADMIN_JWT_SECRET)),
            set_env(
                "CONTROL_PLANE_INTERNAL_AUTH_TOKEN",
                Some(TEST_INTERNAL_AUTH_TOKEN),
            ),
        ]
    }

    fn restore_test_env(old_values: [Option<String>; 4]) {
        let [old_username, old_password, old_secret, old_internal] = old_values;
        set_env("ADMIN_USERNAME", old_username.as_deref());
        set_env("ADMIN_PASSWORD", old_password.as_deref());
        set_env("ADMIN_JWT_SECRET", old_secret.as_deref());
        set_env("CONTROL_PLANE_INTERNAL_AUTH_TOKEN", old_internal.as_deref());
    }

    fn build_test_app(usage_repo: Arc<dyn UsageQueryRepository>) -> axum::Router {
        let store: Arc<dyn ControlPlaneStore> = Arc::new(InMemoryStore::default());
        let admin_auth = AdminAuthService::from_env().expect("admin auth");
        build_app_with_store_ttl_usage_repo_import_store_and_admin_auth(
            store,
            super::DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC,
            Some(usage_repo),
            Arc::new(InMemoryOAuthImportJobStore::default()),
            admin_auth,
        )
    }

    async fn login_and_get_admin_token(app: &axum::Router) -> String {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "username": TEST_ADMIN_USERNAME,
                            "password": TEST_ADMIN_PASSWORD,
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .expect("admin login response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        payload["access_token"]
            .as_str()
            .expect("access_token must be present")
            .to_string()
    }

    fn sample_summary() -> UsageSummaryQueryResponse {
        UsageSummaryQueryResponse {
            start_ts: 1_700_000_000,
            end_ts: 1_700_086_400,
            account_total_requests: 3,
            tenant_api_key_total_requests: 2,
            unique_account_count: 1,
            unique_tenant_api_key_count: 1,
            estimated_cost_microusd: Some(2_750_000),
            dashboard_metrics: Some(UsageDashboardMetrics {
                total_requests: 3,
                estimated_cost_microusd: Some(2_750_000),
                token_breakdown: UsageDashboardTokenBreakdown {
                    input_tokens: 1_200,
                    cached_input_tokens: 200,
                    output_tokens: 600,
                    reasoning_tokens: 40,
                    total_tokens: 2_040,
                },
                avg_first_token_latency_ms: Some(120),
                token_trends: vec![UsageDashboardTokenTrendPoint {
                    hour_start: 1_700_000_000,
                    request_count: 2,
                    input_tokens: 800,
                    cached_input_tokens: 100,
                    output_tokens: 400,
                    reasoning_tokens: 20,
                    total_tokens: 1_320,
                    estimated_cost_microusd: Some(1_500_000),
                }],
                model_request_distribution: vec![UsageDashboardModelDistributionItem {
                    model: "gpt-5.3-codex".to_string(),
                    request_count: 3,
                    total_tokens: 2_040,
                }],
                model_token_distribution: vec![UsageDashboardModelDistributionItem {
                    model: "gpt-5.3-codex".to_string(),
                    request_count: 3,
                    total_tokens: 2_040,
                }],
            }),
        }
    }

    fn sample_request_log_row() -> crate::usage::RequestLogRow {
        crate::usage::RequestLogRow {
            id: Uuid::new_v4(),
            account_id: Uuid::new_v4(),
            tenant_id: Some(Uuid::new_v4()),
            api_key_id: Some(Uuid::new_v4()),
            request_id: Some("req_cost_surface".to_string()),
            path: "/v1/responses".to_string(),
            method: "POST".to_string(),
            model: Some("gpt-5.3-codex".to_string()),
            service_tier: Some("default".to_string()),
            input_tokens: Some(800),
            cached_input_tokens: Some(100),
            output_tokens: Some(400),
            reasoning_tokens: Some(20),
            first_token_latency_ms: Some(88),
            status_code: 200,
            latency_ms: 420,
            is_stream: false,
            error_code: None,
            billing_phase: Some("captured".to_string()),
            authorization_id: Some(Uuid::new_v4()),
            capture_status: Some("captured".to_string()),
            estimated_cost_microusd: Some(1_500_000),
            created_at: Utc::now(),
            event_version: 1,
        }
    }

    #[tokio::test]
    async fn admin_usage_summary_exposes_estimated_cost_fields() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old_values = configure_test_env();
        let app = build_test_app(Arc::new(CostSurfaceUsageRepo {
            rows: vec![sample_request_log_row()],
            summary: sample_summary(),
        }));
        let access_token = login_and_get_admin_token(&app).await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/admin/usage/summary?start_ts=1700000000&end_ts=1700086400")
                    .header("authorization", format!("Bearer {access_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        restore_test_env(old_values);

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["estimated_cost_microusd"], 2_750_000);
        assert_eq!(
            payload["dashboard_metrics"]["estimated_cost_microusd"],
            2_750_000
        );
        assert_eq!(
            payload["dashboard_metrics"]["token_trends"][0]["estimated_cost_microusd"],
            1_500_000
        );
    }

    #[tokio::test]
    async fn admin_request_logs_expose_estimated_cost_per_item() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old_values = configure_test_env();
        let app = build_test_app(Arc::new(CostSurfaceUsageRepo {
            rows: vec![sample_request_log_row()],
            summary: sample_summary(),
        }));
        let access_token = login_and_get_admin_token(&app).await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/admin/request-logs?start_ts=1700000000&end_ts=1700086400")
                    .header("authorization", format!("Bearer {access_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        restore_test_env(old_values);

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["items"][0]["estimated_cost_microusd"], 1_500_000);
    }
}

fn add_multi_tenant_routes(
    router: Router<AppState>,
    capabilities: &SystemCapabilitiesResponse,
) -> Router<AppState> {
    let router = if capabilities.allows_multi_tenant() {
        router
            .route("/api/v1/tenants", post(create_tenant).get(list_tenants))
            .route(
                "/api/v1/admin/tenants",
                get(list_admin_tenants).post(create_admin_tenant),
            )
            .route(
                "/api/v1/admin/tenants/ensure-default",
                post(ensure_default_admin_tenant),
            )
            .route(
                "/api/v1/admin/tenants/{tenant_id}",
                patch(patch_admin_tenant),
            )
    } else {
        router
    };

    if capabilities.allows_tenant_portal() {
        router
            .route(
                "/api/v1/admin/impersonations",
                post(create_admin_impersonation),
            )
            .route(
                "/api/v1/admin/impersonations/{session_id}",
                delete(delete_admin_impersonation),
            )
    } else {
        router
    }
}

fn add_admin_tenant_credit_routes(
    router: Router<AppState>,
    capabilities: &SystemCapabilitiesResponse,
) -> Router<AppState> {
    if capabilities.allows_tenant_recharge() {
        router
            .route(
                "/api/v1/admin/tenants/{tenant_id}/credits/recharge",
                post(recharge_admin_tenant_credits),
            )
            .route(
                "/api/v1/admin/tenants/{tenant_id}/credits/balance",
                get(get_admin_tenant_credit_balance),
            )
            .route(
                "/api/v1/admin/tenants/{tenant_id}/credits/summary",
                get(get_admin_tenant_credit_summary),
            )
            .route(
                "/api/v1/admin/tenants/{tenant_id}/credits/ledger",
                get(list_admin_tenant_credit_ledger),
            )
    } else {
        router
    }
}

fn add_tenant_portal_routes(
    router: Router<AppState>,
    capabilities: &SystemCapabilitiesResponse,
) -> Router<AppState> {
    if !capabilities.allows_tenant_portal() {
        return router;
    }

    let router = router
        .route("/api/v1/tenant/auth/login", post(tenant_login))
        .route("/api/v1/tenant/auth/logout", post(tenant_logout))
        .route("/api/v1/tenant/auth/me", get(tenant_me))
        .route(
            "/api/v1/tenant/keys",
            get(list_tenant_api_keys).post(create_tenant_api_key),
        )
        .route(
            "/api/v1/tenant/api-key-groups",
            get(list_tenant_api_key_groups),
        )
        .route(
            "/api/v1/tenant/keys/{key_id}",
            patch(patch_tenant_api_key).delete(delete_tenant_api_key),
        )
        .route(
            "/api/v1/tenant/usage/summary",
            get(get_tenant_usage_summary),
        )
        .route(
            "/api/v1/tenant/usage/trends/hourly",
            get(get_tenant_usage_hourly_trends),
        )
        .route(
            "/api/v1/tenant/usage/leaderboard/tenants",
            get(get_tenant_scope_tenant_usage_leaderboard),
        )
        .route(
            "/api/v1/tenant/usage/leaderboard/accounts",
            get(get_tenant_scope_account_usage_leaderboard),
        )
        .route(
            "/api/v1/tenant/usage/leaderboard/api-keys",
            get(get_tenant_scope_api_key_usage_leaderboard),
        )
        .route("/api/v1/tenant/request-logs", get(list_tenant_request_logs))
        .route("/api/v1/tenant/audit-logs", get(list_tenant_audit_logs));

    let router = if capabilities.allows_tenant_self_service() {
        router
            .route("/api/v1/tenant/auth/register", post(tenant_register))
            .route(
                "/api/v1/tenant/auth/verify-email",
                post(tenant_verify_email),
            )
            .route(
                "/api/v1/tenant/auth/password/forgot",
                post(tenant_forgot_password),
            )
            .route(
                "/api/v1/tenant/auth/password/reset",
                post(tenant_reset_password),
            )
    } else {
        router
    };

    if capabilities.allows_tenant_recharge() {
        router
            .route(
                "/api/v1/tenant/credits/balance",
                get(get_tenant_credit_balance),
            )
            .route(
                "/api/v1/tenant/credits/summary",
                get(get_tenant_credit_summary),
            )
            .route(
                "/api/v1/tenant/credits/ledger",
                get(list_tenant_credit_ledger),
            )
            .route(
                "/api/v1/tenant/credits/checkin",
                post(claim_tenant_daily_checkin),
            )
    } else {
        router
    }
}

fn add_internal_billing_routes(
    router: Router<AppState>,
    capabilities: &SystemCapabilitiesResponse,
) -> Router<AppState> {
    let router = router.route(
        "/internal/v1/billing/pricing",
        post(internal_billing_pricing),
    );

    if capabilities.allows_credit_billing() {
        router
            .route(
                "/internal/v1/billing/precheck/{tenant_id}",
                get(internal_billing_precheck),
            )
            .route(
                "/internal/v1/billing/authorize",
                post(internal_billing_authorize),
            )
            .route(
                "/internal/v1/billing/capture",
                post(internal_billing_capture),
            )
            .route(
                "/internal/v1/billing/release",
                post(internal_billing_release),
            )
    } else {
        router
    }
}

pub fn build_app() -> Router {
    build_app_with_store_ttl_and_usage_repo(
        Arc::new(InMemoryStore::default()),
        DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC,
        None,
    )
}

pub fn build_app_with_store(store: Arc<dyn ControlPlaneStore>) -> Router {
    build_app_with_store_ttl_and_usage_repo(store, DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC, None)
}

pub fn build_app_with_store_and_ttl(
    store: Arc<dyn ControlPlaneStore>,
    auth_validate_cache_ttl_sec: u64,
) -> Router {
    build_app_with_store_ttl_and_usage_repo(store, auth_validate_cache_ttl_sec, None)
}

pub fn build_app_with_store_ttl_and_usage_repo(
    store: Arc<dyn ControlPlaneStore>,
    auth_validate_cache_ttl_sec: u64,
    usage_repo: Option<Arc<dyn UsageQueryRepository>>,
) -> Router {
    build_app_with_store_ttl_usage_repo_and_import_store(
        store,
        auth_validate_cache_ttl_sec,
        usage_repo,
        Arc::new(InMemoryOAuthImportJobStore::default()),
    )
}

pub fn build_app_with_store_ttl_usage_repo_and_import_store(
    store: Arc<dyn ControlPlaneStore>,
    auth_validate_cache_ttl_sec: u64,
    usage_repo: Option<Arc<dyn UsageQueryRepository>>,
    import_job_store: Arc<dyn OAuthImportJobStore>,
) -> Router {
    let admin_auth = AdminAuthService::from_env()
        .expect("ADMIN_USERNAME/ADMIN_PASSWORD/ADMIN_JWT_SECRET must be set");

    build_app_with_store_ttl_usage_repo_import_store_and_admin_auth(
        store,
        auth_validate_cache_ttl_sec,
        usage_repo,
        import_job_store,
        admin_auth,
    )
}

pub fn build_app_with_store_ttl_usage_repo_import_store_and_admin_auth(
    store: Arc<dyn ControlPlaneStore>,
    auth_validate_cache_ttl_sec: u64,
    usage_repo: Option<Arc<dyn UsageQueryRepository>>,
    import_job_store: Arc<dyn OAuthImportJobStore>,
    admin_auth: AdminAuthService,
) -> Router {
    build_app_with_store_ttl_usage_repos_import_store_and_admin_auth(
        store,
        auth_validate_cache_ttl_sec,
        usage_repo,
        None,
        import_job_store,
        admin_auth,
    )
}

pub fn build_app_with_store_ttl_usage_repos_import_store_and_admin_auth(
    store: Arc<dyn ControlPlaneStore>,
    auth_validate_cache_ttl_sec: u64,
    usage_repo: Option<Arc<dyn UsageQueryRepository>>,
    usage_ingest_repo: Option<Arc<dyn UsageIngestRepository>>,
    import_job_store: Arc<dyn OAuthImportJobStore>,
    admin_auth: AdminAuthService,
) -> Router {
    build_app_with_store_ttl_usage_repos_import_store_admin_auth_and_sqlite_repo(
        store,
        auth_validate_cache_ttl_sec,
        usage_repo,
        usage_ingest_repo,
        import_job_store,
        admin_auth,
        None,
    )
}

pub fn build_app_with_store_ttl_usage_repos_import_store_admin_auth_and_sqlite_repo(
    store: Arc<dyn ControlPlaneStore>,
    auth_validate_cache_ttl_sec: u64,
    usage_repo: Option<Arc<dyn UsageQueryRepository>>,
    usage_ingest_repo: Option<Arc<dyn UsageIngestRepository>>,
    import_job_store: Arc<dyn OAuthImportJobStore>,
    admin_auth: AdminAuthService,
    sqlite_usage_repo: Option<Arc<SqliteUsageRepo>>,
) -> Router {
    let outbound_proxy_runtime = Arc::new(crate::outbound_proxy_runtime::OutboundProxyRuntime::new());
    outbound_proxy_runtime.attach_store(store.clone());
    build_app_with_store_ttl_usage_repos_import_store_admin_auth_and_sqlite_repo_and_proxy_runtime(
        store,
        auth_validate_cache_ttl_sec,
        usage_repo,
        usage_ingest_repo,
        import_job_store,
        admin_auth,
        sqlite_usage_repo,
        outbound_proxy_runtime,
    )
}

pub fn build_app_with_store_ttl_usage_repos_import_store_admin_auth_and_sqlite_repo_and_proxy_runtime(
    store: Arc<dyn ControlPlaneStore>,
    auth_validate_cache_ttl_sec: u64,
    usage_repo: Option<Arc<dyn UsageQueryRepository>>,
    usage_ingest_repo: Option<Arc<dyn UsageIngestRepository>>,
    import_job_store: Arc<dyn OAuthImportJobStore>,
    admin_auth: AdminAuthService,
    sqlite_usage_repo: Option<Arc<SqliteUsageRepo>>,
    outbound_proxy_runtime: Arc<crate::outbound_proxy_runtime::OutboundProxyRuntime>,
) -> Router {
    let import_job_manager = OAuthImportJobManager::new(
        store.clone(),
        import_job_store,
        import_job_concurrency_from_env(),
        import_job_claim_batch_size_from_env(),
    );
    import_job_manager.resume_recoverable_jobs();
    let runtime_config = build_runtime_config_from_env(auth_validate_cache_ttl_sec);
    let upstream_error_learning_base_url = runtime_config.data_plane_base_url.clone();
    let model_probe_interval_sec = parse_model_probe_interval_sec();
    let oauth_import_max_body_bytes = oauth_import_multipart_max_bytes();
    let codex_oauth_callback_listen_mode = codex_oauth_callback_listen_mode_from_env();
    let codex_oauth_callback_listen_addr = codex_oauth_callback_listen_addr_from_env();
    let tenant_auth_service = store.postgres_pool().map(|pool| {
        Arc::new(
            TenantAuthService::from_pool(pool)
                .expect("TENANT_JWT_SECRET (or ADMIN_JWT_SECRET fallback) must be set"),
        )
    });

    let edition_capabilities = system_capabilities_from_env();
    let state = AppState {
        store,
        usage_repo,
        usage_ingest_repo,
        tenant_auth_service,
        sqlite_usage_repo,
        auth_validate_cache_ttl_sec,
        system_capabilities: edition_capabilities.clone(),
        admin_auth,
        internal_auth_token: resolve_internal_auth_token()
            .expect("CONTROL_PLANE_INTERNAL_AUTH_TOKEN must be set"),
        import_job_manager,
        started_at: Utc::now(),
        runtime_config: Arc::new(std::sync::RwLock::new(runtime_config)),
        admin_logs: Arc::new(std::sync::RwLock::new(VecDeque::new())),
        model_catalog_last_error: Arc::new(std::sync::RwLock::new(None)),
        model_probe_cache: Arc::new(std::sync::RwLock::new(ModelProbeCache::default())),
        oauth_login_sessions: Arc::new(std::sync::RwLock::new(HashMap::new())),
        oauth_probe_sessions: Arc::new(std::sync::RwLock::new(HashMap::new())),
        codex_oauth_callback_listen_mode,
        codex_oauth_callback_listen_addr,
        codex_oauth_callback_listener: Arc::new(tokio::sync::Mutex::new(None)),
        outbound_proxy_runtime: outbound_proxy_runtime.clone(),
        upstream_error_learning_runtime: Arc::new(
            crate::upstream_error_learning::UpstreamErrorLearningRuntime::from_env_with_outbound_proxy_runtime(
                &upstream_error_learning_base_url,
                outbound_proxy_runtime,
            ),
        ),
        model_probe_interval_sec,
    };

    #[cfg(not(test))]
    spawn_model_probe_loop(state.clone());

    let app = Router::new()
        .route("/health", get(health))
        .route("/livez", get(livez))
        .route("/readyz", get(readyz))
        .route("/api/v1/system/capabilities", get(system_capabilities))
        .route("/api/v1/api-keys", post(create_api_key).get(list_api_keys))
        .route(
            "/api/v1/upstream-accounts",
            post(create_upstream_account).get(list_upstream_accounts),
        )
        .route(
            "/api/v1/upstream-accounts/{account_id}",
            patch(update_upstream_account_enabled).delete(delete_upstream_account),
        )
        .route(
            "/api/v1/upstream-accounts/batch-actions",
            post(batch_operate_upstream_accounts),
        )
        .route(
            "/api/v1/upstream-accounts/oauth/validate-refresh-token",
            post(validate_oauth_refresh_token),
        )
        .route(
            "/api/v1/upstream-accounts/oauth/import-refresh-token",
            post(import_oauth_refresh_token),
        )
        .route(
            "/api/v1/upstream-accounts/oauth/codex/login-sessions",
            post(create_codex_oauth_login_session),
        )
        .route(
            "/api/v1/upstream-accounts/oauth/codex/login-sessions/{session_id}",
            get(get_codex_oauth_login_session),
        )
        .route(
            "/api/v1/upstream-accounts/oauth/codex/login-sessions/{session_id}/callback",
            post(submit_codex_oauth_login_session_callback),
        )
        .route(
            "/api/v1/upstream-accounts/oauth/codex/probe-sessions",
            post(create_codex_oauth_probe_session),
        )
        .route(
            "/api/v1/upstream-accounts/oauth/codex/probe-sessions/{session_id}",
            get(get_codex_oauth_probe_session),
        )
        .route(
            "/api/v1/upstream-accounts/oauth/codex/probe-sessions/{session_id}/callback",
            post(submit_codex_oauth_probe_session_callback),
        )
        .route(
            "/api/v1/upstream-accounts/oauth/codex/callback",
            get(handle_codex_oauth_callback),
        )
        .route(
            "/api/v1/upstream-accounts/oauth/codex/probe/callback",
            get(handle_codex_oauth_probe_callback),
        )
        .route("/auth/callback", get(handle_codex_oauth_callback))
        .route(
            "/auth/probe-callback",
            get(handle_codex_oauth_probe_callback),
        )
        .route(
            "/api/v1/upstream-accounts/{account_id}/oauth/refresh",
            post(refresh_oauth_account),
        )
        .route(
            "/api/v1/upstream-accounts/{account_id}/oauth/refresh-jobs",
            post(create_oauth_refresh_job),
        )
        .route(
            "/api/v1/upstream-accounts/{account_id}/oauth/status",
            get(get_oauth_account_status),
        )
        .route(
            "/api/v1/upstream-accounts/oauth/statuses",
            post(get_oauth_account_statuses),
        )
        .route(
            "/api/v1/upstream-accounts/oauth/rate-limits/refresh-jobs",
            post(create_oauth_rate_limit_refresh_job),
        )
        .route(
            "/api/v1/upstream-accounts/oauth/rate-limits/refresh-jobs/{job_id}",
            get(get_oauth_rate_limit_refresh_job),
        )
        .route(
            "/api/v1/upstream-accounts/{account_id}/oauth/family/disable",
            post(disable_oauth_account_family),
        )
        .route(
            "/api/v1/upstream-accounts/{account_id}/oauth/family/enable",
            post(enable_oauth_account_family),
        )
        .route("/api/v1/admin/auth/login", post(admin_login))
        .route("/api/v1/admin/auth/logout", post(admin_logout))
        .route("/api/v1/admin/auth/me", get(admin_me))
        .route("/api/v1/admin/system/state", get(admin_system_state))
        .route(
            "/api/v1/admin/config",
            get(get_admin_runtime_config).put(update_admin_runtime_config),
        )
        .route("/api/v1/admin/logs", get(list_admin_logs))
        .route(
            "/api/v1/admin/proxies",
            get(list_admin_proxies).post(create_admin_outbound_proxy_node),
        )
        .route(
            "/api/v1/admin/proxies/settings",
            put(update_admin_outbound_proxy_settings),
        )
        .route(
            "/api/v1/admin/proxies/{proxy_id}",
            put(update_admin_outbound_proxy_node).delete(delete_admin_outbound_proxy_node),
        )
        .route("/api/v1/admin/proxies/test", post(test_admin_proxies))
        .route("/api/v1/admin/models", get(list_admin_models))
        .route(
            "/api/v1/admin/models/sync-openai",
            post(sync_openai_admin_models_catalog),
        )
        .route("/api/v1/admin/models/probe", post(probe_admin_models))
        .route(
            "/api/v1/admin/keys",
            get(list_admin_api_keys).post(create_admin_api_key),
        )
        .route(
            "/api/v1/admin/keys/{key_id}",
            patch(update_admin_api_key_enabled),
        )
        .route(
            "/api/v1/admin/usage/overview",
            get(get_admin_usage_overview),
        )
        .route("/api/v1/admin/usage/summary", get(get_admin_usage_summary))
        .route(
            "/api/v1/admin/usage/trends/hourly",
            get(get_admin_usage_hourly_trends),
        )
        .route("/api/v1/admin/request-logs", get(list_admin_request_logs))
        .route(
            "/api/v1/admin/request-correlation/{request_id}",
            get(get_admin_request_correlation),
        )
        .route("/api/v1/admin/audit-logs", get(list_admin_audit_logs))
        .route(
            "/api/v1/admin/model-pricing",
            get(list_admin_model_pricing).post(upsert_admin_model_pricing),
        )
        .route(
            "/api/v1/admin/model-pricing/{pricing_id}",
            delete(delete_admin_model_pricing),
        )
        .route(
            "/api/v1/admin/api-key-groups",
            get(list_admin_api_key_groups).post(upsert_admin_api_key_group),
        )
        .route(
            "/api/v1/admin/api-key-groups/{group_id}",
            delete(delete_admin_api_key_group),
        )
        .route(
            "/api/v1/admin/api-key-group-model-policies",
            post(upsert_admin_api_key_group_model_policy),
        )
        .route(
            "/api/v1/admin/api-key-group-model-policies/{policy_id}",
            delete(delete_admin_api_key_group_model_policy),
        )
        .route(
            "/api/v1/admin/model-routing/profiles",
            get(list_admin_routing_profiles).post(upsert_admin_routing_profile),
        )
        .route(
            "/api/v1/admin/model-routing/profiles/{profile_id}",
            delete(delete_admin_routing_profile),
        )
        .route(
            "/api/v1/admin/model-routing/model-policies",
            get(list_admin_model_routing_policies).post(upsert_admin_model_routing_policy),
        )
        .route(
            "/api/v1/admin/model-routing/model-policies/{policy_id}",
            delete(delete_admin_model_routing_policy),
        )
        .route(
            "/api/v1/admin/model-routing/settings",
            get(get_admin_model_routing_settings).put(update_admin_model_routing_settings),
        )
        .route(
            "/api/v1/admin/model-routing/error-learning/settings",
            get(get_admin_upstream_error_learning_settings)
                .put(update_admin_upstream_error_learning_settings),
        )
        .route(
            "/api/v1/admin/model-routing/upstream-errors",
            get(list_admin_upstream_error_templates),
        )
        .route(
            "/api/v1/admin/model-routing/builtin-error-templates",
            get(list_admin_builtin_error_templates),
        )
        .route(
            "/api/v1/admin/model-routing/upstream-errors/{template_id}",
            put(update_admin_upstream_error_template),
        )
        .route(
            "/api/v1/admin/model-routing/builtin-error-templates/{template_kind}/{template_code}",
            put(update_admin_builtin_error_template),
        )
        .route(
            "/api/v1/admin/model-routing/upstream-errors/{template_id}/approve",
            post(approve_admin_upstream_error_template),
        )
        .route(
            "/api/v1/admin/model-routing/upstream-errors/{template_id}/reject",
            post(reject_admin_upstream_error_template),
        )
        .route(
            "/api/v1/admin/model-routing/upstream-errors/{template_id}/rewrite",
            post(rewrite_admin_upstream_error_template),
        )
        .route(
            "/api/v1/admin/model-routing/builtin-error-templates/{template_kind}/{template_code}/rewrite",
            post(rewrite_admin_builtin_error_template),
        )
        .route(
            "/api/v1/admin/model-routing/builtin-error-templates/{template_kind}/{template_code}/reset",
            post(reset_admin_builtin_error_template),
        )
        .route(
            "/api/v1/admin/model-routing/versions",
            get(list_admin_routing_plan_versions),
        )
        .route(
            "/api/v1/upstream-accounts/oauth/import-jobs",
            post(create_oauth_import_job).layer(DefaultBodyLimit::max(oauth_import_max_body_bytes)),
        )
        .route(
            "/api/v1/upstream-accounts/oauth/import-jobs/{job_id}",
            get(get_oauth_import_job),
        )
        .route(
            "/api/v1/upstream-accounts/oauth/import-jobs/{job_id}/items",
            get(list_oauth_import_job_items),
        )
        .route(
            "/api/v1/upstream-accounts/oauth/import-jobs/{job_id}/retry-failed",
            post(retry_failed_oauth_import_items),
        )
        .route(
            "/api/v1/upstream-accounts/oauth/import-jobs/{job_id}/pause",
            post(pause_oauth_import_job),
        )
        .route(
            "/api/v1/upstream-accounts/oauth/import-jobs/{job_id}/resume",
            post(resume_oauth_import_job),
        )
        .route(
            "/api/v1/upstream-accounts/oauth/import-jobs/{job_id}/cancel",
            post(cancel_oauth_import_job),
        )
        .route("/api/v1/policies/routing", post(set_routing_policy))
        .route("/api/v1/policies/retry", post(set_retry_policy))
        .route(
            "/api/v1/policies/stream-retry",
            post(set_stream_retry_policy),
        )
        .route(
            "/internal/v1/upstream-accounts/{account_id}/oauth/refresh",
            post(internal_refresh_oauth_account),
        )
        .route(
            "/internal/v1/upstream-accounts/{account_id}/disable",
            post(internal_disable_upstream_account),
        )
        .route(
            "/internal/v1/upstream-accounts/{account_id}/health/seen-ok",
            post(internal_mark_upstream_account_seen_ok),
        )
        .route(
            "/internal/v1/upstream-accounts/{account_id}/models/seen-ok",
            post(internal_mark_upstream_model_seen_ok),
        )
        .route(
            "/internal/v1/upstream-errors/resolve",
            post(internal_resolve_upstream_error_template),
        )
        .route("/internal/v1/auth/validate", post(validate_api_key))
        .route(
            "/internal/v1/usage/request-logs",
            post(internal_ingest_request_log),
        )
        .route("/api/v1/data-plane/snapshot", get(data_plane_snapshot))
        .route(
            "/api/v1/data-plane/snapshot/events",
            get(data_plane_snapshot_events),
        )
        .route(
            "/api/v1/usage/hourly/accounts",
            get(list_hourly_account_usage),
        )
        .route(
            "/api/v1/usage/hourly/tenant-api-keys",
            get(list_hourly_tenant_api_key_usage),
        )
        .route("/api/v1/usage/trends/hourly", get(get_usage_hourly_trends))
        .route(
            "/api/v1/usage/trends/hourly/tenants",
            get(get_usage_hourly_tenant_trends),
        )
        .route("/api/v1/usage/summary", get(get_usage_summary))
        .route(
            "/api/v1/usage/leaderboard/tenants",
            get(get_tenant_usage_leaderboard),
        )
        .route(
            "/api/v1/usage/leaderboard/accounts",
            get(get_account_usage_leaderboard),
        )
        .route(
            "/api/v1/usage/leaderboard/api-keys",
            get(get_api_key_usage_leaderboard),
        )
        .route(
            "/api/v1/usage/leaderboard/overview",
            get(get_usage_leaderboard_overview),
        );
    let app = add_multi_tenant_routes(app, &edition_capabilities);
    let app = add_admin_tenant_credit_routes(app, &edition_capabilities);
    let app = add_tenant_portal_routes(app, &edition_capabilities);
    let app = add_internal_billing_routes(app, &edition_capabilities);

    app.layer(middleware::from_fn(request_id_middleware))
        .with_state(state)
}

include!("app/core_handlers.rs");
include!("app/tail_handlers.rs");
