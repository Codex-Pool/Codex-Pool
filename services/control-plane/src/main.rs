use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use control_plane::admin_auth::AdminAuthService;
use control_plane::app::{
    build_app_with_store_ttl_usage_repo_import_store_and_admin_auth,
    codex_oauth_callback_listen_mode_from_env, CodexOAuthCallbackListenMode,
    DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC,
};
use control_plane::config::ControlPlaneConfig;
use control_plane::import_jobs::{InMemoryOAuthImportJobStore, PostgresOAuthImportJobStore};
use control_plane::store::postgres::PostgresStore;
use control_plane::store::{ControlPlaneStore, InMemoryStore};
use control_plane::tenant::{
    record_billing_reconcile_runtime_failed, record_billing_reconcile_runtime_stats,
    BillingReconcileFactRequest, BillingReconcileRequest, BillingReconcileStats, TenantAuthService,
};
use control_plane::usage::clickhouse_repo::{ClickHouseUsageRepo, UsageQueryRepository};
use tokio::time::MissedTickBehavior;

const SNAPSHOT_REVISION_FLUSH_MS_ENV: &str = "CONTROL_PLANE_SNAPSHOT_REVISION_FLUSH_MS";
const SNAPSHOT_REVISION_MAX_BATCH_ENV: &str = "CONTROL_PLANE_SNAPSHOT_REVISION_MAX_BATCH";
const RATE_LIMIT_CACHE_REFRESH_ENABLED_ENV: &str = "CONTROL_PLANE_RATE_LIMIT_CACHE_REFRESH_ENABLED";
const RATE_LIMIT_CACHE_REFRESH_INTERVAL_SEC_ENV: &str =
    "CONTROL_PLANE_RATE_LIMIT_CACHE_REFRESH_INTERVAL_SEC";
const OAUTH_VAULT_ACTIVATE_ENABLED_ENV: &str = "CONTROL_PLANE_VAULT_ACTIVATE_ENABLED";
const OAUTH_VAULT_ACTIVATE_INTERVAL_SEC_ENV: &str = "CONTROL_PLANE_VAULT_ACTIVATE_INTERVAL_SEC";
const DATA_PLANE_OUTBOX_CLEANUP_INTERVAL_SEC_ENV: &str =
    "CONTROL_PLANE_DATA_PLANE_OUTBOX_CLEANUP_INTERVAL_SEC";
const DATA_PLANE_OUTBOX_RETENTION_SEC_ENV: &str = "CONTROL_PLANE_DATA_PLANE_OUTBOX_RETENTION_SEC";
const DEFAULT_SNAPSHOT_REVISION_FLUSH_MS: u64 = 200;
const MIN_SNAPSHOT_REVISION_FLUSH_MS: u64 = 50;
const MAX_SNAPSHOT_REVISION_FLUSH_MS: u64 = 5_000;
const DEFAULT_SNAPSHOT_REVISION_MAX_BATCH: usize = 1_000;
const MIN_SNAPSHOT_REVISION_MAX_BATCH: usize = 1;
const MAX_SNAPSHOT_REVISION_MAX_BATCH: usize = 10_000;
const DEFAULT_RATE_LIMIT_CACHE_REFRESH_ENABLED: bool = false;
const DEFAULT_RATE_LIMIT_CACHE_REFRESH_INTERVAL_SEC: u64 = 30;
const MIN_RATE_LIMIT_CACHE_REFRESH_INTERVAL_SEC: u64 = 5;
const MAX_RATE_LIMIT_CACHE_REFRESH_INTERVAL_SEC: u64 = 3_600;
const DEFAULT_OAUTH_VAULT_ACTIVATE_ENABLED: bool = true;
const DEFAULT_OAUTH_VAULT_ACTIVATE_INTERVAL_SEC: u64 = 30;
const MIN_OAUTH_VAULT_ACTIVATE_INTERVAL_SEC: u64 = 5;
const MAX_OAUTH_VAULT_ACTIVATE_INTERVAL_SEC: u64 = 3_600;
const DEFAULT_DATA_PLANE_OUTBOX_CLEANUP_INTERVAL_SEC: u64 = 300;
const MIN_DATA_PLANE_OUTBOX_CLEANUP_INTERVAL_SEC: u64 = 30;
const MAX_DATA_PLANE_OUTBOX_CLEANUP_INTERVAL_SEC: u64 = 3_600;
const DEFAULT_DATA_PLANE_OUTBOX_RETENTION_SEC: u64 = 7 * 24 * 60 * 60;
const MIN_DATA_PLANE_OUTBOX_RETENTION_SEC: u64 = 60;
const MAX_DATA_PLANE_OUTBOX_RETENTION_SEC: u64 = 30 * 24 * 60 * 60;
const BILLING_RECONCILE_ENABLED_ENV: &str = "CONTROL_PLANE_BILLING_RECONCILE_ENABLED";
const BILLING_RECONCILE_INTERVAL_SEC_ENV: &str = "CONTROL_PLANE_BILLING_RECONCILE_INTERVAL_SEC";
const BILLING_RECONCILE_BATCH_ENV: &str = "CONTROL_PLANE_BILLING_RECONCILE_BATCH";
const BILLING_RECONCILE_STALE_SEC_ENV: &str = "CONTROL_PLANE_BILLING_RECONCILE_STALE_SEC";
const BILLING_RECONCILE_LOOKBACK_SEC_ENV: &str = "CONTROL_PLANE_BILLING_RECONCILE_LOOKBACK_SEC";
const BILLING_RECONCILE_FULL_SWEEP_INTERVAL_SEC_ENV: &str =
    "CONTROL_PLANE_BILLING_RECONCILE_FULL_SWEEP_INTERVAL_SEC";
const CODEX_OAUTH_CALLBACK_LISTEN_ENV: &str = "CODEX_OAUTH_CALLBACK_LISTEN";
const DEFAULT_CODEX_OAUTH_CALLBACK_LISTEN_ADDR: &str = "127.0.0.1:1455";
const DEFAULT_BILLING_RECONCILE_ENABLED: bool = true;
const DEFAULT_BILLING_RECONCILE_INTERVAL_SEC: u64 = 300;
const MIN_BILLING_RECONCILE_INTERVAL_SEC: u64 = 30;
const MAX_BILLING_RECONCILE_INTERVAL_SEC: u64 = 3_600;
const DEFAULT_BILLING_RECONCILE_BATCH: usize = 200;
const MIN_BILLING_RECONCILE_BATCH: usize = 1;
const MAX_BILLING_RECONCILE_BATCH: usize = 5_000;
const DEFAULT_BILLING_RECONCILE_STALE_SEC: u64 = 900;
const MIN_BILLING_RECONCILE_STALE_SEC: u64 = 60;
const MAX_BILLING_RECONCILE_STALE_SEC: u64 = 86_400;
const DEFAULT_BILLING_RECONCILE_LOOKBACK_SEC: u64 = 24 * 60 * 60;
const MIN_BILLING_RECONCILE_LOOKBACK_SEC: u64 = 60;
const MAX_BILLING_RECONCILE_LOOKBACK_SEC: u64 = 30 * 24 * 60 * 60;
const DEFAULT_BILLING_RECONCILE_FULL_SWEEP_INTERVAL_SEC: u64 = 60 * 60;
const MIN_BILLING_RECONCILE_FULL_SWEEP_INTERVAL_SEC: u64 = 60;
const MAX_BILLING_RECONCILE_FULL_SWEEP_INTERVAL_SEC: u64 = 24 * 60 * 60;

fn snapshot_revision_flush_ms_from_env() -> u64 {
    std::env::var(SNAPSHOT_REVISION_FLUSH_MS_ENV)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(DEFAULT_SNAPSHOT_REVISION_FLUSH_MS)
        .clamp(
            MIN_SNAPSHOT_REVISION_FLUSH_MS,
            MAX_SNAPSHOT_REVISION_FLUSH_MS,
        )
}

fn snapshot_revision_max_batch_from_env() -> usize {
    std::env::var(SNAPSHOT_REVISION_MAX_BATCH_ENV)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(DEFAULT_SNAPSHOT_REVISION_MAX_BATCH)
        .clamp(
            MIN_SNAPSHOT_REVISION_MAX_BATCH,
            MAX_SNAPSHOT_REVISION_MAX_BATCH,
        )
}

fn rate_limit_cache_refresh_enabled_from_env() -> bool {
    std::env::var(RATE_LIMIT_CACHE_REFRESH_ENABLED_ENV)
        .ok()
        .and_then(|raw| {
            let normalized = raw.trim().to_ascii_lowercase();
            match normalized.as_str() {
                "1" | "true" | "yes" | "on" => Some(true),
                "0" | "false" | "no" | "off" => Some(false),
                _ => None,
            }
        })
        .unwrap_or(DEFAULT_RATE_LIMIT_CACHE_REFRESH_ENABLED)
}

fn rate_limit_cache_refresh_interval_sec_from_env() -> u64 {
    std::env::var(RATE_LIMIT_CACHE_REFRESH_INTERVAL_SEC_ENV)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(DEFAULT_RATE_LIMIT_CACHE_REFRESH_INTERVAL_SEC)
        .clamp(
            MIN_RATE_LIMIT_CACHE_REFRESH_INTERVAL_SEC,
            MAX_RATE_LIMIT_CACHE_REFRESH_INTERVAL_SEC,
        )
}

fn oauth_vault_activate_enabled_from_env() -> bool {
    std::env::var(OAUTH_VAULT_ACTIVATE_ENABLED_ENV)
        .ok()
        .and_then(|raw| {
            let normalized = raw.trim().to_ascii_lowercase();
            match normalized.as_str() {
                "1" | "true" | "yes" | "on" => Some(true),
                "0" | "false" | "no" | "off" => Some(false),
                _ => None,
            }
        })
        .unwrap_or(DEFAULT_OAUTH_VAULT_ACTIVATE_ENABLED)
}

fn oauth_vault_activate_interval_sec_from_env() -> u64 {
    std::env::var(OAUTH_VAULT_ACTIVATE_INTERVAL_SEC_ENV)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(DEFAULT_OAUTH_VAULT_ACTIVATE_INTERVAL_SEC)
        .clamp(
            MIN_OAUTH_VAULT_ACTIVATE_INTERVAL_SEC,
            MAX_OAUTH_VAULT_ACTIVATE_INTERVAL_SEC,
        )
}

fn data_plane_outbox_cleanup_interval_sec_from_env() -> u64 {
    std::env::var(DATA_PLANE_OUTBOX_CLEANUP_INTERVAL_SEC_ENV)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(DEFAULT_DATA_PLANE_OUTBOX_CLEANUP_INTERVAL_SEC)
        .clamp(
            MIN_DATA_PLANE_OUTBOX_CLEANUP_INTERVAL_SEC,
            MAX_DATA_PLANE_OUTBOX_CLEANUP_INTERVAL_SEC,
        )
}

fn data_plane_outbox_retention_sec_from_env() -> u64 {
    std::env::var(DATA_PLANE_OUTBOX_RETENTION_SEC_ENV)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(DEFAULT_DATA_PLANE_OUTBOX_RETENTION_SEC)
        .clamp(
            MIN_DATA_PLANE_OUTBOX_RETENTION_SEC,
            MAX_DATA_PLANE_OUTBOX_RETENTION_SEC,
        )
}

fn billing_reconcile_enabled_from_env() -> bool {
    std::env::var(BILLING_RECONCILE_ENABLED_ENV)
        .ok()
        .and_then(|raw| {
            let normalized = raw.trim().to_ascii_lowercase();
            match normalized.as_str() {
                "1" | "true" | "yes" | "on" => Some(true),
                "0" | "false" | "no" | "off" => Some(false),
                _ => None,
            }
        })
        .unwrap_or(DEFAULT_BILLING_RECONCILE_ENABLED)
}

fn billing_reconcile_interval_sec_from_env() -> u64 {
    std::env::var(BILLING_RECONCILE_INTERVAL_SEC_ENV)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(DEFAULT_BILLING_RECONCILE_INTERVAL_SEC)
        .clamp(
            MIN_BILLING_RECONCILE_INTERVAL_SEC,
            MAX_BILLING_RECONCILE_INTERVAL_SEC,
        )
}

fn billing_reconcile_batch_from_env() -> usize {
    std::env::var(BILLING_RECONCILE_BATCH_ENV)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(DEFAULT_BILLING_RECONCILE_BATCH)
        .clamp(MIN_BILLING_RECONCILE_BATCH, MAX_BILLING_RECONCILE_BATCH)
}

fn billing_reconcile_stale_sec_from_env() -> u64 {
    std::env::var(BILLING_RECONCILE_STALE_SEC_ENV)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(DEFAULT_BILLING_RECONCILE_STALE_SEC)
        .clamp(
            MIN_BILLING_RECONCILE_STALE_SEC,
            MAX_BILLING_RECONCILE_STALE_SEC,
        )
}

fn billing_reconcile_lookback_sec_from_env() -> u64 {
    std::env::var(BILLING_RECONCILE_LOOKBACK_SEC_ENV)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(DEFAULT_BILLING_RECONCILE_LOOKBACK_SEC)
        .clamp(
            MIN_BILLING_RECONCILE_LOOKBACK_SEC,
            MAX_BILLING_RECONCILE_LOOKBACK_SEC,
        )
}

fn billing_reconcile_full_sweep_interval_sec_from_env() -> u64 {
    std::env::var(BILLING_RECONCILE_FULL_SWEEP_INTERVAL_SEC_ENV)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(DEFAULT_BILLING_RECONCILE_FULL_SWEEP_INTERVAL_SEC)
        .clamp(
            MIN_BILLING_RECONCILE_FULL_SWEEP_INTERVAL_SEC,
            MAX_BILLING_RECONCILE_FULL_SWEEP_INTERVAL_SEC,
        )
}

fn codex_oauth_callback_listen_addr_from_env() -> anyhow::Result<Option<SocketAddr>> {
    let raw = std::env::var(CODEX_OAUTH_CALLBACK_LISTEN_ENV)
        .unwrap_or_else(|_| DEFAULT_CODEX_OAUTH_CALLBACK_LISTEN_ADDR.to_string());
    let normalized = raw.trim();
    if normalized.is_empty() {
        return Ok(None);
    }
    let lowered = normalized.to_ascii_lowercase();
    if lowered == "off" || lowered == "none" || lowered == "disabled" {
        return Ok(None);
    }
    normalized.parse::<SocketAddr>().map(Some).map_err(|err| {
        anyhow::anyhow!(
            "invalid {} value '{}': {}",
            CODEX_OAUTH_CALLBACK_LISTEN_ENV,
            normalized,
            err
        )
    })
}

fn merge_reconcile_stats(total: &mut BillingReconcileStats, delta: &BillingReconcileStats) {
    total.scanned = total.scanned.saturating_add(delta.scanned);
    total.adjusted = total.adjusted.saturating_add(delta.adjusted);
    total.released_authorizations = total
        .released_authorizations
        .saturating_add(delta.released_authorizations);
    total.adjusted_microcredits_total = total
        .adjusted_microcredits_total
        .saturating_add(delta.adjusted_microcredits_total);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = ControlPlaneConfig::from_env(DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC)?;
    config.apply_runtime_env_defaults();
    control_plane::security::ensure_api_key_hasher_configured()?;

    let (store, import_job_store, admin_auth): (
        Arc<dyn ControlPlaneStore>,
        Arc<dyn control_plane::import_jobs::OAuthImportJobStore>,
        AdminAuthService,
    ) = match config.database_url.as_deref() {
        Some(database_url) => {
            let postgres_store = PostgresStore::connect(database_url).await?;
            let import_store =
                PostgresOAuthImportJobStore::new(postgres_store.clone_pool()).await?;
            let admin_auth = AdminAuthService::from_env_with_postgres(postgres_store.clone_pool())?;
            admin_auth.ensure_bootstrap_admin_user().await?;
            (Arc::new(postgres_store), Arc::new(import_store), admin_auth)
        }
        None => (
            Arc::new(InMemoryStore::default()),
            Arc::new(InMemoryOAuthImportJobStore::default()),
            AdminAuthService::from_env()?,
        ),
    };

    match store.recover_oauth_rate_limit_refresh_jobs().await {
        Ok(recovered) if recovered > 0 => {
            tracing::warn!(
                recovered,
                "recovered stale oauth rate-limit refresh jobs from previous process"
            );
        }
        Ok(_) => {}
        Err(err) => {
            tracing::warn!(error = %err, "failed to recover oauth rate-limit refresh jobs");
        }
    }

    if config.oauth_refresh_enabled {
        let refresh_store = store.clone();
        let refresh_interval = Duration::from_secs(config.oauth_refresh_interval_sec.max(1));
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(refresh_interval);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                ticker.tick().await;
                if let Err(err) = refresh_store.refresh_expiring_oauth_accounts().await {
                    tracing::warn!(error = %err, "oauth refresh loop tick failed");
                }
            }
        });
        tracing::info!(
            interval_sec = config.oauth_refresh_interval_sec,
            "oauth refresh loop started"
        );
    } else {
        tracing::info!("oauth refresh loop disabled by config");
    }

    if oauth_vault_activate_enabled_from_env() {
        let vault_store = store.clone();
        let interval_sec = oauth_vault_activate_interval_sec_from_env();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(interval_sec));
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                ticker.tick().await;
                if let Err(err) = vault_store.activate_oauth_refresh_token_vault().await {
                    tracing::warn!(error = %err, "oauth vault activation loop tick failed");
                }
            }
        });
        tracing::info!(interval_sec, "oauth vault activation loop started");
    } else {
        tracing::info!("oauth vault activation loop disabled by config");
    }

    if rate_limit_cache_refresh_enabled_from_env() {
        let rate_limit_store = store.clone();
        let interval_sec = rate_limit_cache_refresh_interval_sec_from_env();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(interval_sec));
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                ticker.tick().await;
                if let Err(err) = rate_limit_store.refresh_due_oauth_rate_limit_caches().await {
                    tracing::warn!(error = %err, "oauth rate-limit cache refresh loop tick failed");
                }
            }
        });
        tracing::info!(interval_sec, "oauth rate-limit cache refresh loop started");
    } else {
        tracing::info!("oauth rate-limit cache refresh loop disabled by config");
    }
    tracing::info!(
        block_rules =
            "refresh_reused_detected|fatal_refresh_failure|active_auth_or_quota_rate_limit_window",
        backoff_rules = "quota=6h|auth=30m|rate_limited=120s|fallback=env",
        self_heal = "auth_signal_triggers_forced_refresh",
        "oauth account health policy active"
    );

    let snapshot_flush_store = store.clone();
    let snapshot_flush_interval_ms = snapshot_revision_flush_ms_from_env();
    let snapshot_flush_max_batch = snapshot_revision_max_batch_from_env();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(snapshot_flush_interval_ms));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if let Err(err) = snapshot_flush_store
                .flush_snapshot_revision(snapshot_flush_max_batch)
                .await
            {
                tracing::warn!(error = %err, "snapshot revision flush tick failed");
            }
        }
    });
    tracing::info!(
        interval_ms = snapshot_flush_interval_ms,
        max_batch = snapshot_flush_max_batch,
        "snapshot revision flush loop started"
    );

    let outbox_cleanup_store = store.clone();
    let outbox_cleanup_interval_sec = data_plane_outbox_cleanup_interval_sec_from_env();
    let outbox_retention_sec = data_plane_outbox_retention_sec_from_env();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(outbox_cleanup_interval_sec));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            match outbox_cleanup_store
                .cleanup_data_plane_outbox(chrono::Duration::seconds(
                    i64::try_from(outbox_retention_sec).unwrap_or(i64::MAX),
                ))
                .await
            {
                Ok(deleted) if deleted > 0 => {
                    tracing::info!(
                        deleted,
                        retention_sec = outbox_retention_sec,
                        "data-plane outbox cleanup deleted old events"
                    );
                }
                Ok(_) => {}
                Err(err) => {
                    tracing::warn!(error = %err, "data-plane outbox cleanup tick failed");
                }
            }
        }
    });
    tracing::info!(
        interval_sec = outbox_cleanup_interval_sec,
        retention_sec = outbox_retention_sec,
        "data-plane outbox cleanup loop started"
    );

    if billing_reconcile_enabled_from_env() {
        if let Some(pool) = store.postgres_pool() {
            let interval_sec = billing_reconcile_interval_sec_from_env();
            let batch = billing_reconcile_batch_from_env();
            let stale_sec = billing_reconcile_stale_sec_from_env();
            let lookback_sec = billing_reconcile_lookback_sec_from_env();
            let full_sweep_interval_sec = billing_reconcile_full_sweep_interval_sec_from_env();
            let request_log_reconcile_repo = config.clickhouse_url.as_ref().map(|clickhouse_url| {
                ClickHouseUsageRepo::new(
                    clickhouse_url,
                    &config.clickhouse_database,
                    &config.clickhouse_account_table,
                    &config.clickhouse_tenant_apikey_table,
                    &config.clickhouse_tenant_account_table,
                    &config.clickhouse_request_log_table,
                )
            });
            let request_log_reconcile_enabled = request_log_reconcile_repo.is_some();
            match TenantAuthService::from_pool(pool) {
                Ok(tenant_auth) => {
                    tokio::spawn(async move {
                        let mut ticker = tokio::time::interval(Duration::from_secs(interval_sec));
                        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
                        let mut cursor_created_at = chrono::Utc::now()
                            .timestamp()
                            .saturating_sub(lookback_sec as i64);
                        let mut cursor_id = String::new();
                        let mut last_full_sweep_started_at = std::time::Instant::now();
                        loop {
                            ticker.tick().await;
                            match tenant_auth
                                .billing_reconcile_once(BillingReconcileRequest {
                                    stale_sec,
                                    batch_size: batch,
                                })
                                .await
                            {
                                Ok(stats) => {
                                    record_billing_reconcile_runtime_stats(&stats);
                                    if stats.scanned > 0 {
                                        tracing::info!(
                                            scanned = stats.scanned,
                                            adjusted = stats.adjusted,
                                            released_authorizations = stats.released_authorizations,
                                            adjusted_microcredits_total =
                                                stats.adjusted_microcredits_total,
                                            "billing reconcile tick completed"
                                        );
                                    }
                                }
                                Err(err) => {
                                    record_billing_reconcile_runtime_failed();
                                    tracing::warn!(
                                        error = %err,
                                        interval_sec,
                                        batch,
                                        stale_sec,
                                        "billing reconcile tick failed"
                                    );
                                }
                            }

                            if let Some(repo) = request_log_reconcile_repo.as_ref() {
                                let now_ts = chrono::Utc::now().timestamp();
                                let lookback_start_ts = now_ts.saturating_sub(lookback_sec as i64);
                                let full_sweep_due = last_full_sweep_started_at.elapsed()
                                    >= Duration::from_secs(full_sweep_interval_sec);
                                if full_sweep_due {
                                    cursor_created_at = lookback_start_ts;
                                    cursor_id.clear();
                                    last_full_sweep_started_at = std::time::Instant::now();
                                } else if cursor_created_at < lookback_start_ts {
                                    cursor_created_at = lookback_start_ts;
                                    cursor_id.clear();
                                }

                                let mut local_cursor_created_at = cursor_created_at;
                                let mut local_cursor_id = cursor_id.clone();
                                let mut request_log_stats = BillingReconcileStats::default();
                                let mut fetch_failed = false;
                                loop {
                                    let facts = match repo
                                        .fetch_billing_reconcile_facts(
                                            lookback_start_ts,
                                            now_ts,
                                            local_cursor_created_at,
                                            &local_cursor_id,
                                            batch,
                                        )
                                        .await
                                    {
                                        Ok(facts) => facts,
                                        Err(err) => {
                                            fetch_failed = true;
                                            record_billing_reconcile_runtime_failed();
                                            tracing::warn!(
                                                error = %err,
                                                lookback_start_ts,
                                                now_ts,
                                                cursor_created_at = local_cursor_created_at,
                                                cursor_id = %local_cursor_id,
                                                batch,
                                                "billing request-log reconcile fetch failed"
                                            );
                                            break;
                                        }
                                    };
                                    if facts.is_empty() {
                                        break;
                                    }

                                    let fact_count = facts.len();
                                    for fact in facts.iter() {
                                        match tenant_auth
                                            .billing_reconcile_request_fact(
                                                BillingReconcileFactRequest {
                                                    tenant_id: fact.tenant_id,
                                                    api_key_id: fact.api_key_id,
                                                    request_id: fact.request_id.clone(),
                                                    model: fact.model.clone(),
                                                    input_tokens: fact.input_tokens,
                                                    cached_input_tokens: None,
                                                    output_tokens: fact.output_tokens,
                                                    reasoning_tokens: None,
                                                },
                                            )
                                            .await
                                        {
                                            Ok(stats) => {
                                                merge_reconcile_stats(
                                                    &mut request_log_stats,
                                                    &stats,
                                                );
                                            }
                                            Err(err) => {
                                                record_billing_reconcile_runtime_failed();
                                                tracing::warn!(
                                                    error = %err,
                                                    tenant_id = %fact.tenant_id,
                                                    request_id = %fact.request_id,
                                                    request_log_id = %fact.id,
                                                    status_code = fact.status_code,
                                                    billing_phase = ?fact.billing_phase,
                                                    capture_status = ?fact.capture_status,
                                                    "billing request-log reconcile apply failed"
                                                );
                                            }
                                        }
                                        local_cursor_created_at = fact.created_at.timestamp();
                                        local_cursor_id = fact.id.to_string();
                                    }

                                    if fact_count < batch {
                                        break;
                                    }
                                }

                                if request_log_stats.scanned > 0 {
                                    record_billing_reconcile_runtime_stats(&request_log_stats);
                                    tracing::info!(
                                        scanned = request_log_stats.scanned,
                                        adjusted = request_log_stats.adjusted,
                                        released_authorizations =
                                            request_log_stats.released_authorizations,
                                        adjusted_microcredits_total =
                                            request_log_stats.adjusted_microcredits_total,
                                        cursor_created_at = local_cursor_created_at,
                                        cursor_id = %local_cursor_id,
                                        "billing request-log reconcile tick completed"
                                    );
                                    cursor_created_at = local_cursor_created_at;
                                    cursor_id = local_cursor_id;
                                }

                                if fetch_failed {
                                    continue;
                                }

                                if request_log_stats.scanned == 0 {
                                    cursor_created_at = now_ts;
                                    cursor_id.clear();
                                }
                            }
                        }
                    });
                    tracing::info!(
                        interval_sec,
                        batch,
                        stale_sec,
                        lookback_sec,
                        full_sweep_interval_sec,
                        request_log_reconcile_enabled,
                        "billing reconcile loop started"
                    );
                }
                Err(err) => {
                    tracing::warn!(
                        error = %err,
                        "billing reconcile loop disabled: failed to initialize tenant auth service"
                    );
                }
            }
        } else {
            tracing::info!("billing reconcile loop skipped: postgres store unavailable");
        }
    } else {
        tracing::info!("billing reconcile loop disabled by config");
    }

    let listener = tokio::net::TcpListener::bind(config.listen_addr).await?;
    let codex_callback_listen_addr = codex_oauth_callback_listen_addr_from_env()?;
    let codex_callback_listen_mode = codex_oauth_callback_listen_mode_from_env();

    let usage_repo: Option<Arc<dyn UsageQueryRepository>> =
        config.clickhouse_url.clone().map(|clickhouse_url| {
            Arc::new(ClickHouseUsageRepo::new(
                &clickhouse_url,
                &config.clickhouse_database,
                &config.clickhouse_account_table,
                &config.clickhouse_tenant_apikey_table,
                &config.clickhouse_tenant_account_table,
                &config.clickhouse_request_log_table,
            )) as Arc<dyn UsageQueryRepository>
        });

    let app = build_app_with_store_ttl_usage_repo_import_store_and_admin_auth(
        store,
        config.auth_validate_cache_ttl_sec,
        usage_repo,
        import_job_store,
        admin_auth,
    );
    if codex_callback_listen_mode == CodexOAuthCallbackListenMode::Always {
        if let Some(callback_addr) = codex_callback_listen_addr {
            if callback_addr == config.listen_addr {
                tracing::info!(
                    listen_addr = %config.listen_addr,
                    "codex oauth callback listener reuses primary control-plane listener"
                );
                axum::serve(listener, app).await?;
                return Ok(());
            }
            let callback_listener = tokio::net::TcpListener::bind(callback_addr).await?;
            tracing::info!(
                listen_addr = %config.listen_addr,
                codex_callback_listen_addr = %callback_addr,
                "starting control-plane and codex oauth callback listeners"
            );
            let callback_app = app.clone();
            tokio::try_join!(
                axum::serve(listener, app),
                axum::serve(callback_listener, callback_app),
            )?;
            return Ok(());
        }
        tracing::info!(
            listen_addr = %config.listen_addr,
            "{} disabled or empty",
            CODEX_OAUTH_CALLBACK_LISTEN_ENV
        );
    } else {
        tracing::info!(
            listen_addr = %config.listen_addr,
            mode = ?codex_callback_listen_mode,
            "starting control-plane listener only; codex oauth callback listener is managed on-demand"
        );
    }
    axum::serve(listener, app).await?;
    Ok(())
}
