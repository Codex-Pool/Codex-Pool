use anyhow::Context;
use sqlx_core::pool::PoolOptions;
use sqlx_sqlite::SqlitePool;
use tokio::sync::{Mutex, watch};
#[cfg(test)]
use std::sync::atomic::AtomicUsize;

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
    #[serde(default)]
    oauth_refresh_token_vault: HashMap<Uuid, OAuthRefreshTokenVaultRecord>,
    session_profiles: HashMap<Uuid, SessionProfileRecord>,
    account_health_states: HashMap<Uuid, UpstreamAccountHealthStateRecord>,
    account_model_support: HashMap<Uuid, AccountModelSupportRecord>,
    #[serde(default)]
    oauth_rate_limit_caches: HashMap<Uuid, OAuthRateLimitCacheRecord>,
    #[serde(default)]
    oauth_rate_limit_refresh_jobs: HashMap<Uuid, OAuthRateLimitRefreshJobSummary>,
    #[serde(default)]
    outbound_proxy_pool_settings: OutboundProxyPoolSettings,
    #[serde(default)]
    outbound_proxy_nodes: HashMap<Uuid, OutboundProxyNode>,
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
            oauth_refresh_token_vault: self.oauth_refresh_token_vault.read().unwrap().clone(),
            session_profiles: self.session_profiles.read().unwrap().clone(),
            account_health_states: self.account_health_states.read().unwrap().clone(),
            account_model_support: self.account_model_support.read().unwrap().clone(),
            oauth_rate_limit_caches: self.oauth_rate_limit_caches.read().unwrap().clone(),
            oauth_rate_limit_refresh_jobs: self
                .oauth_rate_limit_refresh_jobs
                .read()
                .unwrap()
                .clone(),
            outbound_proxy_pool_settings: self.outbound_proxy_pool_settings.read().unwrap().clone(),
            outbound_proxy_nodes: self.outbound_proxy_nodes.read().unwrap().clone(),
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
            oauth_refresh_token_vault: Arc::new(RwLock::new(state.oauth_refresh_token_vault)),
            session_profiles: Arc::new(RwLock::new(state.session_profiles)),
            account_health_states: Arc::new(RwLock::new(state.account_health_states)),
            account_model_support: Arc::new(RwLock::new(state.account_model_support)),
            oauth_rate_limit_caches: Arc::new(RwLock::new(state.oauth_rate_limit_caches)),
            oauth_rate_limit_refresh_jobs: Arc::new(RwLock::new(
                state.oauth_rate_limit_refresh_jobs,
            )),
            outbound_proxy_pool_settings: Arc::new(RwLock::new(
                state.outbound_proxy_pool_settings,
            )),
            outbound_proxy_nodes: Arc::new(RwLock::new(state.outbound_proxy_nodes)),
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
    revision_tx: watch::Sender<u64>,
    persist_lock: Mutex<()>,
    persist_generation: AtomicU64,
    persisted_generation: AtomicU64,
    #[cfg(test)]
    persist_write_count: AtomicUsize,
}

impl SqliteBackedStore {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let oauth_client: Arc<dyn OAuthTokenClient> = Arc::new(OpenAiOAuthClient::from_env());
        let credential_cipher = CredentialCipher::from_env().unwrap_or(None);
        Self::connect_with_oauth(database_url, oauth_client, credential_cipher).await
    }

    pub async fn connect_with_oauth(
        database_url: &str,
        oauth_client: Arc<dyn OAuthTokenClient>,
        credential_cipher: Option<CredentialCipher>,
    ) -> Result<Self> {
        let normalized = normalize_sqlite_database_url(database_url);
        let pool = PoolOptions::new()
            .max_connections(1)
            .connect(&normalized)
            .await
            .with_context(|| format!("failed to connect sqlite store at {normalized}"))?;

        Self::initialize_schema(&pool).await?;
        let inner = match Self::load_state(&pool).await? {
            Some(state) => InMemoryStore::from_sqlite_state(state, oauth_client, credential_cipher),
            None => InMemoryStore::new_with_oauth(oauth_client, credential_cipher),
        };
        let initial_revision = inner.revision.load(Ordering::Relaxed).max(1);
        let (revision_tx, _) = watch::channel(initial_revision);

        Ok(Self {
            pool,
            inner,
            revision_tx,
            persist_lock: Mutex::new(()),
            persist_generation: AtomicU64::new(initial_revision),
            persisted_generation: AtomicU64::new(initial_revision),
            #[cfg(test)]
            persist_write_count: AtomicUsize::new(0),
        })
    }

    pub fn clone_pool(&self) -> SqlitePool {
        self.pool.clone()
    }

    pub fn subscribe_revisions(&self) -> watch::Receiver<u64> {
        self.revision_tx.subscribe()
    }

    #[cfg(test)]
    fn persist_write_count(&self) -> usize {
        self.persist_write_count.load(Ordering::SeqCst)
    }

    fn current_revision(&self) -> u64 {
        self.inner.revision.load(Ordering::Relaxed).max(1)
    }

    fn mark_persist_pending(&self) -> u64 {
        self.persist_generation.fetch_add(1, Ordering::SeqCst) + 1
    }

    async fn persist_state_after_write(&self) -> Result<()> {
        self.mark_persist_pending();
        self.persist_state().await
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
        let pending_generation = self.persist_generation.load(Ordering::SeqCst);
        if pending_generation <= self.persisted_generation.load(Ordering::SeqCst) {
            return Ok(());
        }

        // Serialize full-state SQLite writes so concurrent mutators can reuse
        // the newest persisted revision instead of rewriting the same payload.
        let _persist_guard = self.persist_lock.lock().await;
        let pending_generation = self.persist_generation.load(Ordering::SeqCst);
        if pending_generation <= self.persisted_generation.load(Ordering::SeqCst) {
            return Ok(());
        }

        let persisted_state = self.inner.export_sqlite_state();
        let persisted_revision = persisted_state.revision.max(1);
        let state_json = serde_json::to_string(&persisted_state)
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

        #[cfg(test)]
        self.persist_write_count.fetch_add(1, Ordering::SeqCst);

        self.persisted_generation
            .store(pending_generation, Ordering::SeqCst);
        let _ = self.revision_tx.send(persisted_revision);

        Ok(())
    }

    async fn persist_if_revision_changed(&self, before: u64) -> Result<()> {
        if self.current_revision() != before {
            self.mark_persist_pending();
            self.persist_state().await?;
        }
        Ok(())
    }
}

pub fn build_sqlite_store_ports(store: Arc<SqliteBackedStore>) -> RuntimeStorePorts {
    RuntimeStorePorts {
        control_plane: store.clone(),
        snapshot_policy: store.clone(),
        tenant_catalog: store.clone(),
        oauth_runtime: store.clone(),
        edition_migration: store,
    }
}

#[async_trait]
impl ControlPlaneStore for SqliteBackedStore {
    async fn create_tenant(&self, req: CreateTenantRequest) -> Result<Tenant> {
        let tenant = ControlPlaneStore::create_tenant(&self.inner, req).await?;
        self.persist_state_after_write().await?;
        Ok(tenant)
    }

    async fn list_tenants(&self) -> Result<Vec<Tenant>> {
        ControlPlaneStore::list_tenants(&self.inner).await
    }

    async fn create_api_key(&self, req: CreateApiKeyRequest) -> Result<CreateApiKeyResponse> {
        let response = ControlPlaneStore::create_api_key(&self.inner, req).await?;
        self.persist_state_after_write().await?;
        Ok(response)
    }

    async fn list_api_keys(&self) -> Result<Vec<ApiKey>> {
        ControlPlaneStore::list_api_keys(&self.inner).await
    }

    async fn set_api_key_enabled(&self, api_key_id: Uuid, enabled: bool) -> Result<ApiKey> {
        let key = self.inner.set_api_key_enabled(api_key_id, enabled).await?;
        self.persist_state_after_write().await?;
        Ok(key)
    }

    async fn outbound_proxy_pool_settings(&self) -> Result<OutboundProxyPoolSettings> {
        self.inner.outbound_proxy_pool_settings().await
    }

    async fn update_outbound_proxy_pool_settings(
        &self,
        req: UpdateOutboundProxyPoolSettingsRequest,
    ) -> Result<OutboundProxyPoolSettings> {
        let settings = self.inner.update_outbound_proxy_pool_settings(req).await?;
        self.persist_state_after_write().await?;
        Ok(settings)
    }

    async fn list_outbound_proxy_nodes(&self) -> Result<Vec<OutboundProxyNode>> {
        self.inner.list_outbound_proxy_nodes().await
    }

    async fn create_outbound_proxy_node(
        &self,
        req: CreateOutboundProxyNodeRequest,
    ) -> Result<OutboundProxyNode> {
        let node = self.inner.create_outbound_proxy_node(req).await?;
        self.persist_state_after_write().await?;
        Ok(node)
    }

    async fn update_outbound_proxy_node(
        &self,
        node_id: Uuid,
        req: UpdateOutboundProxyNodeRequest,
    ) -> Result<OutboundProxyNode> {
        let node = self.inner.update_outbound_proxy_node(node_id, req).await?;
        self.persist_state_after_write().await?;
        Ok(node)
    }

    async fn delete_outbound_proxy_node(&self, node_id: Uuid) -> Result<()> {
        self.inner.delete_outbound_proxy_node(node_id).await?;
        self.persist_state_after_write().await
    }

    async fn record_outbound_proxy_test_result(
        &self,
        node_id: Uuid,
        last_test_status: Option<String>,
        last_latency_ms: Option<u64>,
        last_error: Option<String>,
        last_tested_at: Option<DateTime<Utc>>,
    ) -> Result<OutboundProxyNode> {
        let node = self
            .inner
            .record_outbound_proxy_test_result(
                node_id,
                last_test_status,
                last_latency_ms,
                last_error,
                last_tested_at,
            )
            .await?;
        self.persist_state_after_write().await?;
        Ok(node)
    }

    async fn validate_api_key(&self, token: &str) -> Result<Option<ValidatedPrincipal>> {
        ControlPlaneStore::validate_api_key(&self.inner, token).await
    }

    async fn create_upstream_account(
        &self,
        req: CreateUpstreamAccountRequest,
    ) -> Result<UpstreamAccount> {
        let account = ControlPlaneStore::create_upstream_account(&self.inner, req).await?;
        self.persist_state_after_write().await?;
        Ok(account)
    }

    async fn list_upstream_accounts(&self) -> Result<Vec<UpstreamAccount>> {
        let before = self.current_revision();
        let items = ControlPlaneStore::list_upstream_accounts(&self.inner).await?;
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
        self.persist_state_after_write().await?;
        Ok(account)
    }

    async fn delete_upstream_account(&self, account_id: Uuid) -> Result<()> {
        self.inner.delete_upstream_account(account_id).await?;
        self.persist_state_after_write().await
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
        self.persist_state_after_write().await?;
        Ok(account)
    }

    async fn upsert_oauth_refresh_token(
        &self,
        req: ImportOAuthRefreshTokenRequest,
    ) -> Result<OAuthUpsertResult> {
        let result = self.inner.upsert_oauth_refresh_token(req).await?;
        self.persist_state_after_write().await?;
        Ok(result)
    }

    async fn queue_oauth_refresh_token(
        &self,
        req: ImportOAuthRefreshTokenRequest,
    ) -> Result<bool> {
        let created = ControlPlaneStore::queue_oauth_refresh_token(&self.inner, req).await?;
        self.persist_state_after_write().await?;
        Ok(created)
    }

    async fn dedupe_oauth_accounts_by_identity(&self) -> Result<u64> {
        let removed = self.inner.dedupe_oauth_accounts_by_identity().await?;
        self.persist_state_after_write().await?;
        Ok(removed)
    }

    async fn upsert_one_time_session_account(
        &self,
        req: UpsertOneTimeSessionAccountRequest,
    ) -> Result<OAuthUpsertResult> {
        let result = self.inner.upsert_one_time_session_account(req).await?;
        self.persist_state_after_write().await?;
        Ok(result)
    }

    async fn refresh_oauth_account(&self, account_id: Uuid) -> Result<OAuthAccountStatusResponse> {
        let status = self.inner.refresh_oauth_account(account_id).await?;
        self.persist_state_after_write().await?;
        Ok(status)
    }

    async fn oauth_account_status(&self, account_id: Uuid) -> Result<OAuthAccountStatusResponse> {
        self.inner.oauth_account_status(account_id).await
    }

    async fn oauth_inventory_summary(&self) -> Result<OAuthInventorySummaryResponse> {
        self.inner.oauth_inventory_summary().await
    }

    async fn oauth_inventory_records(&self) -> Result<Vec<OAuthInventoryRecord>> {
        self.inner.oauth_inventory_records().await
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
        self.persist_state_after_write().await?;
        Ok(policy)
    }

    async fn upsert_retry_policy(&self, req: UpsertRetryPolicyRequest) -> Result<RoutingPolicy> {
        let policy = self.inner.upsert_retry_policy(req).await?;
        self.persist_state_after_write().await?;
        Ok(policy)
    }

    async fn upsert_stream_retry_policy(
        &self,
        req: UpsertStreamRetryPolicyRequest,
    ) -> Result<RoutingPolicy> {
        let policy = self.inner.upsert_stream_retry_policy(req).await?;
        self.persist_state_after_write().await?;
        Ok(policy)
    }

    async fn list_routing_profiles(&self) -> Result<Vec<RoutingProfile>> {
        ControlPlaneStore::list_routing_profiles(&self.inner).await
    }

    async fn upsert_routing_profile(
        &self,
        req: UpsertRoutingProfileRequest,
    ) -> Result<RoutingProfile> {
        let profile = self.inner.upsert_routing_profile(req).await?;
        self.persist_state_after_write().await?;
        Ok(profile)
    }

    async fn delete_routing_profile(&self, profile_id: Uuid) -> Result<()> {
        self.inner.delete_routing_profile(profile_id).await?;
        self.persist_state_after_write().await
    }

    async fn list_model_routing_policies(&self) -> Result<Vec<ModelRoutingPolicy>> {
        ControlPlaneStore::list_model_routing_policies(&self.inner).await
    }

    async fn upsert_model_routing_policy(
        &self,
        req: UpsertModelRoutingPolicyRequest,
    ) -> Result<ModelRoutingPolicy> {
        let policy = self.inner.upsert_model_routing_policy(req).await?;
        self.persist_state_after_write().await?;
        Ok(policy)
    }

    async fn delete_model_routing_policy(&self, policy_id: Uuid) -> Result<()> {
        self.inner.delete_model_routing_policy(policy_id).await?;
        self.persist_state_after_write().await
    }

    async fn model_routing_settings(&self) -> Result<ModelRoutingSettings> {
        ControlPlaneStore::model_routing_settings(&self.inner).await
    }

    async fn update_model_routing_settings(
        &self,
        req: UpdateModelRoutingSettingsRequest,
    ) -> Result<ModelRoutingSettings> {
        let settings = self.inner.update_model_routing_settings(req).await?;
        self.persist_state_after_write().await?;
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
        self.persist_state_after_write().await?;
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
        self.persist_state_after_write().await?;
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
        self.persist_state_after_write().await?;
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
        self.persist_state_after_write().await
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
        self.persist_state_after_write().await
    }

    async fn refresh_expiring_oauth_accounts(&self) -> Result<()> {
        let before = self.current_revision();
        ControlPlaneStore::refresh_expiring_oauth_accounts(&self.inner).await?;
        self.persist_if_revision_changed(before).await
    }

    async fn activate_oauth_refresh_token_vault(&self) -> Result<u64> {
        let activated = self.inner.activate_oauth_refresh_token_vault_inner().await?;
        self.persist_state_after_write().await?;
        Ok(activated)
    }

    async fn mark_upstream_account_pending_purge(
        &self,
        account_id: Uuid,
        reason: Option<String>,
    ) -> Result<UpstreamAccount> {
        let account =
            ControlPlaneStore::mark_upstream_account_pending_purge(&self.inner, account_id, reason)
                .await?;
        self.persist_state_after_write().await?;
        Ok(account)
    }

    async fn purge_pending_upstream_accounts(&self) -> Result<u64> {
        let purged = ControlPlaneStore::purge_pending_upstream_accounts(&self.inner).await?;
        if purged > 0 {
            self.persist_state_after_write().await?;
        }
        Ok(purged)
    }

    async fn refresh_due_oauth_rate_limit_caches(&self) -> Result<u64> {
        let before = self.current_revision();
        let refreshed =
            ControlPlaneStore::refresh_due_oauth_rate_limit_caches(&self.inner).await?;
        self.persist_if_revision_changed(before).await?;
        Ok(refreshed)
    }

    async fn recover_oauth_rate_limit_refresh_jobs(&self) -> Result<u64> {
        let recovered =
            ControlPlaneStore::recover_oauth_rate_limit_refresh_jobs(&self.inner).await?;
        if recovered > 0 {
            self.persist_state_after_write().await?;
        }
        Ok(recovered)
    }

    async fn create_oauth_rate_limit_refresh_job(&self) -> Result<OAuthRateLimitRefreshJobSummary> {
        let summary = self.inner.create_oauth_rate_limit_refresh_job().await?;
        self.persist_state_after_write().await?;
        Ok(summary)
    }

    async fn oauth_rate_limit_refresh_job(
        &self,
        job_id: Uuid,
    ) -> Result<OAuthRateLimitRefreshJobSummary> {
        self.inner.oauth_rate_limit_refresh_job(job_id).await
    }

    async fn run_oauth_rate_limit_refresh_job(&self, job_id: Uuid) -> Result<()> {
        self.inner.run_oauth_rate_limit_refresh_job(job_id).await?;
        self.persist_state_after_write().await?;
        Ok(())
    }

    async fn flush_snapshot_revision(&self, max_batch: usize) -> Result<u32> {
        ControlPlaneStore::flush_snapshot_revision(&self.inner, max_batch).await
    }

    async fn set_oauth_family_enabled(
        &self,
        account_id: Uuid,
        enabled: bool,
    ) -> Result<OAuthFamilyActionResponse> {
        let response = self.inner.set_oauth_family_enabled(account_id, enabled).await?;
        self.persist_state_after_write().await?;
        Ok(response)
    }

    async fn snapshot(&self) -> Result<DataPlaneSnapshot> {
        ControlPlaneStore::snapshot(&self.inner).await
    }

    async fn cleanup_data_plane_outbox(&self, retention: Duration) -> Result<u64> {
        ControlPlaneStore::cleanup_data_plane_outbox(&self.inner, retention).await
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
        ControlPlaneStore::data_plane_snapshot_events(&self.inner, after, limit).await
    }

    async fn mark_account_seen_ok(
        &self,
        account_id: Uuid,
        seen_ok_at: DateTime<Utc>,
        min_write_interval_sec: i64,
    ) -> Result<bool> {
        let changed = ControlPlaneStore::mark_account_seen_ok(
            &self.inner,
            account_id,
            seen_ok_at,
            min_write_interval_sec,
        )
        .await?;
        if changed {
            self.persist_state_after_write().await?;
        }
        Ok(changed)
    }

    async fn maybe_refresh_oauth_rate_limit_cache_on_seen_ok(
        &self,
        account_id: Uuid,
    ) -> Result<()> {
        let before = self.current_revision();
        ControlPlaneStore::maybe_refresh_oauth_rate_limit_cache_on_seen_ok(&self.inner, account_id)
            .await?;
        self.persist_if_revision_changed(before).await
    }

    async fn update_oauth_rate_limit_cache_from_observation(
        &self,
        account_id: Uuid,
        rate_limits: Vec<OAuthRateLimitSnapshot>,
        observed_at: DateTime<Utc>,
    ) -> Result<()> {
        ControlPlaneStore::update_oauth_rate_limit_cache_from_observation(
            &self.inner,
            account_id,
            rate_limits,
            observed_at,
        )
        .await?;
        self.persist_state_after_write().await
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
        let changed = ControlPlaneStore::record_upstream_account_live_result(
            &self.inner,
            account_id,
            reported_at,
            status,
            source,
            status_code,
            error_code,
            error_message_preview,
        )
        .await?;
        self.persist_state_after_write().await?;
        Ok(changed)
    }
}

#[cfg(test)]
mod sqlite_backed_store_tests {
    use super::{
        normalize_sqlite_database_url, AccountPoolState, OAuthVaultRecordStatus,
        SqliteBackedStore,
    };
    use crate::contracts::{
        ImportOAuthRefreshTokenRequest, OAuthAccountPoolState, OAuthInventoryFailureStage,
    };
    use crate::crypto::CredentialCipher;
    use crate::oauth::{
        OAuthRefreshErrorCode, OAuthTokenClient, OAuthTokenClientError, OAuthTokenInfo,
    };
    use crate::store::ControlPlaneStore;
    use crate::contracts::{
        CreateApiKeyRequest, CreateTenantRequest, CreateUpstreamAccountRequest,
    };
    use crate::store::UpsertOneTimeSessionAccountRequest;
    use async_trait::async_trait;
    use base64::Engine;
    use chrono::{Duration, Utc};
    use codex_pool_core::model::{UpstreamAuthProvider, UpstreamMode};
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, LazyLock, Mutex};
    use tokio::sync::Barrier;
    use uuid::Uuid;

    static SQLITE_ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    fn temp_sqlite_url(name: &str) -> String {
        let path = std::env::temp_dir().join(format!("{name}-{}.sqlite3", Uuid::new_v4()));
        normalize_sqlite_database_url(&path.display().to_string())
    }

    fn test_cipher(seed: u8) -> CredentialCipher {
        CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([seed; 32]),
        )
        .expect("build credential cipher")
    }

    #[derive(Clone)]
    struct StaticOAuthTokenClient;

    #[async_trait]
    impl OAuthTokenClient for StaticOAuthTokenClient {
        async fn refresh_token(
            &self,
            refresh_token: &str,
            _base_url: Option<&str>,
        ) -> Result<OAuthTokenInfo, crate::oauth::OAuthTokenClientError> {
            Ok(OAuthTokenInfo {
                access_token: format!("access-{refresh_token}"),
                refresh_token: refresh_token.to_string(),
                expires_at: Utc::now() + Duration::seconds(3600),
                token_type: Some("Bearer".to_string()),
                scope: Some("model.read".to_string()),
                email: Some("sqlite-oauth@example.com".to_string()),
                oauth_subject: Some("auth0|sqlite-oauth".to_string()),
                oauth_identity_provider: Some("google-oauth2".to_string()),
                email_verified: Some(true),
                chatgpt_account_id: Some("acct_sqlite_demo".to_string()),
                chatgpt_user_id: Some("user_sqlite_demo".to_string()),
                chatgpt_plan_type: Some("free".to_string()),
                chatgpt_subscription_active_start: None,
                chatgpt_subscription_active_until: None,
                chatgpt_subscription_last_checked: None,
                chatgpt_account_user_id: Some("acct_user_sqlite_demo".to_string()),
                chatgpt_compute_residency: Some("us".to_string()),
                workspace_name: None,
                organizations: Some(vec![json!({
                    "id": "org_sqlite_demo",
                    "title": "Personal",
                })]),
                groups: Some(vec![]),
            })
        }
    }

    #[derive(Clone)]
    struct CountingOAuthTokenClient {
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl OAuthTokenClient for CountingOAuthTokenClient {
        async fn refresh_token(
            &self,
            refresh_token: &str,
            _base_url: Option<&str>,
        ) -> Result<OAuthTokenInfo, crate::oauth::OAuthTokenClientError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(OAuthTokenInfo {
                access_token: format!("counting-access-{refresh_token}"),
                refresh_token: refresh_token.to_string(),
                expires_at: Utc::now() + Duration::seconds(3600),
                token_type: Some("Bearer".to_string()),
                scope: Some("model.read".to_string()),
                email: Some("counting-sqlite-oauth@example.com".to_string()),
                oauth_subject: Some("auth0|counting-sqlite-oauth".to_string()),
                oauth_identity_provider: Some("google-oauth2".to_string()),
                email_verified: Some(true),
                chatgpt_account_id: Some(format!("acct_{refresh_token}")),
                chatgpt_user_id: Some(format!("user_{refresh_token}")),
                chatgpt_plan_type: Some("free".to_string()),
                chatgpt_subscription_active_start: None,
                chatgpt_subscription_active_until: None,
                chatgpt_subscription_last_checked: None,
                chatgpt_account_user_id: Some(format!("acct_user_{refresh_token}")),
                chatgpt_compute_residency: Some("us".to_string()),
                workspace_name: None,
                organizations: Some(vec![json!({
                    "id": "org_counting_sqlite_demo",
                    "title": "Personal",
                })]),
                groups: Some(vec![]),
            })
        }
    }

    #[derive(Clone)]
    struct AdmissionProbeOAuthTokenClient {
        refresh_calls: Arc<AtomicUsize>,
        rate_limits: Vec<crate::contracts::OAuthRateLimitSnapshot>,
    }

    #[async_trait]
    impl OAuthTokenClient for AdmissionProbeOAuthTokenClient {
        async fn refresh_token(
            &self,
            refresh_token: &str,
            _base_url: Option<&str>,
        ) -> Result<OAuthTokenInfo, crate::oauth::OAuthTokenClientError> {
            self.refresh_calls.fetch_add(1, Ordering::SeqCst);
            StaticOAuthTokenClient
                .refresh_token(refresh_token, None)
                .await
        }

        async fn fetch_rate_limits(
            &self,
            _access_token: &str,
            _base_url: Option<&str>,
            _chatgpt_account_id: Option<&str>,
        ) -> Result<Vec<crate::contracts::OAuthRateLimitSnapshot>, crate::oauth::OAuthTokenClientError> {
            Ok(self.rate_limits.clone())
        }
    }

    #[derive(Clone)]
    struct RateLimitedOAuthTokenClient;

    #[async_trait]
    impl OAuthTokenClient for RateLimitedOAuthTokenClient {
        async fn refresh_token(
            &self,
            _refresh_token: &str,
            _base_url: Option<&str>,
        ) -> Result<OAuthTokenInfo, crate::oauth::OAuthTokenClientError> {
            Err(OAuthTokenClientError::Upstream {
                code: OAuthRefreshErrorCode::RateLimited,
                message: "rate_limited: upstream busy".to_string(),
            })
        }
    }

    #[derive(Clone)]
    struct RevokedOAuthTokenClient;

    #[async_trait]
    impl OAuthTokenClient for RevokedOAuthTokenClient {
        async fn refresh_token(
            &self,
            _refresh_token: &str,
            _base_url: Option<&str>,
        ) -> Result<OAuthTokenInfo, crate::oauth::OAuthTokenClientError> {
            Err(OAuthTokenClientError::InvalidRefreshToken {
                code: OAuthRefreshErrorCode::RefreshTokenRevoked,
                message: "refresh token revoked by upstream".to_string(),
            })
        }
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

    #[tokio::test]
    async fn sqlite_backed_store_snapshot_cursor_matches_revision() {
        let database_url = temp_sqlite_url("cp-store-snapshot-cursor");
        let store = SqliteBackedStore::connect(&database_url)
            .await
            .expect("connect sqlite store");

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

        let snapshot = store.snapshot().await.expect("load sqlite snapshot");
        assert_eq!(snapshot.cursor, snapshot.revision);
    }

    #[tokio::test]
    async fn sqlite_backed_store_revision_subscription_observes_writes() {
        let database_url = temp_sqlite_url("cp-store-revision-subscribe");
        let store = SqliteBackedStore::connect(&database_url)
            .await
            .expect("connect sqlite store");
        let mut revision_rx = store.subscribe_revisions();
        let initial_revision = *revision_rx.borrow();

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

        tokio::time::timeout(std::time::Duration::from_secs(1), revision_rx.changed())
            .await
            .expect("revision notification should arrive")
            .expect("revision sender should stay open");
        assert!(*revision_rx.borrow() > initial_revision);
    }

    #[tokio::test]
    async fn sqlite_backed_store_concurrent_deletes_should_not_persist_every_item() {
        let database_url = temp_sqlite_url("cp-store-delete-persist-coalesce");
        let store = Arc::new(
            SqliteBackedStore::connect(&database_url)
                .await
                .expect("connect sqlite store"),
        );

        let mut account_ids = Vec::new();
        for index in 0..12 {
            let account = store
                .create_upstream_account(CreateUpstreamAccountRequest {
                    label: format!("delete-target-{index}"),
                    mode: UpstreamMode::OpenAiApiKey,
                    base_url: "https://api.openai.com".to_string(),
                    bearer_token: format!("delete-token-{index}"),
                    chatgpt_account_id: None,
                    auth_provider: Some(UpstreamAuthProvider::LegacyBearer),
                    enabled: Some(true),
                    priority: Some(10),
                })
                .await
                .expect("create upstream account");
            account_ids.push(account.id);
        }

        let baseline_persist_writes = store.persist_write_count();
        let barrier = Arc::new(Barrier::new(account_ids.len() + 1));
        let mut handles = Vec::new();

        for account_id in account_ids {
            let store = store.clone();
            let barrier = barrier.clone();
            handles.push(tokio::spawn(async move {
                barrier.wait().await;
                store
                    .delete_upstream_account(account_id)
                    .await
                    .expect("delete upstream account");
            }));
        }

        barrier.wait().await;
        for handle in handles {
            handle.await.expect("join delete task");
        }

        let persist_writes = store.persist_write_count() - baseline_persist_writes;
        assert!(
            persist_writes < 12,
            "expected concurrent deletes to coalesce persists, got {persist_writes} writes for 12 deletes"
        );
        assert!(
            store.list_upstream_accounts().await.expect("list accounts").is_empty(),
            "all accounts should be deleted"
        );
    }

    #[tokio::test]
    async fn sqlite_backed_store_mixed_delete_and_queue_import_should_not_persist_every_item() {
        let database_url = temp_sqlite_url("cp-store-mixed-persist-coalesce");
        let store = Arc::new(
            SqliteBackedStore::connect_with_oauth(
                &database_url,
                Arc::new(StaticOAuthTokenClient),
                Some(test_cipher(29)),
            )
            .await
            .expect("connect sqlite store with oauth"),
        );

        let mut account_ids = Vec::new();
        for index in 0..8 {
            let account = store
                .create_upstream_account(CreateUpstreamAccountRequest {
                    label: format!("mixed-delete-{index}"),
                    mode: UpstreamMode::OpenAiApiKey,
                    base_url: "https://api.openai.com".to_string(),
                    bearer_token: format!("mixed-delete-token-{index}"),
                    chatgpt_account_id: None,
                    auth_provider: Some(UpstreamAuthProvider::LegacyBearer),
                    enabled: Some(true),
                    priority: Some(10),
                })
                .await
                .expect("create upstream account");
            account_ids.push(account.id);
        }

        let import_requests = (0..8)
            .map(|index| ImportOAuthRefreshTokenRequest {
                label: format!("mixed-queue-{index}"),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: format!("rt-mixed-queue-{index}"),
                fallback_access_token: None,
                fallback_token_expires_at: None,
                chatgpt_account_id: Some(format!("acct-mixed-queue-{index}")),
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: Some("free".to_string()),
                source_type: Some("codex".to_string()),
            })
            .collect::<Vec<_>>();

        let baseline_persist_writes = store.persist_write_count();
        let total_operations = account_ids.len() + import_requests.len();
        let barrier = Arc::new(Barrier::new(total_operations + 1));
        let mut handles = Vec::new();

        for account_id in account_ids {
            let store = store.clone();
            let barrier = barrier.clone();
            handles.push(tokio::spawn(async move {
                barrier.wait().await;
                store
                    .delete_upstream_account(account_id)
                    .await
                    .expect("delete upstream account");
            }));
        }

        for request in import_requests {
            let store = store.clone();
            let barrier = barrier.clone();
            handles.push(tokio::spawn(async move {
                barrier.wait().await;
                let created = store
                    .queue_oauth_refresh_token(request)
                    .await
                    .expect("queue oauth refresh token");
                assert!(created, "each queued refresh token should create a new vault item");
            }));
        }

        barrier.wait().await;
        for handle in handles {
            handle.await.expect("join mixed write task");
        }

        let persist_writes = store.persist_write_count() - baseline_persist_writes;
        assert!(
            persist_writes < total_operations,
            "expected mixed concurrent writes to coalesce persists, got {persist_writes} writes for {total_operations} operations"
        );
        assert!(
            store.list_upstream_accounts().await.expect("list accounts").is_empty(),
            "deletes should remove all runtime accounts while queued imports stay cold"
        );
        assert_eq!(
            store.inner.oauth_refresh_token_vault.read().unwrap().len(),
            8,
            "all queued imports should be retained in the vault"
        );
    }

    #[tokio::test]
    async fn sqlite_backed_store_queue_oauth_refresh_token_keeps_account_out_of_runtime_until_activation(
    ) {
        let database_url = temp_sqlite_url("cp-store-sqlite-vault-queue");
        let store = SqliteBackedStore::connect_with_oauth(
            &database_url,
            Arc::new(StaticOAuthTokenClient),
            Some(test_cipher(21)),
        )
        .await
        .expect("connect sqlite store with oauth");

        let created = store
            .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "sqlite-vault-demo".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-sqlite-vault-demo".to_string(),
                fallback_access_token: None,
                fallback_token_expires_at: None,
                chatgpt_account_id: Some("acct_sqlite_demo".to_string()),
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: Some("free".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("queue oauth refresh token");

        assert!(created);

        let queued_accounts = store
            .list_upstream_accounts()
            .await
            .expect("list accounts after queue");
        assert!(queued_accounts.is_empty(), "queued oauth account should stay cold before activation");

        let activated = store
            .activate_oauth_refresh_token_vault()
            .await
            .expect("activate oauth vault");
        assert_eq!(activated, 1);

        let active_accounts = store
            .list_upstream_accounts()
            .await
            .expect("list accounts after activation");
        assert_eq!(active_accounts.len(), 1);
        assert_eq!(active_accounts[0].label, "sqlite-vault-demo");
    }

    #[tokio::test]
    async fn sqlite_backed_store_queue_oauth_refresh_token_persists_ready_inventory_without_refresh()
    {
        let database_url = temp_sqlite_url("cp-store-sqlite-vault-ready");
        let refresh_calls = Arc::new(AtomicUsize::new(0));
        let store = SqliteBackedStore::connect_with_oauth(
            &database_url,
            Arc::new(AdmissionProbeOAuthTokenClient {
                refresh_calls: refresh_calls.clone(),
                rate_limits: vec![crate::contracts::OAuthRateLimitSnapshot {
                    limit_id: Some("five_hours".to_string()),
                    limit_name: Some("5 hours".to_string()),
                    primary: Some(crate::contracts::OAuthRateLimitWindow {
                        used_percent: 18.0,
                        window_minutes: Some(300),
                        resets_at: Some(Utc::now() + Duration::minutes(30)),
                    }),
                    secondary: None,
                }],
            }),
            Some(test_cipher(31)),
        )
        .await
        .expect("connect sqlite store with oauth");

        store
            .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "sqlite-vault-ready".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-sqlite-vault-ready".to_string(),
                fallback_access_token: Some("ak-sqlite-ready".to_string()),
                fallback_token_expires_at: Some(Utc::now() + Duration::hours(2)),
                chatgpt_account_id: Some("acct_sqlite_ready".to_string()),
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: Some("free".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("queue oauth refresh token");

        assert_eq!(refresh_calls.load(Ordering::SeqCst), 0);
        assert!(
            store.list_upstream_accounts().await.expect("list accounts").is_empty(),
            "ready inventory should stay out of runtime before activation"
        );

        let summary = store
            .oauth_inventory_summary()
            .await
            .expect("inventory summary");
        assert_eq!(summary.ready, 1);

        let records = store
            .oauth_inventory_records()
            .await
            .expect("inventory records");
        let record = records.first().expect("inventory record");
        assert_eq!(record.vault_status, OAuthVaultRecordStatus::Ready);
        assert!(record.admission_checked_at.is_some());
    }

    #[tokio::test]
    async fn sqlite_backed_store_queue_oauth_refresh_token_persists_no_quota_inventory() {
        let database_url = temp_sqlite_url("cp-store-sqlite-vault-no-quota");
        let refresh_calls = Arc::new(AtomicUsize::new(0));
        let store = SqliteBackedStore::connect_with_oauth(
            &database_url,
            Arc::new(AdmissionProbeOAuthTokenClient {
                refresh_calls: refresh_calls.clone(),
                rate_limits: vec![crate::contracts::OAuthRateLimitSnapshot {
                    limit_id: Some("five_hours".to_string()),
                    limit_name: Some("5 hours".to_string()),
                    primary: Some(crate::contracts::OAuthRateLimitWindow {
                        used_percent: 100.0,
                        window_minutes: Some(300),
                        resets_at: Some(Utc::now() + Duration::minutes(45)),
                    }),
                    secondary: None,
                }],
            }),
            Some(test_cipher(32)),
        )
        .await
        .expect("connect sqlite store with oauth");

        store
            .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "sqlite-vault-no-quota".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-sqlite-vault-no-quota".to_string(),
                fallback_access_token: Some("ak-sqlite-no-quota".to_string()),
                fallback_token_expires_at: Some(Utc::now() + Duration::hours(2)),
                chatgpt_account_id: Some("acct_sqlite_no_quota".to_string()),
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: Some("free".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("queue oauth refresh token");

        assert_eq!(refresh_calls.load(Ordering::SeqCst), 0);

        let records = store
            .oauth_inventory_records()
            .await
            .expect("inventory records");
        let record = records.first().expect("inventory record");
        assert_eq!(record.vault_status, OAuthVaultRecordStatus::NoQuota);
        assert!(record.admission_retry_after.is_some());
    }

    #[tokio::test]
    async fn sqlite_backed_store_activation_uses_ready_without_refresh_and_needs_refresh_with_single_refresh()
    {
        let database_url = temp_sqlite_url("cp-store-sqlite-ready-then-refresh");
        let refresh_calls = Arc::new(AtomicUsize::new(0));
        let store = SqliteBackedStore::connect_with_oauth(
            &database_url,
            Arc::new(AdmissionProbeOAuthTokenClient {
                refresh_calls: refresh_calls.clone(),
                rate_limits: vec![crate::contracts::OAuthRateLimitSnapshot {
                    limit_id: Some("five_hours".to_string()),
                    limit_name: Some("5 hours".to_string()),
                    primary: Some(crate::contracts::OAuthRateLimitWindow {
                        used_percent: 22.0,
                        window_minutes: Some(300),
                        resets_at: Some(Utc::now() + Duration::minutes(25)),
                    }),
                    secondary: None,
                }],
            }),
            Some(test_cipher(33)),
        )
        .await
        .expect("connect sqlite store with oauth");

        store
            .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "sqlite-ready-first".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-sqlite-ready-first".to_string(),
                fallback_access_token: Some("ak-sqlite-ready-first".to_string()),
                fallback_token_expires_at: Some(Utc::now() + Duration::hours(2)),
                chatgpt_account_id: Some("acct_sqlite_ready_first".to_string()),
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: Some("free".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("queue ready oauth refresh token");
        store
            .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "sqlite-needs-refresh".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-sqlite-needs-refresh".to_string(),
                fallback_access_token: Some("opaque-ak-needs-refresh".to_string()),
                fallback_token_expires_at: None,
                chatgpt_account_id: Some("acct_sqlite_needs_refresh".to_string()),
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: Some("free".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("queue needs-refresh oauth refresh token");

        assert_eq!(refresh_calls.load(Ordering::SeqCst), 0);

        let activated = store
            .activate_oauth_refresh_token_vault()
            .await
            .expect("activate oauth vault");
        assert_eq!(activated, 2);
        assert_eq!(
            refresh_calls.load(Ordering::SeqCst),
            1,
            "ready item should materialize without refresh while needs_refresh consumes one refresh"
        );
    }

    #[tokio::test]
    async fn sqlite_backed_store_queued_vault_survives_reopen_before_activation() {
        let database_url = temp_sqlite_url("cp-store-sqlite-vault-reopen");
        let store = SqliteBackedStore::connect_with_oauth(
            &database_url,
            Arc::new(StaticOAuthTokenClient),
            Some(test_cipher(22)),
        )
        .await
        .expect("connect sqlite store with oauth");

        store
            .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "sqlite-vault-reopen".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-sqlite-vault-reopen".to_string(),
                fallback_access_token: None,
                fallback_token_expires_at: None,
                chatgpt_account_id: Some("acct_sqlite_demo".to_string()),
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: Some("free".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("queue oauth refresh token");

        drop(store);

        let reopened = SqliteBackedStore::connect_with_oauth(
            &database_url,
            Arc::new(StaticOAuthTokenClient),
            Some(test_cipher(22)),
        )
        .await
        .expect("reopen sqlite store with oauth");

        let before_activation = reopened
            .list_upstream_accounts()
            .await
            .expect("list accounts after reopen");
        assert!(
            before_activation.is_empty(),
            "queued vault items should survive reopen without becoming runtime accounts"
        );

        let activated = reopened
            .activate_oauth_refresh_token_vault()
            .await
            .expect("activate oauth vault after reopen");
        assert_eq!(activated, 1);

        let after_activation = reopened
            .list_upstream_accounts()
            .await
            .expect("list accounts after reopen activation");
        assert_eq!(after_activation.len(), 1);
        assert_eq!(after_activation[0].label, "sqlite-vault-reopen");
    }

    #[tokio::test]
    async fn sqlite_backed_store_snapshot_excludes_quarantined_accounts() {
        let database_url = temp_sqlite_url("cp-store-sqlite-quarantine-snapshot");
        let store = SqliteBackedStore::connect_with_oauth(
            &database_url,
            Arc::new(StaticOAuthTokenClient),
            Some(test_cipher(23)),
        )
        .await
        .expect("connect sqlite store with oauth");

        store
            .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "sqlite-quarantine".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-sqlite-quarantine".to_string(),
                fallback_access_token: None,
                fallback_token_expires_at: None,
                chatgpt_account_id: Some("acct_sqlite_quarantine".to_string()),
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: Some("free".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("queue oauth refresh token");
        store
            .activate_oauth_refresh_token_vault()
            .await
            .expect("activate vault");

        let account = store
            .list_upstream_accounts()
            .await
            .expect("list accounts")
            .into_iter()
            .next()
            .expect("materialized account");

        {
            let mut health = store.inner.account_health_states.write().unwrap();
            let state = health.entry(account.id).or_default();
            state.pool_state = AccountPoolState::Quarantine;
            state.quarantine_until = Some(Utc::now() + Duration::minutes(5));
            state.quarantine_reason = Some("rate_limited".to_string());
        }
        store
            .persist_state_after_write()
            .await
            .expect("persist quarantine state");

        let snapshot = store.snapshot().await.expect("load snapshot");
        assert!(
            snapshot.accounts.iter().all(|item| item.id != account.id),
            "quarantined account should not enter runtime snapshot"
        );
    }

    #[tokio::test]
    async fn sqlite_backed_store_snapshot_recovers_expired_quarantine() {
        let database_url = temp_sqlite_url("cp-store-sqlite-quarantine-recover");
        let store = SqliteBackedStore::connect_with_oauth(
            &database_url,
            Arc::new(StaticOAuthTokenClient),
            Some(test_cipher(26)),
        )
        .await
        .expect("connect sqlite store with oauth");

        store
            .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "sqlite-quarantine-recover".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-sqlite-quarantine-recover".to_string(),
                fallback_access_token: None,
                fallback_token_expires_at: None,
                chatgpt_account_id: Some("acct_sqlite_quarantine_recover".to_string()),
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: Some("free".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("queue oauth refresh token");
        store
            .activate_oauth_refresh_token_vault()
            .await
            .expect("activate vault");

        let account = store
            .list_upstream_accounts()
            .await
            .expect("list accounts")
            .into_iter()
            .next()
            .expect("materialized account");

        {
            let mut health = store.inner.account_health_states.write().unwrap();
            let state = health.entry(account.id).or_default();
            state.pool_state = AccountPoolState::Quarantine;
            state.quarantine_until = Some(Utc::now() - Duration::seconds(5));
            state.quarantine_reason = Some("rate_limited".to_string());
        }
        store
            .persist_state_after_write()
            .await
            .expect("persist quarantine state");

        let snapshot = store.snapshot().await.expect("load snapshot");
        assert!(
            snapshot.accounts.iter().any(|item| item.id == account.id),
            "expired quarantine should automatically recover into runtime snapshot"
        );

        let status = store
            .oauth_account_status(account.id)
            .await
            .expect("load oauth account status after recovery");
        assert_eq!(status.pool_state, OAuthAccountPoolState::Active);
        assert_eq!(status.quarantine_reason, None);
        assert_eq!(status.quarantine_until, None);
    }

    #[tokio::test]
    async fn sqlite_backed_store_refresh_expiring_oauth_accounts_skips_quarantined_accounts() {
        let database_url = temp_sqlite_url("cp-store-sqlite-quarantine-refresh");
        let calls = Arc::new(AtomicUsize::new(0));
        let store = SqliteBackedStore::connect_with_oauth(
            &database_url,
            Arc::new(CountingOAuthTokenClient {
                calls: calls.clone(),
            }),
            Some(test_cipher(24)),
        )
        .await
        .expect("connect sqlite store with oauth");

        for (label, refresh_token) in [
            ("sqlite-active-refresh", "rt-sqlite-active-refresh"),
            ("sqlite-quarantine-refresh", "rt-sqlite-quarantine-refresh"),
        ] {
            store
                .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                    label: label.to_string(),
                    base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                    refresh_token: refresh_token.to_string(),
                    fallback_access_token: None,
                    fallback_token_expires_at: None,
                    chatgpt_account_id: None,
                    mode: Some(UpstreamMode::CodexOauth),
                    enabled: Some(true),
                    priority: Some(100),
                    chatgpt_plan_type: Some("free".to_string()),
                    source_type: Some("codex".to_string()),
                })
                .await
                .expect("queue oauth refresh token");
        }

        let activated = store
            .activate_oauth_refresh_token_vault()
            .await
            .expect("activate vault");
        assert_eq!(activated, 2);
        calls.store(0, Ordering::SeqCst);

        let accounts = store
            .list_upstream_accounts()
            .await
            .expect("list accounts after activation");
        let quarantine_id = accounts
            .iter()
            .find(|item| item.label == "sqlite-quarantine-refresh")
            .map(|item| item.id)
            .expect("quarantine account id");

        {
            let mut credentials = store.inner.oauth_credentials.write().unwrap();
            for credential in credentials.values_mut() {
                credential.token_expires_at = Utc::now() + Duration::seconds(30);
            }
        }
        {
            let mut health = store.inner.account_health_states.write().unwrap();
            let state = health.entry(quarantine_id).or_default();
            state.pool_state = AccountPoolState::Quarantine;
            state.quarantine_until = Some(Utc::now() + Duration::minutes(5));
            state.quarantine_reason = Some("rate_limited".to_string());
        }
        store
            .persist_state_after_write()
            .await
            .expect("persist quarantine state");

        store
            .refresh_expiring_oauth_accounts()
            .await
            .expect("refresh expiring oauth accounts");

        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "refresh loop should only touch active expiring accounts"
        );
    }

    #[tokio::test]
    async fn sqlite_backed_store_activation_backoffs_nonfatal_vault_failures() {
        let database_url = temp_sqlite_url("cp-store-sqlite-vault-backoff");
        let store = SqliteBackedStore::connect_with_oauth(
            &database_url,
            Arc::new(RateLimitedOAuthTokenClient),
            Some(test_cipher(27)),
        )
        .await
        .expect("connect sqlite store with oauth");

        store
            .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "sqlite-vault-backoff".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-sqlite-vault-backoff".to_string(),
                fallback_access_token: None,
                fallback_token_expires_at: None,
                chatgpt_account_id: Some("acct_sqlite_vault_backoff".to_string()),
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: Some("free".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("queue oauth refresh token");

        let activated = store
            .activate_oauth_refresh_token_vault()
            .await
            .expect("activate oauth vault");
        assert_eq!(activated, 0);
        assert!(
            store.list_upstream_accounts().await.expect("list accounts").is_empty(),
            "nonfatal activation failures should keep item in vault instead of materializing account"
        );

        let vault = store.inner.oauth_refresh_token_vault.read().unwrap();
        let record = vault.values().next().expect("vault record");
        assert_eq!(record.status, OAuthVaultRecordStatus::NeedsRefresh);
        assert_eq!(record.failure_count, 1);
        assert_eq!(record.last_error_code.as_deref(), Some("rate_limited"));
        assert!(record.backoff_until.is_some());
        assert_eq!(record.next_attempt_at, record.backoff_until);
    }

    #[tokio::test]
    async fn sqlite_backed_store_runtime_pool_cap_blocks_oauth_activation() {
        let _guard = SQLITE_ENV_LOCK.lock().expect("lock sqlite env");
        let old_target = std::env::var("CONTROL_PLANE_ACTIVE_POOL_TARGET").ok();
        let old_min = std::env::var("CONTROL_PLANE_ACTIVE_POOL_MIN").ok();
        let old_batch = std::env::var("CONTROL_PLANE_VAULT_ACTIVATE_BATCH_SIZE").ok();
        let old_concurrency = std::env::var("CONTROL_PLANE_VAULT_ACTIVATE_CONCURRENCY").ok();
        let old_max_rps = std::env::var("CONTROL_PLANE_VAULT_ACTIVATE_MAX_RPS").ok();
        let old_runtime_cap = std::env::var("CONTROL_PLANE_RUNTIME_POOL_CAP").ok();

        std::env::set_var("CONTROL_PLANE_ACTIVE_POOL_TARGET", "4");
        std::env::set_var("CONTROL_PLANE_ACTIVE_POOL_MIN", "0");
        std::env::set_var("CONTROL_PLANE_VAULT_ACTIVATE_BATCH_SIZE", "4");
        std::env::set_var("CONTROL_PLANE_VAULT_ACTIVATE_CONCURRENCY", "1");
        std::env::set_var("CONTROL_PLANE_VAULT_ACTIVATE_MAX_RPS", "10");
        std::env::set_var("CONTROL_PLANE_RUNTIME_POOL_CAP", "2");

        let database_url = temp_sqlite_url("cp-store-sqlite-runtime-cap");
        let store = SqliteBackedStore::connect_with_oauth(
            &database_url,
            Arc::new(StaticOAuthTokenClient),
            Some(test_cipher(29)),
        )
        .await
        .expect("connect sqlite store with runtime cap");

        for index in 0..2 {
            store
                .upsert_one_time_session_account(UpsertOneTimeSessionAccountRequest {
                    label: format!("legacy-runtime-{index}"),
                    mode: UpstreamMode::CodexOauth,
                    base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                    access_token: format!("legacy-runtime-token-{index}"),
                    chatgpt_account_id: Some(format!("acct_runtime_legacy_{index}")),
                    enabled: Some(true),
                    priority: Some(100),
                    token_expires_at: Some(Utc::now() + Duration::hours(2)),
                    chatgpt_plan_type: Some("free".to_string()),
                    source_type: Some("codex".to_string()),
                })
                .await
                .expect("upsert legacy runtime account");
        }

        store
            .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "runtime-cap-oauth".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-runtime-cap-oauth".to_string(),
                fallback_access_token: Some("ak-runtime-cap-oauth".to_string()),
                fallback_token_expires_at: Some(Utc::now() + Duration::hours(2)),
                chatgpt_account_id: Some("acct_runtime_cap_oauth".to_string()),
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: Some("free".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("queue oauth refresh token under runtime cap");

        let activated = store
            .activate_oauth_refresh_token_vault()
            .await
            .expect("activate oauth vault with runtime cap");
        assert_eq!(activated, 0);
        assert_eq!(
            store
                .list_upstream_accounts()
                .await
                .expect("list accounts after runtime-cap activation")
                .len(),
            2,
            "legacy bearer runtime accounts should consume unified runtime cap slots"
        );
        let inventory = store
            .oauth_inventory_summary()
            .await
            .expect("load inventory summary after runtime-cap activation");
        assert_eq!(inventory.total, 1);

        if let Some(value) = old_target {
            std::env::set_var("CONTROL_PLANE_ACTIVE_POOL_TARGET", value);
        } else {
            std::env::remove_var("CONTROL_PLANE_ACTIVE_POOL_TARGET");
        }
        if let Some(value) = old_min {
            std::env::set_var("CONTROL_PLANE_ACTIVE_POOL_MIN", value);
        } else {
            std::env::remove_var("CONTROL_PLANE_ACTIVE_POOL_MIN");
        }
        if let Some(value) = old_batch {
            std::env::set_var("CONTROL_PLANE_VAULT_ACTIVATE_BATCH_SIZE", value);
        } else {
            std::env::remove_var("CONTROL_PLANE_VAULT_ACTIVATE_BATCH_SIZE");
        }
        if let Some(value) = old_concurrency {
            std::env::set_var("CONTROL_PLANE_VAULT_ACTIVATE_CONCURRENCY", value);
        } else {
            std::env::remove_var("CONTROL_PLANE_VAULT_ACTIVATE_CONCURRENCY");
        }
        if let Some(value) = old_max_rps {
            std::env::set_var("CONTROL_PLANE_VAULT_ACTIVATE_MAX_RPS", value);
        } else {
            std::env::remove_var("CONTROL_PLANE_VAULT_ACTIVATE_MAX_RPS");
        }
        if let Some(value) = old_runtime_cap {
            std::env::set_var("CONTROL_PLANE_RUNTIME_POOL_CAP", value);
        } else {
            std::env::remove_var("CONTROL_PLANE_RUNTIME_POOL_CAP");
        }
    }

    #[tokio::test]
    async fn sqlite_backed_store_activation_marks_fatal_vault_failures_failed() {
        let database_url = temp_sqlite_url("cp-store-sqlite-vault-fatal");
        let store = SqliteBackedStore::connect_with_oauth(
            &database_url,
            Arc::new(RevokedOAuthTokenClient),
            Some(test_cipher(28)),
        )
        .await
        .expect("connect sqlite store with oauth");

        store
            .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "sqlite-vault-fatal".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-sqlite-vault-fatal".to_string(),
                fallback_access_token: None,
                fallback_token_expires_at: None,
                chatgpt_account_id: Some("acct_sqlite_vault_fatal".to_string()),
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: Some("free".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("queue oauth refresh token");

        for attempt in 0..3 {
            let activated = store
                .activate_oauth_refresh_token_vault()
                .await
                .expect("activate oauth vault");
            assert_eq!(activated, 0);
            assert!(
                store.list_upstream_accounts().await.expect("list accounts").is_empty(),
                "fatal activation failures should never materialize runtime accounts"
            );

            let vault = store.inner.oauth_refresh_token_vault.read().unwrap();
            let record = vault.values().next().expect("vault record");
            assert_eq!(
                record.last_error_code.as_deref(),
                Some("refresh_token_revoked")
            );
            assert_eq!(
                record.failure_stage,
                Some(OAuthInventoryFailureStage::ActivationRefresh)
            );
            assert_eq!(record.failure_count, attempt + 1);

            if attempt < 2 {
                assert_eq!(record.status, OAuthVaultRecordStatus::NeedsRefresh);
                assert!(record.retryable);
                assert!(record.backoff_until.is_some());
                assert!(record.next_attempt_at.is_some());
                assert!(record.next_retry_at.is_some());
                assert_eq!(record.terminal_reason, None);
            } else {
                assert_eq!(record.status, OAuthVaultRecordStatus::Failed);
                assert!(!record.retryable);
                assert_eq!(
                    record.terminal_reason.as_deref(),
                    Some("refresh_token_revoked")
                );
                assert_eq!(record.backoff_until, None);
                assert_eq!(record.next_attempt_at, None);
                assert_eq!(record.next_retry_at, None);
            }
            drop(vault);

            let mut vault = store.inner.oauth_refresh_token_vault.write().unwrap();
            let record = vault.values_mut().next().expect("vault record");
            record.backoff_until = None;
            record.next_attempt_at = Some(Utc::now());
            record.next_retry_at = Some(Utc::now());
        }
    }

    #[tokio::test]
    async fn sqlite_backed_store_oauth_status_exposes_pool_state() {
        let database_url = temp_sqlite_url("cp-store-sqlite-status-pool-state");
        let store = SqliteBackedStore::connect_with_oauth(
            &database_url,
            Arc::new(StaticOAuthTokenClient),
            Some(test_cipher(25)),
        )
        .await
        .expect("connect sqlite store with oauth");

        store
            .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "sqlite-status-pool-state".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-sqlite-status-pool-state".to_string(),
                fallback_access_token: None,
                fallback_token_expires_at: None,
                chatgpt_account_id: None,
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: Some("free".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("queue oauth refresh token");
        store
            .activate_oauth_refresh_token_vault()
            .await
            .expect("activate vault");

        let account = store
            .list_upstream_accounts()
            .await
            .expect("list accounts")
            .into_iter()
            .next()
            .expect("materialized account");
        let expected_until = Utc::now() + Duration::minutes(10);
        {
            let mut health = store.inner.account_health_states.write().unwrap();
            let state = health.entry(account.id).or_default();
            state.pool_state = AccountPoolState::Quarantine;
            state.quarantine_until = Some(expected_until);
            state.quarantine_reason = Some("rate_limited".to_string());
        }

        let status = store
            .oauth_account_status(account.id)
            .await
            .expect("load oauth account status");
        assert_eq!(status.pool_state, OAuthAccountPoolState::Quarantine);
        assert_eq!(status.quarantine_reason.as_deref(), Some("rate_limited"));
        assert_eq!(status.quarantine_until, Some(expected_until));
    }

    #[tokio::test]
    async fn sqlite_backed_store_purges_pending_purge_accounts_after_due_time() {
        let database_url = temp_sqlite_url("cp-store-sqlite-pending-purge");
        let store = SqliteBackedStore::connect_with_oauth(
            &database_url,
            Arc::new(StaticOAuthTokenClient),
            Some(test_cipher(30)),
        )
        .await
        .expect("connect sqlite store with oauth");

        store
            .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "sqlite-pending-purge".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-sqlite-pending-purge".to_string(),
                fallback_access_token: None,
                fallback_token_expires_at: None,
                chatgpt_account_id: Some("acct_sqlite_pending_purge".to_string()),
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: Some("free".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("queue oauth refresh token");
        store
            .activate_oauth_refresh_token_vault()
            .await
            .expect("activate vault");

        let account = store
            .list_upstream_accounts()
            .await
            .expect("list accounts")
            .into_iter()
            .next()
            .expect("materialized account");

        let pending = store
            .mark_upstream_account_pending_purge(
                account.id,
                Some("account_deactivated".to_string()),
            )
            .await
            .expect("mark pending purge");
        assert!(!pending.enabled);

        let status = store
            .oauth_account_status(account.id)
            .await
            .expect("load oauth account status");
        assert_eq!(status.pool_state, OAuthAccountPoolState::PendingPurge);
        assert_eq!(
            status.pending_purge_reason.as_deref(),
            Some("account_deactivated")
        );
        assert!(status.pending_purge_at.is_some());

        let purged = store
            .purge_pending_upstream_accounts()
            .await
            .expect("purge pending accounts before due");
        assert_eq!(purged, 0);

        {
            let mut health = store.inner.account_health_states.write().unwrap();
            let state = health.entry(account.id).or_default();
            state.pending_purge_at = Some(Utc::now() - Duration::seconds(5));
        }
        store
            .persist_state_after_write()
            .await
            .expect("persist pending purge state");

        let purged = store
            .purge_pending_upstream_accounts()
            .await
            .expect("purge pending accounts after due");
        assert_eq!(purged, 1);
        assert!(
            store.list_upstream_accounts().await.expect("list accounts").is_empty(),
            "purged account should be deleted from runtime store"
        );
    }
}
