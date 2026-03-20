use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

const ALIVE_RING_REDIS_PREFIX_ENV: &str = "DATA_PLANE_HEALTH_REDIS_PREFIX";
const DEFAULT_ALIVE_RING_REDIS_PREFIX: &str = "codex_pool:health";
const SEEN_OK_FAILURE_BODY_PREVIEW_LIMIT: usize = 512;

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

#[cfg(feature = "redis-backend")]
#[derive(Debug, Clone)]
struct AliveRingCacheState {
    account_ids: Vec<Uuid>,
    cursor: usize,
    fetched_at: Option<Instant>,
}

#[cfg(feature = "redis-backend")]
#[derive(Clone)]
pub struct AliveRingRouter {
    client: redis::Client,
    key: String,
    fetch_limit: usize,
    candidate_count: usize,
    cache_ttl: Duration,
    cache_state: Arc<RwLock<AliveRingCacheState>>,
}

#[cfg(feature = "redis-backend")]
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

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LiveResultReportStatus {
    Ok,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LiveResultReportSource {
    Active,
    Passive,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LiveResultReportRequest {
    pub status: LiveResultReportStatus,
    pub source: LiveResultReportSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObservedRateLimitReportSource {
    SseHeaders,
    WebsocketEvent,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ObservedRateLimitWindow {
    pub used_percent: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_minutes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ObservedCreditsSnapshot {
    pub has_credits: bool,
    pub unlimited: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ObservedRateLimitSnapshot {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<ObservedRateLimitWindow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary: Option<ObservedRateLimitWindow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credits: Option<ObservedCreditsSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ObservedRateLimitReportRequest {
    pub source: ObservedRateLimitReportSource,
    pub observed_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rate_limits: Vec<ObservedRateLimitSnapshot>,
}

#[derive(Clone)]
pub struct SeenOkReporter {
    client: reqwest::Client,
    endpoint_base: String,
    internal_auth_token: Arc<str>,
    min_interval: Duration,
    latest_reported: Arc<RwLock<HashMap<Uuid, Instant>>>,
    latest_live_reported: Arc<RwLock<HashMap<Uuid, (Instant, String)>>>,
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
            latest_live_reported: Arc::new(RwLock::new(HashMap::new())),
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

    fn should_report_live_result(
        &self,
        account_id: Uuid,
        report: &LiveResultReportRequest,
    ) -> bool {
        let now = Instant::now();
        let signature = format!(
            "{:?}|{:?}|{:?}|{}|{}|{}",
            report.status,
            report.source,
            report.status_code,
            report.error_code.as_deref().unwrap_or_default(),
            report.error_message.as_deref().unwrap_or_default(),
            report.model.as_deref().unwrap_or_default(),
        );
        let mut latest = match self.latest_live_reported.write() {
            Ok(lock) => lock,
            Err(_) => return false,
        };
        if let Some((last, last_signature)) = latest.get(&account_id) {
            if *last_signature == signature && now.duration_since(*last) < self.min_interval {
                return false;
            }
        }
        latest.insert(account_id, (now, signature));
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
            .post(&endpoint)
            .bearer_auth(self.internal_auth_token.as_ref());
        self.log_if_report_failed(request, &endpoint, account_id, None, "seen_ok")
            .await;
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
            .post(&endpoint)
            .bearer_auth(self.internal_auth_token.as_ref())
            .json(&ModelSeenOkReportRequest {
                model: model.to_string(),
                status_code: Some(200),
            });
        self.log_if_report_failed(request, &endpoint, account_id, Some(model), "model seen_ok")
            .await;
    }

    pub async fn report_live_result(&self, account_id: Uuid, report: LiveResultReportRequest) {
        if !self.should_report_live_result(account_id, &report) {
            return;
        }

        let endpoint = format!(
            "{}/internal/v1/upstream-accounts/{account_id}/health/live-result",
            self.endpoint_base
        );
        let request = self
            .client
            .post(&endpoint)
            .bearer_auth(self.internal_auth_token.as_ref())
            .json(&report);
        self.log_if_report_failed(
            request,
            &endpoint,
            account_id,
            report.model.as_deref(),
            "live result",
        )
        .await;
    }

    pub async fn report_observed_rate_limits(
        &self,
        account_id: Uuid,
        report: ObservedRateLimitReportRequest,
    ) {
        if report.rate_limits.is_empty() {
            return;
        }

        let endpoint = format!(
            "{}/internal/v1/upstream-accounts/{account_id}/rate-limits/observed",
            self.endpoint_base
        );
        let request = self
            .client
            .post(&endpoint)
            .bearer_auth(self.internal_auth_token.as_ref())
            .json(&report);
        self.log_if_report_failed(request, &endpoint, account_id, None, "observed rate limits")
            .await;
    }

    async fn log_if_report_failed(
        &self,
        request: reqwest::RequestBuilder,
        endpoint: &str,
        account_id: Uuid,
        model: Option<&str>,
        signal_kind: &str,
    ) {
        match request.send().await {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    return;
                }
                let response_body = match response.text().await {
                    Ok(body) => sanitize_seen_ok_failure_body_for_log(&body),
                    Err(err) => format!("<failed to read response body: {err}>"),
                };
                if let Some(model) = model {
                    tracing::warn!(
                        account_id = %account_id,
                        model,
                        endpoint,
                        status_code = status.as_u16(),
                        response_body,
                        "failed to report {signal_kind} signal to control plane"
                    );
                } else {
                    tracing::warn!(
                        account_id = %account_id,
                        endpoint,
                        status_code = status.as_u16(),
                        response_body,
                        "failed to report {signal_kind} signal to control plane"
                    );
                }
            }
            Err(err) => {
                if let Some(model) = model {
                    tracing::warn!(
                        error = %err,
                        account_id = %account_id,
                        model,
                        endpoint,
                        "failed to report {signal_kind} signal to control plane"
                    );
                } else {
                    tracing::warn!(
                        error = %err,
                        account_id = %account_id,
                        endpoint,
                        "failed to report {signal_kind} signal to control plane"
                    );
                }
            }
        }
    }
}

fn sanitize_seen_ok_failure_body_for_log(raw: &str) -> String {
    let collapsed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return "<empty>".to_string();
    }

    let mut chars = collapsed.chars();
    let preview: String = chars
        .by_ref()
        .take(SEEN_OK_FAILURE_BODY_PREVIEW_LIMIT)
        .collect();
    if chars.next().is_some() {
        format!("{preview}...")
    } else {
        preview
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

fn parse_bool_flag(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io;
    use std::sync::{Arc, Mutex};

    use tracing_subscriber::fmt::writer::MakeWriter;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[derive(Clone, Default)]
    struct TestLogWriter {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    impl TestLogWriter {
        fn contents(&self) -> String {
            let bytes = self.buffer.lock().expect("test log writer lock poisoned");
            String::from_utf8_lossy(&bytes).into_owned()
        }
    }

    struct TestLogGuard {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    impl io::Write for TestLogGuard {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buffer
                .lock()
                .expect("test log writer lock poisoned")
                .extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for TestLogWriter {
        type Writer = TestLogGuard;

        fn make_writer(&'a self) -> Self::Writer {
            TestLogGuard {
                buffer: Arc::clone(&self.buffer),
            }
        }
    }

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

    #[tokio::test(flavor = "current_thread")]
    async fn seen_ok_reporter_logs_non_success_http_responses() {
        let control_plane = MockServer::start().await;
        let account_id = Uuid::new_v4();
        Mock::given(method("POST"))
            .and(path(format!(
                "/internal/v1/upstream-accounts/{account_id}/health/seen-ok"
            )))
            .respond_with(ResponseTemplate::new(503).set_body_string("control plane unavailable"))
            .mount(&control_plane)
            .await;

        let reporter = SeenOkReporter::new(
            control_plane.uri(),
            Arc::<str>::from("internal-token"),
            Duration::from_millis(250),
            Duration::from_secs(60),
        )
        .expect("reporter must build");
        let logs = TestLogWriter::default();
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .without_time()
            .with_target(false)
            .with_writer(logs.clone())
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);

        reporter.report_seen_ok(account_id).await;

        let logged = logs.contents();
        assert!(
            logged.contains("failed to report seen_ok signal to control plane"),
            "expected reporter failure log, got: {logged}"
        );
        assert!(
            logged.contains("503"),
            "expected reporter failure log to include status code, got: {logged}"
        );
        assert!(
            logged.contains("control plane unavailable"),
            "expected reporter failure log to include response body, got: {logged}"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn seen_ok_reporter_posts_success_live_result_payload() {
        let control_plane = MockServer::start().await;
        let account_id = Uuid::new_v4();
        Mock::given(method("POST"))
            .and(path(format!(
                "/internal/v1/upstream-accounts/{account_id}/health/live-result"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
            .mount(&control_plane)
            .await;

        let reporter = SeenOkReporter::new(
            control_plane.uri(),
            Arc::<str>::from("internal-token"),
            Duration::from_millis(250),
            Duration::from_secs(0),
        )
        .expect("reporter must build");

        reporter
            .report_live_result(
                account_id,
                LiveResultReportRequest {
                    status: LiveResultReportStatus::Ok,
                    source: LiveResultReportSource::Passive,
                    status_code: Some(200),
                    error_code: None,
                    error_message: None,
                    upstream_request_id: Some("req-live-ok".to_string()),
                    model: Some("gpt-5.1-codex-mini".to_string()),
                },
            )
            .await;

        let requests = control_plane.received_requests().await.unwrap();
        let live_request = requests
            .iter()
            .find(|request| request.url.path().ends_with("/health/live-result"))
            .expect("expected live-result request");
        let body: serde_json::Value =
            serde_json::from_slice(&live_request.body).expect("request body should be json");

        assert_eq!(body["status"], "ok");
        assert_eq!(body["source"], "passive");
        assert_eq!(body["status_code"], 200);
        assert_eq!(body["upstream_request_id"], "req-live-ok");
        assert_eq!(body["model"], "gpt-5.1-codex-mini");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn seen_ok_reporter_posts_failed_live_result_payload() {
        let control_plane = MockServer::start().await;
        let account_id = Uuid::new_v4();
        Mock::given(method("POST"))
            .and(path(format!(
                "/internal/v1/upstream-accounts/{account_id}/health/live-result"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
            .mount(&control_plane)
            .await;

        let reporter = SeenOkReporter::new(
            control_plane.uri(),
            Arc::<str>::from("internal-token"),
            Duration::from_millis(250),
            Duration::from_secs(0),
        )
        .expect("reporter must build");

        reporter
            .report_live_result(
                account_id,
                LiveResultReportRequest {
                    status: LiveResultReportStatus::Failed,
                    source: LiveResultReportSource::Passive,
                    status_code: Some(402),
                    error_code: Some("deactivated_workspace".to_string()),
                    error_message: Some("workspace is deactivated".to_string()),
                    upstream_request_id: Some("req-live-failed".to_string()),
                    model: Some("gpt-5.1-codex-mini".to_string()),
                },
            )
            .await;

        let requests = control_plane.received_requests().await.unwrap();
        let live_request = requests
            .iter()
            .find(|request| request.url.path().ends_with("/health/live-result"))
            .expect("expected live-result request");
        let body: serde_json::Value =
            serde_json::from_slice(&live_request.body).expect("request body should be json");

        assert_eq!(body["status"], "failed");
        assert_eq!(body["source"], "passive");
        assert_eq!(body["status_code"], 402);
        assert_eq!(body["error_code"], "deactivated_workspace");
        assert_eq!(body["error_message"], "workspace is deactivated");
        assert_eq!(body["upstream_request_id"], "req-live-failed");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn seen_ok_reporter_posts_observed_rate_limit_payload() {
        let control_plane = MockServer::start().await;
        let account_id = Uuid::new_v4();
        Mock::given(method("POST"))
            .and(path(format!(
                "/internal/v1/upstream-accounts/{account_id}/rate-limits/observed"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
            .mount(&control_plane)
            .await;

        let reporter = SeenOkReporter::new(
            control_plane.uri(),
            Arc::<str>::from("internal-token"),
            Duration::from_millis(250),
            Duration::from_secs(0),
        )
        .expect("reporter must build");

        reporter
            .report_observed_rate_limits(
                account_id,
                ObservedRateLimitReportRequest {
                    source: ObservedRateLimitReportSource::SseHeaders,
                    observed_at: Utc::now(),
                    rate_limits: vec![ObservedRateLimitSnapshot {
                        limit_id: Some("codex".to_string()),
                        limit_name: None,
                        primary: Some(ObservedRateLimitWindow {
                            used_percent: 91.5,
                            window_minutes: Some(300),
                            resets_at: Some(Utc::now()),
                        }),
                        secondary: None,
                        credits: None,
                        plan_type: None,
                    }],
                },
            )
            .await;

        let requests = control_plane.received_requests().await.unwrap();
        let observed_request = requests
            .iter()
            .find(|request| request.url.path().ends_with("/rate-limits/observed"))
            .expect("expected observed rate-limit request");
        let body: serde_json::Value =
            serde_json::from_slice(&observed_request.body).expect("request body should be json");

        assert_eq!(body["source"], "sse_headers");
        assert_eq!(body["rate_limits"][0]["limit_id"], "codex");
        assert_eq!(body["rate_limits"][0]["primary"]["used_percent"], 91.5);
        assert_eq!(body["rate_limits"][0]["primary"]["window_minutes"], 300);
    }
}
