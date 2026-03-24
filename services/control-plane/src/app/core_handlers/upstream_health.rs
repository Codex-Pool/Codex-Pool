#[cfg(feature = "redis-backend")]
const UPSTREAM_HEALTH_REDIS_PREFIX_ENV: &str = "CONTROL_PLANE_HEALTH_REDIS_PREFIX";
#[cfg(feature = "redis-backend")]
const DEFAULT_UPSTREAM_HEALTH_REDIS_PREFIX: &str = "codex_pool:health";
#[cfg(feature = "redis-backend")]
const UPSTREAM_HEALTH_ALIVE_RING_SIZE_ENV: &str = "CONTROL_PLANE_ALIVE_RING_SIZE";
#[cfg(feature = "redis-backend")]
const DEFAULT_UPSTREAM_HEALTH_ALIVE_RING_SIZE: usize = 5_000;
const UPSTREAM_SEEN_OK_MIN_WRITE_INTERVAL_SEC_ENV: &str =
    "CONTROL_PLANE_UPSTREAM_SEEN_OK_MIN_WRITE_INTERVAL_SEC";
const DEFAULT_UPSTREAM_SEEN_OK_MIN_WRITE_INTERVAL_SEC: i64 = 10;
const UPSTREAM_SEEN_OK_RATE_LIMIT_REFRESH_ENABLED_ENV: &str =
    "CONTROL_PLANE_UPSTREAM_SEEN_OK_RATE_LIMIT_REFRESH_ENABLED";
const DEFAULT_UPSTREAM_SEEN_OK_RATE_LIMIT_REFRESH_ENABLED: bool = true;
const LIVE_RESULT_LOG_MESSAGE_MAX_CHARS: usize = 240;

#[cfg(feature = "redis-backend")]
#[derive(Clone)]
struct UpstreamAliveRingClient {
    client: redis::Client,
    key: String,
    max_size: i64,
}

#[cfg(not(feature = "redis-backend"))]
#[derive(Clone)]
struct UpstreamAliveRingClient;

#[cfg(feature = "redis-backend")]
impl UpstreamAliveRingClient {
    fn from_redis_url(redis_url: &str) -> Option<Self> {
        let client = redis::Client::open(redis_url).ok()?;
        let prefix = std::env::var(UPSTREAM_HEALTH_REDIS_PREFIX_ENV)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_UPSTREAM_HEALTH_REDIS_PREFIX.to_string());
        let max_size = std::env::var(UPSTREAM_HEALTH_ALIVE_RING_SIZE_ENV)
            .ok()
            .and_then(|raw| raw.parse::<usize>().ok())
            .unwrap_or(DEFAULT_UPSTREAM_HEALTH_ALIVE_RING_SIZE)
            .clamp(1, 100_000);
        Some(Self {
            client,
            key: format!("{prefix}:alive_ring:v1"),
            max_size: i64::try_from(max_size).unwrap_or(i64::MAX),
        })
    }

    async fn touch(&self, account_id: Uuid) -> anyhow::Result<()> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .context("failed to connect redis for alive ring touch")?;
        let _: () = redis::pipe()
            .atomic()
            .cmd("LREM")
            .arg(&self.key)
            .arg(0)
            .arg(account_id.to_string())
            .ignore()
            .cmd("LPUSH")
            .arg(&self.key)
            .arg(account_id.to_string())
            .ignore()
            .cmd("LTRIM")
            .arg(&self.key)
            .arg(0)
            .arg(self.max_size - 1)
            .ignore()
            .query_async(&mut conn)
            .await
            .context("failed to update alive ring")?;
        Ok(())
    }
}

#[cfg(not(feature = "redis-backend"))]
impl UpstreamAliveRingClient {
    async fn touch(&self, _account_id: Uuid) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(feature = "redis-backend")]
fn build_alive_ring_client_from_state(state: &AppState) -> Option<UpstreamAliveRingClient> {
    let redis_url = state
        .runtime_config
        .read()
        .ok()
        .and_then(|runtime| runtime.redis_url.clone())?;
    UpstreamAliveRingClient::from_redis_url(&redis_url)
}

#[cfg(not(feature = "redis-backend"))]
fn build_alive_ring_client_from_state(_state: &AppState) -> Option<UpstreamAliveRingClient> {
    None
}

fn upstream_seen_ok_min_write_interval_sec_from_env() -> i64 {
    std::env::var(UPSTREAM_SEEN_OK_MIN_WRITE_INTERVAL_SEC_ENV)
        .ok()
        .and_then(|raw| raw.parse::<i64>().ok())
        .unwrap_or(DEFAULT_UPSTREAM_SEEN_OK_MIN_WRITE_INTERVAL_SEC)
        .clamp(0, 3600)
}

// `seen_ok` 触发的是“账号刚刚成功服务过流量”的被动刷新，只补当前 in-use 账号；
// 它和全池定时 sweep 是两个独立概念，所以必须有单独开关，避免误以为关掉 sweep
// 就等于不需要 in-use 自动刷新。
fn upstream_seen_ok_rate_limit_refresh_enabled_from_env() -> bool {
    std::env::var(UPSTREAM_SEEN_OK_RATE_LIMIT_REFRESH_ENABLED_ENV)
        .ok()
        .and_then(|raw| parse_bool_flag(&raw))
        .unwrap_or(DEFAULT_UPSTREAM_SEEN_OK_RATE_LIMIT_REFRESH_ENABLED)
}

fn summarize_live_result_error_message_for_log(value: Option<&str>) -> String {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return "none".to_string();
    };

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(value) {
        if let Some(event_type) = json.get("type").and_then(|item| item.as_str()) {
            return format!("upstream_event:{event_type}");
        }
        if let Some(error_code) = json
            .get("error")
            .and_then(|error| error.get("code"))
            .and_then(|item| item.as_str())
            .filter(|value| !value.trim().is_empty())
        {
            return format!("upstream_error:{error_code}");
        }
        if let Some(error_message) = json
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(|item| item.as_str())
        {
            return truncate_live_result_log_text(error_message);
        }
        if let Some(object) = json.as_object() {
            let mut keys = object.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            let keys = keys.into_iter().take(6).collect::<Vec<_>>().join(",");
            if !keys.is_empty() {
                return format!("json_payload:{keys}");
            }
        }
    }

    truncate_live_result_log_text(value)
}

fn truncate_live_result_log_text(value: &str) -> String {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = collapsed.chars();
    let truncated = chars
        .by_ref()
        .take(LIVE_RESULT_LOG_MESSAGE_MAX_CHARS)
        .collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}...[truncated]")
    } else {
        truncated
    }
}

#[derive(Debug, Deserialize)]
struct InternalModelSeenOkRequest {
    model: String,
    #[serde(default)]
    status_code: Option<u16>,
}

#[derive(Debug, Deserialize)]
struct InternalObservedRateLimitRequest {
    #[serde(default)]
    observed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    rate_limits: Vec<InternalObservedRateLimitSnapshot>,
}

#[derive(Debug, Deserialize)]
struct InternalObservedRateLimitSnapshot {
    #[serde(default)]
    limit_id: Option<String>,
    #[serde(default)]
    limit_name: Option<String>,
    #[serde(default)]
    primary: Option<InternalObservedRateLimitWindow>,
    #[serde(default)]
    secondary: Option<InternalObservedRateLimitWindow>,
}

#[derive(Debug, Deserialize)]
struct InternalObservedRateLimitWindow {
    used_percent: f64,
    #[serde(default)]
    window_minutes: Option<i64>,
    #[serde(default)]
    resets_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum InternalLiveResultStatus {
    Ok,
    Failed,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum InternalLiveResultSource {
    Active,
    Passive,
}

#[derive(Debug, Deserialize)]
struct InternalLiveResultRequest {
    status: InternalLiveResultStatus,
    source: InternalLiveResultSource,
    #[serde(default)]
    status_code: Option<u16>,
    #[serde(default)]
    error_code: Option<String>,
    #[serde(default)]
    error_message: Option<String>,
    #[serde(default)]
    upstream_request_id: Option<String>,
    #[serde(default)]
    model: Option<String>,
}

#[derive(Debug, Serialize)]
struct InternalSeenOkResponse {
    ok: bool,
    accepted: bool,
    account_id: Uuid,
    seen_ok_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct InternalModelSeenOkResponse {
    ok: bool,
    accepted: bool,
    account_id: Uuid,
    model: String,
    seen_ok_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct InternalObservedRateLimitResponse {
    ok: bool,
    account_id: Uuid,
    observed_at: DateTime<Utc>,
    persisted_limits: usize,
}

#[derive(Debug, Serialize)]
struct InternalLiveResultResponse {
    ok: bool,
    accepted: bool,
    account_id: Uuid,
    status: InternalLiveResultStatus,
    source: InternalLiveResultSource,
    reported_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status_code: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<String>,
}

async fn apply_seen_ok_signal(
    state: &AppState,
    account_id: Uuid,
) -> Result<(DateTime<Utc>, bool), (StatusCode, Json<ErrorEnvelope>)> {
    let seen_ok_at = Utc::now();
    let accepted = state
        .store
        .mark_account_seen_ok(
            account_id,
            seen_ok_at,
            upstream_seen_ok_min_write_interval_sec_from_env(),
        )
        .await
        .map_err(internal_error)?;

    if accepted {
        if let Some(alive_ring) = build_alive_ring_client_from_state(&state) {
            if let Err(err) = alive_ring.touch(account_id).await {
                tracing::warn!(
                    error = %err,
                    account_id = %account_id,
                    "failed to push seen_ok account into alive ring"
                );
            }
        }
        if upstream_seen_ok_rate_limit_refresh_enabled_from_env() {
            let store = state.store.clone();
            tokio::spawn(async move {
                if let Err(err) = store
                    .maybe_refresh_oauth_rate_limit_cache_on_seen_ok(account_id)
                    .await
                {
                    tracing::warn!(
                        error = %err,
                        account_id = %account_id,
                        "failed to refresh oauth rate-limit cache after seen_ok"
                    );
                }
            });
        }
    }

    Ok((seen_ok_at, accepted))
}

async fn internal_mark_upstream_account_seen_ok(
    Path(account_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<InternalSeenOkResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    require_internal_service_token(&state, &headers)?;

    let (seen_ok_at, accepted) = apply_seen_ok_signal(&state, account_id).await?;

    Ok(Json(InternalSeenOkResponse {
        ok: true,
        accepted,
        account_id,
        seen_ok_at,
    }))
}

async fn internal_report_upstream_account_live_result(
    Path(account_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<InternalLiveResultRequest>,
) -> Result<Json<InternalLiveResultResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    require_internal_service_token(&state, &headers)?;

    let reported_at = Utc::now();
    let normalized_error_code = req.error_code.as_ref().and_then(|raw| {
        let normalized = raw.trim().to_ascii_lowercase().replace([' ', '-'], "_");
        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    });
    let accepted = match req.status {
        InternalLiveResultStatus::Ok => state
            .store
            .record_upstream_account_live_result(
                account_id,
                reported_at,
                crate::contracts::OAuthLiveResultStatus::Ok,
                match req.source {
                    InternalLiveResultSource::Active => crate::contracts::OAuthLiveResultSource::Active,
                    InternalLiveResultSource::Passive => {
                        crate::contracts::OAuthLiveResultSource::Passive
                    }
                },
                req.status_code,
                None,
                None,
            )
            .await
            .map_err(|err| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorEnvelope::new("store_error", err.to_string())),
                )
            })?,
        InternalLiveResultStatus::Failed => {
            let error_message_for_log =
                summarize_live_result_error_message_for_log(req.error_message.as_deref());
            tracing::info!(
                account_id = %account_id,
                source = ?req.source,
                status_code = req.status_code,
                error_code = normalized_error_code.as_deref().unwrap_or_default(),
                error_message = error_message_for_log.as_str(),
                upstream_request_id = req.upstream_request_id.as_deref().unwrap_or_default(),
                model = req.model.as_deref().unwrap_or_default(),
                "received upstream live-result failure signal"
            );
            state
                .store
                .record_upstream_account_live_result(
                    account_id,
                    reported_at,
                    crate::contracts::OAuthLiveResultStatus::Failed,
                    match req.source {
                        InternalLiveResultSource::Active => {
                            crate::contracts::OAuthLiveResultSource::Active
                        }
                        InternalLiveResultSource::Passive => {
                            crate::contracts::OAuthLiveResultSource::Passive
                        }
                    },
                    req.status_code,
                    normalized_error_code.clone(),
                    req.error_message.clone(),
                )
                .await
                .map_err(|err| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorEnvelope::new("store_error", err.to_string())),
                    )
                })?
        }
    };

    Ok(Json(InternalLiveResultResponse {
        ok: true,
        accepted,
        account_id,
        status: req.status,
        source: req.source,
        reported_at,
        status_code: req.status_code,
        error_code: normalized_error_code,
    }))
}

async fn internal_mark_upstream_model_seen_ok(
    Path(account_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<InternalModelSeenOkRequest>,
) -> Result<Json<InternalModelSeenOkResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    require_internal_service_token(&state, &headers)?;

    let model = req.model.trim();
    if model.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorEnvelope::new("invalid_model", "model is required")),
        ));
    }

    let seen_ok_at = Utc::now();
    let http_status = req
        .status_code
        .filter(|status| (200..600).contains(status))
        .unwrap_or(200);
    {
        let mut cache = state
            .model_probe_cache
            .write()
            .expect("model_probe_cache lock poisoned");
        mark_model_available_in_probe_cache(&mut cache, model, seen_ok_at, http_status);
    }

    Ok(Json(InternalModelSeenOkResponse {
        ok: true,
        accepted: true,
        account_id,
        model: model.to_string(),
        seen_ok_at,
    }))
}

async fn internal_update_observed_rate_limits(
    Path(account_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<InternalObservedRateLimitRequest>,
) -> Result<Json<InternalObservedRateLimitResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    require_internal_service_token(&state, &headers)?;

    let observed_at = req.observed_at.unwrap_or_else(Utc::now);
    let rate_limits = req
        .rate_limits
        .into_iter()
        .map(|snapshot| OAuthRateLimitSnapshot {
            limit_id: snapshot.limit_id,
            limit_name: snapshot.limit_name,
            primary: snapshot.primary.map(|window| OAuthRateLimitWindow {
                used_percent: window.used_percent,
                window_minutes: window.window_minutes,
                resets_at: window.resets_at,
            }),
            secondary: snapshot.secondary.map(|window| OAuthRateLimitWindow {
                used_percent: window.used_percent,
                window_minutes: window.window_minutes,
                resets_at: window.resets_at,
            }),
        })
        .collect::<Vec<_>>();

    let persisted_limits = rate_limits.len();
    state
        .store
        .update_oauth_rate_limit_cache_from_observation(account_id, rate_limits, observed_at)
        .await
        .map_err(internal_error)?;

    Ok(Json(InternalObservedRateLimitResponse {
        ok: true,
        account_id,
        observed_at,
        persisted_limits,
    }))
}

#[cfg(test)]
mod upstream_health_logging_tests {
    use super::summarize_live_result_error_message_for_log;

    #[test]
    fn summarize_live_result_error_message_for_log_collapses_large_response_payload() {
        let summary = summarize_live_result_error_message_for_log(Some(
            r#"{"type":"response.created","response":{"id":"resp_123","model":"gpt-5.4","messages":[{"role":"user","content":"secret prompt"}]}}"#,
        ));

        assert!(summary.contains("response.created"));
        assert!(!summary.contains("secret prompt"));
        assert!(!summary.contains("\"messages\":["));
        assert!(!summary.contains("\"response\":{\"id\""));
    }
}
