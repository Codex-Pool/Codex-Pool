use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use codex_pool_core::api::{
    DataPlaneSnapshot, DataPlaneSnapshotEventsResponse, UpdateClaudeCodeRoutingSettingsRequest,
};
use codex_pool_core::model::{
    default_builtin_error_templates, AccountRoutingTraits, AiErrorLearningSettings, ApiKey,
    BuiltinErrorTemplateKind, BuiltinErrorTemplateOverrideRecord, BuiltinErrorTemplateRecord,
    ClaudeCodeRoutingSettings, CompiledModelRoutingPolicy, CompiledRoutingPlan,
    CompiledRoutingProfile, LocalizedErrorTemplates, ModelRoutingPolicy, ModelRoutingSettings,
    ModelRoutingTriggerMode, OutboundProxyNode, OutboundProxyPoolSettings, RoutingPlanVersion,
    RoutingPolicy, RoutingProfile, RoutingStrategy, Tenant, UpstreamAccount, UpstreamAuthProvider,
    UpstreamErrorTemplateRecord, UpstreamErrorTemplateStatus, UpstreamMode,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::contracts::{
    AccountHealthFreshness, AccountPoolOperatorState, AccountPoolReasonClass, AccountPoolRecord,
    AccountPoolRecordScope, AccountPoolSummaryResponse, AccountProbeOutcome, CreateApiKeyRequest,
    CreateApiKeyResponse, CreateOutboundProxyNodeRequest, CreateTenantRequest,
    CreateUpstreamAccountRequest, ImportOAuthRefreshTokenRequest, OAuthAccountPoolState,
    OAuthAccountStatusResponse, OAuthFamilyActionResponse, OAuthHealthSignalsSummaryResponse,
    OAuthInventoryFailureStage, OAuthInventoryRecord, OAuthInventorySummaryResponse,
    OAuthLiveResultSource, OAuthLiveResultStatus, OAuthRateLimitRefreshErrorSummary,
    OAuthRateLimitRefreshJobStatus, OAuthRateLimitRefreshJobSummary, OAuthRateLimitSnapshot,
    OAuthRefreshStatus, OAuthRuntimePoolSummaryResponse, OAuthVaultRecordStatus,
    RefreshCredentialState, SessionCredentialKind, UpdateAiErrorLearningSettingsRequest,
    UpdateModelRoutingSettingsRequest, UpdateOutboundProxyNodeRequest,
    UpdateOutboundProxyPoolSettingsRequest, UpsertModelRoutingPolicyRequest,
    UpsertRetryPolicyRequest, UpsertRoutingPolicyRequest, UpsertRoutingProfileRequest,
    UpsertStreamRetryPolicyRequest, ValidateOAuthRefreshTokenRequest,
    ValidateOAuthRefreshTokenResponse,
};
use crate::crypto::CredentialCipher;
use crate::oauth::{OAuthTokenClient, OAuthTokenInfo, OpenAiOAuthClient};

#[cfg(feature = "postgres-backend")]
pub use sqlx_postgres::PgPool;
#[cfg(not(feature = "postgres-backend"))]
#[derive(Debug, Clone)]
pub struct PgPool;

#[cfg(feature = "postgres-backend")]
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
const OAUTH_VAULT_ACTIVATE_BATCH_SIZE_ENV: &str = "CONTROL_PLANE_VAULT_ACTIVATE_BATCH_SIZE";
const DEFAULT_OAUTH_VAULT_ACTIVATE_BATCH_SIZE: usize = 200;
const MIN_OAUTH_VAULT_ACTIVATE_BATCH_SIZE: usize = 1;
const MAX_OAUTH_VAULT_ACTIVATE_BATCH_SIZE: usize = 2_000;
const OAUTH_VAULT_ACTIVATE_CONCURRENCY_ENV: &str = "CONTROL_PLANE_VAULT_ACTIVATE_CONCURRENCY";
const DEFAULT_OAUTH_VAULT_ACTIVATE_CONCURRENCY: usize = 8;
const MIN_OAUTH_VAULT_ACTIVATE_CONCURRENCY: usize = 1;
const OAUTH_VAULT_TERMINAL_AUTH_RETRY_LIMIT: u32 = 3;
const MAX_OAUTH_VAULT_ACTIVATE_CONCURRENCY: usize = 64;
const OAUTH_VAULT_ACTIVATE_MAX_RPS_ENV: &str = "CONTROL_PLANE_VAULT_ACTIVATE_MAX_RPS";
const DEFAULT_OAUTH_VAULT_ACTIVATE_MAX_RPS: u32 = 1;
const MIN_OAUTH_VAULT_ACTIVATE_MAX_RPS: u32 = 1;
const MAX_OAUTH_VAULT_ACTIVATE_MAX_RPS: u32 = 100;
const ACTIVE_POOL_TARGET_ENV: &str = "CONTROL_PLANE_ACTIVE_POOL_TARGET";
const DEFAULT_ACTIVE_POOL_TARGET: usize = 5_000;
const MIN_ACTIVE_POOL_TARGET: usize = 1;
const MAX_ACTIVE_POOL_TARGET: usize = 100_000;
const ACTIVE_POOL_MIN_ENV: &str = "CONTROL_PLANE_ACTIVE_POOL_MIN";
const DEFAULT_ACTIVE_POOL_MIN: usize = 4_500;
const MIN_ACTIVE_POOL_MIN: usize = 1;
const MAX_ACTIVE_POOL_MIN: usize = 100_000;
const RUNTIME_POOL_CAP_ENV: &str = "CONTROL_PLANE_RUNTIME_POOL_CAP";
const DEFAULT_RUNTIME_POOL_CAP: usize = 100_000;
const MIN_RUNTIME_POOL_CAP: usize = 1;
const MAX_RUNTIME_POOL_CAP: usize = 100_000;
const PENDING_PURGE_DELAY_SEC_ENV: &str = "CONTROL_PLANE_PENDING_PURGE_DELAY_SEC";
const DEFAULT_PENDING_PURGE_DELAY_SEC: i64 = 300;
const MIN_PENDING_PURGE_DELAY_SEC: i64 = 5;
const MAX_PENDING_PURGE_DELAY_SEC: i64 = 7 * 24 * 60 * 60;
const PENDING_PURGE_BATCH_SIZE_ENV: &str = "CONTROL_PLANE_PENDING_PURGE_BATCH_SIZE";
const DEFAULT_PENDING_PURGE_BATCH_SIZE: usize = 200;
const MIN_PENDING_PURGE_BATCH_SIZE: usize = 1;
const MAX_PENDING_PURGE_BATCH_SIZE: usize = 5_000;
const TOKEN_INVALIDATED_PURGE_WINDOW_SEC_ENV: &str =
    "CONTROL_PLANE_TOKEN_INVALIDATED_PURGE_WINDOW_SEC";
const DEFAULT_TOKEN_INVALIDATED_PURGE_WINDOW_SEC: i64 = 3_600;
const MIN_TOKEN_INVALIDATED_PURGE_WINDOW_SEC: i64 = 60;
const MAX_TOKEN_INVALIDATED_PURGE_WINDOW_SEC: i64 = 7 * 24 * 60 * 60;
const TOKEN_INVALIDATED_LEGACY_PURGE_THRESHOLD_ENV: &str =
    "CONTROL_PLANE_TOKEN_INVALIDATED_LEGACY_PURGE_THRESHOLD";
const DEFAULT_TOKEN_INVALIDATED_LEGACY_PURGE_THRESHOLD: u32 = 2;
const MIN_TOKEN_INVALIDATED_PURGE_THRESHOLD: u32 = 1;
const MAX_TOKEN_INVALIDATED_PURGE_THRESHOLD: u32 = 10;
const TOKEN_INVALIDATED_OAUTH_PURGE_THRESHOLD_ENV: &str =
    "CONTROL_PLANE_TOKEN_INVALIDATED_OAUTH_PURGE_THRESHOLD";
const DEFAULT_TOKEN_INVALIDATED_OAUTH_PURGE_THRESHOLD: u32 = 3;
const RATE_LIMIT_CACHE_TTL_SEC_ENV: &str = "CONTROL_PLANE_RATE_LIMIT_CACHE_TTL_SEC";
const DEFAULT_RATE_LIMIT_CACHE_TTL_SEC: i64 = 180;
const MIN_RATE_LIMIT_CACHE_TTL_SEC: i64 = 30;
const MAX_RATE_LIMIT_CACHE_TTL_SEC: i64 = 86_400;
const RATE_LIMIT_REFRESH_BATCH_SIZE_ENV: &str = "CONTROL_PLANE_RATE_LIMIT_REFRESH_BATCH_SIZE";
const DEFAULT_RATE_LIMIT_REFRESH_BATCH_SIZE: usize = 200;
const MIN_RATE_LIMIT_REFRESH_BATCH_SIZE: usize = 1;
const MAX_RATE_LIMIT_REFRESH_BATCH_SIZE: usize = 2_000;
const RATE_LIMIT_REFRESH_CONCURRENCY_ENV: &str = "CONTROL_PLANE_RATE_LIMIT_REFRESH_CONCURRENCY";
const DEFAULT_RATE_LIMIT_REFRESH_CONCURRENCY: usize = 8;
const MIN_RATE_LIMIT_REFRESH_CONCURRENCY: usize = 1;
const MAX_RATE_LIMIT_REFRESH_CONCURRENCY: usize = 64;
const RATE_LIMIT_REFRESH_ERROR_BACKOFF_SEC_ENV: &str =
    "CONTROL_PLANE_RATE_LIMIT_REFRESH_ERROR_BACKOFF_SEC";
const DEFAULT_RATE_LIMIT_REFRESH_ERROR_BACKOFF_SEC: i64 = 60;
const MIN_RATE_LIMIT_REFRESH_ERROR_BACKOFF_SEC: i64 = 5;
const MAX_RATE_LIMIT_REFRESH_ERROR_BACKOFF_SEC: i64 = 3_600;
const ACTIVE_PATROL_BATCH_SIZE_ENV: &str = "CONTROL_PLANE_ACTIVE_PATROL_BATCH_SIZE";
const DEFAULT_ACTIVE_PATROL_BATCH_SIZE: usize = 5;
const MIN_ACTIVE_PATROL_BATCH_SIZE: usize = 1;
const MAX_ACTIVE_PATROL_BATCH_SIZE: usize = 100;
const ACCOUNT_HEALTH_FRESHNESS_TTL_SEC: i64 = 15 * 60;
const PRIMARY_RATE_LIMIT_WINDOW_MINUTES: i64 = 300;
const SECONDARY_RATE_LIMIT_WINDOW_MINUTES: i64 = 10_080;
const DEFAULT_CODEX_ACCOUNT_BASE_URL: &str = "https://chatgpt.com/backend-api/codex";
const CODEX_ACCOUNT_BASE_PATH: &str = "/backend-api/codex";

fn merge_localized_error_templates(
    base: &LocalizedErrorTemplates,
    override_templates: &LocalizedErrorTemplates,
) -> LocalizedErrorTemplates {
    LocalizedErrorTemplates {
        en: override_templates.en.clone().or_else(|| base.en.clone()),
        zh_cn: override_templates
            .zh_cn
            .clone()
            .or_else(|| base.zh_cn.clone()),
        zh_tw: override_templates
            .zh_tw
            .clone()
            .or_else(|| base.zh_tw.clone()),
        ja: override_templates.ja.clone().or_else(|| base.ja.clone()),
        ru: override_templates.ru.clone().or_else(|| base.ru.clone()),
    }
}

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

fn oauth_vault_activate_batch_size_from_env() -> usize {
    std::env::var(OAUTH_VAULT_ACTIVATE_BATCH_SIZE_ENV)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(DEFAULT_OAUTH_VAULT_ACTIVATE_BATCH_SIZE)
        .clamp(
            MIN_OAUTH_VAULT_ACTIVATE_BATCH_SIZE,
            MAX_OAUTH_VAULT_ACTIVATE_BATCH_SIZE,
        )
}

fn oauth_vault_activate_concurrency_from_env() -> usize {
    std::env::var(OAUTH_VAULT_ACTIVATE_CONCURRENCY_ENV)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(DEFAULT_OAUTH_VAULT_ACTIVATE_CONCURRENCY)
        .clamp(
            MIN_OAUTH_VAULT_ACTIVATE_CONCURRENCY,
            MAX_OAUTH_VAULT_ACTIVATE_CONCURRENCY,
        )
}

fn oauth_vault_activate_max_rps_from_env() -> u32 {
    std::env::var(OAUTH_VAULT_ACTIVATE_MAX_RPS_ENV)
        .ok()
        .and_then(|raw| raw.parse::<u32>().ok())
        .unwrap_or(DEFAULT_OAUTH_VAULT_ACTIVATE_MAX_RPS)
        .clamp(
            MIN_OAUTH_VAULT_ACTIVATE_MAX_RPS,
            MAX_OAUTH_VAULT_ACTIVATE_MAX_RPS,
        )
}

fn active_pool_target_from_env() -> usize {
    std::env::var(ACTIVE_POOL_TARGET_ENV)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(DEFAULT_ACTIVE_POOL_TARGET)
        .clamp(MIN_ACTIVE_POOL_TARGET, MAX_ACTIVE_POOL_TARGET)
}

fn active_pool_min_from_env() -> usize {
    std::env::var(ACTIVE_POOL_MIN_ENV)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(DEFAULT_ACTIVE_POOL_MIN)
        .clamp(MIN_ACTIVE_POOL_MIN, MAX_ACTIVE_POOL_MIN)
}

fn runtime_pool_cap_from_env() -> usize {
    std::env::var(RUNTIME_POOL_CAP_ENV)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(DEFAULT_RUNTIME_POOL_CAP)
        .clamp(MIN_RUNTIME_POOL_CAP, MAX_RUNTIME_POOL_CAP)
}

fn pending_purge_delay_sec_from_env() -> i64 {
    std::env::var(PENDING_PURGE_DELAY_SEC_ENV)
        .ok()
        .and_then(|raw| raw.parse::<i64>().ok())
        .unwrap_or(DEFAULT_PENDING_PURGE_DELAY_SEC)
        .clamp(MIN_PENDING_PURGE_DELAY_SEC, MAX_PENDING_PURGE_DELAY_SEC)
}

fn pending_purge_batch_size_from_env() -> usize {
    std::env::var(PENDING_PURGE_BATCH_SIZE_ENV)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(DEFAULT_PENDING_PURGE_BATCH_SIZE)
        .clamp(MIN_PENDING_PURGE_BATCH_SIZE, MAX_PENDING_PURGE_BATCH_SIZE)
}

fn token_invalidated_purge_window_sec_from_env() -> i64 {
    std::env::var(TOKEN_INVALIDATED_PURGE_WINDOW_SEC_ENV)
        .ok()
        .and_then(|raw| raw.parse::<i64>().ok())
        .unwrap_or(DEFAULT_TOKEN_INVALIDATED_PURGE_WINDOW_SEC)
        .clamp(
            MIN_TOKEN_INVALIDATED_PURGE_WINDOW_SEC,
            MAX_TOKEN_INVALIDATED_PURGE_WINDOW_SEC,
        )
}

fn token_invalidated_purge_threshold_for_provider(provider: UpstreamAuthProvider) -> u32 {
    let env_name = match provider {
        UpstreamAuthProvider::OAuthRefreshToken => TOKEN_INVALIDATED_OAUTH_PURGE_THRESHOLD_ENV,
        UpstreamAuthProvider::LegacyBearer => TOKEN_INVALIDATED_LEGACY_PURGE_THRESHOLD_ENV,
    };
    let default = match provider {
        UpstreamAuthProvider::OAuthRefreshToken => DEFAULT_TOKEN_INVALIDATED_OAUTH_PURGE_THRESHOLD,
        UpstreamAuthProvider::LegacyBearer => DEFAULT_TOKEN_INVALIDATED_LEGACY_PURGE_THRESHOLD,
    };

    std::env::var(env_name)
        .ok()
        .and_then(|raw| raw.parse::<u32>().ok())
        .unwrap_or(default)
        .clamp(
            MIN_TOKEN_INVALIDATED_PURGE_THRESHOLD,
            MAX_TOKEN_INVALIDATED_PURGE_THRESHOLD,
        )
}

fn rate_limit_cache_ttl_sec_from_env() -> i64 {
    std::env::var(RATE_LIMIT_CACHE_TTL_SEC_ENV)
        .ok()
        .and_then(|raw| raw.parse::<i64>().ok())
        .unwrap_or(DEFAULT_RATE_LIMIT_CACHE_TTL_SEC)
        .clamp(MIN_RATE_LIMIT_CACHE_TTL_SEC, MAX_RATE_LIMIT_CACHE_TTL_SEC)
}

fn rate_limit_refresh_batch_size_from_env() -> usize {
    std::env::var(RATE_LIMIT_REFRESH_BATCH_SIZE_ENV)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(DEFAULT_RATE_LIMIT_REFRESH_BATCH_SIZE)
        .clamp(
            MIN_RATE_LIMIT_REFRESH_BATCH_SIZE,
            MAX_RATE_LIMIT_REFRESH_BATCH_SIZE,
        )
}

fn rate_limit_refresh_concurrency_from_env() -> usize {
    std::env::var(RATE_LIMIT_REFRESH_CONCURRENCY_ENV)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(DEFAULT_RATE_LIMIT_REFRESH_CONCURRENCY)
        .clamp(
            MIN_RATE_LIMIT_REFRESH_CONCURRENCY,
            MAX_RATE_LIMIT_REFRESH_CONCURRENCY,
        )
}

fn rate_limit_refresh_error_backoff_sec_from_env() -> i64 {
    std::env::var(RATE_LIMIT_REFRESH_ERROR_BACKOFF_SEC_ENV)
        .ok()
        .and_then(|raw| raw.parse::<i64>().ok())
        .unwrap_or(DEFAULT_RATE_LIMIT_REFRESH_ERROR_BACKOFF_SEC)
        .clamp(
            MIN_RATE_LIMIT_REFRESH_ERROR_BACKOFF_SEC,
            MAX_RATE_LIMIT_REFRESH_ERROR_BACKOFF_SEC,
        )
}

fn active_patrol_batch_size_from_env() -> usize {
    std::env::var(ACTIVE_PATROL_BATCH_SIZE_ENV)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(DEFAULT_ACTIVE_PATROL_BATCH_SIZE)
        .clamp(MIN_ACTIVE_PATROL_BATCH_SIZE, MAX_ACTIVE_PATROL_BATCH_SIZE)
}

fn normalize_health_error_code(raw: &str) -> String {
    raw.trim().to_ascii_lowercase()
}

fn is_quota_error_signal(error_code: &str, error_message: &str) -> bool {
    let code = normalize_health_error_code(error_code);
    if matches!(
        code.as_str(),
        "quota_exhausted"
            | "usage_limit"
            | "insufficient_quota"
            | "quota_exceeded"
            | "billing_hard_limit_reached"
    ) {
        return true;
    }

    let message = error_message.to_ascii_lowercase();
    message.contains("usage limit")
        || message.contains("insufficient quota")
        || message.contains("quota exceeded")
        || message.contains("billing hard limit")
        || message.contains("start a free trial of plus")
}

fn trim_base_url(raw: &str) -> String {
    raw.trim().trim_end_matches('/').to_string()
}

fn normalize_upstream_account_base_url(mode: &UpstreamMode, raw: &str) -> String {
    let trimmed = trim_base_url(raw);
    if !matches!(
        mode,
        UpstreamMode::ChatGptSession | UpstreamMode::CodexOauth
    ) {
        return trimmed;
    }

    if trimmed.is_empty() {
        return DEFAULT_CODEX_ACCOUNT_BASE_URL.to_string();
    }

    let Ok(mut parsed) = reqwest::Url::parse(&trimmed) else {
        return trimmed;
    };
    parsed.set_query(None);
    parsed.set_fragment(None);

    let current_path = parsed.path().trim_end_matches('/');
    let normalized_path = if current_path.is_empty() || current_path == "/" || current_path == "/v1"
    {
        CODEX_ACCOUNT_BASE_PATH.to_string()
    } else if current_path.ends_with(CODEX_ACCOUNT_BASE_PATH) {
        current_path.to_string()
    } else if current_path.ends_with("/v1") {
        format!(
            "{}{}",
            current_path.trim_end_matches("/v1"),
            CODEX_ACCOUNT_BASE_PATH
        )
    } else {
        format!("{current_path}{CODEX_ACCOUNT_BASE_PATH}")
    };
    parsed.set_path(&normalized_path);
    trim_base_url(parsed.as_str())
}

fn is_auth_error_signal(error_code: &str, error_message: &str) -> bool {
    let code = normalize_health_error_code(error_code);
    if matches!(
        code.as_str(),
        "auth_expired"
            | "token_invalidated"
            | "invalid_refresh_token"
            | "refresh_token_reused"
            | "refresh_token_revoked"
            | "missing_client_id"
            | "unauthorized_client"
    ) {
        return true;
    }

    let message = error_message.to_ascii_lowercase();
    message.contains("access token could not be refreshed")
        || message.contains("logged out")
        || message.contains("signed in to another account")
        || message.contains("invalid refresh token")
}

fn normalized_auth_failure_reason_code(
    auth_provider: &UpstreamAuthProvider,
    error_code: &str,
    error_message: &str,
) -> String {
    let normalized = normalize_health_error_code(error_code);
    if normalized == "token_invalidated" {
        return normalized;
    }

    if matches!(auth_provider, UpstreamAuthProvider::LegacyBearer)
        && is_auth_error_signal(error_code, error_message)
    {
        return "token_invalidated".to_string();
    }

    if normalized.is_empty() {
        return "invalid_refresh_token".to_string();
    }

    normalized
}

fn is_rate_limited_signal(error_code: &str, error_message: &str) -> bool {
    let code = normalize_health_error_code(error_code);
    if matches!(code.as_str(), "rate_limited") {
        return true;
    }

    let message = error_message.to_ascii_lowercase();
    message.contains("rate limit") || message.contains("too many requests")
}

fn is_transient_upstream_error_signal(error_code: &str, error_message: &str) -> bool {
    let code = normalize_health_error_code(error_code);
    if matches!(
        code.as_str(),
        "upstream_unavailable" | "overloaded" | "timeout"
    ) {
        return true;
    }

    let message = error_message.to_ascii_lowercase();
    message.contains("service unavailable")
        || message.contains("temporarily unavailable")
        || message.contains("upstream unavailable")
        || message.contains("timed out")
        || message.contains("timeout")
        || message.contains("connection reset")
}

fn is_fatal_refresh_error_code(error_code: Option<&str>) -> bool {
    let Some(error_code) = error_code else {
        return false;
    };

    matches!(
        normalize_health_error_code(error_code).as_str(),
        "refresh_token_reused"
            | "refresh_token_revoked"
            | "invalid_refresh_token"
            | "missing_client_id"
            | "unauthorized_client"
    )
}

fn has_refresh_credential(auth_provider: &UpstreamAuthProvider) -> bool {
    matches!(auth_provider, UpstreamAuthProvider::OAuthRefreshToken)
}

fn refresh_credential_state(
    auth_provider: &UpstreamAuthProvider,
    last_refresh_status: &OAuthRefreshStatus,
    refresh_reused_detected: bool,
    last_refresh_error_code: Option<&str>,
) -> Option<RefreshCredentialState> {
    if !has_refresh_credential(auth_provider) {
        return None;
    }

    if refresh_reused_detected || is_fatal_refresh_error_code(last_refresh_error_code) {
        return Some(RefreshCredentialState::TerminalInvalid);
    }

    if matches!(last_refresh_status, OAuthRefreshStatus::Failed) {
        return Some(RefreshCredentialState::TransientFailed);
    }

    Some(RefreshCredentialState::Healthy)
}

fn is_blocking_rate_limit_error(
    rate_limits_last_error_code: Option<&str>,
    rate_limits_last_error: Option<&str>,
) -> bool {
    let Some(error_code) = rate_limits_last_error_code else {
        return false;
    };
    let error_message = rate_limits_last_error.unwrap_or_default();
    is_quota_error_signal(error_code, error_message)
        || is_auth_error_signal(error_code, error_message)
        || matches!(
            normalize_health_error_code(error_code).as_str(),
            "primary_window_exhausted" | "secondary_window_exhausted"
        )
}

fn has_active_rate_limit_block(
    now: DateTime<Utc>,
    rate_limits_expires_at: Option<DateTime<Utc>>,
    rate_limits_last_error_code: Option<&str>,
    rate_limits_last_error: Option<&str>,
) -> bool {
    rate_limits_expires_at.is_some_and(|expires_at| expires_at > now)
        && is_blocking_rate_limit_error(rate_limits_last_error_code, rate_limits_last_error)
}

fn rate_limit_failure_backoff_seconds(error_code: &str, error_message: &str) -> i64 {
    if is_quota_error_signal(error_code, error_message) {
        return 6 * 60 * 60;
    }
    if is_auth_error_signal(error_code, error_message) {
        return 30 * 60;
    }
    if is_rate_limited_signal(error_code, error_message) {
        return 120;
    }

    rate_limit_refresh_error_backoff_sec_from_env()
}

fn admission_probe_retry_after(
    checked_at: DateTime<Utc>,
    error_code: &str,
    error_message: &str,
) -> Option<DateTime<Utc>> {
    if is_quota_error_signal(error_code, error_message)
        || is_rate_limited_signal(error_code, error_message)
    {
        return Some(
            checked_at
                + Duration::seconds(rate_limit_failure_backoff_seconds(
                    error_code,
                    error_message,
                )),
        );
    }
    if is_transient_upstream_error_signal(error_code, error_message) {
        return Some(checked_at + Duration::minutes(5));
    }
    None
}

fn admission_probe_retry_after_with_budget(
    checked_at: DateTime<Utc>,
    error_code: &str,
    error_message: &str,
    transient_retry_count: u32,
) -> Option<DateTime<Utc>> {
    if is_quota_error_signal(error_code, error_message)
        || is_rate_limited_signal(error_code, error_message)
    {
        return admission_probe_retry_after(checked_at, error_code, error_message);
    }

    if !is_transient_upstream_error_signal(error_code, error_message) {
        return None;
    }

    let delay = match transient_retry_count {
        0 => Duration::minutes(5),
        1 => Duration::minutes(30),
        _ => Duration::hours(6),
    };
    Some(checked_at + delay)
}

fn can_retry_transient_admission_failure(transient_retry_count: u32) -> bool {
    transient_retry_count < OAUTH_VAULT_TERMINAL_AUTH_RETRY_LIMIT
}

fn can_retry_fatal_activation_failure(failure_count: u32) -> bool {
    failure_count < OAUTH_VAULT_TERMINAL_AUTH_RETRY_LIMIT
}

struct SeenOkRateLimitRefreshContext<'a> {
    token_expires_at: Option<DateTime<Utc>>,
    last_refresh_status: &'a OAuthRefreshStatus,
    refresh_reused_detected: bool,
    last_refresh_error_code: Option<&'a str>,
    rate_limits_expires_at: Option<DateTime<Utc>>,
    rate_limits_last_error_code: Option<&'a str>,
    rate_limits_last_error: Option<&'a str>,
}

fn should_refresh_rate_limit_cache_on_seen_ok(
    now: DateTime<Utc>,
    ctx: SeenOkRateLimitRefreshContext<'_>,
) -> bool {
    if !ctx
        .token_expires_at
        .is_some_and(|expires_at| expires_at > now + Duration::seconds(OAUTH_MIN_VALID_SEC))
    {
        return false;
    }
    if ctx.refresh_reused_detected {
        return false;
    }
    if matches!(ctx.last_refresh_status, OAuthRefreshStatus::Failed)
        && is_fatal_refresh_error_code(ctx.last_refresh_error_code)
    {
        return false;
    }

    if has_active_rate_limit_block(
        now,
        ctx.rate_limits_expires_at,
        ctx.rate_limits_last_error_code,
        ctx.rate_limits_last_error,
    ) {
        return true;
    }

    true
}

fn has_usable_access_token_fallback(
    has_access_token_fallback: bool,
    fallback_token_expires_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> bool {
    has_access_token_fallback
        && fallback_token_expires_at
            .map(|expires_at| expires_at > now + Duration::seconds(OAUTH_MIN_VALID_SEC))
            .unwrap_or(true)
}

fn refresh_credential_is_terminal_invalid(
    last_refresh_status: &OAuthRefreshStatus,
    refresh_reused_detected: bool,
    last_refresh_error_code: Option<&str>,
) -> bool {
    refresh_reused_detected
        || (matches!(last_refresh_status, OAuthRefreshStatus::Failed)
            && is_fatal_refresh_error_code(last_refresh_error_code))
}

fn should_use_access_token_fallback_for_runtime(
    token_expires_at: Option<DateTime<Utc>>,
    has_access_token_fallback: bool,
    fallback_token_expires_at: Option<DateTime<Utc>>,
    last_refresh_status: &OAuthRefreshStatus,
    refresh_reused_detected: bool,
    last_refresh_error_code: Option<&str>,
    now: DateTime<Utc>,
) -> bool {
    has_usable_access_token_fallback(has_access_token_fallback, fallback_token_expires_at, now)
        && refresh_credential_is_terminal_invalid(
            last_refresh_status,
            refresh_reused_detected,
            last_refresh_error_code,
        )
        && !token_expires_at
            .is_some_and(|expires_at| expires_at > now + Duration::seconds(OAUTH_MIN_VALID_SEC))
}

#[allow(clippy::too_many_arguments)]
fn oauth_effective_enabled(
    enabled: bool,
    auth_provider: &UpstreamAuthProvider,
    credential_kind: Option<&SessionCredentialKind>,
    token_expires_at: Option<DateTime<Utc>>,
    has_access_token_fallback: bool,
    fallback_token_expires_at: Option<DateTime<Utc>>,
    last_refresh_status: &OAuthRefreshStatus,
    refresh_reused_detected: bool,
    last_refresh_error_code: Option<&str>,
    rate_limits_expires_at: Option<DateTime<Utc>>,
    rate_limits_last_error_code: Option<&str>,
    rate_limits_last_error: Option<&str>,
    now: DateTime<Utc>,
) -> bool {
    let fallback_usable =
        has_usable_access_token_fallback(has_access_token_fallback, fallback_token_expires_at, now);
    let base_enabled = match (auth_provider, credential_kind) {
        (UpstreamAuthProvider::OAuthRefreshToken, _) => {
            enabled
                && (token_expires_at.is_some_and(|expires_at| {
                    expires_at > now + Duration::seconds(OAUTH_MIN_VALID_SEC)
                }) || fallback_usable)
        }
        (_, Some(SessionCredentialKind::OneTimeAccessToken)) => {
            enabled
                && token_expires_at
                    .map(|expires_at| expires_at > now + Duration::seconds(OAUTH_MIN_VALID_SEC))
                    .unwrap_or(true)
        }
        _ => enabled,
    };
    if !base_enabled {
        return false;
    }

    if matches!(auth_provider, UpstreamAuthProvider::OAuthRefreshToken) {
        if refresh_credential_is_terminal_invalid(
            last_refresh_status,
            refresh_reused_detected,
            last_refresh_error_code,
        ) && !fallback_usable
        {
            return false;
        }
        if has_active_rate_limit_block(
            now,
            rate_limits_expires_at,
            rate_limits_last_error_code,
            rate_limits_last_error,
        ) {
            return false;
        }
    }

    true
}

fn derive_rate_limit_block(
    snapshots: &[OAuthRateLimitSnapshot],
    now: DateTime<Utc>,
) -> (Option<DateTime<Utc>>, Option<String>) {
    if let Some(blocked_until) = find_blocked_until_for_window(
        snapshots,
        true,
        Some(SECONDARY_RATE_LIMIT_WINDOW_MINUTES),
        now,
    ) {
        return (
            Some(blocked_until),
            Some("secondary_window_exhausted".to_string()),
        );
    }
    if let Some(blocked_until) = find_blocked_until_for_window(
        snapshots,
        false,
        Some(PRIMARY_RATE_LIMIT_WINDOW_MINUTES),
        now,
    ) {
        return (
            Some(blocked_until),
            Some("primary_window_exhausted".to_string()),
        );
    }

    (None, None)
}

fn rate_limit_block_message(block_reason: &str) -> String {
    match normalize_health_error_code(block_reason).as_str() {
        "secondary_window_exhausted" => {
            "secondary rate limit window is exhausted until reset".to_string()
        }
        "primary_window_exhausted" => {
            "primary rate limit window is exhausted until reset".to_string()
        }
        _ => "rate limit window is exhausted until reset".to_string(),
    }
}

fn find_blocked_until_for_window(
    snapshots: &[OAuthRateLimitSnapshot],
    secondary: bool,
    window_minutes: Option<i64>,
    now: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    snapshots
        .iter()
        .filter_map(|snapshot| {
            let window = if secondary {
                snapshot.secondary.as_ref()
            } else {
                snapshot.primary.as_ref()
            }?;
            if window.used_percent < 100.0 {
                return None;
            }
            if let Some(expected_minutes) = window_minutes {
                if let Some(actual_minutes) = window.window_minutes {
                    if actual_minutes != expected_minutes {
                        return None;
                    }
                }
            }
            let resets_at = window.resets_at?;
            (resets_at > now).then_some(resets_at)
        })
        .max()
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

pub struct RuntimeStorePorts {
    pub control_plane: Arc<dyn ControlPlaneStore>,
    pub snapshot_policy: Arc<dyn SnapshotPolicyStore>,
    pub tenant_catalog: Arc<dyn TenantCatalogStore>,
    pub oauth_runtime: Arc<dyn OAuthRuntimeStore>,
    pub edition_migration: Arc<dyn EditionMigrationStore>,
}

#[async_trait]
pub trait ControlPlaneStore: Send + Sync {
    async fn create_tenant(&self, req: CreateTenantRequest) -> Result<Tenant>;
    async fn list_tenants(&self) -> Result<Vec<Tenant>>;
    async fn create_api_key(&self, req: CreateApiKeyRequest) -> Result<CreateApiKeyResponse>;
    async fn list_api_keys(&self) -> Result<Vec<ApiKey>>;
    async fn set_api_key_enabled(&self, _api_key_id: Uuid, _enabled: bool) -> Result<ApiKey> {
        Err(anyhow!("api key update is not implemented"))
    }
    async fn outbound_proxy_pool_settings(&self) -> Result<OutboundProxyPoolSettings> {
        Err(anyhow!(
            "outbound proxy pool settings repository is not implemented"
        ))
    }
    async fn update_outbound_proxy_pool_settings(
        &self,
        _req: UpdateOutboundProxyPoolSettingsRequest,
    ) -> Result<OutboundProxyPoolSettings> {
        Err(anyhow!(
            "outbound proxy pool settings repository is not implemented"
        ))
    }
    async fn list_outbound_proxy_nodes(&self) -> Result<Vec<OutboundProxyNode>> {
        Err(anyhow!("outbound proxy node repository is not implemented"))
    }
    async fn create_outbound_proxy_node(
        &self,
        _req: CreateOutboundProxyNodeRequest,
    ) -> Result<OutboundProxyNode> {
        Err(anyhow!("outbound proxy node repository is not implemented"))
    }
    async fn update_outbound_proxy_node(
        &self,
        _node_id: Uuid,
        _req: UpdateOutboundProxyNodeRequest,
    ) -> Result<OutboundProxyNode> {
        Err(anyhow!("outbound proxy node repository is not implemented"))
    }
    async fn delete_outbound_proxy_node(&self, _node_id: Uuid) -> Result<()> {
        Err(anyhow!("outbound proxy node repository is not implemented"))
    }
    async fn record_outbound_proxy_test_result(
        &self,
        _node_id: Uuid,
        _last_test_status: Option<String>,
        _last_latency_ms: Option<u64>,
        _last_error: Option<String>,
        _last_tested_at: Option<DateTime<Utc>>,
    ) -> Result<OutboundProxyNode> {
        Err(anyhow!("outbound proxy node repository is not implemented"))
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
    async fn queue_oauth_refresh_token(&self, req: ImportOAuthRefreshTokenRequest) -> Result<bool> {
        let upserted = self.upsert_oauth_refresh_token(req).await?;
        Ok(upserted.created)
    }
    async fn configure_system_event_runtime(
        &self,
        _runtime: Option<Arc<crate::system_events::SystemEventLogRuntime>>,
    ) -> Result<()> {
        Ok(())
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
    async fn reprobe_runtime_oauth_account(&self, account_id: Uuid) -> Result<()> {
        self.refresh_oauth_account(account_id).await.map(|_| ())
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
    async fn oauth_inventory_summary(&self) -> Result<OAuthInventorySummaryResponse> {
        Err(anyhow!("oauth inventory summary query is not implemented"))
    }
    async fn oauth_inventory_records(&self) -> Result<Vec<OAuthInventoryRecord>> {
        Err(anyhow!("oauth inventory records query is not implemented"))
    }
    async fn mark_oauth_inventory_record_failed(
        &self,
        _record_id: Uuid,
        _reason: Option<String>,
    ) -> Result<()> {
        Err(anyhow!("oauth inventory mutation is not implemented"))
    }
    async fn mark_oauth_inventory_records_failed(
        &self,
        record_ids: Vec<Uuid>,
        reason: Option<String>,
    ) -> Result<()> {
        for record_id in record_ids {
            self.mark_oauth_inventory_record_failed(record_id, reason.clone())
                .await?;
        }
        Ok(())
    }
    async fn delete_oauth_inventory_record(&self, _record_id: Uuid) -> Result<()> {
        Err(anyhow!("oauth inventory deletion is not implemented"))
    }
    async fn delete_oauth_inventory_records(&self, record_ids: Vec<Uuid>) -> Result<()> {
        for record_id in record_ids {
            self.delete_oauth_inventory_record(record_id).await?;
        }
        Ok(())
    }
    async fn oauth_runtime_pool_summary(&self) -> Result<OAuthRuntimePoolSummaryResponse> {
        let accounts = self.list_upstream_accounts().await?;
        let statuses = self
            .oauth_account_statuses(accounts.iter().map(|item| item.id).collect())
            .await?;
        let mut summary = OAuthRuntimePoolSummaryResponse {
            total: accounts.len() as u64,
            ..Default::default()
        };
        for status in statuses {
            match status.pool_state {
                OAuthAccountPoolState::Active => summary.active = summary.active.saturating_add(1),
                OAuthAccountPoolState::Quarantine => {
                    summary.quarantine = summary.quarantine.saturating_add(1)
                }
                OAuthAccountPoolState::PendingPurge => {
                    summary.pending_purge = summary.pending_purge.saturating_add(1)
                }
            }
            match status.auth_provider {
                UpstreamAuthProvider::OAuthRefreshToken => {
                    summary.oauth_refresh_token = summary.oauth_refresh_token.saturating_add(1)
                }
                UpstreamAuthProvider::LegacyBearer => {
                    summary.legacy_bearer = summary.legacy_bearer.saturating_add(1)
                }
            }
            if status.rate_limits_fetched_at.is_some() {
                summary.rate_limits_ready = summary.rate_limits_ready.saturating_add(1);
            }
        }
        Ok(summary)
    }
    async fn oauth_health_signals_summary(&self) -> Result<OAuthHealthSignalsSummaryResponse> {
        let accounts = self.list_upstream_accounts().await?;
        let statuses = self
            .oauth_account_statuses(accounts.iter().map(|item| item.id).collect())
            .await?;
        let mut summary = OAuthHealthSignalsSummaryResponse {
            total: statuses.len() as u64,
            ..Default::default()
        };
        for status in statuses {
            match status.last_live_result_status {
                Some(OAuthLiveResultStatus::Ok) => {
                    summary.live_result_ok = summary.live_result_ok.saturating_add(1)
                }
                Some(OAuthLiveResultStatus::Failed) => {
                    summary.live_result_failed = summary.live_result_failed.saturating_add(1)
                }
                None => {}
            }

            if matches!(status.pool_state, OAuthAccountPoolState::PendingPurge) {
                summary.pending_purge_signals = summary.pending_purge_signals.saturating_add(1);
            }
            if matches!(status.pool_state, OAuthAccountPoolState::Quarantine) {
                summary.quarantine_signals = summary.quarantine_signals.saturating_add(1);
            }
        }
        Ok(summary)
    }
    async fn restore_oauth_inventory_record(&self, _record_id: Uuid) -> Result<()> {
        Err(anyhow!("oauth inventory restore is not implemented"))
    }
    async fn restore_oauth_inventory_records(&self, record_ids: Vec<Uuid>) -> Result<()> {
        for record_id in record_ids {
            self.restore_oauth_inventory_record(record_id).await?;
        }
        Ok(())
    }
    async fn reprobe_oauth_inventory_record(&self, _record_id: Uuid) -> Result<()> {
        Err(anyhow!("oauth inventory reprobe is not implemented"))
    }
    async fn reprobe_oauth_inventory_records(&self, record_ids: Vec<Uuid>) -> Result<()> {
        for record_id in record_ids {
            self.reprobe_oauth_inventory_record(record_id).await?;
        }
        Ok(())
    }
    async fn purge_due_oauth_inventory_records(&self) -> Result<u64> {
        Ok(0)
    }
    async fn patrol_active_oauth_accounts(&self) -> Result<u64> {
        Ok(0)
    }
    async fn account_pool_summary(&self) -> Result<AccountPoolSummaryResponse> {
        let records = self.account_pool_records().await?;
        let mut summary = AccountPoolSummaryResponse {
            total: records.len() as u64,
            ..Default::default()
        };
        for record in records {
            match record.operator_state {
                AccountPoolOperatorState::Inventory => {
                    summary.inventory = summary.inventory.saturating_add(1)
                }
                AccountPoolOperatorState::Routable => {
                    summary.routable = summary.routable.saturating_add(1)
                }
                AccountPoolOperatorState::Cooling => {
                    summary.cooling = summary.cooling.saturating_add(1)
                }
                AccountPoolOperatorState::PendingDelete => {
                    summary.pending_delete = summary.pending_delete.saturating_add(1)
                }
            }
            match record.reason_class {
                AccountPoolReasonClass::Healthy => {
                    summary.healthy = summary.healthy.saturating_add(1)
                }
                AccountPoolReasonClass::Quota => summary.quota = summary.quota.saturating_add(1),
                AccountPoolReasonClass::Fatal => summary.fatal = summary.fatal.saturating_add(1),
                AccountPoolReasonClass::Transient => {
                    summary.transient = summary.transient.saturating_add(1)
                }
                AccountPoolReasonClass::Admin => summary.admin = summary.admin.saturating_add(1),
            }
        }
        Ok(summary)
    }
    async fn account_pool_records(&self) -> Result<Vec<AccountPoolRecord>> {
        let accounts = self.list_upstream_accounts().await?;
        let account_ids = accounts.iter().map(|item| item.id).collect::<Vec<_>>();
        let statuses = self.oauth_account_statuses(account_ids).await?;
        let status_map = statuses
            .into_iter()
            .map(|status| (status.account_id, status))
            .collect::<HashMap<_, _>>();

        let mut records = accounts
            .iter()
            .filter_map(|account| {
                status_map
                    .get(&account.id)
                    .map(|status| build_runtime_account_pool_record(account, status))
            })
            .collect::<Vec<_>>();

        records.extend(
            self.oauth_inventory_records()
                .await?
                .iter()
                .map(build_inventory_account_pool_record),
        );
        records.sort_by(|left, right| {
            account_pool_operator_state_order(left.operator_state)
                .cmp(&account_pool_operator_state_order(right.operator_state))
                .then_with(|| right.updated_at.cmp(&left.updated_at))
                .then_with(|| left.label.cmp(&right.label))
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(records)
    }
    async fn account_pool_record(&self, record_id: Uuid) -> Result<AccountPoolRecord> {
        self.account_pool_records()
            .await?
            .into_iter()
            .find(|record| record.id == record_id)
            .ok_or_else(|| anyhow!("account pool record not found"))
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
        Err(anyhow!(
            "model routing policy repository is not implemented"
        ))
    }
    async fn upsert_model_routing_policy(
        &self,
        _req: UpsertModelRoutingPolicyRequest,
    ) -> Result<ModelRoutingPolicy> {
        Err(anyhow!(
            "model routing policy repository is not implemented"
        ))
    }
    async fn delete_model_routing_policy(&self, _policy_id: Uuid) -> Result<()> {
        Err(anyhow!(
            "model routing policy repository is not implemented"
        ))
    }
    async fn model_routing_settings(&self) -> Result<ModelRoutingSettings> {
        Err(anyhow!(
            "model routing settings repository is not implemented"
        ))
    }
    async fn update_model_routing_settings(
        &self,
        _req: UpdateModelRoutingSettingsRequest,
    ) -> Result<ModelRoutingSettings> {
        Err(anyhow!(
            "model routing settings repository is not implemented"
        ))
    }
    async fn claude_code_routing_settings(&self) -> Result<ClaudeCodeRoutingSettings> {
        Err(anyhow!(
            "claude code routing settings repository is not implemented"
        ))
    }
    async fn update_claude_code_routing_settings(
        &self,
        _req: UpdateClaudeCodeRoutingSettingsRequest,
    ) -> Result<ClaudeCodeRoutingSettings> {
        Err(anyhow!(
            "claude code routing settings repository is not implemented"
        ))
    }
    async fn upstream_error_learning_settings(&self) -> Result<AiErrorLearningSettings> {
        Err(anyhow!(
            "upstream error learning settings repository is not implemented"
        ))
    }
    async fn update_upstream_error_learning_settings(
        &self,
        _req: UpdateAiErrorLearningSettingsRequest,
    ) -> Result<AiErrorLearningSettings> {
        Err(anyhow!(
            "upstream error learning settings repository is not implemented"
        ))
    }
    async fn list_upstream_error_templates(
        &self,
        _status: Option<UpstreamErrorTemplateStatus>,
    ) -> Result<Vec<UpstreamErrorTemplateRecord>> {
        Err(anyhow!(
            "upstream error template repository is not implemented"
        ))
    }
    async fn upstream_error_template_by_id(
        &self,
        _template_id: Uuid,
    ) -> Result<Option<UpstreamErrorTemplateRecord>> {
        Err(anyhow!(
            "upstream error template repository is not implemented"
        ))
    }
    async fn upstream_error_template_by_fingerprint(
        &self,
        _fingerprint: &str,
    ) -> Result<Option<UpstreamErrorTemplateRecord>> {
        Err(anyhow!(
            "upstream error template repository is not implemented"
        ))
    }
    async fn save_upstream_error_template(
        &self,
        _template: UpstreamErrorTemplateRecord,
    ) -> Result<UpstreamErrorTemplateRecord> {
        Err(anyhow!(
            "upstream error template repository is not implemented"
        ))
    }
    async fn list_builtin_error_template_overrides(
        &self,
    ) -> Result<Vec<BuiltinErrorTemplateOverrideRecord>> {
        Err(anyhow!(
            "builtin error template override repository is not implemented"
        ))
    }
    async fn save_builtin_error_template_override(
        &self,
        _record: BuiltinErrorTemplateOverrideRecord,
    ) -> Result<BuiltinErrorTemplateOverrideRecord> {
        Err(anyhow!(
            "builtin error template override repository is not implemented"
        ))
    }
    async fn delete_builtin_error_template_override(
        &self,
        _kind: BuiltinErrorTemplateKind,
        _code: &str,
    ) -> Result<()> {
        Err(anyhow!(
            "builtin error template override repository is not implemented"
        ))
    }
    async fn list_builtin_error_templates(&self) -> Result<Vec<BuiltinErrorTemplateRecord>> {
        let mut templates = default_builtin_error_templates();
        let overrides = self.list_builtin_error_template_overrides().await?;
        let overrides = overrides
            .into_iter()
            .map(|record| ((record.kind, record.code.clone()), record))
            .collect::<HashMap<_, _>>();

        for template in &mut templates {
            if let Some(record) = overrides.get(&(template.kind, template.code.clone())) {
                template.templates =
                    merge_localized_error_templates(&template.default_templates, &record.templates);
                template.is_overridden = true;
                template.updated_at = Some(record.updated_at);
            }
        }

        templates.sort_by(|left, right| {
            left.kind
                .cmp(&right.kind)
                .then_with(|| left.code.cmp(&right.code))
        });
        Ok(templates)
    }
    async fn builtin_error_template(
        &self,
        kind: BuiltinErrorTemplateKind,
        code: &str,
    ) -> Result<Option<BuiltinErrorTemplateRecord>> {
        Ok(self
            .list_builtin_error_templates()
            .await?
            .into_iter()
            .find(|template| template.kind == kind && template.code == code))
    }
    async fn list_routing_plan_versions(&self) -> Result<Vec<RoutingPlanVersion>> {
        Err(anyhow!(
            "routing plan version repository is not implemented"
        ))
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
    async fn mark_upstream_account_pending_purge(
        &self,
        account_id: Uuid,
        _reason: Option<String>,
    ) -> Result<UpstreamAccount> {
        self.set_upstream_account_enabled(account_id, false).await
    }
    async fn purge_pending_upstream_accounts(&self) -> Result<u64> {
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
    async fn update_oauth_rate_limit_cache_from_observation(
        &self,
        _account_id: Uuid,
        _rate_limits: Vec<OAuthRateLimitSnapshot>,
        _observed_at: DateTime<Utc>,
    ) -> Result<()> {
        Ok(())
    }
    async fn record_upstream_account_live_result(
        &self,
        _account_id: Uuid,
        _reported_at: DateTime<Utc>,
        _status: OAuthLiveResultStatus,
        _source: OAuthLiveResultSource,
        _status_code: Option<u16>,
        _error_code: Option<String>,
        _error_message_preview: Option<String>,
    ) -> Result<bool> {
        Ok(false)
    }
}

#[async_trait]
pub trait SnapshotPolicyStore: Send + Sync {
    async fn snapshot(&self) -> Result<DataPlaneSnapshot>;
    async fn flush_snapshot_revision(&self, max_batch: usize) -> Result<u32>;
    async fn cleanup_data_plane_outbox(&self, retention: Duration) -> Result<u64>;
    async fn data_plane_snapshot_events(
        &self,
        after: u64,
        limit: u32,
    ) -> Result<DataPlaneSnapshotEventsResponse>;
}

#[async_trait]
impl<T> SnapshotPolicyStore for T
where
    T: ControlPlaneStore + Send + Sync + ?Sized,
{
    async fn snapshot(&self) -> Result<DataPlaneSnapshot> {
        ControlPlaneStore::snapshot(self).await
    }

    async fn flush_snapshot_revision(&self, max_batch: usize) -> Result<u32> {
        ControlPlaneStore::flush_snapshot_revision(self, max_batch).await
    }

    async fn cleanup_data_plane_outbox(&self, retention: Duration) -> Result<u64> {
        ControlPlaneStore::cleanup_data_plane_outbox(self, retention).await
    }

    async fn data_plane_snapshot_events(
        &self,
        after: u64,
        limit: u32,
    ) -> Result<DataPlaneSnapshotEventsResponse> {
        ControlPlaneStore::data_plane_snapshot_events(self, after, limit).await
    }
}

#[async_trait]
pub trait TenantCatalogStore: Send + Sync {
    async fn create_tenant(&self, req: CreateTenantRequest) -> Result<Tenant>;
    async fn list_tenants(&self) -> Result<Vec<Tenant>>;
    async fn create_api_key(&self, req: CreateApiKeyRequest) -> Result<CreateApiKeyResponse>;
    async fn list_api_keys(&self) -> Result<Vec<ApiKey>>;
    async fn validate_api_key(&self, token: &str) -> Result<Option<ValidatedPrincipal>>;
}

#[async_trait]
impl<T> TenantCatalogStore for T
where
    T: ControlPlaneStore + Send + Sync + ?Sized,
{
    async fn create_tenant(&self, req: CreateTenantRequest) -> Result<Tenant> {
        ControlPlaneStore::create_tenant(self, req).await
    }

    async fn list_tenants(&self) -> Result<Vec<Tenant>> {
        ControlPlaneStore::list_tenants(self).await
    }

    async fn create_api_key(&self, req: CreateApiKeyRequest) -> Result<CreateApiKeyResponse> {
        ControlPlaneStore::create_api_key(self, req).await
    }

    async fn list_api_keys(&self) -> Result<Vec<ApiKey>> {
        ControlPlaneStore::list_api_keys(self).await
    }

    async fn validate_api_key(&self, token: &str) -> Result<Option<ValidatedPrincipal>> {
        ControlPlaneStore::validate_api_key(self, token).await
    }
}

#[async_trait]
pub trait OAuthRuntimeStore: Send + Sync {
    async fn create_upstream_account(
        &self,
        req: CreateUpstreamAccountRequest,
    ) -> Result<UpstreamAccount>;
    async fn list_upstream_accounts(&self) -> Result<Vec<UpstreamAccount>>;
    async fn refresh_expiring_oauth_accounts(&self) -> Result<()>;
    async fn activate_oauth_refresh_token_vault(&self) -> Result<u64>;
    async fn refresh_due_oauth_rate_limit_caches(&self) -> Result<u64>;
    async fn recover_oauth_rate_limit_refresh_jobs(&self) -> Result<u64>;
    async fn mark_account_seen_ok(
        &self,
        account_id: Uuid,
        seen_ok_at: DateTime<Utc>,
        min_write_interval_sec: i64,
    ) -> Result<bool>;
    async fn maybe_refresh_oauth_rate_limit_cache_on_seen_ok(&self, account_id: Uuid)
        -> Result<()>;
    async fn update_oauth_rate_limit_cache_from_observation(
        &self,
        account_id: Uuid,
        rate_limits: Vec<OAuthRateLimitSnapshot>,
        observed_at: DateTime<Utc>,
    ) -> Result<()>;
    async fn record_upstream_account_live_result(
        &self,
        account_id: Uuid,
        reported_at: DateTime<Utc>,
        status: OAuthLiveResultStatus,
        source: OAuthLiveResultSource,
        status_code: Option<u16>,
        error_code: Option<String>,
        error_message_preview: Option<String>,
    ) -> Result<bool>;
}

#[async_trait]
impl<T> OAuthRuntimeStore for T
where
    T: ControlPlaneStore + Send + Sync + ?Sized,
{
    async fn create_upstream_account(
        &self,
        req: CreateUpstreamAccountRequest,
    ) -> Result<UpstreamAccount> {
        ControlPlaneStore::create_upstream_account(self, req).await
    }

    async fn list_upstream_accounts(&self) -> Result<Vec<UpstreamAccount>> {
        ControlPlaneStore::list_upstream_accounts(self).await
    }

    async fn refresh_expiring_oauth_accounts(&self) -> Result<()> {
        ControlPlaneStore::refresh_expiring_oauth_accounts(self).await
    }

    async fn activate_oauth_refresh_token_vault(&self) -> Result<u64> {
        ControlPlaneStore::activate_oauth_refresh_token_vault(self).await
    }

    async fn refresh_due_oauth_rate_limit_caches(&self) -> Result<u64> {
        ControlPlaneStore::refresh_due_oauth_rate_limit_caches(self).await
    }

    async fn recover_oauth_rate_limit_refresh_jobs(&self) -> Result<u64> {
        ControlPlaneStore::recover_oauth_rate_limit_refresh_jobs(self).await
    }

    async fn mark_account_seen_ok(
        &self,
        account_id: Uuid,
        seen_ok_at: DateTime<Utc>,
        min_write_interval_sec: i64,
    ) -> Result<bool> {
        ControlPlaneStore::mark_account_seen_ok(
            self,
            account_id,
            seen_ok_at,
            min_write_interval_sec,
        )
        .await
    }

    async fn maybe_refresh_oauth_rate_limit_cache_on_seen_ok(
        &self,
        account_id: Uuid,
    ) -> Result<()> {
        ControlPlaneStore::maybe_refresh_oauth_rate_limit_cache_on_seen_ok(self, account_id).await
    }

    async fn update_oauth_rate_limit_cache_from_observation(
        &self,
        account_id: Uuid,
        rate_limits: Vec<OAuthRateLimitSnapshot>,
        observed_at: DateTime<Utc>,
    ) -> Result<()> {
        ControlPlaneStore::update_oauth_rate_limit_cache_from_observation(
            self,
            account_id,
            rate_limits,
            observed_at,
        )
        .await
    }

    async fn record_upstream_account_live_result(
        &self,
        account_id: Uuid,
        reported_at: DateTime<Utc>,
        status: OAuthLiveResultStatus,
        source: OAuthLiveResultSource,
        status_code: Option<u16>,
        error_code: Option<String>,
        error_message_preview: Option<String>,
    ) -> Result<bool> {
        ControlPlaneStore::record_upstream_account_live_result(
            self,
            account_id,
            reported_at,
            status,
            source,
            status_code,
            error_code,
            error_message_preview,
        )
        .await
    }
}

#[async_trait]
pub trait EditionMigrationStore: Send + Sync {
    async fn list_tenants(&self) -> Result<Vec<Tenant>>;
    async fn list_api_keys(&self) -> Result<Vec<ApiKey>>;
    async fn list_upstream_accounts(&self) -> Result<Vec<UpstreamAccount>>;
    async fn list_routing_profiles(&self) -> Result<Vec<RoutingProfile>>;
    async fn list_model_routing_policies(&self) -> Result<Vec<ModelRoutingPolicy>>;
    async fn model_routing_settings(&self) -> Result<ModelRoutingSettings>;
    async fn snapshot(&self) -> Result<DataPlaneSnapshot>;
}

#[async_trait]
impl<T> EditionMigrationStore for T
where
    T: ControlPlaneStore + Send + Sync + ?Sized,
{
    async fn list_tenants(&self) -> Result<Vec<Tenant>> {
        ControlPlaneStore::list_tenants(self).await
    }

    async fn list_api_keys(&self) -> Result<Vec<ApiKey>> {
        ControlPlaneStore::list_api_keys(self).await
    }

    async fn list_upstream_accounts(&self) -> Result<Vec<UpstreamAccount>> {
        ControlPlaneStore::list_upstream_accounts(self).await
    }

    async fn list_routing_profiles(&self) -> Result<Vec<RoutingProfile>> {
        ControlPlaneStore::list_routing_profiles(self).await
    }

    async fn list_model_routing_policies(&self) -> Result<Vec<ModelRoutingPolicy>> {
        ControlPlaneStore::list_model_routing_policies(self).await
    }

    async fn model_routing_settings(&self) -> Result<ModelRoutingSettings> {
        ControlPlaneStore::model_routing_settings(self).await
    }

    async fn snapshot(&self) -> Result<DataPlaneSnapshot> {
        ControlPlaneStore::snapshot(self).await
    }
}

fn account_pool_operator_state_order(state: AccountPoolOperatorState) -> u8 {
    match state {
        AccountPoolOperatorState::Routable => 0,
        AccountPoolOperatorState::Cooling => 1,
        AccountPoolOperatorState::Inventory => 2,
        AccountPoolOperatorState::PendingDelete => 3,
    }
}

fn account_pool_reason_class_from_code(
    code: Option<&str>,
    fallback: AccountPoolReasonClass,
) -> AccountPoolReasonClass {
    let normalized = code
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase());
    match normalized.as_deref() {
        Some(
            "token_invalidated"
            | "account_deactivated"
            | "invalid_refresh_token"
            | "refresh_token_revoked"
            | "refresh_token_reused"
            | "terminal_invalid"
            | "credential_cipher_missing",
        ) => AccountPoolReasonClass::Fatal,
        Some("rate_limited" | "quota_exhausted" | "no_quota") => AccountPoolReasonClass::Quota,
        Some(
            "upstream_unavailable" | "transport_error" | "overloaded" | "upstream_network_error",
        ) => AccountPoolReasonClass::Transient,
        Some(code) if code.starts_with("operator_") || code == "disabled_by_admin" => {
            AccountPoolReasonClass::Admin
        }
        _ => fallback,
    }
}

fn account_pool_reason_class_name(reason_class: AccountPoolReasonClass) -> &'static str {
    match reason_class {
        AccountPoolReasonClass::Healthy => "healthy",
        AccountPoolReasonClass::Quota => "quota",
        AccountPoolReasonClass::Fatal => "fatal",
        AccountPoolReasonClass::Transient => "transient",
        AccountPoolReasonClass::Admin => "admin",
    }
}

fn account_pool_state_event_name(state: AccountPoolState) -> &'static str {
    match state {
        AccountPoolState::Active => "routable",
        AccountPoolState::Quarantine => "cooling",
        AccountPoolState::PendingPurge => "pending_delete",
    }
}

fn auth_provider_name(provider: UpstreamAuthProvider) -> &'static str {
    match provider {
        UpstreamAuthProvider::OAuthRefreshToken => "oauth_refresh_token",
        UpstreamAuthProvider::LegacyBearer => "legacy_bearer",
    }
}

fn account_health_freshness_from_signals(
    now: DateTime<Utc>,
    last_seen_ok_at: Option<DateTime<Utc>>,
    last_probe_at: Option<DateTime<Utc>>,
    last_probe_outcome: Option<AccountProbeOutcome>,
    last_live_result_at: Option<DateTime<Utc>>,
    last_live_result_status: Option<&OAuthLiveResultStatus>,
) -> AccountHealthFreshness {
    let cutoff = now - Duration::seconds(ACCOUNT_HEALTH_FRESHNESS_TTL_SEC);

    let last_success_at = [
        last_seen_ok_at,
        last_probe_at.filter(|_| matches!(last_probe_outcome, Some(AccountProbeOutcome::Ok))),
        last_live_result_at
            .filter(|_| matches!(last_live_result_status, Some(OAuthLiveResultStatus::Ok))),
    ]
    .into_iter()
    .flatten()
    .max();

    let last_failure_at = [
        last_probe_at.filter(|_| {
            matches!(
                last_probe_outcome,
                Some(
                    AccountProbeOutcome::Fatal
                        | AccountProbeOutcome::Quota
                        | AccountProbeOutcome::Transient
                )
            )
        }),
        last_live_result_at
            .filter(|_| matches!(last_live_result_status, Some(OAuthLiveResultStatus::Failed))),
    ]
    .into_iter()
    .flatten()
    .max();

    if last_success_at.is_none() && last_failure_at.is_none() {
        return AccountHealthFreshness::Unknown;
    }

    if last_failure_at.is_some_and(|last_failure_at| {
        last_failure_at >= cutoff
            && last_success_at.is_none_or(|last_success_at| last_failure_at >= last_success_at)
    }) {
        return AccountHealthFreshness::Stale;
    }

    if last_success_at.is_some_and(|last_success_at| last_success_at >= cutoff) {
        return AccountHealthFreshness::Fresh;
    }

    AccountHealthFreshness::Stale
}

fn build_runtime_account_pool_record(
    account: &UpstreamAccount,
    status: &OAuthAccountStatusResponse,
) -> AccountPoolRecord {
    let (operator_state, next_action_at, fallback_code, fallback_reason_class) =
        match status.pool_state {
            OAuthAccountPoolState::PendingPurge => (
                AccountPoolOperatorState::PendingDelete,
                status.pending_purge_at,
                status
                    .pending_purge_reason
                    .clone()
                    .or_else(|| status.last_live_error_code.clone())
                    .or_else(|| status.last_refresh_error_code.clone()),
                AccountPoolReasonClass::Fatal,
            ),
            OAuthAccountPoolState::Quarantine => (
                AccountPoolOperatorState::Cooling,
                status.quarantine_until,
                status
                    .quarantine_reason
                    .clone()
                    .or_else(|| status.last_live_error_code.clone())
                    .or_else(|| status.rate_limits_last_error_code.clone())
                    .or_else(|| status.last_refresh_error_code.clone()),
                AccountPoolReasonClass::Transient,
            ),
            OAuthAccountPoolState::Active if status.effective_enabled => (
                AccountPoolOperatorState::Routable,
                None,
                None,
                AccountPoolReasonClass::Healthy,
            ),
            OAuthAccountPoolState::Active => {
                let fallback_code = if !account.enabled {
                    Some("disabled_by_admin".to_string())
                } else {
                    status
                        .last_live_error_code
                        .clone()
                        .or_else(|| status.rate_limits_last_error_code.clone())
                        .or_else(|| status.last_refresh_error_code.clone())
                        .or_else(|| match status.refresh_credential_state {
                            Some(RefreshCredentialState::TerminalInvalid) => {
                                Some("terminal_invalid".to_string())
                            }
                            Some(RefreshCredentialState::TransientFailed) => {
                                Some("refresh_failed".to_string())
                            }
                            Some(RefreshCredentialState::Healthy) | None => None,
                        })
                };
                let fallback_reason_class = if !account.enabled {
                    AccountPoolReasonClass::Admin
                } else if status.rate_limits_last_error_code.as_deref() == Some("rate_limited")
                    || status.rate_limits_last_error_code.as_deref() == Some("quota_exhausted")
                {
                    AccountPoolReasonClass::Quota
                } else if matches!(
                    status.refresh_credential_state,
                    Some(RefreshCredentialState::TerminalInvalid)
                ) {
                    AccountPoolReasonClass::Fatal
                } else {
                    AccountPoolReasonClass::Transient
                };
                (
                    AccountPoolOperatorState::Cooling,
                    status.quarantine_until.or(status.rate_limits_expires_at),
                    fallback_code,
                    fallback_reason_class,
                )
            }
        };

    let reason_class =
        account_pool_reason_class_from_code(fallback_code.as_deref(), fallback_reason_class);
    let health_freshness = account_health_freshness_from_signals(
        Utc::now(),
        status.last_seen_ok_at,
        status.last_probe_at,
        status.last_probe_outcome,
        status.last_live_result_at,
        status.last_live_result_status.as_ref(),
    );

    AccountPoolRecord {
        id: account.id,
        record_scope: AccountPoolRecordScope::Runtime,
        operator_state,
        health_freshness,
        reason_class,
        reason_code: fallback_code,
        route_eligible: matches!(operator_state, AccountPoolOperatorState::Routable),
        next_action_at,
        last_signal_at: status.last_live_result_at,
        last_signal_source: status.last_live_result_source.clone(),
        recent_signal_heatmap: None,
        last_probe_at: status.last_probe_at,
        last_probe_outcome: status.last_probe_outcome,
        label: account.label.clone(),
        email: status.email.clone(),
        chatgpt_account_id: account
            .chatgpt_account_id
            .clone()
            .or_else(|| status.chatgpt_account_user_id.clone()),
        chatgpt_plan_type: status.chatgpt_plan_type.clone(),
        source_type: status.source_type.clone(),
        mode: Some(account.mode.clone()),
        auth_provider: Some(status.auth_provider.clone()),
        credential_kind: status.credential_kind.clone(),
        has_refresh_credential: status.has_refresh_credential,
        has_access_token_fallback: status.has_access_token_fallback,
        refresh_credential_state: status.refresh_credential_state.clone(),
        enabled: Some(account.enabled),
        rate_limits: status.rate_limits.clone(),
        rate_limits_fetched_at: status.rate_limits_fetched_at,
        created_at: account.created_at,
        updated_at: status
            .last_live_result_at
            .or(status.last_refresh_at)
            .or(status.rate_limits_fetched_at)
            .unwrap_or(account.created_at),
    }
}

fn build_inventory_account_pool_record(record: &OAuthInventoryRecord) -> AccountPoolRecord {
    let (operator_state, next_action_at, fallback_reason_class) = match record.vault_status {
        OAuthVaultRecordStatus::Queued
        | OAuthVaultRecordStatus::Ready
        | OAuthVaultRecordStatus::NeedsRefresh => (
            AccountPoolOperatorState::Inventory,
            record.next_retry_at.or(record.admission_retry_after),
            AccountPoolReasonClass::Healthy,
        ),
        OAuthVaultRecordStatus::NoQuota => (
            AccountPoolOperatorState::Cooling,
            record.admission_retry_after.or(record.next_retry_at),
            AccountPoolReasonClass::Quota,
        ),
        OAuthVaultRecordStatus::Failed if record.retryable => (
            AccountPoolOperatorState::Cooling,
            record.next_retry_at.or(record.admission_retry_after),
            AccountPoolReasonClass::Transient,
        ),
        OAuthVaultRecordStatus::Failed => (
            AccountPoolOperatorState::PendingDelete,
            record.next_retry_at.or(record.admission_retry_after),
            AccountPoolReasonClass::Fatal,
        ),
    };
    let reason_code = record
        .terminal_reason
        .clone()
        .or_else(|| record.admission_error_code.clone());
    let reason_class =
        account_pool_reason_class_from_code(reason_code.as_deref(), fallback_reason_class);

    AccountPoolRecord {
        id: record.id,
        record_scope: AccountPoolRecordScope::Inventory,
        operator_state,
        health_freshness: AccountHealthFreshness::Unknown,
        reason_class,
        reason_code,
        route_eligible: false,
        next_action_at,
        last_signal_at: record.admission_checked_at,
        last_signal_source: None,
        recent_signal_heatmap: None,
        last_probe_at: None,
        last_probe_outcome: None,
        label: record.label.clone(),
        email: record.email.clone(),
        chatgpt_account_id: record.chatgpt_account_id.clone(),
        chatgpt_plan_type: record.chatgpt_plan_type.clone(),
        source_type: record.source_type.clone(),
        mode: None,
        auth_provider: Some(UpstreamAuthProvider::OAuthRefreshToken),
        credential_kind: Some(SessionCredentialKind::RefreshRotatable),
        has_refresh_credential: record.has_refresh_token,
        has_access_token_fallback: record.has_access_token_fallback,
        refresh_credential_state: None,
        enabled: None,
        rate_limits: record.admission_rate_limits.clone(),
        rate_limits_fetched_at: record.admission_checked_at,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OAuthCredentialRecord {
    access_token_enc: String,
    refresh_token_enc: String,
    #[serde(default)]
    fallback_access_token_enc: Option<String>,
    refresh_token_sha256: String,
    token_family_id: String,
    token_version: u64,
    token_expires_at: DateTime<Utc>,
    #[serde(default)]
    fallback_token_expires_at: Option<DateTime<Utc>>,
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
            fallback_access_token_enc: None,
            refresh_token_sha256: refresh_token_sha256(&token_info.refresh_token),
            token_family_id: Uuid::new_v4().to_string(),
            token_version: 1,
            token_expires_at: token_info.expires_at,
            fallback_token_expires_at: None,
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

    fn has_access_token_fallback(&self) -> bool {
        self.fallback_access_token_enc.is_some()
    }

    fn set_fallback_access_token(
        &mut self,
        cipher: &CredentialCipher,
        fallback_access_token: Option<&str>,
        fallback_token_expires_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let normalized = fallback_access_token
            .map(str::trim)
            .filter(|value| !value.is_empty());
        self.fallback_access_token_enc = match normalized {
            Some(token) => Some(cipher.encrypt(token)?),
            None => None,
        };
        self.fallback_token_expires_at = if normalized.is_some() {
            fallback_token_expires_at
        } else {
            None
        };
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct UpstreamAccountHealthStateRecord {
    seen_ok_at: Option<DateTime<Utc>>,
    #[serde(default)]
    last_probe_at: Option<DateTime<Utc>>,
    #[serde(default)]
    last_probe_outcome: Option<AccountProbeOutcome>,
    #[serde(default)]
    last_live_result_at: Option<DateTime<Utc>>,
    #[serde(default)]
    last_live_result_status: Option<OAuthLiveResultStatus>,
    #[serde(default)]
    last_live_result_source: Option<OAuthLiveResultSource>,
    #[serde(default)]
    last_live_result_status_code: Option<u16>,
    #[serde(default)]
    last_live_error_code: Option<String>,
    #[serde(default)]
    last_live_error_message_preview: Option<String>,
    #[serde(default)]
    pool_state: AccountPoolState,
    #[serde(default)]
    quarantine_until: Option<DateTime<Utc>>,
    #[serde(default)]
    quarantine_reason: Option<String>,
    #[serde(default)]
    pending_purge_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pending_purge_reason: Option<String>,
    #[serde(default)]
    last_pool_transition_at: Option<DateTime<Utc>>,
    #[serde(default)]
    token_invalidated_strike_count: u32,
    #[serde(default)]
    token_invalidated_first_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct AccountModelSupportRecord {
    supported_models: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
enum AccountPoolState {
    #[default]
    Active,
    Quarantine,
    PendingPurge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OAuthRefreshTokenVaultRecord {
    id: Uuid,
    label: String,
    #[serde(default)]
    email: Option<String>,
    base_url: String,
    refresh_token_enc: String,
    #[serde(default)]
    fallback_access_token_enc: Option<String>,
    #[serde(default)]
    fallback_token_expires_at: Option<DateTime<Utc>>,
    refresh_token_sha256: String,
    #[serde(default)]
    chatgpt_account_id: Option<String>,
    #[serde(default)]
    chatgpt_plan_type: Option<String>,
    #[serde(default)]
    source_type: Option<String>,
    desired_mode: UpstreamMode,
    desired_enabled: bool,
    desired_priority: i32,
    #[serde(default)]
    status: OAuthVaultRecordStatus,
    #[serde(default)]
    failure_count: u32,
    #[serde(default)]
    backoff_until: Option<DateTime<Utc>>,
    #[serde(default)]
    next_attempt_at: Option<DateTime<Utc>>,
    #[serde(default)]
    last_error_code: Option<String>,
    #[serde(default)]
    last_error_message: Option<String>,
    #[serde(default)]
    admission_source: Option<String>,
    #[serde(default)]
    admission_checked_at: Option<DateTime<Utc>>,
    #[serde(default)]
    admission_retry_after: Option<DateTime<Utc>>,
    #[serde(default)]
    admission_error_code: Option<String>,
    #[serde(default)]
    admission_error_message: Option<String>,
    #[serde(default)]
    admission_rate_limits: Vec<OAuthRateLimitSnapshot>,
    #[serde(default)]
    admission_rate_limits_expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    failure_stage: Option<OAuthInventoryFailureStage>,
    #[serde(default)]
    attempt_count: u32,
    #[serde(default)]
    transient_retry_count: u32,
    #[serde(default)]
    next_retry_at: Option<DateTime<Utc>>,
    #[serde(default)]
    retryable: bool,
    #[serde(default)]
    terminal_reason: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct OAuthRateLimitCacheRecord {
    rate_limits: Vec<OAuthRateLimitSnapshot>,
    fetched_at: Option<DateTime<Utc>>,
    expires_at: Option<DateTime<Utc>>,
    last_error_code: Option<String>,
    last_error: Option<String>,
}

pub struct InMemoryStore {
    tenants: Arc<RwLock<HashMap<Uuid, Tenant>>>,
    api_keys: Arc<RwLock<HashMap<Uuid, ApiKey>>>,
    api_key_tokens: Arc<RwLock<HashMap<String, Uuid>>>,
    accounts: Arc<RwLock<HashMap<Uuid, UpstreamAccount>>>,
    account_auth_providers: Arc<RwLock<HashMap<Uuid, UpstreamAuthProvider>>>,
    oauth_credentials: Arc<RwLock<HashMap<Uuid, OAuthCredentialRecord>>>,
    oauth_refresh_token_vault: Arc<RwLock<HashMap<Uuid, OAuthRefreshTokenVaultRecord>>>,
    session_profiles: Arc<RwLock<HashMap<Uuid, SessionProfileRecord>>>,
    account_health_states: Arc<RwLock<HashMap<Uuid, UpstreamAccountHealthStateRecord>>>,
    account_model_support: Arc<RwLock<HashMap<Uuid, AccountModelSupportRecord>>>,
    oauth_rate_limit_caches: Arc<RwLock<HashMap<Uuid, OAuthRateLimitCacheRecord>>>,
    oauth_rate_limit_refresh_jobs: Arc<RwLock<HashMap<Uuid, OAuthRateLimitRefreshJobSummary>>>,
    outbound_proxy_pool_settings: Arc<RwLock<OutboundProxyPoolSettings>>,
    outbound_proxy_nodes: Arc<RwLock<HashMap<Uuid, OutboundProxyNode>>>,
    policies: Arc<RwLock<HashMap<Uuid, RoutingPolicy>>>,
    routing_profiles: Arc<RwLock<HashMap<Uuid, RoutingProfile>>>,
    model_routing_policies: Arc<RwLock<HashMap<Uuid, ModelRoutingPolicy>>>,
    model_routing_settings: Arc<RwLock<ModelRoutingSettings>>,
    claude_code_routing_settings: Arc<RwLock<ClaudeCodeRoutingSettings>>,
    upstream_error_learning_settings: Arc<RwLock<AiErrorLearningSettings>>,
    upstream_error_templates: Arc<RwLock<HashMap<Uuid, UpstreamErrorTemplateRecord>>>,
    upstream_error_template_index: Arc<RwLock<HashMap<String, Uuid>>>,
    builtin_error_template_overrides: Arc<
        RwLock<HashMap<(BuiltinErrorTemplateKind, String), BuiltinErrorTemplateOverrideRecord>>,
    >,
    routing_plan_versions: Arc<RwLock<Vec<RoutingPlanVersion>>>,
    system_event_runtime: Arc<RwLock<Option<Arc<crate::system_events::SystemEventLogRuntime>>>>,
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
