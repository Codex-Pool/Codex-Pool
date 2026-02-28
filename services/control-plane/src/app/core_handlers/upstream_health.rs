const UPSTREAM_OP_TYPE_PROBE: &str = "probe";
const UPSTREAM_HEALTH_REDIS_PREFIX_ENV: &str = "CONTROL_PLANE_HEALTH_REDIS_PREFIX";
const DEFAULT_UPSTREAM_HEALTH_REDIS_PREFIX: &str = "codex_pool:health";
const UPSTREAM_HEALTH_ALIVE_RING_SIZE_ENV: &str = "CONTROL_PLANE_ALIVE_RING_SIZE";
const DEFAULT_UPSTREAM_HEALTH_ALIVE_RING_SIZE: usize = 5_000;
const UPSTREAM_PROBE_ENABLED_ENV: &str = "CONTROL_PLANE_UPSTREAM_PROBE_ENABLED";
const UPSTREAM_PROBE_TICK_SEC_ENV: &str = "CONTROL_PLANE_UPSTREAM_PROBE_TICK_SEC";
const DEFAULT_UPSTREAM_PROBE_TICK_SEC: u64 = 10;
const UPSTREAM_PROBE_BATCH_SIZE_ENV: &str = "CONTROL_PLANE_UPSTREAM_PROBE_BATCH_SIZE";
const DEFAULT_UPSTREAM_PROBE_BATCH_SIZE: usize = 100;
const UPSTREAM_PROBE_SEEN_OK_SUPPRESS_SEC_ENV: &str =
    "CONTROL_PLANE_UPSTREAM_PROBE_SEEN_OK_SUPPRESS_SEC";
const DEFAULT_UPSTREAM_PROBE_SEEN_OK_SUPPRESS_SEC: i64 = 600;
const UPSTREAM_PROBE_LOCK_TTL_SEC_ENV: &str = "CONTROL_PLANE_UPSTREAM_PROBE_LOCK_TTL_SEC";
const DEFAULT_UPSTREAM_PROBE_LOCK_TTL_SEC: i64 = 30;
const UPSTREAM_PROBE_TIMEOUT_MS_ENV: &str = "CONTROL_PLANE_UPSTREAM_PROBE_TIMEOUT_MS";
const DEFAULT_UPSTREAM_PROBE_TIMEOUT_MS: u64 = 3_000;
const UPSTREAM_PROBE_OK_INTERVAL_SEC_ENV: &str = "CONTROL_PLANE_UPSTREAM_PROBE_OK_INTERVAL_SEC";
const DEFAULT_UPSTREAM_PROBE_OK_INTERVAL_SEC: i64 = 300;
const UPSTREAM_PROBE_FAIL_MIN_INTERVAL_SEC_ENV: &str =
    "CONTROL_PLANE_UPSTREAM_PROBE_FAIL_MIN_INTERVAL_SEC";
const DEFAULT_UPSTREAM_PROBE_FAIL_MIN_INTERVAL_SEC: i64 = 30;
const UPSTREAM_PROBE_FAIL_MAX_INTERVAL_SEC_ENV: &str =
    "CONTROL_PLANE_UPSTREAM_PROBE_FAIL_MAX_INTERVAL_SEC";
const DEFAULT_UPSTREAM_PROBE_FAIL_MAX_INTERVAL_SEC: i64 = 3600;
const UPSTREAM_SEEN_OK_MIN_WRITE_INTERVAL_SEC_ENV: &str =
    "CONTROL_PLANE_UPSTREAM_SEEN_OK_MIN_WRITE_INTERVAL_SEC";
const DEFAULT_UPSTREAM_SEEN_OK_MIN_WRITE_INTERVAL_SEC: i64 = 10;
const PROBE_MODEL: &str = "gpt-5.1-codex-mini";
const PROBE_INPUT: &str = "PING";
const PROBE_MAX_OUTPUT_TOKENS: i64 = 1;

#[derive(Debug, Clone)]
struct UpstreamProbeRuntimeConfig {
    enabled: bool,
    tick_sec: u64,
    batch_size: usize,
    seen_ok_suppress_sec: i64,
    lock_ttl_sec: i64,
    timeout_ms: u64,
    ok_interval_sec: i64,
    fail_min_interval_sec: i64,
    fail_max_interval_sec: i64,
    seen_ok_min_write_interval_sec: i64,
}

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

    async fn remove(&self, account_id: Uuid) -> anyhow::Result<()> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .context("failed to connect redis for alive ring remove")?;
        let _: i64 = redis::cmd("LREM")
            .arg(&self.key)
            .arg(0)
            .arg(account_id.to_string())
            .query_async(&mut conn)
            .await
            .context("failed to remove account from alive ring")?;
        Ok(())
    }
}

fn upstream_probe_runtime_config_from_env() -> UpstreamProbeRuntimeConfig {
    let enabled = std::env::var(UPSTREAM_PROBE_ENABLED_ENV)
        .ok()
        .and_then(|raw| parse_bool_flag(&raw))
        .unwrap_or(true);
    let tick_sec = std::env::var(UPSTREAM_PROBE_TICK_SEC_ENV)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(DEFAULT_UPSTREAM_PROBE_TICK_SEC)
        .clamp(1, 300);
    let batch_size = std::env::var(UPSTREAM_PROBE_BATCH_SIZE_ENV)
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(DEFAULT_UPSTREAM_PROBE_BATCH_SIZE)
        .clamp(1, 5000);
    let seen_ok_suppress_sec = std::env::var(UPSTREAM_PROBE_SEEN_OK_SUPPRESS_SEC_ENV)
        .ok()
        .and_then(|raw| raw.parse::<i64>().ok())
        .unwrap_or(DEFAULT_UPSTREAM_PROBE_SEEN_OK_SUPPRESS_SEC)
        .clamp(0, 86_400);
    let lock_ttl_sec = std::env::var(UPSTREAM_PROBE_LOCK_TTL_SEC_ENV)
        .ok()
        .and_then(|raw| raw.parse::<i64>().ok())
        .unwrap_or(DEFAULT_UPSTREAM_PROBE_LOCK_TTL_SEC)
        .clamp(1, 600);
    let timeout_ms = std::env::var(UPSTREAM_PROBE_TIMEOUT_MS_ENV)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(DEFAULT_UPSTREAM_PROBE_TIMEOUT_MS)
        .clamp(200, 30_000);
    let ok_interval_sec = std::env::var(UPSTREAM_PROBE_OK_INTERVAL_SEC_ENV)
        .ok()
        .and_then(|raw| raw.parse::<i64>().ok())
        .unwrap_or(DEFAULT_UPSTREAM_PROBE_OK_INTERVAL_SEC)
        .clamp(5, 86_400);
    let fail_min_interval_sec = std::env::var(UPSTREAM_PROBE_FAIL_MIN_INTERVAL_SEC_ENV)
        .ok()
        .and_then(|raw| raw.parse::<i64>().ok())
        .unwrap_or(DEFAULT_UPSTREAM_PROBE_FAIL_MIN_INTERVAL_SEC)
        .clamp(1, 86_400);
    let fail_max_interval_sec = std::env::var(UPSTREAM_PROBE_FAIL_MAX_INTERVAL_SEC_ENV)
        .ok()
        .and_then(|raw| raw.parse::<i64>().ok())
        .unwrap_or(DEFAULT_UPSTREAM_PROBE_FAIL_MAX_INTERVAL_SEC)
        .clamp(fail_min_interval_sec, 86_400 * 7);
    let seen_ok_min_write_interval_sec = std::env::var(UPSTREAM_SEEN_OK_MIN_WRITE_INTERVAL_SEC_ENV)
        .ok()
        .and_then(|raw| raw.parse::<i64>().ok())
        .unwrap_or(DEFAULT_UPSTREAM_SEEN_OK_MIN_WRITE_INTERVAL_SEC)
        .clamp(0, 3600);

    UpstreamProbeRuntimeConfig {
        enabled,
        tick_sec,
        batch_size,
        seen_ok_suppress_sec,
        lock_ttl_sec,
        timeout_ms,
        ok_interval_sec,
        fail_min_interval_sec,
        fail_max_interval_sec,
        seen_ok_min_write_interval_sec,
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

fn probe_jitter_seconds(account_id: Uuid, modulo: i64) -> i64 {
    if modulo <= 0 {
        return 0;
    }
    i64::try_from(account_id.as_u128() % u128::try_from(modulo).unwrap_or(1)).unwrap_or(0)
}

fn schedule_probe_ok_next(now: DateTime<Utc>, account_id: Uuid, ok_interval_sec: i64) -> DateTime<Utc> {
    let jitter = probe_jitter_seconds(account_id, 7);
    now + chrono::Duration::seconds(ok_interval_sec.saturating_add(jitter))
}

fn schedule_probe_fail_next(
    now: DateTime<Utc>,
    account_id: Uuid,
    prior_failure_count: u32,
    min_interval_sec: i64,
    max_interval_sec: i64,
) -> DateTime<Utc> {
    let capped_shift = prior_failure_count.min(8);
    let factor = 1_i64.checked_shl(capped_shift).unwrap_or(i64::MAX);
    let base = min_interval_sec.saturating_mul(factor).clamp(min_interval_sec, max_interval_sec);
    let jitter = probe_jitter_seconds(account_id, 13);
    now + chrono::Duration::seconds(base.saturating_add(jitter).min(max_interval_sec))
}

fn truncate_probe_error(raw: String) -> String {
    const MAX_LEN: usize = 256;
    if raw.len() <= MAX_LEN {
        return raw;
    }
    raw.chars().take(MAX_LEN).collect()
}

fn parse_probe_error_code(body: &str) -> Option<String> {
    let parsed = serde_json::from_str::<serde_json::Value>(body).ok()?;
    parsed
        .get("error")
        .and_then(|value| value.get("code"))
        .and_then(|value| value.as_str())
        .map(ToString::to_string)
        .or_else(|| {
            parsed
                .get("code")
                .and_then(|value| value.as_str())
                .map(ToString::to_string)
        })
}

async fn probe_single_account(
    account: UpstreamAccount,
    prior_failure_count: u32,
    client: &reqwest::Client,
    config: &UpstreamProbeRuntimeConfig,
) -> UpstreamProbeWrite {
    let now = Utc::now();
    let responses_url = match crate::upstream_api::build_upstream_responses_url(
        &account.base_url,
        &account.mode,
    ) {
        Ok(url) => url,
        Err(err) => {
            return UpstreamProbeWrite {
                status: UpstreamProbeStatus::Fail,
                observed_at: now,
                next_probe_at: schedule_probe_fail_next(
                    now,
                    account.id,
                    prior_failure_count.saturating_add(1),
                    config.fail_min_interval_sec,
                    config.fail_max_interval_sec,
                ),
                http_status: None,
                error_code: Some("invalid_upstream_url".to_string()),
                error_message: Some(truncate_probe_error(err.to_string())),
            };
        }
    };

    let mut request = client
        .post(responses_url)
        .bearer_auth(account.bearer_token)
        .json(&json!({
            "model": PROBE_MODEL,
            "input": PROBE_INPUT,
            "max_output_tokens": PROBE_MAX_OUTPUT_TOKENS
        }));
    if let Some(chatgpt_account_id) = account.chatgpt_account_id.as_deref() {
        request = request.header("chatgpt-account-id", chatgpt_account_id);
    }

    match request.send().await {
        Ok(response) => {
            let status = response.status();
            if status.is_success() {
                UpstreamProbeWrite {
                    status: UpstreamProbeStatus::Ok,
                    observed_at: now,
                    next_probe_at: schedule_probe_ok_next(now, account.id, config.ok_interval_sec),
                    http_status: Some(status.as_u16()),
                    error_code: None,
                    error_message: None,
                }
            } else {
                let response_body = response.text().await.unwrap_or_default();
                UpstreamProbeWrite {
                    status: UpstreamProbeStatus::Fail,
                    observed_at: now,
                    next_probe_at: schedule_probe_fail_next(
                        now,
                        account.id,
                        prior_failure_count.saturating_add(1),
                        config.fail_min_interval_sec,
                        config.fail_max_interval_sec,
                    ),
                    http_status: Some(status.as_u16()),
                    error_code: parse_probe_error_code(&response_body)
                        .or_else(|| Some(format!("http_{}", status.as_u16()))),
                    error_message: Some(truncate_probe_error(response_body)),
                }
            }
        }
        Err(err) => UpstreamProbeWrite {
            status: UpstreamProbeStatus::Fail,
            observed_at: now,
            next_probe_at: schedule_probe_fail_next(
                now,
                account.id,
                prior_failure_count.saturating_add(1),
                config.fail_min_interval_sec,
                config.fail_max_interval_sec,
            ),
            http_status: None,
            error_code: Some("network_error".to_string()),
            error_message: Some(truncate_probe_error(err.to_string())),
        },
    }
}

async fn run_upstream_probe_cycle(
    state: &AppState,
    config: &UpstreamProbeRuntimeConfig,
    http_client: &reqwest::Client,
    alive_ring: Option<&UpstreamAliveRingClient>,
    worker_id: &str,
) -> anyhow::Result<()> {
    let claimed = state
        .store
        .claim_due_probe_accounts(
            config.batch_size,
            config.seen_ok_suppress_sec,
            config.lock_ttl_sec,
            worker_id,
        )
        .await?;
    if claimed.is_empty() {
        return Ok(());
    }

    let snapshot = state.store.snapshot().await?;
    let accounts = snapshot
        .accounts
        .into_iter()
        .filter(|account| account.enabled)
        .map(|account| (account.id, account))
        .collect::<HashMap<_, _>>();

    for item in claimed {
        let account_id = item.account_id;
        let write = if let Some(account) = accounts.get(&account_id).cloned() {
            probe_single_account(account, item.failure_count, http_client, config).await
        } else {
            let now = Utc::now();
            UpstreamProbeWrite {
                status: UpstreamProbeStatus::Fail,
                observed_at: now,
                next_probe_at: schedule_probe_fail_next(
                    now,
                    account_id,
                    item.failure_count.saturating_add(1),
                    config.fail_min_interval_sec,
                    config.fail_max_interval_sec,
                ),
                http_status: None,
                error_code: Some("account_not_probeable".to_string()),
                error_message: Some("account is not enabled in current snapshot".to_string()),
            }
        };

        if let Err(err) = state.store.record_upstream_probe(account_id, write.clone()).await {
            tracing::warn!(error = %err, account_id = %account_id, "failed to persist probe result");
        }

        if let Some(client) = alive_ring {
            let ring_result = match write.status {
                UpstreamProbeStatus::Ok => client.touch(account_id).await,
                UpstreamProbeStatus::Fail => client.remove(account_id).await,
            };
            if let Err(err) = ring_result {
                tracing::warn!(error = %err, account_id = %account_id, "failed to update alive ring");
            }
        }

        if let Err(err) = state
            .store
            .release_upstream_op_lock(account_id, UPSTREAM_OP_TYPE_PROBE)
            .await
        {
            tracing::warn!(error = %err, account_id = %account_id, "failed to release probe lock");
        }
    }

    Ok(())
}

fn spawn_upstream_probe_loop(state: AppState) {
    let config = upstream_probe_runtime_config_from_env();
    if !config.enabled {
        tracing::info!("upstream probe loop disabled");
        return;
    }

    let timeout = Duration::from_millis(config.timeout_ms);
    let http_client = match reqwest::Client::builder().timeout(timeout).build() {
        Ok(client) => client,
        Err(err) => {
            tracing::warn!(error = %err, "failed to build upstream probe http client");
            return;
        }
    };
    let alive_ring = build_alive_ring_client_from_state(&state);
    let worker_id = Uuid::new_v4().to_string();
    let log_tick_sec = config.tick_sec;
    let log_batch_size = config.batch_size;

    tokio::spawn(async move {
        if let Err(err) = run_upstream_probe_cycle(
            &state,
            &config,
            &http_client,
            alive_ring.as_ref(),
            &worker_id,
        )
        .await
        {
            tracing::warn!(error = %err, "initial upstream probe cycle failed");
        }

        let mut ticker = tokio::time::interval(Duration::from_secs(config.tick_sec));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let _ = ticker.tick().await;
        loop {
            let _ = ticker.tick().await;
            if let Err(err) = run_upstream_probe_cycle(
                &state,
                &config,
                &http_client,
                alive_ring.as_ref(),
                &worker_id,
            )
            .await
            {
                tracing::warn!(error = %err, "scheduled upstream probe cycle failed");
            }
        }
    });

    tracing::info!(
        tick_sec = log_tick_sec,
        batch_size = log_batch_size,
        "upstream probe loop started"
    );
}

#[derive(Debug, Serialize)]
struct InternalSeenOkResponse {
    ok: bool,
    accepted: bool,
    account_id: Uuid,
    seen_ok_at: DateTime<Utc>,
}

async fn internal_mark_upstream_account_seen_ok(
    Path(account_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<InternalSeenOkResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    require_internal_service_token(&state, &headers)?;

    let config = upstream_probe_runtime_config_from_env();
    let seen_ok_at = Utc::now();
    let accepted = state
        .store
        .mark_account_seen_ok(
            account_id,
            seen_ok_at,
            config.seen_ok_min_write_interval_sec,
        )
        .await
        .map_err(internal_error)?;

    if accepted {
        if let Some(alive_ring) = build_alive_ring_client_from_state(&state) {
            if let Err(err) = alive_ring.touch(account_id).await {
                tracing::warn!(error = %err, account_id = %account_id, "failed to push seen_ok account into alive ring");
            }
        }
    }

    Ok(Json(InternalSeenOkResponse {
        ok: true,
        accepted,
        account_id,
        seen_ok_at,
    }))
}
