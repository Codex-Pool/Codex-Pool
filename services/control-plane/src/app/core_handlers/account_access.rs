const DEFAULT_CODEX_IMPORT_BASE_URL: &str = "https://chatgpt.com/backend-api/codex";
const DEFAULT_CODEX_IMPORT_PRIORITY: i32 = 100;
const DEFAULT_CODEX_IMPORT_ENABLED: bool = true;
const DEFAULT_CODEX_OAUTH_SCOPE: &str = "openid email profile offline_access";
const DEFAULT_CODEX_OAUTH_REDIRECT_URL: &str = "http://localhost:1455/auth/callback";

#[derive(Debug, Clone, Deserialize)]
struct CreateCodexOAuthLoginSessionRequest {
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    priority: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
struct CreateCodexOAuthProbeSessionRequest {
    #[serde(default)]
    base_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct CodexOAuthCallbackQuery {
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    error_description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct SubmitCodexOAuthCallbackRequest {
    redirect_url: String,
}

#[derive(Debug, Clone, Serialize)]
struct CodexOAuthLoginSessionResponse {
    session_id: String,
    status: CodexOAuthLoginSessionStatus,
    authorize_url: String,
    callback_url: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<CodexOAuthLoginSessionError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<CodexOAuthLoginSessionResult>,
}

#[derive(Debug, Clone, Serialize)]
struct CodexOAuthProbeSessionResponse {
    session_id: String,
    status: CodexOAuthLoginSessionStatus,
    authorize_url: String,
    callback_url: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<CodexOAuthLoginSessionError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<CodexOAuthProbeSessionResult>,
}

fn codex_oauth_provider_not_configured_error() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorEnvelope::new(
            "oauth_provider_not_configured",
            "oauth provider is not configured",
        )),
    )
}

fn codex_oauth_callback_listener_unavailable_error() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorEnvelope::new(
            "oauth_callback_listener_unavailable",
            "oauth callback listener is unavailable",
        )),
    )
}

fn codex_oauth_not_found_error() -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorEnvelope::new("not_found", "resource not found")),
    )
}

fn codex_oauth_invalid_request_error(
    message: impl Into<String>,
) -> (StatusCode, Json<ErrorEnvelope>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorEnvelope::new("invalid_request", message)),
    )
}

fn codex_oauth_callback_html(success: bool, message: &str) -> String {
    let title = if success {
        "Codex OAuth Login Completed"
    } else {
        "Codex OAuth Login Failed"
    };
    let safe_message = message
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{title}</title></head>\
         <body style=\"font-family:system-ui,-apple-system,Segoe UI,Roboto,sans-serif;padding:24px;line-height:1.5;\">\
         <h2 style=\"margin:0 0 8px;\">{title}</h2><p style=\"margin:0;color:#334155;\">{safe_message}</p>\
         <script>setTimeout(function(){{if(window.opener){{window.close();}}}},600);</script>\
         </body></html>"
    )
}

fn normalize_codex_import_base_url(base_url: Option<String>) -> String {
    let mut normalized = base_url
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_CODEX_IMPORT_BASE_URL.to_string());
    while normalized.ends_with('/') {
        normalized.pop();
    }
    normalized
}

fn resolve_codex_oauth_redirect_url_with_default(
    env_key: &str,
    default_url: &str,
) -> Option<String> {
    let configured = std::env::var(env_key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default_url.to_string());
    let mut parsed = reqwest::Url::parse(&configured).ok()?;
    parsed.set_query(None);
    parsed.set_fragment(None);
    Some(parsed.to_string())
}

fn resolve_codex_oauth_redirect_url() -> Option<String> {
    resolve_codex_oauth_redirect_url_with_default(
        "CODEX_OAUTH_REDIRECT_URL",
        DEFAULT_CODEX_OAUTH_REDIRECT_URL,
    )
}

fn resolve_codex_oauth_probe_redirect_url() -> Option<String> {
    resolve_codex_oauth_redirect_url_with_default(
        "CODEX_OAUTH_PROBE_REDIRECT_URL",
        DEFAULT_CODEX_OAUTH_REDIRECT_URL,
    )
}

fn random_urlsafe_token() -> String {
    let bytes: [u8; 32] = rand::random();
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
}

fn pkce_code_challenge(code_verifier: &str) -> String {
    let digest = <sha2::Sha256 as sha2::Digest>::digest(code_verifier.as_bytes());
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, digest)
}

fn codex_session_response(record: &CodexOAuthLoginSessionRecord) -> CodexOAuthLoginSessionResponse {
    CodexOAuthLoginSessionResponse {
        session_id: record.session_id.clone(),
        status: record.status,
        authorize_url: record.authorize_url.clone(),
        callback_url: record.callback_url.clone(),
        created_at: record.created_at,
        updated_at: record.updated_at,
        expires_at: record.expires_at,
        error: record.error.clone(),
        result: record.result.clone(),
    }
}

fn codex_probe_session_response(
    record: &CodexOAuthProbeSessionRecord,
) -> CodexOAuthProbeSessionResponse {
    CodexOAuthProbeSessionResponse {
        session_id: record.session_id.clone(),
        status: record.status,
        authorize_url: record.authorize_url.clone(),
        callback_url: record.callback_url.clone(),
        created_at: record.created_at,
        updated_at: record.updated_at,
        expires_at: record.expires_at,
        error: record.error.clone(),
        result: record.result.clone(),
    }
}

fn codex_error_message_from_code(code: &str) -> &'static str {
    match code {
        "oauth_provider_not_configured" => "oauth provider is not configured",
        "oauth_access_denied" => "oauth access denied by user",
        "oauth_exchange_failed" => "oauth code exchange failed",
        "rate_limited" => "oauth provider rate limited the request",
        "upstream_network_error" => "network error while contacting oauth provider",
        "upstream_unavailable" => "oauth provider is temporarily unavailable",
        "invalid_refresh_token" => "refresh token is invalid or expired",
        "refresh_token_reused" => "refresh token has been reused",
        "import_failed" => "failed to import oauth account",
        _ => "request failed",
    }
}

fn codex_exchange_error_to_session_error(
    err: &crate::oauth::OAuthTokenClientError,
) -> CodexOAuthLoginSessionError {
    match err {
        crate::oauth::OAuthTokenClientError::NotConfigured => CodexOAuthLoginSessionError {
            code: "oauth_provider_not_configured".to_string(),
            message: codex_error_message_from_code("oauth_provider_not_configured").to_string(),
        },
        crate::oauth::OAuthTokenClientError::Parse => CodexOAuthLoginSessionError {
            code: "oauth_exchange_failed".to_string(),
            message: codex_error_message_from_code("oauth_exchange_failed").to_string(),
        },
        crate::oauth::OAuthTokenClientError::InvalidRefreshToken { code, .. } => {
            let mapped = match code {
                crate::oauth::OAuthRefreshErrorCode::RefreshTokenReused => "refresh_token_reused",
                _ => "invalid_refresh_token",
            };
            CodexOAuthLoginSessionError {
                code: mapped.to_string(),
                message: codex_error_message_from_code(mapped).to_string(),
            }
        }
        crate::oauth::OAuthTokenClientError::Upstream { code, message } => {
            let mapped = match code {
                crate::oauth::OAuthRefreshErrorCode::MissingClientId
                | crate::oauth::OAuthRefreshErrorCode::UnauthorizedClient => {
                    "oauth_provider_not_configured"
                }
                crate::oauth::OAuthRefreshErrorCode::RateLimited => "rate_limited",
                crate::oauth::OAuthRefreshErrorCode::UpstreamUnavailable => {
                    if message.contains("status=") {
                        "upstream_unavailable"
                    } else {
                        "upstream_network_error"
                    }
                }
                crate::oauth::OAuthRefreshErrorCode::RefreshTokenReused => "refresh_token_reused",
                crate::oauth::OAuthRefreshErrorCode::InvalidRefreshToken
                | crate::oauth::OAuthRefreshErrorCode::RefreshTokenRevoked => {
                    "invalid_refresh_token"
                }
                crate::oauth::OAuthRefreshErrorCode::Unknown => "oauth_exchange_failed",
            };
            CodexOAuthLoginSessionError {
                code: mapped.to_string(),
                message: codex_error_message_from_code(mapped).to_string(),
            }
        }
    }
}

fn codex_import_error_to_session_error(err: &anyhow::Error) -> CodexOAuthLoginSessionError {
    let lowered = err.to_string().to_ascii_lowercase();
    let mapped = if lowered.contains("refresh_token_reused") {
        "refresh_token_reused"
    } else if lowered.contains("invalid_refresh_token")
        || lowered.contains("invalid refresh token")
        || lowered.contains("refresh token is invalid")
    {
        "invalid_refresh_token"
    } else if lowered.contains("connection")
        || lowered.contains("dns")
        || lowered.contains("network")
    {
        "upstream_network_error"
    } else if lowered.contains("rate limit") {
        "rate_limited"
    } else if lowered.contains("upstream unavailable") {
        "upstream_unavailable"
    } else if lowered.contains("oauth token endpoint is not configured")
        || lowered.contains("missing_client_id")
    {
        "oauth_provider_not_configured"
    } else {
        "import_failed"
    };
    CodexOAuthLoginSessionError {
        code: mapped.to_string(),
        message: codex_error_message_from_code(mapped).to_string(),
    }
}

fn codex_find_session_id_by_state(
    sessions: &std::collections::HashMap<String, CodexOAuthLoginSessionRecord>,
    state: &str,
) -> Option<String> {
    sessions
        .values()
        .find(|session| session.state == state)
        .map(|session| session.session_id.clone())
}

fn codex_find_probe_session_id_by_state(
    sessions: &std::collections::HashMap<String, CodexOAuthProbeSessionRecord>,
    state: &str,
) -> Option<String> {
    sessions
        .values()
        .find(|session| session.state == state)
        .map(|session| session.session_id.clone())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodexOAuthCallbackTarget {
    Login,
    Probe,
}

fn codex_find_callback_target_by_state(
    state: &AppState,
    callback_state: &str,
) -> Option<CodexOAuthCallbackTarget> {
    {
        let sessions = state
            .oauth_login_sessions
            .read()
            .expect("oauth login session lock poisoned");
        if codex_find_session_id_by_state(&sessions, callback_state).is_some() {
            return Some(CodexOAuthCallbackTarget::Login);
        }
    }

    {
        let sessions = state
            .oauth_probe_sessions
            .read()
            .expect("oauth probe session lock poisoned");
        if codex_find_probe_session_id_by_state(&sessions, callback_state).is_some() {
            return Some(CodexOAuthCallbackTarget::Probe);
        }
    }

    None
}

fn codex_callback_state(query: &CodexOAuthCallbackQuery) -> Option<&str> {
    query.state
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn codex_oauth_callback_success_response(message: String) -> axum::response::Response {
    (
        StatusCode::OK,
        axum::response::Html(codex_oauth_callback_html(true, &message)),
    )
        .into_response()
}

fn codex_oauth_callback_failure_response(envelope: ErrorEnvelope) -> axum::response::Response {
    (
        StatusCode::OK,
        axum::response::Html(codex_oauth_callback_html(
            false,
            &format!("{} ({})", envelope.error.message, envelope.error.code),
        )),
    )
        .into_response()
}

fn codex_mark_expired_if_needed(
    session: &mut CodexOAuthLoginSessionRecord,
    now: DateTime<Utc>,
) {
    if now < session.expires_at {
        return;
    }
    if matches!(
        session.status,
        CodexOAuthLoginSessionStatus::WaitingCallback
            | CodexOAuthLoginSessionStatus::Exchanging
            | CodexOAuthLoginSessionStatus::Importing
    ) {
        session.status = CodexOAuthLoginSessionStatus::Expired;
        session.error = Some(CodexOAuthLoginSessionError {
            code: "invalid_request".to_string(),
            message: "login session expired".to_string(),
        });
        session.updated_at = now;
    }
}

fn cleanup_codex_oauth_login_sessions(
    sessions: &mut std::collections::HashMap<String, CodexOAuthLoginSessionRecord>,
) {
    let now = Utc::now();
    let retention_cutoff = now - chrono::Duration::seconds(OAUTH_LOGIN_SESSION_RETENTION_SEC);
    for session in sessions.values_mut() {
        codex_mark_expired_if_needed(session, now);
    }
    sessions.retain(|_, session| session.updated_at >= retention_cutoff);
}

fn codex_mark_probe_expired_if_needed(
    session: &mut CodexOAuthProbeSessionRecord,
    now: DateTime<Utc>,
) {
    if now < session.expires_at {
        return;
    }
    if matches!(
        session.status,
        CodexOAuthLoginSessionStatus::WaitingCallback
            | CodexOAuthLoginSessionStatus::Exchanging
            | CodexOAuthLoginSessionStatus::Importing
    ) {
        session.status = CodexOAuthLoginSessionStatus::Expired;
        session.error = Some(CodexOAuthLoginSessionError {
            code: "invalid_request".to_string(),
            message: "login session expired".to_string(),
        });
        session.updated_at = now;
    }
}

fn cleanup_codex_oauth_probe_sessions(
    sessions: &mut std::collections::HashMap<String, CodexOAuthProbeSessionRecord>,
) {
    let now = Utc::now();
    let retention_cutoff = now - chrono::Duration::seconds(OAUTH_LOGIN_SESSION_RETENTION_SEC);
    for session in sessions.values_mut() {
        codex_mark_probe_expired_if_needed(session, now);
    }
    sessions.retain(|_, session| session.updated_at >= retention_cutoff);
}

fn preview_oauth_secret(raw: Option<&str>) -> Option<String> {
    let value = raw.map(str::trim).filter(|value| !value.is_empty())?;
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= 12 {
        return Some("••••".to_string());
    }
    let prefix = chars.iter().take(6).collect::<String>();
    let suffix = chars
        .iter()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    Some(format!("{prefix}...{suffix}"))
}

fn parse_callback_query_from_redirect_url(
    redirect_url: &str,
) -> Result<CodexOAuthCallbackQuery, (StatusCode, Json<ErrorEnvelope>)> {
    let trimmed = redirect_url.trim();
    if trimmed.is_empty() {
        return Err(codex_oauth_invalid_request_error(
            "redirect_url must not be empty",
        ));
    }
    let parsed = reqwest::Url::parse(trimmed).map_err(|_| {
        codex_oauth_invalid_request_error("redirect_url must be a valid absolute URL")
    })?;
    let mut pairs = parsed.query_pairs().into_owned().collect::<std::collections::HashMap<_, _>>();
    Ok(CodexOAuthCallbackQuery {
        code: pairs.remove("code"),
        state: pairs.remove("state"),
        error: pairs.remove("error"),
        error_description: pairs.remove("error_description"),
    })
}

fn derive_codex_import_label(
    label: Option<&str>,
    email: Option<&str>,
    account_id: Option<&str>,
) -> String {
    if let Some(label) = label.map(str::trim).filter(|value| !value.is_empty()) {
        return label.to_string();
    }
    if let Some(email) = email.map(str::trim).filter(|value| !value.is_empty()) {
        return format!("codex-{email}");
    }
    if let Some(account_id) = account_id.map(str::trim).filter(|value| !value.is_empty()) {
        return format!("codex-{account_id}");
    }
    format!("codex-oauth-{}", Uuid::new_v4().simple())
}

async fn process_codex_oauth_callback_flow(
    state: &AppState,
    session_id_hint: Option<String>,
    callback: CodexOAuthCallbackQuery,
) -> Result<CodexOAuthLoginSessionResponse, (StatusCode, Json<ErrorEnvelope>)> {
    let now = Utc::now();
    let callback_state = callback
        .state
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    let mut early_response = None;
    let prepared_session = {
        let mut sessions = state
            .oauth_login_sessions
            .write()
            .expect("oauth login session lock poisoned");
        cleanup_codex_oauth_login_sessions(&mut sessions);
        let session_id = match session_id_hint {
            Some(session_id) => session_id,
            None => {
                let state_value = callback_state
                    .as_deref()
                    .ok_or_else(|| codex_oauth_invalid_request_error("missing callback state"))?;
                codex_find_session_id_by_state(&sessions, state_value)
                    .ok_or_else(codex_oauth_not_found_error)?
            }
        };
        let session = sessions
            .get_mut(&session_id)
            .ok_or_else(codex_oauth_not_found_error)?;
        codex_mark_expired_if_needed(session, now);
        if session.status == CodexOAuthLoginSessionStatus::Expired {
            return Err(codex_oauth_invalid_request_error("login session expired"));
        }

        if let Some(state_value) = callback_state.as_deref() {
            if state_value != session.state {
                return Err(codex_oauth_invalid_request_error("callback state mismatch"));
            }
        }

        if let Some(error) = callback
            .error
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let denied_by_description = callback
                .error_description
                .as_deref()
                .map(str::to_ascii_lowercase)
                .is_some_and(|value| value.contains("access denied"));
            let mapped_code = if error.eq_ignore_ascii_case("access_denied") || denied_by_description
            {
                "oauth_access_denied"
            } else {
                "oauth_exchange_failed"
            };
            session.status = CodexOAuthLoginSessionStatus::Failed;
            session.error = Some(CodexOAuthLoginSessionError {
                code: mapped_code.to_string(),
                message: codex_error_message_from_code(mapped_code).to_string(),
            });
            session.result = None;
            session.updated_at = now;
            early_response = Some(codex_session_response(session));
            None
        } else {
            let code = callback
                .code
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| codex_oauth_invalid_request_error("missing oauth code"))?
                .to_string();

            session.status = CodexOAuthLoginSessionStatus::Exchanging;
            session.error = None;
            session.updated_at = now;
            Some((
                session_id,
                code,
                session.callback_url.clone(),
                session.code_verifier.clone(),
                session.base_url.clone(),
                session.label.clone(),
                session.enabled,
                session.priority,
            ))
        }
    };
    if let Some(response) = early_response {
        stop_codex_oauth_callback_listener_if_idle(state).await;
        return Ok(response);
    }
    let (session_id, code, callback_url, code_verifier, base_url, label, enabled, priority) =
        prepared_session.ok_or_else(codex_oauth_not_found_error)?;

    let oauth_client = crate::oauth::OpenAiOAuthClient::from_parts_with_outbound_proxy_runtime(
        std::env::var("OPENAI_OAUTH_TOKEN_URL")
            .unwrap_or_else(|_| "https://auth.openai.com/oauth/token".to_string()),
        std::env::var("OPENAI_OAUTH_AUTHORIZE_URL")
            .unwrap_or_else(|_| "https://auth.openai.com/oauth/authorize".to_string()),
        std::env::var("OPENAI_OAUTH_CLIENT_ID").ok(),
        std::env::var("OPENAI_OAUTH_TIMEOUT_SEC")
            .ok()
            .and_then(|value| value.parse::<u64>().ok()),
        Some(state.outbound_proxy_runtime.clone()),
    );
    let exchange = match oauth_client
        .exchange_authorization_code(&code, &callback_url, &code_verifier)
        .await
    {
        Ok(exchange) => exchange,
        Err(err) => {
            tracing::warn!(error = %err, "codex oauth code exchange failed");
            let response = {
                let mut sessions = state
                    .oauth_login_sessions
                    .write()
                    .expect("oauth login session lock poisoned");
                cleanup_codex_oauth_login_sessions(&mut sessions);
                let session = sessions
                    .get_mut(&session_id)
                    .ok_or_else(codex_oauth_not_found_error)?;
                session.status = CodexOAuthLoginSessionStatus::Failed;
                session.error = Some(codex_exchange_error_to_session_error(&err));
                session.result = None;
                session.updated_at = Utc::now();
                codex_session_response(session)
            };
            stop_codex_oauth_callback_listener_if_idle(state).await;
            return Ok(response);
        }
    };

    let email = exchange
        .id_token_claims
        .as_ref()
        .and_then(|claims| claims.email.clone());
    let plan_type = exchange
        .id_token_claims
        .as_ref()
        .and_then(|claims| claims.chatgpt_plan_type.clone());
    let chatgpt_account_id = exchange
        .chatgpt_account_id
        .clone()
        .or_else(|| {
            exchange
                .id_token_claims
                .as_ref()
                .and_then(|claims| claims.chatgpt_account_id.clone())
        });
    let refresh_token = exchange
        .refresh_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| codex_oauth_invalid_request_error("oauth callback did not include refresh_token"))?;
    let label_value = derive_codex_import_label(
        label.as_deref(),
        email.as_deref(),
        chatgpt_account_id.as_deref(),
    );

    {
        let mut sessions = state
            .oauth_login_sessions
            .write()
            .expect("oauth login session lock poisoned");
        cleanup_codex_oauth_login_sessions(&mut sessions);
        if let Some(session) = sessions.get_mut(&session_id) {
            session.status = CodexOAuthLoginSessionStatus::Importing;
            session.updated_at = Utc::now();
        }
    }

    let upsert_request = ImportOAuthRefreshTokenRequest {
        label: label_value,
        base_url: base_url.clone(),
        refresh_token,
        chatgpt_account_id: chatgpt_account_id.clone(),
        mode: Some(codex_pool_core::model::UpstreamMode::CodexOauth),
        enabled: Some(enabled),
        priority: Some(priority),
        chatgpt_plan_type: plan_type.clone(),
        source_type: Some("codex".to_string()),
    };

    let upsert_result = match state.store.upsert_oauth_refresh_token(upsert_request).await {
        Ok(result) => result,
        Err(err) => {
            tracing::warn!(error = %err, "codex oauth import failed");
            let response = {
                let mut sessions = state
                    .oauth_login_sessions
                    .write()
                    .expect("oauth login session lock poisoned");
                cleanup_codex_oauth_login_sessions(&mut sessions);
                let session = sessions
                    .get_mut(&session_id)
                    .ok_or_else(codex_oauth_not_found_error)?;
                session.status = CodexOAuthLoginSessionStatus::Failed;
                session.error = Some(codex_import_error_to_session_error(&err));
                session.result = None;
                session.updated_at = Utc::now();
                codex_session_response(session)
            };
            stop_codex_oauth_callback_listener_if_idle(state).await;
            return Ok(response);
        }
    };

    let response = {
        let mut sessions = state
            .oauth_login_sessions
            .write()
            .expect("oauth login session lock poisoned");
        cleanup_codex_oauth_login_sessions(&mut sessions);
        let session = sessions
            .get_mut(&session_id)
            .ok_or_else(codex_oauth_not_found_error)?;
        session.status = CodexOAuthLoginSessionStatus::Completed;
        session.error = None;
        session.updated_at = Utc::now();
        session.result = Some(CodexOAuthLoginSessionResult {
            created: upsert_result.created,
            account: upsert_result.account,
            email,
            chatgpt_account_id,
            chatgpt_plan_type: plan_type,
        });
        codex_session_response(session)
    };
    stop_codex_oauth_callback_listener_if_idle(state).await;
    Ok(response)
}

async fn process_codex_oauth_probe_callback_flow(
    state: &AppState,
    session_id_hint: Option<String>,
    callback: CodexOAuthCallbackQuery,
) -> Result<CodexOAuthProbeSessionResponse, (StatusCode, Json<ErrorEnvelope>)> {
    let now = Utc::now();
    let callback_state = callback
        .state
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    let mut early_response = None;
    let prepared_session = {
        let mut sessions = state
            .oauth_probe_sessions
            .write()
            .expect("oauth probe session lock poisoned");
        cleanup_codex_oauth_probe_sessions(&mut sessions);
        let session_id = match session_id_hint {
            Some(session_id) => session_id,
            None => {
                let state_value = callback_state
                    .as_deref()
                    .ok_or_else(|| codex_oauth_invalid_request_error("missing callback state"))?;
                codex_find_probe_session_id_by_state(&sessions, state_value)
                    .ok_or_else(codex_oauth_not_found_error)?
            }
        };
        let session = sessions
            .get_mut(&session_id)
            .ok_or_else(codex_oauth_not_found_error)?;
        codex_mark_probe_expired_if_needed(session, now);
        if session.status == CodexOAuthLoginSessionStatus::Expired {
            return Err(codex_oauth_invalid_request_error("login session expired"));
        }

        if let Some(state_value) = callback_state.as_deref() {
            if state_value != session.state {
                return Err(codex_oauth_invalid_request_error("callback state mismatch"));
            }
        }

        if let Some(error) = callback
            .error
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let denied_by_description = callback
                .error_description
                .as_deref()
                .map(str::to_ascii_lowercase)
                .is_some_and(|value| value.contains("access denied"));
            let mapped_code = if error.eq_ignore_ascii_case("access_denied") || denied_by_description
            {
                "oauth_access_denied"
            } else {
                "oauth_exchange_failed"
            };
            session.status = CodexOAuthLoginSessionStatus::Failed;
            session.error = Some(CodexOAuthLoginSessionError {
                code: mapped_code.to_string(),
                message: codex_error_message_from_code(mapped_code).to_string(),
            });
            session.result = None;
            session.updated_at = now;
            early_response = Some(codex_probe_session_response(session));
            None
        } else {
            let code = callback
                .code
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| codex_oauth_invalid_request_error("missing oauth code"))?
                .to_string();

            session.status = CodexOAuthLoginSessionStatus::Exchanging;
            session.error = None;
            session.updated_at = now;
            Some((
                session_id,
                code,
                session.callback_url.clone(),
                session.code_verifier.clone(),
                session.base_url.clone(),
            ))
        }
    };
    if let Some(response) = early_response {
        stop_codex_oauth_callback_listener_if_idle(state).await;
        return Ok(response);
    }
    let (session_id, code, callback_url, code_verifier, base_url) =
        prepared_session.ok_or_else(codex_oauth_not_found_error)?;

    let oauth_client = crate::oauth::OpenAiOAuthClient::from_parts_with_outbound_proxy_runtime(
        std::env::var("OPENAI_OAUTH_TOKEN_URL")
            .unwrap_or_else(|_| "https://auth.openai.com/oauth/token".to_string()),
        std::env::var("OPENAI_OAUTH_AUTHORIZE_URL")
            .unwrap_or_else(|_| "https://auth.openai.com/oauth/authorize".to_string()),
        std::env::var("OPENAI_OAUTH_CLIENT_ID").ok(),
        std::env::var("OPENAI_OAUTH_TIMEOUT_SEC")
            .ok()
            .and_then(|value| value.parse::<u64>().ok()),
        Some(state.outbound_proxy_runtime.clone()),
    );
    let exchange = match oauth_client
        .exchange_authorization_code(&code, &callback_url, &code_verifier)
        .await
    {
        Ok(exchange) => exchange,
        Err(err) => {
            tracing::warn!(error = %err, "codex oauth probe code exchange failed");
            let response = {
                let mut sessions = state
                    .oauth_probe_sessions
                    .write()
                    .expect("oauth probe session lock poisoned");
                cleanup_codex_oauth_probe_sessions(&mut sessions);
                let session = sessions
                    .get_mut(&session_id)
                    .ok_or_else(codex_oauth_not_found_error)?;
                session.status = CodexOAuthLoginSessionStatus::Failed;
                session.error = Some(codex_exchange_error_to_session_error(&err));
                session.result = None;
                session.updated_at = Utc::now();
                codex_probe_session_response(session)
            };
            stop_codex_oauth_callback_listener_if_idle(state).await;
            return Ok(response);
        }
    };

    let email = exchange
        .id_token_claims
        .as_ref()
        .and_then(|claims| claims.email.clone());
    let plan_type = exchange
        .id_token_claims
        .as_ref()
        .and_then(|claims| claims.chatgpt_plan_type.clone());
    let chatgpt_account_id = exchange
        .chatgpt_account_id
        .clone()
        .or_else(|| {
            exchange
                .id_token_claims
                .as_ref()
                .and_then(|claims| claims.chatgpt_account_id.clone())
        });

    let response = {
        let mut sessions = state
            .oauth_probe_sessions
            .write()
            .expect("oauth probe session lock poisoned");
        cleanup_codex_oauth_probe_sessions(&mut sessions);
        let session = sessions
            .get_mut(&session_id)
            .ok_or_else(codex_oauth_not_found_error)?;
        session.status = CodexOAuthLoginSessionStatus::Completed;
        session.error = None;
        session.updated_at = Utc::now();
        session.result = Some(CodexOAuthProbeSessionResult {
            base_url,
            access_token_present: !exchange.access_token.trim().is_empty(),
            refresh_token_present: exchange
                .refresh_token
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty()),
            id_token_present: exchange
                .id_token_raw
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty()),
            access_token_preview: preview_oauth_secret(Some(&exchange.access_token)),
            refresh_token_preview: preview_oauth_secret(exchange.refresh_token.as_deref()),
            token_type: exchange.token_type.clone(),
            scope: exchange.scope.clone(),
            expires_at: exchange.expires_at,
            email,
            chatgpt_account_id,
            chatgpt_plan_type: plan_type,
            raw_exchange_payload: Some(exchange.raw_payload.clone()),
            id_token_claims_raw: exchange
                .id_token_claims
                .as_ref()
                .map(|claims| claims.raw_claims.clone()),
        });
        codex_probe_session_response(session)
    };

    stop_codex_oauth_callback_listener_if_idle(state).await;
    Ok(response)
}

async fn create_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateTenantRequest>,
) -> Result<Json<codex_pool_core::model::Tenant>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    state
        .store
        .create_tenant(req)
        .await
        .map(Json)
        .map_err(internal_error)
}

async fn list_tenants(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<codex_pool_core::model::Tenant>>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    state
        .store
        .list_tenants()
        .await
        .map(Json)
        .map_err(internal_error)
}

async fn create_api_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<Json<codex_pool_core::api::CreateApiKeyResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    state
        .store
        .create_api_key(req)
        .await
        .map(Json)
        .map_err(internal_error)
}

async fn list_api_keys(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<codex_pool_core::model::ApiKey>>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    state
        .store
        .list_api_keys()
        .await
        .map(Json)
        .map_err(internal_error)
}

async fn create_upstream_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateUpstreamAccountRequest>,
) -> Result<Json<codex_pool_core::model::UpstreamAccount>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    state
        .store
        .create_upstream_account(req)
        .await
        .map(Json)
        .map_err(internal_error)
}

async fn list_upstream_accounts(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<codex_pool_core::model::UpstreamAccount>>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    state
        .store
        .list_upstream_accounts()
        .await
        .map(Json)
        .map_err(internal_error)
}

async fn update_upstream_account_enabled(
    Path(account_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpstreamAccountPatchRequest>,
) -> Result<Json<codex_pool_core::model::UpstreamAccount>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    state
        .store
        .set_upstream_account_enabled(account_id, req.enabled)
        .await
        .map(Json)
        .map_err(internal_error)
}

async fn delete_upstream_account(
    Path(account_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<StatusCode, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    state
        .store
        .delete_upstream_account(account_id)
        .await
        .map_err(internal_error)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn batch_operate_upstream_accounts(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpstreamAccountBatchActionRequest>,
) -> Result<Json<UpstreamAccountBatchActionResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    if req.account_ids.is_empty() {
        return Err(invalid_request_error("account_ids must not be empty"));
    }
    if req.account_ids.len() > UPSTREAM_ACCOUNT_BATCH_ACTION_MAX_ITEMS {
        return Err(invalid_request_error(
            "account_ids exceeds maximum allowed batch size",
        ));
    }

    let mut seen = std::collections::HashSet::with_capacity(req.account_ids.len());
    let account_ids = req
        .account_ids
        .into_iter()
        .filter(|account_id| seen.insert(*account_id))
        .collect::<Vec<_>>();
    let action = req.action;
    let store = state.store.clone();
    let import_job_manager = state.import_job_manager.clone();
    let items = futures_util::stream::iter(account_ids.into_iter().map(|account_id| {
        let store = store.clone();
        let import_job_manager = import_job_manager.clone();
        async move {
            execute_upstream_account_batch_action_item(
                action,
                account_id,
                store,
                import_job_manager,
            )
            .await
        }
    }))
    .buffered(upstream_account_batch_action_concurrency_from_env())
    .collect::<Vec<_>>()
    .await;

    let success_count = items.iter().filter(|item| item.ok).count();
    let failed_count = items.len().saturating_sub(success_count);
    Ok(Json(UpstreamAccountBatchActionResponse {
        action,
        total: items.len(),
        success_count,
        failed_count,
        items,
    }))
}

async fn execute_upstream_account_batch_action_item(
    action: UpstreamAccountBatchActionKind,
    account_id: Uuid,
    store: Arc<dyn ControlPlaneStore>,
    import_job_manager: OAuthImportJobManager,
) -> UpstreamAccountBatchActionItem {
    let outcome = match action {
        UpstreamAccountBatchActionKind::Enable => store
            .set_upstream_account_enabled(account_id, true)
            .await
            .map(|_| None),
        UpstreamAccountBatchActionKind::Disable => store
            .set_upstream_account_enabled(account_id, false)
            .await
            .map(|_| None),
        UpstreamAccountBatchActionKind::Delete => store.delete_upstream_account(account_id).await.map(|_| None),
        UpstreamAccountBatchActionKind::RefreshLogin => import_job_manager
            .create_manual_refresh_job(account_id)
            .await
            .map(|summary| Some(summary.job_id)),
        UpstreamAccountBatchActionKind::PauseFamily => store
            .set_oauth_family_enabled(account_id, false)
            .await
            .map(|_| None),
        UpstreamAccountBatchActionKind::ResumeFamily => store
            .set_oauth_family_enabled(account_id, true)
            .await
            .map(|_| None),
    };

    match outcome {
        Ok(job_id) => UpstreamAccountBatchActionItem {
            account_id,
            ok: true,
            job_id,
            error: None,
        },
        Err(error) => {
            tracing::warn!(
                action = ?action,
                account_id = %account_id,
                error = %error,
                "upstream account batch action failed"
            );
            UpstreamAccountBatchActionItem {
                account_id,
                ok: false,
                job_id: None,
                error: Some(map_upstream_account_batch_action_error(error)),
            }
        }
    }
}

fn map_upstream_account_batch_action_error(err: anyhow::Error) -> UpstreamAccountBatchActionError {
    let lowered = err.to_string().to_ascii_lowercase();
    if lowered.contains("not found") {
        return UpstreamAccountBatchActionError {
            code: "not_found".to_string(),
            message: "resource not found".to_string(),
        };
    }
    if lowered.contains("invalid")
        || lowered.contains("oauth")
        || lowered.contains("must")
        || lowered.contains("missing")
        || lowered.contains("unsupported")
    {
        return UpstreamAccountBatchActionError {
            code: "invalid_request".to_string(),
            message: "invalid request".to_string(),
        };
    }
    UpstreamAccountBatchActionError {
        code: "internal_error".to_string(),
        message: "internal server error".to_string(),
    }
}

async fn validate_oauth_refresh_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ValidateOAuthRefreshTokenRequest>,
) -> Result<Json<ValidateOAuthRefreshTokenResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    state
        .store
        .validate_oauth_refresh_token(req)
        .await
        .map(Json)
        .map_err(internal_error)
}

async fn import_oauth_refresh_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ImportOAuthRefreshTokenRequest>,
) -> Result<Json<codex_pool_core::model::UpstreamAccount>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    state
        .store
        .upsert_oauth_refresh_token(req)
        .await
        .map(|result| Json(result.account))
        .map_err(internal_error)
}

async fn create_codex_oauth_login_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateCodexOAuthLoginSessionRequest>,
) -> Result<Json<CodexOAuthLoginSessionResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;

    let callback_url =
        resolve_codex_oauth_redirect_url().ok_or_else(codex_oauth_provider_not_configured_error)?;
    let code_verifier = random_urlsafe_token();
    let code_challenge = pkce_code_challenge(&code_verifier);
    let session_id = Uuid::new_v4().to_string();
    let state_token = random_urlsafe_token();
    let base_url = normalize_codex_import_base_url(req.base_url);
    let label = req
        .label
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let enabled = req.enabled.unwrap_or(DEFAULT_CODEX_IMPORT_ENABLED);
    let priority = req.priority.unwrap_or(DEFAULT_CODEX_IMPORT_PRIORITY);

    let oauth_client = crate::oauth::OpenAiOAuthClient::from_parts_with_outbound_proxy_runtime(
        std::env::var("OPENAI_OAUTH_TOKEN_URL")
            .unwrap_or_else(|_| "https://auth.openai.com/oauth/token".to_string()),
        std::env::var("OPENAI_OAUTH_AUTHORIZE_URL")
            .unwrap_or_else(|_| "https://auth.openai.com/oauth/authorize".to_string()),
        std::env::var("OPENAI_OAUTH_CLIENT_ID").ok(),
        std::env::var("OPENAI_OAUTH_TIMEOUT_SEC")
            .ok()
            .and_then(|value| value.parse::<u64>().ok()),
        Some(state.outbound_proxy_runtime.clone()),
    );
    let authorize_url = oauth_client
        .build_authorize_url(
            &callback_url,
            &state_token,
            &code_challenge,
            Some(DEFAULT_CODEX_OAUTH_SCOPE),
        )
        .map_err(|_| codex_oauth_provider_not_configured_error())?;
    if let Err(err) = ensure_codex_oauth_callback_listener_started(&state).await {
        tracing::warn!(
            error = %err,
            "failed to start codex oauth callback listener before creating login session"
        );
        return Err(codex_oauth_callback_listener_unavailable_error());
    }

    let now = Utc::now();
    let record = CodexOAuthLoginSessionRecord {
        session_id: session_id.clone(),
        state: state_token,
        code_verifier,
        authorize_url,
        callback_url,
        base_url,
        label,
        enabled,
        priority,
        status: CodexOAuthLoginSessionStatus::WaitingCallback,
        error: None,
        result: None,
        created_at: now,
        updated_at: now,
        expires_at: now + chrono::Duration::seconds(OAUTH_LOGIN_SESSION_TTL_SEC),
    };

    let mut sessions = state
        .oauth_login_sessions
        .write()
        .expect("oauth login session lock poisoned");
    cleanup_codex_oauth_login_sessions(&mut sessions);
    sessions.insert(session_id, record.clone());
    Ok(Json(codex_session_response(&record)))
}

async fn create_codex_oauth_probe_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateCodexOAuthProbeSessionRequest>,
) -> Result<Json<CodexOAuthProbeSessionResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;

    let callback_url = resolve_codex_oauth_probe_redirect_url()
        .ok_or_else(codex_oauth_provider_not_configured_error)?;
    let code_verifier = random_urlsafe_token();
    let code_challenge = pkce_code_challenge(&code_verifier);
    let session_id = Uuid::new_v4().to_string();
    let state_token = random_urlsafe_token();
    let base_url = normalize_codex_import_base_url(req.base_url);

    let oauth_client = crate::oauth::OpenAiOAuthClient::from_parts_with_outbound_proxy_runtime(
        std::env::var("OPENAI_OAUTH_TOKEN_URL")
            .unwrap_or_else(|_| "https://auth.openai.com/oauth/token".to_string()),
        std::env::var("OPENAI_OAUTH_AUTHORIZE_URL")
            .unwrap_or_else(|_| "https://auth.openai.com/oauth/authorize".to_string()),
        std::env::var("OPENAI_OAUTH_CLIENT_ID").ok(),
        std::env::var("OPENAI_OAUTH_TIMEOUT_SEC")
            .ok()
            .and_then(|value| value.parse::<u64>().ok()),
        Some(state.outbound_proxy_runtime.clone()),
    );
    let authorize_url = oauth_client
        .build_authorize_url(
            &callback_url,
            &state_token,
            &code_challenge,
            Some(DEFAULT_CODEX_OAUTH_SCOPE),
        )
        .map_err(|_| codex_oauth_provider_not_configured_error())?;
    if let Err(err) = ensure_codex_oauth_callback_listener_started(&state).await {
        tracing::warn!(
            error = %err,
            "failed to start codex oauth callback listener before creating probe session"
        );
        return Err(codex_oauth_callback_listener_unavailable_error());
    }

    let now = Utc::now();
    let record = CodexOAuthProbeSessionRecord {
        session_id: session_id.clone(),
        state: state_token,
        code_verifier,
        authorize_url,
        callback_url,
        base_url,
        status: CodexOAuthLoginSessionStatus::WaitingCallback,
        error: None,
        result: None,
        created_at: now,
        updated_at: now,
        expires_at: now + chrono::Duration::seconds(OAUTH_LOGIN_SESSION_TTL_SEC),
    };

    let mut sessions = state
        .oauth_probe_sessions
        .write()
        .expect("oauth probe session lock poisoned");
    cleanup_codex_oauth_probe_sessions(&mut sessions);
    sessions.insert(session_id, record.clone());
    Ok(Json(codex_probe_session_response(&record)))
}

async fn get_codex_oauth_login_session(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CodexOAuthLoginSessionResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    let mut sessions = state
        .oauth_login_sessions
        .write()
        .expect("oauth login session lock poisoned");
    cleanup_codex_oauth_login_sessions(&mut sessions);
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(codex_oauth_not_found_error)?;
    codex_mark_expired_if_needed(session, Utc::now());
    Ok(Json(codex_session_response(session)))
}

async fn get_codex_oauth_probe_session(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CodexOAuthProbeSessionResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    let mut sessions = state
        .oauth_probe_sessions
        .write()
        .expect("oauth probe session lock poisoned");
    cleanup_codex_oauth_probe_sessions(&mut sessions);
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(codex_oauth_not_found_error)?;
    codex_mark_probe_expired_if_needed(session, Utc::now());
    Ok(Json(codex_probe_session_response(session)))
}

async fn submit_codex_oauth_login_session_callback(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SubmitCodexOAuthCallbackRequest>,
) -> Result<Json<CodexOAuthLoginSessionResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    let callback = parse_callback_query_from_redirect_url(&req.redirect_url)?;
    process_codex_oauth_callback_flow(&state, Some(session_id), callback)
        .await
        .map(Json)
}

async fn submit_codex_oauth_probe_session_callback(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SubmitCodexOAuthCallbackRequest>,
) -> Result<Json<CodexOAuthProbeSessionResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    let callback = parse_callback_query_from_redirect_url(&req.redirect_url)?;
    process_codex_oauth_probe_callback_flow(&state, Some(session_id), callback)
        .await
        .map(Json)
}

async fn handle_codex_oauth_callback(
    State(state): State<AppState>,
    Query(query): Query<CodexOAuthCallbackQuery>,
) -> impl IntoResponse {
    let callback_target = codex_callback_state(&query)
        .and_then(|state_value| codex_find_callback_target_by_state(&state, state_value));

    match callback_target {
        Some(CodexOAuthCallbackTarget::Probe) => {
            match process_codex_oauth_probe_callback_flow(&state, None, query).await {
                Ok(response) => codex_oauth_callback_success_response(format!(
                    "Probe session {} completed successfully.",
                    response.session_id
                )),
                Err((_, Json(envelope))) => codex_oauth_callback_failure_response(envelope),
            }
        }
        _ => match process_codex_oauth_callback_flow(&state, None, query).await {
            Ok(response) => codex_oauth_callback_success_response(format!(
                "Session {} imported successfully.",
                response.session_id
            )),
            Err((_, Json(envelope))) => codex_oauth_callback_failure_response(envelope),
        },
    }
}

async fn handle_codex_oauth_probe_callback(
    State(state): State<AppState>,
    Query(query): Query<CodexOAuthCallbackQuery>,
) -> impl IntoResponse {
    match process_codex_oauth_probe_callback_flow(&state, None, query).await {
        Ok(response) => (
            StatusCode::OK,
            axum::response::Html(codex_oauth_callback_html(
                true,
                &format!("Probe session {} completed successfully.", response.session_id),
            )),
        )
            .into_response(),
        Err((_, Json(envelope))) => (
            StatusCode::OK,
            axum::response::Html(codex_oauth_callback_html(
                false,
                &format!("{} ({})", envelope.error.message, envelope.error.code),
            )),
        )
            .into_response(),
    }
}

async fn refresh_oauth_account(
    Path(account_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<OAuthAccountStatusResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    state
        .store
        .refresh_oauth_account(account_id)
        .await
        .map(Json)
        .map_err(internal_error)
}

async fn create_oauth_refresh_job(
    Path(account_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<OAuthImportJobSummary>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    state
        .import_job_manager
        .create_manual_refresh_job(account_id)
        .await
        .map(Json)
        .map_err(internal_error)
}

async fn get_oauth_account_status(
    Path(account_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<OAuthAccountStatusResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    state
        .store
        .oauth_account_status(account_id)
        .await
        .map(Json)
        .map_err(internal_error)
}

async fn get_oauth_account_statuses(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<OAuthAccountStatusesRequest>,
) -> Result<Json<OAuthAccountStatusesResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    let account_ids = req.account_ids;
    if account_ids.is_empty() {
        return Ok(Json(OAuthAccountStatusesResponse { items: Vec::new() }));
    }

    state
        .store
        .oauth_account_statuses(account_ids)
        .await
        .map(|items| Json(OAuthAccountStatusesResponse { items }))
        .map_err(internal_error)
}

fn map_oauth_rate_limit_refresh_job_error(
    err: anyhow::Error,
) -> (StatusCode, Json<ErrorEnvelope>) {
    if err
        .to_string()
        .to_ascii_lowercase()
        .contains("job not found")
    {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorEnvelope::new("not_found", "resource not found")),
        );
    }
    internal_error(err)
}

async fn create_oauth_rate_limit_refresh_job(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<OAuthRateLimitRefreshJobSummary>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    let summary = state
        .store
        .create_oauth_rate_limit_refresh_job()
        .await
        .map_err(map_oauth_rate_limit_refresh_job_error)?;

    if summary.status == OAuthRateLimitRefreshJobStatus::Queued {
        let store = state.store.clone();
        let job_id = summary.job_id;
        tokio::spawn(async move {
            if let Err(err) = store.run_oauth_rate_limit_refresh_job(job_id).await {
                tracing::warn!(job_id = %job_id, error = %err, "oauth rate-limit refresh job run failed");
            }
        });
    }

    Ok(Json(summary))
}

async fn get_oauth_rate_limit_refresh_job(
    Path(job_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<OAuthRateLimitRefreshJobSummary>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    state
        .store
        .oauth_rate_limit_refresh_job(job_id)
        .await
        .map(Json)
        .map_err(map_oauth_rate_limit_refresh_job_error)
}

async fn disable_oauth_account_family(
    Path(account_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<OAuthFamilyActionResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    state
        .store
        .set_oauth_family_enabled(account_id, false)
        .await
        .map(Json)
        .map_err(internal_error)
}

async fn enable_oauth_account_family(
    Path(account_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<OAuthFamilyActionResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    state
        .store
        .set_oauth_family_enabled(account_id, true)
        .await
        .map(Json)
        .map_err(internal_error)
}

async fn admin_login(
    State(state): State<AppState>,
    Json(req): Json<AdminLoginRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorEnvelope>)> {
    let response = state.admin_auth.login(req).await.map_err(internal_error)?;
    let Some(response) = response else {
        return Err(admin_unauthorized_error());
    };
    let cookie_value = build_admin_session_cookie(&response.access_token, response.expires_in);
    let cookie_header = axum::http::HeaderValue::from_str(&cookie_value).map_err(|err| {
        internal_error(anyhow::anyhow!("failed to encode admin session cookie: {err}"))
    })?;

    let mut http_response = Json(response).into_response();
    http_response
        .headers_mut()
        .insert(axum::http::header::SET_COOKIE, cookie_header);

    Ok(http_response)
}

async fn admin_logout() -> Result<impl IntoResponse, (StatusCode, Json<ErrorEnvelope>)> {
    let cookie_value = build_admin_session_clear_cookie();
    let cookie_header = axum::http::HeaderValue::from_str(&cookie_value).map_err(|err| {
        internal_error(anyhow::anyhow!(
            "failed to encode admin session clear-cookie: {err}"
        ))
    })?;
    let mut http_response = StatusCode::NO_CONTENT.into_response();
    http_response
        .headers_mut()
        .insert(axum::http::header::SET_COOKIE, cookie_header);
    Ok(http_response)
}

async fn admin_me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminMeResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let principal = require_admin_principal(&state, &headers)?;
    Ok(Json(state.admin_auth.me(&principal)))
}

async fn tenant_register(
    State(state): State<AppState>,
    Json(req): Json<crate::tenant::TenantRegisterRequest>,
) -> Result<Json<crate::tenant::TenantRegisterResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let tenant_auth = require_tenant_auth_service(&state)?;
    tenant_auth
        .register(req)
        .await
        .map(Json)
        .map_err(map_tenant_error)
}

async fn tenant_verify_email(
    State(state): State<AppState>,
    Json(req): Json<crate::tenant::TenantVerifyEmailRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorEnvelope>)> {
    let tenant_auth = require_tenant_auth_service(&state)?;
    tenant_auth
        .verify_email(req)
        .await
        .map_err(map_tenant_error)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn tenant_login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<crate::tenant::TenantLoginRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorEnvelope>)> {
    let tenant_auth = require_tenant_auth_service(&state)?;
    let request_ip = crate::tenant::extract_client_ip(&headers);
    let response = tenant_auth
        .login(req, request_ip.as_deref())
        .await
        .map_err(map_tenant_error)?;
    let Some(response) = response else {
        return Err(tenant_unauthorized_error());
    };
    let cookie_value = tenant_auth.build_session_cookie(&response.access_token);
    let cookie_header = axum::http::HeaderValue::from_str(&cookie_value).map_err(|err| {
        internal_error(anyhow::anyhow!("failed to encode tenant session cookie: {err}"))
    })?;
    let mut http_response = Json(response).into_response();
    http_response
        .headers_mut()
        .insert(axum::http::header::SET_COOKIE, cookie_header);
    Ok(http_response)
}

async fn tenant_logout(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorEnvelope>)> {
    let tenant_auth = require_tenant_auth_service(&state)?;
    let cookie_value = tenant_auth.build_session_clear_cookie();
    let cookie_header = axum::http::HeaderValue::from_str(&cookie_value).map_err(|err| {
        internal_error(anyhow::anyhow!(
            "failed to encode tenant session clear-cookie: {err}"
        ))
    })?;
    let mut http_response = StatusCode::NO_CONTENT.into_response();
    http_response
        .headers_mut()
        .insert(axum::http::header::SET_COOKIE, cookie_header);
    Ok(http_response)
}

async fn tenant_me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<crate::tenant::TenantMeResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let tenant_auth = require_tenant_auth_service(&state)?;
    let principal = require_tenant_principal(&state, &headers).await?;
    Ok(Json(tenant_auth.me(&principal)))
}

async fn tenant_forgot_password(
    State(state): State<AppState>,
    Json(req): Json<crate::tenant::TenantForgotPasswordRequest>,
) -> Result<Json<TenantAcceptedResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let tenant_auth = require_tenant_auth_service(&state)?;
    let debug_code = tenant_auth
        .forgot_password(req)
        .await
        .map_err(map_tenant_error)?;
    Ok(Json(TenantAcceptedResponse {
        accepted: true,
        debug_code,
    }))
}

async fn tenant_reset_password(
    State(state): State<AppState>,
    Json(req): Json<crate::tenant::TenantResetPasswordRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorEnvelope>)> {
    let tenant_auth = require_tenant_auth_service(&state)?;
    tenant_auth
        .reset_password(req)
        .await
        .map_err(map_tenant_error)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_tenant_api_keys(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<crate::tenant::TenantApiKeyRecord>>, (StatusCode, Json<ErrorEnvelope>)> {
    let tenant_auth = require_tenant_auth_service(&state)?;
    let principal = require_tenant_principal(&state, &headers).await?;
    tenant_auth
        .list_tenant_api_keys(principal.tenant_id)
        .await
        .map(Json)
        .map_err(map_tenant_error)
}

async fn list_tenant_api_key_groups(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<crate::tenant::ApiKeyGroupItem>>, (StatusCode, Json<ErrorEnvelope>)> {
    let tenant_auth = require_tenant_auth_service(&state)?;
    let _principal = require_tenant_principal(&state, &headers).await?;
    tenant_auth
        .tenant_list_api_key_groups()
        .await
        .map(Json)
        .map_err(map_tenant_error)
}

async fn create_tenant_api_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<crate::tenant::TenantCreateApiKeyRequest>,
) -> Result<Json<crate::tenant::TenantCreateApiKeyResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let tenant_auth = require_tenant_auth_service(&state)?;
    let principal = require_tenant_principal(&state, &headers).await?;
    let response = tenant_auth
        .create_tenant_api_key(principal.tenant_id, req)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "tenant_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: Some(principal.tenant_id),
            action: "tenant.api_key.create".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("api_key".to_string()),
            target_id: Some(response.record.id.to_string()),
            payload_json: json!({
                "name": response.record.name.clone(),
                "ip_allowlist_count": response.record.ip_allowlist.len(),
                "model_allowlist_count": response.record.model_allowlist.len(),
                "group_id": response.record.group_id,
                "group_name": response.record.group.name.clone(),
            }),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(Json(response))
}

async fn patch_tenant_api_key(
    Path(key_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<crate::tenant::TenantPatchApiKeyRequest>,
) -> Result<Json<crate::tenant::TenantApiKeyRecord>, (StatusCode, Json<ErrorEnvelope>)> {
    let tenant_auth = require_tenant_auth_service(&state)?;
    let principal = require_tenant_principal(&state, &headers).await?;
    let response = tenant_auth
        .patch_tenant_api_key(principal.tenant_id, key_id, req)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "tenant_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: Some(principal.tenant_id),
            action: "tenant.api_key.patch".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("api_key".to_string()),
            target_id: Some(response.id.to_string()),
            payload_json: json!({
                "enabled": response.enabled,
                "ip_allowlist_count": response.ip_allowlist.len(),
                "model_allowlist_count": response.model_allowlist.len(),
                "group_id": response.group_id,
                "group_name": response.group.name.clone(),
            }),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(Json(response))
}

async fn delete_tenant_api_key(
    Path(key_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorEnvelope>)> {
    let tenant_auth = require_tenant_auth_service(&state)?;
    let principal = require_tenant_principal(&state, &headers).await?;
    tenant_auth
        .delete_tenant_api_key(principal.tenant_id, key_id)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "tenant_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: Some(principal.tenant_id),
            action: "tenant.api_key.delete".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("api_key".to_string()),
            target_id: Some(key_id.to_string()),
            payload_json: json!({}),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_tenant_credit_balance(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<crate::tenant::TenantCreditBalanceResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let tenant_auth = require_tenant_auth_service(&state)?;
    let principal = require_tenant_principal(&state, &headers).await?;
    let response = tenant_auth
        .get_credit_balance(principal.tenant_id)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "tenant_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: Some(principal.tenant_id),
            action: "tenant.credits.balance.get".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("tenant_credit_account".to_string()),
            target_id: Some(principal.tenant_id.to_string()),
            payload_json: json!({
                "balance_microcredits": response.balance_microcredits,
                "updated_at": response.updated_at,
            }),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(Json(response))
}

async fn get_tenant_credit_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<crate::tenant::TenantCreditSummaryResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let tenant_auth = require_tenant_auth_service(&state)?;
    let principal = require_tenant_principal(&state, &headers).await?;
    let response = tenant_auth
        .get_credit_summary(principal.tenant_id)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "tenant_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: Some(principal.tenant_id),
            action: "tenant.credits.summary.get".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("tenant_credit_summary".to_string()),
            target_id: Some(principal.tenant_id.to_string()),
            payload_json: json!({
                "balance_microcredits": response.balance_microcredits,
                "today_consumed_microcredits": response.today_consumed_microcredits,
                "month_consumed_microcredits": response.month_consumed_microcredits,
                "updated_at": response.updated_at,
            }),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(Json(response))
}

async fn list_tenant_credit_ledger(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<TenantCreditLedgerQuery>,
) -> Result<Json<crate::tenant::TenantCreditLedgerResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let tenant_auth = require_tenant_auth_service(&state)?;
    let principal = require_tenant_principal(&state, &headers).await?;
    let limit = query.limit.unwrap_or(100);
    let response = tenant_auth
        .list_credit_ledger(principal.tenant_id, limit)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "tenant_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: Some(principal.tenant_id),
            action: "tenant.credits.ledger.list".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("tenant_credit_ledger".to_string()),
            target_id: Some(principal.tenant_id.to_string()),
            payload_json: json!({
                "limit": limit,
                "item_count": response.items.len(),
            }),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(Json(response))
}
