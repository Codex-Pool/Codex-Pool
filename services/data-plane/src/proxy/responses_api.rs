use std::collections::HashMap;
use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};

use axum::extract::{Path, Query};
use axum::http::header::CONTENT_TYPE;
use axum::response::IntoResponse;
use chrono::Utc;
use serde_json::{Map, json};
use tokio::sync::{Mutex, Notify, RwLock, Semaphore};

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
    continuation_cursors: Arc<RwLock<HashMap<String, ConversationCursor>>>,
    permits: Arc<Semaphore>,
    next_dispatch_at: Arc<Mutex<Instant>>,
    self_base_url: Arc<str>,
    retention: Duration,
    cleanup_interval: Duration,
    max_rps: u32,
    in_flight_total: Arc<AtomicU64>,
}

#[derive(Debug)]
struct StoredResponseRecord {
    owner_key: String,
    response: Value,
    allow_retrieve: bool,
    input_items: Vec<Value>,
    request_body: Option<Value>,
    cancelled: bool,
    background: bool,
    stream_state: Option<StoredResponseStream>,
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
    owner_key: String,
    principal_token: String,
    headers: Vec<(String, String)>,
    body: Value,
    detected_locale: String,
    conversation_id: Option<String>,
    stream: bool,
}

#[derive(Debug)]
struct StoredResponseStream {
    events: Vec<StoredStreamEvent>,
    next_sequence_number: u64,
    terminal: bool,
    notify: Arc<Notify>,
}

#[derive(Debug, Clone)]
struct StoredStreamEvent {
    sequence_number: u64,
    bytes: Bytes,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ResponseRetrieveQuery {
    #[serde(default)]
    include: Vec<String>,
    stream: Option<bool>,
    starting_after: Option<u64>,
    include_obfuscation: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ResponseInputItemsQuery {
    limit: Option<usize>,
    order: Option<String>,
    after: Option<String>,
    #[serde(default)]
    include: Vec<String>,
}

#[derive(Debug)]
enum CancelResponseOutcome {
    NotFound,
    NotCancellable,
    Response(Value),
}

#[derive(Debug)]
enum StreamLookupOutcome {
    NotFound,
    NotStreamable,
    Response(Response),
}

impl StoredResponseStream {
    fn new() -> Self {
        Self {
            events: Vec::new(),
            next_sequence_number: 1,
            terminal: false,
            notify: Arc::new(Notify::new()),
        }
    }

    fn append_frame(
        &mut self,
        event_name: Option<&str>,
        mut event_value: Value,
    ) -> (u64, bool, Option<Value>) {
        let sequence_number = self.next_sequence_number;
        self.next_sequence_number += 1;
        if let Some(object) = event_value.as_object_mut() {
            object.insert(
                "sequence_number".to_string(),
                Value::Number(serde_json::Number::from(sequence_number)),
            );
        }
        let terminal = matches!(
            event_value.get("type").and_then(Value::as_str),
            Some(
                "response.completed"
                    | "response.done"
                    | "response.failed"
                    | "response.incomplete"
                    | "response.cancelled"
            )
        );
        let response = event_value.get("response").cloned();
        let bytes = build_sse_frame(event_name, &event_value);
        self.events.push(StoredStreamEvent {
            sequence_number,
            bytes,
        });
        if terminal {
            self.terminal = true;
        }
        self.notify.notify_waiters();
        (sequence_number, terminal, response)
    }
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
            continuation_cursors: Arc::new(RwLock::new(HashMap::new())),
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

        let mut continuation_cursors = self.continuation_cursors.write().await;
        continuation_cursors.retain(|_, cursor| cursor.expires_at > now);
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

    async fn lookup_input_items(
        &self,
        owner_key: &str,
        response_id: &str,
        query: &ResponseInputItemsQuery,
    ) -> Option<Value> {
        let entries = self.entries.read().await;
        let record = entries.get(response_id)?;
        if record.owner_key != owner_key
            || record.expires_at <= Instant::now()
            || !record.allow_retrieve
        {
            return None;
        }

        let mut items = record.input_items.clone();
        drop(entries);

        let order = query.order.as_deref().unwrap_or("desc");
        if order != "asc" {
            items.reverse();
        }

        if let Some(after) = query.after.as_deref() {
            if let Some(position) = items
                .iter()
                .position(|item| item.get("id").and_then(Value::as_str) == Some(after))
            {
                items = items.into_iter().skip(position + 1).collect();
            }
        }

        let limit = query.limit.unwrap_or(20).clamp(1, 100);
        let has_more = items.len() > limit;
        let page = items.into_iter().take(limit).collect::<Vec<_>>();
        let first_id = page
            .first()
            .and_then(|item| item.get("id").cloned())
            .unwrap_or(Value::Null);
        let last_id = page
            .last()
            .and_then(|item| item.get("id").cloned())
            .unwrap_or(Value::Null);

        Some(json!({
            "object": "list",
            "data": page,
            "first_id": first_id,
            "last_id": last_id,
            "has_more": has_more,
        }))
    }

    async fn cancel_response(&self, owner_key: &str, response_id: &str) -> CancelResponseOutcome {
        let mut entries = self.entries.write().await;
        let Some(record) = entries.get_mut(response_id) else {
            return CancelResponseOutcome::NotFound;
        };
        if record.owner_key != owner_key || record.expires_at <= Instant::now() {
            return CancelResponseOutcome::NotFound;
        }
        if !record.background {
            return CancelResponseOutcome::NotCancellable;
        }
        record.cancelled = true;
        if !is_terminal_response_status(record.response.get("status").and_then(Value::as_str)) {
            set_response_status(&mut record.response, "cancelled");
            set_response_timestamp(&mut record.response, "cancelled_at", Utc::now().timestamp());
            if let Some(stream_state) = record.stream_state.as_mut() {
                let cancellation_event = json!({
                    "type": "response.cancelled",
                    "response": record.response.clone(),
                });
                let _ = stream_state.append_frame(Some("response.cancelled"), cancellation_event);
            }
        }
        CancelResponseOutcome::Response(record.response.clone())
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

    async fn current_continuation_cursor_response_id(
        &self,
        owner_key: &str,
        continuation_cursor_key: &str,
    ) -> Option<String> {
        let continuation_cursors = self.continuation_cursors.read().await;
        let cursor = continuation_cursors.get(continuation_cursor_key)?;
        if cursor.owner_key != owner_key || cursor.expires_at <= Instant::now() {
            return None;
        }
        Some(cursor.response_id.clone())
    }

    async fn store_continuation_cursor(
        &self,
        owner_key: String,
        continuation_cursor_key: String,
        response_id: String,
    ) {
        self.continuation_cursors.write().await.insert(
            continuation_cursor_key,
            ConversationCursor {
                owner_key,
                response_id,
                expires_at: Instant::now() + self.retention,
            },
        );
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
            input_items: derive_input_items(&request_body, response_id.as_str()),
            request_body: Some(request_body.clone()),
            cancelled: false,
            background: true,
            stream_state: request_body
                .get("stream")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                .then(StoredResponseStream::new),
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
        rewrite_response_ids(&mut response_value, response_id);
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

    async fn apply_background_response_snapshot(
        &self,
        response_id: &str,
        mut response_value: Value,
        conversation_id: Option<String>,
        terminal: bool,
    ) {
        let mut entries = self.entries.write().await;
        let Some(record) = entries.get_mut(response_id) else {
            return;
        };
        if record.cancelled {
            return;
        }
        if let Some(object) = response_value.as_object_mut() {
            object.insert("background".to_string(), Value::Bool(record.background));
            if let Some(conversation_id_value) = conversation_id.as_deref() {
                object
                    .entry("conversation".to_string())
                    .or_insert_with(|| Value::String(conversation_id_value.to_string()));
            }
        }
        let owner_key = record.owner_key.clone();
        let expires_at = record.expires_at;
        record.response = response_value.clone();
        if terminal {
            record.request_body = None;
        }
        drop(entries);

        if terminal {
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
                input_items: derive_input_items(request_body, response_id.as_str()),
                request_body: None,
                cancelled: false,
                background: false,
                stream_state: None,
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

    async fn append_background_stream_frame(
        &self,
        owner_key: &str,
        response_id: &str,
        frame: &[u8],
        conversation_id: Option<String>,
    ) {
        let parsed = parse_sse_frame(frame);
        let Some(event_value) = parsed
            .as_ref()
            .and_then(|(_, payload)| serde_json::from_slice::<Value>(payload).ok())
        else {
            return;
        };
        let mut event_value = event_value;
        rewrite_response_ids(&mut event_value, response_id);

        let (terminal, response_snapshot) = {
            let mut entries = self.entries.write().await;
            let Some(record) = entries.get_mut(response_id) else {
                return;
            };
            if record.owner_key != owner_key || record.expires_at <= Instant::now() {
                return;
            }
            let Some(stream_state) = record.stream_state.as_mut() else {
                return;
            };
            let (_, terminal, response_snapshot) =
                stream_state.append_frame(parsed.as_ref().and_then(|(name, _)| name.as_deref()), event_value);
            (terminal, response_snapshot)
        };

        if let Some(response_snapshot) = response_snapshot {
            self.apply_background_response_snapshot(
                response_id,
                response_snapshot,
                conversation_id,
                terminal,
            )
            .await;
        }
    }

    async fn stream_response(
        &self,
        owner_key: &str,
        response_id: &str,
        starting_after: Option<u64>,
    ) -> StreamLookupOutcome {
        let notify = {
            let entries = self.entries.read().await;
            let Some(record) = entries.get(response_id) else {
                return StreamLookupOutcome::NotFound;
            };
            if record.owner_key != owner_key || record.expires_at <= Instant::now() {
                return StreamLookupOutcome::NotFound;
            }
            let Some(stream_state) = record.stream_state.as_ref() else {
                return StreamLookupOutcome::NotStreamable;
            };
            stream_state.notify.clone()
        };

        let entries = self.entries.clone();
        let owner_key = owner_key.to_string();
        let response_id = response_id.to_string();
        let mut next_sequence = starting_after.unwrap_or(0) + 1;
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<Bytes, Infallible>>(16);

        tokio::spawn(async move {
            loop {
                let (pending_events, terminal) = {
                    let entries = entries.read().await;
                    let Some(record) = entries.get(response_id.as_str()) else {
                        return;
                    };
                    if record.owner_key != owner_key || record.expires_at <= Instant::now() {
                        return;
                    }
                    let Some(stream_state) = record.stream_state.as_ref() else {
                        return;
                    };
                    let pending_events = stream_state
                        .events
                        .iter()
                        .filter(|event| event.sequence_number >= next_sequence)
                        .cloned()
                        .collect::<Vec<_>>();
                    (pending_events, stream_state.terminal)
                };

                for event in pending_events {
                    next_sequence = event.sequence_number + 1;
                    if tx.send(Ok(event.bytes.clone())).await.is_err() {
                        return;
                    }
                }

                if terminal {
                    return;
                }
                notify.notified().await;
            }
        });

        let body = Body::from_stream(ReceiverStream::new(rx));
        let response = Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "text/event-stream")
            .header(axum::http::header::CACHE_CONTROL, "no-cache")
            .body(body)
            .unwrap_or_else(|_| Response::new(Body::empty()));
        StreamLookupOutcome::Response(response)
    }
}

pub async fn responses_retrieve_handler(
    State(state): State<Arc<AppState>>,
    principal: Option<axum::Extension<ApiPrincipal>>,
    Path(response_id): Path<String>,
    Query(query): Query<ResponseRetrieveQuery>,
) -> Response {
    let owner_key = response_owner_key(principal.as_ref().map(|item| &item.0));
    let _ = (&query.include, query.include_obfuscation);
    if query.stream.unwrap_or(false) {
        return match state
            .background_responses
            .stream_response(
                owner_key.as_str(),
                response_id.as_str(),
                query.starting_after,
            )
            .await
        {
            StreamLookupOutcome::Response(response) => response,
            StreamLookupOutcome::NotFound => localized_json_error_with_state(
                state.as_ref(),
                "en",
                StatusCode::NOT_FOUND,
                "response_not_found",
                "response was not found",
            ),
            StreamLookupOutcome::NotStreamable => localized_json_error_with_state(
                state.as_ref(),
                "en",
                StatusCode::BAD_REQUEST,
                "response_not_streamable",
                "response was not created as a background stream",
            ),
        };
    }
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

pub async fn responses_input_items_handler(
    State(state): State<Arc<AppState>>,
    principal: Option<axum::Extension<ApiPrincipal>>,
    Path(response_id): Path<String>,
    Query(query): Query<ResponseInputItemsQuery>,
) -> Response {
    let owner_key = response_owner_key(principal.as_ref().map(|item| &item.0));
    let _ = &query.include;
    match state
        .background_responses
        .lookup_input_items(owner_key.as_str(), response_id.as_str(), &query)
        .await
    {
        Some(items) => axum::Json(items).into_response(),
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
        CancelResponseOutcome::Response(response) => axum::Json(response).into_response(),
        CancelResponseOutcome::NotFound => localized_json_error_with_state(
            state.as_ref(),
            "en",
            StatusCode::NOT_FOUND,
            "response_not_found",
            "response was not found",
        ),
        CancelResponseOutcome::NotCancellable => localized_json_error_with_state(
            state.as_ref(),
            "en",
            StatusCode::BAD_REQUEST,
            "response_not_cancellable",
            "response was not created in background mode",
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
    if request_explicitly_disables_store(&request_value) {
        let response = localized_json_error_with_state(
            state.as_ref(),
            detected_locale,
            StatusCode::BAD_REQUEST,
            "background_requires_store",
            "background responses require store=true",
        );
        return Some(response);
    }
    let owner_key = response_owner_key(principal);
    let principal_token = principal.map(|item| item.token.clone()).unwrap_or_default();
    let conversation_id = parsed_policy_context.conversation_id.clone();
    let requested_stream = request_value
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if !requested_stream {
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

    let background_response_id = response_id.clone();
    let snapshot = BackgroundRequestSnapshot {
        response_id,
        owner_key: owner_key.clone(),
        principal_token,
        headers: collect_background_headers(headers),
        body: request_value,
        detected_locale: detected_locale.to_string(),
        conversation_id,
        stream: requested_stream,
    };
    spawn_background_response_worker(state.clone(), snapshot);

    if requested_stream {
        return match state
            .background_responses
            .stream_response(owner_key.as_str(), background_response_id.as_str(), None)
            .await
        {
            StreamLookupOutcome::Response(response) => Some(response),
            StreamLookupOutcome::NotFound | StreamLookupOutcome::NotStreamable => {
                Some(localized_json_error_with_state(
                    state.as_ref(),
                    detected_locale,
                    StatusCode::BAD_GATEWAY,
                    "background_stream_unavailable",
                    "background stream is unavailable",
                ))
            }
        };
    }

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

fn derive_input_items(request_body: &Value, response_id: &str) -> Vec<Value> {
    let base_items = match request_body.get("input") {
        Some(Value::String(text)) => vec![json!({
            "type": "message",
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": text
            }]
        })],
        Some(Value::Array(items)) => items.clone(),
        Some(Value::Object(object)) => vec![Value::Object(object.clone())],
        _ => Vec::new(),
    };

    base_items
        .into_iter()
        .enumerate()
        .map(|(index, item)| ensure_input_item_shape(item, response_id, index))
        .collect()
}

fn rewrite_response_ids(value: &mut Value, response_id: &str) {
    if let Some(object) = value.as_object_mut() {
        if object.contains_key("response_id") {
            object.insert(
                "response_id".to_string(),
                Value::String(response_id.to_string()),
            );
        }
        if let Some(response_object) = object.get_mut("response").and_then(Value::as_object_mut) {
            response_object.insert("id".to_string(), Value::String(response_id.to_string()));
        }
        if object
            .get("object")
            .and_then(Value::as_str)
            .is_some_and(|kind| kind == "response")
        {
            object.insert("id".to_string(), Value::String(response_id.to_string()));
        }
    }
}

fn ensure_input_item_shape(mut item: Value, response_id: &str, index: usize) -> Value {
    let generated_id = format!("item_{response_id}_{index}");
    if let Some(object) = item.as_object_mut() {
        if !object.contains_key("type") && object.contains_key("role") && object.contains_key("content")
        {
            object.insert("type".to_string(), Value::String("message".to_string()));
        }
        object
            .entry("id".to_string())
            .or_insert_with(|| Value::String(generated_id));
    }
    item
}

fn find_sse_frame_end(buffer: &[u8]) -> Option<usize> {
    for (index, window) in buffer.windows(4).enumerate() {
        if window == b"\r\n\r\n" {
            return Some(index + 4);
        }
    }
    for (index, window) in buffer.windows(2).enumerate() {
        if window == b"\n\n" {
            return Some(index + 2);
        }
    }
    None
}

fn parse_sse_frame(frame: &[u8]) -> Option<(Option<String>, Vec<u8>)> {
    let mut event_name: Option<String> = None;
    let mut payload_lines: Vec<Vec<u8>> = Vec::new();
    for raw_line in frame.split(|byte| *byte == b'\n') {
        let line = trim_ascii(raw_line);
        if line.is_empty() {
            continue;
        }
        if let Some(raw_event_name) = line.strip_prefix(b"event:") {
            let event = trim_ascii(raw_event_name);
            if !event.is_empty() {
                event_name = Some(String::from_utf8_lossy(event).to_string());
            }
            continue;
        }
        if let Some(raw_payload) = line.strip_prefix(b"data:") {
            let payload = trim_ascii(raw_payload);
            if !payload.is_empty() {
                payload_lines.push(payload.to_vec());
            }
            continue;
        }
        payload_lines.push(line.to_vec());
    }
    if payload_lines.is_empty() {
        return None;
    }
    Some((event_name, payload_lines.join(&b'\n')))
}

fn build_sse_frame(event_name: Option<&str>, event_value: &Value) -> Bytes {
    let mut frame = String::new();
    if let Some(event_name) = event_name {
        frame.push_str("event: ");
        frame.push_str(event_name);
        frame.push('\n');
    }
    frame.push_str("data: ");
    frame.push_str(&event_value.to_string());
    frame.push_str("\n\n");
    Bytes::from(frame)
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
    let continuation_cursor_key = request_value
        .get("prompt_cache_key")
        .and_then(Value::as_str)
        .or_else(|| {
            request_value
                .get("metadata")
                .and_then(|meta| meta.get("session_id"))
                .and_then(Value::as_str)
        })
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let response_id = serde_json::from_slice::<Value>(response_body)
        .ok()
        .and_then(|value| value.get("id").and_then(Value::as_str).map(ToString::to_string));
    let owner_key = response_owner_key(principal);
    state
        .background_responses
        .store_completed_response(
            owner_key.clone(),
            &request_value,
            response_body,
            parsed_policy_context.conversation_id.clone(),
            force_store,
        )
        .await;
    if let (Some(continuation_cursor_key), Some(response_id)) =
        (continuation_cursor_key.as_deref(), response_id.as_deref())
    {
        state
            .background_responses
            .store_continuation_cursor(
                owner_key.clone(),
                continuation_cursor_key.to_string(),
                response_id.to_string(),
            )
            .await;
        emit_continuation_cursor_system_event(
            state,
            "continuation_cursor_saved",
            SystemEventSeverity::Info,
            None,
            parsed_policy_context.request_id.as_deref(),
            parsed_policy_context.model.as_deref(),
            Some("/v1/responses"),
            Some("POST"),
            continuation_cursor_key,
            Some(response_id),
            Some(owner_key.as_str()),
            Some("saved continuation cursor for HTTP responses replay"),
        )
        .await;
    }
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
            object.insert("stream".to_string(), Value::Bool(snapshot.stream));
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
                if status.is_success() && snapshot.stream {
                    let mut body_stream = response.bytes_stream();
                    let mut buffer = Vec::new();
                    let mut saw_terminal = false;
                    while let Some(item) = body_stream.next().await {
                        match item {
                            Ok(chunk) => {
                                buffer.extend_from_slice(&chunk);
                                while let Some(frame_end) = find_sse_frame_end(&buffer) {
                                    let frame = buffer.drain(..frame_end).collect::<Vec<_>>();
                                    state
                                        .background_responses
                                        .append_background_stream_frame(
                                            snapshot.owner_key.as_str(),
                                            snapshot.response_id.as_str(),
                                            &frame,
                                            snapshot.conversation_id.clone(),
                                        )
                                        .await;
                                    let status = state
                                        .background_responses
                                        .lookup_response(
                                            snapshot.owner_key.as_str(),
                                            snapshot.response_id.as_str(),
                                        )
                                        .await
                                        .and_then(|value| {
                                            value.get("status")
                                                .and_then(Value::as_str)
                                                .map(ToString::to_string)
                                        });
                                    if status
                                        .as_deref()
                                        .is_some_and(|status| is_terminal_response_status(Some(status)))
                                    {
                                        saw_terminal = true;
                                    }
                                }
                            }
                            Err(err) => {
                                warn!(
                                    error = %err,
                                    response_id = %snapshot.response_id,
                                    "background response stream self-request failed during read"
                                );
                                state
                                    .background_responses
                                    .apply_background_failure(
                                        snapshot.response_id.as_str(),
                                        &snapshot.body,
                                        StatusCode::BAD_GATEWAY,
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
                        }
                    }

                    if let Some(frame_end) = find_sse_frame_end(&buffer).or_else(|| {
                        (!trim_ascii(&buffer).is_empty()).then_some(buffer.len())
                    }) {
                        let frame = buffer.drain(..frame_end).collect::<Vec<_>>();
                        state
                            .background_responses
                            .append_background_stream_frame(
                                snapshot.owner_key.as_str(),
                                snapshot.response_id.as_str(),
                                &frame,
                                snapshot.conversation_id.clone(),
                            )
                            .await;
                    }

                    if !saw_terminal {
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
                } else {
                    let response_body = response.bytes().await.ok();
                    if status.is_success() {
                        if let Some(response_body) = response_body {
                            if let Ok(response_value) = serde_json::from_slice::<Value>(&response_body)
                            {
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
