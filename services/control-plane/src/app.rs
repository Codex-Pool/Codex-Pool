use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context};
use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::http::{HeaderMap, HeaderName, HeaderValue, Request};
use axum::middleware;
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::{delete, get, patch, post};
use axum::Json;
use axum::{response::IntoResponse, Router};
use chrono::{DateTime, Utc};
use codex_pool_core::api::{
    AccountUsageLeaderboardResponse, AdminLoginRequest, AdminMeResponse,
    ApiKeyUsageLeaderboardResponse, CreateApiKeyRequest, CreateTenantRequest,
    CreateUpstreamAccountRequest, ErrorEnvelope, HourlyAccountUsagePoint,
    HourlyTenantApiKeyUsagePoint, ImportOAuthRefreshTokenRequest, OAuthAccountStatusResponse,
    OAuthFamilyActionResponse, OAuthImportItemStatus, OAuthImportJobActionResponse,
    OAuthImportJobItemsResponse, OAuthImportJobSummary, OAuthRateLimitRefreshJobStatus,
    OAuthRateLimitRefreshJobSummary, PolicyResponse, TenantUsageLeaderboardResponse,
    UpsertRetryPolicyRequest, UpsertRoutingPolicyRequest, UpsertStreamRetryPolicyRequest,
    UsageHourlyTenantTrendsResponse, UsageHourlyTrendsResponse, UsageLeaderboardOverviewResponse,
    UsageQueryResponse, UsageSummaryQueryResponse, ValidateApiKeyRequest, ValidateApiKeyResponse,
    ValidateOAuthRefreshTokenRequest, ValidateOAuthRefreshTokenResponse,
};
use codex_pool_core::model::{ApiKey, UpstreamAccount};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::admin_auth::{AdminAuthService, AdminPrincipal};
use crate::import_jobs::{
    CreateOAuthImportJobOptions, ImportUploadFile, InMemoryOAuthImportJobStore,
    OAuthImportJobManager, OAuthImportJobStore,
};
use crate::store::{ControlPlaneStore, InMemoryStore};
use crate::tenant::TenantAuthService;
use crate::usage::clickhouse_repo::UsageQueryRepository;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AdminProxyItem {
    id: String,
    label: String,
    base_url: String,
    enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_test_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_error: Option<String>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
struct AdminProxyTestRequest {
    proxy_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct AdminProxyTestResponse {
    tested: usize,
    results: Vec<AdminProxyItem>,
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
            "/api/v1/upstream-accounts/oauth/codex/callback",
            get(handle_codex_oauth_callback),
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
    if has_pending_sessions {
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
    pub tenant_auth_service: Option<Arc<TenantAuthService>>,
    pub auth_validate_cache_ttl_sec: u64,
    pub admin_auth: AdminAuthService,
    pub internal_auth_token: Arc<str>,
    pub import_job_manager: OAuthImportJobManager,
    pub started_at: DateTime<Utc>,
    runtime_config: Arc<std::sync::RwLock<RuntimeConfigSnapshot>>,
    admin_logs: Arc<std::sync::RwLock<VecDeque<AdminLogEntry>>>,
    admin_proxies: Arc<std::sync::RwLock<Vec<AdminProxyItem>>>,
    model_catalog_last_error: Arc<std::sync::RwLock<Option<String>>>,
    model_probe_cache: Arc<std::sync::RwLock<ModelProbeCache>>,
    oauth_login_sessions: Arc<std::sync::RwLock<HashMap<String, CodexOAuthLoginSessionRecord>>>,
    codex_oauth_callback_listen_mode: CodexOAuthCallbackListenMode,
    codex_oauth_callback_listen_addr: Option<SocketAddr>,
    codex_oauth_callback_listener:
        Arc<tokio::sync::Mutex<Option<CodexOAuthCallbackListenerRuntime>>>,
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

fn build_admin_proxies_from_env(runtime_config: &RuntimeConfigSnapshot) -> Vec<AdminProxyItem> {
    let from_env = std::env::var("ADMIN_PROXIES_JSON")
        .ok()
        .and_then(|raw| serde_json::from_str::<Vec<AdminProxyItem>>(&raw).ok())
        .unwrap_or_default();
    if !from_env.is_empty() {
        return from_env;
    }

    vec![AdminProxyItem {
        id: "data-plane-default".to_string(),
        label: "Data Plane".to_string(),
        base_url: runtime_config.data_plane_base_url.clone(),
        enabled: true,
        last_test_status: None,
        last_latency_ms: None,
        last_error: None,
        updated_at: Utc::now(),
    }]
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
    use std::sync::{LazyLock, Mutex};

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    fn set_env(key: &str, value: Option<&str>) -> Option<String> {
        let previous = std::env::var(key).ok();
        match value {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
        previous
    }

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
    let import_job_manager = OAuthImportJobManager::new(
        store.clone(),
        import_job_store,
        import_job_concurrency_from_env(),
        import_job_claim_batch_size_from_env(),
    );
    import_job_manager.resume_recoverable_jobs();
    let runtime_config = build_runtime_config_from_env(auth_validate_cache_ttl_sec);
    let admin_proxies = build_admin_proxies_from_env(&runtime_config);
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

    let state = AppState {
        store,
        usage_repo,
        tenant_auth_service,
        auth_validate_cache_ttl_sec,
        admin_auth,
        internal_auth_token: resolve_internal_auth_token()
            .expect("CONTROL_PLANE_INTERNAL_AUTH_TOKEN must be set"),
        import_job_manager,
        started_at: Utc::now(),
        runtime_config: Arc::new(std::sync::RwLock::new(runtime_config)),
        admin_logs: Arc::new(std::sync::RwLock::new(VecDeque::new())),
        admin_proxies: Arc::new(std::sync::RwLock::new(admin_proxies)),
        model_catalog_last_error: Arc::new(std::sync::RwLock::new(None)),
        model_probe_cache: Arc::new(std::sync::RwLock::new(ModelProbeCache::default())),
        oauth_login_sessions: Arc::new(std::sync::RwLock::new(HashMap::new())),
        codex_oauth_callback_listen_mode,
        codex_oauth_callback_listen_addr,
        codex_oauth_callback_listener: Arc::new(tokio::sync::Mutex::new(None)),
        model_probe_interval_sec,
    };

    #[cfg(not(test))]
    spawn_model_probe_loop(state.clone());

    Router::new()
        .route("/health", get(health))
        .route("/livez", get(livez))
        .route("/readyz", get(readyz))
        .route("/api/v1/tenants", post(create_tenant).get(list_tenants))
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
            "/api/v1/upstream-accounts/oauth/codex/callback",
            get(handle_codex_oauth_callback),
        )
        .route("/auth/callback", get(handle_codex_oauth_callback))
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
        .route("/api/v1/admin/proxies", get(list_admin_proxies))
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
            "/api/v1/admin/impersonations",
            post(create_admin_impersonation),
        )
        .route(
            "/api/v1/admin/impersonations/{session_id}",
            delete(delete_admin_impersonation),
        )
        .route("/api/v1/tenant/auth/register", post(tenant_register))
        .route(
            "/api/v1/tenant/auth/verify-email",
            post(tenant_verify_email),
        )
        .route("/api/v1/tenant/auth/login", post(tenant_login))
        .route("/api/v1/tenant/auth/logout", post(tenant_logout))
        .route("/api/v1/tenant/auth/me", get(tenant_me))
        .route(
            "/api/v1/tenant/auth/password/forgot",
            post(tenant_forgot_password),
        )
        .route(
            "/api/v1/tenant/auth/password/reset",
            post(tenant_reset_password),
        )
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
        .route("/api/v1/tenant/audit-logs", get(list_tenant_audit_logs))
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
        .route("/internal/v1/auth/validate", post(validate_api_key))
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
            "/internal/v1/billing/pricing",
            post(internal_billing_pricing),
        )
        .route(
            "/internal/v1/billing/release",
            post(internal_billing_release),
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
        )
        .layer(middleware::from_fn(request_id_middleware))
        .with_state(state)
}

include!("app/core_handlers.rs");
include!("app/tail_handlers.rs");
