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

const OAUTH_VAULT_STATUS_QUEUED: &str = "queued";
const OAUTH_VAULT_STATUS_READY: &str = "ready";
const OAUTH_VAULT_STATUS_NEEDS_REFRESH: &str = "needs_refresh";
const OAUTH_VAULT_STATUS_NO_QUOTA: &str = "no_quota";
const OAUTH_VAULT_STATUS_FAILED: &str = "failed";
const OAUTH_INVENTORY_FAILURE_STAGE_ADMISSION_PROBE: &str = "admission_probe";
const OAUTH_INVENTORY_FAILURE_STAGE_ACTIVATION_REFRESH: &str = "activation_refresh";
const OAUTH_INVENTORY_FAILURE_STAGE_ACTIVATION_RATE_LIMITS: &str = "activation_rate_limits";
const OAUTH_INVENTORY_FAILURE_STAGE_RUNTIME_REFRESH: &str = "runtime_refresh";

fn oauth_vault_status_to_db(status: OAuthVaultRecordStatus) -> &'static str {
    match status {
        OAuthVaultRecordStatus::Queued => OAUTH_VAULT_STATUS_QUEUED,
        OAuthVaultRecordStatus::Ready => OAUTH_VAULT_STATUS_READY,
        OAuthVaultRecordStatus::NeedsRefresh => OAUTH_VAULT_STATUS_NEEDS_REFRESH,
        OAuthVaultRecordStatus::NoQuota => OAUTH_VAULT_STATUS_NO_QUOTA,
        OAuthVaultRecordStatus::Failed => OAUTH_VAULT_STATUS_FAILED,
    }
}

fn parse_oauth_vault_record_status(raw: &str) -> Result<OAuthVaultRecordStatus> {
    match raw {
        OAUTH_VAULT_STATUS_QUEUED => Ok(OAuthVaultRecordStatus::Queued),
        OAUTH_VAULT_STATUS_READY => Ok(OAuthVaultRecordStatus::Ready),
        OAUTH_VAULT_STATUS_NEEDS_REFRESH => Ok(OAuthVaultRecordStatus::NeedsRefresh),
        OAUTH_VAULT_STATUS_NO_QUOTA => Ok(OAuthVaultRecordStatus::NoQuota),
        OAUTH_VAULT_STATUS_FAILED => Ok(OAuthVaultRecordStatus::Failed),
        _ => Err(anyhow!(
            "unsupported oauth vault record status in postgres: {raw}"
        )),
    }
}

fn oauth_inventory_failure_stage_to_db(stage: OAuthInventoryFailureStage) -> &'static str {
    match stage {
        OAuthInventoryFailureStage::AdmissionProbe => OAUTH_INVENTORY_FAILURE_STAGE_ADMISSION_PROBE,
        OAuthInventoryFailureStage::ActivationRefresh => {
            OAUTH_INVENTORY_FAILURE_STAGE_ACTIVATION_REFRESH
        }
        OAuthInventoryFailureStage::ActivationRateLimits => {
            OAUTH_INVENTORY_FAILURE_STAGE_ACTIVATION_RATE_LIMITS
        }
        OAuthInventoryFailureStage::RuntimeRefresh => OAUTH_INVENTORY_FAILURE_STAGE_RUNTIME_REFRESH,
    }
}

fn parse_oauth_inventory_failure_stage(raw: &str) -> Result<OAuthInventoryFailureStage> {
    match raw {
        OAUTH_INVENTORY_FAILURE_STAGE_ADMISSION_PROBE => {
            Ok(OAuthInventoryFailureStage::AdmissionProbe)
        }
        OAUTH_INVENTORY_FAILURE_STAGE_ACTIVATION_REFRESH => {
            Ok(OAuthInventoryFailureStage::ActivationRefresh)
        }
        OAUTH_INVENTORY_FAILURE_STAGE_ACTIVATION_RATE_LIMITS => {
            Ok(OAuthInventoryFailureStage::ActivationRateLimits)
        }
        OAUTH_INVENTORY_FAILURE_STAGE_RUNTIME_REFRESH => {
            Ok(OAuthInventoryFailureStage::RuntimeRefresh)
        }
        _ => Err(anyhow!(
            "unsupported oauth inventory failure stage in postgres: {raw}"
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
    is_quota_error_signal(error_code, error_message)
        || is_auth_error_signal(error_code, error_message)
        || matches!(
            normalize_error_code_for_health(error_code).as_str(),
            "primary_window_exhausted" | "secondary_window_exhausted"
        )
}

fn rate_limit_block_message(block_reason: &str) -> String {
    match normalize_error_code_for_health(block_reason).as_str() {
        "secondary_window_exhausted" => {
            "secondary rate limit window is exhausted until reset".to_string()
        }
        "primary_window_exhausted" => {
            "primary rate limit window is exhausted until reset".to_string()
        }
        _ => "rate limit window is exhausted until reset".to_string(),
    }
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
            let reset_at = window.resets_at?;
            (reset_at > now).then_some(reset_at)
        })
        .max()
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

fn derive_admission_rate_limits_expires_at(
    snapshots: &[OAuthRateLimitSnapshot],
    checked_at: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    if snapshots.is_empty() {
        return None;
    }
    let (blocked_until, _) = derive_rate_limit_block(snapshots, checked_at);
    Some(
        blocked_until.unwrap_or_else(|| {
            checked_at + Duration::seconds(rate_limit_cache_ttl_sec_from_env())
        }),
    )
}

fn parse_jwt_exp_from_access_token(access_token: &str) -> Option<DateTime<Utc>> {
    use base64::Engine as _;

    let mut parts = access_token.split('.');
    let _header = parts.next()?;
    let payload = parts.next()?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(payload))
        .ok()?;
    let payload_json = serde_json::from_slice::<serde_json::Value>(&decoded).ok()?;
    let exp = payload_json.get("exp").and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
    })?;
    (exp > 0)
        .then_some(exp)
        .and_then(|seconds| DateTime::<Utc>::from_timestamp(seconds, 0))
}

fn stable_vault_token_family_id(refresh_token_sha256: &str) -> String {
    format!("vault:{refresh_token_sha256}")
}

fn oauth_vault_activation_priority(status: OAuthVaultRecordStatus) -> u8 {
    match status {
        OAuthVaultRecordStatus::Ready => 0,
        OAuthVaultRecordStatus::NeedsRefresh => 1,
        OAuthVaultRecordStatus::Queued => 2,
        OAuthVaultRecordStatus::NoQuota => 3,
        OAuthVaultRecordStatus::Failed => 4,
    }
}

fn oauth_vault_activation_fallback_status(
    status: OAuthVaultRecordStatus,
) -> OAuthVaultRecordStatus {
    match status {
        OAuthVaultRecordStatus::Ready => OAuthVaultRecordStatus::NeedsRefresh,
        other => other,
    }
}

fn should_trigger_refresh_after_rate_limit_failure(error_code: &str, error_message: &str) -> bool {
    is_auth_error_signal(error_code, error_message)
        && !matches!(
            normalize_error_code_for_health(error_code).as_str(),
            "refresh_token_reused" | "refresh_token_revoked"
        )
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
    if !ctx.token_expires_at.is_some_and(|expires_at| {
        expires_at > now + Duration::seconds(OAUTH_MIN_VALID_SEC)
    }) {
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

    // Fresh cache should not suppress `seen_ok`-triggered refresh: a live successful request is a
    // stronger signal than cache age, so in-use accounts get a best-effort wham refresh.
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
    has_usable_access_token_fallback(
        has_access_token_fallback,
        fallback_token_expires_at,
        now,
    ) && refresh_credential_is_terminal_invalid(
        last_refresh_status,
        refresh_reused_detected,
        last_refresh_error_code,
    ) && !token_expires_at.is_some_and(|expires_at| {
        expires_at > now + Duration::seconds(OAUTH_MIN_VALID_SEC)
    })
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
    let fallback_usable = has_usable_access_token_fallback(
        has_access_token_fallback,
        fallback_token_expires_at,
        now,
    );
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

fn parse_routing_policy_row(row: &sqlx_postgres::PgRow) -> Result<RoutingPolicy> {
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

    async fn outbound_proxy_pool_settings(&self) -> Result<OutboundProxyPoolSettings> {
        self.load_outbound_proxy_pool_settings_inner().await
    }

    async fn update_outbound_proxy_pool_settings(
        &self,
        req: UpdateOutboundProxyPoolSettingsRequest,
    ) -> Result<OutboundProxyPoolSettings> {
        self.update_outbound_proxy_pool_settings_inner(req).await
    }

    async fn list_outbound_proxy_nodes(&self) -> Result<Vec<OutboundProxyNode>> {
        self.list_outbound_proxy_nodes_inner().await
    }

    async fn create_outbound_proxy_node(
        &self,
        req: CreateOutboundProxyNodeRequest,
    ) -> Result<OutboundProxyNode> {
        self.create_outbound_proxy_node_inner(req).await
    }

    async fn update_outbound_proxy_node(
        &self,
        node_id: Uuid,
        req: UpdateOutboundProxyNodeRequest,
    ) -> Result<OutboundProxyNode> {
        self.update_outbound_proxy_node_inner(node_id, req).await
    }

    async fn delete_outbound_proxy_node(&self, node_id: Uuid) -> Result<()> {
        self.delete_outbound_proxy_node_inner(node_id).await
    }

    async fn record_outbound_proxy_test_result(
        &self,
        node_id: Uuid,
        last_test_status: Option<String>,
        last_latency_ms: Option<u64>,
        last_error: Option<String>,
        last_tested_at: Option<DateTime<Utc>>,
    ) -> Result<OutboundProxyNode> {
        self.record_outbound_proxy_test_result_inner(
            node_id,
            last_test_status,
            last_latency_ms,
            last_error,
            last_tested_at,
        )
        .await
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

    async fn dedupe_oauth_accounts_by_identity(&self) -> Result<u64> {
        self.dedupe_oauth_accounts_by_identity_inner(None, None, None)
            .await
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

    async fn oauth_inventory_summary(&self) -> Result<OAuthInventorySummaryResponse> {
        self.oauth_inventory_summary_inner().await
    }

    async fn oauth_inventory_records(&self) -> Result<Vec<OAuthInventoryRecord>> {
        self.oauth_inventory_records_inner().await
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

    async fn list_routing_profiles(&self) -> Result<Vec<RoutingProfile>> {
        self.list_routing_profiles_inner().await
    }

    async fn upsert_routing_profile(
        &self,
        req: UpsertRoutingProfileRequest,
    ) -> Result<RoutingProfile> {
        self.upsert_routing_profile_inner(req).await
    }

    async fn delete_routing_profile(&self, profile_id: Uuid) -> Result<()> {
        self.delete_routing_profile_inner(profile_id).await
    }

    async fn list_model_routing_policies(&self) -> Result<Vec<ModelRoutingPolicy>> {
        self.list_model_routing_policies_inner().await
    }

    async fn upsert_model_routing_policy(
        &self,
        req: UpsertModelRoutingPolicyRequest,
    ) -> Result<ModelRoutingPolicy> {
        self.upsert_model_routing_policy_inner(req).await
    }

    async fn delete_model_routing_policy(&self, policy_id: Uuid) -> Result<()> {
        self.delete_model_routing_policy_inner(policy_id).await
    }

    async fn model_routing_settings(&self) -> Result<ModelRoutingSettings> {
        self.load_model_routing_settings_inner().await
    }

    async fn update_model_routing_settings(
        &self,
        req: UpdateModelRoutingSettingsRequest,
    ) -> Result<ModelRoutingSettings> {
        self.update_model_routing_settings_inner(req).await
    }

    async fn upstream_error_learning_settings(&self) -> Result<AiErrorLearningSettings> {
        self.load_upstream_error_learning_settings_inner().await
    }

    async fn update_upstream_error_learning_settings(
        &self,
        req: UpdateAiErrorLearningSettingsRequest,
    ) -> Result<AiErrorLearningSettings> {
        self.update_upstream_error_learning_settings_inner(req).await
    }

    async fn list_upstream_error_templates(
        &self,
        status: Option<UpstreamErrorTemplateStatus>,
    ) -> Result<Vec<UpstreamErrorTemplateRecord>> {
        self.list_upstream_error_templates_inner(status).await
    }

    async fn upstream_error_template_by_id(
        &self,
        template_id: Uuid,
    ) -> Result<Option<UpstreamErrorTemplateRecord>> {
        self.load_upstream_error_template_by_id_inner(template_id).await
    }

    async fn upstream_error_template_by_fingerprint(
        &self,
        fingerprint: &str,
    ) -> Result<Option<UpstreamErrorTemplateRecord>> {
        self.load_upstream_error_template_by_fingerprint_inner(fingerprint)
            .await
    }

    async fn save_upstream_error_template(
        &self,
        template: UpstreamErrorTemplateRecord,
    ) -> Result<UpstreamErrorTemplateRecord> {
        self.save_upstream_error_template_inner(template).await
    }

    async fn list_builtin_error_template_overrides(
        &self,
    ) -> Result<Vec<BuiltinErrorTemplateOverrideRecord>> {
        self.list_builtin_error_template_overrides_inner().await
    }

    async fn save_builtin_error_template_override(
        &self,
        record: BuiltinErrorTemplateOverrideRecord,
    ) -> Result<BuiltinErrorTemplateOverrideRecord> {
        self.save_builtin_error_template_override_inner(record).await
    }

    async fn delete_builtin_error_template_override(
        &self,
        kind: BuiltinErrorTemplateKind,
        code: &str,
    ) -> Result<()> {
        self.delete_builtin_error_template_override_inner(kind, code)
            .await
    }

    async fn list_routing_plan_versions(&self) -> Result<Vec<RoutingPlanVersion>> {
        self.list_routing_plan_versions_inner().await
    }

    async fn record_account_model_support(
        &self,
        account_id: Uuid,
        supported_models: Vec<String>,
        checked_at: DateTime<Utc>,
    ) -> Result<()> {
        self.record_account_model_support_inner(account_id, supported_models, checked_at)
            .await
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

    async fn mark_account_seen_ok(
        &self,
        account_id: Uuid,
        seen_ok_at: DateTime<Utc>,
        min_write_interval_sec: i64,
    ) -> Result<bool> {
        self.mark_account_seen_ok_inner(account_id, seen_ok_at, min_write_interval_sec)
            .await
    }

    async fn maybe_refresh_oauth_rate_limit_cache_on_seen_ok(
        &self,
        account_id: Uuid,
    ) -> Result<()> {
        self.maybe_refresh_oauth_rate_limit_cache_on_seen_ok_inner(account_id)
            .await
    }

    async fn update_oauth_rate_limit_cache_from_observation(
        &self,
        account_id: Uuid,
        rate_limits: Vec<OAuthRateLimitSnapshot>,
        observed_at: DateTime<Utc>,
    ) -> Result<()> {
        self.persist_rate_limit_cache_success(account_id, rate_limits, observed_at)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::{SeenOkRateLimitRefreshContext, should_refresh_rate_limit_cache_on_seen_ok};
    use chrono::{Duration, Utc};
    use crate::contracts::OAuthRefreshStatus;

    #[test]
    fn seen_ok_refresh_policy_does_not_skip_fresh_cache() {
        let now = Utc::now();
        let should_refresh = should_refresh_rate_limit_cache_on_seen_ok(
            now,
            SeenOkRateLimitRefreshContext {
                token_expires_at: Some(now + Duration::minutes(30)),
                last_refresh_status: &OAuthRefreshStatus::Ok,
                refresh_reused_detected: false,
                last_refresh_error_code: None,
                rate_limits_expires_at: Some(now + Duration::minutes(3)),
                rate_limits_last_error_code: None,
                rate_limits_last_error: None,
            },
        );
        assert!(should_refresh);
    }

    #[test]
    fn seen_ok_refresh_policy_still_blocks_fatal_refresh_failures() {
        let now = Utc::now();
        let should_refresh = should_refresh_rate_limit_cache_on_seen_ok(
            now,
            SeenOkRateLimitRefreshContext {
                token_expires_at: Some(now + Duration::minutes(30)),
                last_refresh_status: &OAuthRefreshStatus::Failed,
                refresh_reused_detected: false,
                last_refresh_error_code: Some("refresh_token_revoked"),
                rate_limits_expires_at: Some(now + Duration::minutes(3)),
                rate_limits_last_error_code: None,
                rate_limits_last_error: None,
            },
        );
        assert!(!should_refresh);
    }
}
