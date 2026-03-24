use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use codex_pool_core::api::ProductEdition;
use control_plane::admin_auth::AdminAuthService;
use control_plane::app::{
    build_app_with_store_and_services, codex_oauth_callback_listen_mode_from_env, AppBuildServices,
    CodexOAuthCallbackListenMode, DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC,
};
use control_plane::config::ControlPlaneConfig;
use control_plane::crypto::CredentialCipher;
use control_plane::import_jobs::{InMemoryOAuthImportJobStore, SqliteOAuthImportJobStore};
#[cfg(feature = "postgres-backend")]
use control_plane::import_jobs::PostgresOAuthImportJobStore;
use control_plane::oauth::OpenAiOAuthClient;
use control_plane::outbound_proxy_runtime::OutboundProxyRuntime;
use control_plane::runtime_profile::{
    resolve_backend_profile, BackendProfile, StoreBackendFamily, UsageIngestBackendFamily,
    UsageQueryBackendFamily,
};
use control_plane::single_binary::{
    apply_single_binary_runtime_env_defaults, merge_personal_single_binary_app,
    merge_single_binary_app,
};
#[cfg(feature = "postgres-backend")]
use control_plane::store::postgres::PostgresStore;
use control_plane::store::{
    normalize_sqlite_database_url, ControlPlaneStore, InMemoryStore, SqliteBackedStore,
};
#[cfg(feature = "clickhouse-backend")]
use control_plane::tenant::BillingReconcileFactRequest;
#[cfg(feature = "clickhouse-backend")]
use control_plane::tenant::BillingReconcileStats;
use control_plane::tenant::{
    record_billing_reconcile_runtime_failed, record_billing_reconcile_runtime_stats,
    BillingReconcileRequest, TenantAuthService,
};
#[cfg(feature = "clickhouse-backend")]
use control_plane::usage::clickhouse_repo::ClickHouseUsageRepo;
#[cfg(feature = "postgres-backend")]
use control_plane::usage::postgres_repo::PostgresUsageRepo;
use control_plane::usage::sqlite_repo::SqliteUsageRepo;
use control_plane::usage::{UsageIngestRepository, UsageQueryRepository};
use tokio::time::MissedTickBehavior;

const SNAPSHOT_REVISION_FLUSH_MS_ENV: &str = "CONTROL_PLANE_SNAPSHOT_REVISION_FLUSH_MS";
const SNAPSHOT_REVISION_MAX_BATCH_ENV: &str = "CONTROL_PLANE_SNAPSHOT_REVISION_MAX_BATCH";
const RATE_LIMIT_CACHE_REFRESH_ENABLED_ENV: &str = "CONTROL_PLANE_RATE_LIMIT_CACHE_REFRESH_ENABLED";
const RATE_LIMIT_CACHE_REFRESH_INTERVAL_SEC_ENV: &str =
    "CONTROL_PLANE_RATE_LIMIT_CACHE_REFRESH_INTERVAL_SEC";
const OAUTH_VAULT_ACTIVATE_ENABLED_ENV: &str = "CONTROL_PLANE_VAULT_ACTIVATE_ENABLED";
const OAUTH_VAULT_ACTIVATE_INTERVAL_SEC_ENV: &str = "CONTROL_PLANE_VAULT_ACTIVATE_INTERVAL_SEC";
const PENDING_PURGE_ENABLED_ENV: &str = "CONTROL_PLANE_PENDING_PURGE_ENABLED";
const PENDING_PURGE_INTERVAL_SEC_ENV: &str = "CONTROL_PLANE_PENDING_PURGE_INTERVAL_SEC";
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
const DEFAULT_PENDING_PURGE_ENABLED: bool = true;
const DEFAULT_PENDING_PURGE_INTERVAL_SEC: u64 = 30;
const MIN_PENDING_PURGE_INTERVAL_SEC: u64 = 5;
const MAX_PENDING_PURGE_INTERVAL_SEC: u64 = 3_600;
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
const CODEX_POOL_EDITION_ENV: &str = "CODEX_POOL_EDITION";
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

struct RuntimeStoreBundle {
    store: Arc<dyn ControlPlaneStore>,
    import_job_store: Arc<dyn control_plane::import_jobs::OAuthImportJobStore>,
    admin_auth: AdminAuthService,
    tenant_auth_service: Option<Arc<TenantAuthService>>,
    personal_runtime_store: Option<Arc<SqliteBackedStore>>,
    #[cfg(feature = "postgres-backend")]
    postgres_store: Option<Arc<PostgresStore>>,
}

struct RuntimeUsageBundle {
    usage_repo: Option<Arc<dyn UsageQueryRepository>>,
    usage_ingest_repo: Option<Arc<dyn UsageIngestRepository>>,
    personal_sqlite_usage_repo: Option<Arc<SqliteUsageRepo>>,
}

async fn build_store_bundle(
    profile: BackendProfile,
    database_url: Option<&str>,
    outbound_proxy_runtime: Arc<OutboundProxyRuntime>,
) -> anyhow::Result<RuntimeStoreBundle> {
    match (profile.store_backend, database_url) {
        (StoreBackendFamily::Sqlite, raw_database_url) => {
            let database_url = personal_sqlite_database_url(raw_database_url)?;
            let sqlite_store = Arc::new(
                SqliteBackedStore::connect_with_oauth(
                    &database_url,
                    Arc::new(OpenAiOAuthClient::from_env_with_outbound_proxy_runtime(
                        outbound_proxy_runtime,
                    )),
                    CredentialCipher::from_env().unwrap_or(None),
                )
                .await?,
            );
            Ok(RuntimeStoreBundle {
                store: sqlite_store.clone(),
                import_job_store: Arc::new(
                    SqliteOAuthImportJobStore::new(sqlite_store.clone_pool()).await?,
                ),
                admin_auth: AdminAuthService::from_env()?,
                tenant_auth_service: None,
                personal_runtime_store: Some(sqlite_store),
                #[cfg(feature = "postgres-backend")]
                postgres_store: None,
            })
        }
        #[cfg(feature = "postgres-backend")]
        (StoreBackendFamily::Postgres, Some(database_url)) => {
            let postgres_store = Arc::new(
                PostgresStore::connect_with_oauth(
                    database_url,
                    Arc::new(OpenAiOAuthClient::from_env_with_outbound_proxy_runtime(
                        outbound_proxy_runtime,
                    )),
                    CredentialCipher::from_env().unwrap_or(None),
                )
                .await?,
            );
            let import_store =
                PostgresOAuthImportJobStore::new(postgres_store.clone_pool()).await?;
            let tenant_auth_service = Arc::new(
                TenantAuthService::from_pool(postgres_store.clone_pool())
                    .expect("TENANT_JWT_SECRET (or ADMIN_JWT_SECRET fallback) must be set"),
            );
            let admin_auth = AdminAuthService::from_env_with_postgres(postgres_store.clone_pool())?;
            admin_auth.ensure_bootstrap_admin_user().await?;
            Ok(RuntimeStoreBundle {
                store: postgres_store.clone(),
                import_job_store: Arc::new(import_store),
                admin_auth,
                tenant_auth_service: Some(tenant_auth_service),
                personal_runtime_store: None,
                postgres_store: Some(postgres_store),
            })
        }
        #[cfg(not(feature = "postgres-backend"))]
        (StoreBackendFamily::Postgres, Some(_database_url)) => Err(anyhow!(
            "database-backed team/business edition requires the postgres-backend cargo feature"
        )),
        (StoreBackendFamily::Postgres, None) | (StoreBackendFamily::InMemory, _) => {
            let in_memory_store: Arc<dyn ControlPlaneStore> =
                Arc::new(InMemoryStore::new_with_oauth(
                    Arc::new(OpenAiOAuthClient::from_env_with_outbound_proxy_runtime(
                        outbound_proxy_runtime,
                    )),
                    CredentialCipher::from_env().unwrap_or(None),
                ));
            Ok(RuntimeStoreBundle {
                store: in_memory_store,
                import_job_store: Arc::new(InMemoryOAuthImportJobStore::default()),
                admin_auth: AdminAuthService::from_env()?,
                tenant_auth_service: None,
                personal_runtime_store: None,
                #[cfg(feature = "postgres-backend")]
                postgres_store: None,
            })
        }
    }
}

async fn build_usage_bundle(
    profile: BackendProfile,
    _config: &ControlPlaneConfig,
    personal_runtime_store: Option<&Arc<SqliteBackedStore>>,
    #[cfg(feature = "postgres-backend")] postgres_store: Option<&Arc<PostgresStore>>,
) -> anyhow::Result<RuntimeUsageBundle> {
    let personal_sqlite_usage_repo = if matches!(profile.store_backend, StoreBackendFamily::Sqlite)
    {
        match personal_runtime_store {
            Some(store) => Some(Arc::new(SqliteUsageRepo::new(store.clone_pool()).await?)),
            None => None,
        }
    } else {
        None
    };

    let usage_repo: Option<Arc<dyn UsageQueryRepository>> = match profile.usage_query_backend {
        UsageQueryBackendFamily::None => None,
        UsageQueryBackendFamily::Sqlite => personal_sqlite_usage_repo
            .clone()
            .map(|repo| repo as Arc<dyn UsageQueryRepository>),
        UsageQueryBackendFamily::Postgres => {
            #[cfg(feature = "postgres-backend")]
            {
                postgres_store.map(|store| {
                    Arc::new(PostgresUsageRepo::new(store.clone_pool()))
                        as Arc<dyn UsageQueryRepository>
                })
            }
            #[cfg(not(feature = "postgres-backend"))]
            {
                None
            }
        }
        UsageQueryBackendFamily::ClickHouse => {
            #[cfg(feature = "clickhouse-backend")]
            {
                _config.clickhouse_url.as_ref().map(|clickhouse_url| {
                    Arc::new(build_clickhouse_usage_repo(_config, clickhouse_url))
                        as Arc<dyn UsageQueryRepository>
                })
            }
            #[cfg(not(feature = "clickhouse-backend"))]
            {
                None
            }
        }
    };

    let usage_ingest_repo: Option<Arc<dyn UsageIngestRepository>> =
        match profile.usage_ingest_backend {
            UsageIngestBackendFamily::None => None,
            UsageIngestBackendFamily::Sqlite => personal_sqlite_usage_repo
                .clone()
                .map(|repo| repo as Arc<dyn UsageIngestRepository>),
            UsageIngestBackendFamily::Postgres => {
                #[cfg(feature = "postgres-backend")]
                {
                    postgres_store.map(|store| {
                        Arc::new(PostgresUsageRepo::new(store.clone_pool()))
                            as Arc<dyn UsageIngestRepository>
                    })
                }
                #[cfg(not(feature = "postgres-backend"))]
                {
                    None
                }
            }
        };

    Ok(RuntimeUsageBundle {
        usage_repo,
        usage_ingest_repo,
        personal_sqlite_usage_repo,
    })
}

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

fn pending_purge_enabled_from_env() -> bool {
    std::env::var(PENDING_PURGE_ENABLED_ENV)
        .ok()
        .and_then(|raw| {
            let normalized = raw.trim().to_ascii_lowercase();
            match normalized.as_str() {
                "1" | "true" | "yes" | "on" => Some(true),
                "0" | "false" | "no" | "off" => Some(false),
                _ => None,
            }
        })
        .unwrap_or(DEFAULT_PENDING_PURGE_ENABLED)
}

fn pending_purge_interval_sec_from_env() -> u64 {
    std::env::var(PENDING_PURGE_INTERVAL_SEC_ENV)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(DEFAULT_PENDING_PURGE_INTERVAL_SEC)
        .clamp(
            MIN_PENDING_PURGE_INTERVAL_SEC,
            MAX_PENDING_PURGE_INTERVAL_SEC,
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

#[cfg(feature = "clickhouse-backend")]
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

fn personal_sqlite_database_url(raw: Option<&str>) -> anyhow::Result<String> {
    if raw.is_some_and(|value| {
        let trimmed = value.trim().to_ascii_lowercase();
        trimmed.starts_with("postgres://") || trimmed.starts_with("postgresql://")
    }) {
        return Err(anyhow!(
            "personal edition requires sqlite storage, but CONTROL_PLANE_DATABASE_URL points to postgres"
        ));
    }

    Ok(normalize_sqlite_database_url(
        raw.unwrap_or("./codex-pool-personal.sqlite"),
    ))
}

#[cfg(feature = "clickhouse-backend")]
fn build_clickhouse_usage_repo(
    config: &ControlPlaneConfig,
    clickhouse_url: &str,
) -> ClickHouseUsageRepo {
    ClickHouseUsageRepo::new(
        clickhouse_url,
        &config.clickhouse_database,
        &config.clickhouse_account_table,
        &config.clickhouse_tenant_apikey_table,
        &config.clickhouse_tenant_account_table,
        &config.clickhouse_request_log_table,
    )
}

fn resolve_runtime_edition(
    env_value: Option<&str>,
    executable_name: Option<&str>,
) -> ProductEdition {
    ProductEdition::resolve_runtime_edition(env_value, executable_name)
}

fn runtime_edition() -> ProductEdition {
    let env_value = std::env::var(CODEX_POOL_EDITION_ENV).ok();
    let executable_name = std::env::args_os()
        .next()
        .and_then(|value| value.into_string().ok());
    resolve_runtime_edition(env_value.as_deref(), executable_name.as_deref())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    codex_pool_core::logging::init_local_tracing();

    let config = ControlPlaneConfig::from_env(DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC)?;
    config.apply_runtime_env_defaults();
    control_plane::security::ensure_api_key_hasher_configured()?;
    let edition = runtime_edition();
    let backend_profile = resolve_backend_profile(
        edition,
        config.database_url.is_some(),
        config.clickhouse_url.is_some(),
    );
    #[cfg(not(feature = "postgres-backend"))]
    if matches!(backend_profile.store_backend, StoreBackendFamily::Postgres) {
        return Err(anyhow!(
            "team/business edition requires the postgres-backend cargo feature"
        ));
    }
    #[cfg(not(feature = "clickhouse-backend"))]
    if matches!(
        backend_profile.usage_query_backend,
        UsageQueryBackendFamily::ClickHouse
    ) {
        return Err(anyhow!(
            "business edition requires the clickhouse-backend cargo feature"
        ));
    }
    if backend_profile.uses_single_binary_merge() {
        let defaults = apply_single_binary_runtime_env_defaults(config.listen_addr);
        tracing::info!(
            listen_addr = defaults.control_plane_listen,
            callback_listen = defaults.codex_oauth_callback_listen,
            control_plane_base_url = defaults.control_plane_base_url,
            auth_validate_url = defaults.auth_validate_url,
            edition = ?edition,
            "single-binary runtime defaults enforced for non-business edition"
        );
    }

    let outbound_proxy_runtime = Arc::new(OutboundProxyRuntime::new());

    let runtime_store_bundle = build_store_bundle(
        backend_profile,
        config.database_url.as_deref(),
        outbound_proxy_runtime.clone(),
    )
    .await?;

    let RuntimeStoreBundle {
        store,
        import_job_store,
        admin_auth,
        tenant_auth_service,
        personal_runtime_store,
        #[cfg(feature = "postgres-backend")]
        postgres_store,
    } = runtime_store_bundle;

    outbound_proxy_runtime.attach_store(store.clone());

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

    if pending_purge_enabled_from_env() {
        let purge_store = store.clone();
        let interval_sec = pending_purge_interval_sec_from_env();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(interval_sec));
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                ticker.tick().await;
                if let Err(err) = purge_store.purge_pending_upstream_accounts().await {
                    tracing::warn!(error = %err, "pending purge loop tick failed");
                }
            }
        });
        tracing::info!(interval_sec, "pending purge loop started");
    } else {
        tracing::info!("pending purge loop disabled by config");
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

    if backend_profile.billing_reconcile_enabled() && billing_reconcile_enabled_from_env() {
        if let Some(tenant_auth) = tenant_auth_service.clone() {
            let interval_sec = billing_reconcile_interval_sec_from_env();
            let batch = billing_reconcile_batch_from_env();
            let stale_sec = billing_reconcile_stale_sec_from_env();
            let lookback_sec = billing_reconcile_lookback_sec_from_env();
            let full_sweep_interval_sec = billing_reconcile_full_sweep_interval_sec_from_env();
            #[cfg(feature = "clickhouse-backend")]
            let request_log_reconcile_repo = config
                .clickhouse_url
                .as_ref()
                .map(|clickhouse_url| build_clickhouse_usage_repo(&config, clickhouse_url));
            #[cfg(feature = "clickhouse-backend")]
            let request_log_reconcile_enabled = request_log_reconcile_repo.is_some();
            #[cfg(not(feature = "clickhouse-backend"))]
            let request_log_reconcile_enabled = false;
            tokio::spawn(async move {
                let mut ticker = tokio::time::interval(Duration::from_secs(interval_sec));
                ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
                #[cfg(feature = "clickhouse-backend")]
                let mut cursor_created_at = chrono::Utc::now()
                    .timestamp()
                    .saturating_sub(lookback_sec as i64);
                #[cfg(feature = "clickhouse-backend")]
                let mut cursor_id = String::new();
                #[cfg(feature = "clickhouse-backend")]
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
                                    adjusted_microcredits_total = stats.adjusted_microcredits_total,
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

                    #[cfg(feature = "clickhouse-backend")]
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
                                    .billing_reconcile_request_fact(BillingReconcileFactRequest {
                                        tenant_id: fact.tenant_id,
                                        api_key_id: fact.api_key_id,
                                        request_id: fact.request_id.clone(),
                                        model: fact.model.clone(),
                                        service_tier: fact.service_tier.clone(),
                                        input_tokens: fact.input_tokens,
                                        cached_input_tokens: None,
                                        output_tokens: fact.output_tokens,
                                        reasoning_tokens: None,
                                    })
                                    .await
                                {
                                    Ok(stats) => {
                                        merge_reconcile_stats(&mut request_log_stats, &stats);
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
        } else {
            tracing::info!("billing reconcile loop skipped: tenant auth runtime unavailable");
        }
    } else if !backend_profile.billing_reconcile_enabled() {
        tracing::info!(
            edition = ?edition,
            "billing reconcile loop disabled outside business edition"
        );
    } else {
        tracing::info!("billing reconcile loop disabled by config");
    }
    let RuntimeUsageBundle {
        usage_repo,
        usage_ingest_repo,
        personal_sqlite_usage_repo,
    } = build_usage_bundle(
        backend_profile,
        &config,
        personal_runtime_store.as_ref(),
        #[cfg(feature = "postgres-backend")]
        postgres_store.as_ref(),
    )
    .await?;

    let app = build_app_with_store_and_services(
        store,
        AppBuildServices {
            auth_validate_cache_ttl_sec: config.auth_validate_cache_ttl_sec,
            usage_repo,
            usage_ingest_repo,
            import_job_store,
            admin_auth,
            system_capabilities: backend_profile.system_capabilities(),
            tenant_auth_service: tenant_auth_service.clone(),
            sqlite_usage_repo: personal_sqlite_usage_repo.clone(),
            outbound_proxy_runtime: outbound_proxy_runtime.clone(),
        },
    );
    let app = match edition {
        ProductEdition::Personal => {
            let sqlite_store =
                personal_runtime_store.expect("personal edition should keep sqlite runtime store");
            let usage_ingest_repo = personal_sqlite_usage_repo
                .clone()
                .expect("personal edition should keep sqlite usage ingest repo")
                as Arc<dyn UsageIngestRepository>;
            merge_personal_single_binary_app(app, sqlite_store, usage_ingest_repo).await?
        }
        ProductEdition::Team => merge_single_binary_app(app).await?,
        ProductEdition::Business => app,
    };
    let listener = tokio::net::TcpListener::bind(config.listen_addr).await?;
    let codex_callback_listen_addr = codex_oauth_callback_listen_addr_from_env()?;
    let codex_callback_listen_mode = codex_oauth_callback_listen_mode_from_env();
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

#[cfg(test)]
mod tests {
    use super::resolve_runtime_edition;
    use codex_pool_core::api::ProductEdition;
    use control_plane::runtime_profile::{
        resolve_backend_profile, BillingRuntimeMode, DeploymentShape, StoreBackendFamily,
        UsageIngestBackendFamily, UsageQueryBackendFamily,
    };

    #[test]
    fn billing_reconcile_runs_only_for_business_edition() {
        assert!(
            !resolve_backend_profile(ProductEdition::Personal, true, false)
                .billing_reconcile_enabled()
        );
        assert!(
            !resolve_backend_profile(ProductEdition::Team, true, false).billing_reconcile_enabled()
        );
        assert!(
            resolve_backend_profile(ProductEdition::Business, true, true)
                .billing_reconcile_enabled()
        );
    }

    #[test]
    fn runtime_edition_uses_env_before_binary_name() {
        assert_eq!(
            resolve_runtime_edition(Some("team"), Some("codex-pool-personal")),
            ProductEdition::Team
        );
    }

    #[test]
    fn runtime_edition_infers_product_binary_names() {
        assert_eq!(
            resolve_runtime_edition(None, Some("codex-pool-personal")),
            ProductEdition::Personal
        );
        assert_eq!(
            resolve_runtime_edition(None, Some("/tmp/codex-pool-team")),
            ProductEdition::Team
        );
        assert_eq!(
            resolve_runtime_edition(None, Some("codex-pool-business")),
            ProductEdition::Business
        );
    }

    #[test]
    fn runtime_edition_defaults_to_business_for_unknown_binary_name() {
        assert_eq!(
            resolve_runtime_edition(None, Some("control-plane")),
            ProductEdition::Business
        );
    }

    #[test]
    fn backend_profile_maps_personal_to_sqlite_single_binary() {
        let profile = resolve_backend_profile(ProductEdition::Personal, true, false);
        assert_eq!(profile.deployment_shape, DeploymentShape::SingleBinary);
        assert_eq!(profile.store_backend, StoreBackendFamily::Sqlite);
        assert_eq!(profile.usage_query_backend, UsageQueryBackendFamily::Sqlite);
        assert_eq!(
            profile.usage_ingest_backend,
            UsageIngestBackendFamily::Sqlite
        );
        assert_eq!(profile.billing_mode, BillingRuntimeMode::CostReportOnly);
        assert!(!profile.allows_tenant_self_service());
    }

    #[test]
    fn backend_profile_maps_team_to_postgres_single_binary() {
        let profile = resolve_backend_profile(ProductEdition::Team, true, false);
        assert_eq!(profile.deployment_shape, DeploymentShape::SingleBinary);
        assert_eq!(profile.store_backend, StoreBackendFamily::Postgres);
        assert_eq!(
            profile.usage_query_backend,
            UsageQueryBackendFamily::Postgres
        );
        assert_eq!(
            profile.usage_ingest_backend,
            UsageIngestBackendFamily::Postgres
        );
        assert_eq!(profile.billing_mode, BillingRuntimeMode::CostReportOnly);
        assert!(!profile.allows_tenant_self_service());
    }

    #[test]
    fn backend_profile_maps_business_to_full_stack_runtime() {
        let profile = resolve_backend_profile(ProductEdition::Business, true, true);
        assert_eq!(profile.deployment_shape, DeploymentShape::MultiService);
        assert_eq!(profile.store_backend, StoreBackendFamily::Postgres);
        assert_eq!(
            profile.usage_query_backend,
            UsageQueryBackendFamily::ClickHouse
        );
        assert_eq!(profile.usage_ingest_backend, UsageIngestBackendFamily::None);
        assert_eq!(profile.billing_mode, BillingRuntimeMode::CreditEnforced);
        assert!(profile.allows_tenant_self_service());
    }
}
