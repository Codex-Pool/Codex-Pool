use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;

use chrono::{DateTime, Timelike, Utc};
use codex_pool_core::events::RequestLogEvent;
use uuid::Uuid;

pub mod clickhouse_repo;
pub mod postgres_repo;
pub mod redis_reader;
pub mod worker;

#[async_trait]
pub trait UsageIngestRepository: Send + Sync {
    async fn ingest_request_log(&self, event: RequestLogEvent) -> Result<()>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageAggregationEvent {
    pub account_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub api_key_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl UsageAggregationEvent {
    pub fn from_request_log_event(
        event: &RequestLogEvent,
        tenant_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Self {
        Self {
            account_id: event.account_id,
            tenant_id,
            api_key_id,
            created_at: event.created_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestLogRow {
    pub id: Uuid,
    pub account_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub api_key_id: Option<Uuid>,
    pub request_id: Option<String>,
    pub path: String,
    pub method: String,
    pub model: Option<String>,
    pub service_tier: Option<String>,
    pub input_tokens: Option<i64>,
    pub cached_input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub reasoning_tokens: Option<i64>,
    pub first_token_latency_ms: Option<u64>,
    pub status_code: u16,
    pub latency_ms: u64,
    pub is_stream: bool,
    pub error_code: Option<String>,
    pub billing_phase: Option<String>,
    pub authorization_id: Option<Uuid>,
    pub capture_status: Option<String>,
    pub estimated_cost_microusd: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub event_version: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillingReconcileFact {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub api_key_id: Option<Uuid>,
    pub request_id: String,
    pub model: Option<String>,
    pub service_tier: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub status_code: u16,
    pub billing_phase: Option<String>,
    pub capture_status: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestLogQuery {
    pub start_ts: i64,
    pub end_ts: i64,
    pub limit: u32,
    pub tenant_id: Option<Uuid>,
    pub api_key_id: Option<Uuid>,
    pub status_code: Option<u16>,
    pub request_id: Option<String>,
    pub keyword: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HourlyAccountUsageRow {
    pub account_id: Uuid,
    pub hour_start: DateTime<Utc>,
    pub request_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HourlyTenantApiKeyUsageRow {
    pub tenant_id: Uuid,
    pub api_key_id: Uuid,
    pub hour_start: DateTime<Utc>,
    pub request_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HourlyTenantAccountUsageRow {
    pub tenant_id: Uuid,
    pub account_id: Uuid,
    pub hour_start: DateTime<Utc>,
    pub request_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HourlyUsageRows {
    pub account_rows: Vec<HourlyAccountUsageRow>,
    pub tenant_api_key_rows: Vec<HourlyTenantApiKeyUsageRow>,
    pub tenant_account_rows: Vec<HourlyTenantAccountUsageRow>,
}

pub fn aggregate_by_hour(events: Vec<UsageAggregationEvent>) -> HourlyUsageRows {
    let mut account_buckets: HashMap<(Uuid, DateTime<Utc>), u64> = HashMap::new();
    let mut tenant_buckets: HashMap<(Uuid, Uuid, DateTime<Utc>), u64> = HashMap::new();
    let mut tenant_account_buckets: HashMap<(Uuid, Uuid, DateTime<Utc>), u64> = HashMap::new();

    for event in events {
        let hour_start = truncate_to_hour(event.created_at);
        *account_buckets
            .entry((event.account_id, hour_start))
            .or_insert(0) += 1;
        if let Some(tenant_id) = event.tenant_id {
            *tenant_account_buckets
                .entry((tenant_id, event.account_id, hour_start))
                .or_insert(0) += 1;
        }

        if let (Some(tenant_id), Some(api_key_id)) = (event.tenant_id, event.api_key_id) {
            *tenant_buckets
                .entry((tenant_id, api_key_id, hour_start))
                .or_insert(0) += 1;
        }
    }

    let mut account_rows = account_buckets
        .into_iter()
        .map(
            |((account_id, hour_start), request_count)| HourlyAccountUsageRow {
                account_id,
                hour_start,
                request_count,
            },
        )
        .collect::<Vec<_>>();
    account_rows.sort_by_key(|row| (row.hour_start, row.account_id));

    let mut tenant_api_key_rows = tenant_buckets
        .into_iter()
        .map(
            |((tenant_id, api_key_id, hour_start), request_count)| HourlyTenantApiKeyUsageRow {
                tenant_id,
                api_key_id,
                hour_start,
                request_count,
            },
        )
        .collect::<Vec<_>>();
    tenant_api_key_rows.sort_by_key(|row| (row.hour_start, row.tenant_id, row.api_key_id));

    let mut tenant_account_rows = tenant_account_buckets
        .into_iter()
        .map(
            |((tenant_id, account_id, hour_start), request_count)| HourlyTenantAccountUsageRow {
                tenant_id,
                account_id,
                hour_start,
                request_count,
            },
        )
        .collect::<Vec<_>>();
    tenant_account_rows.sort_by_key(|row| (row.hour_start, row.tenant_id, row.account_id));

    HourlyUsageRows {
        account_rows,
        tenant_api_key_rows,
        tenant_account_rows,
    }
}

pub fn request_log_row_from_event(
    event: &RequestLogEvent,
    tenant_id: Option<Uuid>,
    api_key_id: Option<Uuid>,
) -> RequestLogRow {
    RequestLogRow {
        id: event.id,
        account_id: event.account_id,
        tenant_id,
        api_key_id,
        request_id: event.request_id.clone(),
        path: event.path.clone(),
        method: event.method.clone(),
        model: event.model.clone(),
        service_tier: event.service_tier.clone(),
        input_tokens: event.input_tokens,
        cached_input_tokens: event.cached_input_tokens,
        output_tokens: event.output_tokens,
        reasoning_tokens: event.reasoning_tokens,
        first_token_latency_ms: event.first_token_latency_ms,
        status_code: event.status_code,
        latency_ms: event.latency_ms,
        is_stream: event.is_stream,
        error_code: event.error_code.clone(),
        billing_phase: event.billing_phase.clone(),
        authorization_id: event.authorization_id,
        capture_status: event.capture_status.clone(),
        estimated_cost_microusd: None,
        created_at: event.created_at,
        event_version: event.event_version,
    }
}

pub fn usage_rows_from_request_log_event(
    event: &RequestLogEvent,
    tenant_id: Option<Uuid>,
    api_key_id: Option<Uuid>,
) -> HourlyUsageRows {
    aggregate_by_hour(vec![UsageAggregationEvent::from_request_log_event(
        event, tenant_id, api_key_id,
    )])
}

fn truncate_to_hour(created_at: DateTime<Utc>) -> DateTime<Utc> {
    created_at
        .with_minute(0)
        .and_then(|v| v.with_second(0))
        .and_then(|v| v.with_nanosecond(0))
        .unwrap_or(created_at)
}
