#[derive(Debug, Clone, Deserialize)]
struct UsageHourlyAccountQuery {
    start_ts: i64,
    end_ts: i64,
    #[serde(default = "default_usage_query_limit")]
    limit: u32,
    account_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
struct UsageHourlyTenantApiKeyQuery {
    start_ts: i64,
    end_ts: i64,
    #[serde(default = "default_usage_query_limit")]
    limit: u32,
    tenant_id: Option<Uuid>,
    api_key_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
struct UsageSummaryQuery {
    start_ts: i64,
    end_ts: i64,
    tenant_id: Option<Uuid>,
    account_id: Option<Uuid>,
    api_key_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
struct UsageHourlyTrendsQuery {
    start_ts: i64,
    end_ts: i64,
    #[serde(default = "default_usage_query_limit")]
    limit: u32,
    account_id: Option<Uuid>,
    tenant_id: Option<Uuid>,
    api_key_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
struct UsageHourlyTenantTrendsQuery {
    start_ts: i64,
    end_ts: i64,
    #[serde(default = "default_usage_query_limit")]
    limit: u32,
    tenant_id: Option<Uuid>,
    api_key_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
struct UsageTenantLeaderboardQuery {
    start_ts: i64,
    end_ts: i64,
    #[serde(default = "default_usage_leaderboard_limit")]
    limit: u32,
    tenant_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
struct UsageAccountLeaderboardQuery {
    start_ts: i64,
    end_ts: i64,
    #[serde(default = "default_usage_leaderboard_limit")]
    limit: u32,
    account_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
struct UsageApiKeyLeaderboardQuery {
    start_ts: i64,
    end_ts: i64,
    #[serde(default = "default_usage_leaderboard_limit")]
    limit: u32,
    tenant_id: Option<Uuid>,
    api_key_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
struct UsageLeaderboardOverviewQuery {
    start_ts: i64,
    end_ts: i64,
    #[serde(default = "default_usage_leaderboard_limit")]
    limit: u32,
    #[serde(default)]
    include_summary: bool,
    tenant_limit: Option<u32>,
    account_limit: Option<u32>,
    api_key_limit: Option<u32>,
    account_id: Option<Uuid>,
    tenant_id: Option<Uuid>,
    api_key_tenant_id: Option<Uuid>,
    api_key_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
struct RequestLogsQuery {
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    limit: Option<u32>,
    tenant_id: Option<Uuid>,
    api_key_id: Option<Uuid>,
    status_code: Option<u16>,
    request_id: Option<String>,
    keyword: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct AuditLogsQuery {
    start_ts: Option<i64>,
    end_ts: Option<i64>,
    limit: Option<u32>,
    tenant_id: Option<Uuid>,
    actor_type: Option<String>,
    actor_id: Option<Uuid>,
    action: Option<String>,
    result_status: Option<String>,
    keyword: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct RequestLogItemResponse {
    id: Uuid,
    account_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    tenant_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_key_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<String>,
    path: String,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    service_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cached_input_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    first_token_latency_ms: Option<u64>,
    status_code: u16,
    latency_ms: u64,
    is_stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    billing_phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    authorization_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    capture_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    estimated_cost_microusd: Option<i64>,
    created_at: DateTime<Utc>,
    event_version: u16,
}

#[derive(Debug, Clone, Serialize)]
struct RequestLogsResponse {
    items: Vec<RequestLogItemResponse>,
}

#[derive(Debug, Clone, Deserialize)]
struct OAuthImportJobItemsQuery {
    status: Option<String>,
    cursor: Option<u64>,
    limit: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct OAuthAccountStatusesRequest {
    account_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
struct OAuthAccountStatusesResponse {
    items: Vec<OAuthAccountStatusResponse>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum UpstreamAccountBatchActionKind {
    Enable,
    Disable,
    Delete,
    RefreshLogin,
    PauseFamily,
    ResumeFamily,
}

#[derive(Debug, Clone, Deserialize)]
struct UpstreamAccountBatchActionRequest {
    action: UpstreamAccountBatchActionKind,
    account_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
struct UpstreamAccountBatchActionResponse {
    action: UpstreamAccountBatchActionKind,
    total: usize,
    success_count: usize,
    failed_count: usize,
    items: Vec<UpstreamAccountBatchActionItem>,
}

#[derive(Debug, Clone, Serialize)]
struct UpstreamAccountBatchActionItem {
    account_id: Uuid,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    job_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<UpstreamAccountBatchActionError>,
}

#[derive(Debug, Clone, Serialize)]
struct UpstreamAccountBatchActionError {
    code: String,
    message: String,
}

#[derive(Debug, Clone, Deserialize)]
struct TenantCreditLedgerQuery {
    limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
struct TenantAcceptedResponse {
    accepted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    debug_code: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct ValidatedUsageHourlyQuery {
    start_ts: i64,
    end_ts: i64,
    limit: u32,
}

fn default_usage_query_limit() -> u32 {
    DEFAULT_USAGE_QUERY_LIMIT
}

fn default_usage_leaderboard_limit() -> u32 {
    DEFAULT_USAGE_LEADERBOARD_LIMIT
}

fn validate_usage_query(
    start_ts: i64,
    end_ts: i64,
    limit: u32,
) -> Result<ValidatedUsageHourlyQuery, (StatusCode, Json<ErrorEnvelope>)> {
    validate_usage_range(start_ts, end_ts)?;

    Ok(ValidatedUsageHourlyQuery {
        start_ts,
        end_ts,
        limit: limit.min(MAX_USAGE_QUERY_LIMIT),
    })
}

fn validate_usage_range(
    start_ts: i64,
    end_ts: i64,
) -> Result<(), (StatusCode, Json<ErrorEnvelope>)> {
    if start_ts > end_ts {
        return Err(invalid_request_error(
            "start_ts must be less than or equal to end_ts",
        ));
    }

    Ok(())
}

async fn health() -> impl IntoResponse {
    Json(json!({ "ok": true }))
}

async fn livez() -> impl IntoResponse {
    Json(json!({ "ok": true }))
}

async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    Json(json!({
        "ok": true,
        "usage_repo_available": state.usage_repo.is_some(),
        "auth_validate_cache_ttl_sec": state.auth_validate_cache_ttl_sec,
    }))
}

async fn system_capabilities(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.system_capabilities.clone())
}

fn localized_message(locale: i18n::Locale, en: &'static str, zh_cn: &'static str) -> &'static str {
    locale.message(en, zh_cn)
}

fn internal_error_with_locale(
    locale: i18n::Locale,
    err: anyhow::Error,
) -> (StatusCode, Json<ErrorEnvelope>) {
    let lowered = err.to_string().to_ascii_lowercase();
    if lowered.contains("account not found") {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorEnvelope::new(
                "not_found",
                localized_message(locale, "resource not found", "资源不存在"),
            )),
        );
    }

    if lowered.contains("refresh_token_reused") {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorEnvelope::new(
                "refresh_token_reused",
                localized_message(
                    locale,
                    "refresh token has been reused; obtain the latest refresh token",
                    "refresh token 已复用，需重新获取最新 refresh token",
                ),
            )),
        );
    }
    if lowered.contains("invalid refresh token") || lowered.contains("invalid_refresh_token") {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorEnvelope::new(
                "invalid_refresh_token",
                localized_message(
                    locale,
                    "refresh token is invalid or expired",
                    "refresh token 无效或已过期",
                ),
            )),
        );
    }
    if lowered.contains("missing_client_id")
        || lowered.contains("oauth token endpoint is not configured")
    {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorEnvelope::new(
                "oauth_provider_not_configured",
                localized_message(
                    locale,
                    "OAuth refresh service is misconfigured (missing client_id or token endpoint)",
                    "OAuth 刷新服务配置不完整（缺少 client_id 或 token endpoint）",
                ),
            )),
        );
    }

    tracing::error!(error = %err, "control-plane store request failed");

    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorEnvelope::new(
            "internal_error",
            localized_message(locale, "internal server error", "服务器内部错误"),
        )),
    )
}

fn internal_error(err: anyhow::Error) -> (StatusCode, Json<ErrorEnvelope>) {
    internal_error_with_locale(i18n::Locale::default_locale(), err)
}

fn unauthorized_error() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorEnvelope::new("unauthorized", "unauthorized")),
    )
}

fn admin_unauthorized_error() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorEnvelope::new("unauthorized", "unauthorized")),
    )
}

fn tenant_unauthorized_error() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorEnvelope::new("unauthorized", "unauthorized")),
    )
}

fn tenant_service_unavailable_error() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorEnvelope::new(
            "service_unavailable",
            "service unavailable",
        )),
    )
}

fn map_tenant_error(err: anyhow::Error) -> (StatusCode, Json<ErrorEnvelope>) {
    let lowered = err.to_string().to_ascii_lowercase();
    if lowered.contains("invalid")
        || lowered.contains("not found")
        || lowered.contains("already")
        || lowered.contains("must")
        || lowered.contains("insufficient")
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorEnvelope::new(
                "invalid_request",
                "invalid tenant request",
            )),
        );
    }
    internal_error(err)
}

fn internal_service_unauthorized_error() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorEnvelope::new("unauthorized", "unauthorized")),
    )
}

const DEFAULT_ADMIN_SESSION_COOKIE_NAME: &str = "cp_admin_session";

fn admin_session_cookie_name() -> String {
    std::env::var("ADMIN_SESSION_COOKIE_NAME")
        .ok()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .unwrap_or_else(|| DEFAULT_ADMIN_SESSION_COOKIE_NAME.to_string())
}

fn admin_session_cookie_secure() -> bool {
    std::env::var("ADMIN_SESSION_COOKIE_SECURE")
        .ok()
        .and_then(|raw| parse_bool_flag(&raw))
        .unwrap_or(false)
}

fn build_admin_session_cookie(token: &str, ttl_sec: u64) -> String {
    let secure = if admin_session_cookie_secure() {
        "; Secure"
    } else {
        ""
    };
    format!(
        "{}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}{}",
        admin_session_cookie_name(),
        token,
        ttl_sec,
        secure
    )
}

fn build_admin_session_clear_cookie() -> String {
    let secure = if admin_session_cookie_secure() {
        "; Secure"
    } else {
        ""
    };
    format!(
        "{}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{}",
        admin_session_cookie_name(),
        secure
    )
}

fn extract_admin_session_cookie_token(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers
        .get(axum::http::header::COOKIE)
        .and_then(|value| value.to_str().ok())?;
    let target_name = admin_session_cookie_name();
    for cookie in cookie_header.split(';') {
        let mut parts = cookie.trim().splitn(2, '=');
        let key = parts.next()?.trim();
        let value = parts.next()?.trim();
        if key == target_name {
            return Some(value.to_string());
        }
    }
    None
}

fn require_admin_principal(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AdminPrincipal, (StatusCode, Json<ErrorEnvelope>)> {
    let authorization = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    if let Ok(principal) = state.admin_auth.verify_bearer_header(authorization) {
        return Ok(principal);
    }

    let Some(token) = extract_admin_session_cookie_token(headers) else {
        return Err(admin_unauthorized_error());
    };
    state
        .admin_auth
        .verify_token(&token)
        .map_err(|_| admin_unauthorized_error())
}

fn require_tenant_auth_service(
    state: &AppState,
) -> Result<Arc<crate::tenant::TenantAuthService>, (StatusCode, Json<ErrorEnvelope>)> {
    state
        .tenant_auth_service
        .clone()
        .ok_or_else(tenant_service_unavailable_error)
}

async fn require_tenant_principal(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<crate::tenant::TenantPrincipal, (StatusCode, Json<ErrorEnvelope>)> {
    let tenant_auth = require_tenant_auth_service(state)?;
    let authorization = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    if let Ok(principal) = tenant_auth.verify_bearer_header(authorization) {
        tenant_auth
            .ensure_principal_active(&principal)
            .await
            .map_err(|_| tenant_unauthorized_error())?;
        return Ok(principal);
    }

    let Some(token) = tenant_auth.extract_cookie_token(headers) else {
        return Err(tenant_unauthorized_error());
    };
    let principal = tenant_auth
        .verify_token(&token)
        .map_err(|_| tenant_unauthorized_error())?;
    tenant_auth
        .ensure_principal_active(&principal)
        .await
        .map_err(|_| tenant_unauthorized_error())?;
    Ok(principal)
}

fn extract_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

async fn write_audit_log_best_effort(state: &AppState, entry: crate::tenant::AuditLogWriteRequest) {
    let Some(tenant_auth) = state.tenant_auth_service.as_ref() else {
        return;
    };
    if let Err(err) = tenant_auth.write_audit_log(entry).await {
        tracing::warn!(error = %err, "failed to persist audit log");
    }
}

fn require_internal_service_token(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(), (StatusCode, Json<ErrorEnvelope>)> {
    let authorization = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(internal_service_unauthorized_error)?;
    let token = authorization
        .strip_prefix("Bearer ")
        .or_else(|| authorization.strip_prefix("bearer "))
        .ok_or_else(internal_service_unauthorized_error)?;
    if token != state.internal_auth_token.as_ref() {
        return Err(internal_service_unauthorized_error());
    }
    Ok(())
}

fn usage_repo_unavailable_error() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorEnvelope::new(
            "service_unavailable",
            "usage repository is unavailable",
        )),
    )
}

fn usage_ingest_repo_unavailable_error() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorEnvelope::new(
            "service_unavailable",
            "usage ingest repository is unavailable",
        )),
    )
}

fn invalid_request_error(message: impl Into<String>) -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorEnvelope::new("invalid_request", message)),
    )
}

fn invalid_multipart_error(
    err: axum::extract::multipart::MultipartError,
) -> (StatusCode, Json<ErrorEnvelope>) {
    let status = err.status();
    let detail = err.body_text();
    tracing::warn!(
        status = %status,
        detail = %detail,
        "invalid multipart payload rejected"
    );
    if status == StatusCode::PAYLOAD_TOO_LARGE {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(ErrorEnvelope::new(
                "payload_too_large",
                "multipart payload is too large",
            )),
        );
    }

    (
        status,
        Json(ErrorEnvelope::new(
            "invalid_multipart",
            "invalid multipart payload",
        )),
    )
}

fn parse_bool_flag(raw: &str) -> Option<bool> {
    if matches!(
        raw.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    ) {
        return Some(true);
    }
    if matches!(
        raw.to_ascii_lowercase().as_str(),
        "0" | "false" | "no" | "off"
    ) {
        return Some(false);
    }
    None
}

fn parse_mode_flag(raw: &str) -> Option<codex_pool_core::model::UpstreamMode> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "chat_gpt_session" | "chat_gpt_oauth" | "chatgpt" | "chatgpt_oauth" => {
            Some(codex_pool_core::model::UpstreamMode::ChatGptSession)
        }
        "codex_oauth" | "codex_session" | "codex" => {
            Some(codex_pool_core::model::UpstreamMode::CodexOauth)
        }
        "open_ai_api_key" | "openai" | "api_key" => {
            Some(codex_pool_core::model::UpstreamMode::OpenAiApiKey)
        }
        _ => None,
    }
}

fn parse_oauth_import_item_status(
    raw: &str,
) -> Result<OAuthImportItemStatus, (StatusCode, Json<ErrorEnvelope>)> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "pending" => Ok(OAuthImportItemStatus::Pending),
        "processing" => Ok(OAuthImportItemStatus::Processing),
        "created" => Ok(OAuthImportItemStatus::Created),
        "updated" => Ok(OAuthImportItemStatus::Updated),
        "failed" => Ok(OAuthImportItemStatus::Failed),
        "skipped" => Ok(OAuthImportItemStatus::Skipped),
        "cancelled" => Ok(OAuthImportItemStatus::Cancelled),
        _ => Err(invalid_request_error(
            "status must be one of: pending, processing, created, updated, failed, skipped, cancelled",
        )),
    }
}
