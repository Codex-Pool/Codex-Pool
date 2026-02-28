use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use codex_pool_core::api::{
    CreateApiKeyRequest, CreateApiKeyResponse, CreateTenantRequest, CreateUpstreamAccountRequest,
    DataPlaneSnapshot, DataPlaneSnapshotEvent, DataPlaneSnapshotEventType,
    DataPlaneSnapshotEventsResponse, ImportOAuthRefreshTokenRequest, OAuthAccountStatusResponse,
    OAuthFamilyActionResponse, OAuthRateLimitRefreshErrorSummary, OAuthRateLimitRefreshJobStatus,
    OAuthRateLimitRefreshJobSummary, OAuthRateLimitSnapshot, OAuthRefreshStatus,
    SessionCredentialKind, UpsertRetryPolicyRequest, UpsertRoutingPolicyRequest,
    UpsertStreamRetryPolicyRequest, ValidateOAuthRefreshTokenRequest,
    ValidateOAuthRefreshTokenResponse,
};
use codex_pool_core::model::{
    ApiKey, RoutingPolicy, RoutingStrategy, Tenant, UpstreamAccount, UpstreamAuthProvider,
    UpstreamMode,
};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use super::UpsertOneTimeSessionAccountRequest;
use super::{
    ClaimedProbeAccount, ControlPlaneStore, OAuthUpsertResult, UpstreamProbeStatus,
    UpstreamProbeWrite, ValidatedPrincipal,
};
use crate::crypto::CredentialCipher;
use crate::oauth::{OAuthTokenClient, OAuthTokenInfo, OpenAiOAuthClient};

const SNAPSHOT_SINGLETON_ROW: bool = true;
const SCHEMA_MIGRATION_LOCK_ID: i64 = 42_037_001;
const AUTH_PROVIDER_LEGACY_BEARER: &str = "legacy_bearer";
const AUTH_PROVIDER_OAUTH_REFRESH_TOKEN: &str = "oauth_refresh_token";
const OAUTH_MANAGED_BEARER_SENTINEL: &str = "__managed_oauth__";
const POOL_STATE_ACTIVE: &str = "active";
const UPSTREAM_OP_TYPE_PROBE: &str = "probe";
const OUTBOX_EVENT_ACCOUNT_UPSERT: &str = "account_upsert";
const OUTBOX_EVENT_ACCOUNT_DELETE: &str = "account_delete";
const OAUTH_REFRESH_WINDOW_SEC: i64 = 300;
const OAUTH_MIN_VALID_SEC: i64 = 60;
const OAUTH_REFRESH_INFLIGHT_TTL_SEC: i64 = 90;
const SESSION_CREDENTIAL_KIND_REFRESH_ROTATABLE: &str = "refresh_rotatable";
const SESSION_CREDENTIAL_KIND_ONE_TIME_ACCESS_TOKEN: &str = "one_time_access_token";
const DB_MAX_CONNECTIONS_ENV: &str = "CONTROL_PLANE_DB_MAX_CONNECTIONS";
const DEFAULT_DB_MAX_CONNECTIONS: u32 = 100;
const MIN_DB_MAX_CONNECTIONS: u32 = 5;
const MAX_DB_MAX_CONNECTIONS: u32 = 100;
const OAUTH_REFRESH_BATCH_SIZE_ENV: &str = "CONTROL_PLANE_OAUTH_REFRESH_BATCH_SIZE";
const DEFAULT_OAUTH_REFRESH_BATCH_SIZE: usize = 200;
const MIN_OAUTH_REFRESH_BATCH_SIZE: usize = 1;
const MAX_OAUTH_REFRESH_BATCH_SIZE: usize = 2000;
const OAUTH_REFRESH_CONCURRENCY_ENV: &str = "CONTROL_PLANE_OAUTH_REFRESH_CONCURRENCY";
const DEFAULT_OAUTH_REFRESH_CONCURRENCY: usize = 8;
const MIN_OAUTH_REFRESH_CONCURRENCY: usize = 1;
const MAX_OAUTH_REFRESH_CONCURRENCY: usize = 64;
const OAUTH_REFRESH_MAX_RPS_ENV: &str = "CONTROL_PLANE_OAUTH_REFRESH_MAX_RPS";
const DEFAULT_OAUTH_REFRESH_MAX_RPS: u32 = 5;
const MIN_OAUTH_REFRESH_MAX_RPS: u32 = 1;
const MAX_OAUTH_REFRESH_MAX_RPS: u32 = 200;
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
const OAUTH_VAULT_ACTIVATE_BATCH_SIZE_ENV: &str = "CONTROL_PLANE_VAULT_ACTIVATE_BATCH_SIZE";
const DEFAULT_OAUTH_VAULT_ACTIVATE_BATCH_SIZE: usize = 200;
const MIN_OAUTH_VAULT_ACTIVATE_BATCH_SIZE: usize = 1;
const MAX_OAUTH_VAULT_ACTIVATE_BATCH_SIZE: usize = 2_000;
const OAUTH_VAULT_ACTIVATE_CONCURRENCY_ENV: &str =
    "CONTROL_PLANE_VAULT_ACTIVATE_CONCURRENCY";
const DEFAULT_OAUTH_VAULT_ACTIVATE_CONCURRENCY: usize = 8;
const MIN_OAUTH_VAULT_ACTIVATE_CONCURRENCY: usize = 1;
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

fn postgres_max_connections_from_env() -> u32 {
    std::env::var(DB_MAX_CONNECTIONS_ENV)
        .ok()
        .and_then(|raw| raw.parse::<u32>().ok())
        .unwrap_or(DEFAULT_DB_MAX_CONNECTIONS)
        .clamp(MIN_DB_MAX_CONNECTIONS, MAX_DB_MAX_CONNECTIONS)
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

fn oauth_refresh_max_rps_from_env() -> u32 {
    std::env::var(OAUTH_REFRESH_MAX_RPS_ENV)
        .ok()
        .and_then(|raw| raw.parse::<u32>().ok())
        .unwrap_or(DEFAULT_OAUTH_REFRESH_MAX_RPS)
        .clamp(MIN_OAUTH_REFRESH_MAX_RPS, MAX_OAUTH_REFRESH_MAX_RPS)
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
        .clamp(MIN_OAUTH_VAULT_ACTIVATE_MAX_RPS, MAX_OAUTH_VAULT_ACTIVATE_MAX_RPS)
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

pub struct PostgresStore {
    pool: PgPool,
    oauth_client: std::sync::Arc<dyn OAuthTokenClient>,
    credential_cipher: Option<CredentialCipher>,
}

include!("postgres/impl_crud.rs");
include!("postgres/impl_oauth_snapshot.rs");
include!("postgres/helpers_trait.rs");

#[cfg(test)]
mod postgres_env_tests {
    use std::sync::{LazyLock, Mutex};

    use super::{
        is_blocking_rate_limit_error, oauth_effective_enabled, oauth_refresh_batch_size_from_env,
        oauth_refresh_concurrency_from_env, oauth_refresh_max_rps_from_env,
        rate_limit_failure_backoff_seconds,
        postgres_max_connections_from_env,
    };
    use chrono::{Duration, Utc};
    use codex_pool_core::api::OAuthRefreshStatus;
    use codex_pool_core::model::UpstreamAuthProvider;
    use codex_pool_core::api::SessionCredentialKind;

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
    fn postgres_max_connections_uses_safe_default() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old = set_env("CONTROL_PLANE_DB_MAX_CONNECTIONS", None);

        assert_eq!(postgres_max_connections_from_env(), 100);

        set_env("CONTROL_PLANE_DB_MAX_CONNECTIONS", old.as_deref());
    }

    #[test]
    fn postgres_max_connections_clamps_low_values() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old = set_env("CONTROL_PLANE_DB_MAX_CONNECTIONS", Some("1"));

        assert_eq!(postgres_max_connections_from_env(), 5);

        set_env("CONTROL_PLANE_DB_MAX_CONNECTIONS", old.as_deref());
    }

    #[test]
    fn oauth_refresh_batch_size_uses_safe_default() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old = set_env("CONTROL_PLANE_OAUTH_REFRESH_BATCH_SIZE", None);

        assert_eq!(oauth_refresh_batch_size_from_env(), 200);

        set_env("CONTROL_PLANE_OAUTH_REFRESH_BATCH_SIZE", old.as_deref());
    }

    #[test]
    fn oauth_refresh_batch_size_clamps_high_values() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old = set_env("CONTROL_PLANE_OAUTH_REFRESH_BATCH_SIZE", Some("99999"));

        assert_eq!(oauth_refresh_batch_size_from_env(), 2000);

        set_env("CONTROL_PLANE_OAUTH_REFRESH_BATCH_SIZE", old.as_deref());
    }

    #[test]
    fn oauth_refresh_concurrency_uses_safe_default() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old = set_env("CONTROL_PLANE_OAUTH_REFRESH_CONCURRENCY", None);

        assert_eq!(oauth_refresh_concurrency_from_env(), 8);

        set_env("CONTROL_PLANE_OAUTH_REFRESH_CONCURRENCY", old.as_deref());
    }

    #[test]
    fn oauth_refresh_concurrency_clamps_low_values() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old = set_env("CONTROL_PLANE_OAUTH_REFRESH_CONCURRENCY", Some("0"));

        assert_eq!(oauth_refresh_concurrency_from_env(), 1);

        set_env("CONTROL_PLANE_OAUTH_REFRESH_CONCURRENCY", old.as_deref());
    }

    #[test]
    fn oauth_refresh_max_rps_uses_safe_default_and_clamps() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old = set_env("CONTROL_PLANE_OAUTH_REFRESH_MAX_RPS", None);
        assert_eq!(oauth_refresh_max_rps_from_env(), 5);

        set_env("CONTROL_PLANE_OAUTH_REFRESH_MAX_RPS", Some("0"));
        assert_eq!(oauth_refresh_max_rps_from_env(), 1);

        set_env("CONTROL_PLANE_OAUTH_REFRESH_MAX_RPS", Some("999"));
        assert_eq!(oauth_refresh_max_rps_from_env(), 200);

        set_env("CONTROL_PLANE_OAUTH_REFRESH_MAX_RPS", old.as_deref());
    }

    #[test]
    fn blocking_rate_limit_error_detects_quota_and_auth() {
        assert!(is_blocking_rate_limit_error(
            Some("usage_limit"),
            Some("You've hit your usage limit")
        ));
        assert!(is_blocking_rate_limit_error(
            Some("invalid_refresh_token"),
            Some("token invalid")
        ));
        assert!(!is_blocking_rate_limit_error(
            Some("upstream_unavailable"),
            Some("gateway timeout")
        ));
    }

    #[test]
    fn rate_limit_failure_backoff_is_class_aware() {
        assert_eq!(
            rate_limit_failure_backoff_seconds("usage_limit", "quota exceeded"),
            6 * 60 * 60
        );
        assert_eq!(
            rate_limit_failure_backoff_seconds("invalid_refresh_token", "auth failed"),
            30 * 60
        );
        assert_eq!(
            rate_limit_failure_backoff_seconds("rate_limited", "too many requests"),
            120
        );
    }

    #[test]
    fn oauth_effective_enabled_blocks_active_quota_window() {
        let now = Utc::now();
        let enabled = oauth_effective_enabled(
            true,
            &UpstreamAuthProvider::OAuthRefreshToken,
            Some(&SessionCredentialKind::RefreshRotatable),
            Some(now + Duration::minutes(30)),
            &OAuthRefreshStatus::Ok,
            false,
            None,
            Some(now + Duration::minutes(10)),
            Some("usage_limit"),
            Some("quota"),
            now,
        );
        assert!(!enabled);
    }

    #[test]
    fn oauth_effective_enabled_blocks_refresh_reused_detected() {
        let now = Utc::now();
        let enabled = oauth_effective_enabled(
            true,
            &UpstreamAuthProvider::OAuthRefreshToken,
            Some(&SessionCredentialKind::RefreshRotatable),
            Some(now + Duration::minutes(30)),
            &OAuthRefreshStatus::Ok,
            true,
            None,
            None,
            None,
            None,
            now,
        );
        assert!(!enabled);
    }

    #[test]
    fn oauth_effective_enabled_blocks_fatal_refresh_failure() {
        let now = Utc::now();
        let enabled = oauth_effective_enabled(
            true,
            &UpstreamAuthProvider::OAuthRefreshToken,
            Some(&SessionCredentialKind::RefreshRotatable),
            Some(now + Duration::minutes(30)),
            &OAuthRefreshStatus::Failed,
            false,
            Some("invalid_refresh_token"),
            None,
            None,
            None,
            now,
        );
        assert!(!enabled);
    }
}
