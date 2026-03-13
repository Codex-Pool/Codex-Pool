use anyhow::Context;
use sqlx_core::pool::PoolOptions;
use sqlx_sqlite::SqlitePool;

const SQLITE_STORE_STATE_ROW_ID: i64 = 1;
const SQLITE_STORE_STATE_VERSION: i64 = 1;

use crate::Row;

pub fn normalize_sqlite_database_url(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "sqlite://./codex-pool-personal.sqlite?mode=rwc".to_string();
    }
    if trimmed.starts_with("sqlite:") {
        if trimmed.contains("?") || trimmed == "sqlite::memory:" {
            return trimmed.to_string();
        }
        return format!("{trimmed}?mode=rwc");
    }
    format!("sqlite://{trimmed}?mode=rwc")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SqlitePersistedStoreState {
    tenants: HashMap<Uuid, Tenant>,
    api_keys: HashMap<Uuid, ApiKey>,
    api_key_tokens: HashMap<String, Uuid>,
    accounts: HashMap<Uuid, UpstreamAccount>,
    account_auth_providers: HashMap<Uuid, UpstreamAuthProvider>,
    oauth_credentials: HashMap<Uuid, OAuthCredentialRecord>,
    session_profiles: HashMap<Uuid, SessionProfileRecord>,
    account_health_states: HashMap<Uuid, UpstreamAccountHealthStateRecord>,
    account_model_support: HashMap<Uuid, AccountModelSupportRecord>,
    policies: HashMap<Uuid, RoutingPolicy>,
    routing_profiles: HashMap<Uuid, RoutingProfile>,
    model_routing_policies: HashMap<Uuid, ModelRoutingPolicy>,
    model_routing_settings: ModelRoutingSettings,
    upstream_error_learning_settings: AiErrorLearningSettings,
    upstream_error_templates: Vec<UpstreamErrorTemplateRecord>,
    builtin_error_template_overrides: Vec<BuiltinErrorTemplateOverrideRecord>,
    routing_plan_versions: Vec<RoutingPlanVersion>,
    revision: u64,
}

impl InMemoryStore {
    fn export_sqlite_state(&self) -> SqlitePersistedStoreState {
        SqlitePersistedStoreState {
            tenants: self.tenants.read().unwrap().clone(),
            api_keys: self.api_keys.read().unwrap().clone(),
            api_key_tokens: self.api_key_tokens.read().unwrap().clone(),
            accounts: self.accounts.read().unwrap().clone(),
            account_auth_providers: self.account_auth_providers.read().unwrap().clone(),
            oauth_credentials: self.oauth_credentials.read().unwrap().clone(),
            session_profiles: self.session_profiles.read().unwrap().clone(),
            account_health_states: self.account_health_states.read().unwrap().clone(),
            account_model_support: self.account_model_support.read().unwrap().clone(),
            policies: self.policies.read().unwrap().clone(),
            routing_profiles: self.routing_profiles.read().unwrap().clone(),
            model_routing_policies: self.model_routing_policies.read().unwrap().clone(),
            model_routing_settings: self.model_routing_settings.read().unwrap().clone(),
            upstream_error_learning_settings: self
                .upstream_error_learning_settings
                .read()
                .unwrap()
                .clone(),
            upstream_error_templates: self
                .upstream_error_templates
                .read()
                .unwrap()
                .values()
                .cloned()
                .collect(),
            builtin_error_template_overrides: self
                .builtin_error_template_overrides
                .read()
                .unwrap()
                .values()
                .cloned()
                .collect(),
            routing_plan_versions: self.routing_plan_versions.read().unwrap().clone(),
            revision: self.revision.load(Ordering::Relaxed).max(1),
        }
    }

    fn from_sqlite_state(
        state: SqlitePersistedStoreState,
        oauth_client: Arc<dyn OAuthTokenClient>,
        credential_cipher: Option<CredentialCipher>,
    ) -> Self {
        let upstream_error_template_index = state
            .upstream_error_templates
            .iter()
            .map(|record| (record.fingerprint.clone(), record.id))
            .collect::<HashMap<_, _>>();
        let upstream_error_templates = state
            .upstream_error_templates
            .into_iter()
            .map(|record| (record.id, record))
            .collect::<HashMap<_, _>>();
        let builtin_error_template_overrides = state
            .builtin_error_template_overrides
            .into_iter()
            .map(|record| ((record.kind, record.code.clone()), record))
            .collect::<HashMap<_, _>>();

        Self {
            tenants: Arc::new(RwLock::new(state.tenants)),
            api_keys: Arc::new(RwLock::new(state.api_keys)),
            api_key_tokens: Arc::new(RwLock::new(state.api_key_tokens)),
            accounts: Arc::new(RwLock::new(state.accounts)),
            account_auth_providers: Arc::new(RwLock::new(state.account_auth_providers)),
            oauth_credentials: Arc::new(RwLock::new(state.oauth_credentials)),
            session_profiles: Arc::new(RwLock::new(state.session_profiles)),
            account_health_states: Arc::new(RwLock::new(state.account_health_states)),
            account_model_support: Arc::new(RwLock::new(state.account_model_support)),
            policies: Arc::new(RwLock::new(state.policies)),
            routing_profiles: Arc::new(RwLock::new(state.routing_profiles)),
            model_routing_policies: Arc::new(RwLock::new(state.model_routing_policies)),
            model_routing_settings: Arc::new(RwLock::new(state.model_routing_settings)),
            upstream_error_learning_settings: Arc::new(RwLock::new(
                state.upstream_error_learning_settings,
            )),
            upstream_error_templates: Arc::new(RwLock::new(upstream_error_templates)),
            upstream_error_template_index: Arc::new(RwLock::new(upstream_error_template_index)),
            builtin_error_template_overrides: Arc::new(RwLock::new(
                builtin_error_template_overrides,
            )),
            routing_plan_versions: Arc::new(RwLock::new(state.routing_plan_versions)),
            revision: Arc::new(AtomicU64::new(state.revision.max(1))),
            oauth_client,
            credential_cipher,
        }
    }
}

pub struct SqliteBackedStore {
    pool: SqlitePool,
    inner: InMemoryStore,
}

impl SqliteBackedStore {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let normalized = normalize_sqlite_database_url(database_url);
        let pool = PoolOptions::new()
            .max_connections(1)
            .connect(&normalized)
            .await
            .with_context(|| format!("failed to connect sqlite store at {normalized}"))?;

        Self::initialize_schema(&pool).await?;
        let oauth_client: Arc<dyn OAuthTokenClient> = Arc::new(OpenAiOAuthClient::from_env());
        let credential_cipher = CredentialCipher::from_env().unwrap_or(None);
        let inner = match Self::load_state(&pool).await? {
            Some(state) => InMemoryStore::from_sqlite_state(state, oauth_client, credential_cipher),
            None => InMemoryStore::new_with_oauth(oauth_client, credential_cipher),
        };

        Ok(Self { pool, inner })
    }

    pub fn clone_pool(&self) -> SqlitePool {
        self.pool.clone()
    }

    fn current_revision(&self) -> u64 {
        self.inner.revision.load(Ordering::Relaxed).max(1)
    }

    async fn initialize_schema(pool: &SqlitePool) -> Result<()> {
        let _ = sqlx::query("PRAGMA journal_mode = WAL")
            .execute(pool)
            .await;
        let _ = sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(pool)
            .await;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS control_plane_state (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                schema_version INTEGER NOT NULL,
                state_json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(pool)
        .await
        .context("failed to create sqlite control_plane_state table")?;

        Ok(())
    }

    async fn load_state(pool: &SqlitePool) -> Result<Option<SqlitePersistedStoreState>> {
        let row = sqlx::query(
            r#"
            SELECT state_json
            FROM control_plane_state
            WHERE id = ?
            "#,
        )
        .bind(SQLITE_STORE_STATE_ROW_ID)
        .fetch_optional(pool)
        .await
        .context("failed to load sqlite control-plane state")?;

        let Some(row) = row else {
            return Ok(None);
        };

        let state_json: String = row.try_get("state_json")?;
        let state = serde_json::from_str::<SqlitePersistedStoreState>(&state_json)
            .context("failed to decode sqlite control-plane state payload")?;
        Ok(Some(state))
    }

    async fn persist_state(&self) -> Result<()> {
        let state_json = serde_json::to_string(&self.inner.export_sqlite_state())
            .context("failed to encode sqlite control-plane state")?;
        let updated_at = Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO control_plane_state (id, schema_version, state_json, updated_at)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                schema_version = excluded.schema_version,
                state_json = excluded.state_json,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(SQLITE_STORE_STATE_ROW_ID)
        .bind(SQLITE_STORE_STATE_VERSION)
        .bind(state_json)
        .bind(updated_at)
        .execute(&self.pool)
        .await
        .context("failed to persist sqlite control-plane state")?;

        Ok(())
    }

    async fn persist_if_revision_changed(&self, before: u64) -> Result<()> {
        if self.current_revision() != before {
            self.persist_state().await?;
        }
        Ok(())
    }
}

#[async_trait]
impl ControlPlaneStore for SqliteBackedStore {
    async fn create_tenant(&self, req: CreateTenantRequest) -> Result<Tenant> {
        let tenant = self.inner.create_tenant(req).await?;
        self.persist_state().await?;
        Ok(tenant)
    }

    async fn list_tenants(&self) -> Result<Vec<Tenant>> {
        self.inner.list_tenants().await
    }

    async fn create_api_key(&self, req: CreateApiKeyRequest) -> Result<CreateApiKeyResponse> {
        let response = self.inner.create_api_key(req).await?;
        self.persist_state().await?;
        Ok(response)
    }

    async fn list_api_keys(&self) -> Result<Vec<ApiKey>> {
        self.inner.list_api_keys().await
    }

    async fn set_api_key_enabled(&self, api_key_id: Uuid, enabled: bool) -> Result<ApiKey> {
        let key = self.inner.set_api_key_enabled(api_key_id, enabled).await?;
        self.persist_state().await?;
        Ok(key)
    }

    async fn validate_api_key(&self, token: &str) -> Result<Option<ValidatedPrincipal>> {
        self.inner.validate_api_key(token).await
    }

    async fn create_upstream_account(
        &self,
        req: CreateUpstreamAccountRequest,
    ) -> Result<UpstreamAccount> {
        let account = self.inner.create_upstream_account(req).await?;
        self.persist_state().await?;
        Ok(account)
    }

    async fn list_upstream_accounts(&self) -> Result<Vec<UpstreamAccount>> {
        let before = self.current_revision();
        let items = self.inner.list_upstream_accounts().await?;
        self.persist_if_revision_changed(before).await?;
        Ok(items)
    }

    async fn set_upstream_account_enabled(
        &self,
        account_id: Uuid,
        enabled: bool,
    ) -> Result<UpstreamAccount> {
        let account = self
            .inner
            .set_upstream_account_enabled(account_id, enabled)
            .await?;
        self.persist_state().await?;
        Ok(account)
    }

    async fn delete_upstream_account(&self, account_id: Uuid) -> Result<()> {
        self.inner.delete_upstream_account(account_id).await?;
        self.persist_state().await
    }

    async fn validate_oauth_refresh_token(
        &self,
        req: ValidateOAuthRefreshTokenRequest,
    ) -> Result<ValidateOAuthRefreshTokenResponse> {
        self.inner.validate_oauth_refresh_token(req).await
    }

    async fn import_oauth_refresh_token(
        &self,
        req: ImportOAuthRefreshTokenRequest,
    ) -> Result<UpstreamAccount> {
        let account = self.inner.import_oauth_refresh_token(req).await?;
        self.persist_state().await?;
        Ok(account)
    }

    async fn upsert_oauth_refresh_token(
        &self,
        req: ImportOAuthRefreshTokenRequest,
    ) -> Result<OAuthUpsertResult> {
        let result = self.inner.upsert_oauth_refresh_token(req).await?;
        self.persist_state().await?;
        Ok(result)
    }

    async fn dedupe_oauth_accounts_by_identity(&self) -> Result<u64> {
        let removed = self.inner.dedupe_oauth_accounts_by_identity().await?;
        self.persist_state().await?;
        Ok(removed)
    }

    async fn upsert_one_time_session_account(
        &self,
        req: UpsertOneTimeSessionAccountRequest,
    ) -> Result<OAuthUpsertResult> {
        let result = self.inner.upsert_one_time_session_account(req).await?;
        self.persist_state().await?;
        Ok(result)
    }

    async fn refresh_oauth_account(&self, account_id: Uuid) -> Result<OAuthAccountStatusResponse> {
        let status = self.inner.refresh_oauth_account(account_id).await?;
        self.persist_state().await?;
        Ok(status)
    }

    async fn oauth_account_status(&self, account_id: Uuid) -> Result<OAuthAccountStatusResponse> {
        self.inner.oauth_account_status(account_id).await
    }

    async fn oauth_account_statuses(
        &self,
        account_ids: Vec<Uuid>,
    ) -> Result<Vec<OAuthAccountStatusResponse>> {
        self.inner.oauth_account_statuses(account_ids).await
    }

    async fn upsert_routing_policy(
        &self,
        req: UpsertRoutingPolicyRequest,
    ) -> Result<RoutingPolicy> {
        let policy = self.inner.upsert_routing_policy(req).await?;
        self.persist_state().await?;
        Ok(policy)
    }

    async fn upsert_retry_policy(&self, req: UpsertRetryPolicyRequest) -> Result<RoutingPolicy> {
        let policy = self.inner.upsert_retry_policy(req).await?;
        self.persist_state().await?;
        Ok(policy)
    }

    async fn upsert_stream_retry_policy(
        &self,
        req: UpsertStreamRetryPolicyRequest,
    ) -> Result<RoutingPolicy> {
        let policy = self.inner.upsert_stream_retry_policy(req).await?;
        self.persist_state().await?;
        Ok(policy)
    }

    async fn list_routing_profiles(&self) -> Result<Vec<RoutingProfile>> {
        self.inner.list_routing_profiles().await
    }

    async fn upsert_routing_profile(
        &self,
        req: UpsertRoutingProfileRequest,
    ) -> Result<RoutingProfile> {
        let profile = self.inner.upsert_routing_profile(req).await?;
        self.persist_state().await?;
        Ok(profile)
    }

    async fn delete_routing_profile(&self, profile_id: Uuid) -> Result<()> {
        self.inner.delete_routing_profile(profile_id).await?;
        self.persist_state().await
    }

    async fn list_model_routing_policies(&self) -> Result<Vec<ModelRoutingPolicy>> {
        self.inner.list_model_routing_policies().await
    }

    async fn upsert_model_routing_policy(
        &self,
        req: UpsertModelRoutingPolicyRequest,
    ) -> Result<ModelRoutingPolicy> {
        let policy = self.inner.upsert_model_routing_policy(req).await?;
        self.persist_state().await?;
        Ok(policy)
    }

    async fn delete_model_routing_policy(&self, policy_id: Uuid) -> Result<()> {
        self.inner.delete_model_routing_policy(policy_id).await?;
        self.persist_state().await
    }

    async fn model_routing_settings(&self) -> Result<ModelRoutingSettings> {
        self.inner.model_routing_settings().await
    }

    async fn update_model_routing_settings(
        &self,
        req: UpdateModelRoutingSettingsRequest,
    ) -> Result<ModelRoutingSettings> {
        let settings = self.inner.update_model_routing_settings(req).await?;
        self.persist_state().await?;
        Ok(settings)
    }

    async fn upstream_error_learning_settings(&self) -> Result<AiErrorLearningSettings> {
        self.inner.upstream_error_learning_settings().await
    }

    async fn update_upstream_error_learning_settings(
        &self,
        req: UpdateAiErrorLearningSettingsRequest,
    ) -> Result<AiErrorLearningSettings> {
        let settings = self
            .inner
            .update_upstream_error_learning_settings(req)
            .await?;
        self.persist_state().await?;
        Ok(settings)
    }

    async fn list_upstream_error_templates(
        &self,
        status: Option<UpstreamErrorTemplateStatus>,
    ) -> Result<Vec<UpstreamErrorTemplateRecord>> {
        self.inner.list_upstream_error_templates(status).await
    }

    async fn upstream_error_template_by_id(
        &self,
        template_id: Uuid,
    ) -> Result<Option<UpstreamErrorTemplateRecord>> {
        self.inner.upstream_error_template_by_id(template_id).await
    }

    async fn upstream_error_template_by_fingerprint(
        &self,
        fingerprint: &str,
    ) -> Result<Option<UpstreamErrorTemplateRecord>> {
        self.inner
            .upstream_error_template_by_fingerprint(fingerprint)
            .await
    }

    async fn save_upstream_error_template(
        &self,
        template: UpstreamErrorTemplateRecord,
    ) -> Result<UpstreamErrorTemplateRecord> {
        let record = self.inner.save_upstream_error_template(template).await?;
        self.persist_state().await?;
        Ok(record)
    }

    async fn list_builtin_error_template_overrides(
        &self,
    ) -> Result<Vec<BuiltinErrorTemplateOverrideRecord>> {
        self.inner.list_builtin_error_template_overrides().await
    }

    async fn save_builtin_error_template_override(
        &self,
        record: BuiltinErrorTemplateOverrideRecord,
    ) -> Result<BuiltinErrorTemplateOverrideRecord> {
        let saved = self.inner.save_builtin_error_template_override(record).await?;
        self.persist_state().await?;
        Ok(saved)
    }

    async fn delete_builtin_error_template_override(
        &self,
        kind: BuiltinErrorTemplateKind,
        code: &str,
    ) -> Result<()> {
        self.inner
            .delete_builtin_error_template_override(kind, code)
            .await?;
        self.persist_state().await
    }

    async fn list_builtin_error_templates(&self) -> Result<Vec<BuiltinErrorTemplateRecord>> {
        self.inner.list_builtin_error_templates().await
    }

    async fn builtin_error_template(
        &self,
        kind: BuiltinErrorTemplateKind,
        code: &str,
    ) -> Result<Option<BuiltinErrorTemplateRecord>> {
        self.inner.builtin_error_template(kind, code).await
    }

    async fn list_routing_plan_versions(&self) -> Result<Vec<RoutingPlanVersion>> {
        self.inner.list_routing_plan_versions().await
    }

    async fn record_account_model_support(
        &self,
        account_id: Uuid,
        supported_models: Vec<String>,
        checked_at: DateTime<Utc>,
    ) -> Result<()> {
        self.inner
            .record_account_model_support(account_id, supported_models, checked_at)
            .await?;
        self.persist_state().await
    }

    async fn refresh_expiring_oauth_accounts(&self) -> Result<()> {
        let before = self.current_revision();
        self.inner.refresh_expiring_oauth_accounts().await?;
        self.persist_if_revision_changed(before).await
    }

    async fn activate_oauth_refresh_token_vault(&self) -> Result<u64> {
        self.inner.activate_oauth_refresh_token_vault().await
    }

    async fn refresh_due_oauth_rate_limit_caches(&self) -> Result<u64> {
        self.inner.refresh_due_oauth_rate_limit_caches().await
    }

    async fn recover_oauth_rate_limit_refresh_jobs(&self) -> Result<u64> {
        self.inner.recover_oauth_rate_limit_refresh_jobs().await
    }

    async fn create_oauth_rate_limit_refresh_job(&self) -> Result<OAuthRateLimitRefreshJobSummary> {
        self.inner.create_oauth_rate_limit_refresh_job().await
    }

    async fn oauth_rate_limit_refresh_job(
        &self,
        job_id: Uuid,
    ) -> Result<OAuthRateLimitRefreshJobSummary> {
        self.inner.oauth_rate_limit_refresh_job(job_id).await
    }

    async fn run_oauth_rate_limit_refresh_job(&self, job_id: Uuid) -> Result<()> {
        self.inner.run_oauth_rate_limit_refresh_job(job_id).await
    }

    async fn flush_snapshot_revision(&self, max_batch: usize) -> Result<u32> {
        self.inner.flush_snapshot_revision(max_batch).await
    }

    async fn set_oauth_family_enabled(
        &self,
        account_id: Uuid,
        enabled: bool,
    ) -> Result<OAuthFamilyActionResponse> {
        let response = self.inner.set_oauth_family_enabled(account_id, enabled).await?;
        self.persist_state().await?;
        Ok(response)
    }

    async fn snapshot(&self) -> Result<DataPlaneSnapshot> {
        self.inner.snapshot().await
    }

    async fn cleanup_data_plane_outbox(&self, retention: Duration) -> Result<u64> {
        self.inner.cleanup_data_plane_outbox(retention).await
    }

    async fn data_plane_snapshot_events(
        &self,
        after: u64,
        limit: u32,
    ) -> Result<DataPlaneSnapshotEventsResponse> {
        let current = self.current_revision();
        if after < current {
            return Err(anyhow!("cursor_gone"));
        }
        self.inner.data_plane_snapshot_events(after, limit).await
    }

    async fn mark_account_seen_ok(
        &self,
        account_id: Uuid,
        seen_ok_at: DateTime<Utc>,
        min_write_interval_sec: i64,
    ) -> Result<bool> {
        let changed = self
            .inner
            .mark_account_seen_ok(account_id, seen_ok_at, min_write_interval_sec)
            .await?;
        if changed {
            self.persist_state().await?;
        }
        Ok(changed)
    }

    async fn maybe_refresh_oauth_rate_limit_cache_on_seen_ok(
        &self,
        account_id: Uuid,
    ) -> Result<()> {
        self.inner
            .maybe_refresh_oauth_rate_limit_cache_on_seen_ok(account_id)
            .await
    }
}

#[cfg(test)]
mod sqlite_backed_store_tests {
    use super::{normalize_sqlite_database_url, SqliteBackedStore};
    use crate::store::ControlPlaneStore;
    use codex_pool_core::api::{
        CreateApiKeyRequest, CreateTenantRequest, CreateUpstreamAccountRequest,
    };
    use codex_pool_core::model::{UpstreamAuthProvider, UpstreamMode};
    use uuid::Uuid;

    fn temp_sqlite_url(name: &str) -> String {
        let path = std::env::temp_dir().join(format!("{name}-{}.sqlite3", Uuid::new_v4()));
        normalize_sqlite_database_url(&path.display().to_string())
    }

    #[tokio::test]
    async fn sqlite_backed_store_persists_accounts_and_api_keys() {
        let database_url = temp_sqlite_url("cp-store-roundtrip");
        let store = SqliteBackedStore::connect(&database_url)
            .await
            .expect("connect sqlite store");
        let tenant = store
            .create_tenant(CreateTenantRequest {
                name: "personal".to_string(),
            })
            .await
            .expect("create tenant");
        let created_key = store
            .create_api_key(CreateApiKeyRequest {
                name: "personal-key".to_string(),
                tenant_id: tenant.id,
            })
            .await
            .expect("create api key");
        let created_account = store
            .create_upstream_account(CreateUpstreamAccountRequest {
                label: "personal-upstream".to_string(),
                mode: UpstreamMode::OpenAiApiKey,
                base_url: "https://api.openai.com".to_string(),
                bearer_token: "test-token".to_string(),
                chatgpt_account_id: None,
                auth_provider: Some(UpstreamAuthProvider::LegacyBearer),
                enabled: Some(true),
                priority: Some(10),
            })
            .await
            .expect("create upstream account");
        drop(store);

        let reopened = SqliteBackedStore::connect(&database_url)
            .await
            .expect("reopen sqlite store");
        let keys = reopened.list_api_keys().await.expect("list api keys");
        let accounts = reopened
            .list_upstream_accounts()
            .await
            .expect("list upstream accounts");
        let validated = reopened
            .validate_api_key(&created_key.plaintext_key)
            .await
            .expect("validate api key");

        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].id, created_key.record.id);
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].id, created_account.id);
        assert_eq!(validated.as_ref().map(|item| item.api_key_id), Some(created_key.record.id));
    }

    #[tokio::test]
    async fn sqlite_backed_store_forces_snapshot_reload_after_revision_change() {
        let database_url = temp_sqlite_url("cp-store-events");
        let store = SqliteBackedStore::connect(&database_url)
            .await
            .expect("connect sqlite store");
        let current = store.snapshot().await.expect("snapshot").revision;
        let response = store
            .data_plane_snapshot_events(current, 500)
            .await
            .expect("snapshot events without changes");
        assert_eq!(response.cursor, current);

        store
            .create_upstream_account(CreateUpstreamAccountRequest {
                label: "personal-upstream".to_string(),
                mode: UpstreamMode::OpenAiApiKey,
                base_url: "https://api.openai.com".to_string(),
                bearer_token: "test-token".to_string(),
                chatgpt_account_id: None,
                auth_provider: Some(UpstreamAuthProvider::LegacyBearer),
                enabled: Some(true),
                priority: Some(10),
            })
            .await
            .expect("create upstream account");

        let error = store
            .data_plane_snapshot_events(current, 500)
            .await
            .expect_err("stale cursor should force full reload");
        assert!(error.to_string().contains("cursor_gone"));
    }
}
