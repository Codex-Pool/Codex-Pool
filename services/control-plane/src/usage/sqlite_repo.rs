use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Timelike, Utc};
use codex_pool_core::api::{
    AccountUsageLeaderboardItem, ApiKeyUsageLeaderboardItem, HourlyAccountUsagePoint,
    HourlyTenantApiKeyUsagePoint, HourlyTenantUsageTotalPoint, HourlyUsageTotalPoint,
    TenantUsageLeaderboardItem, UsageDashboardMetrics, UsageDashboardModelDistributionItem,
    UsageDashboardTokenBreakdown, UsageDashboardTokenTrendPoint, UsageSummaryQueryResponse,
};
use codex_pool_core::events::RequestLogEvent;
use sqlx_sqlite::{SqlitePool, SqliteRow};
use uuid::Uuid;

use crate::cost::{calculate_estimated_cost_microusd, TokenPriceMicrousd};
use crate::usage::clickhouse_repo::UsageQueryRepository;
use crate::usage::{
    request_log_row_from_event, RequestLogQuery, RequestLogRow, UsageIngestRepository,
};
use crate::Row;

#[derive(Clone)]
pub struct SqliteUsageRepo {
    pool: SqlitePool,
}

#[derive(Debug, Clone)]
struct SummaryAggregateRow {
    total_requests: i64,
    unique_account_count: i64,
    tenant_api_key_total_requests: i64,
    unique_tenant_api_key_count: i64,
    input_tokens: i64,
    cached_input_tokens: i64,
    output_tokens: i64,
    reasoning_tokens: i64,
    avg_first_token_latency_ms: Option<f64>,
    estimated_cost_microusd: Option<i64>,
}

impl SqliteUsageRepo {
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        Self::initialize_schema(&pool).await?;
        Ok(Self { pool })
    }

    pub fn from_pool(pool: SqlitePool) -> Self {
        Self { pool }
    }

    async fn initialize_schema(pool: &SqlitePool) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS usage_request_logs (
                id TEXT PRIMARY KEY,
                account_id TEXT NOT NULL,
                tenant_id TEXT NULL,
                api_key_id TEXT NULL,
                request_id TEXT NULL,
                path TEXT NOT NULL,
                method TEXT NOT NULL,
                model TEXT NULL,
                service_tier TEXT NULL,
                input_tokens INTEGER NULL,
                cached_input_tokens INTEGER NULL,
                output_tokens INTEGER NULL,
                reasoning_tokens INTEGER NULL,
                first_token_latency_ms INTEGER NULL,
                status_code INTEGER NOT NULL,
                latency_ms INTEGER NOT NULL,
                is_stream INTEGER NOT NULL,
                error_code TEXT NULL,
                billing_phase TEXT NULL,
                authorization_id TEXT NULL,
                capture_status TEXT NULL,
                estimated_cost_microusd INTEGER NULL,
                created_at TEXT NOT NULL,
                created_at_ts INTEGER NOT NULL,
                hour_start_ts INTEGER NOT NULL,
                event_version INTEGER NOT NULL
            )
            "#,
        )
        .execute(pool)
        .await
        .context("failed to create sqlite usage_request_logs table")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_usage_request_logs_created_at_ts
            ON usage_request_logs (created_at_ts DESC)
            "#,
        )
        .execute(pool)
        .await
        .context("failed to create sqlite usage_request_logs created_at index")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_usage_request_logs_hour_start
            ON usage_request_logs (hour_start_ts, account_id)
            "#,
        )
        .execute(pool)
        .await
        .context("failed to create sqlite usage_request_logs hour_start index")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_usage_request_logs_request_id
            ON usage_request_logs (request_id)
            "#,
        )
        .execute(pool)
        .await
        .context("failed to create sqlite usage_request_logs request_id index")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS openai_models_catalog (
                model_id TEXT PRIMARY KEY,
                input_price_microcredits INTEGER NULL,
                cached_input_price_microcredits INTEGER NULL,
                output_price_microcredits INTEGER NULL
            )
            "#,
        )
        .execute(pool)
        .await
        .context("failed to create sqlite openai_models_catalog table")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS model_pricing_overrides (
                model TEXT NOT NULL,
                service_tier TEXT NOT NULL DEFAULT 'default',
                input_price_microcredits INTEGER NOT NULL,
                cached_input_price_microcredits INTEGER NOT NULL,
                output_price_microcredits INTEGER NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                PRIMARY KEY (model, service_tier)
            )
            "#,
        )
        .execute(pool)
        .await
        .context("failed to create sqlite model_pricing_overrides table")?;

        Ok(())
    }

    fn truncate_to_hour(created_at: DateTime<Utc>) -> DateTime<Utc> {
        created_at
            .with_minute(0)
            .and_then(|value| value.with_second(0))
            .and_then(|value| value.with_nanosecond(0))
            .unwrap_or(created_at)
    }

    fn as_u64(value: i64, field: &str) -> Result<u64> {
        u64::try_from(value).with_context(|| format!("{field} must be non-negative, got {value}"))
    }

    fn as_u16(value: i64, field: &str) -> Result<u16> {
        u16::try_from(value).with_context(|| format!("{field} is out of range: {value}"))
    }

    fn parse_uuid(raw: &str, field: &str) -> Result<Uuid> {
        Uuid::parse_str(raw).with_context(|| format!("failed to parse {field} uuid: {raw}"))
    }

    fn parse_optional_uuid(raw: Option<String>, field: &str) -> Result<Option<Uuid>> {
        raw.map(|value| Self::parse_uuid(&value, field)).transpose()
    }

    fn parse_created_at(raw: &str) -> Result<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(raw)
            .map(|value| value.with_timezone(&Utc))
            .with_context(|| format!("failed to parse created_at timestamp: {raw}"))
    }

    fn parse_i64_env_positive(key: &str) -> Option<i64> {
        std::env::var(key)
            .ok()
            .and_then(|raw| raw.parse::<i64>().ok())
            .filter(|value| *value > 0)
    }

    fn parse_i64_env_non_negative(key: &str) -> Option<i64> {
        std::env::var(key)
            .ok()
            .and_then(|raw| raw.parse::<i64>().ok())
            .filter(|value| *value >= 0)
    }

    fn normalize_cached_input_price_microusd(input_price: i64, cached_input_price: i64) -> i64 {
        if cached_input_price <= 0 {
            return input_price.max(0) / 10;
        }
        cached_input_price.max(0)
    }

    fn normalize_service_tier(raw: Option<&str>) -> String {
        match raw
            .unwrap_or("default")
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "priority" | "fast" => "priority".to_string(),
            "flex" => "flex".to_string(),
            _ => "default".to_string(),
        }
    }

    fn pricing_override_lookup_tiers(service_tier: &str) -> Vec<&str> {
        match service_tier {
            "priority" => vec!["priority", "default"],
            "flex" => vec!["flex", "default"],
            _ => vec!["default"],
        }
    }

    fn fallback_pricing() -> Option<TokenPriceMicrousd> {
        let input_price_microusd =
            Self::parse_i64_env_positive("BILLING_DEFAULT_INPUT_PRICE_MICROCREDITS")?;
        let output_price_microusd =
            Self::parse_i64_env_positive("BILLING_DEFAULT_OUTPUT_PRICE_MICROCREDITS")?;
        let cached_input_price_microusd =
            Self::parse_i64_env_non_negative("BILLING_DEFAULT_CACHED_INPUT_PRICE_MICROCREDITS")
                .unwrap_or(input_price_microusd / 10)
                .max(0);
        Some(TokenPriceMicrousd {
            input_price_microusd,
            cached_input_price_microusd,
            output_price_microusd,
        })
    }

    async fn resolve_base_model_pricing(
        &self,
        model: &str,
        service_tier: Option<&str>,
    ) -> Result<Option<TokenPriceMicrousd>> {
        let normalized_model = model.trim();
        if normalized_model.is_empty() {
            return Ok(None);
        }

        let normalized_service_tier = Self::normalize_service_tier(service_tier);
        for lookup_tier in Self::pricing_override_lookup_tiers(&normalized_service_tier) {
            if let Some(row) = sqlx::query(
                r#"
                SELECT input_price_microcredits, cached_input_price_microcredits, output_price_microcredits
                FROM model_pricing_overrides
                WHERE model = ? AND service_tier = ? AND enabled = 1
                "#,
            )
            .bind(normalized_model)
            .bind(lookup_tier)
            .fetch_optional(&self.pool)
            .await
            .context("failed to query sqlite model pricing override for usage cost")?
            {
                let input_price_microusd: i64 = row.try_get("input_price_microcredits")?;
                let cached_input_price_microusd = Self::normalize_cached_input_price_microusd(
                    input_price_microusd,
                    row.try_get("cached_input_price_microcredits")?,
                );
                return Ok(Some(TokenPriceMicrousd {
                    input_price_microusd,
                    cached_input_price_microusd,
                    output_price_microusd: row.try_get("output_price_microcredits")?,
                }));
            }
        }

        if let Some(row) = sqlx::query(
            r#"
            SELECT input_price_microcredits, cached_input_price_microcredits, output_price_microcredits
            FROM openai_models_catalog
            WHERE model_id = ?
            "#,
        )
        .bind(normalized_model)
        .fetch_optional(&self.pool)
        .await
        .context("failed to query sqlite official pricing catalog for usage cost")?
        {
            let input_price_microusd: i64 = row.try_get("input_price_microcredits")?;
            let cached_input_price_microusd = Self::normalize_cached_input_price_microusd(
                input_price_microusd,
                row.try_get("cached_input_price_microcredits")?,
            );
            return Ok(Some(TokenPriceMicrousd {
                input_price_microusd,
                cached_input_price_microusd,
                output_price_microusd: row.try_get("output_price_microcredits")?,
            }));
        }

        Ok(Self::fallback_pricing())
    }

    async fn resolve_request_log_cost_microusd(
        &self,
        model: Option<&str>,
        service_tier: Option<&str>,
        input_tokens: Option<i64>,
        cached_input_tokens: Option<i64>,
        output_tokens: Option<i64>,
    ) -> Result<Option<i64>> {
        let Some(model) = model.map(str::trim).filter(|value| !value.is_empty()) else {
            return Ok(None);
        };
        let Some(pricing) = self.resolve_base_model_pricing(model, service_tier).await? else {
            return Ok(None);
        };

        Ok(Some(calculate_estimated_cost_microusd(
            input_tokens.unwrap_or(0),
            cached_input_tokens.unwrap_or(0),
            output_tokens.unwrap_or(0),
            pricing,
        )))
    }

    fn summary_base_sql() -> &'static str {
        r#"
        FROM usage_request_logs
        WHERE created_at_ts >= ? AND created_at_ts <= ?
          AND (? IS NULL OR tenant_id = ?)
          AND (? IS NULL OR account_id = ?)
          AND (? IS NULL OR api_key_id = ?)
        "#
    }

    async fn fetch_summary_aggregate(
        &self,
        start_ts: i64,
        end_ts: i64,
        tenant_id: Option<Uuid>,
        account_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<SummaryAggregateRow> {
        let tenant_id = tenant_id.map(|value| value.to_string());
        let account_id = account_id.map(|value| value.to_string());
        let api_key_id = api_key_id.map(|value| value.to_string());
        let sql = format!(
            r#"
            SELECT
                COUNT(*) AS total_requests,
                COUNT(DISTINCT account_id) AS unique_account_count,
                COUNT(CASE WHEN tenant_id IS NOT NULL AND api_key_id IS NOT NULL THEN 1 END) AS tenant_api_key_total_requests,
                COUNT(DISTINCT CASE WHEN tenant_id IS NOT NULL AND api_key_id IS NOT NULL THEN tenant_id || ':' || api_key_id END) AS unique_tenant_api_key_count,
                COALESCE(SUM(input_tokens), 0) AS input_tokens,
                COALESCE(SUM(cached_input_tokens), 0) AS cached_input_tokens,
                COALESCE(SUM(output_tokens), 0) AS output_tokens,
                COALESCE(SUM(reasoning_tokens), 0) AS reasoning_tokens,
                AVG(first_token_latency_ms) AS avg_first_token_latency_ms,
                SUM(estimated_cost_microusd) AS estimated_cost_microusd
            {}
            "#,
            Self::summary_base_sql()
        );

        let row = sqlx::query(&sql)
            .bind(start_ts)
            .bind(end_ts)
            .bind(tenant_id.clone())
            .bind(tenant_id)
            .bind(account_id.clone())
            .bind(account_id)
            .bind(api_key_id.clone())
            .bind(api_key_id)
            .fetch_one(&self.pool)
            .await
            .context("failed to query sqlite usage summary aggregate")?;

        Ok(SummaryAggregateRow {
            total_requests: row.try_get("total_requests")?,
            unique_account_count: row.try_get("unique_account_count")?,
            tenant_api_key_total_requests: row.try_get("tenant_api_key_total_requests")?,
            unique_tenant_api_key_count: row.try_get("unique_tenant_api_key_count")?,
            input_tokens: row.try_get("input_tokens")?,
            cached_input_tokens: row.try_get("cached_input_tokens")?,
            output_tokens: row.try_get("output_tokens")?,
            reasoning_tokens: row.try_get("reasoning_tokens")?,
            avg_first_token_latency_ms: row.try_get("avg_first_token_latency_ms")?,
            estimated_cost_microusd: row.try_get("estimated_cost_microusd")?,
        })
    }

    async fn fetch_token_trends(
        &self,
        start_ts: i64,
        end_ts: i64,
        tenant_id: Option<Uuid>,
        account_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<Vec<UsageDashboardTokenTrendPoint>> {
        let tenant_id = tenant_id.map(|value| value.to_string());
        let account_id = account_id.map(|value| value.to_string());
        let api_key_id = api_key_id.map(|value| value.to_string());
        let sql = format!(
            r#"
            SELECT
                hour_start_ts,
                COUNT(*) AS request_count,
                COALESCE(SUM(input_tokens), 0) AS input_tokens,
                COALESCE(SUM(cached_input_tokens), 0) AS cached_input_tokens,
                COALESCE(SUM(output_tokens), 0) AS output_tokens,
                COALESCE(SUM(reasoning_tokens), 0) AS reasoning_tokens,
                SUM(estimated_cost_microusd) AS estimated_cost_microusd
            {}
            GROUP BY hour_start_ts
            ORDER BY hour_start_ts ASC
            "#,
            Self::summary_base_sql()
        );

        let rows = sqlx::query(&sql)
            .bind(start_ts)
            .bind(end_ts)
            .bind(tenant_id.clone())
            .bind(tenant_id)
            .bind(account_id.clone())
            .bind(account_id)
            .bind(api_key_id.clone())
            .bind(api_key_id)
            .fetch_all(&self.pool)
            .await
            .context("failed to query sqlite dashboard token trends")?;

        rows.into_iter()
            .map(|row: SqliteRow| {
                let input_tokens = Self::as_u64(row.try_get("input_tokens")?, "input_tokens")?;
                let cached_input_tokens =
                    Self::as_u64(row.try_get("cached_input_tokens")?, "cached_input_tokens")?;
                let output_tokens = Self::as_u64(row.try_get("output_tokens")?, "output_tokens")?;
                let reasoning_tokens =
                    Self::as_u64(row.try_get("reasoning_tokens")?, "reasoning_tokens")?;
                Ok(UsageDashboardTokenTrendPoint {
                    hour_start: row.try_get("hour_start_ts")?,
                    request_count: Self::as_u64(row.try_get("request_count")?, "request_count")?,
                    input_tokens,
                    cached_input_tokens,
                    output_tokens,
                    reasoning_tokens,
                    total_tokens: input_tokens
                        .saturating_add(cached_input_tokens)
                        .saturating_add(output_tokens)
                        .saturating_add(reasoning_tokens),
                    estimated_cost_microusd: row.try_get("estimated_cost_microusd")?,
                })
            })
            .collect()
    }

    async fn fetch_model_distribution(
        &self,
        start_ts: i64,
        end_ts: i64,
        tenant_id: Option<Uuid>,
        account_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
        order_by_tokens: bool,
    ) -> Result<Vec<UsageDashboardModelDistributionItem>> {
        let tenant_id = tenant_id.map(|value| value.to_string());
        let account_id = account_id.map(|value| value.to_string());
        let api_key_id = api_key_id.map(|value| value.to_string());
        let order_sql = if order_by_tokens {
            "ORDER BY total_tokens DESC, request_count DESC, model ASC"
        } else {
            "ORDER BY request_count DESC, total_tokens DESC, model ASC"
        };
        let sql = format!(
            r#"
            SELECT
                model,
                COUNT(*) AS request_count,
                COALESCE(SUM(COALESCE(input_tokens, 0) + COALESCE(cached_input_tokens, 0) + COALESCE(output_tokens, 0) + COALESCE(reasoning_tokens, 0)), 0) AS total_tokens
            {}
              AND model IS NOT NULL
              AND TRIM(model) <> ''
            GROUP BY model
            {}
            LIMIT 10
            "#,
            Self::summary_base_sql(),
            order_sql
        );

        let rows = sqlx::query(&sql)
            .bind(start_ts)
            .bind(end_ts)
            .bind(tenant_id.clone())
            .bind(tenant_id)
            .bind(account_id.clone())
            .bind(account_id)
            .bind(api_key_id.clone())
            .bind(api_key_id)
            .fetch_all(&self.pool)
            .await
            .context("failed to query sqlite dashboard model distribution")?;

        rows.into_iter()
            .map(|row: SqliteRow| {
                Ok(UsageDashboardModelDistributionItem {
                    model: row.try_get("model")?,
                    request_count: Self::as_u64(row.try_get("request_count")?, "request_count")?,
                    total_tokens: Self::as_u64(row.try_get("total_tokens")?, "total_tokens")?,
                })
            })
            .collect()
    }

    fn build_dashboard_metrics(
        aggregate: &SummaryAggregateRow,
        token_trends: Vec<UsageDashboardTokenTrendPoint>,
        model_request_distribution: Vec<UsageDashboardModelDistributionItem>,
        model_token_distribution: Vec<UsageDashboardModelDistributionItem>,
    ) -> Result<UsageDashboardMetrics> {
        let input_tokens = Self::as_u64(aggregate.input_tokens, "input_tokens")?;
        let cached_input_tokens =
            Self::as_u64(aggregate.cached_input_tokens, "cached_input_tokens")?;
        let output_tokens = Self::as_u64(aggregate.output_tokens, "output_tokens")?;
        let reasoning_tokens = Self::as_u64(aggregate.reasoning_tokens, "reasoning_tokens")?;

        Ok(UsageDashboardMetrics {
            total_requests: Self::as_u64(aggregate.total_requests, "total_requests")?,
            estimated_cost_microusd: aggregate.estimated_cost_microusd,
            token_breakdown: UsageDashboardTokenBreakdown {
                input_tokens,
                cached_input_tokens,
                output_tokens,
                reasoning_tokens,
                total_tokens: input_tokens
                    .saturating_add(cached_input_tokens)
                    .saturating_add(output_tokens)
                    .saturating_add(reasoning_tokens),
            },
            avg_first_token_latency_ms: aggregate
                .avg_first_token_latency_ms
                .map(|value| value.round() as u64),
            token_trends,
            model_request_distribution,
            model_token_distribution,
        })
    }
}

#[async_trait]
impl UsageIngestRepository for SqliteUsageRepo {
    async fn ingest_request_log(&self, event: RequestLogEvent) -> Result<()> {
        let mut request_log_row =
            request_log_row_from_event(&event, event.tenant_id, event.api_key_id);
        request_log_row.estimated_cost_microusd = self
            .resolve_request_log_cost_microusd(
                request_log_row.model.as_deref(),
                request_log_row.service_tier.as_deref(),
                request_log_row.input_tokens,
                request_log_row.cached_input_tokens,
                request_log_row.output_tokens,
            )
            .await?;
        let created_at = request_log_row.created_at.to_rfc3339();
        let created_at_ts = request_log_row.created_at.timestamp();
        let hour_start_ts = Self::truncate_to_hour(request_log_row.created_at).timestamp();

        sqlx::query(
            r#"
            INSERT OR IGNORE INTO usage_request_logs (
                id, account_id, tenant_id, api_key_id, request_id, path, method, model, service_tier,
                input_tokens, cached_input_tokens, output_tokens, reasoning_tokens,
                first_token_latency_ms, status_code, latency_ms, is_stream, error_code, billing_phase,
                authorization_id, capture_status, estimated_cost_microusd,
                created_at, created_at_ts, hour_start_ts, event_version
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(request_log_row.id.to_string())
        .bind(request_log_row.account_id.to_string())
        .bind(request_log_row.tenant_id.map(|value| value.to_string()))
        .bind(request_log_row.api_key_id.map(|value| value.to_string()))
        .bind(request_log_row.request_id)
        .bind(request_log_row.path)
        .bind(request_log_row.method)
        .bind(request_log_row.model)
        .bind(request_log_row.service_tier)
        .bind(request_log_row.input_tokens)
        .bind(request_log_row.cached_input_tokens)
        .bind(request_log_row.output_tokens)
        .bind(request_log_row.reasoning_tokens)
        .bind(request_log_row.first_token_latency_ms.map(|value| value as i64))
        .bind(i64::from(request_log_row.status_code))
        .bind(request_log_row.latency_ms as i64)
        .bind(if request_log_row.is_stream { 1_i64 } else { 0_i64 })
        .bind(request_log_row.error_code)
        .bind(request_log_row.billing_phase)
        .bind(request_log_row.authorization_id.map(|value| value.to_string()))
        .bind(request_log_row.capture_status)
        .bind(request_log_row.estimated_cost_microusd)
        .bind(created_at)
        .bind(created_at_ts)
        .bind(hour_start_ts)
        .bind(i64::from(request_log_row.event_version))
        .execute(&self.pool)
        .await
        .context("failed to insert sqlite usage request log")?;

        Ok(())
    }
}

#[async_trait]
impl UsageQueryRepository for SqliteUsageRepo {
    async fn query_hourly_accounts(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        account_id: Option<Uuid>,
    ) -> Result<Vec<HourlyAccountUsagePoint>> {
        let sql = r#"
            SELECT account_id, hour_start_ts, COUNT(*) AS request_count
            FROM usage_request_logs
            WHERE created_at_ts >= ? AND created_at_ts <= ?
              AND (? IS NULL OR account_id = ?)
            GROUP BY account_id, hour_start_ts
            ORDER BY hour_start_ts ASC, account_id ASC
            LIMIT ?
        "#;
        let account_id = account_id.map(|value| value.to_string());
        let rows = sqlx::query(sql)
            .bind(start_ts)
            .bind(end_ts)
            .bind(account_id.clone())
            .bind(account_id)
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await
            .context("failed to query sqlite hourly account usage")?;

        rows.into_iter()
            .map(|row| {
                Ok(HourlyAccountUsagePoint {
                    account_id: Self::parse_uuid(
                        &row.try_get::<String, _>("account_id")?,
                        "account_id",
                    )?,
                    hour_start: row.try_get("hour_start_ts")?,
                    request_count: Self::as_u64(row.try_get("request_count")?, "request_count")?,
                })
            })
            .collect()
    }

    async fn query_hourly_tenant_api_keys(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        tenant_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<Vec<HourlyTenantApiKeyUsagePoint>> {
        let tenant_id = tenant_id.map(|value| value.to_string());
        let api_key_id = api_key_id.map(|value| value.to_string());
        let rows = sqlx::query(
            r#"
            SELECT tenant_id, api_key_id, hour_start_ts, COUNT(*) AS request_count
            FROM usage_request_logs
            WHERE created_at_ts >= ? AND created_at_ts <= ?
              AND tenant_id IS NOT NULL
              AND api_key_id IS NOT NULL
              AND (? IS NULL OR tenant_id = ?)
              AND (? IS NULL OR api_key_id = ?)
            GROUP BY tenant_id, api_key_id, hour_start_ts
            ORDER BY hour_start_ts ASC, tenant_id ASC, api_key_id ASC
            LIMIT ?
            "#,
        )
        .bind(start_ts)
        .bind(end_ts)
        .bind(tenant_id.clone())
        .bind(tenant_id)
        .bind(api_key_id.clone())
        .bind(api_key_id)
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .context("failed to query sqlite hourly tenant/api key usage")?;

        rows.into_iter()
            .map(|row| {
                Ok(HourlyTenantApiKeyUsagePoint {
                    tenant_id: Self::parse_uuid(
                        &row.try_get::<String, _>("tenant_id")?,
                        "tenant_id",
                    )?,
                    api_key_id: Self::parse_uuid(
                        &row.try_get::<String, _>("api_key_id")?,
                        "api_key_id",
                    )?,
                    hour_start: row.try_get("hour_start_ts")?,
                    request_count: Self::as_u64(row.try_get("request_count")?, "request_count")?,
                })
            })
            .collect()
    }

    async fn query_hourly_account_totals(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        account_id: Option<Uuid>,
    ) -> Result<Vec<HourlyUsageTotalPoint>> {
        let account_id = account_id.map(|value| value.to_string());
        let rows = sqlx::query(
            r#"
            SELECT hour_start_ts, COUNT(*) AS request_count
            FROM usage_request_logs
            WHERE created_at_ts >= ? AND created_at_ts <= ?
              AND (? IS NULL OR account_id = ?)
            GROUP BY hour_start_ts
            ORDER BY hour_start_ts ASC
            LIMIT ?
            "#,
        )
        .bind(start_ts)
        .bind(end_ts)
        .bind(account_id.clone())
        .bind(account_id)
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .context("failed to query sqlite hourly account totals")?;

        rows.into_iter()
            .map(|row| {
                Ok(HourlyUsageTotalPoint {
                    hour_start: row.try_get("hour_start_ts")?,
                    request_count: Self::as_u64(row.try_get("request_count")?, "request_count")?,
                })
            })
            .collect()
    }

    async fn query_hourly_tenant_api_key_totals(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        tenant_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<Vec<HourlyUsageTotalPoint>> {
        let tenant_id = tenant_id.map(|value| value.to_string());
        let api_key_id = api_key_id.map(|value| value.to_string());
        let rows = sqlx::query(
            r#"
            SELECT hour_start_ts, COUNT(*) AS request_count
            FROM usage_request_logs
            WHERE created_at_ts >= ? AND created_at_ts <= ?
              AND tenant_id IS NOT NULL
              AND api_key_id IS NOT NULL
              AND (? IS NULL OR tenant_id = ?)
              AND (? IS NULL OR api_key_id = ?)
            GROUP BY hour_start_ts
            ORDER BY hour_start_ts ASC
            LIMIT ?
            "#,
        )
        .bind(start_ts)
        .bind(end_ts)
        .bind(tenant_id.clone())
        .bind(tenant_id)
        .bind(api_key_id.clone())
        .bind(api_key_id)
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .context("failed to query sqlite hourly tenant/api key totals")?;

        rows.into_iter()
            .map(|row| {
                Ok(HourlyUsageTotalPoint {
                    hour_start: row.try_get("hour_start_ts")?,
                    request_count: Self::as_u64(row.try_get("request_count")?, "request_count")?,
                })
            })
            .collect()
    }

    async fn query_hourly_tenant_totals(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        tenant_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<Vec<HourlyTenantUsageTotalPoint>> {
        let tenant_id = tenant_id.map(|value| value.to_string());
        let api_key_id = api_key_id.map(|value| value.to_string());
        let rows = sqlx::query(
            r#"
            SELECT tenant_id, hour_start_ts, COUNT(*) AS request_count
            FROM usage_request_logs
            WHERE created_at_ts >= ? AND created_at_ts <= ?
              AND tenant_id IS NOT NULL
              AND (? IS NULL OR tenant_id = ?)
              AND (? IS NULL OR api_key_id = ?)
            GROUP BY tenant_id, hour_start_ts
            ORDER BY hour_start_ts ASC, tenant_id ASC
            LIMIT ?
            "#,
        )
        .bind(start_ts)
        .bind(end_ts)
        .bind(tenant_id.clone())
        .bind(tenant_id)
        .bind(api_key_id.clone())
        .bind(api_key_id)
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .context("failed to query sqlite hourly tenant totals")?;

        rows.into_iter()
            .map(|row| {
                Ok(HourlyTenantUsageTotalPoint {
                    tenant_id: Self::parse_uuid(
                        &row.try_get::<String, _>("tenant_id")?,
                        "tenant_id",
                    )?,
                    hour_start: row.try_get("hour_start_ts")?,
                    request_count: Self::as_u64(row.try_get("request_count")?, "request_count")?,
                })
            })
            .collect()
    }

    async fn query_summary(
        &self,
        start_ts: i64,
        end_ts: i64,
        tenant_id: Option<Uuid>,
        account_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<UsageSummaryQueryResponse> {
        let aggregate = self
            .fetch_summary_aggregate(start_ts, end_ts, tenant_id, account_id, api_key_id)
            .await?;
        let token_trends = self
            .fetch_token_trends(start_ts, end_ts, tenant_id, account_id, api_key_id)
            .await?;
        let model_request_distribution = self
            .fetch_model_distribution(start_ts, end_ts, tenant_id, account_id, api_key_id, false)
            .await?;
        let model_token_distribution = self
            .fetch_model_distribution(start_ts, end_ts, tenant_id, account_id, api_key_id, true)
            .await?;
        let dashboard_metrics = Self::build_dashboard_metrics(
            &aggregate,
            token_trends,
            model_request_distribution,
            model_token_distribution,
        )?;

        Ok(UsageSummaryQueryResponse {
            start_ts,
            end_ts,
            account_total_requests: Self::as_u64(aggregate.total_requests, "total_requests")?,
            tenant_api_key_total_requests: Self::as_u64(
                aggregate.tenant_api_key_total_requests,
                "tenant_api_key_total_requests",
            )?,
            unique_account_count: Self::as_u64(
                aggregate.unique_account_count,
                "unique_account_count",
            )?,
            unique_tenant_api_key_count: Self::as_u64(
                aggregate.unique_tenant_api_key_count,
                "unique_tenant_api_key_count",
            )?,
            estimated_cost_microusd: aggregate.estimated_cost_microusd,
            dashboard_metrics: Some(dashboard_metrics),
        })
    }

    async fn query_tenant_leaderboard(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        tenant_id: Option<Uuid>,
    ) -> Result<Vec<TenantUsageLeaderboardItem>> {
        let tenant_id = tenant_id.map(|value| value.to_string());
        let rows = sqlx::query(
            r#"
            SELECT tenant_id, COUNT(*) AS total_requests
            FROM usage_request_logs
            WHERE created_at_ts >= ? AND created_at_ts <= ?
              AND tenant_id IS NOT NULL
              AND (? IS NULL OR tenant_id = ?)
            GROUP BY tenant_id
            ORDER BY total_requests DESC, tenant_id ASC
            LIMIT ?
            "#,
        )
        .bind(start_ts)
        .bind(end_ts)
        .bind(tenant_id.clone())
        .bind(tenant_id)
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .context("failed to query sqlite tenant leaderboard")?;

        rows.into_iter()
            .map(|row| {
                Ok(TenantUsageLeaderboardItem {
                    tenant_id: Self::parse_uuid(
                        &row.try_get::<String, _>("tenant_id")?,
                        "tenant_id",
                    )?,
                    total_requests: Self::as_u64(row.try_get("total_requests")?, "total_requests")?,
                })
            })
            .collect()
    }

    async fn query_account_leaderboard(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        account_id: Option<Uuid>,
    ) -> Result<Vec<AccountUsageLeaderboardItem>> {
        let account_id = account_id.map(|value| value.to_string());
        let rows = sqlx::query(
            r#"
            SELECT account_id, COUNT(*) AS total_requests
            FROM usage_request_logs
            WHERE created_at_ts >= ? AND created_at_ts <= ?
              AND (? IS NULL OR account_id = ?)
            GROUP BY account_id
            ORDER BY total_requests DESC, account_id ASC
            LIMIT ?
            "#,
        )
        .bind(start_ts)
        .bind(end_ts)
        .bind(account_id.clone())
        .bind(account_id)
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .context("failed to query sqlite account leaderboard")?;

        rows.into_iter()
            .map(|row| {
                Ok(AccountUsageLeaderboardItem {
                    account_id: Self::parse_uuid(
                        &row.try_get::<String, _>("account_id")?,
                        "account_id",
                    )?,
                    total_requests: Self::as_u64(row.try_get("total_requests")?, "total_requests")?,
                })
            })
            .collect()
    }

    async fn query_tenant_scoped_account_leaderboard(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        tenant_id: Uuid,
        account_id: Option<Uuid>,
    ) -> Result<Vec<AccountUsageLeaderboardItem>> {
        let tenant_id = tenant_id.to_string();
        let account_id = account_id.map(|value| value.to_string());
        let rows = sqlx::query(
            r#"
            SELECT account_id, COUNT(*) AS total_requests
            FROM usage_request_logs
            WHERE created_at_ts >= ? AND created_at_ts <= ?
              AND tenant_id = ?
              AND (? IS NULL OR account_id = ?)
            GROUP BY account_id
            ORDER BY total_requests DESC, account_id ASC
            LIMIT ?
            "#,
        )
        .bind(start_ts)
        .bind(end_ts)
        .bind(tenant_id)
        .bind(account_id.clone())
        .bind(account_id)
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .context("failed to query sqlite tenant-scoped account leaderboard")?;

        rows.into_iter()
            .map(|row| {
                Ok(AccountUsageLeaderboardItem {
                    account_id: Self::parse_uuid(
                        &row.try_get::<String, _>("account_id")?,
                        "account_id",
                    )?,
                    total_requests: Self::as_u64(row.try_get("total_requests")?, "total_requests")?,
                })
            })
            .collect()
    }

    async fn query_api_key_leaderboard(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        tenant_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<Vec<ApiKeyUsageLeaderboardItem>> {
        let tenant_id = tenant_id.map(|value| value.to_string());
        let api_key_id = api_key_id.map(|value| value.to_string());
        let rows = sqlx::query(
            r#"
            SELECT tenant_id, api_key_id, COUNT(*) AS total_requests
            FROM usage_request_logs
            WHERE created_at_ts >= ? AND created_at_ts <= ?
              AND tenant_id IS NOT NULL
              AND api_key_id IS NOT NULL
              AND (? IS NULL OR tenant_id = ?)
              AND (? IS NULL OR api_key_id = ?)
            GROUP BY tenant_id, api_key_id
            ORDER BY total_requests DESC, tenant_id ASC, api_key_id ASC
            LIMIT ?
            "#,
        )
        .bind(start_ts)
        .bind(end_ts)
        .bind(tenant_id.clone())
        .bind(tenant_id)
        .bind(api_key_id.clone())
        .bind(api_key_id)
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .context("failed to query sqlite api key leaderboard")?;

        rows.into_iter()
            .map(|row| {
                Ok(ApiKeyUsageLeaderboardItem {
                    tenant_id: Self::parse_uuid(
                        &row.try_get::<String, _>("tenant_id")?,
                        "tenant_id",
                    )?,
                    api_key_id: Self::parse_uuid(
                        &row.try_get::<String, _>("api_key_id")?,
                        "api_key_id",
                    )?,
                    total_requests: Self::as_u64(row.try_get("total_requests")?, "total_requests")?,
                })
            })
            .collect()
    }

    async fn query_request_logs(&self, query: RequestLogQuery) -> Result<Vec<RequestLogRow>> {
        let tenant_id = query.tenant_id.map(|value| value.to_string());
        let api_key_id = query.api_key_id.map(|value| value.to_string());
        let request_id = query.request_id.clone();
        let status_code = query.status_code.map(i64::from);
        let keyword = query.keyword.clone().map(|value| format!("%{value}%"));
        let rows = sqlx::query(
            r#"
            SELECT
                id, account_id, tenant_id, api_key_id, request_id, path, method, model, service_tier,
                input_tokens, cached_input_tokens, output_tokens, reasoning_tokens,
                first_token_latency_ms, status_code, latency_ms, is_stream, error_code,
                billing_phase, authorization_id, capture_status, estimated_cost_microusd,
                created_at, event_version
            FROM usage_request_logs
            WHERE created_at_ts >= ? AND created_at_ts <= ?
              AND (? IS NULL OR tenant_id = ?)
              AND (? IS NULL OR api_key_id = ?)
              AND (? IS NULL OR status_code = ?)
              AND (? IS NULL OR request_id = ?)
              AND (
                    ? IS NULL
                    OR request_id LIKE ?
                    OR model LIKE ?
                    OR path LIKE ?
                    OR error_code LIKE ?
                  )
            ORDER BY created_at_ts DESC, id DESC
            LIMIT ?
            "#,
        )
        .bind(query.start_ts)
        .bind(query.end_ts)
        .bind(tenant_id.clone())
        .bind(tenant_id)
        .bind(api_key_id.clone())
        .bind(api_key_id)
        .bind(status_code)
        .bind(status_code)
        .bind(request_id.clone())
        .bind(request_id)
        .bind(keyword.clone())
        .bind(keyword.clone())
        .bind(keyword.clone())
        .bind(keyword.clone())
        .bind(keyword)
        .bind(i64::from(query.limit))
        .fetch_all(&self.pool)
        .await
        .context("failed to query sqlite request logs")?;

        rows.into_iter()
            .map(|row| {
                let first_token_latency_ms = row
                    .try_get::<Option<i64>, _>("first_token_latency_ms")?
                    .map(|value| {
                        u64::try_from(value).context("first_token_latency_ms out of range")
                    })
                    .transpose()?;
                let latency_ms: i64 = row.try_get("latency_ms")?;
                let event_version: i64 = row.try_get("event_version")?;
                Ok(RequestLogRow {
                    id: Self::parse_uuid(&row.try_get::<String, _>("id")?, "id")?,
                    account_id: Self::parse_uuid(
                        &row.try_get::<String, _>("account_id")?,
                        "account_id",
                    )?,
                    tenant_id: Self::parse_optional_uuid(row.try_get("tenant_id")?, "tenant_id")?,
                    api_key_id: Self::parse_optional_uuid(
                        row.try_get("api_key_id")?,
                        "api_key_id",
                    )?,
                    request_id: row.try_get("request_id")?,
                    path: row.try_get("path")?,
                    method: row.try_get("method")?,
                    model: row.try_get("model")?,
                    service_tier: row.try_get("service_tier")?,
                    input_tokens: row.try_get("input_tokens")?,
                    cached_input_tokens: row.try_get("cached_input_tokens")?,
                    output_tokens: row.try_get("output_tokens")?,
                    reasoning_tokens: row.try_get("reasoning_tokens")?,
                    first_token_latency_ms,
                    status_code: Self::as_u16(row.try_get("status_code")?, "status_code")?,
                    latency_ms: Self::as_u64(latency_ms, "latency_ms")?,
                    is_stream: row.try_get::<i64, _>("is_stream")? != 0,
                    error_code: row.try_get("error_code")?,
                    billing_phase: row.try_get("billing_phase")?,
                    authorization_id: Self::parse_optional_uuid(
                        row.try_get("authorization_id")?,
                        "authorization_id",
                    )?,
                    capture_status: row.try_get("capture_status")?,
                    estimated_cost_microusd: row.try_get("estimated_cost_microusd")?,
                    created_at: Self::parse_created_at(&row.try_get::<String, _>("created_at")?)?,
                    event_version: u16::try_from(event_version)
                        .context("event_version out of range for u16")?,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::SqliteUsageRepo;
    use crate::store::normalize_sqlite_database_url;
    use crate::usage::clickhouse_repo::UsageQueryRepository;
    use crate::usage::UsageIngestRepository;
    use chrono::Utc;
    use codex_pool_core::events::RequestLogEvent;
    use sqlx_core::pool::PoolOptions;
    use sqlx_sqlite::Sqlite;
    use uuid::Uuid;

    async fn build_repo(name: &str) -> SqliteUsageRepo {
        let path = std::env::temp_dir().join(format!("{name}-{}.sqlite3", Uuid::new_v4()));
        let database_url = normalize_sqlite_database_url(&path.display().to_string());
        let pool = PoolOptions::<Sqlite>::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .expect("connect sqlite usage db");
        SqliteUsageRepo::new(pool)
            .await
            .expect("create sqlite usage repo")
    }

    #[tokio::test]
    async fn sqlite_usage_repo_ingests_logs_and_surfaces_cost() {
        std::env::set_var("BILLING_DEFAULT_INPUT_PRICE_MICROCREDITS", "1000000");
        std::env::set_var("BILLING_DEFAULT_CACHED_INPUT_PRICE_MICROCREDITS", "100000");
        std::env::set_var("BILLING_DEFAULT_OUTPUT_PRICE_MICROCREDITS", "4000000");

        let repo = build_repo("cp-usage-cost").await;
        let account_id = Uuid::new_v4();
        let api_key_id = Uuid::new_v4();
        let request_id = "req-sqlite-cost";
        repo.ingest_request_log(RequestLogEvent {
            id: Uuid::new_v4(),
            account_id,
            tenant_id: None,
            api_key_id: Some(api_key_id),
            event_version: 2,
            path: "/v1/responses".to_string(),
            method: "POST".to_string(),
            status_code: 200,
            latency_ms: 120,
            is_stream: false,
            error_code: None,
            request_id: Some(request_id.to_string()),
            model: Some("gpt-5-mini".to_string()),
            service_tier: None,
            input_tokens: Some(1_000),
            cached_input_tokens: Some(200),
            output_tokens: Some(500),
            reasoning_tokens: Some(25),
            first_token_latency_ms: Some(80),
            billing_phase: Some("captured".to_string()),
            authorization_id: None,
            capture_status: Some("captured".to_string()),
            created_at: Utc::now(),
        })
        .await
        .expect("ingest request log");

        let now_ts = Utc::now().timestamp();
        let summary = repo
            .query_summary(
                now_ts - 3600,
                now_ts + 3600,
                None,
                Some(account_id),
                Some(api_key_id),
            )
            .await
            .expect("query summary");
        let logs = repo
            .query_request_logs(crate::usage::RequestLogQuery {
                start_ts: now_ts - 3600,
                end_ts: now_ts + 3600,
                limit: 20,
                tenant_id: None,
                api_key_id: Some(api_key_id),
                status_code: None,
                request_id: Some(request_id.to_string()),
                keyword: None,
            })
            .await
            .expect("query request logs");

        assert_eq!(summary.account_total_requests, 1);
        assert!(summary.estimated_cost_microusd.is_some());
        assert_eq!(logs.len(), 1);
        assert_eq!(
            logs[0].estimated_cost_microusd,
            summary.estimated_cost_microusd
        );
    }
}
