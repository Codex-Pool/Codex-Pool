use anyhow::{Context, Result};
use async_trait::async_trait;
use clickhouse::{Client, Row};
use codex_pool_core::api::{
    AccountUsageLeaderboardItem, ApiKeyUsageLeaderboardItem, HourlyAccountUsagePoint,
    HourlyTenantApiKeyUsagePoint, HourlyTenantUsageTotalPoint, HourlyUsageTotalPoint,
    TenantUsageLeaderboardItem, UsageDashboardMetrics, UsageDashboardModelDistributionItem,
    UsageDashboardTokenBreakdown, UsageDashboardTokenTrendPoint, UsageSummaryQueryResponse,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::usage::worker::UsageAggregationRepository;
use crate::usage::{
    BillingReconcileFact, HourlyAccountUsageRow, HourlyTenantAccountUsageRow,
    HourlyTenantApiKeyUsageRow, RequestLogQuery, RequestLogRow,
};

#[async_trait]
pub trait UsageQueryRepository: Send + Sync {
    async fn query_hourly_accounts(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        account_id: Option<Uuid>,
    ) -> Result<Vec<HourlyAccountUsagePoint>>;

    async fn query_hourly_tenant_api_keys(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        tenant_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<Vec<HourlyTenantApiKeyUsagePoint>>;

    async fn query_hourly_account_totals(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        account_id: Option<Uuid>,
    ) -> Result<Vec<HourlyUsageTotalPoint>>;

    async fn query_hourly_tenant_api_key_totals(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        tenant_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<Vec<HourlyUsageTotalPoint>>;

    async fn query_hourly_tenant_totals(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        tenant_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<Vec<HourlyTenantUsageTotalPoint>>;

    async fn query_summary(
        &self,
        start_ts: i64,
        end_ts: i64,
        tenant_id: Option<Uuid>,
        account_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<UsageSummaryQueryResponse>;

    async fn query_tenant_leaderboard(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        tenant_id: Option<Uuid>,
    ) -> Result<Vec<TenantUsageLeaderboardItem>>;

    async fn query_account_leaderboard(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        account_id: Option<Uuid>,
    ) -> Result<Vec<AccountUsageLeaderboardItem>>;

    async fn query_tenant_scoped_account_leaderboard(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        tenant_id: Uuid,
        account_id: Option<Uuid>,
    ) -> Result<Vec<AccountUsageLeaderboardItem>> {
        let _ = tenant_id;
        self.query_account_leaderboard(start_ts, end_ts, limit, account_id)
            .await
    }

    async fn query_api_key_leaderboard(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        tenant_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<Vec<ApiKeyUsageLeaderboardItem>>;

    async fn query_request_logs(&self, _query: RequestLogQuery) -> Result<Vec<RequestLogRow>> {
        Err(anyhow::anyhow!("request log query is not implemented"))
    }
}

#[derive(Clone)]
pub struct ClickHouseUsageRepo {
    ch_client: Client,
    account_table: String,
    tenant_api_key_table: String,
    tenant_account_table: String,
    request_log_table: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Row)]
struct ClickHouseHourlyAccountUsageRow {
    account_id: String,
    hour_start: i64,
    request_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Row)]
struct ClickHouseHourlyTenantApiKeyUsageRow {
    tenant_id: String,
    api_key_id: String,
    hour_start: i64,
    request_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Row)]
struct ClickHouseHourlyTenantAccountUsageRow {
    tenant_id: String,
    account_id: String,
    hour_start: i64,
    request_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Row)]
struct ClickHouseHourlyUsageTotalRow {
    hour_start: i64,
    request_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Row)]
struct ClickHouseHourlyTenantUsageTotalRow {
    tenant_id: String,
    hour_start: i64,
    request_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Row)]
struct ClickHouseAccountUsageSummaryRow {
    account_total_requests: u64,
    unique_account_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Row)]
struct ClickHouseTenantApiKeyUsageSummaryRow {
    tenant_api_key_total_requests: u64,
    unique_tenant_api_key_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Row)]
struct ClickHouseDashboardSummaryRow {
    total_requests: u64,
    input_tokens: u64,
    cached_input_tokens: u64,
    output_tokens: u64,
    reasoning_tokens: u64,
    avg_first_token_latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Row)]
struct ClickHouseDashboardTokenTrendRow {
    hour_start: i64,
    request_count: u64,
    input_tokens: u64,
    cached_input_tokens: u64,
    output_tokens: u64,
    reasoning_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Row)]
struct ClickHouseDashboardModelDistributionRow {
    model: String,
    request_count: u64,
    total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Row)]
struct ClickHouseTenantUsageLeaderboardRow {
    tenant_id: String,
    total_requests: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Row)]
struct ClickHouseAccountUsageLeaderboardRow {
    account_id: String,
    total_requests: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Row)]
struct ClickHouseApiKeyUsageLeaderboardRow {
    tenant_id: String,
    api_key_id: String,
    total_requests: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Row)]
struct ClickHouseTenantScopedAccountUsageLeaderboardRow {
    account_id: String,
    total_requests: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Row)]
struct ClickHouseRequestLogInsertRow {
    id: String,
    account_id: String,
    tenant_id: Option<String>,
    api_key_id: Option<String>,
    request_id: Option<String>,
    path: String,
    method: String,
    model: Option<String>,
    service_tier: Option<String>,
    input_tokens: Option<i64>,
    cached_input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    reasoning_tokens: Option<i64>,
    first_token_latency_ms: Option<u64>,
    status_code: u16,
    latency_ms: u64,
    is_stream: u8,
    error_code: Option<String>,
    billing_phase: Option<String>,
    authorization_id: Option<String>,
    capture_status: Option<String>,
    created_at: i64,
    event_version: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, Row)]
struct ClickHouseRequestLogQueryRow {
    id: String,
    account_id: String,
    tenant_id: Option<String>,
    api_key_id: Option<String>,
    request_id: Option<String>,
    path: String,
    method: String,
    model: Option<String>,
    service_tier: Option<String>,
    input_tokens: Option<i64>,
    cached_input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    reasoning_tokens: Option<i64>,
    first_token_latency_ms: Option<u64>,
    status_code: u16,
    latency_ms: u64,
    is_stream: u8,
    error_code: Option<String>,
    billing_phase: Option<String>,
    authorization_id: Option<String>,
    capture_status: Option<String>,
    created_at: i64,
    event_version: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, Row)]
struct ClickHouseBillingReconcileFactRow {
    id: String,
    tenant_id: String,
    api_key_id: Option<String>,
    request_id: String,
    model: Option<String>,
    service_tier: Option<String>,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    status_code: u16,
    billing_phase: Option<String>,
    capture_status: Option<String>,
    created_at: i64,
}

impl From<HourlyAccountUsageRow> for ClickHouseHourlyAccountUsageRow {
    fn from(row: HourlyAccountUsageRow) -> Self {
        Self {
            account_id: row.account_id.to_string(),
            hour_start: row.hour_start.timestamp(),
            request_count: row.request_count,
        }
    }
}

impl From<HourlyTenantApiKeyUsageRow> for ClickHouseHourlyTenantApiKeyUsageRow {
    fn from(row: HourlyTenantApiKeyUsageRow) -> Self {
        Self {
            tenant_id: row.tenant_id.to_string(),
            api_key_id: row.api_key_id.to_string(),
            hour_start: row.hour_start.timestamp(),
            request_count: row.request_count,
        }
    }
}

impl From<HourlyTenantAccountUsageRow> for ClickHouseHourlyTenantAccountUsageRow {
    fn from(row: HourlyTenantAccountUsageRow) -> Self {
        Self {
            tenant_id: row.tenant_id.to_string(),
            account_id: row.account_id.to_string(),
            hour_start: row.hour_start.timestamp(),
            request_count: row.request_count,
        }
    }
}

impl TryFrom<ClickHouseHourlyAccountUsageRow> for HourlyAccountUsagePoint {
    type Error = anyhow::Error;

    fn try_from(row: ClickHouseHourlyAccountUsageRow) -> Result<Self> {
        let account_id = Uuid::parse_str(&row.account_id)
            .with_context(|| format!("invalid account_id in clickhouse row: {}", row.account_id))?;

        Ok(Self {
            account_id,
            hour_start: row.hour_start,
            request_count: row.request_count,
        })
    }
}

impl TryFrom<ClickHouseHourlyTenantApiKeyUsageRow> for HourlyTenantApiKeyUsagePoint {
    type Error = anyhow::Error;

    fn try_from(row: ClickHouseHourlyTenantApiKeyUsageRow) -> Result<Self> {
        let tenant_id = Uuid::parse_str(&row.tenant_id)
            .with_context(|| format!("invalid tenant_id in clickhouse row: {}", row.tenant_id))?;
        let api_key_id = Uuid::parse_str(&row.api_key_id)
            .with_context(|| format!("invalid api_key_id in clickhouse row: {}", row.api_key_id))?;

        Ok(Self {
            tenant_id,
            api_key_id,
            hour_start: row.hour_start,
            request_count: row.request_count,
        })
    }
}

impl From<RequestLogRow> for ClickHouseRequestLogInsertRow {
    fn from(row: RequestLogRow) -> Self {
        Self {
            id: row.id.to_string(),
            account_id: row.account_id.to_string(),
            tenant_id: row.tenant_id.map(|value| value.to_string()),
            api_key_id: row.api_key_id.map(|value| value.to_string()),
            request_id: row.request_id,
            path: row.path,
            method: row.method,
            model: row.model,
            service_tier: row.service_tier,
            input_tokens: row.input_tokens,
            cached_input_tokens: row.cached_input_tokens,
            output_tokens: row.output_tokens,
            reasoning_tokens: row.reasoning_tokens,
            first_token_latency_ms: row.first_token_latency_ms,
            status_code: row.status_code,
            latency_ms: row.latency_ms,
            is_stream: if row.is_stream { 1 } else { 0 },
            error_code: row.error_code,
            billing_phase: row.billing_phase,
            authorization_id: row.authorization_id.map(|value| value.to_string()),
            capture_status: row.capture_status,
            created_at: row.created_at.timestamp(),
            event_version: row.event_version,
        }
    }
}

impl TryFrom<ClickHouseRequestLogQueryRow> for RequestLogRow {
    type Error = anyhow::Error;

    fn try_from(row: ClickHouseRequestLogQueryRow) -> Result<Self> {
        let id = Uuid::parse_str(&row.id)
            .with_context(|| format!("invalid request log id in clickhouse row: {}", row.id))?;
        let account_id = Uuid::parse_str(&row.account_id).with_context(|| {
            format!(
                "invalid request log account_id in clickhouse row: {}",
                row.account_id
            )
        })?;
        let tenant_id = row
            .tenant_id
            .as_deref()
            .map(Uuid::parse_str)
            .transpose()
            .with_context(|| {
                format!(
                    "invalid request log tenant_id in clickhouse row: {:?}",
                    row.tenant_id
                )
            })?;
        let api_key_id = row
            .api_key_id
            .as_deref()
            .map(Uuid::parse_str)
            .transpose()
            .with_context(|| {
                format!(
                    "invalid request log api_key_id in clickhouse row: {:?}",
                    row.api_key_id
                )
            })?;
        let created_at = chrono::DateTime::<chrono::Utc>::from_timestamp(row.created_at, 0)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "invalid request log created_at timestamp in clickhouse row: {}",
                    row.created_at
                )
            })?;
        Ok(Self {
            id,
            account_id,
            tenant_id,
            api_key_id,
            request_id: row.request_id,
            path: row.path,
            method: row.method,
            model: row.model,
            service_tier: row.service_tier,
            input_tokens: row.input_tokens,
            cached_input_tokens: row.cached_input_tokens,
            output_tokens: row.output_tokens,
            reasoning_tokens: row.reasoning_tokens,
            first_token_latency_ms: row.first_token_latency_ms,
            status_code: row.status_code,
            latency_ms: row.latency_ms,
            is_stream: row.is_stream > 0,
            error_code: row.error_code,
            billing_phase: row.billing_phase,
            authorization_id: row
                .authorization_id
                .as_deref()
                .map(Uuid::parse_str)
                .transpose()
                .with_context(|| {
                    format!(
                        "invalid request log authorization_id in clickhouse row: {:?}",
                        row.authorization_id
                    )
                })?,
            capture_status: row.capture_status,
            estimated_cost_microusd: None,
            created_at,
            event_version: row.event_version,
        })
    }
}

impl TryFrom<ClickHouseBillingReconcileFactRow> for BillingReconcileFact {
    type Error = anyhow::Error;

    fn try_from(row: ClickHouseBillingReconcileFactRow) -> Result<Self> {
        let id = Uuid::parse_str(&row.id)
            .with_context(|| format!("invalid reconcile row id in clickhouse: {}", row.id))?;
        let tenant_id = Uuid::parse_str(&row.tenant_id).with_context(|| {
            format!(
                "invalid reconcile row tenant_id in clickhouse: {}",
                row.tenant_id
            )
        })?;
        let api_key_id = row
            .api_key_id
            .as_deref()
            .map(Uuid::parse_str)
            .transpose()
            .with_context(|| {
                format!(
                    "invalid reconcile row api_key_id in clickhouse: {:?}",
                    row.api_key_id
                )
            })?;
        let created_at = chrono::DateTime::<chrono::Utc>::from_timestamp(row.created_at, 0)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "invalid reconcile row created_at timestamp in clickhouse: {}",
                    row.created_at
                )
            })?;

        Ok(Self {
            id,
            tenant_id,
            api_key_id,
            request_id: row.request_id,
            model: row.model,
            service_tier: row.service_tier,
            input_tokens: row.input_tokens,
            output_tokens: row.output_tokens,
            status_code: row.status_code,
            billing_phase: row.billing_phase,
            capture_status: row.capture_status,
            created_at,
        })
    }
}
