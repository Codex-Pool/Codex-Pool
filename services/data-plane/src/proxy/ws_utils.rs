fn trim_ascii(raw: &[u8]) -> &[u8] {
    let mut start = 0usize;
    while start < raw.len() && raw[start].is_ascii_whitespace() {
        start += 1;
    }
    let mut end = raw.len();
    while end > start && raw[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    &raw[start..end]
}

fn response_with_bytes(status: StatusCode, headers: &HeaderMap, body: bytes::Bytes) -> Response {
    response_with_body(status, headers, Body::from(body))
}

fn response_with_body(status: StatusCode, headers: &HeaderMap, body: Body) -> Response {
    let mut response = Response::builder().status(status);
    if let Some(target_headers) = response.headers_mut() {
        for (name, value) in headers {
            if is_hop_by_hop_header(name) || *name == CONTENT_LENGTH {
                continue;
            }
            target_headers.insert(name, value.clone());
        }
    }

    response
        .body(body)
        .unwrap_or_else(|_| Response::new(Body::from("internal response error")))
}

#[derive(Debug)]
struct UpstreamWebSocketClose {
    code: u16,
    reason: String,
}

#[derive(Debug)]
enum ProxyWebSocketStreamError {
    UpstreamClosed(UpstreamWebSocketClose),
}

const WS_RESPONSE_COMPLETED_BILLING_PHASE: &str = "ws_response_completed";
const WS_LOGICAL_REQUEST_METHOD: &str = "POST";
const WS_BILLING_RELEASED_PHASE: &str = "released";
const WS_BILLING_RELEASED_FAILED_PHASE: &str = "released_failed";
const WS_BILLING_RELEASED_MISSING_USAGE_PHASE: &str = "released_missing_usage";
const WS_BILLING_RELEASED_INCOMPLETE_PHASE: &str = "released_incomplete";
const WS_BILLING_RELEASED_INTERRUPTED_PHASE: &str = "released_interrupted";
const WS_RELEASE_REASON_MISSING_USAGE: &str = "stream_usage_missing";
const WS_RELEASE_REASON_RESPONSE_INCOMPLETE: &str = "ws_response_incomplete";
const WS_RELEASE_REASON_UPSTREAM_ERROR: &str = "ws_upstream_error";
const WS_RELEASE_REASON_CONNECTION_CLOSED: &str = "ws_connection_closed";
const WS_RELEASE_REASON_UPSTREAM_CLOSED: &str = "ws_upstream_closed";

#[derive(Debug, Clone)]
struct WsLogicalUsageConnectionContext {
    account_id: Uuid,
    tenant_id: Option<Uuid>,
    api_key_id: Option<Uuid>,
    principal: Option<ApiPrincipal>,
    request_headers: HeaderMap,
    request_path: String,
    request_method: String,
}

#[derive(Debug, Clone)]
struct WsLogicalResponseSeed {
    request_id: Option<String>,
    response_id: Option<String>,
    model: Option<String>,
    started_at: Instant,
}

#[derive(Debug, Clone)]
struct WsTrackedResponse {
    seed: WsLogicalResponseSeed,
    billing_session: Option<BillingSession>,
}

#[derive(Debug, Clone)]
enum WsPendingBillingAction {
    Capture {
        billing_session: BillingSession,
        usage: Option<UsageTokens>,
    },
    Release {
        billing_session: BillingSession,
        release_reason: &'static str,
    },
}

#[derive(Debug, Default)]
struct WsLogicalResponseTracker {
    pending_requests: VecDeque<WsTrackedResponse>,
    active_by_response_id: std::collections::HashMap<String, WsTrackedResponse>,
    completed_response_ids: HashSet<String>,
    pending_billing_actions: VecDeque<WsPendingBillingAction>,
}

impl WsLogicalResponseTracker {
    fn register_logical_request(
        &mut self,
        seed: WsLogicalResponseSeed,
        billing_session: Option<BillingSession>,
    ) {
        let tracked = WsTrackedResponse {
            seed,
            billing_session,
        };
        if let Some(response_id) = tracked.seed.response_id.clone() {
            if self.completed_response_ids.contains(&response_id) {
                return;
            }
            self.active_by_response_id.insert(response_id, tracked);
        } else {
            self.pending_requests.push_back(tracked);
        }
    }

    #[cfg(test)]
    fn observe_downstream_text(&mut self, text: &str) {
        let Some(value) = parse_ws_json_text(text) else {
            return;
        };
        let Some(seed) = extract_ws_logical_request_seed(&value) else {
            return;
        };
        self.register_logical_request(seed, None);
    }

    fn observe_upstream_message(
        &mut self,
        message: &TungsteniteMessage,
        context: &WsLogicalUsageConnectionContext,
    ) -> Vec<RequestLogEvent> {
        if let TungsteniteMessage::Text(text) = message {
            return self.observe_upstream_text(text.as_ref(), context);
        }
        Vec::new()
    }

    fn observe_upstream_text(
        &mut self,
        text: &str,
        context: &WsLogicalUsageConnectionContext,
    ) -> Vec<RequestLogEvent> {
        let Some(value) = parse_ws_json_text(text) else {
            return Vec::new();
        };

        if is_ws_response_created_event(&value) {
            self.register_response_created(&value);
        }

        if is_ws_response_incomplete_event(&value) {
            self.release_response(&value, WS_RELEASE_REASON_RESPONSE_INCOMPLETE);
            return Vec::new();
        }

        if is_ws_error_event(&value) {
            self.release_response(&value, WS_RELEASE_REASON_UPSTREAM_ERROR);
            return Vec::new();
        }

        if is_ws_response_completed_event(&value) {
            if let Some(event) = self.complete_response(&value, context) {
                return vec![event];
            }
        }

        Vec::new()
    }

    fn register_response_created(&mut self, value: &Value) {
        let Some(response_id) = extract_ws_response_id(value) else {
            return;
        };
        if self.completed_response_ids.contains(&response_id) {
            return;
        }

        let mut tracked = self
            .active_by_response_id
            .remove(&response_id)
            .or_else(|| self.pending_requests.pop_front())
            .unwrap_or_else(|| WsTrackedResponse {
                seed: WsLogicalResponseSeed {
                    request_id: None,
                    response_id: Some(response_id.clone()),
                    model: None,
                    started_at: Instant::now(),
                },
                billing_session: None,
            });

        if tracked.seed.request_id.is_none() {
            tracked.seed.request_id =
                extract_ws_request_id(value).or_else(|| Some(response_id.clone()));
        }
        if tracked.seed.response_id.is_none() {
            tracked.seed.response_id = Some(response_id.clone());
        }
        if tracked.seed.model.is_none() {
            tracked.seed.model = extract_ws_model(value);
        }

        self.active_by_response_id.insert(response_id, tracked);
    }

    fn complete_response(
        &mut self,
        value: &Value,
        context: &WsLogicalUsageConnectionContext,
    ) -> Option<RequestLogEvent> {
        let response_id = extract_ws_response_id(value);
        if let Some(response_id) = response_id.as_ref() {
            if self.completed_response_ids.contains(response_id) {
                return None;
            }
        }

        let mut tracked = response_id
            .as_ref()
            .and_then(|item| self.active_by_response_id.remove(item))
            .or_else(|| self.pending_requests.pop_front())
            .unwrap_or_else(|| WsTrackedResponse {
                seed: WsLogicalResponseSeed {
                    request_id: None,
                    response_id: response_id.clone(),
                    model: None,
                    started_at: Instant::now(),
                },
                billing_session: None,
            });

        if tracked.seed.request_id.is_none() {
            tracked.seed.request_id = extract_ws_request_id(value).or_else(|| response_id.clone());
        }
        if tracked.seed.response_id.is_none() {
            tracked.seed.response_id = response_id.clone();
        }
        if tracked.seed.model.is_none() {
            tracked.seed.model = extract_ws_model(value);
        }

        let usage = extract_usage_tokens_from_value(value);
        if tracked.seed.request_id.is_none()
            && tracked.seed.response_id.is_none()
            && tracked.seed.model.is_none()
            && usage.is_none()
        {
            return None;
        }

        if let Some(response_id) = tracked.seed.response_id.as_ref() {
            self.completed_response_ids.insert(response_id.clone());
        }

        if let Some(billing_session) = tracked.billing_session {
            self.pending_billing_actions
                .push_back(WsPendingBillingAction::Capture {
                    billing_session,
                    usage,
                });
            return None;
        }

        Some(RequestLogEvent {
            id: Uuid::new_v4(),
            account_id: context.account_id,
            tenant_id: context.tenant_id,
            api_key_id: context.api_key_id,
            event_version: 2,
            path: context.request_path.clone(),
            method: context.request_method.clone(),
            status_code: StatusCode::OK.as_u16(),
            latency_ms: tracked.seed.started_at.elapsed().as_millis() as u64,
            is_stream: true,
            error_code: None,
            request_id: tracked.seed.request_id.or(tracked.seed.response_id.clone()),
            model: tracked.seed.model,
            input_tokens: usage.as_ref().map(|item| item.input_tokens),
            cached_input_tokens: usage.as_ref().map(|item| item.cached_input_tokens),
            output_tokens: usage.as_ref().map(|item| item.output_tokens),
            reasoning_tokens: usage.as_ref().map(|item| item.reasoning_tokens),
            first_token_latency_ms: None,
            billing_phase: Some(WS_RESPONSE_COMPLETED_BILLING_PHASE.to_string()),
            authorization_id: None,
            capture_status: None,
            created_at: chrono::Utc::now(),
        })
    }

    fn release_response(&mut self, value: &Value, release_reason: &'static str) {
        let response_id = extract_ws_response_id(value);
        if let Some(response_id) = response_id.as_ref() {
            if self.completed_response_ids.contains(response_id) {
                return;
            }
        }

        let mut tracked = response_id
            .as_ref()
            .and_then(|item| self.active_by_response_id.remove(item))
            .or_else(|| self.pending_requests.pop_front())
            .unwrap_or_else(|| WsTrackedResponse {
                seed: WsLogicalResponseSeed {
                    request_id: None,
                    response_id: response_id.clone(),
                    model: None,
                    started_at: Instant::now(),
                },
                billing_session: None,
            });

        if tracked.seed.request_id.is_none() {
            tracked.seed.request_id = extract_ws_request_id(value).or_else(|| response_id.clone());
        }
        if tracked.seed.response_id.is_none() {
            tracked.seed.response_id = response_id.clone();
        }
        if tracked.seed.model.is_none() {
            tracked.seed.model = extract_ws_model(value);
        }

        if let Some(response_id) = tracked.seed.response_id.as_ref() {
            self.completed_response_ids.insert(response_id.clone());
        }

        if let Some(billing_session) = tracked.billing_session {
            self.pending_billing_actions
                .push_back(WsPendingBillingAction::Release {
                    billing_session,
                    release_reason,
                });
        }
    }

    fn drain_pending_billing_actions(&mut self) -> Vec<WsPendingBillingAction> {
        self.pending_billing_actions.drain(..).collect()
    }

    fn take_unfinished_billing_sessions(&mut self) -> Vec<BillingSession> {
        let mut sessions = Vec::new();
        for tracked in self.pending_requests.drain(..) {
            if let Some(billing_session) = tracked.billing_session {
                sessions.push(billing_session);
            }
        }
        for (_, tracked) in self.active_by_response_id.drain() {
            if let Some(billing_session) = tracked.billing_session {
                sessions.push(billing_session);
            }
        }
        self.pending_billing_actions.clear();
        sessions
    }
}

impl std::fmt::Display for ProxyWebSocketStreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UpstreamClosed(close) => {
                write!(f, "upstream websocket closed code={} reason={}", close.code, close.reason)
            }
        }
    }
}

impl std::error::Error for ProxyWebSocketStreamError {}

async fn proxy_websocket_streams(
    downstream_socket: WebSocket,
    upstream_socket: UpstreamWebSocket,
    state: Arc<AppState>,
    ws_usage_context: WsLogicalUsageConnectionContext,
) -> Result<(), ProxyWebSocketStreamError> {
    let (downstream_sender, mut downstream_receiver) = downstream_socket.split();
    let (mut upstream_sender, mut upstream_receiver) = upstream_socket.split();
    let downstream_sender = Arc::new(tokio::sync::Mutex::new(downstream_sender));
    let tracker = std::sync::Arc::new(std::sync::Mutex::new(WsLogicalResponseTracker::default()));
    let downstream_tracker = tracker.clone();
    let upstream_tracker = tracker.clone();
    let downstream_state = state.clone();
    let upstream_state = state.clone();
    let downstream_sender_for_errors = downstream_sender.clone();
    let downstream_sender_for_upstream = downstream_sender.clone();

    let downstream_to_upstream = async {
        while let Some(message) = downstream_receiver.next().await {
            let Ok(message) = message else {
                break;
            };

            if let AxumWsMessage::Text(text) = &message {
                if let Some(value) = parse_ws_json_text(text.as_ref()) {
                    if let Some(seed) = extract_ws_logical_request_seed(&value) {
                        let parsed_policy_context =
                            parse_ws_request_policy_context(&ws_usage_context.request_headers, &value);
                        if let Err(error_response) = enforce_principal_request_policy(
                            ws_usage_context.principal.as_ref(),
                            &ws_usage_context.request_headers,
                            &parsed_policy_context,
                        ) {
                            let ws_error = ws_error_message_from_response(*error_response).await;
                            let send_result = {
                                let mut sender = downstream_sender_for_errors.lock().await;
                                sender.send(ws_error).await
                            };
                            if send_result.is_err() {
                                break;
                            }
                            continue;
                        }

                        let pending_billing_session = match build_pending_billing_session(
                            downstream_state.as_ref(),
                            ws_usage_context.principal.as_ref(),
                            &ws_usage_context.request_headers,
                            &parsed_policy_context,
                            &ws_usage_context.request_path,
                            WS_LOGICAL_REQUEST_METHOD,
                        )
                        .await
                        {
                            Ok(value) => value,
                            Err(error_response) => {
                                let ws_error = ws_error_message_from_response(*error_response).await;
                                let send_result = {
                                    let mut sender = downstream_sender_for_errors.lock().await;
                                    sender.send(ws_error).await
                                };
                                if send_result.is_err() {
                                    break;
                                }
                                continue;
                            }
                        };

                        let billing_session = if let Some(pending_session) =
                            pending_billing_session.as_ref()
                        {
                            match authorize_billing_session(
                                downstream_state.as_ref(),
                                pending_session,
                                ws_usage_context.account_id,
                                &ws_usage_context.request_path,
                                WS_LOGICAL_REQUEST_METHOD,
                                seed.started_at,
                                true,
                            )
                            .await
                            {
                                Ok(session) => Some(session),
                                Err(error_response) => {
                                    let ws_error =
                                        ws_error_message_from_response(*error_response).await;
                                    let send_result = {
                                        let mut sender = downstream_sender_for_errors.lock().await;
                                        sender.send(ws_error).await
                                    };
                                    if send_result.is_err() {
                                        break;
                                    }
                                    continue;
                                }
                            }
                        } else {
                            None
                        };

                        if let Ok(mut tracker) = downstream_tracker.lock() {
                            tracker.register_logical_request(seed, billing_session);
                        }
                    }
                }
            }

            let should_close = matches!(message, AxumWsMessage::Close(_));
            if upstream_sender
                .send(axum_message_to_tungstenite(message))
                .await
                .is_err()
            {
                break;
            }
            if should_close {
                break;
            }
        }
        let _ = upstream_sender.close().await;
        Ok::<(), ProxyWebSocketStreamError>(())
    };

    let upstream_to_downstream = async {
        let mut upstream_close: Option<UpstreamWebSocketClose> = None;
        while let Some(message) = upstream_receiver.next().await {
            let Ok(message) = message else {
                break;
            };
            let (pending_events, pending_billing_actions) =
                if let Ok(mut tracker) = upstream_tracker.lock() {
                    let events = tracker.observe_upstream_message(&message, &ws_usage_context);
                    let billing_actions = tracker.drain_pending_billing_actions();
                    (events, billing_actions)
                } else {
                    (Vec::new(), Vec::new())
                };
            for event in pending_events {
                upstream_state.event_sink.emit_request_log(event).await;
            }
            process_ws_pending_billing_actions(upstream_state.clone(), pending_billing_actions).await;
            let should_close = matches!(message, TungsteniteMessage::Close(_));
            if let TungsteniteMessage::Close(frame) = &message {
                let close = frame
                    .as_ref()
                    .map(|frame| UpstreamWebSocketClose {
                        code: u16::from(frame.code),
                        reason: frame.reason.to_string(),
                    })
                    .unwrap_or_else(|| UpstreamWebSocketClose {
                        code: 1000,
                        reason: String::new(),
                    });
                upstream_close = Some(close);
            }
            if let Some(mapped) = tungstenite_message_to_axum(message) {
                let send_result = {
                    let mut sender = downstream_sender.lock().await;
                    sender.send(mapped).await
                };
                if send_result.is_err() {
                    break;
                }
            }
            if should_close {
                break;
            }
        }
        let _ = downstream_sender_for_upstream.lock().await.close().await;
        if let Some(close) = upstream_close {
            return Err(ProxyWebSocketStreamError::UpstreamClosed(close));
        }
        Ok::<(), ProxyWebSocketStreamError>(())
    };

    let (downstream_to_upstream_result, upstream_to_downstream_result) =
        tokio::join!(downstream_to_upstream, upstream_to_downstream);

    let release_reason = match &upstream_to_downstream_result {
        Err(ProxyWebSocketStreamError::UpstreamClosed(_)) => WS_RELEASE_REASON_UPSTREAM_CLOSED,
        _ => WS_RELEASE_REASON_CONNECTION_CLOSED,
    };
    let lingering_billing_sessions = if let Ok(mut tracker) = tracker.lock() {
        tracker.take_unfinished_billing_sessions()
    } else {
        Vec::new()
    };
    for billing_session in lingering_billing_sessions {
        release_ws_billing_session(
            state.clone(),
            billing_session,
            release_reason,
            ws_billing_phase_for_release_reason(release_reason),
        )
        .await;
    }

    downstream_to_upstream_result?;
    upstream_to_downstream_result?;

    Ok(())
}

fn parse_ws_json_text(text: &str) -> Option<Value> {
    serde_json::from_str::<Value>(text).ok()
}

fn parse_ws_request_policy_context(
    headers: &HeaderMap,
    value: &Value,
) -> ParsedRequestPolicyContext {
    let request_id = extract_ws_request_id(value).or_else(|| {
        headers
            .get("x-request-id")
            .and_then(|item| item.to_str().ok())
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(ToString::to_string)
    });
    let request_value = value.get("response").unwrap_or(value);
    let model = request_value
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string);
    let estimated_input_tokens = estimate_request_input_tokens(request_value);
    ParsedRequestPolicyContext {
        model,
        stream: true,
        request_id,
        estimated_input_tokens,
        sticky_key_hint: None,
        session_key_hint: sticky_session_key_from_headers(headers).or_else(|| {
            request_value
                .get("metadata")
                .and_then(|meta| meta.get("session_id"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
        }),
    }
}

fn ws_billing_phase_for_release_reason(release_reason: &str) -> &'static str {
    match release_reason {
        WS_RELEASE_REASON_MISSING_USAGE => WS_BILLING_RELEASED_MISSING_USAGE_PHASE,
        WS_RELEASE_REASON_RESPONSE_INCOMPLETE | WS_RELEASE_REASON_UPSTREAM_ERROR => {
            WS_BILLING_RELEASED_INCOMPLETE_PHASE
        }
        _ => WS_BILLING_RELEASED_INTERRUPTED_PHASE,
    }
}

async fn ws_error_message_from_response(error_response: Response) -> AxumWsMessage {
    let (parts, body) = error_response.into_parts();
    let body_bytes = axum::body::to_bytes(body, 64 * 1024)
        .await
        .unwrap_or_default();
    let error = serde_json::from_slice::<Value>(&body_bytes)
        .ok()
        .and_then(|value| value.get("error").cloned())
        .unwrap_or_else(|| serde_json::json!(ErrorEnvelope::new(
            "internal_error",
            "request failed",
        )));
    AxumWsMessage::Text(
        serde_json::json!({
            "type": "error",
            "status": parts.status.as_u16(),
            "error": error,
        })
        .to_string()
        .into(),
    )
}

async fn process_ws_pending_billing_actions(
    state: Arc<AppState>,
    actions: Vec<WsPendingBillingAction>,
) {
    for action in actions {
        match action {
            WsPendingBillingAction::Capture {
                billing_session,
                usage,
            } => match usage {
                Some(usage_tokens) => {
                    match settle_authorized_billing(state.as_ref(), &billing_session, usage_tokens).await
                    {
                        Ok(settle_result) => {
                            emit_stream_request_log_event(
                                state.as_ref(),
                                &billing_session,
                                WS_BILLING_RELEASED_PHASE,
                                Some(settle_result.capture_status.as_str()),
                                Some(settle_result.input_tokens),
                                Some(settle_result.cached_input_tokens),
                                Some(settle_result.output_tokens),
                                Some(settle_result.reasoning_tokens),
                            )
                            .await;
                        }
                        Err(err) => {
                            warn!(
                                status = ?err.status(),
                                request_id = %billing_session.request_id,
                                authorization_id = %billing_session.authorization_id,
                                reserved_microcredits = billing_session.reserved_microcredits,
                                "websocket billing finalize failed"
                            );
                            emit_stream_request_log_event(
                                state.as_ref(),
                                &billing_session,
                                WS_BILLING_RELEASED_FAILED_PHASE,
                                None,
                                Some(usage_tokens.input_tokens),
                                Some(usage_tokens.cached_input_tokens),
                                Some(usage_tokens.output_tokens),
                                Some(usage_tokens.reasoning_tokens),
                            )
                            .await;
                        }
                    }
                }
                None => {
                    release_ws_billing_session(
                        state.clone(),
                        billing_session,
                        WS_RELEASE_REASON_MISSING_USAGE,
                        WS_BILLING_RELEASED_MISSING_USAGE_PHASE,
                    )
                    .await;
                }
            },
            WsPendingBillingAction::Release {
                billing_session,
                release_reason,
            } => {
                release_ws_billing_session(
                    state.clone(),
                    billing_session,
                    release_reason,
                    ws_billing_phase_for_release_reason(release_reason),
                )
                .await;
            }
        }
    }
}

async fn release_ws_billing_session(
    state: Arc<AppState>,
    billing_session: BillingSession,
    release_reason: &str,
    billing_phase: &str,
) {
    emit_stream_request_log_event(
        state.as_ref(),
        &billing_session,
        billing_phase,
        None,
        None,
        None,
        None,
        None,
    )
    .await;
    release_billing_hold_best_effort(
        state,
        Some(billing_session),
        Some(BillingReleaseFailureContext {
            release_reason: Some(release_reason.to_string()),
            failover_action: Some("return_failure".to_string()),
            failover_reason_class: Some(release_reason.to_string()),
            ..Default::default()
        }),
    )
    .await;
}

fn ws_string_at_path<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(str::trim).filter(|item| !item.is_empty())
}

fn extract_ws_event_type(value: &Value) -> Option<&str> {
    ws_string_at_path(value, &["type"])
}

fn extract_ws_response_id(value: &Value) -> Option<String> {
    ws_string_at_path(value, &["response", "id"])
        .or_else(|| ws_string_at_path(value, &["response_id"]))
        .map(ToString::to_string)
}

fn extract_ws_request_id(value: &Value) -> Option<String> {
    ws_string_at_path(value, &["request_id"])
        .or_else(|| ws_string_at_path(value, &["client_request_id"]))
        .or_else(|| ws_string_at_path(value, &["event_id"]))
        .map(ToString::to_string)
}

fn extract_ws_model(value: &Value) -> Option<String> {
    ws_string_at_path(value, &["response", "model"])
        .or_else(|| ws_string_at_path(value, &["model"]))
        .map(ToString::to_string)
}

fn extract_ws_logical_request_seed(value: &Value) -> Option<WsLogicalResponseSeed> {
    let event_type = extract_ws_event_type(value)?;
    if event_type != "response.create" && !event_type.ends_with(".create") {
        return None;
    }

    Some(WsLogicalResponseSeed {
        request_id: extract_ws_request_id(value),
        response_id: extract_ws_response_id(value),
        model: extract_ws_model(value),
        started_at: Instant::now(),
    })
}

fn is_ws_response_created_event(value: &Value) -> bool {
    matches!(extract_ws_event_type(value), Some("response.created"))
}

fn is_ws_response_completed_event(value: &Value) -> bool {
    matches!(extract_ws_event_type(value), Some("response.completed" | "response.done"))
}

fn is_ws_response_incomplete_event(value: &Value) -> bool {
    matches!(extract_ws_event_type(value), Some("response.incomplete"))
}

fn is_ws_error_event(value: &Value) -> bool {
    matches!(extract_ws_event_type(value), Some("error"))
}

fn axum_message_to_tungstenite(message: AxumWsMessage) -> TungsteniteMessage {
    match message {
        AxumWsMessage::Text(text) => TungsteniteMessage::Text(text.to_string().into()),
        AxumWsMessage::Binary(bytes) => TungsteniteMessage::Binary(bytes),
        AxumWsMessage::Ping(payload) => TungsteniteMessage::Ping(payload),
        AxumWsMessage::Pong(payload) => TungsteniteMessage::Pong(payload),
        AxumWsMessage::Close(frame) => {
            TungsteniteMessage::Close(frame.map(axum_close_frame_to_tungstenite))
        }
    }
}

fn tungstenite_message_to_axum(message: TungsteniteMessage) -> Option<AxumWsMessage> {
    match message {
        TungsteniteMessage::Text(text) => Some(AxumWsMessage::Text(text.to_string().into())),
        TungsteniteMessage::Binary(bytes) => Some(AxumWsMessage::Binary(bytes)),
        TungsteniteMessage::Ping(payload) => Some(AxumWsMessage::Ping(payload)),
        TungsteniteMessage::Pong(payload) => Some(AxumWsMessage::Pong(payload)),
        TungsteniteMessage::Close(frame) => Some(AxumWsMessage::Close(
            frame.map(tungstenite_close_frame_to_axum),
        )),
        TungsteniteMessage::Frame(_) => None,
    }
}

fn axum_close_frame_to_tungstenite(frame: AxumCloseFrame) -> TungsteniteCloseFrame {
    TungsteniteCloseFrame {
        code: CloseCode::from(frame.code),
        reason: frame.reason.to_string().into(),
    }
}

fn tungstenite_close_frame_to_axum(frame: TungsteniteCloseFrame) -> AxumCloseFrame {
    AxumCloseFrame {
        code: frame.code.into(),
        reason: frame.reason.to_string().into(),
    }
}

fn json_error(status: StatusCode, code: &str, message: &str) -> Response {
    let payload = serde_json::to_vec(&ErrorEnvelope::new(code, message)).unwrap_or_default();
    let mut response = Response::builder().status(status);
    if let Some(headers) = response.headers_mut() {
        headers.insert(
            axum::http::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
    }

    response
        .body(Body::from(payload))
        .unwrap_or_else(|_| Response::new(Body::from("internal response error")))
}

fn is_body_too_large_error(err: &axum::Error) -> bool {
    let lowered = err.to_string().to_ascii_lowercase();
    lowered.contains("length limit")
        || lowered.contains("body too large")
        || lowered.contains("payload too large")
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::HeaderMap;
    use axum::http::HeaderName;
    use axum::http::StatusCode;
    use bytes::Bytes;
    use codex_pool_core::model::UpstreamMode;
    use std::time::Duration;

    use super::{
        build_upstream_url, build_upstream_ws_url, compose_upstream_path, ejection_ttl_for_status,
        ensure_client_version_query, extract_upstream_error_code, is_body_too_large_error,
        is_compatibility_passthrough_header, is_websocket_passthrough_header,
        parse_request_policy_context, recovery_action_for_upstream_error_code,
        sticky_session_key_from_headers, WsLogicalResponseTracker,
        WsLogicalUsageConnectionContext,
        ProxyRecoveryAction,
    };
    use uuid::Uuid;

    #[test]
    fn builds_upstream_url_with_base_path() {
        let url = build_upstream_url(
            "https://chatgpt.com/backend-api/codex",
            &UpstreamMode::ChatGptSession,
            "/v1/responses",
            Some("a=1"),
        )
        .unwrap();

        assert_eq!(url, "https://chatgpt.com/backend-api/codex/responses?a=1");
    }

    #[test]
    fn treats_openai_beta_and_subagent_as_compatibility_headers() {
        let openai_beta = HeaderName::from_static("openai-beta");
        let subagent = HeaderName::from_static("x-openai-subagent");
        let turn_state = HeaderName::from_static("x-codex-turn-state");
        let turn_metadata = HeaderName::from_static("x-codex-turn-metadata");
        let beta_features = HeaderName::from_static("x-codex-beta-features");
        let session_id = HeaderName::from_static("session_id");
        let conversation_id = HeaderName::from_static("conversation_id");
        let x_session_id = HeaderName::from_static("x-session-id");

        assert!(is_compatibility_passthrough_header(&openai_beta));
        assert!(is_compatibility_passthrough_header(&subagent));
        assert!(is_compatibility_passthrough_header(&turn_state));
        assert!(is_compatibility_passthrough_header(&turn_metadata));
        assert!(is_compatibility_passthrough_header(&beta_features));
        assert!(is_compatibility_passthrough_header(&session_id));
        assert!(is_compatibility_passthrough_header(&conversation_id));
        assert!(is_compatibility_passthrough_header(&x_session_id));
    }

    #[test]
    fn builds_upstream_websocket_url_with_base_path() {
        let url = build_upstream_ws_url(
            "https://chatgpt.com/backend-api/codex",
            &UpstreamMode::ChatGptSession,
            "/v1/responses",
            Some("a=1"),
        )
        .unwrap();

        assert_eq!(
            url.as_str(),
            "wss://chatgpt.com/backend-api/codex/responses?a=1"
        );
    }

    #[test]
    fn avoids_duplicate_base_path_when_client_path_already_prefixed() {
        let path = compose_upstream_path("/backend-api/codex", "/backend-api/codex/responses");
        assert_eq!(path, "/backend-api/codex/responses");
    }

    #[test]
    fn builds_upstream_url_without_duplicate_backend_api_prefix() {
        let url = build_upstream_url(
            "https://chatgpt.com/backend-api/codex",
            &UpstreamMode::ChatGptSession,
            "/backend-api/codex/responses",
            None,
        )
        .unwrap();

        assert_eq!(url, "https://chatgpt.com/backend-api/codex/responses");
    }

    #[test]
    fn builds_upstream_websocket_url_without_duplicate_backend_api_prefix() {
        let url = build_upstream_ws_url(
            "https://chatgpt.com/backend-api/codex",
            &UpstreamMode::ChatGptSession,
            "/backend-api/codex/responses",
            None,
        )
        .unwrap();

        assert_eq!(
            url.as_str(),
            "wss://chatgpt.com/backend-api/codex/responses"
        );
    }

    #[test]
    fn treats_session_id_and_x_codex_as_websocket_passthrough_headers() {
        let session_id = HeaderName::from_static("session_id");
        let conversation_id = HeaderName::from_static("conversation_id");
        let x_session_id = HeaderName::from_static("x-session-id");
        let codex_state = HeaderName::from_static("x-codex-turn-state");

        assert!(is_websocket_passthrough_header(&session_id, true));
        assert!(is_websocket_passthrough_header(&conversation_id, true));
        assert!(is_websocket_passthrough_header(&x_session_id, true));
        assert!(is_websocket_passthrough_header(&codex_state, true));
    }

    #[test]
    fn appends_client_version_query_for_codex_models_when_missing() {
        let query = ensure_client_version_query(Some("a=1"));
        assert!(query.contains("a=1"));
        assert!(query.contains("client_version=0.1.0"));
    }

    #[test]
    fn keeps_existing_client_version_query_for_codex_models() {
        let query = ensure_client_version_query(Some("client_version=9.9.9&a=1"));
        assert_eq!(query, "client_version=9.9.9&a=1");
    }

    #[test]
    fn keeps_openai_mode_path_unchanged_even_with_codex_base_path() {
        let url = build_upstream_url(
            "https://chatgpt.com/backend-api/codex",
            &UpstreamMode::OpenAiApiKey,
            "/v1/responses",
            None,
        )
        .unwrap();

        assert_eq!(url, "https://chatgpt.com/backend-api/codex/v1/responses");
    }

    #[test]
    fn applies_layered_ejection_ttl_by_status_code() {
        let base = Duration::from_secs(30);

        assert_eq!(
            ejection_ttl_for_status(StatusCode::TOO_MANY_REQUESTS, base, false),
            Some(Duration::from_secs(30))
        );
        assert_eq!(
            ejection_ttl_for_status(StatusCode::UNAUTHORIZED, base, false),
            Some(Duration::from_secs(300))
        );
        assert_eq!(
            ejection_ttl_for_status(StatusCode::INTERNAL_SERVER_ERROR, base, false),
            Some(Duration::from_secs(10))
        );
        assert_eq!(
            ejection_ttl_for_status(StatusCode::SERVICE_UNAVAILABLE, base, true),
            Some(Duration::from_secs(30))
        );
        assert_eq!(
            ejection_ttl_for_status(StatusCode::BAD_REQUEST, base, false),
            None
        );
    }

    #[test]
    fn extracts_sticky_session_key_from_session_or_conversation_header() {
        let mut headers = HeaderMap::new();
        headers.insert("session_id", "session-abc".parse().unwrap());
        assert_eq!(
            sticky_session_key_from_headers(&headers).as_deref(),
            Some("session-abc")
        );

        let mut headers = HeaderMap::new();
        headers.insert("conversation_id", "conv-123".parse().unwrap());
        assert_eq!(
            sticky_session_key_from_headers(&headers).as_deref(),
            Some("conv-123")
        );

        let mut headers = HeaderMap::new();
        headers.insert("x-session-id", "x-session-xyz".parse().unwrap());
        assert_eq!(
            sticky_session_key_from_headers(&headers).as_deref(),
            Some("x-session-xyz")
        );

        let mut headers = HeaderMap::new();
        headers.insert("x-codex-turn-state", "turn-state-1".parse().unwrap());
        assert_eq!(
            sticky_session_key_from_headers(&headers).as_deref(),
            Some("turn-state-1")
        );
    }

    #[test]
    fn parses_zstd_compressed_request_body_for_policy_context() {
        let json = br#"{"model":"gpt-4.1-mini","stream":true,"prompt_cache_key":"conv-1","input":"hello"}"#;
        let compressed =
            zstd::stream::encode_all(std::io::Cursor::new(json.as_slice()), 3).unwrap();
        let body = Bytes::from(compressed);
        let mut headers = HeaderMap::new();
        headers.insert("content-encoding", "zstd".parse().unwrap());

        let context = parse_request_policy_context(&headers, &body);
        assert_eq!(context.model.as_deref(), Some("gpt-4.1-mini"));
        assert!(context.stream);
        assert_eq!(context.sticky_key_hint.as_deref(), Some("conv-1"));
        assert!(context.estimated_input_tokens.is_some());
    }

    #[test]
    fn extracts_upstream_error_code_from_standard_error_payload() {
        let body = br#"{"error":{"code":"token_invalidated","message":"invalid token"}}"#;
        assert_eq!(
            extract_upstream_error_code(body),
            Some("token_invalidated".to_string())
        );
    }

    #[test]
    fn extracts_upstream_error_code_from_top_level_code() {
        let body = br#"{"code":"account_deactivated"}"#;
        assert_eq!(
            extract_upstream_error_code(body),
            Some("account_deactivated".to_string())
        );
    }

    #[test]
    fn returns_none_for_non_json_body() {
        let body = b"not-json";
        assert_eq!(extract_upstream_error_code(body), None);
    }

    #[test]
    fn maps_recovery_actions_for_known_error_codes() {
        assert_eq!(
            recovery_action_for_upstream_error_code(Some("token_invalidated")),
            Some(ProxyRecoveryAction::RotateRefreshToken)
        );
        assert_eq!(
            recovery_action_for_upstream_error_code(Some("account_deactivated")),
            Some(ProxyRecoveryAction::DisableAccount)
        );
        assert_eq!(recovery_action_for_upstream_error_code(Some("other")), None);
        assert_eq!(recovery_action_for_upstream_error_code(None), None);
    }

    #[tokio::test]
    async fn classifies_length_limit_errors_as_payload_too_large() {
        let err = axum::body::to_bytes(Body::from(vec![0_u8; 16]), 8)
            .await
            .expect_err("expected length limit error");
        assert!(is_body_too_large_error(&err));
    }

    fn ws_usage_test_context() -> WsLogicalUsageConnectionContext {
        WsLogicalUsageConnectionContext {
            account_id: Uuid::nil(),
            tenant_id: None,
            api_key_id: None,
            principal: None,
            request_headers: HeaderMap::new(),
            request_path: "/v1/responses".to_string(),
            request_method: "GET".to_string(),
        }
    }

    #[test]
    fn ws_logical_usage_records_completed_response() {
        let mut tracker = WsLogicalResponseTracker::default();
        tracker.observe_downstream_text(
            r#"{"type":"response.create","request_id":"req-1","response":{"model":"gpt-5.4"}}"#,
        );

        assert!(tracker
            .observe_upstream_text(
                r#"{"type":"response.created","response":{"id":"resp-1","model":"gpt-5.4"}}"#,
                &ws_usage_test_context(),
            )
            .is_empty());

        let events = tracker.observe_upstream_text(
            r#"{"type":"response.completed","response":{"id":"resp-1","usage":{"input_tokens":11,"output_tokens":7}}}"#,
            &ws_usage_test_context(),
        );

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].request_id.as_deref(), Some("req-1"));
        assert_eq!(events[0].model.as_deref(), Some("gpt-5.4"));
        assert_eq!(events[0].input_tokens, Some(11));
        assert_eq!(events[0].output_tokens, Some(7));
        assert_eq!(events[0].billing_phase.as_deref(), Some("ws_response_completed"));
    }

    #[test]
    fn ws_logical_usage_records_multiple_completed_responses_in_one_session() {
        let mut tracker = WsLogicalResponseTracker::default();
        tracker.observe_downstream_text(
            r#"{"type":"response.create","request_id":"req-1","response":{"model":"gpt-5.4"}}"#,
        );
        tracker.observe_downstream_text(
            r#"{"type":"response.create","request_id":"req-2","response":{"model":"gpt-5.4-mini"}}"#,
        );

        assert!(tracker
            .observe_upstream_text(
                r#"{"type":"response.created","response":{"id":"resp-1","model":"gpt-5.4"}}"#,
                &ws_usage_test_context(),
            )
            .is_empty());
        assert!(tracker
            .observe_upstream_text(
                r#"{"type":"response.created","response":{"id":"resp-2","model":"gpt-5.4-mini"}}"#,
                &ws_usage_test_context(),
            )
            .is_empty());

        let first = tracker.observe_upstream_text(
            r#"{"type":"response.completed","response":{"id":"resp-1","usage":{"input_tokens":5,"output_tokens":3}}}"#,
            &ws_usage_test_context(),
        );
        let second = tracker.observe_upstream_text(
            r#"{"type":"response.completed","response":{"id":"resp-2","usage":{"input_tokens":9,"output_tokens":4}}}"#,
            &ws_usage_test_context(),
        );

        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 1);
        assert_eq!(first[0].request_id.as_deref(), Some("req-1"));
        assert_eq!(second[0].request_id.as_deref(), Some("req-2"));
    }

    #[test]
    fn ws_logical_usage_ignores_unfinished_response() {
        let mut tracker = WsLogicalResponseTracker::default();
        tracker.observe_downstream_text(
            r#"{"type":"response.create","request_id":"req-1","response":{"model":"gpt-5.4"}}"#,
        );
        let events = tracker.observe_upstream_text(
            r#"{"type":"response.created","response":{"id":"resp-1","model":"gpt-5.4"}}"#,
            &ws_usage_test_context(),
        );
        assert!(events.is_empty());
    }

    #[test]
    fn ws_logical_usage_records_completion_without_usage_tokens() {
        let mut tracker = WsLogicalResponseTracker::default();
        tracker.observe_downstream_text(
            r#"{"type":"response.create","request_id":"req-1","response":{"model":"gpt-5.4"}}"#,
        );
        assert!(tracker
            .observe_upstream_text(
                r#"{"type":"response.created","response":{"id":"resp-1","model":"gpt-5.4"}}"#,
                &ws_usage_test_context(),
            )
            .is_empty());

        let events = tracker.observe_upstream_text(
            r#"{"type":"response.completed","response":{"id":"resp-1"}}"#,
            &ws_usage_test_context(),
        );

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].request_id.as_deref(), Some("req-1"));
        assert_eq!(events[0].model.as_deref(), Some("gpt-5.4"));
        assert_eq!(events[0].input_tokens, None);
        assert_eq!(events[0].output_tokens, None);
    }
}
