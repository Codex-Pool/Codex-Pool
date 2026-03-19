use anyhow::anyhow;
use codex_pool_core::api::{
    ResolveUpstreamErrorTemplateRequest, ResolveUpstreamErrorTemplateResponse,
};
use codex_pool_core::model::{
    BuiltinErrorTemplateKind, UpstreamErrorAction, UpstreamErrorRetryScope,
    UpstreamErrorTemplateRecord,
};
use regex::Regex;
use std::sync::LazyLock;

const DEFAULT_ERROR_LOCALE: &str = "en";
const SANITIZED_UPSTREAM_RAW_MAX_CHARS: usize = 2_048;
const SANITIZED_UPSTREAM_RAW_MAX_DEPTH: usize = 6;
const SANITIZED_UPSTREAM_RAW_MAX_ITEMS: usize = 12;
static EMAIL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b[a-z0-9._%+\-]+@[a-z0-9.\-]+\.[a-z]{2,}\b").unwrap());
static UUID_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\b")
        .unwrap()
});
static REQUEST_ID_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:req|resp|msg|conv|chatcmpl|cmpl|file)_[a-z0-9_\-]+\b").unwrap()
});
static MODEL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\b(?:gpt-[a-z0-9.\-]+|o\d[a-z0-9.\-]*|codex[a-z0-9.\-]+|claude-[a-z0-9.\-]+|gemini-[a-z0-9.\-]+|deepseek-[a-z0-9.\-]+)\b",
    )
    .unwrap()
});
static NUMBER_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b\d{4,}\b").unwrap());
static WHITESPACE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

#[derive(Debug, Clone, PartialEq, Eq)]
struct LearnedTemplateResolution {
    semantic_error_code: String,
    localized_message: String,
    action: UpstreamErrorAction,
    retry_scope: UpstreamErrorRetryScope,
    fingerprint: String,
}

fn detect_request_locale(_headers: &axum::http::HeaderMap, _body: &bytes::Bytes) -> String {
    if let Some(locale) = detect_locale_from_headers(_headers) {
        return locale;
    }
    if let Some(locale) = detect_locale_from_request_body(_body) {
        return locale;
    }
    DEFAULT_ERROR_LOCALE.to_string()
}

fn sanitize_upstream_error_raw(raw: Option<&str>, model: Option<&str>) -> Option<String> {
    let raw = raw.map(str::trim).filter(|value| !value.is_empty())?;
    let sanitized = if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) {
        serde_json::to_string(&sanitize_upstream_error_json_value(&value, None, 0, model))
            .unwrap_or_else(|_| sanitize_upstream_error_text(raw, model))
    } else {
        sanitize_upstream_error_text(raw, model)
    };
    Some(truncate_upstream_error_text(&sanitized))
}

fn sanitize_upstream_error_json_value(
    value: &serde_json::Value,
    key: Option<&str>,
    depth: usize,
    model: Option<&str>,
) -> serde_json::Value {
    if key.is_some_and(is_sensitive_upstream_error_key) {
        return serde_json::Value::String("[redacted]".to_string());
    }
    if depth >= SANITIZED_UPSTREAM_RAW_MAX_DEPTH {
        return serde_json::Value::String("[truncated]".to_string());
    }

    match value {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().cloned().collect();
            keys.sort();
            let mut sanitized = serde_json::Map::new();
            for key in keys.into_iter().take(SANITIZED_UPSTREAM_RAW_MAX_ITEMS) {
                if let Some(child) = map.get(&key) {
                    sanitized.insert(
                        key.clone(),
                        sanitize_upstream_error_json_value(
                            child,
                            Some(key.as_str()),
                            depth + 1,
                            model,
                        ),
                    );
                }
            }
            if map.len() > SANITIZED_UPSTREAM_RAW_MAX_ITEMS {
                sanitized.insert("_truncated".to_string(), serde_json::Value::Bool(true));
            }
            serde_json::Value::Object(sanitized)
        }
        serde_json::Value::Array(items) => {
            let mut sanitized = Vec::new();
            for item in items.iter().take(SANITIZED_UPSTREAM_RAW_MAX_ITEMS) {
                sanitized.push(sanitize_upstream_error_json_value(
                    item,
                    None,
                    depth + 1,
                    model,
                ));
            }
            if items.len() > SANITIZED_UPSTREAM_RAW_MAX_ITEMS {
                sanitized.push(serde_json::Value::String("[truncated]".to_string()));
            }
            serde_json::Value::Array(sanitized)
        }
        serde_json::Value::String(text) => {
            serde_json::Value::String(sanitize_upstream_error_text(text, model))
        }
        _ => value.clone(),
    }
}

fn is_sensitive_upstream_error_key(key: &str) -> bool {
    matches!(
        key.trim().to_ascii_lowercase().as_str(),
        "input"
            | "prompt"
            | "messages"
            | "instructions"
            | "content"
            | "text"
            | "arguments"
            | "args"
            | "user_prompt"
            | "conversation"
            | "history"
            | "tool_input"
            | "tool_output"
            | "tool_calls"
            | "response"
            | "completion"
            | "raw_input"
            | "raw_prompt"
            | "file"
            | "files"
            | "attachment"
            | "attachments"
    )
}

fn sanitize_upstream_error_text(text: &str, model: Option<&str>) -> String {
    let mut sanitized = text.trim().to_string();
    if let Some(model) = model.map(str::trim).filter(|value| !value.is_empty()) {
        sanitized = sanitized.replace(model, "{model}");
    }
    sanitized = EMAIL_RE.replace_all(&sanitized, "{email}").into_owned();
    sanitized = UUID_RE.replace_all(&sanitized, "{id}").into_owned();
    sanitized = REQUEST_ID_RE.replace_all(&sanitized, "{id}").into_owned();
    sanitized = MODEL_RE.replace_all(&sanitized, "{model}").into_owned();
    sanitized = NUMBER_RE.replace_all(&sanitized, "{number}").into_owned();
    WHITESPACE_RE
        .replace_all(sanitized.trim(), " ")
        .into_owned()
}

fn truncate_upstream_error_text(value: &str) -> String {
    let mut chars = value.chars();
    let truncated: String = chars
        .by_ref()
        .take(SANITIZED_UPSTREAM_RAW_MAX_CHARS)
        .collect();
    if chars.next().is_some() {
        format!("{truncated}...[truncated]")
    } else {
        truncated
    }
}

fn normalize_upstream_error_fingerprint(
    provider: &str,
    normalized_status_code: u16,
    normalized_upstream_message: &str,
    model: Option<&str>,
) -> String {
    let mut normalized = normalized_upstream_message.trim().to_ascii_lowercase();
    if let Some(model) = model
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
    {
        normalized = normalized.replace(&model, "{model}");
    }
    normalized = EMAIL_RE.replace_all(&normalized, "{email}").into_owned();
    normalized = UUID_RE.replace_all(&normalized, "{id}").into_owned();
    normalized = REQUEST_ID_RE.replace_all(&normalized, "{id}").into_owned();
    normalized = MODEL_RE.replace_all(&normalized, "{model}").into_owned();
    normalized = NUMBER_RE.replace_all(&normalized, "{number}").into_owned();
    normalized = WHITESPACE_RE
        .replace_all(normalized.trim(), " ")
        .into_owned();
    format!(
        "{}:{}:{}",
        provider.trim().to_ascii_lowercase(),
        normalized_status_code,
        normalized
    )
}

async fn resolve_template_via_control_plane(
    http_client: &reqwest::Client,
    control_plane_base_url: &str,
    internal_auth_token: &str,
    timeout_ms: u64,
    payload: &ResolveUpstreamErrorTemplateRequest,
) -> anyhow::Result<LearnedTemplateResolution> {
    let target_locale = payload.target_locale.clone();
    let endpoint = format!(
        "{}/internal/v1/upstream-errors/resolve",
        control_plane_base_url.trim_end_matches('/')
    );
    let response = http_client
        .post(endpoint)
        .bearer_auth(internal_auth_token)
        .timeout(std::time::Duration::from_millis(
            timeout_ms.clamp(100, 10_000),
        ))
        .json(payload)
        .send()
        .await
        .context("failed to request internal upstream error resolve")?;
    let response = response
        .error_for_status()
        .context("internal upstream error resolve returned error status")?;
    let payload: ResolveUpstreamErrorTemplateResponse = response
        .json()
        .await
        .context("failed to decode internal upstream error resolve response")?;
    let localized_message = localized_message_from_template(&payload.template, &target_locale)
        .or_else(|| localized_message_from_template(&payload.template, DEFAULT_ERROR_LOCALE))
        .ok_or_else(|| anyhow!("resolved template did not contain any localized message"))?;

    Ok(LearnedTemplateResolution {
        semantic_error_code: payload.template.semantic_error_code.clone(),
        localized_message,
        action: payload.template.action,
        retry_scope: payload.template.retry_scope,
        fingerprint: payload.template.fingerprint.clone(),
    })
}

async fn resolve_upstream_error_learning(
    state: &AppState,
    provider: &str,
    error_context: &UpstreamErrorContext,
    locale: &str,
    model: Option<&str>,
) -> Option<LearnedTemplateResolution> {
    let sanitized_upstream_raw =
        sanitize_upstream_error_raw(error_context.raw_error.as_deref(), model);
    let normalized_message = error_context
        .error_message
        .as_deref()
        .or(error_context.error_code.as_deref())
        .or(sanitized_upstream_raw.as_deref())
        .unwrap_or("unknown upstream error");
    let fingerprint = normalize_upstream_error_fingerprint(
        provider,
        error_context.status.as_u16(),
        normalized_message,
        model,
    );

    if let Some(template) = state
        .approved_upstream_error_templates
        .read()
        .ok()
        .and_then(|templates| templates.get(&fingerprint).cloned())
    {
        let localized_message = localized_message_from_template(&template, locale)
            .or_else(|| localized_message_from_template(&template, DEFAULT_ERROR_LOCALE))?;
        return Some(LearnedTemplateResolution {
            semantic_error_code: template.semantic_error_code.clone(),
            localized_message,
            action: template.action,
            retry_scope: template.retry_scope,
            fingerprint,
        });
    }

    let settings = state.ai_error_learning_settings.read().ok()?.clone();
    if !settings.enabled {
        return None;
    }
    if !should_attempt_ai_error_learning(error_context) {
        return None;
    }
    let control_plane_base_url = state.control_plane_base_url.as_deref()?;
    let payload = ResolveUpstreamErrorTemplateRequest {
        fingerprint,
        provider: provider.trim().to_string(),
        normalized_status_code: error_context.status.as_u16(),
        normalized_upstream_message: normalized_message.to_string(),
        sanitized_upstream_raw,
        target_locale: canonicalize_locale(locale)
            .unwrap_or_else(|| DEFAULT_ERROR_LOCALE.to_string()),
        model: model.map(ToString::to_string),
    };

    resolve_template_via_control_plane(
        &state.http_client,
        control_plane_base_url,
        state.control_plane_internal_auth_token.as_ref(),
        settings.first_seen_timeout_ms,
        &payload,
    )
    .await
    .ok()
}

fn should_attempt_ai_error_learning(error_context: &UpstreamErrorContext) -> bool {
    matches!(error_context.class, UpstreamErrorClass::Unknown)
        || (matches!(error_context.class, UpstreamErrorClass::NonRetryableClient)
            && error_context.error_code.is_none()
            && error_context
                .raw_error
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty()))
}

async fn apply_upstream_error_learning(
    state: &Arc<AppState>,
    source: UpstreamErrorSource,
    provider: &str,
    locale: &str,
    model: Option<&str>,
    mut error_context: UpstreamErrorContext,
) -> (Response, UpstreamErrorContext) {
    error_context.learned_resolution =
        resolve_upstream_error_learning(state.as_ref(), provider, &error_context, locale, model)
            .await;
    let response = normalize_upstream_error_response(source, &error_context);
    (response, error_context)
}

fn upstream_provider_label(_mode: &UpstreamMode) -> &'static str {
    "openai_compatible"
}

fn builtin_error_template_key(kind: BuiltinErrorTemplateKind, code: &str) -> String {
    let kind = match kind {
        BuiltinErrorTemplateKind::GatewayError => "gateway_error",
        BuiltinErrorTemplateKind::HeuristicUpstream => "heuristic_upstream",
    };
    format!("{kind}:{code}")
}

fn localized_message_from_builtin_template(
    state: &AppState,
    code: &str,
    locale: &str,
) -> Option<String> {
    let key = builtin_error_template_key(BuiltinErrorTemplateKind::GatewayError, code);
    let templates = state.builtin_error_templates.read().ok()?;
    let template = templates.get(&key)?;
    localized_message_from_template_like(template, locale)
}

fn localized_message_from_template_like(
    template: &impl BuiltinLikeTemplate,
    locale: &str,
) -> Option<String> {
    match canonicalize_locale(locale)
        .as_deref()
        .unwrap_or(DEFAULT_ERROR_LOCALE)
    {
        "zh-CN" => template.templates().zh_cn.clone(),
        "zh-TW" => template.templates().zh_tw.clone(),
        "ja" => template.templates().ja.clone(),
        "ru" => template.templates().ru.clone(),
        _ => template.templates().en.clone(),
    }
}

trait BuiltinLikeTemplate {
    fn templates(&self) -> &codex_pool_core::model::LocalizedErrorTemplates;
}

impl BuiltinLikeTemplate for codex_pool_core::model::BuiltinErrorTemplateRecord {
    fn templates(&self) -> &codex_pool_core::model::LocalizedErrorTemplates {
        &self.templates
    }
}

fn localized_gateway_message(code: &str, locale: &str, fallback: &str) -> String {
    let locale = canonicalize_locale(locale).unwrap_or_else(|| DEFAULT_ERROR_LOCALE.to_string());
    match locale.as_str() {
        "zh-CN" => match code {
            "payload_too_large" => "请求体超过了服务端限制。".to_string(),
            "invalid_request_body" => "请求体解析失败。".to_string(),
            "no_upstream_account" => "当前没有可用的上游账号。".to_string(),
            "invalid_upstream_url" => "上游地址无效。".to_string(),
            "upstream_transport_error" => "上游请求失败。".to_string(),
            "proxy_unavailable" => "当前配置的出口代理不可用。".to_string(),
            "invalid_websocket_upgrade" => "无效的 WebSocket 升级请求。".to_string(),
            "upstream_websocket_connect_error" => "连接上游 WebSocket 失败。".to_string(),
            "websocket_upgrade_required" => "上游要求使用 WebSocket 协议升级。".to_string(),
            "websocket_handshake_error" => "上游 WebSocket 握手失败。".to_string(),
            "invalid_request_rate_limited" => "无效请求过多，请稍后再试。".to_string(),
            _ => fallback.to_string(),
        },
        "zh-TW" => match code {
            "payload_too_large" => "請求體超過了伺服器限制。".to_string(),
            "invalid_request_body" => "請求體解析失敗。".to_string(),
            "no_upstream_account" => "目前沒有可用的上游帳號。".to_string(),
            "invalid_upstream_url" => "上游位址無效。".to_string(),
            "upstream_transport_error" => "上游請求失敗。".to_string(),
            "proxy_unavailable" => "目前設定的出口代理不可用。".to_string(),
            "invalid_websocket_upgrade" => "無效的 WebSocket 升級請求。".to_string(),
            "upstream_websocket_connect_error" => "連接上游 WebSocket 失敗。".to_string(),
            "websocket_upgrade_required" => "上游要求使用 WebSocket 協議升級。".to_string(),
            "websocket_handshake_error" => "上游 WebSocket 握手失敗。".to_string(),
            "invalid_request_rate_limited" => "無效請求過多，請稍後再試。".to_string(),
            _ => fallback.to_string(),
        },
        "ja" => match code {
            "payload_too_large" => "リクエスト本文がサーバー制限を超えています。".to_string(),
            "invalid_request_body" => "リクエスト本文を解析できませんでした。".to_string(),
            "no_upstream_account" => "利用可能な上流アカウントがありません。".to_string(),
            "invalid_upstream_url" => "上流 URL が無効です。".to_string(),
            "upstream_transport_error" => "上流リクエストに失敗しました。".to_string(),
            "proxy_unavailable" => {
                "設定されたアウトバウンドプロキシは現在利用できません。".to_string()
            }
            "invalid_websocket_upgrade" => "無効な WebSocket アップグレード要求です。".to_string(),
            "upstream_websocket_connect_error" => {
                "上流 WebSocket への接続に失敗しました。".to_string()
            }
            "websocket_upgrade_required" => {
                "上流は WebSocket アップグレードを要求しています。".to_string()
            }
            "websocket_handshake_error" => {
                "上流 WebSocket ハンドシェイクに失敗しました。".to_string()
            }
            "invalid_request_rate_limited" => {
                "無効なリクエストが多すぎます。しばらくしてから再試行してください。".to_string()
            }
            _ => fallback.to_string(),
        },
        "ru" => match code {
            "payload_too_large" => "Тело запроса превышает лимит сервера.".to_string(),
            "invalid_request_body" => "Не удалось разобрать тело запроса.".to_string(),
            "no_upstream_account" => "Сейчас нет доступных upstream-аккаунтов.".to_string(),
            "invalid_upstream_url" => "Некорректный upstream URL.".to_string(),
            "upstream_transport_error" => "Ошибка запроса к upstream.".to_string(),
            "proxy_unavailable" => "Настроенный исходящий прокси сейчас недоступен.".to_string(),
            "invalid_websocket_upgrade" => {
                "Некорректный запрос на обновление WebSocket.".to_string()
            }
            "upstream_websocket_connect_error" => {
                "Не удалось подключиться к upstream WebSocket.".to_string()
            }
            "websocket_upgrade_required" => "Upstream требует обновления до WebSocket.".to_string(),
            "websocket_handshake_error" => "Ошибка рукопожатия upstream WebSocket.".to_string(),
            "invalid_request_rate_limited" => {
                "Слишком много некорректных запросов, попробуйте позже.".to_string()
            }
            _ => fallback.to_string(),
        },
        _ => fallback.to_string(),
    }
}

fn localized_gateway_message_with_state(
    state: &AppState,
    code: &str,
    locale: &str,
    fallback: &str,
) -> String {
    localized_message_from_builtin_template(state, code, locale)
        .unwrap_or_else(|| localized_gateway_message(code, locale, fallback))
}

pub(crate) fn localized_json_error_with_state(
    state: &AppState,
    locale: &str,
    status: StatusCode,
    code: &str,
    fallback: &str,
) -> Response {
    let message = localized_gateway_message_with_state(state, code, locale, fallback);
    json_error(status, code, &message)
}

fn localized_message_from_template(
    template: &UpstreamErrorTemplateRecord,
    locale: &str,
) -> Option<String> {
    match canonicalize_locale(locale)
        .as_deref()
        .unwrap_or(DEFAULT_ERROR_LOCALE)
    {
        "zh-CN" => template.templates.zh_cn.clone(),
        "zh-TW" => template.templates.zh_tw.clone(),
        "ja" => template.templates.ja.clone(),
        "ru" => template.templates.ru.clone(),
        _ => template.templates.en.clone(),
    }
}

fn detect_locale_from_headers(headers: &HeaderMap) -> Option<String> {
    for header_name in ["x-codex-locale", "x-user-locale", "content-language"] {
        let locale = headers
            .get(header_name)
            .and_then(|value| value.to_str().ok())
            .and_then(canonicalize_locale);
        if locale.is_some() {
            return locale;
        }
    }
    headers
        .get(axum::http::header::ACCEPT_LANGUAGE)
        .and_then(|value| value.to_str().ok())
        .and_then(locale_from_accept_language)
}

fn locale_from_accept_language(raw: &str) -> Option<String> {
    raw.split(',')
        .filter_map(|entry| entry.split(';').next())
        .find_map(canonicalize_locale)
}

fn detect_locale_from_request_body(body: &bytes::Bytes) -> Option<String> {
    let value = parse_request_json_body(&HeaderMap::new(), body)?;
    let mut sample = String::new();
    collect_request_text_sample(&value, &mut sample, 512);
    detect_locale_from_text(&sample)
}

fn collect_request_text_sample(value: &serde_json::Value, output: &mut String, max_chars: usize) {
    if output.chars().count() >= max_chars {
        return;
    }
    match value {
        serde_json::Value::String(text) => {
            let remaining = max_chars.saturating_sub(output.chars().count());
            output.extend(text.chars().take(remaining));
            output.push(' ');
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_request_text_sample(item, output, max_chars);
                if output.chars().count() >= max_chars {
                    break;
                }
            }
        }
        serde_json::Value::Object(map) => {
            for key in [
                "instructions",
                "input",
                "text",
                "content",
                "messages",
                "prompt",
            ] {
                if let Some(item) = map.get(key) {
                    collect_request_text_sample(item, output, max_chars);
                    if output.chars().count() >= max_chars {
                        break;
                    }
                }
            }
        }
        _ => {}
    }
}

fn detect_locale_from_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed
        .chars()
        .any(|ch| ('\u{3040}'..='\u{30ff}').contains(&ch))
    {
        return Some("ja".to_string());
    }
    if trimmed
        .chars()
        .any(|ch| ('\u{0400}'..='\u{04ff}').contains(&ch))
    {
        return Some("ru".to_string());
    }
    if trimmed
        .chars()
        .any(|ch| ('\u{4e00}'..='\u{9fff}').contains(&ch))
    {
        return Some("zh-CN".to_string());
    }
    None
}

fn canonicalize_locale(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let normalized = raw.replace('_', "-").to_ascii_lowercase();
    if normalized.starts_with("zh-tw") || normalized.starts_with("zh-hant") {
        return Some("zh-TW".to_string());
    }
    if normalized.starts_with("zh") {
        return Some("zh-CN".to_string());
    }
    if normalized.starts_with("ja") {
        return Some("ja".to_string());
    }
    if normalized.starts_with("ru") {
        return Some("ru".to_string());
    }
    if normalized.starts_with("en") {
        return Some("en".to_string());
    }
    None
}

#[cfg(test)]
mod ai_error_learning_tests {
    use super::*;
    use crate::event::NoopEventSink;
    use crate::router::RoundRobinRouter;
    use crate::routing_cache::InMemoryRoutingCache;
    use codex_pool_core::model::{
        AiErrorLearningSettings, BuiltinErrorTemplateKind, BuiltinErrorTemplateRecord,
        LocalizedErrorTemplates, RoutingStrategy, UpstreamMode,
    };
    use serde_json::json;
    use std::collections::{HashMap, HashSet};
    use std::sync::atomic::AtomicU64;
    use std::sync::Arc;
    use std::time::Duration;
    use uuid::Uuid;
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_state() -> Arc<AppState> {
        test_state_with_control_plane_base_url(Some("http://127.0.0.1:8090".to_string()))
    }

    fn test_state_with_control_plane_base_url(
        control_plane_base_url: Option<String>,
    ) -> Arc<AppState> {
        let account = UpstreamAccount {
            id: Uuid::new_v4(),
            label: "acc-1".to_string(),
            mode: UpstreamMode::CodexOauth,
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            bearer_token: "upstream-token".to_string(),
            chatgpt_account_id: Some("acct_123".to_string()),
            enabled: true,
            priority: 100,
            created_at: chrono::Utc::now(),
        };
        Arc::new(AppState {
            router: RoundRobinRouter::new(vec![account]),
            http_client: reqwest::Client::new(),
            outbound_proxy_runtime: Arc::new(
                crate::outbound_proxy_runtime::OutboundProxyRuntime::new(),
            ),
            control_plane_base_url,
            routing_strategy: RoutingStrategy::RoundRobin,
            account_ejection_ttl: Duration::from_secs(30),
            enable_request_failover: true,
            same_account_quick_retry_max: 1,
            request_failover_wait: Duration::from_millis(2_000),
            retry_poll_interval: Duration::from_millis(100),
            sticky_prefer_non_conflicting: true,
            shared_routing_cache_enabled: true,
            enable_metered_stream_billing: true,
            billing_authorize_required_for_stream: true,
            stream_billing_reserve_microcredits: 2_000_000,
            billing_dynamic_preauth_enabled: true,
            billing_preauth_expected_output_tokens: 256,
            billing_preauth_safety_factor: 1.3,
            billing_preauth_min_microcredits: 1_000,
            billing_preauth_max_microcredits: 1_000_000_000_000,
            billing_preauth_unit_price_microcredits: 10_000,
            stream_billing_drain_timeout: Duration::from_millis(5_000),
            billing_capture_retry_max: 3,
            billing_capture_retry_backoff: Duration::from_millis(200),
            billing_pricing_cache: std::sync::RwLock::new(HashMap::new()),
            models_cache: std::sync::RwLock::new(std::collections::HashMap::new()),
            routing_cache: Arc::new(InMemoryRoutingCache::new()),
            alive_ring_router: None,
            seen_ok_reporter: None,
            event_sink: Arc::new(NoopEventSink),
            auth_validator: None,
            control_plane_internal_auth_token: Arc::<str>::from("cp-internal-test-token"),
            auth_fail_open: false,
            allowed_api_keys: HashSet::new(),
            snapshot_revision: AtomicU64::new(0),
            snapshot_cursor: AtomicU64::new(0),
            snapshot_remote_cursor: AtomicU64::new(0),
            snapshot_events_apply_total: AtomicU64::new(0),
            snapshot_events_cursor_gone_total: AtomicU64::new(0),
            route_update_notify: Arc::new(tokio::sync::Notify::new()),
            ai_error_learning_settings: std::sync::RwLock::new(AiErrorLearningSettings {
                enabled: true,
                first_seen_timeout_ms: 2_000,
                review_hit_threshold: 10,
                updated_at: None,
            }),
            approved_upstream_error_templates: std::sync::RwLock::new(HashMap::new()),
            builtin_error_templates: std::sync::RwLock::new(HashMap::new()),
            max_request_body_bytes: 10 * 1024 * 1024,
            failover_attempt_total: AtomicU64::new(0),
            failover_success_total: AtomicU64::new(0),
            failover_exhausted_total: AtomicU64::new(0),
            same_account_retry_total: AtomicU64::new(0),
            billing_authorize_total: AtomicU64::new(0),
            billing_authorize_failed_total: AtomicU64::new(0),
            billing_capture_total: AtomicU64::new(0),
            billing_capture_failed_total: AtomicU64::new(0),
            billing_release_total: AtomicU64::new(0),
            billing_idempotent_hit_total: AtomicU64::new(0),
            billing_preauth_dynamic_total: AtomicU64::new(0),
            billing_preauth_fallback_total: AtomicU64::new(0),
            billing_preauth_amount_microcredits_sum: AtomicU64::new(0),
            billing_preauth_error_ratio_ppm_sum_total: AtomicU64::new(0),
            billing_preauth_error_ratio_count_total: AtomicU64::new(0),
            billing_preauth_capture_missing_total: AtomicU64::new(0),
            billing_settle_complete_total: AtomicU64::new(0),
            billing_release_without_capture_total: AtomicU64::new(0),
            billing_preauth_error_ratio_recent_ppm: std::sync::RwLock::new(
                std::collections::VecDeque::new(),
            ),
            billing_preauth_error_ratio_by_model_ppm: std::sync::RwLock::new(HashMap::new()),
            stream_usage_missing_total: AtomicU64::new(0),
            stream_usage_estimated_total: AtomicU64::new(0),
            stream_drain_timeout_total: AtomicU64::new(0),
            stream_response_total: AtomicU64::new(0),
            stream_protocol_sse_header_total: AtomicU64::new(0),
            stream_protocol_header_missing_total: AtomicU64::new(0),
            stream_usage_json_line_fallback_total: AtomicU64::new(0),
            invalid_request_guard_enabled: true,
            invalid_request_guard_window: Duration::from_secs(30),
            invalid_request_guard_threshold: 12,
            invalid_request_guard_block_ttl: Duration::from_secs(120),
            invalid_request_guard: std::sync::RwLock::new(HashMap::new()),
            invalid_request_guard_block_total: AtomicU64::new(0),
        })
    }

    #[tokio::test]
    async fn localized_json_error_with_state_prefers_builtin_gateway_templates() {
        let state = test_state();
        state.builtin_error_templates.write().unwrap().insert(
            builtin_error_template_key(
                BuiltinErrorTemplateKind::GatewayError,
                "no_upstream_account",
            ),
            BuiltinErrorTemplateRecord {
                kind: BuiltinErrorTemplateKind::GatewayError,
                code: "no_upstream_account".to_string(),
                templates: LocalizedErrorTemplates {
                    en: Some("No upstream accounts are ready right now.".to_string()),
                    zh_cn: Some("当前没有就绪的上游账号。".to_string()),
                    ..LocalizedErrorTemplates::default()
                },
                default_templates: LocalizedErrorTemplates {
                    en: Some("No upstream accounts are currently available.".to_string()),
                    zh_cn: Some("当前没有可用的上游账号。".to_string()),
                    ..LocalizedErrorTemplates::default()
                },
                action: None,
                retry_scope: None,
                is_overridden: true,
                updated_at: Some(chrono::Utc::now()),
            },
        );

        let response = localized_json_error_with_state(
            state.as_ref(),
            "zh-CN",
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "no_upstream_account",
            "no active upstream accounts",
        );
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["error"]["code"], "no_upstream_account");
        assert_eq!(payload["error"]["message"], "当前没有就绪的上游账号。");
    }

    #[test]
    fn ai_error_learning_detects_locale_from_accept_language_before_prompt_heuristic() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            "accept-language",
            "zh-CN,zh;q=0.9,en;q=0.8".parse().unwrap(),
        );
        let body = bytes::Bytes::from_static(br#"{"input":"hello world","model":"gpt-5.4"}"#);

        let locale = detect_request_locale(&headers, &body);

        assert_eq!(locale, "zh-CN");
    }

    #[test]
    fn ai_error_learning_normalizes_unknown_error_fingerprint_without_leaking_model_or_ids() {
        let fingerprint = normalize_upstream_error_fingerprint(
            "openai_compatible",
            axum::http::StatusCode::BAD_REQUEST.as_u16(),
            "Model gpt-5.4 does not exist for request req_123456 and user abc@example.com",
            Some("gpt-5.4"),
        );

        assert_eq!(
            fingerprint,
            "openai_compatible:400:model {model} does not exist for request {id} and user {email}"
        );
    }

    #[test]
    fn ai_error_learning_sanitizes_raw_json_before_forwarding_to_ai() {
        let sanitized = sanitize_upstream_error_raw(
            Some(
                r#"{"detail":"The 'gpt-5.4-ai-error-e2e-invalid' model is not supported for request req_123456 and user abc@example.com.","input":"请把这段提示词原样返回","messages":[{"role":"user","content":"sensitive content"}]}"#,
            ),
            Some("gpt-5.4-ai-error-e2e-invalid"),
        )
        .expect("sanitized raw should exist");

        assert!(sanitized.contains(
            r#""detail":"The '{model}' model is not supported for request {id} and user {email}.""#
        ));
        assert!(sanitized.contains(r#""input":"[redacted]""#));
        assert!(sanitized.contains(r#""messages":"[redacted]""#));
        assert!(!sanitized.contains("请把这段提示词原样返回"));
        assert!(!sanitized.contains("sensitive content"));
    }

    #[tokio::test]
    async fn ai_error_learning_resolves_template_via_control_plane_internal_api() {
        let control_plane = MockServer::start().await;
        let payload = ResolveUpstreamErrorTemplateRequest {
            fingerprint: "openai_compatible:400:unsupported_model".to_string(),
            provider: "openai_compatible".to_string(),
            normalized_status_code: 400,
            normalized_upstream_message: "The requested model does not exist".to_string(),
            sanitized_upstream_raw: Some(
                r#"{"detail":"The requested model {model} does not exist"}"#.to_string(),
            ),
            target_locale: "zh-CN".to_string(),
            model: Some("gpt-5.4".to_string()),
        };
        let template_id = Uuid::new_v4();
        Mock::given(method("POST"))
            .and(path("/internal/v1/upstream-errors/resolve"))
            .and(header("authorization", "Bearer cp-internal-test-token"))
            .and(body_json(json!(payload)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "created": true,
                "template": {
                    "id": template_id,
                    "fingerprint": payload.fingerprint,
                    "provider": payload.provider,
                    "normalized_status_code": payload.normalized_status_code,
                    "semantic_error_code": "unsupported_model",
                    "action": "return_failure",
                    "retry_scope": "none",
                    "status": "provisional_live",
                    "templates": {
                        "en": "The requested model is not available.",
                        "zh-CN": "请求的模型当前不可用。"
                    },
                    "representative_samples": ["The requested model does not exist"],
                    "hit_count": 1,
                    "first_seen_at": "2026-03-12T00:00:00Z",
                    "last_seen_at": "2026-03-12T00:00:00Z",
                    "updated_at": "2026-03-12T00:00:00Z"
                }
            })))
            .mount(&control_plane)
            .await;

        let result = resolve_template_via_control_plane(
            &reqwest::Client::new(),
            &control_plane.uri(),
            "cp-internal-test-token",
            2_000,
            &payload,
        )
        .await
        .expect("resolve should succeed");

        assert_eq!(result.semantic_error_code, "unsupported_model");
        assert_eq!(result.localized_message, "请求的模型当前不可用。");
        assert_eq!(result.action, UpstreamErrorAction::ReturnFailure);
        assert_eq!(result.retry_scope, UpstreamErrorRetryScope::None);
        assert_eq!(
            result.fingerprint,
            "openai_compatible:400:unsupported_model"
        );
    }

    #[tokio::test]
    async fn ai_error_learning_uses_sanitized_raw_for_non_retryable_client_resolve() {
        let control_plane = MockServer::start().await;
        let state = test_state_with_control_plane_base_url(Some(control_plane.uri()));
        let sanitized_upstream_raw = r#"{"detail":"The '{model}' model is not supported when using Codex with a ChatGPT account.","input":"[redacted]"}"#;
        Mock::given(method("POST"))
            .and(path("/internal/v1/upstream-errors/resolve"))
            .and(header("authorization", "Bearer cp-internal-test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "created": true,
                "template": {
                    "id": Uuid::new_v4(),
                    "fingerprint": "openai_compatible:400:the '{model}' model is not supported when using codex with a chatgpt account.",
                    "provider": "openai_compatible",
                    "normalized_status_code": 400,
                    "semantic_error_code": "unsupported_model",
                    "action": "return_failure",
                    "retry_scope": "none",
                    "status": "provisional_live",
                    "templates": {
                        "en": "The requested model is not available."
                    },
                    "representative_samples": ["The 'gpt-5.4-ai-error-e2e-invalid' model is not supported when using Codex with a ChatGPT account."],
                    "hit_count": 1,
                    "first_seen_at": "2026-03-12T00:00:00Z",
                    "last_seen_at": "2026-03-12T00:00:00Z",
                    "updated_at": "2026-03-12T00:00:00Z"
                }
            })))
            .mount(&control_plane)
            .await;

        let resolution = resolve_upstream_error_learning(
            state.as_ref(),
            "openai_compatible",
            &UpstreamErrorContext {
                upstream_status: axum::http::StatusCode::BAD_REQUEST,
                status: axum::http::StatusCode::BAD_REQUEST,
                error_code: None,
                error_message: Some(
                    "The 'gpt-5.4-ai-error-e2e-invalid' model is not supported when using Codex with a ChatGPT account."
                        .to_string(),
                ),
                raw_error: Some(
                    r#"{"detail":"The 'gpt-5.4-ai-error-e2e-invalid' model is not supported when using Codex with a ChatGPT account.","input":"secret prompt"}"#
                        .to_string(),
                ),
                retry_after: None,
                upstream_request_id: None,
                class: UpstreamErrorClass::NonRetryableClient,
                learned_resolution: None,
            },
            "en",
            Some("gpt-5.4-ai-error-e2e-invalid"),
        )
        .await
        .expect("non-retryable client errors without explicit code should still resolve");

        assert_eq!(resolution.semantic_error_code, "unsupported_model");
        assert_eq!(
            resolution.localized_message,
            "The requested model is not available."
        );

        let requests = control_plane
            .received_requests()
            .await
            .expect("received requests should be available");
        assert_eq!(requests.len(), 1);
        let request_json: serde_json::Value =
            serde_json::from_slice(&requests[0].body).expect("request body should be json");
        assert_eq!(
            request_json["fingerprint"],
            "openai_compatible:400:the '{model}' model is not supported when using codex with a chatgpt account."
        );
        assert_eq!(request_json["provider"], "openai_compatible");
        assert_eq!(request_json["normalized_status_code"], 400);
        assert_eq!(
            request_json["normalized_upstream_message"],
            "The 'gpt-5.4-ai-error-e2e-invalid' model is not supported when using Codex with a ChatGPT account."
        );
        assert_eq!(
            request_json["sanitized_upstream_raw"],
            sanitized_upstream_raw
        );
    }

    #[tokio::test]
    async fn ai_error_learning_prefers_approved_snapshot_template_before_control_plane() {
        let state = test_state();
        let template = UpstreamErrorTemplateRecord {
            id: Uuid::new_v4(),
            fingerprint: "openai_compatible:400:model {model} does not exist".to_string(),
            provider: "openai_compatible".to_string(),
            normalized_status_code: 400,
            semantic_error_code: "unsupported_model".to_string(),
            action: UpstreamErrorAction::ReturnFailure,
            retry_scope: UpstreamErrorRetryScope::None,
            status: codex_pool_core::model::UpstreamErrorTemplateStatus::Approved,
            templates: codex_pool_core::model::LocalizedErrorTemplates {
                en: Some("The requested model is not available.".to_string()),
                zh_cn: Some("请求的模型当前不可用。".to_string()),
                ..codex_pool_core::model::LocalizedErrorTemplates::default()
            },
            representative_samples: vec!["Model {model} does not exist".to_string()],
            hit_count: 11,
            first_seen_at: chrono::Utc::now(),
            last_seen_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        state
            .approved_upstream_error_templates
            .write()
            .unwrap()
            .insert(template.fingerprint.clone(), template);
        let error_context = UpstreamErrorContext {
            upstream_status: axum::http::StatusCode::BAD_REQUEST,
            status: axum::http::StatusCode::BAD_REQUEST,
            error_code: None,
            error_message: Some("Model gpt-5.4 does not exist".to_string()),
            raw_error: Some(r#"{"error":{"message":"Model gpt-5.4 does not exist"}}"#.to_string()),
            retry_after: None,
            upstream_request_id: None,
            class: UpstreamErrorClass::Unknown,
            learned_resolution: None,
        };

        let resolution = resolve_upstream_error_learning(
            state.as_ref(),
            "openai_compatible",
            &error_context,
            "zh-CN",
            Some("gpt-5.4"),
        )
        .await
        .expect("approved template should be reused");

        assert_eq!(resolution.semantic_error_code, "unsupported_model");
        assert_eq!(resolution.localized_message, "请求的模型当前不可用。");
        assert_eq!(resolution.action, UpstreamErrorAction::ReturnFailure);
    }
}
