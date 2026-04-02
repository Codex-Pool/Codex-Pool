#[cfg(feature = "postgres-backend")]
use serde::de::DeserializeOwned;

use crate::edition_migration::{
    AccountAuthProviderMigrationRecord, AccountModelSupportMigrationRecord,
    ApiKeyTokenMigrationRecord, ControlPlaneMigrationBundle, OAuthCredentialMigrationRecord,
    SessionProfileMigrationRecord, UpstreamAccountHealthStateMigrationRecord,
};
#[cfg(feature = "postgres-backend")]
use crate::edition_migration::{
    EditionMigrationArchiveItem, EditionMigrationArchiveKind, EditionMigrationArchiveManifest,
};

#[cfg(feature = "postgres-backend")]
fn enum_string<T: ::serde::Serialize>(value: &T, label: &str) -> Result<String> {
    serde_json::to_value(value)
        .with_context(|| format!("failed to encode {label}"))?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| ::anyhow::anyhow!("{label} did not serialize to a string"))
}

#[cfg(feature = "postgres-backend")]
fn parse_enum<T: DeserializeOwned>(raw: &str, label: &str) -> Result<T> {
    serde_json::from_value(serde_json::Value::String(raw.to_string()))
        .with_context(|| format!("failed to decode {label}: {raw}"))
}

#[cfg(feature = "postgres-backend")]
fn archive_item(
    kind: EditionMigrationArchiveKind,
    description: &str,
    rows: Vec<serde_json::Value>,
) -> EditionMigrationArchiveItem {
    EditionMigrationArchiveItem {
        kind,
        count: rows.len() as u64,
        description: description.to_string(),
        rows,
    }
}

#[cfg(feature = "postgres-backend")]
async fn export_tenant_user_archive_rows(pool: &PgPool) -> Result<Vec<serde_json::Value>> {
    sqlx::query(
        r#"
        SELECT
            id,
            tenant_id,
            email,
            password_hash,
            email_verified,
            enabled,
            created_at,
            updated_at,
            last_login_at
        FROM tenant_users
        ORDER BY tenant_id ASC, email ASC, id ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to export tenant_users archive rows")?
    .into_iter()
    .map(|row| {
        Ok(serde_json::json!({
            "id": row.try_get::<Uuid, _>("id")?,
            "tenant_id": row.try_get::<Uuid, _>("tenant_id")?,
            "email": row.try_get::<String, _>("email")?,
            "password_hash": row.try_get::<String, _>("password_hash")?,
            "email_verified": row.try_get::<bool, _>("email_verified")?,
            "enabled": row.try_get::<bool, _>("enabled")?,
            "created_at": row.try_get::<DateTime<Utc>, _>("created_at")?,
            "updated_at": row.try_get::<DateTime<Utc>, _>("updated_at")?,
            "last_login_at": row.try_get::<Option<DateTime<Utc>>, _>("last_login_at")?,
        }))
    })
    .collect()
}

#[cfg(feature = "postgres-backend")]
async fn export_credit_account_archive_rows(pool: &PgPool) -> Result<Vec<serde_json::Value>> {
    sqlx::query(
        r#"
        SELECT tenant_id, balance_microcredits, updated_at
        FROM tenant_credit_accounts
        ORDER BY tenant_id ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to export tenant_credit_accounts archive rows")?
    .into_iter()
    .map(|row| {
        Ok(serde_json::json!({
            "tenant_id": row.try_get::<Uuid, _>("tenant_id")?,
            "balance_microcredits": row.try_get::<i64, _>("balance_microcredits")?,
            "updated_at": row.try_get::<DateTime<Utc>, _>("updated_at")?,
        }))
    })
    .collect()
}

#[cfg(feature = "postgres-backend")]
async fn export_credit_ledger_archive_rows(pool: &PgPool) -> Result<Vec<serde_json::Value>> {
    sqlx::query(
        r#"
        SELECT
            id,
            tenant_id,
            api_key_id,
            request_id,
            event_type,
            delta_microcredits,
            balance_after_microcredits,
            unit_price_microcredits,
            input_tokens,
            output_tokens,
            model,
            meta_json,
            created_at
        FROM tenant_credit_ledger
        ORDER BY tenant_id ASC, created_at ASC, id ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to export tenant_credit_ledger archive rows")?
    .into_iter()
    .map(|row| {
        Ok(serde_json::json!({
            "id": row.try_get::<Uuid, _>("id")?,
            "tenant_id": row.try_get::<Uuid, _>("tenant_id")?,
            "api_key_id": row.try_get::<Option<Uuid>, _>("api_key_id")?,
            "request_id": row.try_get::<Option<String>, _>("request_id")?,
            "event_type": row.try_get::<String, _>("event_type")?,
            "delta_microcredits": row.try_get::<i64, _>("delta_microcredits")?,
            "balance_after_microcredits": row.try_get::<i64, _>("balance_after_microcredits")?,
            "unit_price_microcredits": row.try_get::<Option<i64>, _>("unit_price_microcredits")?,
            "input_tokens": row.try_get::<Option<i64>, _>("input_tokens")?,
            "output_tokens": row.try_get::<Option<i64>, _>("output_tokens")?,
            "model": row.try_get::<Option<String>, _>("model")?,
            "meta_json": row.try_get::<serde_json::Value, _>("meta_json")?,
            "created_at": row.try_get::<DateTime<Utc>, _>("created_at")?,
        }))
    })
    .collect()
}

#[cfg(feature = "postgres-backend")]
async fn export_credit_authorization_archive_rows(pool: &PgPool) -> Result<Vec<serde_json::Value>> {
    sqlx::query(
        r#"
        SELECT
            id,
            tenant_id,
            api_key_id,
            request_id,
            model,
            reserved_microcredits,
            captured_microcredits,
            status,
            expires_at,
            meta_json,
            created_at,
            updated_at
        FROM tenant_credit_authorizations
        ORDER BY tenant_id ASC, created_at ASC, id ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to export tenant_credit_authorizations archive rows")?
    .into_iter()
    .map(|row| {
        Ok(serde_json::json!({
            "id": row.try_get::<Uuid, _>("id")?,
            "tenant_id": row.try_get::<Uuid, _>("tenant_id")?,
            "api_key_id": row.try_get::<Option<Uuid>, _>("api_key_id")?,
            "request_id": row.try_get::<String, _>("request_id")?,
            "model": row.try_get::<Option<String>, _>("model")?,
            "reserved_microcredits": row.try_get::<i64, _>("reserved_microcredits")?,
            "captured_microcredits": row.try_get::<i64, _>("captured_microcredits")?,
            "status": row.try_get::<String, _>("status")?,
            "expires_at": row.try_get::<DateTime<Utc>, _>("expires_at")?,
            "meta_json": row.try_get::<serde_json::Value, _>("meta_json")?,
            "created_at": row.try_get::<DateTime<Utc>, _>("created_at")?,
            "updated_at": row.try_get::<DateTime<Utc>, _>("updated_at")?,
        }))
    })
    .collect()
}

fn sort_bundle(bundle: &mut ControlPlaneMigrationBundle) {
    bundle.tenants.sort_by_key(|item| item.id);
    bundle.api_keys.sort_by_key(|item| item.id);
    bundle.api_key_tokens.sort_by(|left, right| {
        left.api_key_id
            .cmp(&right.api_key_id)
            .then_with(|| left.token.cmp(&right.token))
    });
    bundle.accounts.sort_by_key(|item| item.id);
    bundle
        .account_auth_providers
        .sort_by_key(|item| item.account_id);
    bundle.oauth_credentials.sort_by_key(|item| item.account_id);
    bundle.session_profiles.sort_by_key(|item| item.account_id);
    bundle
        .account_health_states
        .sort_by_key(|item| item.account_id);
    bundle
        .account_model_support
        .sort_by_key(|item| item.account_id);
    bundle.routing_policies.sort_by_key(|item| item.tenant_id);
    bundle.routing_profiles.sort_by_key(|item| item.id);
    bundle.model_routing_policies.sort_by_key(|item| item.id);
    bundle.outbound_proxy_nodes.sort_by_key(|item| item.id);
    bundle
        .upstream_error_templates
        .sort_by_key(|item| (item.provider.clone(), item.fingerprint.clone()));
    bundle
        .builtin_error_template_overrides
        .sort_by_key(|item| (item.kind, item.code.clone()));
    bundle
        .routing_plan_versions
        .sort_by_key(|item| (item.published_at, item.id));
}

fn sqlite_state_to_bundle(state: SqlitePersistedStoreState) -> ControlPlaneMigrationBundle {
    let mut bundle = ControlPlaneMigrationBundle {
        tenants: state.tenants.into_values().collect(),
        api_keys: state.api_keys.into_values().collect(),
        api_key_tokens: state
            .api_key_tokens
            .into_iter()
            .map(|(token, api_key_id)| ApiKeyTokenMigrationRecord { token, api_key_id })
            .collect(),
        accounts: state.accounts.into_values().collect(),
        account_auth_providers: state
            .account_auth_providers
            .into_iter()
            .map(
                |(account_id, auth_provider)| AccountAuthProviderMigrationRecord {
                    account_id,
                    auth_provider,
                },
            )
            .collect(),
        oauth_credentials: state
            .oauth_credentials
            .into_iter()
            .map(|(account_id, record)| OAuthCredentialMigrationRecord {
                account_id,
                access_token_enc: record.access_token_enc,
                fallback_access_token_enc: record.fallback_access_token_enc,
                refresh_token_enc: record.refresh_token_enc,
                refresh_token_sha256: record.refresh_token_sha256,
                token_family_id: record.token_family_id,
                token_version: record.token_version,
                token_expires_at: record.token_expires_at,
                fallback_token_expires_at: record.fallback_token_expires_at,
                last_refresh_at: record.last_refresh_at,
                last_refresh_status: record.last_refresh_status,
                refresh_reused_detected: record.refresh_reused_detected,
                last_refresh_error_code: record.last_refresh_error_code,
                last_refresh_error: record.last_refresh_error,
                refresh_failure_count: record.refresh_failure_count,
                refresh_backoff_until: record.refresh_backoff_until,
            })
            .collect(),
        session_profiles: state
            .session_profiles
            .into_iter()
            .map(|(account_id, record)| SessionProfileMigrationRecord {
                account_id,
                credential_kind: record.credential_kind,
                token_expires_at: record.token_expires_at,
                email: record.email,
                oauth_subject: record.oauth_subject,
                oauth_identity_provider: record.oauth_identity_provider,
                email_verified: record.email_verified,
                chatgpt_plan_type: record.chatgpt_plan_type,
                chatgpt_user_id: record.chatgpt_user_id,
                chatgpt_subscription_active_start: record.chatgpt_subscription_active_start,
                chatgpt_subscription_active_until: record.chatgpt_subscription_active_until,
                chatgpt_subscription_last_checked: record.chatgpt_subscription_last_checked,
                chatgpt_account_user_id: record.chatgpt_account_user_id,
                chatgpt_compute_residency: record.chatgpt_compute_residency,
                workspace_name: record.workspace_name,
                organizations: record.organizations,
                groups: record.groups,
                source_type: record.source_type,
            })
            .collect(),
        account_health_states: state
            .account_health_states
            .into_iter()
            .map(
                |(account_id, record)| UpstreamAccountHealthStateMigrationRecord {
                    account_id,
                    seen_ok_at: record.seen_ok_at,
                },
            )
            .collect(),
        account_model_support: state
            .account_model_support
            .into_iter()
            .map(|(account_id, record)| AccountModelSupportMigrationRecord {
                account_id,
                supported_models: record.supported_models,
            })
            .collect(),
        routing_policies: state.policies.into_values().collect(),
        routing_profiles: state.routing_profiles.into_values().collect(),
        model_routing_policies: state.model_routing_policies.into_values().collect(),
        model_routing_settings: Some(state.model_routing_settings),
        claude_code_routing_settings: Some(state.claude_code_routing_settings),
        outbound_proxy_pool_settings: Some(state.outbound_proxy_pool_settings),
        outbound_proxy_nodes: state.outbound_proxy_nodes.into_values().collect(),
        upstream_error_learning_settings: Some(state.upstream_error_learning_settings),
        upstream_error_templates: state.upstream_error_templates,
        builtin_error_template_overrides: state.builtin_error_template_overrides,
        routing_plan_versions: state.routing_plan_versions,
        revision: state.revision.max(1),
    };
    sort_bundle(&mut bundle);
    bundle
}

fn bundle_to_sqlite_state(bundle: &ControlPlaneMigrationBundle) -> SqlitePersistedStoreState {
    SqlitePersistedStoreState {
        tenants: bundle
            .tenants
            .iter()
            .cloned()
            .map(|item| (item.id, item))
            .collect(),
        api_keys: bundle
            .api_keys
            .iter()
            .cloned()
            .map(|item| (item.id, item))
            .collect(),
        api_key_tokens: bundle
            .api_key_tokens
            .iter()
            .cloned()
            .map(|item| (item.token, item.api_key_id))
            .collect(),
        accounts: bundle
            .accounts
            .iter()
            .cloned()
            .map(|item| (item.id, item))
            .collect(),
        account_auth_providers: bundle
            .account_auth_providers
            .iter()
            .cloned()
            .map(|item| (item.account_id, item.auth_provider))
            .collect(),
        oauth_credentials: bundle
            .oauth_credentials
            .iter()
            .cloned()
            .map(|item| {
                (
                    item.account_id,
                    OAuthCredentialRecord {
                        access_token_enc: item.access_token_enc,
                        fallback_access_token_enc: item.fallback_access_token_enc,
                        refresh_token_enc: item.refresh_token_enc,
                        refresh_token_sha256: item.refresh_token_sha256,
                        token_family_id: item.token_family_id,
                        token_version: item.token_version,
                        token_expires_at: item.token_expires_at,
                        fallback_token_expires_at: item.fallback_token_expires_at,
                        last_refresh_at: item.last_refresh_at,
                        last_refresh_status: item.last_refresh_status,
                        refresh_reused_detected: item.refresh_reused_detected,
                        last_refresh_error_code: item.last_refresh_error_code,
                        last_refresh_error: item.last_refresh_error,
                        refresh_failure_count: item.refresh_failure_count,
                        refresh_backoff_until: item.refresh_backoff_until,
                    },
                )
            })
            .collect(),
        oauth_refresh_token_vault: HashMap::new(),
        session_profiles: bundle
            .session_profiles
            .iter()
            .cloned()
            .map(|item| {
                (
                    item.account_id,
                    SessionProfileRecord {
                        credential_kind: item.credential_kind,
                        token_expires_at: item.token_expires_at,
                        email: item.email,
                        oauth_subject: item.oauth_subject,
                        oauth_identity_provider: item.oauth_identity_provider,
                        email_verified: item.email_verified,
                        chatgpt_plan_type: item.chatgpt_plan_type,
                        chatgpt_user_id: item.chatgpt_user_id,
                        chatgpt_subscription_active_start: item.chatgpt_subscription_active_start,
                        chatgpt_subscription_active_until: item.chatgpt_subscription_active_until,
                        chatgpt_subscription_last_checked: item.chatgpt_subscription_last_checked,
                        chatgpt_account_user_id: item.chatgpt_account_user_id,
                        chatgpt_compute_residency: item.chatgpt_compute_residency,
                        workspace_name: item.workspace_name,
                        organizations: item.organizations,
                        groups: item.groups,
                        source_type: item.source_type,
                    },
                )
            })
            .collect(),
        account_health_states: bundle
            .account_health_states
            .iter()
            .map(|item| {
                (
                    item.account_id,
                    UpstreamAccountHealthStateRecord {
                        seen_ok_at: item.seen_ok_at,
                        ..Default::default()
                    },
                )
            })
            .collect(),
        account_model_support: bundle
            .account_model_support
            .iter()
            .cloned()
            .map(|item| {
                (
                    item.account_id,
                    AccountModelSupportRecord {
                        supported_models: item.supported_models,
                    },
                )
            })
            .collect(),
        oauth_rate_limit_caches: HashMap::new(),
        oauth_rate_limit_refresh_jobs: HashMap::new(),
        policies: bundle
            .routing_policies
            .iter()
            .cloned()
            .map(|item| (item.tenant_id, item))
            .collect(),
        routing_profiles: bundle
            .routing_profiles
            .iter()
            .cloned()
            .map(|item| (item.id, item))
            .collect(),
        model_routing_policies: bundle
            .model_routing_policies
            .iter()
            .cloned()
            .map(|item| (item.id, item))
            .collect(),
        model_routing_settings: bundle
            .model_routing_settings
            .clone()
            .unwrap_or_else(crate::edition_migration::default_model_routing_settings),
        claude_code_routing_settings: bundle
            .claude_code_routing_settings
            .clone()
            .unwrap_or_else(crate::edition_migration::default_claude_code_routing_settings),
        outbound_proxy_pool_settings: bundle
            .outbound_proxy_pool_settings
            .clone()
            .unwrap_or_default(),
        outbound_proxy_nodes: bundle
            .outbound_proxy_nodes
            .iter()
            .cloned()
            .map(|item| (item.id, item))
            .collect(),
        upstream_error_learning_settings: bundle
            .upstream_error_learning_settings
            .clone()
            .unwrap_or_default(),
        upstream_error_templates: bundle.upstream_error_templates.clone(),
        builtin_error_template_overrides: bundle.builtin_error_template_overrides.clone(),
        routing_plan_versions: bundle.routing_plan_versions.clone(),
        revision: bundle.revision.max(1),
    }
}

async fn ensure_sqlite_target_empty(pool: &sqlx_sqlite::SqlitePool) -> Result<()> {
    let existing = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM control_plane_state")
        .fetch_one(pool)
        .await
        .context("failed to check sqlite control_plane_state emptiness")?;
    if existing > 0 {
        ::anyhow::bail!("sqlite target already contains control-plane state");
    }
    Ok(())
}

#[cfg(feature = "postgres-backend")]
async fn ensure_postgres_target_empty(pool: &sqlx_postgres::PgPool) -> Result<()> {
    let mut occupied = Vec::new();
    for (label, table_name) in [
        ("tenants", "tenants"),
        ("api_keys", "api_keys"),
        ("upstream_accounts", "upstream_accounts"),
        ("routing_profiles", "routing_profiles"),
        ("model_routing_policies", "model_routing_policies"),
        ("outbound_proxy_nodes", "outbound_proxy_nodes"),
    ] {
        let sql = format!("SELECT COUNT(*) FROM {table_name}");
        let count: i64 = sqlx::query_scalar(&sql)
            .fetch_one(pool)
            .await
            .with_context(|| format!("failed to inspect postgres table {table_name}"))?;
        if count > 0 {
            occupied.push(format!("{label}={count}"));
        }
    }
    if !occupied.is_empty() {
        ::anyhow::bail!("postgres target is not empty: {}", occupied.join(", "));
    }
    Ok(())
}

impl SqliteBackedStore {
    pub async fn export_migration_bundle(&self) -> Result<ControlPlaneMigrationBundle> {
        Ok(sqlite_state_to_bundle(self.inner.export_sqlite_state()))
    }

    pub async fn import_migration_bundle(
        database_url: &str,
        bundle: &ControlPlaneMigrationBundle,
    ) -> Result<()> {
        let normalized = normalize_sqlite_database_url(database_url);
        let pool = sqlx_core::pool::PoolOptions::new()
            .max_connections(1)
            .connect(&normalized)
            .await
            .with_context(|| format!("failed to connect sqlite store at {normalized}"))?;
        Self::initialize_schema(&pool).await?;
        ensure_sqlite_target_empty(&pool).await?;

        let state_json = serde_json::to_string(&bundle_to_sqlite_state(bundle))
            .context("failed to encode sqlite migration state")?;
        let updated_at = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO control_plane_state (id, schema_version, state_json, updated_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(SQLITE_STORE_STATE_ROW_ID)
        .bind(SQLITE_STORE_STATE_VERSION)
        .bind(state_json)
        .bind(updated_at)
        .execute(&pool)
        .await
        .context("failed to import sqlite control-plane state")?;

        Ok(())
    }
}

#[cfg(feature = "postgres-backend")]
impl postgres::PostgresStore {
    pub async fn export_migration_bundle(&self) -> Result<ControlPlaneMigrationBundle> {
        let pool = self.clone_pool();

        let tenants = ControlPlaneStore::list_tenants(self).await?;
        let api_keys = ControlPlaneStore::list_api_keys(self).await?;
        let accounts = ControlPlaneStore::list_upstream_accounts(self).await?;
        let routing_policies = sqlx::query(
            "SELECT tenant_id, strategy, max_retries, stream_max_retries, updated_at FROM routing_policies ORDER BY tenant_id ASC",
        )
        .fetch_all(&pool)
        .await
        .context("failed to export routing_policies")?
        .into_iter()
        .map(|row| {
            let strategy_raw: String = row.try_get("strategy")?;
            Ok(RoutingPolicy {
                tenant_id: row.try_get("tenant_id")?,
                strategy: parse_enum(&strategy_raw, "routing strategy")?,
                max_retries: u32::try_from(row.try_get::<i64, _>("max_retries")?)
                    .context("max_retries out of range")?,
                stream_max_retries: u32::try_from(
                    row.try_get::<i64, _>("stream_max_retries")?,
                )
                .context("stream_max_retries out of range")?,
                updated_at: row.try_get("updated_at")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
        let routing_profiles = ControlPlaneStore::list_routing_profiles(self).await?;
        let model_routing_policies = ControlPlaneStore::list_model_routing_policies(self).await?;
        let model_routing_settings = Some(ControlPlaneStore::model_routing_settings(self).await?);
        let claude_code_routing_settings =
            Some(ControlPlaneStore::claude_code_routing_settings(self).await?);
        let outbound_proxy_pool_settings = Some(self.outbound_proxy_pool_settings().await?);
        let outbound_proxy_nodes = self.list_outbound_proxy_nodes().await?;
        let upstream_error_learning_settings = Some(self.upstream_error_learning_settings().await?);
        let upstream_error_templates = self.list_upstream_error_templates(None).await?;
        let builtin_error_template_overrides = self.list_builtin_error_template_overrides().await?;
        let routing_plan_versions = self.list_routing_plan_versions().await?;

        let api_key_tokens = sqlx::query(
            "SELECT token, api_key_id FROM api_key_tokens ORDER BY api_key_id ASC, token ASC",
        )
        .fetch_all(&pool)
        .await
        .context("failed to export api_key_tokens")?
        .into_iter()
        .map(|row| {
            Ok(ApiKeyTokenMigrationRecord {
                token: row.try_get("token")?,
                api_key_id: row.try_get("api_key_id")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;

        let account_auth_providers =
            sqlx::query("SELECT id, auth_provider FROM upstream_accounts ORDER BY id ASC")
                .fetch_all(&pool)
                .await
                .context("failed to export upstream account auth providers")?
                .into_iter()
                .map(|row| {
                    let account_id: uuid::Uuid = row.try_get("id")?;
                    let raw: String = row.try_get("auth_provider")?;
                    Ok(AccountAuthProviderMigrationRecord {
                        account_id,
                        auth_provider: parse_enum(&raw, "upstream account auth provider")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;

        let oauth_credentials = sqlx::query(
            r#"
            SELECT
                account_id,
                access_token_enc,
                fallback_access_token_enc,
                refresh_token_enc,
                refresh_token_sha256,
                token_family_id,
                token_version,
                token_expires_at,
                fallback_token_expires_at,
                last_refresh_at,
                last_refresh_status,
                refresh_reused_detected,
                last_refresh_error_code,
                last_refresh_error,
                refresh_failure_count,
                refresh_backoff_until
            FROM upstream_account_oauth_credentials
            ORDER BY account_id ASC
            "#,
        )
        .fetch_all(&pool)
        .await
        .context("failed to export oauth credentials")?
        .into_iter()
        .map(|row| {
            let account_id: uuid::Uuid = row.try_get("account_id")?;
            let status_raw: String = row.try_get("last_refresh_status")?;
            Ok(OAuthCredentialMigrationRecord {
                account_id,
                access_token_enc: row.try_get("access_token_enc")?,
                fallback_access_token_enc: row.try_get("fallback_access_token_enc")?,
                refresh_token_enc: row.try_get("refresh_token_enc")?,
                refresh_token_sha256: row.try_get("refresh_token_sha256")?,
                token_family_id: row
                    .try_get::<Option<String>, _>("token_family_id")?
                    .unwrap_or_else(|| account_id.to_string()),
                token_version: u64::try_from(row.try_get::<i64, _>("token_version")?)
                    .context("token_version out of range")?,
                token_expires_at: row.try_get("token_expires_at")?,
                fallback_token_expires_at: row.try_get("fallback_token_expires_at")?,
                last_refresh_at: row.try_get("last_refresh_at")?,
                last_refresh_status: parse_enum(&status_raw, "oauth refresh status")?,
                refresh_reused_detected: row.try_get("refresh_reused_detected")?,
                last_refresh_error_code: row.try_get("last_refresh_error_code")?,
                last_refresh_error: row.try_get("last_refresh_error")?,
                refresh_failure_count: u32::try_from(
                    row.try_get::<i32, _>("refresh_failure_count")?,
                )
                .context("refresh_failure_count out of range")?,
                refresh_backoff_until: row.try_get("refresh_backoff_until")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;

        let session_profiles = sqlx::query(
            r#"
            SELECT
                account_id,
                credential_kind,
                token_expires_at,
                email,
                oauth_subject,
                oauth_identity_provider,
                email_verified,
                chatgpt_plan_type,
                chatgpt_user_id,
                chatgpt_subscription_active_start,
                chatgpt_subscription_active_until,
                chatgpt_subscription_last_checked,
                chatgpt_account_user_id,
                chatgpt_compute_residency,
                workspace_name,
                organizations_json::text AS organizations_json_text,
                groups_json::text AS groups_json_text,
                source_type
            FROM upstream_account_session_profiles
            ORDER BY account_id ASC
            "#,
        )
        .fetch_all(&pool)
        .await
        .context("failed to export session profiles")?
        .into_iter()
        .map(|row| {
            let kind_raw: String = row.try_get("credential_kind")?;
            Ok(SessionProfileMigrationRecord {
                account_id: row.try_get("account_id")?,
                credential_kind: parse_enum(&kind_raw, "session credential kind")?,
                token_expires_at: row.try_get("token_expires_at")?,
                email: row.try_get("email")?,
                oauth_subject: row.try_get("oauth_subject")?,
                oauth_identity_provider: row.try_get("oauth_identity_provider")?,
                email_verified: row.try_get("email_verified")?,
                chatgpt_plan_type: row.try_get("chatgpt_plan_type")?,
                chatgpt_user_id: row.try_get("chatgpt_user_id")?,
                chatgpt_subscription_active_start: row
                    .try_get("chatgpt_subscription_active_start")?,
                chatgpt_subscription_active_until: row
                    .try_get("chatgpt_subscription_active_until")?,
                chatgpt_subscription_last_checked: row
                    .try_get("chatgpt_subscription_last_checked")?,
                chatgpt_account_user_id: row.try_get("chatgpt_account_user_id")?,
                chatgpt_compute_residency: row.try_get("chatgpt_compute_residency")?,
                workspace_name: row.try_get("workspace_name")?,
                organizations: row
                    .try_get::<Option<String>, _>("organizations_json_text")?
                    .map(|raw| serde_json::from_str(&raw))
                    .transpose()
                    .context("failed to decode organizations_json")?,
                groups: row
                    .try_get::<Option<String>, _>("groups_json_text")?
                    .map(|raw| serde_json::from_str(&raw))
                    .transpose()
                    .context("failed to decode groups_json")?,
                source_type: row.try_get("source_type")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;

        let account_health_states = sqlx::query(
            "SELECT account_id, seen_ok_at FROM upstream_account_health_state ORDER BY account_id ASC",
        )
        .fetch_all(&pool)
        .await
        .context("failed to export upstream account health state")?
        .into_iter()
        .map(|row| {
            Ok(UpstreamAccountHealthStateMigrationRecord {
                account_id: row.try_get("account_id")?,
                seen_ok_at: row.try_get("seen_ok_at")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;

        let account_model_support = sqlx::query(
            r#"
            SELECT account_id, supported_models_json::text AS supported_models_json_text
            FROM upstream_account_model_support
            ORDER BY account_id ASC
            "#,
        )
        .fetch_all(&pool)
        .await
        .context("failed to export account model support")?
        .into_iter()
        .map(|row| {
            let supported_models = serde_json::from_str::<Vec<String>>(
                &row.try_get::<String, _>("supported_models_json_text")?,
            )
            .context("failed to decode supported_models_json")?;
            Ok(AccountModelSupportMigrationRecord {
                account_id: row.try_get("account_id")?,
                supported_models,
            })
        })
        .collect::<Result<Vec<_>>>()?;

        let revision = sqlx::query_scalar::<_, i64>(
            "SELECT revision FROM snapshot_state WHERE singleton = true",
        )
        .fetch_optional(&pool)
        .await
        .context("failed to export snapshot revision")?
        .unwrap_or(1)
        .max(1) as u64;

        let mut bundle = ControlPlaneMigrationBundle {
            tenants,
            api_keys,
            api_key_tokens,
            accounts,
            account_auth_providers,
            oauth_credentials,
            session_profiles,
            account_health_states,
            account_model_support,
            routing_policies,
            routing_profiles,
            model_routing_policies,
            model_routing_settings,
            claude_code_routing_settings,
            outbound_proxy_pool_settings,
            outbound_proxy_nodes,
            upstream_error_learning_settings,
            upstream_error_templates,
            builtin_error_template_overrides,
            routing_plan_versions,
            revision,
        };
        sort_bundle(&mut bundle);
        Ok(bundle)
    }

    pub async fn export_archive_manifest(
        &self,
        edition: codex_pool_core::api::ProductEdition,
    ) -> Result<EditionMigrationArchiveManifest> {
        let pool = self.clone_pool();
        let mut items = Vec::new();

        if matches!(
            edition,
            codex_pool_core::api::ProductEdition::Team
                | codex_pool_core::api::ProductEdition::Business
        ) {
            items.push(archive_item(
                EditionMigrationArchiveKind::TenantUsers,
                "租户登录/会话相关数据",
                export_tenant_user_archive_rows(&pool).await?,
            ));
        }
        if edition == codex_pool_core::api::ProductEdition::Business {
            items.push(archive_item(
                EditionMigrationArchiveKind::TenantCreditAccounts,
                "信用账户主表",
                export_credit_account_archive_rows(&pool).await?,
            ));
            items.push(archive_item(
                EditionMigrationArchiveKind::TenantCreditLedger,
                "信用账本流水",
                export_credit_ledger_archive_rows(&pool).await?,
            ));
            items.push(archive_item(
                EditionMigrationArchiveKind::TenantCreditAuthorizations,
                "信用授权与 capture/release 记录",
                export_credit_authorization_archive_rows(&pool).await?,
            ));
        }

        Ok(EditionMigrationArchiveManifest { items })
    }

    pub async fn import_migration_bundle(
        database_url: &str,
        bundle: &ControlPlaneMigrationBundle,
    ) -> Result<()> {
        let store = Self::connect(database_url).await?;
        let pool = store.clone_pool();
        ensure_postgres_target_empty(&pool).await?;

        let mut tx = pool
            .begin()
            .await
            .context("failed to start postgres migration transaction")?;

        for tenant in &bundle.tenants {
            sqlx::query(
                r#"
                INSERT INTO tenants (id, name, created_at)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(tenant.id)
            .bind(&tenant.name)
            .bind(tenant.created_at)
            .execute(tx.as_mut())
            .await
            .context("failed to import tenant")?;
        }

        for api_key in &bundle.api_keys {
            sqlx::query(
                r#"
                INSERT INTO api_keys (id, tenant_id, name, key_prefix, key_hash, enabled, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
            )
            .bind(api_key.id)
            .bind(api_key.tenant_id)
            .bind(&api_key.name)
            .bind(&api_key.key_prefix)
            .bind(&api_key.key_hash)
            .bind(api_key.enabled)
            .bind(api_key.created_at)
            .execute(tx.as_mut())
            .await
            .context("failed to import api key")?;
        }

        for token in &bundle.api_key_tokens {
            sqlx::query(
                r#"
                INSERT INTO api_key_tokens (token, api_key_id)
                VALUES ($1, $2)
                "#,
            )
            .bind(&token.token)
            .bind(token.api_key_id)
            .execute(tx.as_mut())
            .await
            .context("failed to import api key token")?;
        }

        let account_auth_providers = bundle
            .account_auth_providers
            .iter()
            .map(|item| (item.account_id, item.auth_provider.clone()))
            .collect::<HashMap<_, _>>();

        for account in &bundle.accounts {
            let auth_provider = account_auth_providers
                .get(&account.id)
                .cloned()
                .unwrap_or(UpstreamAuthProvider::LegacyBearer);
            sqlx::query(
                r#"
                INSERT INTO upstream_accounts (
                    id, label, mode, base_url, bearer_token, chatgpt_account_id,
                    enabled, auth_provider, pool_state, priority, created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'active', $9, $10)
                "#,
            )
            .bind(account.id)
            .bind(&account.label)
            .bind(enum_string(&account.mode, "upstream mode")?)
            .bind(&account.base_url)
            .bind(&account.bearer_token)
            .bind(&account.chatgpt_account_id)
            .bind(account.enabled)
            .bind(enum_string(&auth_provider, "upstream auth provider")?)
            .bind(account.priority)
            .bind(account.created_at)
            .execute(tx.as_mut())
            .await
            .context("failed to import upstream account")?;
        }

        for credential in &bundle.oauth_credentials {
            sqlx::query(
                r#"
                INSERT INTO upstream_account_oauth_credentials (
                    account_id,
                    access_token_enc,
                    fallback_access_token_enc,
                    refresh_token_enc,
                    refresh_token_sha256,
                    token_family_id,
                    token_version,
                    token_expires_at,
                    fallback_token_expires_at,
                    last_refresh_at,
                    last_refresh_status,
                    refresh_reused_detected,
                    last_refresh_error_code,
                    last_refresh_error,
                    refresh_failure_count,
                    refresh_backoff_until,
                    updated_at
                )
                VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17
                )
                "#,
            )
            .bind(credential.account_id)
            .bind(&credential.access_token_enc)
            .bind(&credential.fallback_access_token_enc)
            .bind(&credential.refresh_token_enc)
            .bind(&credential.refresh_token_sha256)
            .bind(&credential.token_family_id)
            .bind(i64::try_from(credential.token_version).context("token_version overflow")?)
            .bind(credential.token_expires_at)
            .bind(credential.fallback_token_expires_at)
            .bind(credential.last_refresh_at)
            .bind(enum_string(
                &credential.last_refresh_status,
                "oauth refresh status",
            )?)
            .bind(credential.refresh_reused_detected)
            .bind(&credential.last_refresh_error_code)
            .bind(&credential.last_refresh_error)
            .bind(
                i32::try_from(credential.refresh_failure_count)
                    .context("refresh_failure_count overflow")?,
            )
            .bind(credential.refresh_backoff_until)
            .bind(Utc::now())
            .execute(tx.as_mut())
            .await
            .context("failed to import oauth credential")?;
        }

        for profile in &bundle.session_profiles {
            sqlx::query(
                r#"
                INSERT INTO upstream_account_session_profiles (
                    account_id,
                    credential_kind,
                    token_expires_at,
                    email,
                    oauth_subject,
                    oauth_identity_provider,
                    email_verified,
                    chatgpt_plan_type,
                    chatgpt_user_id,
                    chatgpt_subscription_active_start,
                    chatgpt_subscription_active_until,
                    chatgpt_subscription_last_checked,
                    chatgpt_account_user_id,
                    chatgpt_compute_residency,
                    workspace_name,
                    organizations_json,
                    groups_json,
                    source_type,
                    updated_at
                )
                VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15,
                    $16::jsonb, $17::jsonb, $18, $19
                )
                "#,
            )
            .bind(profile.account_id)
            .bind(enum_string(
                &profile.credential_kind,
                "session credential kind",
            )?)
            .bind(profile.token_expires_at)
            .bind(&profile.email)
            .bind(&profile.oauth_subject)
            .bind(&profile.oauth_identity_provider)
            .bind(profile.email_verified)
            .bind(&profile.chatgpt_plan_type)
            .bind(&profile.chatgpt_user_id)
            .bind(profile.chatgpt_subscription_active_start)
            .bind(profile.chatgpt_subscription_active_until)
            .bind(profile.chatgpt_subscription_last_checked)
            .bind(&profile.chatgpt_account_user_id)
            .bind(&profile.chatgpt_compute_residency)
            .bind(&profile.workspace_name)
            .bind(
                profile
                    .organizations
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()
                    .context("failed to encode organizations json")?,
            )
            .bind(
                profile
                    .groups
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()
                    .context("failed to encode groups json")?,
            )
            .bind(&profile.source_type)
            .bind(Utc::now())
            .execute(tx.as_mut())
            .await
            .context("failed to import session profile")?;
        }

        for state in &bundle.account_health_states {
            sqlx::query(
                r#"
                INSERT INTO upstream_account_health_state (account_id, seen_ok_at)
                VALUES ($1, $2)
                "#,
            )
            .bind(state.account_id)
            .bind(state.seen_ok_at)
            .execute(tx.as_mut())
            .await
            .context("failed to import upstream account health state")?;
        }

        for support in &bundle.account_model_support {
            sqlx::query(
                r#"
                INSERT INTO upstream_account_model_support (
                    account_id,
                    supported_models_json,
                    checked_at,
                    updated_at
                )
                VALUES ($1, $2::jsonb, $3, $4)
                "#,
            )
            .bind(support.account_id)
            .bind(
                serde_json::to_string(&support.supported_models)
                    .context("failed to encode supported_models_json")?,
            )
            .bind(Option::<chrono::DateTime<Utc>>::None)
            .bind(Utc::now())
            .execute(tx.as_mut())
            .await
            .context("failed to import account model support")?;
        }

        for policy in &bundle.routing_policies {
            sqlx::query(
                r#"
                INSERT INTO routing_policies (
                    tenant_id,
                    strategy,
                    max_retries,
                    stream_max_retries,
                    updated_at
                )
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(policy.tenant_id)
            .bind(enum_string(&policy.strategy, "routing strategy")?)
            .bind(i64::from(policy.max_retries))
            .bind(i64::from(policy.stream_max_retries))
            .bind(policy.updated_at)
            .execute(tx.as_mut())
            .await
            .context("failed to import routing policy")?;
        }

        for profile in &bundle.routing_profiles {
            sqlx::query(
                r#"
                INSERT INTO routing_profiles (
                    id,
                    name,
                    description,
                    enabled,
                    priority,
                    selector_json,
                    created_at,
                    updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7, $8)
                "#,
            )
            .bind(profile.id)
            .bind(&profile.name)
            .bind(&profile.description)
            .bind(profile.enabled)
            .bind(profile.priority)
            .bind(
                serde_json::to_string(&profile.selector)
                    .context("failed to encode routing profile selector")?,
            )
            .bind(profile.created_at)
            .bind(profile.updated_at)
            .execute(tx.as_mut())
            .await
            .context("failed to import routing profile")?;
        }

        for policy in &bundle.model_routing_policies {
            sqlx::query(
                r#"
                INSERT INTO model_routing_policies (
                    id,
                    name,
                    family,
                    exact_models_json,
                    model_prefixes_json,
                    fallback_profile_ids_json,
                    enabled,
                    priority,
                    created_at,
                    updated_at
                )
                VALUES ($1, $2, $3, $4::jsonb, $5::jsonb, $6::jsonb, $7, $8, $9, $10)
                "#,
            )
            .bind(policy.id)
            .bind(&policy.name)
            .bind(&policy.family)
            .bind(
                serde_json::to_string(&policy.exact_models)
                    .context("failed to encode exact_models_json")?,
            )
            .bind(
                serde_json::to_string(&policy.model_prefixes)
                    .context("failed to encode model_prefixes_json")?,
            )
            .bind(
                serde_json::to_string(&policy.fallback_profile_ids)
                    .context("failed to encode fallback_profile_ids_json")?,
            )
            .bind(policy.enabled)
            .bind(policy.priority)
            .bind(policy.created_at)
            .bind(policy.updated_at)
            .execute(tx.as_mut())
            .await
            .context("failed to import model routing policy")?;
        }

        if let Some(settings) = &bundle.model_routing_settings {
            sqlx::query(
                r#"
                UPDATE ai_routing_settings
                SET
                    enabled = $2,
                    auto_publish = $3,
                    planner_model_chain_json = $4::jsonb,
                    trigger_mode = $5,
                    kill_switch = $6,
                    updated_at = $7
                WHERE singleton = $1
                "#,
            )
            .bind(true)
            .bind(settings.enabled)
            .bind(settings.auto_publish)
            .bind(
                serde_json::to_string(&settings.planner_model_chain)
                    .context("failed to encode planner_model_chain_json")?,
            )
            .bind(enum_string(
                &settings.trigger_mode,
                "model routing trigger mode",
            )?)
            .bind(settings.kill_switch)
            .bind(settings.updated_at)
            .execute(tx.as_mut())
            .await
            .context("failed to import ai_routing_settings")?;
        }

        if let Some(settings) = &bundle.outbound_proxy_pool_settings {
            sqlx::query(
                r#"
                UPDATE outbound_proxy_pool_settings
                SET
                    enabled = $2,
                    fail_mode = $3,
                    updated_at = $4
                WHERE singleton = $1
                "#,
            )
            .bind(true)
            .bind(settings.enabled)
            .bind(enum_string(
                &settings.fail_mode,
                "outbound proxy fail mode",
            )?)
            .bind(settings.updated_at)
            .execute(tx.as_mut())
            .await
            .context("failed to import outbound_proxy_pool_settings")?;
        }

        for node in &bundle.outbound_proxy_nodes {
            sqlx::query(
                r#"
                INSERT INTO outbound_proxy_nodes (
                    id,
                    label,
                    proxy_url,
                    enabled,
                    weight,
                    last_test_status,
                    last_latency_ms,
                    last_error,
                    last_tested_at,
                    created_at,
                    updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                "#,
            )
            .bind(node.id)
            .bind(&node.label)
            .bind(&node.proxy_url)
            .bind(node.enabled)
            .bind(i64::from(node.weight))
            .bind(&node.last_test_status)
            .bind(
                node.last_latency_ms
                    .map(i64::try_from)
                    .transpose()
                    .context("outbound proxy last_latency_ms overflow")?,
            )
            .bind(&node.last_error)
            .bind(node.last_tested_at)
            .bind(node.created_at)
            .bind(node.updated_at)
            .execute(tx.as_mut())
            .await
            .context("failed to import outbound proxy node")?;
        }

        if let Some(settings) = &bundle.upstream_error_learning_settings {
            sqlx::query(
                r#"
                UPDATE upstream_error_learning_settings
                SET
                    enabled = $2,
                    first_seen_timeout_ms = $3,
                    review_hit_threshold = $4,
                    updated_at = $5
                WHERE singleton = $1
                "#,
            )
            .bind(true)
            .bind(settings.enabled)
            .bind(
                i64::try_from(settings.first_seen_timeout_ms)
                    .context("first_seen_timeout_ms overflow")?,
            )
            .bind(
                i32::try_from(settings.review_hit_threshold)
                    .context("review_hit_threshold overflow")?,
            )
            .bind(settings.updated_at.unwrap_or_else(Utc::now))
            .execute(tx.as_mut())
            .await
            .context("failed to import upstream_error_learning_settings")?;
        }

        for template in &bundle.upstream_error_templates {
            sqlx::query(
                r#"
                INSERT INTO upstream_error_templates (
                    id,
                    fingerprint,
                    provider,
                    normalized_status_code,
                    semantic_error_code,
                    action,
                    retry_scope,
                    status,
                    templates_json,
                    representative_samples_json,
                    hit_count,
                    first_seen_at,
                    last_seen_at,
                    updated_at
                )
                VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9::jsonb, $10::jsonb, $11, $12, $13, $14
                )
                "#,
            )
            .bind(template.id)
            .bind(&template.fingerprint)
            .bind(&template.provider)
            .bind(i32::from(template.normalized_status_code))
            .bind(&template.semantic_error_code)
            .bind(enum_string(&template.action, "upstream error action")?)
            .bind(enum_string(
                &template.retry_scope,
                "upstream error retry scope",
            )?)
            .bind(enum_string(
                &template.status,
                "upstream error template status",
            )?)
            .bind(
                serde_json::to_string(&template.templates)
                    .context("failed to encode upstream error templates_json")?,
            )
            .bind(
                serde_json::to_string(&template.representative_samples)
                    .context("failed to encode representative_samples_json")?,
            )
            .bind(i64::try_from(template.hit_count).context("hit_count overflow")?)
            .bind(template.first_seen_at)
            .bind(template.last_seen_at)
            .bind(template.updated_at)
            .execute(tx.as_mut())
            .await
            .context("failed to import upstream error template")?;
        }

        for record in &bundle.builtin_error_template_overrides {
            sqlx::query(
                r#"
                INSERT INTO builtin_error_template_overrides (
                    template_kind,
                    template_code,
                    templates_json,
                    updated_at
                )
                VALUES ($1, $2, $3::jsonb, $4)
                "#,
            )
            .bind(enum_string(&record.kind, "builtin error template kind")?)
            .bind(&record.code)
            .bind(
                serde_json::to_string(&record.templates)
                    .context("failed to encode builtin override templates_json")?,
            )
            .bind(record.updated_at)
            .execute(tx.as_mut())
            .await
            .context("failed to import builtin error template override")?;
        }

        for version in &bundle.routing_plan_versions {
            sqlx::query(
                r#"
                INSERT INTO routing_plan_versions (id, reason, published_at, compiled_plan_json)
                VALUES ($1, $2, $3, $4::jsonb)
                "#,
            )
            .bind(version.id)
            .bind(&version.reason)
            .bind(version.published_at)
            .bind(
                serde_json::to_string(&version.compiled_plan)
                    .context("failed to encode compiled_plan_json")?,
            )
            .execute(tx.as_mut())
            .await
            .context("failed to import routing plan version")?;
        }

        sqlx::query(
            r#"
            UPDATE snapshot_state
            SET revision = $2, dirty = false
            WHERE singleton = $1
            "#,
        )
        .bind(true)
        .bind(i64::try_from(bundle.revision.max(1)).context("revision overflow")?)
        .execute(tx.as_mut())
        .await
        .context("failed to import snapshot revision")?;

        tx.commit()
            .await
            .context("failed to commit postgres migration transaction")?;
        Ok(())
    }
}
