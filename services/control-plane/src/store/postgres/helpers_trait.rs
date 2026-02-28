fn upstream_mode_to_db(mode: &UpstreamMode) -> &'static str {
    match mode {
        UpstreamMode::OpenAiApiKey => "open_ai_api_key",
        UpstreamMode::ChatGptSession => "chat_gpt_session",
        UpstreamMode::CodexOauth => "codex_oauth",
    }
}

fn parse_upstream_mode(raw: &str) -> Result<UpstreamMode> {
    match raw {
        "open_ai_api_key" => Ok(UpstreamMode::OpenAiApiKey),
        "chat_gpt_session" => Ok(UpstreamMode::ChatGptSession),
        "chat_gpt_oauth" => Ok(UpstreamMode::ChatGptSession),
        "codex_oauth" => Ok(UpstreamMode::CodexOauth),
        "codex_session" => Ok(UpstreamMode::CodexOauth),
        _ => Err(anyhow!("unsupported upstream mode in postgres: {raw}")),
    }
}

fn resolve_oauth_import_mode(mode: Option<UpstreamMode>, source_type: Option<&str>) -> UpstreamMode {
    if let Some(mode) = mode {
        return mode;
    }

    if source_type.is_some_and(|raw| raw.trim().eq_ignore_ascii_case("codex")) {
        return UpstreamMode::CodexOauth;
    }

    UpstreamMode::ChatGptSession
}

fn upstream_auth_provider_to_db(provider: &UpstreamAuthProvider) -> &'static str {
    match provider {
        UpstreamAuthProvider::LegacyBearer => AUTH_PROVIDER_LEGACY_BEARER,
        UpstreamAuthProvider::OAuthRefreshToken => AUTH_PROVIDER_OAUTH_REFRESH_TOKEN,
    }
}

fn parse_upstream_auth_provider(raw: &str) -> Result<UpstreamAuthProvider> {
    match raw {
        AUTH_PROVIDER_LEGACY_BEARER => Ok(UpstreamAuthProvider::LegacyBearer),
        AUTH_PROVIDER_OAUTH_REFRESH_TOKEN => Ok(UpstreamAuthProvider::OAuthRefreshToken),
        _ => Err(anyhow!(
            "unsupported upstream auth provider in postgres: {raw}"
        )),
    }
}

fn parse_oauth_refresh_status(raw: &str) -> Result<OAuthRefreshStatus> {
    match raw {
        "never" => Ok(OAuthRefreshStatus::Never),
        "ok" => Ok(OAuthRefreshStatus::Ok),
        "failed" => Ok(OAuthRefreshStatus::Failed),
        _ => Err(anyhow!(
            "unsupported oauth refresh status in postgres: {raw}"
        )),
    }
}

fn session_credential_kind_to_db(kind: &SessionCredentialKind) -> &'static str {
    match kind {
        SessionCredentialKind::RefreshRotatable => SESSION_CREDENTIAL_KIND_REFRESH_ROTATABLE,
        SessionCredentialKind::OneTimeAccessToken => SESSION_CREDENTIAL_KIND_ONE_TIME_ACCESS_TOKEN,
    }
}

fn parse_session_credential_kind(raw: &str) -> Result<SessionCredentialKind> {
    match raw {
        SESSION_CREDENTIAL_KIND_REFRESH_ROTATABLE => Ok(SessionCredentialKind::RefreshRotatable),
        SESSION_CREDENTIAL_KIND_ONE_TIME_ACCESS_TOKEN => {
            Ok(SessionCredentialKind::OneTimeAccessToken)
        }
        _ => Err(anyhow!(
            "unsupported session credential kind in postgres: {raw}"
        )),
    }
}

fn truncate_error_message(raw: String) -> String {
    const MAX_LEN: usize = 256;
    if raw.len() <= MAX_LEN {
        return raw;
    }

    raw.chars().take(MAX_LEN).collect()
}

fn should_revoke_oauth_token_family(error_code: &str) -> bool {
    matches!(error_code, "refresh_token_reused" | "refresh_token_revoked")
}

fn normalize_error_code_for_health(raw: &str) -> String {
    raw.trim().to_ascii_lowercase()
}

fn is_quota_error_signal(error_code: &str, error_message: &str) -> bool {
    let code = normalize_error_code_for_health(error_code);
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

fn is_auth_error_signal(error_code: &str, error_message: &str) -> bool {
    let code = normalize_error_code_for_health(error_code);
    if matches!(
        code.as_str(),
        "auth_expired"
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

fn is_rate_limited_signal(error_code: &str, error_message: &str) -> bool {
    let code = normalize_error_code_for_health(error_code);
    if matches!(code.as_str(), "rate_limited") {
        return true;
    }
    let message = error_message.to_ascii_lowercase();
    message.contains("rate limit") || message.contains("too many requests")
}

fn is_fatal_refresh_error_code(error_code: Option<&str>) -> bool {
    let Some(error_code) = error_code else {
        return false;
    };
    matches!(
        normalize_error_code_for_health(error_code).as_str(),
        "refresh_token_reused"
            | "refresh_token_revoked"
            | "invalid_refresh_token"
            | "missing_client_id"
            | "unauthorized_client"
    )
}

fn is_blocking_rate_limit_error(
    rate_limits_last_error_code: Option<&str>,
    rate_limits_last_error: Option<&str>,
) -> bool {
    let Some(error_code) = rate_limits_last_error_code else {
        return false;
    };
    let error_message = rate_limits_last_error.unwrap_or_default();
    is_quota_error_signal(error_code, error_message) || is_auth_error_signal(error_code, error_message)
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

fn should_trigger_refresh_after_rate_limit_failure(error_code: &str, error_message: &str) -> bool {
    is_auth_error_signal(error_code, error_message)
        && !matches!(
            normalize_error_code_for_health(error_code).as_str(),
            "refresh_token_reused" | "refresh_token_revoked"
        )
}

#[allow(clippy::too_many_arguments)]
fn oauth_effective_enabled(
    enabled: bool,
    auth_provider: &UpstreamAuthProvider,
    credential_kind: Option<&SessionCredentialKind>,
    token_expires_at: Option<DateTime<Utc>>,
    last_refresh_status: &OAuthRefreshStatus,
    refresh_reused_detected: bool,
    last_refresh_error_code: Option<&str>,
    rate_limits_expires_at: Option<DateTime<Utc>>,
    rate_limits_last_error_code: Option<&str>,
    rate_limits_last_error: Option<&str>,
    now: DateTime<Utc>,
) -> bool {
    let base_enabled = match (auth_provider, credential_kind) {
        (UpstreamAuthProvider::OAuthRefreshToken, _) => {
            enabled
                && token_expires_at
                    .is_some_and(|expires_at| expires_at > now + Duration::seconds(OAUTH_MIN_VALID_SEC))
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
        if refresh_reused_detected {
            return false;
        }
        if matches!(last_refresh_status, OAuthRefreshStatus::Failed)
            && is_fatal_refresh_error_code(last_refresh_error_code)
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

fn routing_strategy_to_db(strategy: &RoutingStrategy) -> &'static str {
    match strategy {
        RoutingStrategy::RoundRobin => "round_robin",
        RoutingStrategy::FillFirst => "fill_first",
    }
}

fn parse_routing_strategy(raw: &str) -> Result<RoutingStrategy> {
    match raw {
        "round_robin" => Ok(RoutingStrategy::RoundRobin),
        "fill_first" => Ok(RoutingStrategy::FillFirst),
        _ => Err(anyhow!("unsupported routing strategy in postgres: {raw}")),
    }
}

fn parse_routing_policy_row(row: &sqlx::postgres::PgRow) -> Result<RoutingPolicy> {
    let max_retries_i64 = row.try_get::<i64, _>("max_retries")?;
    let stream_max_retries_i64 = row.try_get::<i64, _>("stream_max_retries")?;
    Ok(RoutingPolicy {
        tenant_id: row.try_get("tenant_id")?,
        strategy: parse_routing_strategy(row.try_get::<String, _>("strategy")?.as_str())?,
        max_retries: u32::try_from(max_retries_i64).context("max_retries out of range")?,
        stream_max_retries: u32::try_from(stream_max_retries_i64)
            .context("stream_max_retries out of range")?,
        updated_at: row.try_get::<DateTime<Utc>, _>("updated_at")?,
    })
}

fn hash_api_key_token(token: &str) -> String {
    crate::security::hash_api_key_token(token)
}

fn refresh_token_sha256(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

#[async_trait]
impl ControlPlaneStore for PostgresStore {
    fn postgres_pool(&self) -> Option<PgPool> {
        Some(self.clone_pool())
    }

    async fn create_tenant(&self, req: CreateTenantRequest) -> Result<Tenant> {
        self.insert_tenant(req).await
    }

    async fn list_tenants(&self) -> Result<Vec<Tenant>> {
        self.fetch_tenants().await
    }

    async fn create_api_key(&self, req: CreateApiKeyRequest) -> Result<CreateApiKeyResponse> {
        self.insert_api_key(req).await
    }

    async fn list_api_keys(&self) -> Result<Vec<ApiKey>> {
        self.fetch_api_keys().await
    }

    async fn set_api_key_enabled(&self, api_key_id: Uuid, enabled: bool) -> Result<ApiKey> {
        self.set_api_key_enabled_inner(api_key_id, enabled).await
    }

    async fn validate_api_key(&self, token: &str) -> Result<Option<ValidatedPrincipal>> {
        self.fetch_validated_principal_by_token(token).await
    }

    async fn create_upstream_account(
        &self,
        req: CreateUpstreamAccountRequest,
    ) -> Result<UpstreamAccount> {
        self.insert_upstream_account(req).await
    }

    async fn list_upstream_accounts(&self) -> Result<Vec<UpstreamAccount>> {
        self.fetch_upstream_accounts().await
    }

    async fn set_upstream_account_enabled(
        &self,
        account_id: Uuid,
        enabled: bool,
    ) -> Result<UpstreamAccount> {
        self.set_upstream_account_enabled_inner(account_id, enabled)
            .await
    }

    async fn delete_upstream_account(&self, account_id: Uuid) -> Result<()> {
        self.delete_upstream_account_inner(account_id).await
    }

    async fn validate_oauth_refresh_token(
        &self,
        req: ValidateOAuthRefreshTokenRequest,
    ) -> Result<ValidateOAuthRefreshTokenResponse> {
        self.validate_oauth_refresh_token_inner(req).await
    }

    async fn import_oauth_refresh_token(
        &self,
        req: ImportOAuthRefreshTokenRequest,
    ) -> Result<UpstreamAccount> {
        self.insert_oauth_account(req).await
    }

    async fn upsert_oauth_refresh_token(
        &self,
        req: ImportOAuthRefreshTokenRequest,
    ) -> Result<OAuthUpsertResult> {
        self.upsert_oauth_account(req).await
    }

    async fn queue_oauth_refresh_token(&self, req: ImportOAuthRefreshTokenRequest) -> Result<bool> {
        self.queue_oauth_refresh_token_vault_inner(req).await
    }

    async fn upsert_one_time_session_account(
        &self,
        req: UpsertOneTimeSessionAccountRequest,
    ) -> Result<OAuthUpsertResult> {
        self.upsert_one_time_session_account_inner(req).await
    }

    async fn refresh_oauth_account(&self, account_id: Uuid) -> Result<OAuthAccountStatusResponse> {
        self.refresh_oauth_account_inner(account_id, true).await
    }

    async fn oauth_account_status(&self, account_id: Uuid) -> Result<OAuthAccountStatusResponse> {
        self.fetch_oauth_account_status(account_id).await
    }

    async fn oauth_account_statuses(
        &self,
        account_ids: Vec<Uuid>,
    ) -> Result<Vec<OAuthAccountStatusResponse>> {
        self.fetch_oauth_account_statuses(&account_ids).await
    }

    async fn upsert_routing_policy(
        &self,
        req: UpsertRoutingPolicyRequest,
    ) -> Result<RoutingPolicy> {
        self.upsert_routing_policy_inner(req).await
    }

    async fn upsert_retry_policy(&self, req: UpsertRetryPolicyRequest) -> Result<RoutingPolicy> {
        self.upsert_retry_policy_inner(req).await
    }

    async fn upsert_stream_retry_policy(
        &self,
        req: UpsertStreamRetryPolicyRequest,
    ) -> Result<RoutingPolicy> {
        self.upsert_stream_retry_policy_inner(req).await
    }

    async fn refresh_expiring_oauth_accounts(&self) -> Result<()> {
        self.refresh_expiring_oauth_accounts_inner().await
    }

    async fn activate_oauth_refresh_token_vault(&self) -> Result<u64> {
        self.activate_oauth_refresh_token_vault_inner().await
    }

    async fn refresh_due_oauth_rate_limit_caches(&self) -> Result<u64> {
        self.refresh_due_oauth_rate_limit_caches_inner().await
    }

    async fn recover_oauth_rate_limit_refresh_jobs(&self) -> Result<u64> {
        self.recover_rate_limit_refresh_jobs_inner().await
    }

    async fn create_oauth_rate_limit_refresh_job(
        &self,
    ) -> Result<OAuthRateLimitRefreshJobSummary> {
        self.create_rate_limit_refresh_job_inner().await
    }

    async fn oauth_rate_limit_refresh_job(
        &self,
        job_id: Uuid,
    ) -> Result<OAuthRateLimitRefreshJobSummary> {
        self.load_oauth_rate_limit_refresh_job_summary_inner(job_id)
            .await
    }

    async fn run_oauth_rate_limit_refresh_job(&self, job_id: Uuid) -> Result<()> {
        self.run_rate_limit_refresh_job_inner(job_id).await
    }

    async fn flush_snapshot_revision(&self, max_batch: usize) -> Result<u32> {
        self.flush_snapshot_revision_batch_inner(max_batch).await
    }

    async fn set_oauth_family_enabled(
        &self,
        account_id: Uuid,
        enabled: bool,
    ) -> Result<OAuthFamilyActionResponse> {
        self.set_oauth_family_enabled_inner(account_id, enabled)
            .await
    }

    async fn snapshot(&self) -> Result<DataPlaneSnapshot> {
        self.snapshot_inner().await
    }

    async fn cleanup_data_plane_outbox(&self, retention: chrono::Duration) -> Result<u64> {
        self.cleanup_data_plane_outbox_inner(retention).await
    }

    async fn data_plane_snapshot_events(
        &self,
        after: u64,
        limit: u32,
    ) -> Result<DataPlaneSnapshotEventsResponse> {
        self.load_data_plane_snapshot_events_inner(after, limit).await
    }

    async fn claim_due_probe_accounts(
        &self,
        limit: usize,
        seen_ok_suppress_sec: i64,
        lock_ttl_sec: i64,
        claimed_by: &str,
    ) -> Result<Vec<ClaimedProbeAccount>> {
        self.claim_due_probe_accounts_inner(limit, seen_ok_suppress_sec, lock_ttl_sec, claimed_by)
            .await
    }

    async fn release_upstream_op_lock(&self, account_id: Uuid, op_type: &str) -> Result<()> {
        self.release_upstream_op_lock_inner(account_id, op_type).await
    }

    async fn record_upstream_probe(&self, account_id: Uuid, write: UpstreamProbeWrite) -> Result<()> {
        self.record_upstream_probe_inner(account_id, write).await
    }

    async fn mark_account_seen_ok(
        &self,
        account_id: Uuid,
        seen_ok_at: DateTime<Utc>,
        min_write_interval_sec: i64,
    ) -> Result<bool> {
        self.mark_account_seen_ok_inner(account_id, seen_ok_at, min_write_interval_sec)
            .await
    }
}
