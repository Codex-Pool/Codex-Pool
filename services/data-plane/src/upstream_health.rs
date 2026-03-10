use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use anyhow::Context;
use serde::Serialize;
use uuid::Uuid;

const ALIVE_RING_REDIS_PREFIX_ENV: &str = "DATA_PLANE_HEALTH_REDIS_PREFIX";
const DEFAULT_ALIVE_RING_REDIS_PREFIX: &str = "codex_pool:health";

#[derive(Debug, Clone)]
pub struct AliveRingConfig {
    pub enabled: bool,
    pub fetch_limit: usize,
    pub candidate_count: usize,
    pub cache_ttl: Duration,
    pub redis_prefix: String,
}

#[derive(Debug, Clone)]
pub struct SeenOkReportConfig {
    pub enabled: bool,
    pub min_interval: Duration,
    pub timeout: Duration,
}

#[derive(Debug, Clone)]
struct AliveRingCacheState {
    account_ids: Vec<Uuid>,
    cursor: usize,
    fetched_at: Option<Instant>,
}

#[derive(Clone)]
pub struct AliveRingRouter {
    client: redis::Client,
    key: String,
    fetch_limit: usize,
    candidate_count: usize,
    cache_ttl: Duration,
    cache_state: Arc<RwLock<AliveRingCacheState>>,
}

impl AliveRingRouter {
    pub fn new(
        redis_url: &str,
        redis_prefix: &str,
        fetch_limit: usize,
        candidate_count: usize,
        cache_ttl: Duration,
    ) -> anyhow::Result<Self> {
        let client = redis::Client::open(redis_url)
            .with_context(|| "failed to initialize alive ring redis client")?;
        Ok(Self {
            client,
            key: format!("{redis_prefix}:alive_ring:v1"),
            fetch_limit: fetch_limit.clamp(1, 50_000),
            candidate_count: candidate_count.clamp(1, 5_000),
            cache_ttl: cache_ttl.max(Duration::from_millis(100)),
            cache_state: Arc::new(RwLock::new(AliveRingCacheState {
                account_ids: Vec::new(),
                cursor: 0,
                fetched_at: None,
            })),
        })
    }

    async fn refresh_cache_if_needed(&self) -> anyhow::Result<()> {
        let should_refresh = {
            let state = self
                .cache_state
                .read()
                .map_err(|_| anyhow::anyhow!("alive ring cache lock poisoned"))?;
            match state.fetched_at {
                Some(fetched_at) => fetched_at.elapsed() >= self.cache_ttl,
                None => true,
            }
        };
        if !should_refresh {
            return Ok(());
        }

        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .with_context(|| "failed to connect redis for alive ring refresh")?;
        let raw_ids = redis::cmd("LRANGE")
            .arg(&self.key)
            .arg(0)
            .arg((self.fetch_limit.saturating_sub(1)) as i64)
            .query_async::<Vec<String>>(&mut conn)
            .await
            .with_context(|| "failed to fetch alive ring ids from redis")?;

        let mut parsed_ids = Vec::with_capacity(raw_ids.len());
        for raw in raw_ids {
            if let Ok(account_id) = Uuid::parse_str(raw.trim()) {
                parsed_ids.push(account_id);
            }
        }

        let mut state = self
            .cache_state
            .write()
            .map_err(|_| anyhow::anyhow!("alive ring cache lock poisoned"))?;
        state.account_ids = parsed_ids;
        state.cursor = 0;
        state.fetched_at = Some(Instant::now());
        Ok(())
    }

    pub async fn next_candidate_ids(&self) -> anyhow::Result<Vec<Uuid>> {
        self.refresh_cache_if_needed().await?;
        let mut state = self
            .cache_state
            .write()
            .map_err(|_| anyhow::anyhow!("alive ring cache lock poisoned"))?;
        if state.account_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut result = Vec::new();
        let len = state.account_ids.len();
        let max_take = self.candidate_count.min(len);
        for offset in 0..max_take {
            let idx = (state.cursor + offset) % len;
            result.push(state.account_ids[idx]);
        }
        state.cursor = (state.cursor + max_take) % len;
        Ok(result)
    }
}

#[derive(Debug, Serialize)]
struct ModelSeenOkReportRequest {
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    status_code: Option<u16>,
}

#[derive(Clone)]
pub struct SeenOkReporter {
    client: reqwest::Client,
    endpoint_base: String,
    internal_auth_token: Arc<str>,
    min_interval: Duration,
    latest_reported: Arc<RwLock<HashMap<Uuid, Instant>>>,
    latest_model_reported: Arc<RwLock<HashMap<(Uuid, String), Instant>>>,
}

impl SeenOkReporter {
    pub fn new(
        endpoint_base: String,
        internal_auth_token: Arc<str>,
        timeout: Duration,
        min_interval: Duration,
    ) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .with_context(|| "failed to build seen_ok reporter client")?;
        Ok(Self {
            client,
            endpoint_base: endpoint_base.trim_end_matches('/').to_string(),
            internal_auth_token,
            min_interval,
            latest_reported: Arc::new(RwLock::new(HashMap::new())),
            latest_model_reported: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    fn should_report(&self, account_id: Uuid) -> bool {
        let now = Instant::now();
        let mut latest = match self.latest_reported.write() {
            Ok(lock) => lock,
            Err(_) => return false,
        };
        if let Some(last) = latest.get(&account_id) {
            if now.duration_since(*last) < self.min_interval {
                return false;
            }
        }
        latest.insert(account_id, now);
        true
    }

    fn should_report_model(&self, account_id: Uuid, model: &str) -> bool {
        let model = model.trim();
        if model.is_empty() {
            return false;
        }

        let now = Instant::now();
        let mut latest = match self.latest_model_reported.write() {
            Ok(lock) => lock,
            Err(_) => return false,
        };
        let key = (account_id, model.to_string());
        if let Some(last) = latest.get(&key) {
            if now.duration_since(*last) < self.min_interval {
                return false;
            }
        }
        latest.insert(key, now);
        true
    }

    pub async fn report_seen_ok(&self, account_id: Uuid) {
        if !self.should_report(account_id) {
            return;
        }

        let endpoint = format!(
            "{}/internal/v1/upstream-accounts/{account_id}/health/seen-ok",
            self.endpoint_base
        );
        let request = self
            .client
            .post(endpoint)
            .bearer_auth(self.internal_auth_token.as_ref());
        if let Err(err) = request.send().await {
            tracing::warn!(
                error = %err,
                account_id = %account_id,
                "failed to report seen_ok signal to control plane"
            );
        }
    }

    pub async fn report_model_seen_ok(&self, account_id: Uuid, model: &str) {
        let model = model.trim();
        if !self.should_report_model(account_id, model) {
            return;
        }

        let endpoint = format!(
            "{}/internal/v1/upstream-accounts/{account_id}/models/seen-ok",
            self.endpoint_base
        );
        let request = self
            .client
            .post(endpoint)
            .bearer_auth(self.internal_auth_token.as_ref())
            .json(&ModelSeenOkReportRequest {
                model: model.to_string(),
                status_code: Some(200),
            });
        if let Err(err) = request.send().await {
            tracing::warn!(
                error = %err,
                account_id = %account_id,
                model,
                "failed to report model seen_ok signal to control plane"
            );
        }
    }
}

pub fn alive_ring_config_from_env() -> AliveRingConfig {
    let enabled = std::env::var("DATA_PLANE_ALIVE_RING_ROUTING_ENABLED")
        .ok()
        .and_then(|raw| parse_bool_flag(&raw))
        .unwrap_or(true);
    let fetch_limit = std::env::var("DATA_PLANE_ALIVE_RING_FETCH_LIMIT")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(200)
        .clamp(1, 50_000);
    let candidate_count = std::env::var("DATA_PLANE_ALIVE_RING_CANDIDATE_COUNT")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .unwrap_or(32)
        .clamp(1, 2000);
    let cache_ttl_ms = std::env::var("DATA_PLANE_ALIVE_RING_CACHE_TTL_MS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(2_000)
        .clamp(100, 30_000);
    let redis_prefix = std::env::var(ALIVE_RING_REDIS_PREFIX_ENV)
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
        .unwrap_or_else(|| DEFAULT_ALIVE_RING_REDIS_PREFIX.to_string());
    AliveRingConfig {
        enabled,
        fetch_limit,
        candidate_count,
        cache_ttl: Duration::from_millis(cache_ttl_ms),
        redis_prefix,
    }
}

pub fn seen_ok_report_config_from_env() -> SeenOkReportConfig {
    let enabled = std::env::var("DATA_PLANE_SEEN_OK_REPORT_ENABLED")
        .ok()
        .and_then(|raw| parse_bool_flag(&raw))
        .unwrap_or(true);
    let min_interval_sec = std::env::var("DATA_PLANE_SEEN_OK_REPORT_MIN_INTERVAL_SEC")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(60)
        .clamp(1, 3600);
    let timeout_ms = std::env::var("DATA_PLANE_SEEN_OK_REPORT_TIMEOUT_MS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(1_500)
        .clamp(200, 30_000);
    SeenOkReportConfig {
        enabled,
        min_interval: Duration::from_secs(min_interval_sec),
        timeout: Duration::from_millis(timeout_ms),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seen_ok_reporter_rate_limits_per_account_and_model() {
        let reporter = SeenOkReporter::new(
            "http://127.0.0.1:8090".to_string(),
            Arc::<str>::from("internal-token"),
            Duration::from_millis(250),
            Duration::from_secs(60),
        )
        .expect("reporter must build");
        let account_id = Uuid::new_v4();

        assert!(reporter.should_report_model(account_id, "gpt-5.3-codex"));
        assert!(!reporter.should_report_model(account_id, "gpt-5.3-codex"));
        assert!(reporter.should_report_model(account_id, "gpt-5.4"));
        assert!(reporter.should_report_model(Uuid::new_v4(), "gpt-5.3-codex"));
        assert!(!reporter.should_report_model(account_id, "   "));
    }
}

fn parse_bool_flag(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}
