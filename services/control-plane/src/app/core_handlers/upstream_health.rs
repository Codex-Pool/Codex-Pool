const UPSTREAM_HEALTH_REDIS_PREFIX_ENV: &str = "CONTROL_PLANE_HEALTH_REDIS_PREFIX";
const DEFAULT_UPSTREAM_HEALTH_REDIS_PREFIX: &str = "codex_pool:health";
const UPSTREAM_HEALTH_ALIVE_RING_SIZE_ENV: &str = "CONTROL_PLANE_ALIVE_RING_SIZE";
const DEFAULT_UPSTREAM_HEALTH_ALIVE_RING_SIZE: usize = 5_000;
const UPSTREAM_SEEN_OK_MIN_WRITE_INTERVAL_SEC_ENV: &str =
    "CONTROL_PLANE_UPSTREAM_SEEN_OK_MIN_WRITE_INTERVAL_SEC";
const DEFAULT_UPSTREAM_SEEN_OK_MIN_WRITE_INTERVAL_SEC: i64 = 10;
const UPSTREAM_SEEN_OK_RATE_LIMIT_REFRESH_ENABLED_ENV: &str =
    "CONTROL_PLANE_UPSTREAM_SEEN_OK_RATE_LIMIT_REFRESH_ENABLED";
const DEFAULT_UPSTREAM_SEEN_OK_RATE_LIMIT_REFRESH_ENABLED: bool = true;

#[derive(Clone)]
struct UpstreamAliveRingClient {
    client: redis::Client,
    key: String,
    max_size: i64,
}

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

fn build_alive_ring_client_from_state(state: &AppState) -> Option<UpstreamAliveRingClient> {
    let redis_url = state
        .runtime_config
        .read()
        .ok()
        .and_then(|runtime| runtime.redis_url.clone())?;
    UpstreamAliveRingClient::from_redis_url(&redis_url)
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

#[derive(Debug, Deserialize)]
struct InternalModelSeenOkRequest {
    model: String,
    #[serde(default)]
    status_code: Option<u16>,
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

async fn internal_mark_upstream_account_seen_ok(
    Path(account_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<InternalSeenOkResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    require_internal_service_token(&state, &headers)?;

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

    Ok(Json(InternalSeenOkResponse {
        ok: true,
        accepted,
        account_id,
        seen_ok_at,
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
