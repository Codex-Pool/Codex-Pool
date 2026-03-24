use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};

use axum::extract::Path;
use axum::http::header::CONTENT_TYPE;
use axum::response::IntoResponse;
use chrono::Utc;
use serde_json::{Map, json};
use tokio::sync::{Mutex, RwLock, Semaphore};

const BACKGROUND_SELF_REQUEST_HEADER: &str = "x-codex-background-task";
const BACKGROUND_RESPONSES_RETENTION_SEC_ENV: &str = "DATA_PLANE_RESPONSES_RETENTION_SEC";
const BACKGROUND_RESPONSES_CLEANUP_INTERVAL_SEC_ENV: &str =
    "DATA_PLANE_RESPONSES_CLEANUP_INTERVAL_SEC";
const BACKGROUND_RESPONSES_MAX_CONCURRENCY_ENV: &str = "DATA_PLANE_RESPONSES_MAX_CONCURRENCY";
const BACKGROUND_RESPONSES_MAX_RPS_ENV: &str = "DATA_PLANE_RESPONSES_MAX_RPS";
const DATA_PLANE_BASE_URL_ENV: &str = "DATA_PLANE_BASE_URL";
const DEFAULT_BACKGROUND_RESPONSES_RETENTION_SEC: u64 = 24 * 60 * 60;
const DEFAULT_BACKGROUND_RESPONSES_CLEANUP_INTERVAL_SEC: u64 = 60;
const DEFAULT_BACKGROUND_RESPONSES_MAX_CONCURRENCY: usize = 2;
const DEFAULT_BACKGROUND_RESPONSES_MAX_RPS: u32 = 1;
const MIN_BACKGROUND_RESPONSES_RETENTION_SEC: u64 = 60;
const MAX_BACKGROUND_RESPONSES_RETENTION_SEC: u64 = 7 * 24 * 60 * 60;
const MIN_BACKGROUND_RESPONSES_CLEANUP_INTERVAL_SEC: u64 = 10;
const MAX_BACKGROUND_RESPONSES_CLEANUP_INTERVAL_SEC: u64 = 60 * 60;
const MIN_BACKGROUND_RESPONSES_MAX_CONCURRENCY: usize = 1;
const MAX_BACKGROUND_RESPONSES_MAX_CONCURRENCY: usize = 64;
const MIN_BACKGROUND_RESPONSES_MAX_RPS: u32 = 1;
const MAX_BACKGROUND_RESPONSES_MAX_RPS: u32 = 100;

#[derive(Debug, Clone)]
pub struct BackgroundResponsesRuntime {
    entries: Arc<RwLock<HashMap<String, StoredResponseRecord>>>,
    conversations: Arc<RwLock<HashMap<String, ConversationCursor>>>,
    permits: Arc<Semaphore>,
    next_dispatch_at: Arc<Mutex<Instant>>,
    self_base_url: Arc<str>,
    retention: Duration,
    cleanup_interval: Duration,
    max_rps: u32,
    in_flight_total: Arc<AtomicU64>,
}

#[derive(Debug, Clone)]
struct StoredResponseRecord {
    owner_key: String,
    response: Value,
    allow_retrieve: bool,
    request_body: Option<Value>,
    cancelled: bool,
    expires_at: Instant,
}

#[derive(Debug, Clone)]
struct ConversationCursor {
    owner_key: String,
    response_id: String,
    expires_at: Instant,
}

#[derive(Debug, Clone)]
struct BackgroundRequestSnapshot {
    response_id: String,
    principal_token: String,
    headers: Vec<(String, String)>,
    body: Value,
    detected_locale: String,
    conversation_id: Option<String>,
}

impl BackgroundResponsesRuntime {
    pub fn from_env(listen_addr: SocketAddr) -> Self {
        let retention = Duration::from_secs(parse_env_u64_with_bounds(
            BACKGROUND_RESPONSES_RETENTION_SEC_ENV,
            DEFAULT_BACKGROUND_RESPONSES_RETENTION_SEC,
            MIN_BACKGROUND_RESPONSES_RETENTION_SEC,
            MAX_BACKGROUND_RESPONSES_RETENTION_SEC,
        ));
        let cleanup_interval = Duration::from_secs(parse_env_u64_with_bounds(
            BACKGROUND_RESPONSES_CLEANUP_INTERVAL_SEC_ENV,
            DEFAULT_BACKGROUND_RESPONSES_CLEANUP_INTERVAL_SEC,
            MIN_BACKGROUND_RESPONSES_CLEANUP_INTERVAL_SEC,
            MAX_BACKGROUND_RESPONSES_CLEANUP_INTERVAL_SEC,
        ));
        let max_concurrency = parse_env_usize_with_bounds(
            BACKGROUND_RESPONSES_MAX_CONCURRENCY_ENV,
            DEFAULT_BACKGROUND_RESPONSES_MAX_CONCURRENCY,
            MIN_BACKGROUND_RESPONSES_MAX_CONCURRENCY,
            MAX_BACKGROUND_RESPONSES_MAX_CONCURRENCY,
        );
        let max_rps = parse_env_u32_with_bounds(
            BACKGROUND_RESPONSES_MAX_RPS_ENV,
            DEFAULT_BACKGROUND_RESPONSES_MAX_RPS,
            MIN_BACKGROUND_RESPONSES_MAX_RPS,
            MAX_BACKGROUND_RESPONSES_MAX_RPS,
        );
        let self_base_url = std::env::var(DATA_PLANE_BASE_URL_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| {
                let host = loopback_host_for_listen_addr(listen_addr);
                format!("http://{host}:{}", listen_addr.port())
            });

        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            conversations: Arc::new(RwLock::new(HashMap::new())),
            permits: Arc::new(Semaphore::new(max_concurrency)),
            next_dispatch_at: Arc::new(Mutex::new(Instant::now())),
            self_base_url: Arc::from(self_base_url),
            retention,
            cleanup_interval,
            max_rps,
            in_flight_total: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn cleanup_interval(&self) -> Duration {
        self.cleanup_interval
    }

    pub async fn cleanup_expired(&self) {
        let now = Instant::now();
        let mut entries = self.entries.write().await;
        entries.retain(|_, record| record.expires_at > now);
        drop(entries);

        let mut conversations = self.conversations.write().await;
        conversations.retain(|_, cursor| cursor.expires_at > now);
    }

    async fn lookup_response(&self, owner_key: &str, response_id: &str) -> Option<Value> {
        let entries = self.entries.read().await;
        let record = entries.get(response_id)?;
        if record.owner_key != owner_key
            || record.expires_at <= Instant::now()
            || !record.allow_retrieve
        {
            return None;
        }
        Some(record.response.clone())
    }

    async fn cancel_response(&self, owner_key: &str, response_id: &str) -> Option<Value> {
        let mut entries = self.entries.write().await;
        let record = entries.get_mut(response_id)?;
        if record.owner_key != owner_key || record.expires_at <= Instant::now() {
            return None;
        }
        record.cancelled = true;
        if !is_terminal_response_status(record.response.get("status").and_then(Value::as_str)) {
            set_response_status(&mut record.response, "cancelled");
            set_response_timestamp(&mut record.response, "cancelled_at", Utc::now().timestamp());
        }
        Some(record.response.clone())
    }

    async fn current_conversation_response_id(
        &self,
        owner_key: &str,
        conversation_id: &str,
    ) -> Option<String> {
        let conversations = self.conversations.read().await;
        let cursor = conversations.get(conversation_id)?;
        if cursor.owner_key != owner_key || cursor.expires_at <= Instant::now() {
            return None;
        }
        Some(cursor.response_id.clone())
    }

    async fn queue_background_response(
        &self,
        owner_key: String,
        request_body: Value,
        conversation_id: Option<String>,
    ) -> String {
        let response_id = format!("resp_{}", Uuid::new_v4().simple());
        let now = Utc::now().timestamp();
        let response = build_response_stub(
            &request_body,
            &response_id,
            "queued",
            true,
            now,
            conversation_id.as_deref(),
        );
        let record = StoredResponseRecord {
            owner_key,
            response,
            allow_retrieve: true,
            request_body: Some(request_body),
            cancelled: false,
            expires_at: Instant::now() + self.retention,
        };
        self.entries.write().await.insert(response_id.clone(), record);
        response_id
    }

    async fn mark_background_in_progress(&self, response_id: &str) -> bool {
        let mut entries = self.entries.write().await;
        let Some(record) = entries.get_mut(response_id) else {
            return false;
        };
        if record.cancelled {
            return false;
        }
        set_response_status(&mut record.response, "in_progress");
        set_response_timestamp(&mut record.response, "started_at", Utc::now().timestamp());
        true
    }

    async fn apply_background_result(
        &self,
        response_id: &str,
        mut response_value: Value,
        conversation_id: Option<String>,
    ) {
        let mut entries = self.entries.write().await;
        let Some(record) = entries.get_mut(response_id) else {
            return;
        };
        if record.cancelled {
            return;
        }
        if let Some(object) = response_value.as_object_mut() {
            object.insert("background".to_string(), Value::Bool(true));
            if let Some(conversation_id_value) = conversation_id.as_deref() {
                object
                    .entry("conversation".to_string())
                    .or_insert_with(|| Value::String(conversation_id_value.to_string()));
            }
        }
        let owner_key = record.owner_key.clone();
        let expires_at = record.expires_at;
        record.response = response_value.clone();
        record.request_body = None;
        drop(entries);

        if let Some(conversation_id) = conversation_id {
            if let Some(final_response_id) = response_value.get("id").and_then(Value::as_str) {
                self.conversations.write().await.insert(
                    conversation_id,
                    ConversationCursor {
                        owner_key,
                        response_id: final_response_id.to_string(),
                        expires_at,
                    },
                );
            }
        }
    }

    async fn apply_background_failure(
        &self,
        response_id: &str,
        request_body: &Value,
        status_code: StatusCode,
        response_body: Option<&Bytes>,
        conversation_id: Option<String>,
    ) {
        let mut entries = self.entries.write().await;
        let Some(record) = entries.get_mut(response_id) else {
            return;
        };
        if record.cancelled {
            return;
        }
        let error = response_body
            .and_then(|body| serde_json::from_slice::<Value>(body).ok())
            .and_then(|value: Value| value.get("error").cloned())
            .unwrap_or_else(|| {
                json!({
                    "code": "background_request_failed",
                    "message": "background request failed"
                })
            });
        record.response = build_failed_background_response(
            request_body,
            response_id,
            status_code,
            error,
            conversation_id.as_deref(),
        );
        record.request_body = None;
    }

    async fn store_completed_response(
        &self,
        owner_key: String,
        request_body: &Value,
        response_body: &Bytes,
        conversation_id: Option<String>,
        force_store: bool,
    ) {
        let Some(mut response_value) = serde_json::from_slice::<Value>(response_body).ok() else {
            return;
        };
        let Some(response_id) = response_value
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string)
        else {
            return;
        };
        if !force_store && request_explicitly_disables_store(request_body) {
            return;
        }
        if let Some(conversation_id_value) = conversation_id.as_deref() {
            if let Some(object) = response_value.as_object_mut() {
                object
                    .entry("conversation".to_string())
                    .or_insert_with(|| Value::String(conversation_id_value.to_string()));
            }
        }

        let expires_at = Instant::now() + self.retention;
        self.entries.write().await.insert(
            response_id.clone(),
            StoredResponseRecord {
                owner_key: owner_key.clone(),
                response: response_value.clone(),
                allow_retrieve: !request_explicitly_disables_store(request_body) || force_store,
                request_body: None,
                cancelled: false,
                expires_at,
            },
        );

        if let Some(conversation_id) = conversation_id {
            self.conversations.write().await.insert(
                conversation_id,
                ConversationCursor {
                    owner_key,
                    response_id,
                    expires_at,
                },
            );
        }
    }

    async fn wait_for_dispatch_slot(&self) {
        let spacing = if self.max_rps <= 1 {
            Duration::from_secs(1)
        } else {
            Duration::from_secs_f64(1.0 / f64::from(self.max_rps))
        };
        let mut next_dispatch_at = self.next_dispatch_at.lock().await;
        let now = Instant::now();
        if *next_dispatch_at > now {
            tokio::time::sleep(*next_dispatch_at - now).await;
        }
        *next_dispatch_at = Instant::now() + spacing;
    }
}

pub async fn responses_retrieve_handler(
    State(state): State<Arc<AppState>>,
    principal: Option<axum::Extension<ApiPrincipal>>,
    Path(response_id): Path<String>,
) -> Response {
    let owner_key = response_owner_key(principal.as_ref().map(|item| &item.0));
    match state
        .background_responses
        .lookup_response(owner_key.as_str(), response_id.as_str())
        .await
    {
        Some(response) => axum::Json(response).into_response(),
        None => localized_json_error_with_state(
            state.as_ref(),
            "en",
            StatusCode::NOT_FOUND,
            "response_not_found",
            "response was not found",
        ),
    }
}

pub async fn responses_cancel_handler(
    State(state): State<Arc<AppState>>,
    principal: Option<axum::Extension<ApiPrincipal>>,
    Path(response_id): Path<String>,
) -> Response {
    let owner_key = response_owner_key(principal.as_ref().map(|item| &item.0));
    match state
        .background_responses
        .cancel_response(owner_key.as_str(), response_id.as_str())
        .await
    {
        Some(response) => axum::Json(response).into_response(),
        None => localized_json_error_with_state(
            state.as_ref(),
            "en",
            StatusCode::NOT_FOUND,
            "response_not_found",
            "response was not found",
        ),
    }
}

pub async fn responses_input_tokens_handler(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
) -> Response {
    let header_locale = detect_request_locale(request.headers(), &Bytes::new());
    let max_request_body_bytes =
        max_request_body_bytes_for_path(state.max_request_body_bytes, "/v1/responses/input_tokens");
    let (_, body) = request.into_parts();
    let body = match axum::body::to_bytes(body, max_request_body_bytes).await {
        Ok(body) => body,
        Err(_) => {
            return localized_json_error_with_state(
                state.as_ref(),
                header_locale.as_str(),
                StatusCode::BAD_REQUEST,
                "invalid_request_body",
                "failed to read request body",
            )
        }
    };
    let Some(value) = serde_json::from_slice::<Value>(&body).ok() else {
        return localized_json_error_with_state(
            state.as_ref(),
            header_locale.as_str(),
            StatusCode::BAD_REQUEST,
            "invalid_request_body",
            "request body must be valid JSON",
        );
    };
    let input_tokens = estimate_request_input_tokens(&value).unwrap_or(0).max(0);
    axum::Json(json!({
        "object": "response.input_tokens",
        "input_tokens": input_tokens
    }))
    .into_response()
}

async fn maybe_handle_background_response_request(
    state: Arc<AppState>,
    principal: Option<&ApiPrincipal>,
    path: &str,
    method: &axum::http::Method,
    headers: &HeaderMap,
    body_bytes: &Bytes,
    parsed_policy_context: &ParsedRequestPolicyContext,
) -> Option<Response> {
    if method != axum::http::Method::POST || path != "/v1/responses" {
        return None;
    }
    if headers
        .get(BACKGROUND_SELF_REQUEST_HEADER)
        .and_then(|value| value.to_str().ok())
        == Some("1")
    {
        return None;
    }
    let mut request_value = parse_request_json_body(headers, body_bytes)?;
    if !request_value
        .get("background")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return None;
    }
    let detected_locale = parsed_policy_context.detected_locale.as_str();
    let owner_key = response_owner_key(principal);
    let principal_token = principal.map(|item| item.token.clone()).unwrap_or_default();
    let conversation_id = parsed_policy_context.conversation_id.clone();

    if request_value.get("stream").and_then(Value::as_bool) == Some(true) {
        if let Some(object) = request_value.as_object_mut() {
            object.insert("stream".to_string(), Value::Bool(false));
        }
    }

    let response_id = state
        .background_responses
        .queue_background_response(
            owner_key.clone(),
            request_value.clone(),
            conversation_id.clone(),
        )
        .await;
    let queued_response = state
        .background_responses
        .lookup_response(owner_key.as_str(), response_id.as_str())
        .await
        .unwrap_or_else(|| {
            build_response_stub(
                &request_value,
                response_id.as_str(),
                "queued",
                true,
                Utc::now().timestamp(),
                conversation_id.as_deref(),
            )
        });

    let snapshot = BackgroundRequestSnapshot {
        response_id,
        principal_token,
        headers: collect_background_headers(headers),
        body: request_value,
        detected_locale: detected_locale.to_string(),
        conversation_id,
    };
    spawn_background_response_worker(state, snapshot);

    let response = axum::Json(queued_response).into_response();
    Some(with_status(response, StatusCode::ACCEPTED))
}

fn response_owner_key(principal: Option<&ApiPrincipal>) -> String {
    if let Some(api_key_id) = principal.and_then(|item| item.api_key_id) {
        return format!("api_key:{api_key_id}");
    }
    if let Some(token) = principal.map(|item| item.token.as_str()) {
        return format!("token:{}", stable_token_hash(token));
    }
    "anonymous".to_string()
}

fn request_explicitly_disables_store(value: &Value) -> bool {
    value.get("store").and_then(Value::as_bool) == Some(false)
}

async fn store_completed_response_from_proxy(
    state: &Arc<AppState>,
    principal: Option<&ApiPrincipal>,
    request_body: &Bytes,
    response_body: &Bytes,
    parsed_policy_context: &ParsedRequestPolicyContext,
    force_store: bool,
) {
    let Some(request_value) = serde_json::from_slice::<Value>(request_body).ok() else {
        return;
    };
    state
        .background_responses
        .store_completed_response(
            response_owner_key(principal),
            &request_value,
            response_body,
            parsed_policy_context.conversation_id.clone(),
            force_store,
        )
        .await;
}

fn apply_conversation_semantics_to_request(
    request_value: &mut Value,
    parsed_policy_context: &mut ParsedRequestPolicyContext,
    previous_response_id: Option<String>,
) -> anyhow::Result<()> {
    let Some(object) = request_value.as_object_mut() else {
        return Ok(());
    };
    let has_previous_response_id = object
        .get("previous_response_id")
        .and_then(Value::as_str)
        .is_some();
    let conversation_id = object
        .get("conversation")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    if has_previous_response_id && conversation_id.is_some() {
        anyhow::bail!("previous_response_id cannot be used together with conversation");
    }

    if let Some(conversation_id) = conversation_id.clone() {
        parsed_policy_context.conversation_id = Some(conversation_id.clone());
        parsed_policy_context.sticky_key_hint = Some(conversation_id.clone());
        parsed_policy_context.session_key_hint = Some(conversation_id.clone());
        if !has_previous_response_id {
            if let Some(previous_response_id) = previous_response_id {
                object.insert(
                    "previous_response_id".to_string(),
                    Value::String(previous_response_id.clone()),
                );
                parsed_policy_context.continuation_key_hint = Some(previous_response_id);
            }
        }
    }

    Ok(())
}

fn spawn_background_response_worker(state: Arc<AppState>, snapshot: BackgroundRequestSnapshot) {
    tokio::spawn(async move {
        let permit = match state.background_responses.permits.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(_) => return,
        };
        let _permit = permit;
        if !state
            .background_responses
            .mark_background_in_progress(snapshot.response_id.as_str())
            .await
        {
            return;
        }
        state
            .background_responses
            .in_flight_total
            .fetch_add(1, Ordering::Relaxed);
        state.background_responses.wait_for_dispatch_slot().await;

        let mut request_value = snapshot.body.clone();
        if let Some(object) = request_value.as_object_mut() {
            object.remove("background");
            object.insert("stream".to_string(), Value::Bool(false));
        }

        let body = match serde_json::to_vec(&request_value) {
            Ok(body) => body,
            Err(err) => {
                warn!(error = %err, response_id = %snapshot.response_id, "failed to serialize background response request");
                state
                    .background_responses
                    .apply_background_failure(
                        snapshot.response_id.as_str(),
                        &snapshot.body,
                        StatusCode::BAD_REQUEST,
                        None,
                        snapshot.conversation_id.clone(),
                    )
                    .await;
                state
                    .background_responses
                    .in_flight_total
                    .fetch_sub(1, Ordering::Relaxed);
                return;
            }
        };

        let url = format!(
            "{}/v1/responses",
            state.background_responses.self_base_url.trim_end_matches('/')
        );
        let mut request_builder = state.http_client.post(url);
        request_builder = request_builder.header(AUTHORIZATION, format!("Bearer {}", snapshot.principal_token));
        request_builder = request_builder.header(CONTENT_TYPE, "application/json");
        request_builder = request_builder.header(BACKGROUND_SELF_REQUEST_HEADER, "1");
        for (name, value) in snapshot.headers {
            request_builder = request_builder.header(name, value);
        }

        let result = request_builder.body(body).send().await;
        match result {
            Ok(response) => {
                let status = response.status();
                let response_body = response.bytes().await.ok();
                if status.is_success() {
                    if let Some(response_body) = response_body {
                        if let Ok(response_value) = serde_json::from_slice::<Value>(&response_body) {
                            state
                                .background_responses
                                .apply_background_result(
                                    snapshot.response_id.as_str(),
                                    response_value,
                                    snapshot.conversation_id,
                                )
                                .await;
                        } else {
                            state
                                .background_responses
                                .apply_background_failure(
                                    snapshot.response_id.as_str(),
                                    &snapshot.body,
                                    StatusCode::BAD_GATEWAY,
                                    Some(&response_body),
                                    snapshot.conversation_id,
                                )
                                .await;
                        }
                    }
                } else {
                    state
                        .background_responses
                        .apply_background_failure(
                            snapshot.response_id.as_str(),
                            &snapshot.body,
                            status,
                            response_body.as_ref(),
                            snapshot.conversation_id,
                        )
                        .await;
                }
            }
            Err(err) => {
                warn!(
                    error = %err,
                    response_id = %snapshot.response_id,
                    locale = %snapshot.detected_locale,
                    "background response self-request failed"
                );
                state
                    .background_responses
                    .apply_background_failure(
                        snapshot.response_id.as_str(),
                        &snapshot.body,
                        StatusCode::BAD_GATEWAY,
                        None,
                        snapshot.conversation_id,
                    )
                    .await;
            }
        }

        state
            .background_responses
            .in_flight_total
            .fetch_sub(1, Ordering::Relaxed);
    });
}

fn collect_background_headers(headers: &HeaderMap) -> Vec<(String, String)> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            if matches!(
                name.as_str(),
                "authorization" | "host" | "content-length" | "content-type" | BACKGROUND_SELF_REQUEST_HEADER
            ) {
                return None;
            }
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_string(), value.to_string()))
        })
        .collect()
}

fn build_response_stub(
    request_body: &Value,
    response_id: &str,
    status: &str,
    background: bool,
    created_at: i64,
    conversation_id: Option<&str>,
) -> Value {
    let mut response = json!({
        "id": response_id,
        "object": "response",
        "created_at": created_at,
        "status": status,
        "background": background,
        "error": Value::Null,
        "incomplete_details": Value::Null,
        "instructions": request_body.get("instructions").cloned().unwrap_or(Value::Null),
        "max_output_tokens": request_body
            .get("max_output_tokens")
            .cloned()
            .unwrap_or(Value::Null),
        "model": request_body.get("model").cloned().unwrap_or(Value::Null),
        "output": Value::Array(Vec::new()),
        "parallel_tool_calls": request_body
            .get("parallel_tool_calls")
            .cloned()
            .unwrap_or(Value::Bool(true)),
        "previous_response_id": request_body
            .get("previous_response_id")
            .cloned()
            .unwrap_or(Value::Null),
        "store": request_body.get("store").cloned().unwrap_or(Value::Bool(true)),
        "temperature": request_body
            .get("temperature")
            .cloned()
            .unwrap_or(Value::Number(serde_json::Number::from(1))),
        "text": request_body
            .get("text")
            .cloned()
            .unwrap_or_else(|| json!({"format": {"type": "text"}})),
        "tool_choice": request_body
            .get("tool_choice")
            .cloned()
            .unwrap_or(Value::String("auto".to_string())),
        "tools": request_body.get("tools").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "top_p": request_body
            .get("top_p")
            .cloned()
            .unwrap_or(Value::Number(serde_json::Number::from(1))),
        "truncation": request_body
            .get("truncation")
            .cloned()
            .unwrap_or(Value::String("disabled".to_string())),
        "usage": Value::Null,
        "metadata": request_body.get("metadata").cloned().unwrap_or_else(|| Value::Object(Map::new())),
        "conversation": conversation_id.map_or(Value::Null, |value| Value::String(value.to_string())),
    });
    if status == "completed" {
        set_response_timestamp(&mut response, "completed_at", Utc::now().timestamp());
    }
    response
}

fn build_failed_background_response(
    request_body: &Value,
    response_id: &str,
    status_code: StatusCode,
    error: Value,
    conversation_id: Option<&str>,
) -> Value {
    let mut response = build_response_stub(
        request_body,
        response_id,
        "failed",
        true,
        Utc::now().timestamp(),
        conversation_id,
    );
    if let Some(object) = response.as_object_mut() {
        object.insert("error".to_string(), error);
        object.insert(
            "status_code".to_string(),
            Value::Number(serde_json::Number::from(u64::from(status_code.as_u16()))),
        );
    }
    response
}

fn set_response_status(response: &mut Value, status: &str) {
    if let Some(object) = response.as_object_mut() {
        object.insert("status".to_string(), Value::String(status.to_string()));
        object.insert(
            "background".to_string(),
            Value::Bool(object.get("background").and_then(Value::as_bool).unwrap_or(true)),
        );
    }
}

fn set_response_timestamp(response: &mut Value, key: &str, value: i64) {
    if let Some(object) = response.as_object_mut() {
        object.insert(
            key.to_string(),
            Value::Number(serde_json::Number::from(value)),
        );
    }
}

fn is_terminal_response_status(status: Option<&str>) -> bool {
    matches!(status, Some("completed" | "failed" | "cancelled" | "incomplete"))
}

fn with_status(mut response: Response, status: StatusCode) -> Response {
    *response.status_mut() = status;
    response
}

fn parse_env_u64_with_bounds(name: &str, default: u64, min: u64, max: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .map(|value| value.clamp(min, max))
        .unwrap_or(default)
}

fn parse_env_usize_with_bounds(name: &str, default: usize, min: usize, max: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .map(|value| value.clamp(min, max))
        .unwrap_or(default)
}

fn parse_env_u32_with_bounds(name: &str, default: u32, min: u32, max: u32) -> u32 {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.parse::<u32>().ok())
        .map(|value| value.clamp(min, max))
        .unwrap_or(default)
}

fn loopback_host_for_listen_addr(listen_addr: SocketAddr) -> IpAddr {
    match listen_addr.ip() {
        IpAddr::V6(_) => IpAddr::V6(std::net::Ipv6Addr::LOCALHOST),
        IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::LOCALHOST),
    }
}

fn stable_token_hash(token: &str) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}
