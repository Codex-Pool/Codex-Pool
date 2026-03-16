use std::collections::{HashSet, VecDeque};
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Context;
use axum::body::Body;
use axum::extract::ws::rejection::WebSocketUpgradeRejection;
use axum::extract::ws::{
    CloseFrame as AxumCloseFrame, Message as AxumWsMessage, WebSocket, WebSocketUpgrade,
};
use axum::extract::State;
use axum::http::header::{AUTHORIZATION, CACHE_CONTROL, CONTENT_LENGTH, ETAG, HOST, IF_NONE_MATCH};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Request, StatusCode};
use axum::response::Response;
use codex_pool_core::api::ErrorEnvelope;
use codex_pool_core::events::RequestLogEvent;
use codex_pool_core::model::{UpstreamAccount, UpstreamMode};
use futures_util::{SinkExt, Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::error::{
    Error as TungsteniteError, ProtocolError, SubProtocolError,
};
use tokio_tungstenite::tungstenite::handshake::client::Request as TungsteniteRequest;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::{
    CloseFrame as TungsteniteCloseFrame, Message as TungsteniteMessage,
};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tracing::warn;
use uuid::Uuid;

use crate::app::{AppState, CachedModelsResponse};
use crate::auth::ApiPrincipal;
use crate::upstream_health::{
    LiveResultReportRequest, LiveResultReportSource, LiveResultReportStatus,
};

const CHATGPT_ACCOUNT_ID: &str = "chatgpt-account-id";
const OPENAI_BETA_HEADER: &str = "openai-beta";
const X_OPENAI_SUBAGENT_HEADER: &str = "x-openai-subagent";
const SESSION_ID_HEADER: &str = "session_id";
const X_SESSION_ID_HEADER: &str = "x-session-id";
const X_CODEX_HEADER_PREFIX: &str = "x-codex-";
const SEC_WEBSOCKET_PROTOCOL_HEADER: &str = "sec-websocket-protocol";
const CODEX_CLIENT_VERSION_QUERY_KEY: &str = "client_version";
const CODEX_MODELS_PATH_SUFFIX: &str = "/models";
const CONVERSATION_ID_HEADER: &str = "conversation_id";
const AUTH_ERROR_EJECTION_MULTIPLIER: u64 = 10;
const AUTH_ERROR_EJECTION_MIN_SEC: u64 = 120;
const AUTH_ERROR_EJECTION_MAX_SEC: u64 = 1800;
const AUTH_EXPIRED_EJECTION_MIN_SEC: u64 = 600;
const AUTH_EXPIRED_EJECTION_MAX_SEC: u64 = 3600;
const QUOTA_EXHAUSTED_EJECTION_MIN_SEC: u64 = 1800;
const QUOTA_EXHAUSTED_EJECTION_MAX_SEC: u64 = 24 * 60 * 60;
const RATE_LIMITED_EJECTION_MIN_SEC: u64 = 30;
const SERVER_ERROR_EJECTION_MIN_SEC: u64 = 5;
const SERVER_ERROR_EJECTION_MAX_SEC: u64 = 60;
const TOKEN_INVALIDATED_RECOVERY_EJECTION_SEC: u64 = 5;
const MIN_DISTINCT_FAILOVER_ATTEMPTS: usize = 2;
const INTERNAL_RECOVERY_TIMEOUT_SEC: u64 = 5;
const INTERNAL_BILLING_TIMEOUT_SEC: u64 = 5;
const INTERNAL_BILLING_PRICING_TIMEOUT_SEC: u64 = 2;
const ROUTING_CACHE_STICKY_TTL_SEC: u64 = 30 * 60;
const BILLING_AUTHORIZATION_TTL_SEC: u64 = 15 * 60;
const BILLING_PRICING_CACHE_TTL_SEC: u64 = 30;
const MODELS_CACHE_TTL_SEC: u64 = 60;
const STREAM_PROXY_BUFFER_SIZE: usize = 16;
const STREAM_USAGE_ESTIMATE_FALLBACK_ENABLED: bool = true;

type UpstreamWebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;
type UpstreamByteStream = Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProxyRecoveryAction {
    RotateRefreshToken,
    DisableAccount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProxyRecoveryOutcome {
    NotApplied,
    RotateSucceeded,
    RotateFailed,
    DisableAttempted,
}

#[derive(Debug, Deserialize)]
struct InternalOAuthRefreshPayload {
    last_refresh_status: String,
}

#[derive(Debug, Default)]
struct ParsedRequestPolicyContext {
    model: Option<String>,
    requested_service_tier: Option<String>,
    stream: bool,
    request_id: Option<String>,
    detected_locale: String,
    estimated_input_tokens: Option<i64>,
    continuation_key_hint: Option<String>,
    sticky_key_hint: Option<String>,
    session_key_hint: Option<String>,
}

#[derive(Debug)]
enum StreamPreludeError {
    EndOfStreamBeforeFirstChunk,
    UpstreamReadFailed(String),
    UpstreamErrorResponse(UpstreamErrorContext),
}

#[derive(Debug, Clone)]
struct PendingBillingSession {
    tenant_id: Uuid,
    api_key_id: Uuid,
    request_id: String,
    trace_request_id: Option<String>,
    model: String,
    requested_service_tier: Option<String>,
    session_key: String,
    request_kind: String,
    is_stream: bool,
    estimated_input_tokens: i64,
    reserved_microcredits: i64,
}

impl PendingBillingSession {
    fn rotate_request_id_for_cross_account_failover(&mut self) {
        let previous_request_id = self.request_id.clone();
        self.request_id = generate_billing_request_id();
        if self.trace_request_id.is_none() && self.session_key == previous_request_id {
            self.session_key = self.request_id.clone();
        }
    }
}

#[derive(Debug, Clone)]
struct BillingSession {
    account_id: Uuid,
    tenant_id: Uuid,
    api_key_id: Uuid,
    request_path: String,
    request_method: String,
    request_started: Instant,
    request_id: String,
    trace_request_id: Option<String>,
    model: String,
    requested_service_tier: Option<String>,
    effective_service_tier: Option<String>,
    session_key: String,
    request_kind: String,
    is_stream: bool,
    first_token_latency_ms: Option<u64>,
    estimated_input_tokens: i64,
    authorization_id: Uuid,
    reserved_microcredits: i64,
}

#[derive(Debug, Clone)]
struct BillingSettleResult {
    authorization_id: Uuid,
    capture_status: String,
    input_tokens: i64,
    cached_input_tokens: i64,
    output_tokens: i64,
    reasoning_tokens: i64,
}

#[derive(Debug, Clone)]
struct StreamRequestLogContext {
    account_id: Uuid,
    tenant_id: Option<Uuid>,
    api_key_id: Option<Uuid>,
    request_path: String,
    request_method: String,
    request_started: Instant,
    request_id: Option<String>,
    model: Option<String>,
    requested_service_tier: Option<String>,
    estimated_input_tokens: Option<i64>,
}

#[derive(Debug, Clone)]
enum BillingSettleOutcome {
    NotNeeded,
    Settled(BillingSettleResult),
    DeferredEstimated {
        authorization_id: Uuid,
        usage: UsageTokens,
    },
}

#[derive(Debug, Default)]
struct SseUsageTracker {
    line_buffer: Vec<u8>,
    usage: Option<UsageTokens>,
    service_tier: Option<String>,
    output_text_chars: usize,
    saw_output_text_delta: bool,
    used_json_line_fallback: bool,
}

#[derive(Debug, Default)]
struct StreamUsageObservation {
    usage: Option<UsageTokens>,
    service_tier: Option<String>,
    estimated_output_tokens: Option<i64>,
    used_json_line_fallback: bool,
}

#[derive(Debug, Serialize)]
struct InternalBillingAuthorizePayload {
    tenant_id: Uuid,
    api_key_id: Option<Uuid>,
    request_id: String,
    trace_request_id: Option<String>,
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    service_tier: Option<String>,
    session_key: Option<String>,
    request_kind: Option<String>,
    reserved_microcredits: i64,
    ttl_sec: Option<u64>,
    #[serde(default)]
    is_stream: bool,
}

#[derive(Debug, Deserialize)]
struct InternalBillingAuthorizeResponse {
    authorization_id: Uuid,
    status: String,
    reserved_microcredits: i64,
}

#[derive(Debug, Serialize)]
struct InternalBillingCapturePayload {
    tenant_id: Uuid,
    api_key_id: Option<Uuid>,
    request_id: String,
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    service_tier: Option<String>,
    session_key: Option<String>,
    request_kind: Option<String>,
    input_tokens: i64,
    #[serde(default)]
    cached_input_tokens: i64,
    output_tokens: i64,
    #[serde(default)]
    reasoning_tokens: i64,
    #[serde(default)]
    is_stream: bool,
}

#[derive(Debug, Deserialize)]
struct InternalBillingCaptureResponse {
    status: String,
    #[serde(default)]
    charged_microcredits: Option<i64>,
}

#[derive(Debug, Serialize)]
struct InternalBillingPricingPayload {
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_key_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    service_tier: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct InternalBillingPricingResponse {
    input_price_microcredits: i64,
    cached_input_price_microcredits: i64,
    output_price_microcredits: i64,
    source: String,
}

#[derive(Debug, Serialize)]
struct InternalBillingReleasePayload {
    tenant_id: Uuid,
    request_id: String,
    #[serde(default)]
    is_stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    release_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    upstream_status_code: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    upstream_error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failover_action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failover_reason_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recovery_action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recovery_outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cross_account_failover_attempted: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct InternalBillingReleaseResponse {
    status: String,
}

#[derive(Debug, Clone, Default)]
struct BillingReleaseFailureContext {
    release_reason: Option<String>,
    upstream_status_code: Option<u16>,
    upstream_error_code: Option<String>,
    failover_action: Option<String>,
    failover_reason_class: Option<String>,
    recovery_action: Option<String>,
    recovery_outcome: Option<String>,
    cross_account_failover_attempted: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct InternalBillingErrorEnvelope {
    error: InternalBillingErrorBody,
}

#[derive(Debug, Deserialize)]
struct InternalBillingErrorBody {
    #[serde(default)]
    code: String,
    message: String,
}

pub async fn proxy_handler(
    State(state): State<std::sync::Arc<AppState>>,
    request: Request<Body>,
) -> Response {
    let principal = request.extensions().get::<ApiPrincipal>().cloned();
    let (parts, body) = request.into_parts();
    let path = parts.uri.path().to_string();
    let query = parts.uri.query().map(|v| v.to_string());
    let method = parts.method.clone();
    let client_version_header = parts
        .headers
        .get("version")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    let is_models_request = method == axum::http::Method::GET && path == "/v1/models";

    let max_request_body_bytes =
        max_request_body_bytes_for_path(state.max_request_body_bytes, &path);
    let header_locale = detect_request_locale(&parts.headers, &bytes::Bytes::new());
    let body_bytes = match axum::body::to_bytes(body, max_request_body_bytes).await {
        Ok(bytes) => bytes,
        Err(err) => {
            warn!(
                error = %err,
                max_request_body_bytes,
                default_max_request_body_bytes = state.max_request_body_bytes,
                "failed to read request body"
            );
            if is_body_too_large_error(&err) {
                return localized_json_error_with_state(
                    state.as_ref(),
                    header_locale.as_str(),
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "payload_too_large",
                    "request body exceeds the configured limit",
                );
            }
            return localized_json_error_with_state(
                state.as_ref(),
                header_locale.as_str(),
                StatusCode::BAD_REQUEST,
                "invalid_request_body",
                "failed to read request body",
            );
        }
    };
    let parsed_policy_context = parse_request_policy_context(&parts.headers, &body_bytes);
    let sticky_key = parsed_policy_context
        .continuation_key_hint
        .clone()
        .or_else(|| sticky_session_key_from_headers(&parts.headers))
        .or_else(|| parsed_policy_context.sticky_key_hint.clone());
    if let Err(response) =
        enforce_principal_request_policy(principal.as_ref(), &parts.headers, &parsed_policy_context)
    {
        return *response;
    }
    if let Some(response) = enforce_invalid_request_guard(&state, principal.as_ref()) {
        return response;
    }
    let mut pending_billing_session = match build_pending_billing_session(
        &state,
        principal.as_ref(),
        &parts.headers,
        &parsed_policy_context,
        &path,
        method.as_str(),
    )
    .await
    {
        Ok(session) => session,
        Err(error_response) => return *error_response,
    };
    let mut billing_session: Option<BillingSession> = None;

    let started = Instant::now();
    let failover_deadline = Instant::now() + state.request_failover_wait;
    let mut tried_account_ids = HashSet::new();
    let mut last_failure: Option<(Response, StatusCode, Uuid)> = None;
    let mut did_cross_account_failover = false;
    let mut forced_distinct_failover_round = false;

    if let Some(sticky_key) = sticky_key.as_deref() {
        if let Ok(Some(account_id)) = state.routing_cache.get_sticky_account_id(sticky_key).await {
            let _ = state.router.bind_sticky(sticky_key, account_id);
        }
    }

    loop {
        let alive_ring_account = if sticky_key.is_none() {
            pick_account_from_alive_ring(&state, &tried_account_ids, None).await
        } else {
            None
        };
        let account = match alive_ring_account.or_else(|| {
            state.router.pick_for_model(
                parsed_policy_context.model.as_deref(),
                sticky_key.as_deref(),
                &tried_account_ids,
                state.sticky_prefer_non_conflicting,
            )
        }) {
            Some(account) => account,
            None => {
                if state.enable_request_failover
                    && !forced_distinct_failover_round
                    && !tried_account_ids.is_empty()
                    && state.router.enabled_total() >= MIN_DISTINCT_FAILOVER_ATTEMPTS
                {
                    forced_distinct_failover_round = true;
                    tried_account_ids.clear();
                    wait_for_route_update_or_deadline(&state, failover_deadline).await;
                    continue;
                }
                if state.enable_request_failover && Instant::now() < failover_deadline {
                    tried_account_ids.clear();
                    wait_for_route_update_or_deadline(&state, failover_deadline).await;
                    continue;
                }

                if let Some((response, status, account_id)) = last_failure.take() {
                    log_failover_decision(
                        UpstreamErrorSource::Http,
                        Some(account_id),
                        Some(status),
                        None,
                        "failover_exhausted",
                        "none",
                        "none",
                        "return_failure",
                    );
                    record_failover_exhausted_if_needed(&state, did_cross_account_failover);
                    emit_request_log_event(
                        &state,
                        account_id,
                        principal.as_ref(),
                        &path,
                        method.as_str(),
                        status,
                        started,
                        false,
                        parsed_policy_context.request_id.as_deref(),
                        parsed_policy_context.model.as_deref(),
                        parsed_policy_context.requested_service_tier.as_deref(),
                    )
                    .await;
                    release_billing_hold_best_effort(
                        state.clone(),
                        billing_session.take(),
                        Some(BillingReleaseFailureContext {
                            release_reason: Some("failover_exhausted".to_string()),
                            upstream_status_code: Some(status.as_u16()),
                            failover_action: Some("return_failure".to_string()),
                            failover_reason_class: Some("failover_exhausted".to_string()),
                            cross_account_failover_attempted: Some(did_cross_account_failover),
                            ..Default::default()
                        }),
                    )
                    .await;
                    return response;
                }

                log_failover_decision(
                    UpstreamErrorSource::Http,
                    None,
                    Some(StatusCode::SERVICE_UNAVAILABLE),
                    None,
                    "no_upstream_account",
                    "none",
                    "none",
                    "return_failure",
                );
                record_failover_exhausted_if_needed(&state, did_cross_account_failover);
                release_billing_hold_best_effort(
                    state.clone(),
                    billing_session.take(),
                    Some(BillingReleaseFailureContext {
                        release_reason: Some("no_upstream_account".to_string()),
                        upstream_status_code: Some(StatusCode::SERVICE_UNAVAILABLE.as_u16()),
                        failover_action: Some("return_failure".to_string()),
                        failover_reason_class: Some("no_upstream_account".to_string()),
                        cross_account_failover_attempted: Some(did_cross_account_failover),
                        ..Default::default()
                    }),
                )
                .await;
                return localized_json_error_with_state(
                    state.as_ref(),
                    parsed_policy_context.detected_locale.as_str(),
                    StatusCode::SERVICE_UNAVAILABLE,
                    "no_upstream_account",
                    "no enabled upstream account is available",
                );
            }
        };

        if state.shared_routing_cache_enabled {
            if let Ok(true) = state.routing_cache.is_unhealthy(account.id).await {
                state.router.mark_unhealthy(
                    account.id,
                    state.retry_poll_interval.max(Duration::from_millis(1)),
                );
                tried_account_ids.insert(account.id);
                continue;
            }
        }

        if let Some(sticky_key) = sticky_key.as_deref() {
            let _ = state
                .routing_cache
                .set_sticky_account_id(
                    sticky_key,
                    account.id,
                    Duration::from_secs(ROUTING_CACHE_STICKY_TTL_SEC),
                )
                .await;
        }

        let upstream_url =
            match build_upstream_url(
                &account.base_url,
                &account.mode,
                &path,
                query.as_deref(),
                client_version_header.as_deref(),
            ) {
                Ok(url) => url,
                Err(err) => {
                    warn!(error = %err, "failed to build upstream url");
                    let response = localized_json_error_with_state(
                        state.as_ref(),
                        parsed_policy_context.detected_locale.as_str(),
                        StatusCode::BAD_GATEWAY,
                        "invalid_upstream_url",
                        "failed to build upstream URL",
                    );
                    emit_request_log_event(
                        &state,
                        account.id,
                        principal.as_ref(),
                        &path,
                        method.as_str(),
                        StatusCode::BAD_GATEWAY,
                        started,
                        false,
                        parsed_policy_context.request_id.as_deref(),
                        parsed_policy_context.model.as_deref(),
                        parsed_policy_context.requested_service_tier.as_deref(),
                    )
                    .await;
                    record_failover_exhausted_if_needed(&state, did_cross_account_failover);
                    release_billing_hold_best_effort(
                        state.clone(),
                        billing_session.take(),
                        Some(BillingReleaseFailureContext {
                            release_reason: Some("invalid_upstream_url".to_string()),
                            upstream_status_code: Some(StatusCode::BAD_GATEWAY.as_u16()),
                            failover_action: Some("return_failure".to_string()),
                            failover_reason_class: Some("invalid_upstream_url".to_string()),
                            cross_account_failover_attempted: Some(did_cross_account_failover),
                            ..Default::default()
                        }),
                    )
                    .await;
                    return response;
                }
            };
        let models_cache_key = is_models_request
            .then(|| build_models_cache_key(account.id, upstream_url.as_str()));
        if let Some(cache_key) = models_cache_key.as_deref() {
            if let Some(response) = serve_models_from_cache(state.as_ref(), &parts.headers, cache_key)
            {
                return response;
            }
        }
        let upstream_request_adaptation = maybe_adapt_openai_responses_request_for_codex_profile(
            &account.mode,
            &account.base_url,
            &path,
            &parts.headers,
            &body_bytes,
        );
        let upstream_request_body = upstream_request_adaptation
            .as_ref()
            .map(|item| item.body.clone())
            .unwrap_or_else(|| body_bytes.clone());
        let bridge_stream_response = upstream_request_adaptation
            .as_ref()
            .is_some_and(|item| item.bridge_stream_response);

        let mut same_account_retry_attempt: u32 = 0;
        loop {
            if billing_session.is_none() {
                if let Some(pending_session) = pending_billing_session.as_ref() {
                    match authorize_billing_session(
                        &state,
                        pending_session,
                        account.id,
                        &path,
                        method.as_str(),
                        started,
                        pending_session.is_stream,
                    )
                    .await
                    {
                        Ok(session) => {
                            billing_session = Some(session);
                        }
                        Err(error_response) => return *error_response,
                    }
                }
            }

            let mut upstream_request = state
                .http_client
                .request(method.clone(), upstream_url.clone());
            upstream_request = apply_passthrough_headers(
                upstream_request,
                &parts.headers,
                upstream_request_adaptation
                    .as_ref()
                    .is_some_and(|item| item.strip_content_encoding),
            );
            if parts.headers.get(SESSION_ID_HEADER).is_none() {
                if let Some(raw_value) = parts.headers.get(X_SESSION_ID_HEADER) {
                    if let Ok(value) = raw_value.to_str() {
                        let value = value.trim();
                        if !value.is_empty() {
                            upstream_request = upstream_request.header(SESSION_ID_HEADER, value);
                        }
                    }
                } else if let Some(raw_value) = parts.headers.get(CONVERSATION_ID_HEADER) {
                    if let Ok(value) = raw_value.to_str() {
                        let value = value.trim();
                        if !value.is_empty() {
                            upstream_request = upstream_request.header(SESSION_ID_HEADER, value);
                        }
                    }
                }
            }
            upstream_request =
                upstream_request.header(AUTHORIZATION, format!("Bearer {}", account.bearer_token));
            if let Some(account_id) = account.chatgpt_account_id.as_deref() {
                upstream_request = upstream_request.header(CHATGPT_ACCOUNT_ID, account_id);
            }
            upstream_request = upstream_request.body(upstream_request_body.clone());

            let upstream_response = match upstream_request.send().await {
                Ok(resp) => resp,
                Err(err) => {
                    warn!(error = %err, account_id = %account.id, "upstream request failed");
                    if state.enable_request_failover
                        && should_retry_same_account_on_transport(
                            same_account_retry_attempt,
                            &state,
                        )
                        && Instant::now() < failover_deadline
                    {
                        log_failover_decision(
                            UpstreamErrorSource::Http,
                            Some(account.id),
                            Some(StatusCode::BAD_GATEWAY),
                            None,
                            "transport_error",
                            "none",
                            "none",
                            "retry_same_account",
                        );
                        same_account_retry_attempt += 1;
                        record_same_account_retry(&state);
                        tokio::time::sleep(state.retry_poll_interval).await;
                        continue;
                    }

                    state
                        .router
                        .mark_unhealthy(account.id, state.account_ejection_ttl);
                    let _ = state
                        .routing_cache
                        .set_unhealthy(account.id, state.account_ejection_ttl)
                        .await;
                    if let Some(sticky_key) = sticky_key.as_deref() {
                        let _ = state.router.unbind_sticky(sticky_key);
                        let _ = state
                            .routing_cache
                            .delete_sticky_account_id(sticky_key)
                            .await;
                    }

                    let response = localized_json_error_with_state(
                        state.as_ref(),
                        parsed_policy_context.detected_locale.as_str(),
                        StatusCode::BAD_GATEWAY,
                        "upstream_transport_error",
                        "upstream request failed",
                    );
                    if should_cross_account_failover(
                        &state,
                        parsed_policy_context.model.as_deref(),
                        sticky_key.as_deref(),
                        failover_deadline,
                        &tried_account_ids,
                        account.id,
                        true,
                    ) {
                        log_failover_decision(
                            UpstreamErrorSource::Http,
                            Some(account.id),
                            Some(StatusCode::BAD_GATEWAY),
                            None,
                            "transport_error",
                            "none",
                            "none",
                            "cross_account_failover",
                        );
                        release_billing_hold_for_cross_account_failover(
                            state.clone(),
                            &mut pending_billing_session,
                            &mut billing_session,
                            StatusCode::BAD_GATEWAY,
                            None,
                            "transport_error",
                        )
                        .await;
                        record_cross_account_failover_attempt(
                            &state,
                            &mut tried_account_ids,
                            account.id,
                            &mut did_cross_account_failover,
                        );
                        last_failure = Some((response, StatusCode::BAD_GATEWAY, account.id));
                        break;
                    }

                    log_failover_decision(
                        UpstreamErrorSource::Http,
                        Some(account.id),
                        Some(StatusCode::BAD_GATEWAY),
                        None,
                        "transport_error",
                        "none",
                        "none",
                        "return_failure",
                    );
                    emit_request_log_event(
                        &state,
                        account.id,
                        principal.as_ref(),
                        &path,
                        method.as_str(),
                        StatusCode::BAD_GATEWAY,
                        started,
                        false,
                        parsed_policy_context.request_id.as_deref(),
                        parsed_policy_context.model.as_deref(),
                        parsed_policy_context.requested_service_tier.as_deref(),
                    )
                    .await;
                    record_failover_exhausted_if_needed(&state, did_cross_account_failover);
                    release_billing_hold_best_effort(
                        state.clone(),
                        billing_session.take(),
                        Some(BillingReleaseFailureContext {
                            release_reason: Some("transport_error".to_string()),
                            upstream_status_code: Some(StatusCode::BAD_GATEWAY.as_u16()),
                            failover_action: Some("return_failure".to_string()),
                            failover_reason_class: Some("transport_error".to_string()),
                            cross_account_failover_attempted: Some(did_cross_account_failover),
                            ..Default::default()
                        }),
                    )
                    .await;
                    return response;
                }
            };

            let status = upstream_response.status();
            let response_headers = upstream_response.headers().clone();
            let content_type_indicates_stream = response_headers
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v.contains("text/event-stream"));
            // Some upstreams may omit SSE content-type headers on successful stream responses.
            // For requests that explicitly ask for streaming, treat successful responses as stream
            // even when content-type is missing or unexpected.
            let is_stream = content_type_indicates_stream
                || (parsed_policy_context.stream && status.is_success());
            if let Some(session) = billing_session.as_mut() {
                session.is_stream = if bridge_stream_response { false } else { is_stream };
            }

            if bridge_stream_response && status.is_success() {
                let (response, upstream_error_context, body) =
                    buffered_codex_compat_response(
                        &state,
                        upstream_provider_label(&account.mode),
                        parsed_policy_context.detected_locale.as_str(),
                        parsed_policy_context.model.as_deref(),
                        status,
                        &response_headers,
                        upstream_response,
                    )
                    .await;
                let is_503_overloaded = status == StatusCode::SERVICE_UNAVAILABLE
                    && upstream_error_context
                        .as_ref()
                        .is_some_and(|context| context.class == UpstreamErrorClass::Overloaded);
                let (
                    response,
                    upstream_error_context,
                    non_stream_observed_usage,
                    non_stream_billing_settle,
                    non_stream_billing_deferred,
                    non_stream_billing_failed,
                    non_stream_effective_service_tier,
                ) = (
                    response,
                    upstream_error_context,
                    extract_usage_tokens(&body),
                    None,
                    None,
                    None,
                    extract_response_service_tier(&body)
                        .or_else(|| parsed_policy_context.requested_service_tier.clone()),
                );
                let mut non_stream_billing_settle = non_stream_billing_settle;
                let mut non_stream_billing_deferred = non_stream_billing_deferred;
                let mut non_stream_billing_failed = non_stream_billing_failed;
                let non_stream_estimated_usage = if non_stream_observed_usage.is_none() {
                    let estimated_input_tokens =
                        parsed_policy_context.estimated_input_tokens.unwrap_or(0).max(0);
                    let estimated_output_tokens =
                        estimate_response_output_tokens(&body).unwrap_or(0).max(0);
                    if estimated_input_tokens > 0 || estimated_output_tokens > 0 {
                        Some(UsageTokens {
                            input_tokens: estimated_input_tokens,
                            cached_input_tokens: 0,
                            output_tokens: estimated_output_tokens,
                            reasoning_tokens: 0,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                };
                let response = match settle_billing_if_needed(
                    state.clone(),
                    billing_session.as_ref(),
                    status,
                    &body,
                )
                .await
                {
                    Ok(outcome) => {
                        match outcome {
                            BillingSettleOutcome::NotNeeded => {}
                            BillingSettleOutcome::Settled(settle_result) => {
                                non_stream_billing_settle = Some(settle_result);
                                billing_session = None;
                            }
                            BillingSettleOutcome::DeferredEstimated {
                                authorization_id,
                                usage,
                            } => {
                                non_stream_billing_deferred = Some((authorization_id, usage));
                                billing_session = None;
                            }
                        }
                        response
                    }
                    Err(_error_response) => {
                        let failed_authorization_id = billing_session
                            .take()
                            .map(|session| session.authorization_id);
                        let failed_usage = non_stream_observed_usage.or(non_stream_estimated_usage);
                        if let (Some(authorization_id), Some(usage)) =
                            (failed_authorization_id, failed_usage)
                        {
                            non_stream_billing_failed = Some((authorization_id, usage));
                        }
                        response
                    }
                };

                if status.is_success() {
                    let (
                        billing_phase,
                        authorization_id,
                        capture_status,
                        input_tokens,
                        cached_input_tokens,
                        output_tokens,
                        reasoning_tokens,
                    ) = if let Some(settle_result) = non_stream_billing_settle.as_ref() {
                        (
                            Some("released"),
                            Some(settle_result.authorization_id),
                            Some(settle_result.capture_status.as_str()),
                            Some(settle_result.input_tokens),
                            Some(settle_result.cached_input_tokens),
                            Some(settle_result.output_tokens),
                            Some(settle_result.reasoning_tokens),
                        )
                    } else if let Some((authorization_id, usage)) = non_stream_billing_deferred {
                        (
                            Some("deferred_estimated"),
                            Some(authorization_id),
                            None,
                            Some(usage.input_tokens),
                            Some(usage.cached_input_tokens),
                            Some(usage.output_tokens),
                            Some(usage.reasoning_tokens),
                        )
                    } else if let Some((authorization_id, usage)) = non_stream_billing_failed {
                        (
                            Some("released_failed"),
                            Some(authorization_id),
                            None,
                            Some(usage.input_tokens),
                            Some(usage.cached_input_tokens),
                            Some(usage.output_tokens),
                            Some(usage.reasoning_tokens),
                        )
                    } else if let Some(usage) = non_stream_observed_usage {
                        (
                            None,
                            None,
                            None,
                            Some(usage.input_tokens),
                            Some(usage.cached_input_tokens),
                            Some(usage.output_tokens),
                            Some(usage.reasoning_tokens),
                        )
                    } else if let Some(usage) = non_stream_estimated_usage {
                        (
                            None,
                            None,
                            None,
                            Some(usage.input_tokens),
                            Some(usage.cached_input_tokens),
                            Some(usage.output_tokens),
                            Some(usage.reasoning_tokens),
                        )
                    } else {
                        (None, None, None, None, None, None, None)
                    };
                    emit_request_log_event_with_billing(
                        &state,
                        account.id,
                        principal.as_ref(),
                        &path,
                        method.as_str(),
                        status,
                        started,
                        false,
                        parsed_policy_context.request_id.as_deref(),
                        parsed_policy_context.model.as_deref(),
                        non_stream_effective_service_tier.as_deref(),
                        billing_phase,
                        authorization_id,
                        capture_status,
                        input_tokens,
                        cached_input_tokens,
                        output_tokens,
                        reasoning_tokens,
                        Some(started.elapsed().as_millis() as u64),
                    )
                    .await;
                    spawn_seen_ok_reports(&state, account.id, parsed_policy_context.model.clone());
                    record_failover_success_if_needed(&state, did_cross_account_failover);
                    return response;
                }

                spawn_failed_live_report(
                    &state,
                    account.id,
                    parsed_policy_context.model.clone(),
                    status,
                    upstream_error_context.as_ref(),
                );
                let retryable = is_failover_retryable_error(
                    UpstreamErrorSource::Http,
                    status,
                    upstream_error_context.as_ref(),
                );
                let same_account_retryable = should_retry_same_account_on_status(
                    UpstreamErrorSource::Http,
                    status,
                    is_503_overloaded,
                    same_account_retry_attempt,
                    &state,
                    upstream_error_context.as_ref(),
                );
                if state.enable_request_failover
                    && retryable
                    && same_account_retryable
                    && Instant::now() < failover_deadline
                {
                    log_failover_decision(
                        UpstreamErrorSource::Http,
                        Some(account.id),
                        Some(status),
                        upstream_error_context.as_ref(),
                        "retryable_status",
                        recovery_action_label(upstream_error_context.as_ref()),
                        "not_applied",
                        "retry_same_account",
                    );
                    same_account_retry_attempt += 1;
                    record_same_account_retry(&state);
                    tokio::time::sleep(state.retry_poll_interval).await;
                    continue;
                }

                let should_failover_across_accounts = should_cross_account_failover(
                    &state,
                    parsed_policy_context.model.as_deref(),
                    sticky_key.as_deref(),
                    failover_deadline,
                    &tried_account_ids,
                    account.id,
                    retryable,
                );
                let recovery_outcome = if should_failover_across_accounts {
                    if upstream_error_context.is_some() {
                        let state_for_recovery = state.clone();
                        let recovery_context = upstream_error_context.clone();
                        tokio::spawn(async move {
                            let _ = apply_recovery_action(
                                state_for_recovery.as_ref(),
                                account.id,
                                recovery_context.as_ref(),
                            )
                            .await;
                        });
                    }
                    ProxyRecoveryOutcome::NotApplied
                } else {
                    apply_recovery_action(&state, account.id, upstream_error_context.as_ref()).await
                };
                if let Some(ejection_ttl) = ejection_ttl_for_response(
                    status,
                    state.account_ejection_ttl,
                    is_503_overloaded,
                    upstream_error_context.as_ref(),
                    recovery_outcome,
                ) {
                    state.router.mark_unhealthy(account.id, ejection_ttl);
                    let _ = state
                        .routing_cache
                        .set_unhealthy(account.id, ejection_ttl)
                        .await;
                    if let Some(sticky_key) = sticky_key.as_deref() {
                        let _ = state.router.unbind_sticky(sticky_key);
                        let _ = state
                            .routing_cache
                            .delete_sticky_account_id(sticky_key)
                            .await;
                    }
                }
                record_invalid_request_guard_failure(
                    &state,
                    principal.as_ref(),
                    UpstreamErrorSource::Http,
                    status,
                    upstream_error_context.as_ref(),
                );

                if should_failover_across_accounts {
                    log_failover_decision(
                        UpstreamErrorSource::Http,
                        Some(account.id),
                        Some(status),
                        upstream_error_context.as_ref(),
                        "retryable_status",
                        recovery_action_label(upstream_error_context.as_ref()),
                        recovery_outcome_label(recovery_outcome),
                        "cross_account_failover",
                    );
                    release_billing_hold_for_cross_account_failover(
                        state.clone(),
                        &mut pending_billing_session,
                        &mut billing_session,
                        status,
                        upstream_error_context.as_ref(),
                        "retryable_status",
                    )
                    .await;
                    record_cross_account_failover_attempt(
                        &state,
                        &mut tried_account_ids,
                        account.id,
                        &mut did_cross_account_failover,
                    );
                    last_failure = Some((response, status, account.id));
                    break;
                }

                log_failover_decision(
                    UpstreamErrorSource::Http,
                    Some(account.id),
                    Some(status),
                    upstream_error_context.as_ref(),
                    "non_retryable_or_budget_exhausted",
                    recovery_action_label(upstream_error_context.as_ref()),
                    recovery_outcome_label(recovery_outcome),
                    "return_failure",
                );
                emit_request_log_event(
                    &state,
                    account.id,
                    principal.as_ref(),
                    &path,
                    method.as_str(),
                    status,
                    started,
                    false,
                    parsed_policy_context.request_id.as_deref(),
                    parsed_policy_context.model.as_deref(),
                    parsed_policy_context.requested_service_tier.as_deref(),
                )
                .await;
                record_failover_exhausted_if_needed(&state, did_cross_account_failover);
                release_billing_hold_best_effort(
                    state.clone(),
                    billing_session.take(),
                    Some(BillingReleaseFailureContext {
                        release_reason: Some("upstream_request_failed".to_string()),
                        upstream_status_code: Some(status.as_u16()),
                        upstream_error_code: upstream_error_context
                            .as_ref()
                            .and_then(|context| context.error_code.clone()),
                        failover_action: Some("return_failure".to_string()),
                        failover_reason_class: Some(
                            "non_retryable_or_budget_exhausted".to_string(),
                        ),
                        recovery_action: Some(
                            recovery_action_label(upstream_error_context.as_ref()).to_string(),
                        ),
                        recovery_outcome: Some(recovery_outcome_label(recovery_outcome).to_string()),
                        cross_account_failover_attempted: Some(did_cross_account_failover),
                    }),
                )
                .await;
                return response;
            }

            if is_stream && status.is_success() {
                match stream_response_with_first_chunk(
                    state.clone(),
                    status,
                    &response_headers,
                    upstream_response,
                    billing_session.clone(),
                    Some(StreamRequestLogContext {
                        account_id: account.id,
                        tenant_id: principal.as_ref().and_then(|item| item.tenant_id),
                        api_key_id: principal.as_ref().and_then(|item| item.api_key_id),
                        request_path: path.clone(),
                        request_method: method.to_string(),
                        request_started: started,
                        request_id: parsed_policy_context.request_id.clone(),
                        model: parsed_policy_context.model.clone(),
                        requested_service_tier: parsed_policy_context.requested_service_tier.clone(),
                        estimated_input_tokens: parsed_policy_context.estimated_input_tokens,
                    }),
                )
                .await
                {
                    Ok(response) => {
                        state
                            .stream_response_total
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if content_type_indicates_stream {
                            state
                                .stream_protocol_sse_header_total
                                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        } else if parsed_policy_context.stream {
                            state
                                .stream_protocol_header_missing_total
                                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                        spawn_seen_ok_reports(
                            &state,
                            account.id,
                            parsed_policy_context.model.clone(),
                        );
                        record_failover_success_if_needed(&state, did_cross_account_failover);
                        return response;
                    }
                    Err(err) => {
                        let mut error_context: Option<UpstreamErrorContext> = None;
                        let mut status = StatusCode::BAD_GATEWAY;
                        let mut response = localized_json_error_with_state(
                            state.as_ref(),
                            parsed_policy_context.detected_locale.as_str(),
                            StatusCode::BAD_GATEWAY,
                            "upstream_transport_error",
                            "upstream request failed",
                        );
                        let mut reason_class = "stream_prelude_error";
                        let mut should_retry_same_account = should_retry_same_account_on_transport(
                            same_account_retry_attempt,
                            &state,
                        );
                        let error_detail = match err {
                            StreamPreludeError::EndOfStreamBeforeFirstChunk => {
                                "end_of_stream_before_first_chunk".to_string()
                            }
                            StreamPreludeError::UpstreamReadFailed(message) => message,
                            StreamPreludeError::UpstreamErrorResponse(context) => {
                                let (learned_response, learned_context) =
                                    apply_upstream_error_learning(
                                        &state,
                                        UpstreamErrorSource::SsePrelude,
                                        upstream_provider_label(&account.mode),
                                        parsed_policy_context.detected_locale.as_str(),
                                        parsed_policy_context.model.as_deref(),
                                        context,
                                    )
                                    .await;
                                status = learned_context.status;
                                response = learned_response;
                                reason_class = "stream_upstream_error";
                                error_context = Some(learned_context);
                                should_retry_same_account = should_retry_same_account_on_status(
                                    UpstreamErrorSource::SsePrelude,
                                    status,
                                    false,
                                    same_account_retry_attempt,
                                    &state,
                                    error_context.as_ref(),
                                );
                                "upstream_stream_error_event".to_string()
                            }
                        };
                        warn!(
                            error = %error_detail,
                            account_id = %account.id,
                            "upstream stream ended before first chunk"
                        );
                        spawn_failed_live_report(
                            &state,
                            account.id,
                            parsed_policy_context.model.clone(),
                            status,
                            error_context.as_ref(),
                        );
                        if state.enable_request_failover
                            && should_retry_same_account
                            && Instant::now() < failover_deadline
                        {
                            log_failover_decision(
                                UpstreamErrorSource::SsePrelude,
                                Some(account.id),
                                Some(status),
                                error_context.as_ref(),
                                reason_class,
                                recovery_action_label(error_context.as_ref()),
                                "none",
                                "retry_same_account",
                            );
                            same_account_retry_attempt += 1;
                            record_same_account_retry(&state);
                            tokio::time::sleep(state.retry_poll_interval).await;
                            continue;
                        }

                        let recovery_outcome =
                            apply_recovery_action(&state, account.id, error_context.as_ref()).await;
                        let ejection_ttl = ejection_ttl_for_response(
                            status,
                            state.account_ejection_ttl,
                            false,
                            error_context.as_ref(),
                            recovery_outcome,
                        )
                        .unwrap_or(state.account_ejection_ttl);
                        state.router.mark_unhealthy(account.id, ejection_ttl);
                        let _ = state
                            .routing_cache
                            .set_unhealthy(account.id, ejection_ttl)
                            .await;
                        if let Some(sticky_key) = sticky_key.as_deref() {
                            let _ = state.router.unbind_sticky(sticky_key);
                            let _ = state
                                .routing_cache
                                .delete_sticky_account_id(sticky_key)
                                .await;
                        }

                        let retryable = is_failover_retryable_error(
                            UpstreamErrorSource::SsePrelude,
                            status,
                            error_context.as_ref(),
                        );
                        if should_cross_account_failover(
                            &state,
                            parsed_policy_context.model.as_deref(),
                            sticky_key.as_deref(),
                            failover_deadline,
                            &tried_account_ids,
                            account.id,
                            retryable,
                        ) {
                            log_failover_decision(
                                UpstreamErrorSource::SsePrelude,
                                Some(account.id),
                                Some(status),
                                error_context.as_ref(),
                                reason_class,
                                recovery_action_label(error_context.as_ref()),
                                recovery_outcome_label(recovery_outcome),
                                "cross_account_failover",
                            );
                            release_billing_hold_for_cross_account_failover(
                                state.clone(),
                                &mut pending_billing_session,
                                &mut billing_session,
                                status,
                                error_context.as_ref(),
                                reason_class,
                            )
                            .await;
                            record_cross_account_failover_attempt(
                                &state,
                                &mut tried_account_ids,
                                account.id,
                                &mut did_cross_account_failover,
                            );
                            last_failure = Some((response, status, account.id));
                            break;
                        }

                        log_failover_decision(
                            UpstreamErrorSource::SsePrelude,
                            Some(account.id),
                            Some(status),
                            error_context.as_ref(),
                            reason_class,
                            recovery_action_label(error_context.as_ref()),
                            recovery_outcome_label(recovery_outcome),
                            "return_failure",
                        );
                        emit_request_log_event(
                            &state,
                            account.id,
                            principal.as_ref(),
                            &path,
                            method.as_str(),
                            status,
                            started,
                            true,
                            parsed_policy_context.request_id.as_deref(),
                            parsed_policy_context.model.as_deref(),
                            parsed_policy_context.requested_service_tier.as_deref(),
                        )
                        .await;
                        record_failover_exhausted_if_needed(&state, did_cross_account_failover);
                        release_billing_hold_best_effort(
                            state.clone(),
                            billing_session.take(),
                            Some(BillingReleaseFailureContext {
                                release_reason: Some(reason_class.to_string()),
                                upstream_status_code: Some(status.as_u16()),
                                upstream_error_code: error_context
                                    .as_ref()
                                    .and_then(|context| context.error_code.clone()),
                                failover_action: Some("return_failure".to_string()),
                                failover_reason_class: Some(reason_class.to_string()),
                                recovery_action: Some(
                                    recovery_action_label(error_context.as_ref()).to_string(),
                                ),
                                recovery_outcome: Some(
                                    recovery_outcome_label(recovery_outcome).to_string(),
                                ),
                                cross_account_failover_attempted: Some(did_cross_account_failover),
                            }),
                        )
                        .await;
                        return response;
                    }
                }
            }

            let mut non_stream_billing_settle: Option<BillingSettleResult> = None;
            let mut non_stream_billing_deferred: Option<(Uuid, UsageTokens)> = None;
            let mut non_stream_billing_failed: Option<(Uuid, UsageTokens)> = None;
                let mut non_stream_observed_usage: Option<UsageTokens> = None;
                let mut non_stream_estimated_usage: Option<UsageTokens> = None;
                let mut non_stream_effective_service_tier =
                    parsed_policy_context.requested_service_tier.clone();

            let (response, upstream_error_context, is_503_overloaded) = if status
                == StatusCode::SERVICE_UNAVAILABLE
            {
                let (response, error_context) = map_service_unavailable(
                    &state,
                    upstream_provider_label(&account.mode),
                    parsed_policy_context.detected_locale.as_str(),
                    parsed_policy_context.model.as_deref(),
                    &response_headers,
                    upstream_response,
                )
                .await;
                let is_overloaded = matches!(error_context.class, UpstreamErrorClass::Overloaded);
                (response, Some(error_context), is_overloaded)
            } else {
                let (mut response, error_context, body) = buffered_response(
                    &state,
                    upstream_provider_label(&account.mode),
                    parsed_policy_context.detected_locale.as_str(),
                    parsed_policy_context.model.as_deref(),
                    status,
                    &response_headers,
                    upstream_response,
                )
                .await;
                non_stream_observed_usage = extract_usage_tokens(&body);
                non_stream_effective_service_tier = extract_response_service_tier(&body)
                    .or_else(|| parsed_policy_context.requested_service_tier.clone());
                if non_stream_observed_usage.is_none() {
                    let estimated_input_tokens = parsed_policy_context
                        .estimated_input_tokens
                        .unwrap_or(0)
                        .max(0);
                    let estimated_output_tokens =
                        estimate_response_output_tokens(&body).unwrap_or(0).max(0);
                    if estimated_input_tokens > 0 || estimated_output_tokens > 0 {
                        non_stream_estimated_usage = Some(UsageTokens {
                            input_tokens: estimated_input_tokens,
                            cached_input_tokens: 0,
                            output_tokens: estimated_output_tokens,
                            reasoning_tokens: 0,
                        });
                    }
                }
                let mut models_cached: Option<CachedModelsResponse> = None;
                if is_models_request && status.is_success() {
                    let cached = cache_models_response(
                        state.as_ref(),
                        &response_headers,
                        &body,
                        models_cache_key.as_deref().unwrap_or_default(),
                    );
                    apply_models_cache_headers(&mut response, &cached, Instant::now());
                    models_cached = Some(cached);
                }
                let response = match settle_billing_if_needed(
                    state.clone(),
                    billing_session.as_ref(),
                    status,
                    &body,
                )
                .await
                {
                    Ok(outcome) => {
                        match outcome {
                            BillingSettleOutcome::NotNeeded => {}
                            BillingSettleOutcome::Settled(settle_result) => {
                                non_stream_billing_settle = Some(settle_result);
                                billing_session = None;
                            }
                            BillingSettleOutcome::DeferredEstimated {
                                authorization_id,
                                usage,
                            } => {
                                non_stream_billing_deferred = Some((authorization_id, usage));
                                billing_session = None;
                            }
                        }
                        response
                    }
                    Err(_error_response) => {
                        let failed_authorization_id = billing_session
                            .take()
                            .map(|session| session.authorization_id);
                        let failed_usage = non_stream_observed_usage.or(non_stream_estimated_usage);
                        if let (Some(authorization_id), Some(usage)) =
                            (failed_authorization_id, failed_usage)
                        {
                            non_stream_billing_failed = Some((authorization_id, usage));
                        }
                        response
                    }
                };
                let response = if let Some(cached) = models_cached.as_ref() {
                    if request_if_none_match_matches(&parts.headers, cached.etag.as_ref()) {
                        build_models_not_modified_response(cached, Instant::now())
                    } else {
                        response
                    }
                } else {
                    response
                };
                (response, error_context, false)
            };

            if status.is_success() {
                let (
                    billing_phase,
                    authorization_id,
                    capture_status,
                    input_tokens,
                    cached_input_tokens,
                    output_tokens,
                    reasoning_tokens,
                ) = if is_stream {
                    (
                        billing_session.as_ref().map(|_| "streaming_open"),
                        billing_session
                            .as_ref()
                            .map(|session| session.authorization_id),
                        None,
                        None,
                        None,
                        None,
                        None,
                    )
                } else if let Some(settle_result) = non_stream_billing_settle.as_ref() {
                    (
                        Some("released"),
                        Some(settle_result.authorization_id),
                        Some(settle_result.capture_status.as_str()),
                        Some(settle_result.input_tokens),
                        Some(settle_result.cached_input_tokens),
                        Some(settle_result.output_tokens),
                        Some(settle_result.reasoning_tokens),
                    )
                } else if let Some((authorization_id, usage)) = non_stream_billing_deferred {
                    (
                        Some("deferred_estimated"),
                        Some(authorization_id),
                        None,
                        Some(usage.input_tokens),
                        Some(usage.cached_input_tokens),
                        Some(usage.output_tokens),
                        Some(usage.reasoning_tokens),
                    )
                } else if let Some((authorization_id, usage)) = non_stream_billing_failed {
                    (
                        Some("released_failed"),
                        Some(authorization_id),
                        None,
                        Some(usage.input_tokens),
                        Some(usage.cached_input_tokens),
                        Some(usage.output_tokens),
                        Some(usage.reasoning_tokens),
                    )
                } else if let Some(usage) = non_stream_observed_usage {
                    (
                        None,
                        None,
                        None,
                        Some(usage.input_tokens),
                        Some(usage.cached_input_tokens),
                        Some(usage.output_tokens),
                        Some(usage.reasoning_tokens),
                    )
                } else if let Some(usage) = non_stream_estimated_usage {
                    (
                        None,
                        None,
                        None,
                        Some(usage.input_tokens),
                        Some(usage.cached_input_tokens),
                        Some(usage.output_tokens),
                        Some(usage.reasoning_tokens),
                    )
                } else {
                    (None, None, None, None, None, None, None)
                };
                emit_request_log_event_with_billing(
                    &state,
                    account.id,
                    principal.as_ref(),
                    &path,
                    method.as_str(),
                    status,
                    started,
                    is_stream,
                    parsed_policy_context.request_id.as_deref(),
                    parsed_policy_context.model.as_deref(),
                    non_stream_effective_service_tier.as_deref(),
                    billing_phase,
                    authorization_id,
                    capture_status,
                    input_tokens,
                    cached_input_tokens,
                    output_tokens,
                    reasoning_tokens,
                    (!is_stream).then_some(started.elapsed().as_millis() as u64),
                )
                .await;
                spawn_seen_ok_reports(&state, account.id, parsed_policy_context.model.clone());
                record_failover_success_if_needed(&state, did_cross_account_failover);
                return response;
            }

            spawn_failed_live_report(
                &state,
                account.id,
                parsed_policy_context.model.clone(),
                status,
                upstream_error_context.as_ref(),
            );
            let retryable = is_failover_retryable_error(
                UpstreamErrorSource::Http,
                status,
                upstream_error_context.as_ref(),
            );
            let same_account_retryable = should_retry_same_account_on_status(
                UpstreamErrorSource::Http,
                status,
                is_503_overloaded,
                same_account_retry_attempt,
                &state,
                upstream_error_context.as_ref(),
            );
            if state.enable_request_failover
                && retryable
                && same_account_retryable
                && Instant::now() < failover_deadline
            {
                log_failover_decision(
                    UpstreamErrorSource::Http,
                    Some(account.id),
                    Some(status),
                    upstream_error_context.as_ref(),
                    "retryable_status",
                    recovery_action_label(upstream_error_context.as_ref()),
                    "not_applied",
                    "retry_same_account",
                );
                same_account_retry_attempt += 1;
                record_same_account_retry(&state);
                tokio::time::sleep(state.retry_poll_interval).await;
                continue;
            }

            let should_failover_across_accounts = should_cross_account_failover(
                &state,
                parsed_policy_context.model.as_deref(),
                sticky_key.as_deref(),
                failover_deadline,
                &tried_account_ids,
                account.id,
                retryable,
            );
            let recovery_outcome = if should_failover_across_accounts {
                if upstream_error_context.is_some() {
                    let state_for_recovery = state.clone();
                    let recovery_context = upstream_error_context.clone();
                    tokio::spawn(async move {
                        let _ = apply_recovery_action(
                            state_for_recovery.as_ref(),
                            account.id,
                            recovery_context.as_ref(),
                        )
                        .await;
                    });
                }
                ProxyRecoveryOutcome::NotApplied
            } else {
                apply_recovery_action(&state, account.id, upstream_error_context.as_ref()).await
            };
            if let Some(ejection_ttl) = ejection_ttl_for_response(
                status,
                state.account_ejection_ttl,
                is_503_overloaded,
                upstream_error_context.as_ref(),
                recovery_outcome,
            ) {
                state.router.mark_unhealthy(account.id, ejection_ttl);
                let _ = state
                    .routing_cache
                    .set_unhealthy(account.id, ejection_ttl)
                    .await;
                if let Some(sticky_key) = sticky_key.as_deref() {
                    let _ = state.router.unbind_sticky(sticky_key);
                    let _ = state
                        .routing_cache
                        .delete_sticky_account_id(sticky_key)
                        .await;
                }
            }
            record_invalid_request_guard_failure(
                &state,
                principal.as_ref(),
                UpstreamErrorSource::Http,
                status,
                upstream_error_context.as_ref(),
            );

            if should_failover_across_accounts {
                log_failover_decision(
                    UpstreamErrorSource::Http,
                    Some(account.id),
                    Some(status),
                    upstream_error_context.as_ref(),
                    "retryable_status",
                    recovery_action_label(upstream_error_context.as_ref()),
                    recovery_outcome_label(recovery_outcome),
                    "cross_account_failover",
                );
                release_billing_hold_for_cross_account_failover(
                    state.clone(),
                    &mut pending_billing_session,
                    &mut billing_session,
                    status,
                    upstream_error_context.as_ref(),
                    "retryable_status",
                )
                .await;
                record_cross_account_failover_attempt(
                    &state,
                    &mut tried_account_ids,
                    account.id,
                    &mut did_cross_account_failover,
                );
                last_failure = Some((response, status, account.id));
                break;
            }

            log_failover_decision(
                UpstreamErrorSource::Http,
                Some(account.id),
                Some(status),
                upstream_error_context.as_ref(),
                "non_retryable_or_budget_exhausted",
                recovery_action_label(upstream_error_context.as_ref()),
                recovery_outcome_label(recovery_outcome),
                "return_failure",
            );
            emit_request_log_event(
                &state,
                account.id,
                principal.as_ref(),
                &path,
                method.as_str(),
                status,
                started,
                is_stream,
                parsed_policy_context.request_id.as_deref(),
                parsed_policy_context.model.as_deref(),
                parsed_policy_context.requested_service_tier.as_deref(),
            )
            .await;
            record_failover_exhausted_if_needed(&state, did_cross_account_failover);
            release_billing_hold_best_effort(
                state.clone(),
                billing_session.take(),
                Some(BillingReleaseFailureContext {
                    release_reason: Some("upstream_request_failed".to_string()),
                    upstream_status_code: Some(status.as_u16()),
                    upstream_error_code: upstream_error_context
                        .as_ref()
                        .and_then(|context| context.error_code.clone()),
                    failover_action: Some("return_failure".to_string()),
                    failover_reason_class: Some("non_retryable_or_budget_exhausted".to_string()),
                    recovery_action: Some(
                        recovery_action_label(upstream_error_context.as_ref()).to_string(),
                    ),
                    recovery_outcome: Some(recovery_outcome_label(recovery_outcome).to_string()),
                    cross_account_failover_attempted: Some(did_cross_account_failover),
                }),
            )
            .await;
            return response;
        }
    }
}

fn max_request_body_bytes_for_path(default_limit: usize, path: &str) -> usize {
    // Large Codex-specific endpoints may send a full trace or conversation snapshot as input.
    // Keep a stricter default limit for most requests, but allow these paths to accept larger payloads.
    const LARGE_LIMIT: usize = 64 * 1024 * 1024;
    match path {
        "/v1/responses/compact"
        | "/backend-api/codex/responses/compact"
        | "/v1/memories/trace_summarize" => default_limit.max(LARGE_LIMIT),
        _ => default_limit,
    }
}

fn build_models_cache_key(account_id: Uuid, upstream_url: &str) -> String {
    format!("{account_id}:{upstream_url}")
}

fn serve_models_from_cache(
    state: &AppState,
    request_headers: &HeaderMap,
    cache_key: &str,
) -> Option<Response> {
    let now = Instant::now();
    let cached = state
        .models_cache
        .read()
        .ok()
        .and_then(|guard| guard.get(cache_key).cloned())?;
    if cached.expires_at <= now {
        return None;
    }
    if request_if_none_match_matches(request_headers, cached.etag.as_ref()) {
        return Some(build_models_not_modified_response(&cached, now));
    }

    let mut response = Response::builder().status(StatusCode::OK);
    if let Some(headers) = response.headers_mut() {
        if let Some(content_type) = cached.content_type.as_deref() {
            if let Ok(value) = HeaderValue::from_str(content_type) {
                headers.insert(axum::http::header::CONTENT_TYPE, value);
            }
        }
        apply_models_cache_headers_to_map(headers, &cached, now);
    }
    Some(
        response
            .body(Body::from(cached.body))
            .unwrap_or_else(|_| Response::new(Body::from("internal response error"))),
    )
}

fn cache_models_response(
    state: &AppState,
    response_headers: &HeaderMap,
    body: &bytes::Bytes,
    cache_key: &str,
) -> CachedModelsResponse {
    let now = Instant::now();
    let etag = response_headers
        .get(ETAG)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| compute_strong_etag(body));
    let content_type = response_headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| Arc::<str>::from(value.to_string()));
    let cached = CachedModelsResponse {
        body: body.clone(),
        etag: Arc::<str>::from(etag),
        content_type,
        expires_at: now + Duration::from_secs(MODELS_CACHE_TTL_SEC),
    };

    if let Ok(mut guard) = state.models_cache.write() {
        guard.insert(cache_key.to_string(), cached.clone());
    }
    cached
}

fn request_if_none_match_matches(request_headers: &HeaderMap, etag: &str) -> bool {
    let Some(raw) = request_headers
        .get(IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    raw.split(',')
        .map(str::trim)
        .any(|item| item == "*" || item == etag)
}

fn build_models_not_modified_response(cached: &CachedModelsResponse, now: Instant) -> Response {
    let mut response = Response::builder().status(StatusCode::NOT_MODIFIED);
    if let Some(headers) = response.headers_mut() {
        apply_models_cache_headers_to_map(headers, cached, now);
    }
    response
        .body(Body::empty())
        .unwrap_or_else(|_| Response::new(Body::from("internal response error")))
}

fn apply_models_cache_headers(
    response: &mut Response,
    cached: &CachedModelsResponse,
    now: Instant,
) {
    let headers = response.headers_mut();
    apply_models_cache_headers_to_map(headers, cached, now);
}

fn apply_models_cache_headers_to_map(
    headers: &mut HeaderMap,
    cached: &CachedModelsResponse,
    now: Instant,
) {
    let max_age = cached.expires_at.saturating_duration_since(now).as_secs();
    if let Ok(value) = HeaderValue::from_str(cached.etag.as_ref()) {
        headers.insert(ETAG, value);
    }
    if let Ok(value) = HeaderValue::from_str(&format!("private, max-age={max_age}")) {
        headers.insert(CACHE_CONTROL, value);
    }
}

fn compute_strong_etag(body: &bytes::Bytes) -> String {
    let digest = Sha256::digest(body.as_ref());
    format!("\"{}\"", hex::encode(digest))
}

pub async fn proxy_websocket_handler(
    State(state): State<std::sync::Arc<AppState>>,
    ws_upgrade: Result<WebSocketUpgrade, WebSocketUpgradeRejection>,
    request: Request<Body>,
) -> Response {
    let ws_upgrade = match ws_upgrade {
        Ok(ws_upgrade) => ws_upgrade,
        Err(err) => {
            warn!(error = %err, "invalid websocket upgrade request");
            let locale = detect_request_locale(
                request.headers(),
                &bytes::Bytes::new(),
            );
            return localized_json_error_with_state(
                state.as_ref(),
                locale.as_str(),
                StatusCode::BAD_REQUEST,
                "invalid_websocket_upgrade",
                "request is not a valid websocket upgrade",
            );
        }
    };

    let principal = request.extensions().get::<ApiPrincipal>().cloned();
    let (parts, _) = request.into_parts();
    let path = parts.uri.path().to_string();
    let query = parts.uri.query().map(str::to_string);
    let request_method = parts.method.to_string();
    let requested_subprotocol = requested_websocket_subprotocol(&parts.headers);
    let sticky_key = sticky_session_key_from_headers(&parts.headers);
    let detected_locale = detect_request_locale(&parts.headers, &bytes::Bytes::new());
    let failover_deadline = Instant::now() + state.request_failover_wait;
    let mut tried_account_ids = HashSet::new();
    let mut last_failure: Option<Response> = None;
    let mut did_cross_account_failover = false;
    let mut forced_distinct_failover_round = false;

    if let Some(sticky_key) = sticky_key.as_deref() {
        if let Ok(Some(account_id)) = state.routing_cache.get_sticky_account_id(sticky_key).await {
            let _ = state.router.bind_sticky(sticky_key, account_id);
        }
    }

    loop {
        let alive_ring_account = if sticky_key.is_none() {
            pick_account_from_alive_ring(&state, &tried_account_ids, None).await
        } else {
            None
        };
        let account = match alive_ring_account.or_else(|| {
            state.router.pick_for_model(
                None,
                sticky_key.as_deref(),
                &tried_account_ids,
                state.sticky_prefer_non_conflicting,
            )
        }) {
            Some(account) => account,
            None => {
                if state.enable_request_failover
                    && !forced_distinct_failover_round
                    && !tried_account_ids.is_empty()
                    && state.router.enabled_total() >= MIN_DISTINCT_FAILOVER_ATTEMPTS
                {
                    forced_distinct_failover_round = true;
                    tried_account_ids.clear();
                    wait_for_route_update_or_deadline(&state, failover_deadline).await;
                    continue;
                }
                if state.enable_request_failover && Instant::now() < failover_deadline {
                    tried_account_ids.clear();
                    wait_for_route_update_or_deadline(&state, failover_deadline).await;
                    continue;
                }
                if let Some(response) = last_failure.take() {
                    record_failover_exhausted_if_needed(&state, did_cross_account_failover);
                    return response;
                }
                record_failover_exhausted_if_needed(&state, did_cross_account_failover);
                return localized_json_error_with_state(
                    state.as_ref(),
                    detected_locale.as_str(),
                    StatusCode::SERVICE_UNAVAILABLE,
                    "no_upstream_account",
                    "no enabled upstream account is available",
                );
            }
        };

        if state.shared_routing_cache_enabled {
            if let Ok(true) = state.routing_cache.is_unhealthy(account.id).await {
                state.router.mark_unhealthy(
                    account.id,
                    state.retry_poll_interval.max(Duration::from_millis(1)),
                );
                tried_account_ids.insert(account.id);
                continue;
            }
        }

        if let Some(sticky_key) = sticky_key.as_deref() {
            let _ = state
                .routing_cache
                .set_sticky_account_id(
                    sticky_key,
                    account.id,
                    Duration::from_secs(ROUTING_CACHE_STICKY_TTL_SEC),
                )
                .await;
        }

        let mut same_account_retry_attempt: u32 = 0;
        let mut omit_upstream_subprotocol = false;
        loop {
            let upstream_request = match build_upstream_websocket_request(
                &account.base_url,
                &account.mode,
                &account.bearer_token,
                account.chatgpt_account_id.as_deref(),
                &path,
                query.as_deref(),
                &parts.headers,
                !omit_upstream_subprotocol,
            ) {
                Ok(request) => request,
                Err(err) => {
                    warn!(error = %err, account_id = %account.id, "failed to build upstream websocket request");
                    mark_account_unhealthy_and_clear_sticky(
                        &state,
                        account.id,
                        sticky_key.as_deref(),
                        state.account_ejection_ttl,
                    )
                    .await;

                    let response = localized_json_error_with_state(
                        state.as_ref(),
                        detected_locale.as_str(),
                        StatusCode::BAD_GATEWAY,
                        "invalid_upstream_url",
                        "failed to build upstream URL",
                    );
                    if should_cross_account_failover(
                        &state,
                        None,
                        sticky_key.as_deref(),
                        failover_deadline,
                        &tried_account_ids,
                        account.id,
                        true,
                    ) {
                        record_cross_account_failover_attempt(
                            &state,
                            &mut tried_account_ids,
                            account.id,
                            &mut did_cross_account_failover,
                        );
                        last_failure = Some(response);
                        break;
                    }
                    record_failover_exhausted_if_needed(&state, did_cross_account_failover);
                    return response;
                }
            };

            let (upstream_socket, upstream_response) = match connect_async(upstream_request).await {
                Ok((upstream_socket, upstream_response)) => (upstream_socket, upstream_response),
                Err(err) => {
                    if !omit_upstream_subprotocol
                        && requested_subprotocol.is_some()
                        && is_upstream_no_subprotocol_error(&err)
                    {
                        warn!(
                            account_id = %account.id,
                            "upstream websocket omitted subprotocol; retrying same account without forwarding sec-websocket-protocol"
                        );
                        omit_upstream_subprotocol = true;
                        continue;
                    }

                    let mut status = StatusCode::BAD_GATEWAY;
                    let mut error_context: Option<UpstreamErrorContext> = None;
                    let mut response = localized_json_error_with_state(
                        state.as_ref(),
                        detected_locale.as_str(),
                        StatusCode::BAD_GATEWAY,
                        "upstream_websocket_connect_error",
                        "failed to connect upstream websocket",
                    );
                    let mut is_http_handshake_error = false;

                    let should_retry_same_account = match &err {
                        tokio_tungstenite::tungstenite::Error::Http(upstream_response) => {
                            is_http_handshake_error = true;
                            status = upstream_response.status();
                            let upstream_body =
                                upstream_response.body().clone().unwrap_or_default();
                            if status == StatusCode::UPGRADE_REQUIRED {
                                response = localized_json_error_with_state(
                                    state.as_ref(),
                                    detected_locale.as_str(),
                                    StatusCode::UPGRADE_REQUIRED,
                                    "websocket_upgrade_required",
                                    "upstream websocket upgrade is required",
                                );
                                should_retry_same_account_on_status(
                                    UpstreamErrorSource::WsHandshake,
                                    status,
                                    false,
                                    same_account_retry_attempt,
                                    &state,
                                    None,
                                )
                            } else {
                                if let Some(context) = build_upstream_error_context(
                                    status,
                                    upstream_response.headers(),
                                    &upstream_body,
                                ) {
                                    let (learned_response, learned_context) =
                                        apply_upstream_error_learning(
                                            &state,
                                            UpstreamErrorSource::WsHandshake,
                                            upstream_provider_label(&account.mode),
                                            detected_locale.as_str(),
                                            None,
                                            context,
                                        )
                                        .await;
                                    status = learned_context.status;
                                    response = learned_response;
                                    error_context = Some(learned_context);
                                }
                                should_retry_same_account_on_status(
                                    UpstreamErrorSource::WsHandshake,
                                    status,
                                    false,
                                    same_account_retry_attempt,
                                    &state,
                                    error_context.as_ref(),
                                )
                            }
                        }
                        _ => should_retry_same_account_on_transport(
                            same_account_retry_attempt,
                            &state,
                        ),
                    };

                    warn!(
                        error = %err,
                        account_id = %account.id,
                        upstream_status = ?status,
                        upstream_error_class = upstream_error_class_label(error_context.as_ref()),
                        "failed to connect upstream websocket"
                    );
                    if state.enable_request_failover
                        && should_retry_same_account
                        && Instant::now() < failover_deadline
                    {
                        log_failover_decision(
                            UpstreamErrorSource::WsHandshake,
                            Some(account.id),
                            Some(status),
                            error_context.as_ref(),
                            if is_http_handshake_error {
                                "websocket_handshake_error"
                            } else {
                                "transport_error"
                            },
                            recovery_action_label(error_context.as_ref()),
                            "not_applied",
                            "retry_same_account",
                        );
                        same_account_retry_attempt += 1;
                        record_same_account_retry(&state);
                        tokio::time::sleep(state.retry_poll_interval).await;
                        continue;
                    }

                    let failover_reason_class = if status == StatusCode::UPGRADE_REQUIRED {
                        "websocket_upgrade_required"
                    } else if is_http_handshake_error {
                        "websocket_handshake_error"
                    } else {
                        "transport_error"
                    };
                    let can_cross_account_failover = if is_http_handshake_error {
                        is_failover_retryable_error(
                            UpstreamErrorSource::WsHandshake,
                            status,
                            error_context.as_ref(),
                        )
                    } else {
                        true
                    };
                    let should_failover_across_accounts = should_cross_account_failover(
                        &state,
                        None,
                        sticky_key.as_deref(),
                        failover_deadline,
                        &tried_account_ids,
                        account.id,
                        can_cross_account_failover,
                    );
                    let recovery_outcome = if should_failover_across_accounts {
                        if error_context.is_some() {
                            let state_for_recovery = state.clone();
                            let recovery_context = error_context.clone();
                            tokio::spawn(async move {
                                let _ = apply_recovery_action(
                                    state_for_recovery.as_ref(),
                                    account.id,
                                    recovery_context.as_ref(),
                                )
                                .await;
                            });
                        }
                        ProxyRecoveryOutcome::NotApplied
                    } else {
                        apply_recovery_action(&state, account.id, error_context.as_ref()).await
                    };

                    let ejection_ttl = if is_http_handshake_error {
                        ejection_ttl_for_response(
                            status,
                            state.account_ejection_ttl,
                            false,
                            error_context.as_ref(),
                            recovery_outcome,
                        )
                    } else {
                        Some(state.account_ejection_ttl)
                    };
                    if let Some(ejection_ttl) = ejection_ttl {
                        mark_account_unhealthy_and_clear_sticky(
                            &state,
                            account.id,
                            sticky_key.as_deref(),
                            ejection_ttl,
                        )
                        .await;
                    }

                    if should_failover_across_accounts {
                        log_failover_decision(
                            UpstreamErrorSource::WsHandshake,
                            Some(account.id),
                            Some(status),
                            error_context.as_ref(),
                            failover_reason_class,
                            recovery_action_label(error_context.as_ref()),
                            recovery_outcome_label(recovery_outcome),
                            "cross_account_failover",
                        );
                        record_cross_account_failover_attempt(
                            &state,
                            &mut tried_account_ids,
                            account.id,
                            &mut did_cross_account_failover,
                        );
                        last_failure = Some(response);
                        break;
                    }

                    log_failover_decision(
                        UpstreamErrorSource::WsHandshake,
                        Some(account.id),
                        Some(status),
                        error_context.as_ref(),
                        failover_reason_class,
                        recovery_action_label(error_context.as_ref()),
                        recovery_outcome_label(recovery_outcome),
                        "return_failure",
                    );
                    record_failover_exhausted_if_needed(&state, did_cross_account_failover);
                    return response;
                }
            };

            let selected_subprotocol = selected_websocket_subprotocol(upstream_response.headers())
                .or_else(|| requested_subprotocol.clone());
            let ws_upgrade = if let Some(protocol) = selected_subprotocol {
                ws_upgrade.protocols([protocol])
            } else {
                ws_upgrade
            };
            let state_for_upgrade = state.clone();
            let sticky_key_for_upgrade = sticky_key.clone();
            let account_id_for_upgrade = account.id;
            let ws_usage_context = WsLogicalUsageConnectionContext {
                account_id: account.id,
                tenant_id: principal.as_ref().and_then(|item| item.tenant_id),
                api_key_id: principal.as_ref().and_then(|item| item.api_key_id),
                principal: principal.clone(),
                adapt_codex_responses_request: is_chatgpt_codex_profile(
                    &account.mode,
                    &account.base_url,
                ),
                request_headers: parts.headers.clone(),
                request_path: path.clone(),
                request_query: query.clone(),
                request_method: request_method.clone(),
                requested_subprotocol: requested_subprotocol.clone(),
                sticky_key: sticky_key.clone(),
            };
            spawn_seen_ok_reports(&state, account.id, None);
            record_failover_success_if_needed(&state, did_cross_account_failover);
            return ws_upgrade.on_upgrade(move |downstream_socket| async move {
                if let Err(err) = proxy_websocket_streams(
                    downstream_socket,
                    upstream_socket,
                    state_for_upgrade.clone(),
                    ws_usage_context,
                )
                .await
                {
                    let ProxyWebSocketStreamError::UpstreamClosed(close) = &err;
                    warn!(
                        account_id = %account_id_for_upgrade,
                        close_code = close.code,
                        close_reason = close.reason,
                        "upstream websocket closed"
                    );
                    if should_eject_account_for_websocket_close(close) {
                        mark_account_unhealthy_and_clear_sticky(
                            &state_for_upgrade,
                            account_id_for_upgrade,
                            sticky_key_for_upgrade.as_deref(),
                            auth_error_ejection_ttl(state_for_upgrade.account_ejection_ttl),
                        )
                        .await;
                    }
                }
            });
        }
    }
}

fn spawn_seen_ok_reports(state: &Arc<AppState>, account_id: Uuid, model_id: Option<String>) {
    if let Some(seen_ok_reporter) = state.seen_ok_reporter.clone() {
        tokio::spawn(async move {
            seen_ok_reporter
                .report_live_result(
                    account_id,
                    LiveResultReportRequest {
                        status: LiveResultReportStatus::Ok,
                        source: LiveResultReportSource::Passive,
                        status_code: Some(StatusCode::OK.as_u16()),
                        error_code: None,
                        error_message: None,
                        upstream_request_id: None,
                        model: model_id.clone(),
                    },
                )
                .await;
            seen_ok_reporter.report_seen_ok(account_id).await;
            if let Some(model_id) = model_id.as_deref() {
                seen_ok_reporter
                    .report_model_seen_ok(account_id, model_id)
                    .await;
            }
        });
    }
}

fn spawn_failed_live_report(
    state: &Arc<AppState>,
    account_id: Uuid,
    model_id: Option<String>,
    status: StatusCode,
    error_context: Option<&UpstreamErrorContext>,
) {
    if let Some(seen_ok_reporter) = state.seen_ok_reporter.clone() {
        let report = LiveResultReportRequest {
            status: LiveResultReportStatus::Failed,
            source: LiveResultReportSource::Passive,
            status_code: Some(status.as_u16()),
            error_code: error_context
                .and_then(|context| context.error_code.clone())
                .or_else(|| live_result_error_code_for_class(error_context.map(|context| context.class))),
            error_message: error_context
                .and_then(|context| context.error_message.clone())
                .or_else(|| error_context.and_then(|context| context.raw_error.clone())),
            upstream_request_id: error_context.and_then(|context| context.upstream_request_id.clone()),
            model: model_id,
        };
        tokio::spawn(async move {
            seen_ok_reporter.report_live_result(account_id, report).await;
        });
    }
}

fn live_result_error_code_for_class(class: Option<UpstreamErrorClass>) -> Option<String> {
    match class {
        Some(UpstreamErrorClass::TokenInvalidated) => Some("token_invalidated".to_string()),
        Some(UpstreamErrorClass::AuthExpired) => Some("auth_expired".to_string()),
        Some(UpstreamErrorClass::AccountDeactivated) => Some("account_deactivated".to_string()),
        Some(UpstreamErrorClass::QuotaExhausted) => Some("quota_exhausted".to_string()),
        Some(UpstreamErrorClass::RateLimited) => Some("rate_limited".to_string()),
        Some(UpstreamErrorClass::Overloaded) => Some("overloaded".to_string()),
        Some(UpstreamErrorClass::UpstreamUnavailable) => Some("upstream_unavailable".to_string()),
        Some(UpstreamErrorClass::TransientServer) => Some("transient_server".to_string()),
        Some(UpstreamErrorClass::NonRetryableClient) => Some("non_retryable_client".to_string()),
        Some(UpstreamErrorClass::Unknown) | None => None,
    }
}

async fn release_billing_hold_for_cross_account_failover(
    state: Arc<AppState>,
    pending_billing_session: &mut Option<PendingBillingSession>,
    billing_session: &mut Option<BillingSession>,
    status: StatusCode,
    error_context: Option<&UpstreamErrorContext>,
    reason_class: &str,
) {
    let released_session = billing_session.take();
    if released_session.is_some() {
        if let Some(pending_billing_session) = pending_billing_session.as_mut() {
            pending_billing_session.rotate_request_id_for_cross_account_failover();
        }
    }

    release_billing_hold_best_effort(
        state,
        released_session,
        Some(BillingReleaseFailureContext {
            release_reason: Some("cross_account_failover".to_string()),
            upstream_status_code: Some(status.as_u16()),
            upstream_error_code: error_context.and_then(|context| context.error_code.clone()),
            failover_action: Some("cross_account_failover".to_string()),
            failover_reason_class: Some(reason_class.to_string()),
            cross_account_failover_attempted: Some(true),
            ..Default::default()
        }),
    )
    .await;
}

async fn wait_for_route_update_or_deadline(state: &AppState, deadline: Instant) {
    let now = Instant::now();
    if now >= deadline {
        return;
    }
    let timeout = state
        .retry_poll_interval
        .min(deadline.saturating_duration_since(now))
        .max(Duration::from_millis(1));
    state.wait_for_route_update(timeout).await;
}

async fn pick_account_from_alive_ring(
    state: &Arc<AppState>,
    excluded_account_ids: &HashSet<Uuid>,
    model: Option<&str>,
) -> Option<UpstreamAccount> {
    let alive_ring = state.alive_ring_router.as_ref()?;
    let candidate_ids = match alive_ring.next_candidate_ids().await {
        Ok(ids) => ids,
        Err(err) => {
            warn!(error = %err, "failed to fetch alive ring candidates");
            return None;
        }
    };
    pick_alive_ring_candidate_from_ids(state, &candidate_ids, excluded_account_ids, model)
}

fn pick_alive_ring_candidate_from_ids(
    state: &Arc<AppState>,
    candidate_ids: &[Uuid],
    excluded_account_ids: &HashSet<Uuid>,
    model: Option<&str>,
) -> Option<UpstreamAccount> {
    for account_id in candidate_ids {
        if excluded_account_ids.contains(&account_id) {
            continue;
        }
        if !state.router.account_matches_model_route(*account_id, model) {
            continue;
        }
        if let Some(account) = state.router.pick_account(*account_id) {
            return Some(account);
        }
    }
    None
}

async fn mark_account_unhealthy_and_clear_sticky(
    state: &Arc<AppState>,
    account_id: Uuid,
    sticky_key: Option<&str>,
    ejection_ttl: Duration,
) {
    state.router.mark_unhealthy(account_id, ejection_ttl);
    let _ = state
        .routing_cache
        .set_unhealthy(account_id, ejection_ttl)
        .await;
    if let Some(sticky_key) = sticky_key {
        let _ = state.router.unbind_sticky(sticky_key);
        let _ = state
            .routing_cache
            .delete_sticky_account_id(sticky_key)
            .await;
    }
}

fn selected_websocket_subprotocol(headers: &HeaderMap) -> Option<String> {
    first_websocket_subprotocol(headers)
}

fn requested_websocket_subprotocol(headers: &HeaderMap) -> Option<String> {
    first_websocket_subprotocol(headers)
}

fn first_websocket_subprotocol(headers: &HeaderMap) -> Option<String> {
    headers
        .get(SEC_WEBSOCKET_PROTOCOL_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn is_upstream_no_subprotocol_error(error: &TungsteniteError) -> bool {
    matches!(
        error,
        TungsteniteError::Protocol(ProtocolError::SecWebSocketSubProtocolError(
            SubProtocolError::NoSubProtocol
        ))
    )
}

fn should_eject_account_for_websocket_close(close: &UpstreamWebSocketClose) -> bool {
    close.code == 1008 || close.reason.to_ascii_lowercase().contains("policy")
}

#[cfg(test)]
mod entry_route_selection_tests {
    use super::*;
    use crate::event::NoopEventSink;
    use crate::routing_cache::InMemoryRoutingCache;
    use crate::router::RoundRobinRouter;
    use crate::upstream_health::SeenOkReporter;
    use codex_pool_core::model::{
        AiErrorLearningSettings, CompiledModelRoutingPolicy, CompiledRoutingPlan,
        CompiledRoutingProfile, RoutingStrategy, UpstreamErrorTemplateRecord,
    };
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::atomic::AtomicU64;
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn account(label: &str) -> UpstreamAccount {
        UpstreamAccount {
            id: Uuid::new_v4(),
            label: label.to_string(),
            mode: UpstreamMode::CodexOauth,
            base_url: format!("https://{label}.example.com/backend-api/codex"),
            bearer_token: format!("{label}-token"),
            chatgpt_account_id: Some(format!("acct-{label}")),
            enabled: true,
            priority: 100,
            created_at: chrono::Utc::now(),
        }
    }

    fn compiled_route(model: &str, account_ids: Vec<Uuid>) -> CompiledRoutingPlan {
        CompiledRoutingPlan {
            version_id: Uuid::new_v4(),
            published_at: chrono::Utc::now(),
            trigger_reason: Some("test".to_string()),
            default_route: Vec::new(),
            policies: vec![CompiledModelRoutingPolicy {
                id: Uuid::new_v4(),
                name: "test-policy".to_string(),
                family: "test-family".to_string(),
                exact_models: vec![model.to_string()],
                model_prefixes: Vec::new(),
                fallback_segments: vec![CompiledRoutingProfile {
                    id: Uuid::new_v4(),
                    name: "primary".to_string(),
                    account_ids,
                }],
            }],
        }
    }

    fn test_state(accounts: Vec<UpstreamAccount>) -> Arc<AppState> {
        test_state_with_reporter(accounts, None)
    }

    fn test_state_with_reporter(
        accounts: Vec<UpstreamAccount>,
        seen_ok_reporter: Option<Arc<SeenOkReporter>>,
    ) -> Arc<AppState> {
        Arc::new(AppState {
            router: RoundRobinRouter::new(accounts),
            http_client: reqwest::Client::new(),
            control_plane_base_url: Some("http://127.0.0.1:8090".to_string()),
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
            seen_ok_reporter,
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
            ai_error_learning_settings: std::sync::RwLock::new(AiErrorLearningSettings::default()),
            approved_upstream_error_templates: std::sync::RwLock::new(
                HashMap::<String, UpstreamErrorTemplateRecord>::new(),
            ),
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
    async fn alive_ring_candidate_selection_respects_compiled_model_route() {
        let free = account("free");
        let paid = account("paid");
        let state = test_state(vec![free.clone(), paid.clone()]);
        state
            .router
            .replace_compiled_routing_plan(Some(compiled_route("gpt-5.4", vec![paid.id])));

        let picked = pick_alive_ring_candidate_from_ids(
            &state,
            &[free.id, paid.id],
            &HashSet::new(),
            Some("gpt-5.4"),
        )
        .expect("alive ring should skip disallowed free account");

        assert_eq!(picked.id, paid.id);
    }

    #[tokio::test]
    async fn spawn_seen_ok_reports_posts_passive_live_result_and_seen_ok() {
        let control_plane = MockServer::start().await;
        let account = account("passive-ok");
        Mock::given(method("POST"))
            .and(path(format!(
                "/internal/v1/upstream-accounts/{}/health/live-result",
                account.id
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
            .mount(&control_plane)
            .await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/internal/v1/upstream-accounts/{}/health/seen-ok",
                account.id
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
            .mount(&control_plane)
            .await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/internal/v1/upstream-accounts/{}/models/seen-ok",
                account.id
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
            .mount(&control_plane)
            .await;
        let reporter = SeenOkReporter::new(
            control_plane.uri(),
            Arc::<str>::from("cp-internal-test-token"),
            Duration::from_millis(250),
            Duration::from_secs(0),
        )
        .expect("reporter must build");
        let state = test_state_with_reporter(vec![account.clone()], Some(Arc::new(reporter)));

        spawn_seen_ok_reports(&state, account.id, Some("gpt-5.1-codex-mini".to_string()));
        tokio::time::sleep(Duration::from_millis(50)).await;

        let requests = control_plane.received_requests().await.unwrap();
        assert!(
            requests
                .iter()
                .any(|request| request.url.path().ends_with("/health/live-result")),
            "expected passive live-result request"
        );
        assert!(
            requests
                .iter()
                .any(|request| request.url.path().ends_with("/health/seen-ok")),
            "expected seen-ok request"
        );
        assert!(
            requests
                .iter()
                .any(|request| request.url.path().ends_with("/models/seen-ok")),
            "expected model seen-ok request"
        );
    }

    #[tokio::test]
    async fn spawn_failed_live_report_posts_passive_failure_payload() {
        let control_plane = MockServer::start().await;
        let account = account("passive-failed");
        Mock::given(method("POST"))
            .and(path(format!(
                "/internal/v1/upstream-accounts/{}/health/live-result",
                account.id
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
            .mount(&control_plane)
            .await;
        let reporter = SeenOkReporter::new(
            control_plane.uri(),
            Arc::<str>::from("cp-internal-test-token"),
            Duration::from_millis(250),
            Duration::from_secs(0),
        )
        .expect("reporter must build");
        let state = test_state_with_reporter(vec![account.clone()], Some(Arc::new(reporter)));
        let error_context = UpstreamErrorContext {
            upstream_status: StatusCode::PAYMENT_REQUIRED,
            status: StatusCode::FORBIDDEN,
            error_code: Some("deactivated_workspace".to_string()),
            error_message: Some("workspace is deactivated".to_string()),
            raw_error: Some(
                "{\"detail\":{\"code\":\"deactivated_workspace\"}}".to_string(),
            ),
            retry_after: None,
            upstream_request_id: Some("req-passive-failed".to_string()),
            class: UpstreamErrorClass::AccountDeactivated,
            learned_resolution: None,
        };

        spawn_failed_live_report(
            &state,
            account.id,
            Some("gpt-5.1-codex-mini".to_string()),
            StatusCode::PAYMENT_REQUIRED,
            Some(&error_context),
        );
        tokio::time::sleep(Duration::from_millis(50)).await;

        let requests = control_plane.received_requests().await.unwrap();
        let live_request = requests
            .iter()
            .find(|request| request.url.path().ends_with("/health/live-result"))
            .expect("expected passive live-result failure request");
        let body: serde_json::Value =
            serde_json::from_slice(&live_request.body).expect("body should be json");

        assert_eq!(body["status"], "failed");
        assert_eq!(body["source"], "passive");
        assert_eq!(body["status_code"], 402);
        assert_eq!(body["error_code"], "deactivated_workspace");
        assert_eq!(body["upstream_request_id"], "req-passive-failed");
    }
}
