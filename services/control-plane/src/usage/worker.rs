use std::future::Future;
use std::time::{Duration, Instant};

use anyhow::Result;
use async_trait::async_trait;
use codex_pool_core::events::RequestLogEvent;
use rand::random;
use serde::Serialize;
use uuid::Uuid;

use crate::usage::{
    aggregate_by_hour, request_log_row_from_event, HourlyAccountUsageRow,
    HourlyTenantAccountUsageRow, HourlyTenantApiKeyUsageRow, RequestLogRow,
    UsageAggregationEvent,
};

#[derive(Debug, Clone)]
pub struct StreamMessage {
    pub message_id: String,
    pub event: RequestLogEvent,
    pub tenant_id: Option<Uuid>,
    pub api_key_id: Option<Uuid>,
}

#[derive(Debug, Clone, Default)]
pub struct StreamReadResult {
    pub messages: Vec<StreamMessage>,
    pub malformed_acked_count: u64,
    pub malformed_missing_event_count: u64,
    pub malformed_invalid_json_count: u64,
    pub malformed_other_count: u64,
    pub malformed_raw_event_bytes_total: u64,
    pub dead_letter_relay_attempt_count: u64,
    pub dead_letter_relay_skipped_count: u64,
    pub dead_letter_relay_success_count: u64,
    pub dead_letter_relay_failed_count: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConsumerGroupBacklog {
    pub pending_count: u64,
    pub lag_count: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct UsageWorkerConfig {
    pub stream_read_count: usize,
    pub stream_block: Duration,
    pub reclaim_count: usize,
    pub reclaim_min_idle: Duration,
    pub flush_min_batch: usize,
    pub flush_interval: Duration,
    pub metrics_log_interval: Duration,
    pub error_backoff: Duration,
    pub error_backoff_factor: u32,
    pub error_backoff_max: Duration,
    pub error_backoff_jitter_pct: u32,
    pub error_backoff_jitter_seed: Option<u64>,
    pub max_consecutive_errors: u32,
}

impl Default for UsageWorkerConfig {
    fn default() -> Self {
        Self {
            stream_read_count: 100,
            stream_block: Duration::from_millis(1000),
            reclaim_count: 100,
            reclaim_min_idle: Duration::from_secs(30),
            flush_min_batch: 100,
            flush_interval: Duration::from_secs(5),
            metrics_log_interval: Duration::from_secs(10),
            error_backoff: Duration::from_millis(1000),
            error_backoff_factor: 2,
            error_backoff_max: Duration::from_millis(10000),
            error_backoff_jitter_pct: 0,
            error_backoff_jitter_seed: None,
            max_consecutive_errors: 0,
        }
    }
}

impl UsageWorkerConfig {
    pub fn compute_backoff(&self, consecutive_errors: u32) -> Duration {
        if let Some(seed) = self.error_backoff_jitter_seed {
            return compute_backoff_with_seed(
                self.error_backoff,
                self.error_backoff_factor,
                self.error_backoff_max,
                self.error_backoff_jitter_pct,
                seed,
                consecutive_errors,
            );
        }

        compute_backoff_with_jitter(
            self.error_backoff,
            self.error_backoff_factor,
            self.error_backoff_max,
            self.error_backoff_jitter_pct,
            consecutive_errors,
        )
    }
}

fn compute_backoff_with_seed(
    error_backoff: Duration,
    error_backoff_factor: u32,
    error_backoff_max: Duration,
    error_backoff_jitter_pct: u32,
    error_backoff_jitter_seed: u64,
    consecutive_errors: u32,
) -> Duration {
    compute_backoff_with_sample(
        error_backoff,
        error_backoff_factor,
        error_backoff_max,
        error_backoff_jitter_pct,
        consecutive_errors,
        seeded_jitter_sample(error_backoff_jitter_seed, consecutive_errors),
    )
}

fn seeded_jitter_sample(seed: u64, consecutive_errors: u32) -> u32 {
    // splitmix64 mixing gives stable pseudo-random samples across platforms.
    let mut mixed = seed ^ u64::from(consecutive_errors).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    mixed = mixed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    mixed ^= mixed >> 31;
    mixed as u32
}

fn compute_backoff_with_jitter(
    error_backoff: Duration,
    error_backoff_factor: u32,
    error_backoff_max: Duration,
    error_backoff_jitter_pct: u32,
    consecutive_errors: u32,
) -> Duration {
    compute_backoff_with_sample(
        error_backoff,
        error_backoff_factor,
        error_backoff_max,
        error_backoff_jitter_pct,
        consecutive_errors,
        random::<u32>(),
    )
}

fn compute_backoff_with_sample(
    error_backoff: Duration,
    error_backoff_factor: u32,
    error_backoff_max: Duration,
    error_backoff_jitter_pct: u32,
    consecutive_errors: u32,
    jitter_sample: u32,
) -> Duration {
    let base_backoff_ms = compute_base_backoff_ms(
        error_backoff,
        error_backoff_factor,
        error_backoff_max,
        consecutive_errors,
    );

    if base_backoff_ms == 0 {
        return Duration::ZERO;
    }

    let max_backoff_ms = duration_to_millis_u64(error_backoff_max);
    let (min_backoff_ms, max_backoff_ms) =
        compute_jitter_bounds_ms(base_backoff_ms, max_backoff_ms, error_backoff_jitter_pct);

    Duration::from_millis(sample_inclusive_range(
        min_backoff_ms,
        max_backoff_ms,
        jitter_sample,
    ))
}

fn compute_base_backoff_ms(
    error_backoff: Duration,
    error_backoff_factor: u32,
    error_backoff_max: Duration,
    consecutive_errors: u32,
) -> u64 {
    if consecutive_errors == 0 {
        return 0;
    }

    let max_backoff_ms = duration_to_millis_u64(error_backoff_max);
    if max_backoff_ms == 0 {
        return 0;
    }

    let mut backoff_ms = duration_to_millis_u64(error_backoff).min(max_backoff_ms);

    for _ in 1..consecutive_errors {
        backoff_ms = backoff_ms
            .saturating_mul(u64::from(error_backoff_factor))
            .min(max_backoff_ms);

        if backoff_ms == max_backoff_ms {
            break;
        }
    }

    backoff_ms
}

fn compute_jitter_bounds_ms(
    base_backoff_ms: u64,
    max_backoff_ms: u64,
    jitter_pct: u32,
) -> (u64, u64) {
    let jitter_pct = jitter_pct.min(100);
    if jitter_pct == 0 {
        let capped = base_backoff_ms.min(max_backoff_ms);
        return (capped, capped);
    }

    let jitter_pct = u64::from(jitter_pct);
    let min_scale = 100_u64.saturating_sub(jitter_pct);
    let max_scale = 100_u64.saturating_add(jitter_pct);

    let min_backoff_ms = base_backoff_ms.saturating_mul(min_scale) / 100;
    let max_backoff_ms = base_backoff_ms
        .saturating_mul(max_scale)
        .saturating_div(100)
        .min(max_backoff_ms);

    (min_backoff_ms, max_backoff_ms)
}

fn sample_inclusive_range(min_value: u64, max_value: u64, sample: u32) -> u64 {
    if min_value >= max_value {
        return min_value;
    }

    let range_width = u128::from(max_value - min_value) + 1;
    let scaled = u128::from(sample)
        .saturating_mul(range_width)
        .saturating_div(u128::from(u32::MAX) + 1);

    min_value.saturating_add(scaled as u64)
}

fn duration_to_millis_u64(value: Duration) -> u64 {
    u64::try_from(value.as_millis()).unwrap_or(u64::MAX)
}

fn is_timeout_like_error(error: &anyhow::Error) -> bool {
    let mut current: Option<&(dyn std::error::Error + 'static)> = error.source();
    while let Some(err) = current {
        let msg = err.to_string().to_ascii_lowercase();
        if msg.contains("timed out") || msg.contains("timeout") {
            return true;
        }
        current = err.source();
    }

    let top = error.to_string().to_ascii_lowercase();
    top.contains("timed out") || top.contains("timeout")
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct WorkerRunStats {
    pub processed_count: u64,
    pub reclaimed_count: u64,
    pub fresh_read_count: u64,
    pub reclaimed_message_count: u64,
    pub fresh_message_count: u64,
    pub malformed_acked_count: u64,
    pub malformed_missing_event_count: u64,
    pub malformed_invalid_json_count: u64,
    pub malformed_other_count: u64,
    pub malformed_raw_event_bytes_total: u64,
    pub dead_letter_relay_attempt_count: u64,
    pub dead_letter_relay_skipped_count: u64,
    pub dead_letter_relay_success_count: u64,
    pub dead_letter_relay_failed_count: u64,
    pub pending_count: u64,
    pub lag_count: Option<u64>,
    pub flush_count: u64,
    pub ack_count: u64,
    pub error_count: u64,
    pub consecutive_errors: u32,
    pub last_backoff_ms: u64,
    pub run_duration_ms: u64,
}

struct ReadMessagesResult {
    messages: Vec<StreamMessage>,
    reclaimed_count: u64,
    fresh_read_count: u64,
    reclaimed_message_count: u64,
    fresh_message_count: u64,
    malformed_acked_count: u64,
    malformed_missing_event_count: u64,
    malformed_invalid_json_count: u64,
    malformed_other_count: u64,
    malformed_raw_event_bytes_total: u64,
    dead_letter_relay_attempt_count: u64,
    dead_letter_relay_skipped_count: u64,
    dead_letter_relay_success_count: u64,
    dead_letter_relay_failed_count: u64,
}

#[async_trait]
pub trait RequestLogStreamReader: Send + Sync {
    async fn ensure_consumer_group(&self) -> Result<()>;

    async fn reclaim_pending(&self, count: usize, min_idle: Duration) -> Result<StreamReadResult>;

    async fn read_group(&self, count: usize, block: Duration) -> Result<StreamReadResult>;

    async fn ack(&self, message_ids: &[String]) -> Result<()>;

    async fn consumer_group_backlog(&self) -> Result<ConsumerGroupBacklog>;
}

#[async_trait]
pub trait UsageAggregationRepository: Send + Sync {
    async fn upsert_hourly(
        &self,
        account_rows: Vec<HourlyAccountUsageRow>,
        tenant_api_key_rows: Vec<HourlyTenantApiKeyUsageRow>,
        tenant_account_rows: Vec<HourlyTenantAccountUsageRow>,
    ) -> Result<()>;

    async fn upsert_request_logs(&self, _rows: Vec<RequestLogRow>) -> Result<()> {
        Ok(())
    }
}

pub struct UsageAggregationWorker<R, Repo>
where
    R: RequestLogStreamReader,
    Repo: UsageAggregationRepository,
{
    stream_reader: R,
    repo: Repo,
    config: UsageWorkerConfig,
}

impl<R, Repo> UsageAggregationWorker<R, Repo>
where
    R: RequestLogStreamReader,
    Repo: UsageAggregationRepository,
{
    pub fn new(stream_reader: R, repo: Repo) -> Self {
        Self::with_config(stream_reader, repo, UsageWorkerConfig::default())
    }

    pub fn with_config(stream_reader: R, repo: Repo, config: UsageWorkerConfig) -> Self {
        Self {
            stream_reader,
            repo,
            config,
        }
    }

    pub async fn run_once(&self) -> Result<()> {
        let _ = self.run_once_with_stats().await?;
        Ok(())
    }

    pub async fn run_once_with_stats(&self) -> Result<WorkerRunStats> {
        let run_started_at = Instant::now();
        let read_result = self.read_messages_with_stats().await?;
        let mut stats = WorkerRunStats {
            processed_count: read_result.messages.len() as u64,
            reclaimed_count: read_result.reclaimed_count,
            fresh_read_count: read_result.fresh_read_count,
            reclaimed_message_count: read_result.reclaimed_message_count,
            fresh_message_count: read_result.fresh_message_count,
            malformed_acked_count: read_result.malformed_acked_count,
            malformed_missing_event_count: read_result.malformed_missing_event_count,
            malformed_invalid_json_count: read_result.malformed_invalid_json_count,
            malformed_other_count: read_result.malformed_other_count,
            malformed_raw_event_bytes_total: read_result.malformed_raw_event_bytes_total,
            dead_letter_relay_attempt_count: read_result.dead_letter_relay_attempt_count,
            dead_letter_relay_skipped_count: read_result.dead_letter_relay_skipped_count,
            dead_letter_relay_success_count: read_result.dead_letter_relay_success_count,
            dead_letter_relay_failed_count: read_result.dead_letter_relay_failed_count,
            ..WorkerRunStats::default()
        };

        self.refresh_consumer_group_backlog_snapshot(&mut stats)
            .await;

        if read_result.messages.is_empty() {
            stats.run_duration_ms = duration_to_millis_u64(run_started_at.elapsed());
            return Ok(stats);
        }

        let acked_count = self.flush_messages(read_result.messages).await?;
        stats.flush_count = 1;
        stats.ack_count = acked_count as u64;
        self.refresh_consumer_group_backlog_snapshot(&mut stats)
            .await;
        stats.run_duration_ms = duration_to_millis_u64(run_started_at.elapsed());

        Ok(stats)
    }

    pub async fn run_forever(&self) -> Result<()> {
        self.run_until_shutdown(std::future::pending::<()>()).await
    }

    pub async fn run_until_shutdown<S>(&self, shutdown: S) -> Result<()>
    where
        S: Future<Output = ()>,
    {
        self.stream_reader.ensure_consumer_group().await?;

        let mut buffered = Vec::new();
        let mut last_flush = Instant::now();
        let mut last_metrics_log = Instant::now();
        let mut stats = WorkerRunStats::default();
        let mut consecutive_errors = 0_u32;
        tokio::pin!(shutdown);

        loop {
            tokio::select! {
                _ = &mut shutdown => {
                    if let Err(error) = self.flush_buffered_messages(&mut buffered, &mut stats).await {
                        stats.error_count += 1;
                        self.refresh_consumer_group_backlog_snapshot(&mut stats).await;
                        self.log_runtime_stats(&stats, buffered.len(), "shutdown-error");
                        return Err(error);
                    }

                    self.refresh_consumer_group_backlog_snapshot(&mut stats).await;
                    self.log_runtime_stats(&stats, buffered.len(), "shutdown");
                    return Ok(());
                }
                read_messages = self.read_messages_with_stats() => {
                    let read_result = match read_messages {
                        Ok(read_result) => read_result,
                        Err(error) => {
                            if is_timeout_like_error(&error) {
                                // Redis XREADGROUP with BLOCK may timeout when no new entries.
                                // Treat as idle poll instead of runtime error/backoff.
                                consecutive_errors = 0;
                                stats.consecutive_errors = 0;
                                stats.last_backoff_ms = 0;
                                self.refresh_consumer_group_backlog_snapshot(&mut stats).await;
                                self.maybe_log_runtime_stats(&stats, buffered.len(), &mut last_metrics_log);
                                continue;
                            }

                            self
                                .handle_runtime_error(
                                    error,
                                    &mut stats,
                                    &mut consecutive_errors,
                                    buffered.len(),
                                    "read-error",
                                )
                                .await?;
                            continue;
                        }
                    };

                    let ReadMessagesResult {
                        mut messages,
                        reclaimed_count,
                        fresh_read_count,
                        reclaimed_message_count,
                        fresh_message_count,
                        malformed_acked_count,
                        malformed_missing_event_count,
                        malformed_invalid_json_count,
                        malformed_other_count,
                        malformed_raw_event_bytes_total,
                        dead_letter_relay_attempt_count,
                        dead_letter_relay_skipped_count,
                        dead_letter_relay_success_count,
                        dead_letter_relay_failed_count,
                    } = read_result;

                    stats.reclaimed_count += reclaimed_count;
                    stats.fresh_read_count += fresh_read_count;
                    stats.reclaimed_message_count += reclaimed_message_count;
                    stats.fresh_message_count += fresh_message_count;
                    stats.malformed_acked_count += malformed_acked_count;
                    stats.malformed_missing_event_count += malformed_missing_event_count;
                    stats.malformed_invalid_json_count += malformed_invalid_json_count;
                    stats.malformed_other_count += malformed_other_count;
                    stats.malformed_raw_event_bytes_total += malformed_raw_event_bytes_total;
                    stats.dead_letter_relay_attempt_count += dead_letter_relay_attempt_count;
                    stats.dead_letter_relay_skipped_count += dead_letter_relay_skipped_count;
                    stats.dead_letter_relay_success_count += dead_letter_relay_success_count;
                    stats.dead_letter_relay_failed_count += dead_letter_relay_failed_count;
                    stats.processed_count += messages.len() as u64;
                    buffered.append(&mut messages);

                    let enough_for_flush = !buffered.is_empty()
                        && (self.config.flush_min_batch == 0
                            || buffered.len() >= self.config.flush_min_batch);
                    let flush_interval_elapsed =
                        !buffered.is_empty() && last_flush.elapsed() >= self.config.flush_interval;

                    if enough_for_flush || flush_interval_elapsed {
                        if let Err(error) =
                            self.flush_buffered_messages(&mut buffered, &mut stats).await
                        {
                            self
                                .handle_runtime_error(
                                    error,
                                    &mut stats,
                                    &mut consecutive_errors,
                                    buffered.len(),
                                    "flush-or-ack-error",
                                )
                                .await?;
                            continue;
                        }
                        last_flush = Instant::now();
                    }

                    consecutive_errors = 0;
                    stats.consecutive_errors = 0;
                    stats.last_backoff_ms = 0;
                    self.refresh_consumer_group_backlog_snapshot(&mut stats).await;

                    self.maybe_log_runtime_stats(&stats, buffered.len(), &mut last_metrics_log);
                }
            }
        }
    }

    async fn read_messages_with_stats(&self) -> Result<ReadMessagesResult> {
        let reclaimed_result = self
            .stream_reader
            .reclaim_pending(self.config.reclaim_count, self.config.reclaim_min_idle)
            .await?;
        let reclaimed_count = reclaimed_result.messages.len() as u64;
        let reclaimed_message_count = reclaimed_count;

        let mut fresh_result = self
            .stream_reader
            .read_group(self.config.stream_read_count, self.config.stream_block)
            .await?;
        let fresh_read_count = fresh_result.messages.len() as u64;
        let fresh_message_count = fresh_read_count;

        let mut messages = reclaimed_result.messages;
        messages.append(&mut fresh_result.messages);

        Ok(ReadMessagesResult {
            messages,
            reclaimed_count,
            fresh_read_count,
            reclaimed_message_count,
            fresh_message_count,
            malformed_acked_count: reclaimed_result
                .malformed_acked_count
                .saturating_add(fresh_result.malformed_acked_count),
            malformed_missing_event_count: reclaimed_result
                .malformed_missing_event_count
                .saturating_add(fresh_result.malformed_missing_event_count),
            malformed_invalid_json_count: reclaimed_result
                .malformed_invalid_json_count
                .saturating_add(fresh_result.malformed_invalid_json_count),
            malformed_other_count: reclaimed_result
                .malformed_other_count
                .saturating_add(fresh_result.malformed_other_count),
            malformed_raw_event_bytes_total: reclaimed_result
                .malformed_raw_event_bytes_total
                .saturating_add(fresh_result.malformed_raw_event_bytes_total),
            dead_letter_relay_attempt_count: reclaimed_result
                .dead_letter_relay_attempt_count
                .saturating_add(fresh_result.dead_letter_relay_attempt_count),
            dead_letter_relay_skipped_count: reclaimed_result
                .dead_letter_relay_skipped_count
                .saturating_add(fresh_result.dead_letter_relay_skipped_count),
            dead_letter_relay_success_count: reclaimed_result
                .dead_letter_relay_success_count
                .saturating_add(fresh_result.dead_letter_relay_success_count),
            dead_letter_relay_failed_count: reclaimed_result
                .dead_letter_relay_failed_count
                .saturating_add(fresh_result.dead_letter_relay_failed_count),
        })
    }

    async fn flush_messages(&self, messages: Vec<StreamMessage>) -> Result<usize> {
        if messages.is_empty() {
            return Ok(0);
        }

        let mut message_ids = Vec::with_capacity(messages.len());
        let mut request_log_rows = Vec::with_capacity(messages.len());
        let mut aggregation_events = Vec::with_capacity(messages.len());
        for message in messages {
            let resolved_tenant_id = message.tenant_id.or(message.event.tenant_id);
            let resolved_api_key_id = message.api_key_id.or(message.event.api_key_id);
            if message.event.billing_phase.as_deref() != Some("streaming_open") {
                aggregation_events.push(UsageAggregationEvent::from_request_log_event(
                    &message.event,
                    resolved_tenant_id,
                    resolved_api_key_id,
                ));
            }
            request_log_rows.push(request_log_row_from_event(
                &message.event,
                resolved_tenant_id,
                resolved_api_key_id,
            ));
            message_ids.push(message.message_id);
        }
        let hourly_rows = aggregate_by_hour(aggregation_events);

        self.repo.upsert_request_logs(request_log_rows).await?;
        self.repo
            .upsert_hourly(
                hourly_rows.account_rows,
                hourly_rows.tenant_api_key_rows,
                hourly_rows.tenant_account_rows,
            )
            .await?;
        self.stream_reader.ack(&message_ids).await?;

        Ok(message_ids.len())
    }

    async fn flush_buffered_messages(
        &self,
        buffered: &mut Vec<StreamMessage>,
        stats: &mut WorkerRunStats,
    ) -> Result<()> {
        if buffered.is_empty() {
            return Ok(());
        }

        let batch = std::mem::take(buffered);
        let acked_count = self.flush_messages(batch).await?;
        stats.flush_count += 1;
        stats.ack_count += acked_count as u64;

        Ok(())
    }

    async fn handle_runtime_error(
        &self,
        error: anyhow::Error,
        stats: &mut WorkerRunStats,
        consecutive_errors: &mut u32,
        buffered_count: usize,
        reason: &'static str,
    ) -> Result<()> {
        stats.error_count += 1;
        *consecutive_errors += 1;
        let backoff = self.config.compute_backoff(*consecutive_errors);
        stats.consecutive_errors = *consecutive_errors;
        stats.last_backoff_ms = duration_to_millis_u64(backoff);

        tracing::warn!(
            reason,
            error = %error,
            consecutive_errors = *consecutive_errors,
            max_consecutive_errors = self.config.max_consecutive_errors,
            error_backoff_ms = backoff.as_millis(),
            error_backoff_factor = self.config.error_backoff_factor,
            error_backoff_max_ms = self.config.error_backoff_max.as_millis(),
            error_backoff_jitter_pct = self.config.error_backoff_jitter_pct.min(100),
            error_backoff_jitter_seed = ?self.config.error_backoff_jitter_seed,
            "usage worker step failed, backing off before retry"
        );
        self.log_runtime_stats(stats, buffered_count, reason);

        if self.config.max_consecutive_errors > 0
            && *consecutive_errors >= self.config.max_consecutive_errors
        {
            tracing::error!(
                reason,
                error = %error,
                consecutive_errors = *consecutive_errors,
                max_consecutive_errors = self.config.max_consecutive_errors,
                "usage worker reached consecutive error limit and will exit"
            );
            return Err(anyhow::anyhow!(
                "usage worker reached max_consecutive_errors={} at {}: {}",
                self.config.max_consecutive_errors,
                reason,
                error
            ));
        }

        tokio::time::sleep(backoff).await;
        Ok(())
    }

    async fn refresh_consumer_group_backlog_snapshot(&self, stats: &mut WorkerRunStats) {
        match self.stream_reader.consumer_group_backlog().await {
            Ok(backlog) => {
                stats.pending_count = backlog.pending_count;
                stats.lag_count = backlog.lag_count;
            }
            Err(error) => {
                tracing::warn!(
                    error = %error,
                    "failed to inspect redis consumer group backlog for usage worker stats"
                );
            }
        }
    }

    fn maybe_log_runtime_stats(
        &self,
        stats: &WorkerRunStats,
        buffered_count: usize,
        last_metrics_log: &mut Instant,
    ) {
        if last_metrics_log.elapsed() < self.config.metrics_log_interval {
            return;
        }

        self.log_runtime_stats(stats, buffered_count, "interval");
        *last_metrics_log = Instant::now();
    }

    fn log_runtime_stats(&self, stats: &WorkerRunStats, buffered_count: usize, reason: &str) {
        tracing::info!(
            reason,
            processed_count = stats.processed_count,
            reclaimed_count = stats.reclaimed_count,
            fresh_read_count = stats.fresh_read_count,
            reclaimed_message_count = stats.reclaimed_message_count,
            fresh_message_count = stats.fresh_message_count,
            malformed_acked_count = stats.malformed_acked_count,
            malformed_missing_event_count = stats.malformed_missing_event_count,
            malformed_invalid_json_count = stats.malformed_invalid_json_count,
            malformed_other_count = stats.malformed_other_count,
            malformed_raw_event_bytes_total = stats.malformed_raw_event_bytes_total,
            dead_letter_relay_attempt_count = stats.dead_letter_relay_attempt_count,
            dead_letter_relay_skipped_count = stats.dead_letter_relay_skipped_count,
            dead_letter_relay_success_count = stats.dead_letter_relay_success_count,
            dead_letter_relay_failed_count = stats.dead_letter_relay_failed_count,
            pending_count = stats.pending_count,
            lag_count = ?stats.lag_count,
            flush_count = stats.flush_count,
            ack_count = stats.ack_count,
            buffered_count,
            error_count = stats.error_count,
            consecutive_errors = stats.consecutive_errors,
            last_backoff_ms = stats.last_backoff_ms,
            "usage worker runtime stats"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::compute_backoff_with_sample;
    use std::time::Duration;

    #[test]
    fn jitter_zero_keeps_base_backoff() {
        let backoff = compute_backoff_with_sample(
            Duration::from_millis(1000),
            2,
            Duration::from_millis(10000),
            0,
            3,
            u32::MAX,
        );

        assert_eq!(backoff, Duration::from_millis(4000));
    }

    #[test]
    fn jitter_sample_spans_expected_bounds() {
        let min_backoff = compute_backoff_with_sample(
            Duration::from_millis(1000),
            2,
            Duration::from_millis(10000),
            20,
            3,
            0,
        );
        let max_backoff = compute_backoff_with_sample(
            Duration::from_millis(1000),
            2,
            Duration::from_millis(10000),
            20,
            3,
            u32::MAX,
        );

        assert_eq!(min_backoff, Duration::from_millis(3200));
        assert_eq!(max_backoff, Duration::from_millis(4800));
    }

    #[test]
    fn jitter_is_clamped_and_respects_max_cap() {
        let min_backoff = compute_backoff_with_sample(
            Duration::from_millis(1000),
            2,
            Duration::from_millis(1500),
            150,
            2,
            0,
        );
        let max_backoff = compute_backoff_with_sample(
            Duration::from_millis(1000),
            2,
            Duration::from_millis(1500),
            150,
            2,
            u32::MAX,
        );

        assert_eq!(min_backoff, Duration::ZERO);
        assert_eq!(max_backoff, Duration::from_millis(1500));
    }
}
