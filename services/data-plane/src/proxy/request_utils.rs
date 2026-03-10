fn build_upstream_url(
    base: &str,
    mode: &UpstreamMode,
    path: &str,
    query: Option<&str>,
) -> anyhow::Result<String> {
    let mut base_url = url::Url::parse(base)?;
    let final_path = normalize_upstream_path(mode, base_url.path(), path);
    let final_query = normalize_upstream_query(mode, &final_path, query);

    base_url.set_path(&final_path);
    base_url.set_query(final_query.as_deref());

    Ok(base_url.to_string())
}

fn build_upstream_ws_url(
    base: &str,
    mode: &UpstreamMode,
    path: &str,
    query: Option<&str>,
) -> anyhow::Result<url::Url> {
    let mut base_url = url::Url::parse(base)?;
    let final_path = normalize_upstream_path(mode, base_url.path(), path);
    base_url.set_path(&final_path);
    base_url.set_query(query);

    match base_url.scheme() {
        "http" => {
            base_url
                .set_scheme("ws")
                .map_err(|_| anyhow::anyhow!("failed to rewrite ws scheme"))?;
        }
        "https" => {
            base_url
                .set_scheme("wss")
                .map_err(|_| anyhow::anyhow!("failed to rewrite wss scheme"))?;
        }
        "ws" | "wss" => {}
        scheme => anyhow::bail!("unsupported upstream websocket scheme: {scheme}"),
    }

    Ok(base_url)
}

fn normalize_upstream_path(mode: &UpstreamMode, base_path: &str, path: &str) -> String {
    let target_path = canonical_codex_path(mode, base_path, path).unwrap_or(path);
    compose_upstream_path(base_path, target_path)
}

fn normalize_upstream_query(
    mode: &UpstreamMode,
    final_path: &str,
    query: Option<&str>,
) -> Option<String> {
    if should_enforce_codex_models_client_version(mode, final_path) {
        return Some(ensure_client_version_query(query));
    }
    query.map(ToString::to_string)
}

fn canonical_codex_path<'a>(
    mode: &UpstreamMode,
    base_path: &str,
    path: &'a str,
) -> Option<&'a str> {
    if !is_chatgpt_codex_profile(mode, base_path) {
        return None;
    }

    match normalize_input_path(path).as_str() {
        "/v1/responses" | "/backend-api/codex/responses" => Some("/responses"),
        "/v1/responses/compact" => Some("/responses/compact"),
        "/v1/memories/trace_summarize" => Some("/memories/trace_summarize"),
        "/v1/models" | "/backend-api/codex/models" => Some("/models"),
        _ => None,
    }
}

fn is_chatgpt_codex_profile(mode: &UpstreamMode, base_path: &str) -> bool {
    if !matches!(
        mode,
        UpstreamMode::ChatGptSession | UpstreamMode::CodexOauth
    ) {
        return false;
    }
    base_path
        .trim_end_matches('/')
        .ends_with("/backend-api/codex")
}

fn should_enforce_codex_models_client_version(mode: &UpstreamMode, final_path: &str) -> bool {
    matches!(
        mode,
        UpstreamMode::ChatGptSession | UpstreamMode::CodexOauth
    ) && final_path.ends_with(CODEX_MODELS_PATH_SUFFIX)
}

fn ensure_client_version_query(query: Option<&str>) -> String {
    if let Some(raw) = query {
        let has_client_version = url::form_urlencoded::parse(raw.as_bytes())
            .any(|(key, _)| key == CODEX_CLIENT_VERSION_QUERY_KEY);
        if has_client_version {
            return raw.to_string();
        }
    }

    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    if let Some(raw) = query {
        for (key, value) in url::form_urlencoded::parse(raw.as_bytes()) {
            serializer.append_pair(&key, &value);
        }
    }
    serializer.append_pair(CODEX_CLIENT_VERSION_QUERY_KEY, env!("CARGO_PKG_VERSION"));
    serializer.finish()
}

fn normalize_input_path(path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    }
}

#[derive(Debug, Clone)]
struct UpstreamRequestCompatibilityAdaptation {
    body: bytes::Bytes,
    strip_content_encoding: bool,
    bridge_stream_response: bool,
}

fn maybe_adapt_openai_responses_request_for_codex_profile(
    mode: &UpstreamMode,
    base_path: &str,
    path: &str,
    headers: &HeaderMap,
    body: &bytes::Bytes,
) -> Option<UpstreamRequestCompatibilityAdaptation> {
    if !is_chatgpt_codex_profile(mode, base_path) {
        return None;
    }
    let normalized_path = normalize_input_path(path);
    if !matches!(normalized_path.as_str(), "/v1/responses" | "/v1/responses/compact") {
        return None;
    }
    let is_compact = normalized_path == "/v1/responses/compact";

    let mut value = parse_request_json_body(headers, body)?;
    let object = value.as_object_mut()?;
    let original_stream = object.get("stream").and_then(Value::as_bool).unwrap_or(false);
    let mut changed = false;

    if !matches!(object.get("instructions"), Some(Value::String(_))) {
        object.insert("instructions".to_string(), Value::String(String::new()));
        changed = true;
    }

    if let Some(adapted_input) = object.get("input").cloned().map(adapt_codex_input_value) {
        if object.get("input") != Some(&adapted_input) {
            object.insert("input".to_string(), adapted_input);
            changed = true;
        }
    }

    if is_compact {
        if object.remove("store").is_some() {
            changed = true;
        }
        if object.remove("stream").is_some() {
            changed = true;
        }
    } else {
        if object.get("store").and_then(Value::as_bool) != Some(false) {
            object.insert("store".to_string(), Value::Bool(false));
            changed = true;
        }

        if !original_stream {
            object.insert("stream".to_string(), Value::Bool(true));
            changed = true;
        }
    }

    if object.remove("max_output_tokens").is_some() {
        changed = true;
    }

    if !changed {
        return None;
    }

    let body = bytes::Bytes::from(serde_json::to_vec(&value).ok()?);
    Some(UpstreamRequestCompatibilityAdaptation {
        body,
        strip_content_encoding: headers.contains_key(axum::http::header::CONTENT_ENCODING),
        bridge_stream_response: !is_compact && !original_stream,
    })
}

fn adapt_codex_input_value(input: Value) -> Value {
    match input {
        Value::String(text) => serde_json::json!([{
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": text
            }]
        }]),
        Value::Array(items) => Value::Array(items),
        Value::Null => Value::Array(Vec::new()),
        other => Value::Array(vec![other]),
    }
}

fn sticky_session_key_from_headers(headers: &HeaderMap) -> Option<String> {
    for header_name in [
        SESSION_ID_HEADER,
        CONVERSATION_ID_HEADER,
        X_SESSION_ID_HEADER,
        "x-codex-turn-state",
    ] {
        if let Some(raw_value) = headers.get(header_name) {
            if let Ok(value) = raw_value.to_str() {
                let value = value.trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpstreamErrorClass {
    TokenInvalidated,
    AuthExpired,
    AccountDeactivated,
    QuotaExhausted,
    RateLimited,
    Overloaded,
    UpstreamUnavailable,
    TransientServer,
    NonRetryableClient,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UpstreamErrorContext {
    status: StatusCode,
    error_code: Option<String>,
    error_message: Option<String>,
    retry_after: Option<Duration>,
    class: UpstreamErrorClass,
}

fn is_failover_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::UNAUTHORIZED
        || status == StatusCode::FORBIDDEN
        || status == StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
}

fn is_failover_retryable_error(
    status: StatusCode,
    error_context: Option<&UpstreamErrorContext>,
) -> bool {
    let Some(context) = error_context else {
        return is_failover_retryable_status(status);
    };
    match context.class {
        UpstreamErrorClass::NonRetryableClient => false,
        UpstreamErrorClass::Unknown => is_failover_retryable_status(status),
        _ => true,
    }
}

fn should_retry_same_account_on_transport(
    same_account_retry_attempt: u32,
    state: &AppState,
) -> bool {
    same_account_retry_attempt < state.same_account_quick_retry_max
}

fn should_retry_same_account_on_status(
    status: StatusCode,
    _is_503_overloaded: bool,
    same_account_retry_attempt: u32,
    state: &AppState,
    error_context: Option<&UpstreamErrorContext>,
) -> bool {
    if same_account_retry_attempt >= state.same_account_quick_retry_max {
        return false;
    }
    if let Some(context) = error_context {
        if !matches!(
            context.class,
            UpstreamErrorClass::TransientServer | UpstreamErrorClass::Overloaded
        ) {
            return false;
        }
    }
    status.is_server_error()
        && status != StatusCode::UNAUTHORIZED
        && status != StatusCode::FORBIDDEN
}

fn has_untried_cross_account_candidate(
    state: &AppState,
    sticky_key: Option<&str>,
    tried_account_ids: &HashSet<Uuid>,
    current_account_id: Uuid,
) -> bool {
    let mut excluded_account_ids = tried_account_ids.clone();
    excluded_account_ids.insert(current_account_id);
    state
        .router
        .pick_with_policy(
            sticky_key,
            &excluded_account_ids,
            state.sticky_prefer_non_conflicting,
        )
        .is_some()
}

fn should_cross_account_failover(
    state: &AppState,
    sticky_key: Option<&str>,
    failover_deadline: Instant,
    tried_account_ids: &HashSet<Uuid>,
    current_account_id: Uuid,
    retryable: bool,
) -> bool {
    if !state.enable_request_failover || !retryable {
        return false;
    }
    if Instant::now() < failover_deadline {
        return true;
    }
    has_untried_cross_account_candidate(state, sticky_key, tried_account_ids, current_account_id)
}

fn record_same_account_retry(state: &AppState) {
    state
        .same_account_retry_total
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

fn record_cross_account_failover_attempt(
    state: &AppState,
    tried_account_ids: &mut HashSet<Uuid>,
    account_id: Uuid,
    did_cross_account_failover: &mut bool,
) {
    if tried_account_ids.insert(account_id) {
        *did_cross_account_failover = true;
        state
            .failover_attempt_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

fn record_failover_success_if_needed(state: &AppState, did_cross_account_failover: bool) {
    if did_cross_account_failover {
        state
            .failover_success_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

fn record_failover_exhausted_if_needed(state: &AppState, did_cross_account_failover: bool) {
    if did_cross_account_failover {
        state
            .failover_exhausted_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

fn invalid_request_guard_key(principal: Option<&ApiPrincipal>) -> Option<String> {
    let principal = principal?;
    if let Some(api_key_id) = principal.api_key_id {
        return Some(format!("api_key:{api_key_id}"));
    }
    if let Some(tenant_id) = principal.tenant_id {
        return Some(format!(
            "tenant:{tenant_id}:token:{}:{}",
            principal.token.len(),
            principal.token.chars().take(8).collect::<String>()
        ));
    }
    Some(format!(
        "token:{}:{}",
        principal.token.len(),
        principal.token.chars().take(8).collect::<String>()
    ))
}

fn prune_invalid_request_window(
    hits: &mut VecDeque<Instant>,
    now: Instant,
    window: Duration,
) {
    while hits
        .front()
        .is_some_and(|item| now.duration_since(*item) > window)
    {
        hits.pop_front();
    }
}

fn enforce_invalid_request_guard(
    state: &AppState,
    principal: Option<&ApiPrincipal>,
) -> Option<Response> {
    if !state.invalid_request_guard_enabled {
        return None;
    }
    let key = invalid_request_guard_key(principal)?;
    let now = Instant::now();
    let Ok(mut guard) = state.invalid_request_guard.write() else {
        warn!("invalid request guard lock poisoned; guard skipped");
        return None;
    };
    let mut should_remove = false;
    let mut blocked = false;

    if let Some((hits, blocked_until)) = guard.get_mut(&key) {
        prune_invalid_request_window(hits, now, state.invalid_request_guard_window);
        if blocked_until.is_some_and(|until| until <= now) {
            *blocked_until = None;
        }
        blocked = blocked_until.is_some_and(|until| until > now);
        should_remove = hits.is_empty() && blocked_until.is_none();
    }

    if should_remove {
        guard.remove(&key);
    }
    drop(guard);

    if !blocked {
        return None;
    }

    warn!(
        tenant_id = ?principal.and_then(|item| item.tenant_id),
        api_key_id = ?principal.and_then(|item| item.api_key_id),
        "invalid request guard blocked request"
    );
    Some(json_error(
        StatusCode::TOO_MANY_REQUESTS,
        "invalid_request_rate_limited",
        "too many invalid requests; retry later",
    ))
}

fn should_track_invalid_request_failure(
    status: StatusCode,
    error_context: Option<&UpstreamErrorContext>,
) -> bool {
    match error_context {
        Some(context) => matches!(context.class, UpstreamErrorClass::NonRetryableClient),
        None => status.is_client_error(),
    }
}

fn record_invalid_request_guard_failure(
    state: &AppState,
    principal: Option<&ApiPrincipal>,
    status: StatusCode,
    error_context: Option<&UpstreamErrorContext>,
) {
    if !state.invalid_request_guard_enabled {
        return;
    }
    if !should_track_invalid_request_failure(status, error_context) {
        return;
    }
    let Some(key) = invalid_request_guard_key(principal) else {
        return;
    };
    let now = Instant::now();
    let Ok(mut guard) = state.invalid_request_guard.write() else {
        warn!("invalid request guard lock poisoned; skip failure record");
        return;
    };

    let (hits, blocked_until) = guard
        .entry(key)
        .or_insert_with(|| (VecDeque::new(), None));
    prune_invalid_request_window(hits, now, state.invalid_request_guard_window);
    if blocked_until.is_some_and(|until| until <= now) {
        *blocked_until = None;
    }
    if blocked_until.is_some_and(|until| until > now) {
        return;
    }

    hits.push_back(now);
    if hits.len() < state.invalid_request_guard_threshold {
        return;
    }

    *blocked_until = Some(now + state.invalid_request_guard_block_ttl);
    hits.clear();
    state
        .invalid_request_guard_block_total
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    warn!(
        tenant_id = ?principal.and_then(|item| item.tenant_id),
        api_key_id = ?principal.and_then(|item| item.api_key_id),
        status_code = status.as_u16(),
        threshold = state.invalid_request_guard_threshold,
        block_ttl_sec = state.invalid_request_guard_block_ttl.as_secs(),
        "invalid request guard activated"
    );
}

fn recovery_action_label(error_context: Option<&UpstreamErrorContext>) -> &'static str {
    match recovery_action_for_error_context(error_context) {
        Some(ProxyRecoveryAction::RotateRefreshToken) => "rotate_refresh_token",
        Some(ProxyRecoveryAction::DisableAccount) => "disable_account",
        None => "none",
    }
}

fn upstream_error_class_label(error_context: Option<&UpstreamErrorContext>) -> &'static str {
    match error_context.map(|context| context.class) {
        Some(UpstreamErrorClass::TokenInvalidated) => "token_invalidated",
        Some(UpstreamErrorClass::AuthExpired) => "auth_expired",
        Some(UpstreamErrorClass::AccountDeactivated) => "account_deactivated",
        Some(UpstreamErrorClass::QuotaExhausted) => "quota_exhausted",
        Some(UpstreamErrorClass::RateLimited) => "rate_limited",
        Some(UpstreamErrorClass::Overloaded) => "overloaded",
        Some(UpstreamErrorClass::UpstreamUnavailable) => "upstream_unavailable",
        Some(UpstreamErrorClass::TransientServer) => "transient_server",
        Some(UpstreamErrorClass::NonRetryableClient) => "non_retryable_client",
        Some(UpstreamErrorClass::Unknown) => "unknown",
        None => "none",
    }
}

fn recovery_outcome_label(outcome: ProxyRecoveryOutcome) -> &'static str {
    match outcome {
        ProxyRecoveryOutcome::NotApplied => "not_applied",
        ProxyRecoveryOutcome::RotateSucceeded => "rotate_succeeded",
        ProxyRecoveryOutcome::RotateFailed => "rotate_failed",
        ProxyRecoveryOutcome::DisableAttempted => "disable_attempted",
    }
}

#[allow(clippy::too_many_arguments)]
fn log_failover_decision(
    account_id: Option<Uuid>,
    status: Option<StatusCode>,
    upstream_error_code: Option<&str>,
    upstream_error_class: &str,
    reason_class: &str,
    recovery_action: &str,
    recovery_outcome: &str,
    action: &str,
) {
    warn!(
        account_id = ?account_id,
        status_code = ?status.map(|item| item.as_u16()),
        upstream_error_code = upstream_error_code.unwrap_or("none"),
        upstream_error_class,
        reason_class,
        recovery_action,
        recovery_outcome,
        action,
        "proxy failover decision"
    );
}

#[allow(clippy::too_many_arguments)]
async fn emit_request_log_event(
    state: &AppState,
    account_id: Uuid,
    principal: Option<&ApiPrincipal>,
    path: &str,
    method: &str,
    status: StatusCode,
    started: Instant,
    is_stream: bool,
    request_id: Option<&str>,
    model: Option<&str>,
) {
    emit_request_log_event_with_billing(
        state, account_id, principal, path, method, status, started, is_stream, request_id, model,
        None, None, None, None, None, None, None, None,
    )
    .await;
}

#[allow(clippy::too_many_arguments)]
async fn emit_request_log_event_with_billing(
    state: &AppState,
    account_id: Uuid,
    principal: Option<&ApiPrincipal>,
    path: &str,
    method: &str,
    status: StatusCode,
    started: Instant,
    is_stream: bool,
    request_id: Option<&str>,
    model: Option<&str>,
    billing_phase: Option<&str>,
    authorization_id: Option<Uuid>,
    capture_status: Option<&str>,
    input_tokens: Option<i64>,
    cached_input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    reasoning_tokens: Option<i64>,
    first_token_latency_ms: Option<u64>,
) {
    state
        .event_sink
        .emit_request_log(RequestLogEvent {
            id: Uuid::new_v4(),
            account_id,
            tenant_id: principal.and_then(|item| item.tenant_id),
            api_key_id: principal.and_then(|item| item.api_key_id),
            event_version: 2,
            path: path.to_string(),
            method: method.to_string(),
            status_code: status.as_u16(),
            latency_ms: started.elapsed().as_millis() as u64,
            is_stream,
            error_code: (status.as_u16() >= 400).then(|| status.as_str().to_string()),
            request_id: request_id.map(ToString::to_string),
            model: model.map(ToString::to_string),
            input_tokens,
            cached_input_tokens,
            output_tokens,
            reasoning_tokens,
            first_token_latency_ms,
            billing_phase: billing_phase.map(ToString::to_string),
            authorization_id,
            capture_status: capture_status.map(ToString::to_string),
            created_at: chrono::Utc::now(),
        })
        .await;
}

fn ejection_ttl_for_status(
    status: StatusCode,
    base_ejection_ttl: Duration,
    is_503_overloaded: bool,
) -> Option<Duration> {
    if status == StatusCode::TOO_MANY_REQUESTS || is_503_overloaded {
        return Some(base_ejection_ttl);
    }

    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return Some(auth_error_ejection_ttl(base_ejection_ttl));
    }

    if status.is_server_error() {
        return Some(server_error_ejection_ttl(base_ejection_ttl));
    }

    None
}

fn ejection_ttl_for_response(
    status: StatusCode,
    base_ejection_ttl: Duration,
    is_503_overloaded: bool,
    error_context: Option<&UpstreamErrorContext>,
    recovery_outcome: ProxyRecoveryOutcome,
) -> Option<Duration> {
    let Some(context) = error_context else {
        return ejection_ttl_for_status(status, base_ejection_ttl, is_503_overloaded);
    };

    match context.class {
        UpstreamErrorClass::TokenInvalidated => match recovery_outcome {
            ProxyRecoveryOutcome::RotateSucceeded => Some(Duration::from_secs(
                TOKEN_INVALIDATED_RECOVERY_EJECTION_SEC.min(base_ejection_ttl.as_secs().max(1)),
            )),
            _ => Some(auth_expired_ejection_ttl(base_ejection_ttl)),
        },
        UpstreamErrorClass::AuthExpired => Some(auth_expired_ejection_ttl(base_ejection_ttl)),
        UpstreamErrorClass::AccountDeactivated => Some(base_ejection_ttl),
        UpstreamErrorClass::QuotaExhausted => Some(quota_exhausted_ejection_ttl(context)),
        UpstreamErrorClass::RateLimited => Some(rate_limited_ejection_ttl(base_ejection_ttl, context)),
        UpstreamErrorClass::Overloaded => Some(base_ejection_ttl),
        UpstreamErrorClass::UpstreamUnavailable => Some(base_ejection_ttl),
        UpstreamErrorClass::TransientServer => Some(server_error_ejection_ttl(base_ejection_ttl)),
        UpstreamErrorClass::NonRetryableClient => None,
        UpstreamErrorClass::Unknown => {
            ejection_ttl_for_status(context.status, base_ejection_ttl, is_503_overloaded)
        }
    }
}

fn recovery_action_for_upstream_error_code(code: Option<&str>) -> Option<ProxyRecoveryAction> {
    match code {
        Some("token_invalidated") => Some(ProxyRecoveryAction::RotateRefreshToken),
        Some("account_deactivated") => Some(ProxyRecoveryAction::DisableAccount),
        Some("refresh_token_reused") | Some("refresh_token_revoked") => {
            Some(ProxyRecoveryAction::DisableAccount)
        }
        _ => None,
    }
}

fn recovery_action_for_error_context(
    error_context: Option<&UpstreamErrorContext>,
) -> Option<ProxyRecoveryAction> {
    let context = error_context?;
    recovery_action_for_upstream_error_code(context.error_code.as_deref()).or(match context.class {
        UpstreamErrorClass::TokenInvalidated | UpstreamErrorClass::AuthExpired => {
            Some(ProxyRecoveryAction::RotateRefreshToken)
        }
        UpstreamErrorClass::AccountDeactivated => Some(ProxyRecoveryAction::DisableAccount),
        _ => None,
    })
}

#[cfg(test)]
fn extract_upstream_error_code(body: &[u8]) -> Option<String> {
    extract_upstream_error_details(body).0
}

fn extract_upstream_error_details(body: &[u8]) -> (Option<String>, Option<String>) {
    if let Ok(value) = serde_json::from_slice::<Value>(body) {
        let code = value
            .get("error")
            .and_then(|error| error.get("code"))
            .and_then(Value::as_str)
            .or_else(|| value.get("code").and_then(Value::as_str))
            .map(ToString::to_string);
        let message = value
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .or_else(|| value.get("message").and_then(Value::as_str))
            .map(ToString::to_string);
        return (code, message);
    }

    let message = std::str::from_utf8(body)
        .ok()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    (None, message)
}

fn build_upstream_error_context(
    status: StatusCode,
    headers: &HeaderMap,
    body: &[u8],
) -> Option<UpstreamErrorContext> {
    let (error_code, error_message) = extract_upstream_error_details(body);
    if status.is_success() && error_code.is_none() && error_message.is_none() {
        return None;
    }

    let class = classify_upstream_error(status, error_code.as_deref(), error_message.as_deref());
    if status.is_success() && matches!(class, UpstreamErrorClass::Unknown) {
        return None;
    }

    let normalized_status = if status.is_success() {
        status_for_error_class(class, StatusCode::BAD_GATEWAY)
    } else {
        status
    };

    Some(UpstreamErrorContext {
        status: normalized_status,
        error_code,
        error_message,
        retry_after: extract_retry_after(headers),
        class,
    })
}

fn classify_upstream_error(
    status: StatusCode,
    error_code: Option<&str>,
    error_message: Option<&str>,
) -> UpstreamErrorClass {
    let code = error_code.unwrap_or_default().to_ascii_lowercase();
    let message = error_message.unwrap_or_default().to_ascii_lowercase();

    if code == "token_invalidated" {
        return UpstreamErrorClass::TokenInvalidated;
    }
    if code == "account_deactivated" {
        return UpstreamErrorClass::AccountDeactivated;
    }
    if matches!(
        code.as_str(),
        "refresh_token_reused"
            | "refresh_token_revoked"
            | "auth_expired"
            | "token_expired"
            | "expired_token"
            | "invalid_token"
    ) {
        return UpstreamErrorClass::AuthExpired;
    }
    if matches!(
        code.as_str(),
        "usage_limit" | "insufficient_quota" | "quota_exceeded" | "billing_hard_limit_reached"
    ) {
        return UpstreamErrorClass::QuotaExhausted;
    }

    if message.contains("usage limit")
        || message.contains("insufficient quota")
        || message.contains("quota")
        || message.contains("start a free trial of plus")
    {
        return UpstreamErrorClass::QuotaExhausted;
    }
    if message.contains("access token could not be refreshed")
        || message.contains("logged out")
        || message.contains("signed in to another account")
        || message.contains("failed to extract accountid from token")
    {
        return UpstreamErrorClass::AuthExpired;
    }
    if message.contains("server is overloaded") || message.contains("slow_down") {
        return UpstreamErrorClass::Overloaded;
    }

    if status == StatusCode::UNAUTHORIZED {
        return UpstreamErrorClass::AuthExpired;
    }
    if status == StatusCode::FORBIDDEN {
        return UpstreamErrorClass::NonRetryableClient;
    }
    if status == StatusCode::TOO_MANY_REQUESTS {
        return UpstreamErrorClass::RateLimited;
    }
    if status == StatusCode::SERVICE_UNAVAILABLE {
        if matches!(code.as_str(), "server_is_overloaded" | "slow_down") {
            return UpstreamErrorClass::Overloaded;
        }
        return UpstreamErrorClass::UpstreamUnavailable;
    }
    if status.is_server_error() {
        return UpstreamErrorClass::TransientServer;
    }
    if status.is_client_error() {
        return UpstreamErrorClass::NonRetryableClient;
    }
    UpstreamErrorClass::Unknown
}

fn status_for_error_class(class: UpstreamErrorClass, fallback: StatusCode) -> StatusCode {
    match class {
        UpstreamErrorClass::TokenInvalidated | UpstreamErrorClass::AuthExpired => {
            StatusCode::UNAUTHORIZED
        }
        UpstreamErrorClass::AccountDeactivated => StatusCode::FORBIDDEN,
        UpstreamErrorClass::QuotaExhausted | UpstreamErrorClass::RateLimited => {
            StatusCode::TOO_MANY_REQUESTS
        }
        UpstreamErrorClass::Overloaded | UpstreamErrorClass::UpstreamUnavailable => {
            StatusCode::SERVICE_UNAVAILABLE
        }
        UpstreamErrorClass::TransientServer => StatusCode::BAD_GATEWAY,
        UpstreamErrorClass::NonRetryableClient | UpstreamErrorClass::Unknown => fallback,
    }
}

fn normalize_upstream_error_response(error_context: &UpstreamErrorContext) -> Response {
    let (code, message) = match error_context.class {
        UpstreamErrorClass::TokenInvalidated => (
            "token_invalidated",
            "upstream account token has been invalidated",
        ),
        UpstreamErrorClass::AuthExpired => (
            "auth_expired",
            "upstream account authentication expired; retry later with another account",
        ),
        UpstreamErrorClass::AccountDeactivated => {
            ("account_deactivated", "upstream account is deactivated")
        }
        UpstreamErrorClass::QuotaExhausted => (
            "quota_exhausted",
            "upstream account quota is exhausted; retry later",
        ),
        UpstreamErrorClass::RateLimited => {
            ("rate_limited", "upstream account is rate limited; retry later")
        }
        UpstreamErrorClass::Overloaded => ("server_overloaded", "upstream service is overloaded"),
        UpstreamErrorClass::UpstreamUnavailable => {
            ("upstream_unavailable", "upstream service is unavailable")
        }
        UpstreamErrorClass::TransientServer => {
            ("upstream_request_failed", "upstream request failed")
        }
        UpstreamErrorClass::NonRetryableClient | UpstreamErrorClass::Unknown => {
            ("upstream_request_failed", "upstream request failed")
        }
    };
    json_error(error_context.status, code, message)
}

fn extract_retry_after(headers: &HeaderMap) -> Option<Duration> {
    let raw = headers
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    if let Ok(seconds) = raw.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }
    let parsed = chrono::DateTime::parse_from_rfc2822(raw).ok()?;
    let now = chrono::Utc::now();
    let target = parsed.with_timezone(&chrono::Utc);
    if target <= now {
        return None;
    }
    let delta = target - now;
    Some(Duration::from_secs(delta.num_seconds().max(0) as u64))
}

fn auth_expired_ejection_ttl(base_ttl: Duration) -> Duration {
    let seconds = auth_error_ejection_ttl(base_ttl)
        .as_secs()
        .clamp(AUTH_EXPIRED_EJECTION_MIN_SEC, AUTH_EXPIRED_EJECTION_MAX_SEC);
    Duration::from_secs(seconds)
}

fn quota_exhausted_ejection_ttl(error_context: &UpstreamErrorContext) -> Duration {
    let seconds = error_context
        .retry_after
        .map(|value| value.as_secs())
        .unwrap_or(QUOTA_EXHAUSTED_EJECTION_MIN_SEC)
        .clamp(QUOTA_EXHAUSTED_EJECTION_MIN_SEC, QUOTA_EXHAUSTED_EJECTION_MAX_SEC);
    Duration::from_secs(seconds)
}

fn rate_limited_ejection_ttl(
    base_ejection_ttl: Duration,
    error_context: &UpstreamErrorContext,
) -> Duration {
    let base_seconds = base_ejection_ttl
        .as_secs()
        .max(RATE_LIMITED_EJECTION_MIN_SEC);
    let seconds = error_context
        .retry_after
        .map(|value| value.as_secs())
        .unwrap_or(base_seconds)
        .max(RATE_LIMITED_EJECTION_MIN_SEC);
    Duration::from_secs(seconds)
}

async fn apply_recovery_action(
    state: &AppState,
    account_id: Uuid,
    error_context: Option<&UpstreamErrorContext>,
) -> ProxyRecoveryOutcome {
    match recovery_action_for_error_context(error_context) {
        Some(ProxyRecoveryAction::RotateRefreshToken) => {
            if trigger_internal_oauth_refresh(state, account_id).await {
                ProxyRecoveryOutcome::RotateSucceeded
            } else {
                ProxyRecoveryOutcome::RotateFailed
            }
        }
        Some(ProxyRecoveryAction::DisableAccount) => {
            let _ = trigger_internal_disable_account(state, account_id).await;
            ProxyRecoveryOutcome::DisableAttempted
        }
        None => ProxyRecoveryOutcome::NotApplied,
    }
}

async fn trigger_internal_oauth_refresh(state: &AppState, account_id: Uuid) -> bool {
    let Some(base_url) = state.control_plane_base_url.as_deref() else {
        return false;
    };
    let endpoint = format!(
        "{}/internal/v1/upstream-accounts/{account_id}/oauth/refresh",
        base_url.trim_end_matches('/')
    );
    let response = match state
        .http_client
        .post(endpoint)
        .bearer_auth(state.control_plane_internal_auth_token.as_ref())
        .timeout(Duration::from_secs(INTERNAL_RECOVERY_TIMEOUT_SEC))
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(err) => {
            warn!(error = %err, account_id = %account_id, "failed to trigger internal oauth refresh");
            return false;
        }
    };
    if !response.status().is_success() {
        warn!(
            status = %response.status(),
            account_id = %account_id,
            "internal oauth refresh returned non-success status"
        );
        return false;
    }
    let payload = match response.json::<InternalOAuthRefreshPayload>().await {
        Ok(payload) => payload,
        Err(err) => {
            warn!(error = %err, account_id = %account_id, "failed to parse internal oauth refresh response");
            return false;
        }
    };
    payload.last_refresh_status == "ok"
}

async fn trigger_internal_disable_account(state: &AppState, account_id: Uuid) -> bool {
    let Some(base_url) = state.control_plane_base_url.as_deref() else {
        return false;
    };
    let endpoint = format!(
        "{}/internal/v1/upstream-accounts/{account_id}/disable",
        base_url.trim_end_matches('/')
    );
    let response = match state
        .http_client
        .post(endpoint)
        .bearer_auth(state.control_plane_internal_auth_token.as_ref())
        .timeout(Duration::from_secs(INTERNAL_RECOVERY_TIMEOUT_SEC))
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(err) => {
            warn!(error = %err, account_id = %account_id, "failed to trigger internal upstream disable");
            return false;
        }
    };
    if !response.status().is_success() {
        warn!(
            status = %response.status(),
            account_id = %account_id,
            "internal upstream disable returned non-success status"
        );
        return false;
    }
    true
}

fn auth_error_ejection_ttl(base_ttl: Duration) -> Duration {
    let seconds = base_ttl
        .as_secs()
        .saturating_mul(AUTH_ERROR_EJECTION_MULTIPLIER)
        .clamp(AUTH_ERROR_EJECTION_MIN_SEC, AUTH_ERROR_EJECTION_MAX_SEC);
    Duration::from_secs(seconds)
}

fn server_error_ejection_ttl(base_ttl: Duration) -> Duration {
    let reduced = (base_ttl.as_secs() / 3).max(SERVER_ERROR_EJECTION_MIN_SEC);
    Duration::from_secs(reduced.clamp(SERVER_ERROR_EJECTION_MIN_SEC, SERVER_ERROR_EJECTION_MAX_SEC))
}

fn compose_upstream_path(base_path: &str, path: &str) -> String {
    let base_path = base_path.trim_end_matches('/');
    let normalized_path = normalize_input_path(path);

    if base_path.is_empty() || base_path == "/" {
        return normalized_path;
    }

    let base_prefix = format!("{base_path}/");
    if normalized_path == base_path || normalized_path.starts_with(&base_prefix) {
        return normalized_path;
    }

    format!("{base_path}/{}", normalized_path.trim_start_matches('/'))
}

#[allow(clippy::too_many_arguments)]
fn build_upstream_websocket_request(
    base_url: &str,
    mode: &UpstreamMode,
    bearer_token: &str,
    chatgpt_account_id: Option<&str>,
    path: &str,
    query: Option<&str>,
    client_headers: &HeaderMap,
    forward_subprotocol: bool,
) -> anyhow::Result<TungsteniteRequest> {
    let upstream_ws_url = build_upstream_ws_url(base_url, mode, path, query)?;
    let mut upstream_request = upstream_ws_url
        .as_str()
        .into_client_request()
        .context("failed to build websocket request")?;
    apply_websocket_passthrough_headers(
        upstream_request.headers_mut(),
        client_headers,
        forward_subprotocol,
    );

    let authorization_header = HeaderValue::from_str(&format!("Bearer {bearer_token}"))
        .context("invalid upstream authorization header value")?;
    upstream_request
        .headers_mut()
        .insert(AUTHORIZATION, authorization_header);
    if let Some(chatgpt_account_id) = chatgpt_account_id {
        let chatgpt_account_header = HeaderValue::from_str(chatgpt_account_id)
            .context("invalid chatgpt account id header value")?;
        upstream_request.headers_mut().insert(
            HeaderName::from_static(CHATGPT_ACCOUNT_ID),
            chatgpt_account_header,
        );
    }

    Ok(upstream_request)
}

fn apply_passthrough_headers(
    mut request_builder: reqwest::RequestBuilder,
    headers: &HeaderMap,
    strip_content_encoding: bool,
) -> reqwest::RequestBuilder {
    for (name, value) in headers {
        if !is_compatibility_passthrough_header(name)
            && (is_hop_by_hop_header(name)
                || *name == HOST
                || *name == CONTENT_LENGTH
                || (strip_content_encoding && *name == axum::http::header::CONTENT_ENCODING)
                || *name == AUTHORIZATION
                || *name == HeaderName::from_static(CHATGPT_ACCOUNT_ID))
        {
            continue;
        }
        request_builder = request_builder.header(name, value);
    }

    request_builder
}

fn is_compatibility_passthrough_header(name: &HeaderName) -> bool {
    is_compatibility_passthrough_header_name(name.as_str())
}

fn is_compatibility_passthrough_header_name(name: &str) -> bool {
    matches!(
        name,
        OPENAI_BETA_HEADER
            | X_OPENAI_SUBAGENT_HEADER
            | SESSION_ID_HEADER
            | CONVERSATION_ID_HEADER
            | X_SESSION_ID_HEADER
    ) || name.starts_with(X_CODEX_HEADER_PREFIX)
}

fn apply_websocket_passthrough_headers(
    upstream_headers: &mut HeaderMap,
    client_headers: &HeaderMap,
    forward_subprotocol: bool,
) {
    for (name, value) in client_headers {
        if is_websocket_passthrough_header(name, forward_subprotocol) {
            upstream_headers.insert(name.clone(), value.clone());
        }
    }
}

fn is_websocket_passthrough_header(name: &HeaderName, forward_subprotocol: bool) -> bool {
    let name = name.as_str();
    is_compatibility_passthrough_header_name(name)
        || (forward_subprotocol && name == SEC_WEBSOCKET_PROTOCOL_HEADER)
}

fn is_hop_by_hop_header(name: &HeaderName) -> bool {
    matches!(
        name.as_str().to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}

async fn map_service_unavailable(
    headers: &HeaderMap,
    upstream_response: reqwest::Response,
) -> (Response, UpstreamErrorContext) {
    let body = upstream_response.bytes().await.unwrap_or_default();
    let context = build_upstream_error_context(StatusCode::SERVICE_UNAVAILABLE, headers, &body)
        .unwrap_or(UpstreamErrorContext {
            status: StatusCode::SERVICE_UNAVAILABLE,
            error_code: None,
            error_message: None,
            retry_after: extract_retry_after(headers),
            class: UpstreamErrorClass::UpstreamUnavailable,
        });
    (normalize_upstream_error_response(&context), context)
}

async fn buffered_response(
    status: StatusCode,
    headers: &HeaderMap,
    upstream_response: reqwest::Response,
) -> (Response, Option<UpstreamErrorContext>, bytes::Bytes) {
    let body = upstream_response.bytes().await.unwrap_or_default();
    let error_context = if status.as_u16() >= 400 {
        build_upstream_error_context(status, headers, &body)
    } else {
        None
    };
    let response = match error_context.as_ref() {
        Some(context) => normalize_upstream_error_response(context),
        None => response_with_bytes(status, headers, body.clone()),
    };
    (
        response,
        error_context,
        body,
    )
}

async fn buffered_codex_compat_response(
    status: StatusCode,
    headers: &HeaderMap,
    upstream_response: reqwest::Response,
) -> (Response, Option<UpstreamErrorContext>, bytes::Bytes) {
    let body = upstream_response.bytes().await.unwrap_or_default();
    if status.as_u16() >= 400 {
        let error_context = build_upstream_error_context(status, headers, &body);
        let response = match error_context.as_ref() {
            Some(context) => normalize_upstream_error_response(context),
            None => response_with_bytes(status, headers, body.clone()),
        };
        return (response, error_context, body);
    }

    if let Some(completed_response) = extract_codex_completed_response(&body) {
        if let Ok(response_body) = serde_json::to_vec(&completed_response) {
            let response_body = bytes::Bytes::from(response_body);
            let mut response_headers = headers.clone();
            response_headers.remove(CONTENT_LENGTH);
            response_headers.remove(axum::http::header::CONTENT_ENCODING);
            response_headers.insert(
                axum::http::header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            return (
                response_with_bytes(status, &response_headers, response_body.clone()),
                None,
                response_body,
            );
        }
    }

    let error_context = UpstreamErrorContext {
        status: StatusCode::BAD_GATEWAY,
        error_code: None,
        error_message: Some("codex upstream stream missing completion event".to_string()),
        retry_after: None,
        class: UpstreamErrorClass::TransientServer,
    };
    (
        normalize_upstream_error_response(&error_context),
        Some(error_context),
        body,
    )
}

fn extract_codex_completed_response(body: &[u8]) -> Option<Value> {
    if let Ok(value) = serde_json::from_slice::<Value>(body) {
        return Some(value);
    }

    let mut completed: Option<Value> = None;
    for raw_line in body.split(|byte| *byte == b'\n') {
        let line = trim_ascii(raw_line);
        if line.is_empty() {
            continue;
        }
        let payload = if let Some(raw_payload) = line.strip_prefix(b"data:") {
            trim_ascii(raw_payload)
        } else {
            line
        };
        if payload.is_empty() || payload == b"[DONE]" {
            continue;
        }
        let Ok(value) = serde_json::from_slice::<Value>(payload) else {
            continue;
        };
        if matches!(
            value.get("type").and_then(Value::as_str),
            Some("response.completed" | "response.done")
        ) {
            if let Some(response) = value.get("response") {
                completed = Some(response.clone());
            }
        }
    }
    completed
}

fn parse_request_policy_context(
    headers: &HeaderMap,
    body: &bytes::Bytes,
) -> ParsedRequestPolicyContext {
    let request_id = headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let Some(value) = parse_request_json_body(headers, body) else {
        return ParsedRequestPolicyContext {
            request_id,
            ..Default::default()
        };
    };
    let model = value
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let stream = value
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let estimated_input_tokens = estimate_request_input_tokens(&value);
    let sticky_key_hint = value
        .get("prompt_cache_key")
        .and_then(Value::as_str)
        .or_else(|| {
            value
                .get("metadata")
                .and_then(|meta| meta.get("session_id"))
                .and_then(Value::as_str)
        })
        .or_else(|| value.get("previous_response_id").and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let previous_response_id = value
        .get("previous_response_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    ParsedRequestPolicyContext {
        model,
        stream,
        request_id,
        estimated_input_tokens,
        continuation_key_hint: previous_response_id.clone(),
        sticky_key_hint,
        session_key_hint: sticky_session_key_from_headers(headers)
            .or_else(|| {
                value
                    .get("metadata")
                    .and_then(|meta| meta.get("session_id"))
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string)
            })
            .or(previous_response_id),
    }
}

const MAX_ZSTD_DECOMPRESSED_BODY_BYTES: usize = 64 * 1024 * 1024;

fn parse_request_json_body(headers: &HeaderMap, body: &bytes::Bytes) -> Option<Value> {
    if body.is_empty() {
        return None;
    }

    if has_zstd_content_encoding(headers) {
        if let Some(decoded) = decode_zstd_body_limited(body, MAX_ZSTD_DECOMPRESSED_BODY_BYTES) {
            if let Ok(value) = serde_json::from_slice::<Value>(&decoded) {
                return Some(value);
            }
        }
    }

    serde_json::from_slice::<Value>(body).ok()
}

fn has_zstd_content_encoding(headers: &HeaderMap) -> bool {
    headers
        .get_all(axum::http::header::CONTENT_ENCODING)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .any(|raw| {
            raw.split(',')
                .map(str::trim)
                .any(|entry| entry.eq_ignore_ascii_case("zstd"))
        })
}

fn decode_zstd_body_limited(body: &bytes::Bytes, max_bytes: usize) -> Option<bytes::Bytes> {
    use std::io::Read;

    let mut decoder = zstd::stream::read::Decoder::new(std::io::Cursor::new(body.as_ref())).ok()?;
    let mut output: Vec<u8> = Vec::new();
    let mut buf = vec![0u8; 16 * 1024];

    loop {
        let n = decoder.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        if output.len().saturating_add(n) > max_bytes {
            return None;
        }
        output.extend_from_slice(&buf[..n]);
    }

    Some(bytes::Bytes::from(output))
}

fn estimate_request_input_tokens(value: &Value) -> Option<i64> {
    let mut total_chars = 0usize;
    if let Some(instructions) = value.get("instructions").and_then(Value::as_str) {
        total_chars = total_chars.saturating_add(instructions.chars().count());
    }
    if let Some(input) = value.get("input") {
        total_chars = total_chars.saturating_add(collect_request_text_chars(input));
    }
    if let Some(messages) = value.get("messages") {
        total_chars = total_chars.saturating_add(collect_request_text_chars(messages));
    }
    if total_chars == 0 {
        None
    } else {
        Some(rough_token_estimate_from_char_count(total_chars))
    }
}

fn estimate_response_output_tokens(body: &bytes::Bytes) -> Option<i64> {
    let Ok(value) = serde_json::from_slice::<Value>(body) else {
        return None;
    };

    let mut total_chars = 0usize;
    if let Some(output_text) = value.get("output_text").and_then(Value::as_str) {
        total_chars = total_chars.saturating_add(output_text.chars().count());
    }
    if total_chars == 0 {
        if let Some(output) = value.get("output") {
            total_chars = total_chars.saturating_add(collect_request_text_chars(output));
        }
    }
    if total_chars == 0 {
        if let Some(choices) = value.get("choices") {
            total_chars = total_chars.saturating_add(collect_request_text_chars(choices));
        }
    }

    if total_chars == 0 {
        // Best-effort fallback for non-standard response payloads.
        total_chars = total_chars.saturating_add(collect_request_text_chars(&value));
    }

    if total_chars == 0 {
        return None;
    }
    Some(rough_token_estimate_from_char_count(total_chars))
}

fn collect_request_text_chars(value: &Value) -> usize {
    match value {
        Value::String(text) => text.chars().count(),
        Value::Array(items) => items
            .iter()
            .map(collect_request_text_chars)
            .sum::<usize>(),
        Value::Object(map) => {
            let mut total = 0usize;
            if let Some(text) = map.get("text").and_then(Value::as_str) {
                total = total.saturating_add(text.chars().count());
            }
            if let Some(text) = map.get("input_text").and_then(Value::as_str) {
                total = total.saturating_add(text.chars().count());
            }
            if let Some(content) = map.get("content") {
                total = total.saturating_add(collect_request_text_chars(content));
            }
            if let Some(parts) = map.get("parts") {
                total = total.saturating_add(collect_request_text_chars(parts));
            }
            total
        }
        _ => 0,
    }
}

fn rough_token_estimate_from_char_count(char_count: usize) -> i64 {
    if char_count == 0 {
        0
    } else {
        // ASCII-heavy payloads are commonly around 4 chars/token.
        ((char_count as i64).saturating_add(3)) / 4
    }
}

fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|raw| raw.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn enforce_principal_request_policy(
    principal: Option<&ApiPrincipal>,
    headers: &HeaderMap,
    context: &ParsedRequestPolicyContext,
) -> std::result::Result<(), Box<Response>> {
    let Some(principal) = principal else {
        return Ok(());
    };

    if principal.api_key_group_invalid {
        return Err(Box::new(json_error(
            StatusCode::FORBIDDEN,
            "api_key_group_invalid",
            "api key group is unavailable; update the api key group assignment",
        )));
    }

    if !principal
        .tenant_status
        .as_deref()
        .unwrap_or("active")
        .eq_ignore_ascii_case("active")
    {
        return Err(Box::new(json_error(
            StatusCode::FORBIDDEN,
            "tenant_inactive",
            "tenant is inactive",
        )));
    }
    if principal
        .tenant_expires_at
        .is_some_and(|expires_at| expires_at <= chrono::Utc::now())
    {
        return Err(Box::new(json_error(
            StatusCode::FORBIDDEN,
            "tenant_expired",
            "tenant subscription/plan is expired",
        )));
    }
    if principal
        .balance_microcredits
        .is_some_and(|balance| balance <= 0)
    {
        return Err(Box::new(json_error(
            StatusCode::PAYMENT_REQUIRED,
            "insufficient_credits",
            "tenant credits are insufficient",
        )));
    }

    if !principal.key_ip_allowlist.is_empty() {
        let Some(client_ip) = extract_client_ip(headers) else {
            return Err(Box::new(json_error(
                StatusCode::FORBIDDEN,
                "ip_not_allowed",
                "missing client IP for whitelist validation",
            )));
        };
        if !principal
            .key_ip_allowlist
            .iter()
            .any(|item| item == &client_ip)
        {
            return Err(Box::new(json_error(
                StatusCode::FORBIDDEN,
                "ip_not_allowed",
                "request IP is not in api key allowlist",
            )));
        }
    }

    if !principal.key_model_allowlist.is_empty() {
        let Some(model) = context.model.as_deref() else {
            return Err(Box::new(json_error(
                StatusCode::BAD_REQUEST,
                "model_required",
                "model is required for model allowlist validation",
            )));
        };
        if !principal
            .key_model_allowlist
            .iter()
            .any(|item| item.eq_ignore_ascii_case(model))
        {
            return Err(Box::new(json_error(
                StatusCode::FORBIDDEN,
                "model_not_allowed",
                "requested model is not in api key allowlist",
            )));
        }
    }

    Ok(())
}

fn extract_usage_tokens(body: &bytes::Bytes) -> Option<UsageTokens> {
    let value = serde_json::from_slice::<Value>(body).ok()?;
    extract_usage_tokens_from_value(&value)
}

fn extract_usage_tokens_from_value(value: &Value) -> Option<UsageTokens> {
    if let Some(tokens) = usage_tokens_from_usage_object(value) {
        return Some(tokens);
    }
    match value {
        Value::Object(map) => map.values().find_map(extract_usage_tokens_from_value),
        Value::Array(items) => items.iter().find_map(extract_usage_tokens_from_value),
        _ => None,
    }
}

fn usage_tokens_from_usage_object(value: &Value) -> Option<UsageTokens> {
    let usage = value.get("usage")?;
    let input_tokens = usage
        .get("input_tokens")
        .or_else(|| usage.get("prompt_tokens"))
        .and_then(Value::as_i64);
    let output_tokens = usage
        .get("output_tokens")
        .or_else(|| usage.get("completion_tokens"))
        .and_then(Value::as_i64);
    let reasoning_tokens = usage
        .get("output_tokens_details")
        .and_then(|details| details.get("reasoning_tokens"))
        .or_else(|| {
            usage.get("completion_tokens_details")
                .and_then(|details| details.get("reasoning_tokens"))
        })
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let normalized_input_tokens = input_tokens.unwrap_or(0).max(0);
    let normalized_output_tokens = output_tokens.unwrap_or(reasoning_tokens).max(0);
    let cached_input_tokens = usage
        .get("input_tokens_details")
        .and_then(|details| details.get("cached_tokens"))
        .or_else(|| {
            usage.get("prompt_tokens_details")
                .and_then(|details| details.get("cached_tokens"))
        })
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0)
        .min(normalized_input_tokens);
    if normalized_input_tokens == 0 && normalized_output_tokens == 0 && reasoning_tokens == 0 {
        return None;
    }
    Some(UsageTokens {
        input_tokens: normalized_input_tokens,
        cached_input_tokens,
        output_tokens: normalized_output_tokens,
        reasoning_tokens,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UsageTokens {
    input_tokens: i64,
    cached_input_tokens: i64,
    output_tokens: i64,
    reasoning_tokens: i64,
}

#[cfg(test)]
mod request_utils_tests {
    use super::{build_upstream_error_context, classify_upstream_error, UpstreamErrorClass};
    use http::{HeaderMap, StatusCode};

    #[test]
    fn classifies_unknown_403_as_non_retryable_client() {
        let class = classify_upstream_error(
            StatusCode::FORBIDDEN,
            Some("model_not_found"),
            Some("model does not exist"),
        );
        assert_eq!(class, UpstreamErrorClass::NonRetryableClient);
    }

    #[test]
    fn classifies_known_auth_refresh_403_as_auth_expired() {
        let class = classify_upstream_error(
            StatusCode::FORBIDDEN,
            None,
            Some("Your access token could not be refreshed because you have since logged out."),
        );
        assert_eq!(class, UpstreamErrorClass::AuthExpired);
    }

    #[test]
    fn classifies_accountid_extraction_failure_as_auth_expired() {
        let class = classify_upstream_error(
            StatusCode::BAD_REQUEST,
            None,
            Some("Failed to extract accountId from token"),
        );
        assert_eq!(class, UpstreamErrorClass::AuthExpired);
    }

    #[test]
    fn builds_error_context_from_plain_text_accountid_failure() {
        let headers = HeaderMap::new();
        let context = build_upstream_error_context(
            StatusCode::BAD_REQUEST,
            &headers,
            b"Failed to extract accountId from token",
        )
        .expect("plain text error body should build context");
        assert_eq!(context.class, UpstreamErrorClass::AuthExpired);
        assert_eq!(context.status, StatusCode::BAD_REQUEST);
        assert_eq!(
            context.error_message.as_deref(),
            Some("Failed to extract accountId from token")
        );
    }
}
