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
    response_with_body_with_rate_limit_policy(status, headers, body, false)
}

fn response_with_body_with_rate_limit_policy(
    status: StatusCode,
    headers: &HeaderMap,
    body: Body,
    preserve_private_rate_limit_headers: bool,
) -> Response {
    let mut response = Response::builder().status(status);
    if let Some(target_headers) = response.headers_mut() {
        for (name, value) in headers {
            if is_hop_by_hop_header(name)
                || *name == CONTENT_LENGTH
                || is_upstream_private_rate_limit_header(name)
            {
                continue;
            }
            target_headers.insert(name, value.clone());
        }
        if preserve_private_rate_limit_headers {
            apply_normalized_private_rate_limit_headers(target_headers, headers);
        }
    }

    response
        .body(body)
        .unwrap_or_else(|_| Response::new(Body::from("internal response error")))
}

fn is_upstream_private_rate_limit_header(name: &HeaderName) -> bool {
    let header = name.as_str().to_ascii_lowercase();
    if !header.starts_with("x-codex-") {
        return false;
    }

    header.contains("-primary-")
        || header.contains("-secondary-")
        || header.starts_with("x-codex-credits-")
        || header.ends_with("-limit-name")
        || header == "x-codex-promo-message"
}

fn apply_normalized_private_rate_limit_headers(
    target_headers: &mut HeaderMap,
    source_headers: &HeaderMap,
) {
    let Some(snapshot) = observed_rate_limits_from_headers(source_headers)
        .into_iter()
        .next()
        .map(rewrite_downstream_rate_limit_snapshot)
    else {
        return;
    };

    let fallback_limit_name = header_string(source_headers, "x-codex-limit-name");
    if let Some(limit_name) = snapshot
        .limit_name
        .as_deref()
        .or(fallback_limit_name.as_deref())
    {
        insert_private_rate_limit_header(target_headers, "x-codex-limit-name", limit_name);
    }
    if let Some(primary) = snapshot.primary.as_ref() {
        insert_private_rate_limit_header(
            target_headers,
            "x-codex-primary-used-percent",
            &primary.used_percent.to_string(),
        );
        if let Some(window_minutes) = primary.window_minutes {
            insert_private_rate_limit_header(
                target_headers,
                "x-codex-primary-window-minutes",
                &window_minutes.to_string(),
            );
        }
        if let Some(resets_at) = primary.resets_at {
            insert_private_rate_limit_header(
                target_headers,
                "x-codex-primary-reset-at",
                &resets_at.timestamp().to_string(),
            );
        }
    }
    if let Some(secondary) = snapshot.secondary.as_ref() {
        insert_private_rate_limit_header(
            target_headers,
            "x-codex-secondary-used-percent",
            &secondary.used_percent.to_string(),
        );
        if let Some(window_minutes) = secondary.window_minutes {
            insert_private_rate_limit_header(
                target_headers,
                "x-codex-secondary-window-minutes",
                &window_minutes.to_string(),
            );
        }
        if let Some(resets_at) = secondary.resets_at {
            insert_private_rate_limit_header(
                target_headers,
                "x-codex-secondary-reset-at",
                &resets_at.timestamp().to_string(),
            );
        }
    }
    if let Some(credits) = snapshot.credits.as_ref() {
        insert_private_rate_limit_header(
            target_headers,
            "x-codex-credits-has-credits",
            if credits.has_credits { "true" } else { "false" },
        );
        insert_private_rate_limit_header(
            target_headers,
            "x-codex-credits-unlimited",
            if credits.unlimited { "true" } else { "false" },
        );
        if let Some(balance) = credits.balance.as_deref() {
            insert_private_rate_limit_header(target_headers, "x-codex-credits-balance", balance);
        }
    }
}

fn insert_private_rate_limit_header(
    target_headers: &mut HeaderMap,
    name: &'static str,
    value: &str,
) {
    if let Ok(parsed) = HeaderValue::from_str(value) {
        target_headers.insert(HeaderName::from_static(name), parsed);
    }
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
const WS_RELEASE_REASON_RESPONSE_FAILED: &str = "ws_response_failed";
const WS_RELEASE_REASON_UPSTREAM_ERROR: &str = "ws_upstream_error";
const WS_RELEASE_REASON_CONNECTION_CLOSED: &str = "ws_connection_closed";
const WS_RELEASE_REASON_UPSTREAM_CLOSED: &str = "ws_upstream_closed";

#[derive(Debug, Clone)]
struct WsLogicalUsageConnectionContext {
    account_id: Uuid,
    tenant_id: Option<Uuid>,
    api_key_id: Option<Uuid>,
    principal: Option<ApiPrincipal>,
    adapt_codex_responses_request: bool,
    preserve_private_rate_limit_events: bool,
    request_headers: HeaderMap,
    request_path: String,
    request_query: Option<String>,
    request_method: String,
    requested_subprotocol: Option<String>,
    sticky_key: Option<String>,
}

#[derive(Debug, Clone)]
struct WsLogicalResponseSeed {
    request_id: Option<String>,
    response_id: Option<String>,
    model: Option<String>,
    requested_service_tier: Option<String>,
    started_at: Instant,
}

#[derive(Debug, Clone)]
struct WsTrackedResponse {
    seed: WsLogicalResponseSeed,
    billing_session: Option<BillingSession>,
    replay_request_texts: Vec<String>,
    response_started: bool,
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
    fn register_tracked_request(&mut self, tracked: WsTrackedResponse) {
        if let Some(response_id) = tracked.seed.response_id.clone() {
            if self.completed_response_ids.contains(&response_id) {
                return;
            }
            self.active_by_response_id.insert(response_id, tracked);
        } else {
            self.pending_requests.push_back(tracked);
        }
    }

    fn register_logical_request(
        &mut self,
        seed: WsLogicalResponseSeed,
        billing_session: Option<BillingSession>,
        replay_request_texts: Vec<String>,
    ) {
        let tracked = WsTrackedResponse {
            seed,
            billing_session,
            replay_request_texts,
            response_started: false,
        };
        self.register_tracked_request(tracked);
    }

    #[cfg(test)]
    fn observe_downstream_text(&mut self, text: &str) {
        let Some(value) = parse_ws_json_text(text) else {
            return;
        };
        let Some(seed) = extract_ws_logical_request_seed(&value) else {
            return;
        };
        self.register_logical_request(seed, None, vec![text.to_string()]);
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

        if is_ws_response_failed_event(&value) {
            self.release_response(&value, WS_RELEASE_REASON_RESPONSE_FAILED);
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
                    requested_service_tier: None,
                    started_at: Instant::now(),
                },
                billing_session: None,
                replay_request_texts: Vec::new(),
                response_started: false,
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
        tracked.response_started = true;

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
                    requested_service_tier: None,
                    started_at: Instant::now(),
                },
                billing_session: None,
                replay_request_texts: Vec::new(),
                response_started: false,
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
        let effective_service_tier =
            extract_service_tier_from_value(value).or(tracked.seed.requested_service_tier.clone());
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

        if let Some(mut billing_session) = tracked.billing_session {
            if billing_session.effective_service_tier.is_none() {
                billing_session.effective_service_tier = effective_service_tier.clone();
            }
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
            service_tier: effective_service_tier,
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
                    requested_service_tier: None,
                    started_at: Instant::now(),
                },
                billing_session: None,
                replay_request_texts: Vec::new(),
                response_started: false,
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

    fn find_retryable_request(&self, value: &Value) -> Option<WsTrackedResponse> {
        let response_id = extract_ws_response_id(value);
        if let Some(response_id) = response_id.as_deref() {
            if let Some(tracked) = self.active_by_response_id.get(response_id) {
                if !tracked.response_started {
                    return Some(tracked.clone());
                }
            }
        }

        let request_id = extract_ws_request_id(value);
        if let Some(request_id) = request_id.as_deref() {
            if let Some(tracked) = self.active_by_response_id.values().find(|tracked| {
                tracked.seed.request_id.as_deref() == Some(request_id) && !tracked.response_started
            }) {
                return Some(tracked.clone());
            }
            if let Some(tracked) = self.pending_requests.iter().find(|tracked| {
                tracked.seed.request_id.as_deref() == Some(request_id) && !tracked.response_started
            }) {
                return Some(tracked.clone());
            }
        }

        self.latest_unstarted_request()
    }

    fn take_retryable_request(&mut self, value: &Value) -> Option<WsTrackedResponse> {
        let response_id = extract_ws_response_id(value);
        if let Some(response_id) = response_id.as_deref() {
            if self
                .active_by_response_id
                .get(response_id)
                .is_some_and(|tracked| !tracked.response_started)
            {
                return self.active_by_response_id.remove(response_id);
            }
        }

        let request_id = extract_ws_request_id(value);
        if let Some(request_id) = request_id.as_deref() {
            if let Some(active_response_id) =
                self.active_by_response_id
                    .iter()
                    .find_map(|(response_id, tracked)| {
                        (tracked.seed.request_id.as_deref() == Some(request_id)
                            && !tracked.response_started)
                            .then(|| response_id.clone())
                    })
            {
                return self.active_by_response_id.remove(&active_response_id);
            }
            if let Some(index) = self.pending_requests.iter().position(|tracked| {
                tracked.seed.request_id.as_deref() == Some(request_id) && !tracked.response_started
            }) {
                return self.pending_requests.remove(index);
            }
        }

        self.take_latest_unstarted_request()
    }

    fn latest_unstarted_request(&self) -> Option<WsTrackedResponse> {
        let mut candidates = self
            .pending_requests
            .iter()
            .filter(|tracked| !tracked.response_started)
            .cloned()
            .collect::<Vec<_>>();
        candidates.extend(
            self.active_by_response_id
                .values()
                .filter(|tracked| !tracked.response_started)
                .cloned(),
        );
        if candidates.len() != 1 {
            return None;
        }
        candidates.pop()
    }

    fn take_latest_unstarted_request(&mut self) -> Option<WsTrackedResponse> {
        let pending_candidates = self
            .pending_requests
            .iter()
            .enumerate()
            .filter(|(_, tracked)| !tracked.response_started)
            .map(|(index, tracked)| (index, tracked.seed.started_at))
            .collect::<Vec<_>>();
        let active_candidates = self
            .active_by_response_id
            .iter()
            .filter(|(_, tracked)| !tracked.response_started)
            .map(|(response_id, tracked)| (response_id.clone(), tracked.seed.started_at))
            .collect::<Vec<_>>();

        if pending_candidates.len() + active_candidates.len() != 1 {
            return None;
        }

        if let Some((pending_index, _)) = pending_candidates.into_iter().next() {
            return self.pending_requests.remove(pending_index);
        }
        if let Some((active_response_id, _)) = active_candidates.into_iter().next() {
            return self.active_by_response_id.remove(&active_response_id);
        }
        None
    }

    fn requeue_tracked_request(&mut self, tracked: WsTrackedResponse) {
        self.register_tracked_request(tracked);
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

    fn has_unfinished_requests(&self) -> bool {
        !self.pending_requests.is_empty() || !self.active_by_response_id.is_empty()
    }
}

impl std::fmt::Display for ProxyWebSocketStreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UpstreamClosed(close) => {
                write!(
                    f,
                    "upstream websocket closed code={} reason={}",
                    close.code, close.reason
                )
            }
        }
    }
}

impl std::error::Error for ProxyWebSocketStreamError {}

async fn proxy_websocket_streams(
    downstream_socket: WebSocket,
    upstream_socket: UpstreamWebSocket,
    state: Arc<AppState>,
    mut ws_usage_context: WsLogicalUsageConnectionContext,
) -> Result<(), ProxyWebSocketStreamError> {
    let (mut downstream_sender, mut downstream_receiver) = downstream_socket.split();
    let (mut upstream_sender, mut upstream_receiver) = upstream_socket.split();
    let mut tracker = WsLogicalResponseTracker::default();
    let mut pending_replay_request_texts = Vec::new();
    let mut upstream_closed_error: Option<UpstreamWebSocketClose> = None;

    let release_reason = loop {
        tokio::select! {
            maybe_downstream = downstream_receiver.next() => {
                let Some(message) = maybe_downstream else {
                    let _ = upstream_sender.close().await;
                    break WS_RELEASE_REASON_CONNECTION_CLOSED;
                };
                let Ok(message) = message else {
                    let _ = upstream_sender.close().await;
                    break WS_RELEASE_REASON_CONNECTION_CLOSED;
                };

                let message = maybe_adapt_ws_downstream_message_for_codex_profile(
                    message,
                    ws_usage_context.adapt_codex_responses_request,
                    ws_usage_context.request_path.as_str(),
                );

                if let AxumWsMessage::Text(text) = &message {
                    if let Some(value) = parse_ws_json_text(text.as_ref()) {
                        if let Some(seed) = extract_ws_logical_request_seed(&value) {
                            let parsed_policy_context =
                                parse_ws_request_policy_context(&ws_usage_context.request_headers, &value);
                            if let Some(sticky_key) = parsed_policy_context
                                .sticky_key_hint
                                .clone()
                                .or(parsed_policy_context.session_key_hint.clone())
                            {
                                ws_usage_context.sticky_key = Some(sticky_key.clone());
                                let _ = state.router.bind_sticky(&sticky_key, ws_usage_context.account_id);
                                let _ = state
                                    .routing_cache
                                    .set_sticky_account_id(
                                        &sticky_key,
                                        ws_usage_context.account_id,
                                        Duration::from_secs(ROUTING_CACHE_STICKY_TTL_SEC),
                                    )
                                    .await;
                            }
                            if let Err(error_response) = enforce_principal_request_policy(
                                ws_usage_context.principal.as_ref(),
                                &ws_usage_context.request_headers,
                                &parsed_policy_context,
                            ) {
                                let ws_error = ws_error_message_from_response(*error_response).await;
                                if downstream_sender.send(ws_error).await.is_err() {
                                    break WS_RELEASE_REASON_CONNECTION_CLOSED;
                                }
                                continue;
                            }

                            let pending_billing_session = match build_pending_billing_session(
                                state.as_ref(),
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
                                    if downstream_sender.send(ws_error).await.is_err() {
                                        break WS_RELEASE_REASON_CONNECTION_CLOSED;
                                    }
                                    continue;
                                }
                            };

                            let billing_session = if let Some(pending_session) =
                                pending_billing_session.as_ref()
                            {
                                match authorize_billing_session(
                                    state.as_ref(),
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
                                        let ws_error = ws_error_message_from_response(*error_response).await;
                                        if downstream_sender.send(ws_error).await.is_err() {
                                            break WS_RELEASE_REASON_CONNECTION_CLOSED;
                                        }
                                        continue;
                                    }
                                }
                            } else {
                                None
                            };

                            let mut replay_request_texts = pending_replay_request_texts.clone();
                            replay_request_texts.push(text.to_string());
                            tracker.register_logical_request(
                                seed,
                                billing_session,
                                replay_request_texts,
                            );
                            pending_replay_request_texts.clear();
                        } else {
                            pending_replay_request_texts.push(text.to_string());
                        }
                    }
                }

                let should_close = matches!(message, AxumWsMessage::Close(_));
                if upstream_sender
                    .send(axum_message_to_tungstenite(message))
                    .await
                    .is_err()
                {
                    break WS_RELEASE_REASON_UPSTREAM_CLOSED;
                }
                if should_close {
                    let _ = upstream_sender.close().await;
                    let _ = downstream_sender.close().await;
                    break WS_RELEASE_REASON_CONNECTION_CLOSED;
                }
            }
            maybe_upstream = upstream_receiver.next() => {
                let Some(message) = maybe_upstream else {
                    break WS_RELEASE_REASON_UPSTREAM_CLOSED;
                };
                let Ok(message) = message else {
                    break WS_RELEASE_REASON_UPSTREAM_CLOSED;
                };

                let mut parsed_upstream_value: Option<Value> = None;
                let mut failover_error_context: Option<UpstreamErrorContext> = None;
                if let TungsteniteMessage::Text(text) = &message {
                    if let Some(value) = parse_ws_json_text(text.as_ref()) {
                        failover_error_context = build_ws_failover_error_context(&value);
                        parsed_upstream_value = Some(value);
                    } else {
                        let plain_text = text.to_string();
                        failover_error_context = build_upstream_error_context(
                            StatusCode::BAD_REQUEST,
                            &HeaderMap::new(),
                            plain_text.as_bytes(),
                        );
                    }
                }

                if let Some(error_context) = failover_error_context.as_ref() {
                    match ws_retry_scope_on_error(error_context) {
                        RetryScope::SameAccount | RetryScope::CrossAccount => {
                            let original_tracked = match parsed_upstream_value.as_ref() {
                                Some(value) => tracker.find_retryable_request(value),
                                None => tracker.latest_unstarted_request(),
                            };
                            if let Some(original_tracked) = original_tracked {
                                let next_upstream = match ws_retry_scope_on_error(error_context) {
                                    RetryScope::SameAccount => {
                                        connect_same_account_upstream_for_ws_session(
                                            &state,
                                            &ws_usage_context,
                                        )
                                        .await
                                        .map(|socket| (ws_usage_context.account_id, socket))
                                    }
                                    RetryScope::CrossAccount => {
                                        let recovery_outcome = apply_recovery_action(
                                            state.as_ref(),
                                            ws_usage_context.account_id,
                                            Some(error_context),
                                        )
                                        .await;
                                        let ejection_ttl = ejection_ttl_for_response(
                                            error_context.status,
                                            state.account_ejection_ttl,
                                            false,
                                            Some(error_context),
                                            recovery_outcome,
                                        )
                                        .unwrap_or(state.account_ejection_ttl);
                                        mark_account_unhealthy_and_clear_sticky(
                                            &state,
                                            ws_usage_context.account_id,
                                            ws_usage_context.sticky_key.as_deref(),
                                            ejection_ttl,
                                        )
                                        .await;

                                        let mut excluded_account_ids = HashSet::new();
                                        excluded_account_ids.insert(ws_usage_context.account_id);
                                        connect_failover_upstream_for_ws_session(
                                            &state,
                                            &ws_usage_context,
                                            original_tracked.seed.model.as_deref(),
                                            &excluded_account_ids,
                                        )
                                        .await
                                        .map(|(account, socket)| (account.id, socket))
                                    }
                                    RetryScope::None => None,
                                };
                                if let Some((next_account_id, next_socket)) = next_upstream {
                                    if let Some(mut retried_tracked) =
                                        build_retried_ws_tracked_request(
                                            &state,
                                            &ws_usage_context,
                                            next_account_id,
                                            &original_tracked,
                                            error_context,
                                        )
                                        .await
                                    {
                                        if !retried_tracked.replay_request_texts.is_empty() {
                                            let (mut new_upstream_sender, new_upstream_receiver) =
                                                next_socket.split();
                                            let mut replay_succeeded = true;
                                            for request_text in
                                                retried_tracked.replay_request_texts.clone()
                                            {
                                                if new_upstream_sender
                                                    .send(TungsteniteMessage::Text(
                                                        request_text.into(),
                                                    ))
                                                    .await
                                                    .is_err()
                                                {
                                                    replay_succeeded = false;
                                                    break;
                                                }
                                            }
                                            if replay_succeeded {
                                                let _ = match parsed_upstream_value.as_ref() {
                                                    Some(value) => {
                                                        tracker.take_retryable_request(value)
                                                    }
                                                    None => tracker.take_latest_unstarted_request(),
                                                };
                                                if original_tracked.billing_session.is_some() {
                                                    release_billing_hold_best_effort(
                                                        state.clone(),
                                                        original_tracked.billing_session.clone(),
                                                        Some(BillingReleaseFailureContext {
                                                            release_reason: Some(
                                                                "ws_retry_rebind".to_string(),
                                                            ),
                                                            failover_action: Some(
                                                                "retry_same_request".to_string(),
                                                            ),
                                                            failover_reason_class: Some(
                                                                "ws_retry_rebind".to_string(),
                                                            ),
                                                            ..Default::default()
                                                        }),
                                                    )
                                                    .await;
                                                }
                                                let _ = upstream_sender.close().await;
                                                upstream_sender = new_upstream_sender;
                                                upstream_receiver = new_upstream_receiver;
                                                ws_usage_context.account_id = next_account_id;
                                                tracker.requeue_tracked_request(retried_tracked);
                                                continue;
                                            }

                                            if retried_tracked.billing_session.is_some() {
                                                release_billing_hold_best_effort(
                                                    state.clone(),
                                                    retried_tracked.billing_session.take(),
                                                    Some(BillingReleaseFailureContext {
                                                        release_reason: Some(
                                                            "ws_retry_rebind".to_string(),
                                                        ),
                                                        failover_action: Some(
                                                            "retry_same_request".to_string(),
                                                        ),
                                                        failover_reason_class: Some(
                                                            "ws_retry_rebind".to_string(),
                                                        ),
                                                        ..Default::default()
                                                    }),
                                                )
                                                .await;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        RetryScope::None => {}
                    }
                }

                let pending_events = tracker.observe_upstream_message(&message, &ws_usage_context);
                if let Some(rate_limits) = extract_observed_rate_limits_from_ws_message(&message) {
                    spawn_observed_rate_limit_report_with_source(
                        &state,
                        ws_usage_context.account_id,
                        ObservedRateLimitReportSource::WebsocketEvent,
                        rate_limits,
                    );
                }
                let pending_billing_actions = tracker.drain_pending_billing_actions();
                for event in pending_events {
                    state.event_sink.emit_request_log(event).await;
                }
                process_ws_pending_billing_actions(state.clone(), pending_billing_actions).await;

                let upstream_close = if let TungsteniteMessage::Close(frame) = &message {
                    Some(frame
                        .as_ref()
                        .map(|frame| UpstreamWebSocketClose {
                            code: u16::from(frame.code),
                            reason: frame.reason.to_string(),
                        })
                        .unwrap_or_else(|| UpstreamWebSocketClose {
                            code: 1000,
                            reason: String::new(),
                        }))
                } else {
                    None
                };

                if let Some(message) = maybe_rewrite_upstream_ws_rate_limit_message(
                    message,
                    ws_usage_context.preserve_private_rate_limit_events,
                ) {
                    if let Some(mapped) = tungstenite_message_to_axum(message) {
                        if downstream_sender.send(mapped).await.is_err() {
                            break WS_RELEASE_REASON_CONNECTION_CLOSED;
                        }
                    }
                }

                if let Some(error_context) = failover_error_context.as_ref() {
                    if !tracker.has_unfinished_requests() {
                        match ws_retry_scope_on_error(error_context) {
                            RetryScope::SameAccount => {
                                if let Some(next_socket) =
                                    connect_same_account_upstream_for_ws_session(
                                        &state,
                                        &ws_usage_context,
                                    )
                                    .await
                                {
                                    let _ = upstream_sender.close().await;
                                    let (new_upstream_sender, new_upstream_receiver) =
                                        next_socket.split();
                                    upstream_sender = new_upstream_sender;
                                    upstream_receiver = new_upstream_receiver;
                                    continue;
                                }
                            }
                            RetryScope::CrossAccount => {
                                let recovery_outcome = apply_recovery_action(
                                    state.as_ref(),
                                    ws_usage_context.account_id,
                                    Some(error_context),
                                )
                                .await;
                                let ejection_ttl = ejection_ttl_for_response(
                                    error_context.status,
                                    state.account_ejection_ttl,
                                    false,
                                    Some(error_context),
                                    recovery_outcome,
                                )
                                .unwrap_or(state.account_ejection_ttl);
                                mark_account_unhealthy_and_clear_sticky(
                                    &state,
                                    ws_usage_context.account_id,
                                    ws_usage_context.sticky_key.as_deref(),
                                    ejection_ttl,
                                )
                                .await;

                                let mut excluded_account_ids = HashSet::new();
                                excluded_account_ids.insert(ws_usage_context.account_id);
                                if let Some((next_account, next_socket)) =
                                    connect_failover_upstream_for_ws_session(
                                        &state,
                                        &ws_usage_context,
                                        None,
                                        &excluded_account_ids,
                                    )
                                    .await
                                {
                                    let _ = upstream_sender.close().await;
                                    let (new_upstream_sender, new_upstream_receiver) =
                                        next_socket.split();
                                    upstream_sender = new_upstream_sender;
                                    upstream_receiver = new_upstream_receiver;
                                    ws_usage_context.account_id = next_account.id;
                                    continue;
                                }
                            }
                            RetryScope::None => {}
                        }
                    }
                }

                if let Some(close) = upstream_close {
                    let _ = downstream_sender.close().await;
                    upstream_closed_error = Some(close);
                    break WS_RELEASE_REASON_UPSTREAM_CLOSED;
                }
            }
        }
    };

    let lingering_billing_sessions = tracker.take_unfinished_billing_sessions();
    for billing_session in lingering_billing_sessions {
        release_ws_billing_session(
            state.clone(),
            billing_session,
            release_reason,
            ws_billing_phase_for_release_reason(release_reason),
        )
        .await;
    }

    if let Some(close) = upstream_closed_error {
        return Err(ProxyWebSocketStreamError::UpstreamClosed(close));
    }

    Ok(())
}

async fn build_retried_ws_tracked_request(
    state: &Arc<AppState>,
    ws_usage_context: &WsLogicalUsageConnectionContext,
    next_account_id: Uuid,
    tracked: &WsTrackedResponse,
    error_context: &UpstreamErrorContext,
) -> Option<WsTrackedResponse> {
    let original_request_text = tracked.replay_request_texts.last()?.clone();
    let retried_request_text =
        rewrite_ws_retry_request_text(&original_request_text, error_context)?;
    let value = parse_ws_json_text(&retried_request_text)?;
    let parsed_policy_context =
        parse_ws_request_policy_context(&ws_usage_context.request_headers, &value);
    let pending_billing_session = match build_pending_billing_session(
        state.as_ref(),
        ws_usage_context.principal.as_ref(),
        &ws_usage_context.request_headers,
        &parsed_policy_context,
        &ws_usage_context.request_path,
        WS_LOGICAL_REQUEST_METHOD,
    )
    .await
    {
        Ok(value) => value,
        Err(_) => return None,
    };

    let billing_session = if let Some(pending_session) = pending_billing_session.as_ref() {
        match authorize_billing_session(
            state.as_ref(),
            pending_session,
            next_account_id,
            &ws_usage_context.request_path,
            WS_LOGICAL_REQUEST_METHOD,
            tracked.seed.started_at,
            true,
        )
        .await
        {
            Ok(session) => Some(session),
            Err(_) => return None,
        }
    } else {
        None
    };

    let mut retried = tracked.clone();
    if let Some(last_request_text) = retried.replay_request_texts.last_mut() {
        *last_request_text = retried_request_text;
    }
    retried.billing_session = billing_session;
    retried.response_started = false;
    Some(retried)
}

fn parse_ws_json_text(text: &str) -> Option<Value> {
    serde_json::from_str::<Value>(text).ok()
}

fn rewrite_ws_retry_request_text(
    original_request_text: &str,
    error_context: &UpstreamErrorContext,
) -> Option<String> {
    if !matches!(
        error_context.error_code.as_deref(),
        Some("previous_response_not_found")
    ) {
        return Some(original_request_text.to_string());
    }

    let mut value = parse_ws_json_text(original_request_text)?;
    if value
        .get("type")
        .and_then(Value::as_str)
        .is_some_and(|item| item == "response.create")
    {
        value.as_object_mut()?.remove("previous_response_id");
    }
    serde_json::to_string(&value).ok()
}

fn extract_observed_rate_limits_from_ws_message(
    message: &TungsteniteMessage,
) -> Option<Vec<ObservedRateLimitSnapshot>> {
    let TungsteniteMessage::Text(text) = message else {
        return None;
    };
    let value = parse_ws_json_text(text.as_ref())?;
    if !is_ws_rate_limit_event(&value) {
        return None;
    }

    let rate_limits = value.get("rate_limits")?;
    let primary = observed_rate_limit_window_from_ws_value(rate_limits.get("primary"));
    let secondary = observed_rate_limit_window_from_ws_value(rate_limits.get("secondary"));
    if primary.is_none() && secondary.is_none() {
        return None;
    }

    Some(vec![ObservedRateLimitSnapshot {
        limit_id: Some("codex".to_string()),
        limit_name: None,
        primary,
        secondary,
        credits: observed_credits_from_ws_value(value.get("credits")),
        plan_type: ws_string_at_path(&value, &["plan_type"]).map(ToString::to_string),
    }])
}

fn maybe_rewrite_upstream_ws_rate_limit_message(
    message: TungsteniteMessage,
    preserve_private_rate_limit_events: bool,
) -> Option<TungsteniteMessage> {
    let text = match message {
        TungsteniteMessage::Text(text) => text,
        other => return Some(other),
    };
    let Some(value) = parse_ws_json_text(text.as_ref()) else {
        return Some(TungsteniteMessage::Text(text));
    };
    if !is_ws_rate_limit_event(&value) {
        return Some(TungsteniteMessage::Text(text));
    }
    if !preserve_private_rate_limit_events {
        return None;
    }

    let rewritten = build_rewritten_ws_rate_limit_event(&value)?;
    Some(TungsteniteMessage::Text(rewritten.to_string().into()))
}

fn build_rewritten_ws_rate_limit_event(value: &Value) -> Option<Value> {
    let snapshots = extract_observed_rate_limits_from_ws_message(&TungsteniteMessage::Text(
        value.to_string().into(),
    ))?;
    let snapshot = snapshots
        .first()
        .cloned()
        .map(rewrite_downstream_rate_limit_snapshot)?;
    let limit_reached = snapshot
        .primary
        .as_ref()
        .is_some_and(|window| window.used_percent >= 100.0)
        || snapshot
            .secondary
            .as_ref()
            .is_some_and(|window| window.used_percent >= 100.0);

    let mut rate_limits = serde_json::Map::new();
    rate_limits.insert("allowed".to_string(), Value::Bool(!limit_reached));
    rate_limits.insert("limit_reached".to_string(), Value::Bool(limit_reached));
    rate_limits.insert(
        "primary".to_string(),
        snapshot
            .primary
            .as_ref()
            .map(serialize_ws_rate_limit_window)
            .unwrap_or(Value::Null),
    );
    rate_limits.insert(
        "secondary".to_string(),
        snapshot
            .secondary
            .as_ref()
            .map(serialize_ws_rate_limit_window)
            .unwrap_or(Value::Null),
    );

    let mut event = serde_json::Map::new();
    event.insert(
        "type".to_string(),
        Value::String("codex.rate_limits".to_string()),
    );
    if let Some(plan_type) = snapshot.plan_type.as_ref() {
        event.insert("plan_type".to_string(), Value::String(plan_type.clone()));
    }
    event.insert("rate_limits".to_string(), Value::Object(rate_limits));
    event.insert("code_review_rate_limits".to_string(), Value::Null);
    event.insert(
        "credits".to_string(),
        snapshot
            .credits
            .as_ref()
            .map(serialize_ws_credits_snapshot)
            .unwrap_or(Value::Null),
    );
    Some(Value::Object(event))
}

fn rewrite_downstream_rate_limit_snapshot(
    mut snapshot: ObservedRateLimitSnapshot,
) -> ObservedRateLimitSnapshot {
    if let Some(primary) = snapshot.primary.as_mut() {
        primary.used_percent = 0.0;
    }
    if let Some(secondary) = snapshot.secondary.as_mut() {
        secondary.used_percent = 0.0;
    }
    snapshot
}

fn observed_rate_limit_window_from_ws_value(
    value: Option<&Value>,
) -> Option<ObservedRateLimitWindow> {
    let value = value?;
    Some(ObservedRateLimitWindow {
        used_percent: value.get("used_percent")?.as_f64()?,
        window_minutes: value.get("window_minutes").and_then(Value::as_i64),
        resets_at: value
            .get("reset_at")
            .and_then(Value::as_i64)
            .and_then(|timestamp| chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0)),
    })
}

fn observed_credits_from_ws_value(value: Option<&Value>) -> Option<ObservedCreditsSnapshot> {
    let value = value?;
    Some(ObservedCreditsSnapshot {
        has_credits: value.get("has_credits")?.as_bool()?,
        unlimited: value.get("unlimited")?.as_bool()?,
        balance: value
            .get("balance")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(ToString::to_string),
    })
}

fn serialize_ws_rate_limit_window(window: &ObservedRateLimitWindow) -> Value {
    let mut value = serde_json::Map::new();
    value.insert(
        "used_percent".to_string(),
        serde_json::json!(window.used_percent),
    );
    value.insert(
        "window_minutes".to_string(),
        window
            .window_minutes
            .map(serde_json::Value::from)
            .unwrap_or(Value::Null),
    );
    value.insert(
        "reset_at".to_string(),
        window
            .resets_at
            .map(|reset_at| Value::from(reset_at.timestamp()))
            .unwrap_or(Value::Null),
    );
    Value::Object(value)
}

fn serialize_ws_credits_snapshot(credits: &ObservedCreditsSnapshot) -> Value {
    let mut value = serde_json::Map::new();
    value.insert("has_credits".to_string(), Value::Bool(credits.has_credits));
    value.insert("unlimited".to_string(), Value::Bool(credits.unlimited));
    value.insert(
        "balance".to_string(),
        credits
            .balance
            .as_ref()
            .map(|balance| Value::String(balance.clone()))
            .unwrap_or(Value::Null),
    );
    Value::Object(value)
}

fn maybe_adapt_ws_downstream_message_for_codex_profile(
    message: AxumWsMessage,
    adapt_codex_responses_request: bool,
    request_path: &str,
) -> AxumWsMessage {
    if !adapt_codex_responses_request {
        return message;
    }
    if !matches!(
        normalize_input_path(request_path).as_str(),
        "/v1/responses" | "/v1/responses/compact"
    ) {
        return message;
    }

    let AxumWsMessage::Text(text) = message else {
        return message;
    };
    let Some(mut value) = parse_ws_json_text(text.as_ref()) else {
        return AxumWsMessage::Text(text);
    };
    if !matches!(extract_ws_event_type(&value), Some("response.create")) {
        return AxumWsMessage::Text(text);
    }
    if !adapt_codex_service_tier_fields_in_value(&mut value) {
        return AxumWsMessage::Text(text);
    }

    match serde_json::to_string(&value) {
        Ok(adapted_text) => AxumWsMessage::Text(adapted_text.into()),
        Err(_) => AxumWsMessage::Text(text),
    }
}

fn build_ws_failover_error_context(value: &Value) -> Option<UpstreamErrorContext> {
    if !is_ws_error_event(value) {
        return None;
    }
    let body = serde_json::to_vec(value).ok()?;
    build_upstream_error_context(StatusCode::BAD_REQUEST, &HeaderMap::new(), &body)
}

fn ws_retry_scope_on_error(error_context: &UpstreamErrorContext) -> RetryScope {
    decide_upstream_error(UpstreamErrorSource::WsPrelude, Some(error_context)).retry_scope
}

async fn connect_same_account_upstream_for_ws_session(
    state: &Arc<AppState>,
    ws_usage_context: &WsLogicalUsageConnectionContext,
) -> Option<UpstreamWebSocket> {
    let account = state.router.pick_account(ws_usage_context.account_id)?;
    let upstream_request = build_upstream_websocket_request(
        &account.base_url,
        &account.mode,
        &account.bearer_token,
        account.chatgpt_account_id.as_deref(),
        &ws_usage_context.request_path,
        ws_usage_context.request_query.as_deref(),
        &ws_usage_context.request_headers,
        ws_usage_context.requested_subprotocol.is_some(),
    )
    .ok()?;
    let Ok((_selected_route, upstream_socket, _)) = state
        .outbound_proxy_runtime
        .connect_websocket(upstream_request)
        .await
    else {
        return None;
    };
    if let Some(sticky_key) = ws_usage_context.sticky_key.as_deref() {
        let _ = state.router.bind_sticky(sticky_key, account.id);
        let _ = state
            .routing_cache
            .set_sticky_account_id(
                sticky_key,
                account.id,
                Duration::from_secs(ROUTING_CACHE_STICKY_TTL_SEC),
            )
            .await;
    }
    Some(upstream_socket)
}

async fn connect_failover_upstream_for_ws_session(
    state: &Arc<AppState>,
    ws_usage_context: &WsLogicalUsageConnectionContext,
    model: Option<&str>,
    excluded_account_ids: &HashSet<Uuid>,
) -> Option<(UpstreamAccount, UpstreamWebSocket)> {
    let mut excluded_account_ids = excluded_account_ids.clone();

    loop {
        let account = state.router.pick_for_model(
            model,
            ws_usage_context.sticky_key.as_deref(),
            &excluded_account_ids,
            state.sticky_prefer_non_conflicting,
        )?;

        if state.shared_routing_cache_enabled {
            if let Ok(true) = state.routing_cache.is_unhealthy(account.id).await {
                state.router.mark_unhealthy(
                    account.id,
                    state.retry_poll_interval.max(Duration::from_millis(1)),
                );
                excluded_account_ids.insert(account.id);
                continue;
            }
        }

        let upstream_request = match build_upstream_websocket_request(
            &account.base_url,
            &account.mode,
            &account.bearer_token,
            account.chatgpt_account_id.as_deref(),
            &ws_usage_context.request_path,
            ws_usage_context.request_query.as_deref(),
            &ws_usage_context.request_headers,
            ws_usage_context.requested_subprotocol.is_some(),
        ) {
            Ok(request) => request,
            Err(err) => {
                warn!(
                    error = %err,
                    account_id = %account.id,
                    "failed to build failover upstream websocket request"
                );
                mark_account_unhealthy_and_clear_sticky(
                    state,
                    account.id,
                    ws_usage_context.sticky_key.as_deref(),
                    state.account_ejection_ttl,
                )
                .await;
                excluded_account_ids.insert(account.id);
                continue;
            }
        };

        let connect_result = state
            .outbound_proxy_runtime
            .connect_websocket(upstream_request)
            .await;
        match connect_result {
            Ok((_selected_route, upstream_socket, _)) => {
                if let Some(sticky_key) = ws_usage_context.sticky_key.as_deref() {
                    let _ = state.router.bind_sticky(sticky_key, account.id);
                    let _ = state
                        .routing_cache
                        .set_sticky_account_id(
                            sticky_key,
                            account.id,
                            Duration::from_secs(ROUTING_CACHE_STICKY_TTL_SEC),
                        )
                        .await;
                }
                return Some((account, upstream_socket));
            }
            Err(err) => {
                if is_outbound_proxy_selection_ws_error(&err) {
                    return None;
                }
                let (status, error_context, is_http_handshake_error) = match &err {
                    TungsteniteError::Http(upstream_response) => {
                        let status = upstream_response.status();
                        let upstream_body = upstream_response.body().clone().unwrap_or_default();
                        let error_context = build_upstream_error_context(
                            status,
                            upstream_response.headers(),
                            &upstream_body,
                        );
                        (status, error_context, true)
                    }
                    _ => (StatusCode::BAD_GATEWAY, None, false),
                };
                let recovery_outcome =
                    apply_recovery_action(state.as_ref(), account.id, error_context.as_ref()).await;
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
                }
                .unwrap_or(state.account_ejection_ttl);
                mark_account_unhealthy_and_clear_sticky(
                    state,
                    account.id,
                    ws_usage_context.sticky_key.as_deref(),
                    ejection_ttl,
                )
                .await;
                excluded_account_ids.insert(account.id);
            }
        }
    }
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
    let previous_response_id = value
        .get("previous_response_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string);
    let header_or_metadata_session_key = sticky_session_key_from_headers(headers).or_else(|| {
        request_value
            .get("metadata")
            .and_then(|meta| meta.get("session_id"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
    });
    let model = request_value
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string);
    let requested_service_tier = extract_service_tier_from_value(request_value);
    let estimated_input_tokens = estimate_request_input_tokens(request_value);
    ParsedRequestPolicyContext {
        model,
        requested_service_tier,
        stream: true,
        request_id,
        detected_locale: detect_request_locale(headers, &bytes::Bytes::new()),
        estimated_input_tokens,
        continuation_key_hint: previous_response_id.clone(),
        sticky_key_hint: previous_response_id
            .clone()
            .or(header_or_metadata_session_key.clone()),
        session_key_hint: previous_response_id.or(header_or_metadata_session_key),
    }
}

fn ws_billing_phase_for_release_reason(release_reason: &str) -> &'static str {
    match release_reason {
        WS_RELEASE_REASON_MISSING_USAGE => WS_BILLING_RELEASED_MISSING_USAGE_PHASE,
        WS_RELEASE_REASON_RESPONSE_INCOMPLETE
        | WS_RELEASE_REASON_RESPONSE_FAILED
        | WS_RELEASE_REASON_UPSTREAM_ERROR => WS_BILLING_RELEASED_INCOMPLETE_PHASE,
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
        .unwrap_or_else(|| {
            serde_json::json!(ErrorEnvelope::new("internal_error", "request failed",))
        });
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
                    match settle_authorized_billing(
                        state.as_ref(),
                        &billing_session,
                        usage_tokens,
                        billing_session.effective_service_tier.as_deref(),
                    )
                    .await
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
    current
        .as_str()
        .map(str::trim)
        .filter(|item| !item.is_empty())
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
    if event_type != "response.create" {
        return None;
    }

    Some(WsLogicalResponseSeed {
        request_id: extract_ws_request_id(value),
        response_id: extract_ws_response_id(value),
        model: extract_ws_model(value),
        requested_service_tier: extract_service_tier_from_value(value),
        started_at: Instant::now(),
    })
}

fn is_ws_response_created_event(value: &Value) -> bool {
    matches!(extract_ws_event_type(value), Some("response.created"))
}

fn is_ws_response_completed_event(value: &Value) -> bool {
    matches!(
        extract_ws_event_type(value),
        Some("response.completed" | "response.done")
    )
}

fn is_ws_response_incomplete_event(value: &Value) -> bool {
    matches!(extract_ws_event_type(value), Some("response.incomplete"))
}

fn is_ws_response_failed_event(value: &Value) -> bool {
    matches!(extract_ws_event_type(value), Some("response.failed"))
}

fn is_ws_error_event(value: &Value) -> bool {
    matches!(extract_ws_event_type(value), Some("error"))
}

fn is_ws_rate_limit_event(value: &Value) -> bool {
    matches!(extract_ws_event_type(value), Some("codex.rate_limits"))
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
    use serde_json::Value;
    use std::time::Duration;
    use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;

    use super::{
        build_upstream_url, build_upstream_ws_url, compose_upstream_path, ejection_ttl_for_status,
        ensure_client_version_query, extract_observed_rate_limits_from_ws_message,
        extract_upstream_error_code, is_body_too_large_error, is_compatibility_passthrough_header,
        is_websocket_passthrough_header, maybe_adapt_ws_downstream_message_for_codex_profile,
        maybe_rewrite_upstream_ws_rate_limit_message, parse_request_policy_context,
        recovery_action_for_upstream_error_code, rewrite_ws_retry_request_text,
        sticky_session_key_from_headers, AxumWsMessage, ProxyRecoveryAction,
        WsLogicalResponseTracker, WsLogicalUsageConnectionContext,
    };
    use uuid::Uuid;

    #[test]
    fn builds_upstream_url_with_base_path() {
        let url = build_upstream_url(
            "https://chatgpt.com/backend-api/codex",
            &UpstreamMode::ChatGptSession,
            "/v1/responses",
            Some("a=1"),
            None,
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
        let query = ensure_client_version_query(Some("a=1"), None);
        assert!(query.contains("a=1"));
        assert!(query.contains("client_version=0.1.0"));
    }

    #[test]
    fn keeps_existing_client_version_query_for_codex_models() {
        let query = ensure_client_version_query(Some("client_version=9.9.9&a=1"), Some("1.2.3"));
        assert_eq!(query, "client_version=9.9.9&a=1");
    }

    #[test]
    fn prefers_version_header_for_codex_models_when_query_missing() {
        let query = ensure_client_version_query(Some("a=1"), Some("2026.03.12"));
        assert!(query.contains("a=1"));
        assert!(query.contains("client_version=2026.03.12"));
    }

    #[test]
    fn keeps_openai_mode_path_unchanged_even_with_codex_base_path() {
        let url = build_upstream_url(
            "https://chatgpt.com/backend-api/codex",
            &UpstreamMode::OpenAiApiKey,
            "/v1/responses",
            None,
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
            adapt_codex_responses_request: false,
            preserve_private_rate_limit_events: false,
            request_headers: HeaderMap::new(),
            request_path: "/v1/responses".to_string(),
            request_query: None,
            request_method: "GET".to_string(),
            requested_subprotocol: None,
            sticky_key: None,
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
        assert_eq!(
            events[0].billing_phase.as_deref(),
            Some("ws_response_completed")
        );
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

    #[test]
    fn ws_logical_usage_ignores_non_response_create_events() {
        let mut tracker = WsLogicalResponseTracker::default();
        tracker.observe_downstream_text(
            r#"{"type":"conversation.item.create","event_id":"evt-1","item":{"type":"message"}}"#,
        );

        assert!(!tracker.has_unfinished_requests());
    }

    #[test]
    fn ws_retryable_request_returns_none_when_multiple_unstarted_requests_have_no_ids() {
        let mut tracker = WsLogicalResponseTracker::default();
        tracker.observe_downstream_text(
            r#"{"type":"response.create","request_id":"req-1","response":{"model":"gpt-5.4"}}"#,
        );
        tracker.observe_downstream_text(
            r#"{"type":"response.create","request_id":"req-2","response":{"model":"gpt-5.4-mini"}}"#,
        );

        let value = serde_json::from_str::<Value>(
            r#"{"type":"error","error":{"code":"usage_limit","message":"quota exhausted"}}"#,
        )
        .unwrap();

        assert!(tracker.find_retryable_request(&value).is_none());
        assert!(tracker.take_retryable_request(&value).is_none());
    }

    #[test]
    fn ws_retryable_request_falls_back_when_only_one_unstarted_request_exists_without_ids() {
        let mut tracker = WsLogicalResponseTracker::default();
        tracker.observe_downstream_text(
            r#"{"type":"response.create","request_id":"req-1","response":{"model":"gpt-5.4"}}"#,
        );

        let value = serde_json::from_str::<Value>(
            r#"{"type":"error","error":{"code":"usage_limit","message":"quota exhausted"}}"#,
        )
        .unwrap();

        let tracked = tracker
            .find_retryable_request(&value)
            .expect("single pending request should be retryable");

        assert_eq!(tracked.seed.request_id.as_deref(), Some("req-1"));
        assert_eq!(tracked.seed.model.as_deref(), Some("gpt-5.4"));
    }

    #[test]
    fn ws_request_policy_context_uses_previous_response_id_for_continuation_hints() {
        let headers = HeaderMap::new();
        let value = serde_json::from_str::<Value>(
            r#"{"type":"response.create","response":{"model":"gpt-5.4"},"previous_response_id":"resp-prev-1"}"#,
        )
        .unwrap();

        let context = super::parse_ws_request_policy_context(&headers, &value);

        assert_eq!(context.sticky_key_hint.as_deref(), Some("resp-prev-1"));
        assert_eq!(context.session_key_hint.as_deref(), Some("resp-prev-1"));
    }

    #[test]
    fn ws_retry_request_rewriter_drops_stale_previous_response_id() {
        let headers = HeaderMap::new();
        let error_context = super::build_upstream_error_context(
            StatusCode::BAD_REQUEST,
            &headers,
            br#"{"type":"error","error":{"code":"previous_response_not_found","message":"previous response was not found"}}"#,
        )
        .expect("ws error payload should build context");

        let rewritten = rewrite_ws_retry_request_text(
            r#"{"type":"response.create","request_id":"req-1","previous_response_id":"resp-prev-1","response":{"model":"gpt-5.4"}}"#,
            &error_context,
        )
        .expect("retry request should rewrite");
        let rewritten_value = serde_json::from_str::<Value>(&rewritten).unwrap();

        assert!(rewritten_value.get("previous_response_id").is_none());
        assert_eq!(rewritten_value["response"]["model"], "gpt-5.4");
    }

    #[test]
    fn ws_codex_profile_adapts_fast_service_tier_to_priority() {
        let adapted = maybe_adapt_ws_downstream_message_for_codex_profile(
            AxumWsMessage::Text(
                r#"{"type":"response.create","model":"gpt-5.4","service_tier":"fast","input":[]}"#
                    .into(),
            ),
            true,
            "/v1/responses",
        );

        let AxumWsMessage::Text(text) = adapted else {
            panic!("expected text websocket message");
        };
        let value = serde_json::from_str::<Value>(text.as_ref()).unwrap();
        assert_eq!(value["service_tier"], "priority");
    }

    #[test]
    fn extracts_observed_rate_limits_from_codex_ws_event() {
        let message = TungsteniteMessage::Text(
            serde_json::json!({
                "type": "codex.rate_limits",
                "plan_type": "plus",
                "rate_limits": {
                    "allowed": true,
                    "limit_reached": false,
                    "primary": {
                        "used_percent": 42.0,
                        "window_minutes": 60,
                        "reset_at": 1700000000
                    },
                    "secondary": {
                        "used_percent": 75.0,
                        "window_minutes": 10080,
                        "reset_at": 1700003600
                    }
                },
                "credits": {
                    "has_credits": true,
                    "unlimited": false,
                    "balance": "123"
                },
                "promo": null
            })
            .to_string()
            .into(),
        );

        let snapshots = extract_observed_rate_limits_from_ws_message(&message)
            .expect("codex rate limit event should produce observed snapshots");

        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].limit_id.as_deref(), Some("codex"));
        assert_eq!(snapshots[0].plan_type.as_deref(), Some("plus"));
        assert_eq!(
            snapshots[0]
                .primary
                .as_ref()
                .map(|window| window.used_percent),
            Some(42.0)
        );
        assert_eq!(
            snapshots[0]
                .secondary
                .as_ref()
                .and_then(|window| window.window_minutes),
            Some(10_080)
        );
        assert_eq!(
            snapshots[0]
                .credits
                .as_ref()
                .and_then(|credits| credits.balance.as_deref()),
            Some("123")
        );
    }

    #[test]
    fn plain_ws_clients_do_not_receive_codex_rate_limit_events() {
        let message = TungsteniteMessage::Text(
            serde_json::json!({
                "type": "codex.rate_limits",
                "plan_type": "plus",
                "rate_limits": {
                    "allowed": true,
                    "limit_reached": false,
                    "primary": {
                        "used_percent": 42.0,
                        "window_minutes": 60,
                        "reset_at": 1700000000
                    },
                    "secondary": null
                },
                "credits": {
                    "has_credits": true,
                    "unlimited": false,
                    "balance": "123"
                }
            })
            .to_string()
            .into(),
        );

        let rewritten = maybe_rewrite_upstream_ws_rate_limit_message(message, false);

        assert!(
            rewritten.is_none(),
            "plain websocket clients should not receive private codex rate limit events"
        );
    }

    #[test]
    fn compat_ws_clients_receive_rewritten_codex_rate_limit_events() {
        let message = TungsteniteMessage::Text(
            serde_json::json!({
                "type": "codex.rate_limits",
                "plan_type": "plus",
                "rate_limits": {
                    "allowed": true,
                    "limit_reached": false,
                    "primary": {
                        "used_percent": 42.0,
                        "window_minutes": 60,
                        "reset_at": 1700000000
                    },
                    "secondary": {
                        "used_percent": 95.0,
                        "window_minutes": 10080,
                        "reset_at": 1700003600
                    }
                },
                "credits": {
                    "has_credits": true,
                    "unlimited": false,
                    "balance": "123"
                },
                "promo": {
                    "message": "should not be raw passthrough"
                }
            })
            .to_string()
            .into(),
        );

        let rewritten = maybe_rewrite_upstream_ws_rate_limit_message(message, true)
            .expect("compat websocket clients should receive a rewritten event");
        let TungsteniteMessage::Text(text) = rewritten else {
            panic!("expected rewritten websocket rate limit event to stay as text");
        };
        let value = serde_json::from_str::<Value>(text.as_ref()).unwrap();

        assert_eq!(value["type"], "codex.rate_limits");
        assert_eq!(value["plan_type"], "plus");
        assert_eq!(value["rate_limits"]["primary"]["used_percent"], 0.0);
        assert_eq!(value["rate_limits"]["secondary"]["used_percent"], 0.0);
        assert_eq!(value["credits"]["balance"], "123");
        assert!(
            value.get("promo").is_none(),
            "rewritten event should drop raw promo payload"
        );
    }
}
