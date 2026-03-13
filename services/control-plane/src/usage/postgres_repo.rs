use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Timelike, Utc};
use codex_pool_core::api::{
    AccountUsageLeaderboardItem, ApiKeyUsageLeaderboardItem, HourlyAccountUsagePoint,
    HourlyTenantApiKeyUsagePoint, HourlyTenantUsageTotalPoint, HourlyUsageTotalPoint,
    TenantUsageLeaderboardItem, UsageDashboardMetrics, UsageDashboardModelDistributionItem,
    UsageDashboardTokenBreakdown, UsageDashboardTokenTrendPoint, UsageSummaryQueryResponse,
};
use codex_pool_core::events::RequestLogEvent;
use sqlx_core::query_builder::QueryBuilder;
use sqlx_postgres::{PgPool, Postgres};
use uuid::Uuid;

use crate::Row;
use crate::cost::{calculate_estimated_cost_microusd, TokenPriceMicrousd};

use crate::usage::clickhouse_repo::UsageQueryRepository;
use crate::usage::{
    request_log_row_from_event, usage_rows_from_request_log_event, RequestLogQuery, RequestLogRow,
    UsageIngestRepository,
};

#[derive(Clone)]
pub struct PostgresUsageRepo {
    pool: PgPool,
}

impl PostgresUsageRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn timestamp_bound(ts: i64, label: &str) -> Result<DateTime<Utc>> {
        DateTime::<Utc>::from_timestamp(ts, 0)
            .ok_or_else(|| anyhow!("invalid {label} unix timestamp: {ts}"))
    }

    fn hourly_bound(ts: i64, label: &str) -> Result<DateTime<Utc>> {
        Ok(Self::truncate_to_hour(Self::timestamp_bound(ts, label)?))
    }

    fn truncate_to_hour(created_at: DateTime<Utc>) -> DateTime<Utc> {
        created_at
            .with_minute(0)
            .and_then(|value| value.with_second(0))
            .and_then(|value| value.with_nanosecond(0))
            .unwrap_or(created_at)
    }

    fn as_i64(value: u64, field: &str) -> Result<i64> {
        i64::try_from(value).with_context(|| format!("{field} exceeds i64 range"))
    }

    fn as_u64(value: i64, field: &str) -> Result<u64> {
        u64::try_from(value).with_context(|| format!("{field} must be non-negative, got {value}"))
    }

    fn as_u16(value: i32, field: &str) -> Result<u16> {
        u16::try_from(value).with_context(|| format!("{field} is out of range: {value}"))
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
            return input_price.max(0);
        }
        cached_input_price
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
                WHERE model = $1 AND service_tier = $2 AND enabled = true
                "#,
            )
            .bind(normalized_model)
            .bind(lookup_tier)
            .fetch_optional(&self.pool)
            .await
            .context("failed to query postgres model pricing override for usage cost")?
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
            WHERE model_id = $1
            "#,
        )
        .bind(normalized_model)
        .fetch_optional(&self.pool)
        .await
        .context("failed to query postgres official pricing catalog for usage cost")?
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

    async fn upsert_hourly_account_row(
        tx: &mut sqlx::Transaction<'_, Postgres>,
        row: crate::usage::HourlyAccountUsageRow,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO usage_hourly_account (account_id, hour_start, request_count)
            VALUES ($1, $2, $3)
            ON CONFLICT (account_id, hour_start) DO UPDATE
            SET request_count = usage_hourly_account.request_count + EXCLUDED.request_count
            "#,
        )
        .bind(row.account_id)
        .bind(row.hour_start)
        .bind(Self::as_i64(
            row.request_count,
            "usage_hourly_account.request_count",
        )?)
        .execute(tx.as_mut())
        .await
        .context("failed to upsert usage_hourly_account row")?;
        Ok(())
    }

    async fn upsert_hourly_tenant_api_key_row(
        tx: &mut sqlx::Transaction<'_, Postgres>,
        row: crate::usage::HourlyTenantApiKeyUsageRow,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO usage_hourly_tenant_api_key (tenant_id, api_key_id, hour_start, request_count)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (tenant_id, api_key_id, hour_start) DO UPDATE
            SET request_count = usage_hourly_tenant_api_key.request_count + EXCLUDED.request_count
            "#,
        )
        .bind(row.tenant_id)
        .bind(row.api_key_id)
        .bind(row.hour_start)
        .bind(Self::as_i64(
            row.request_count,
            "usage_hourly_tenant_api_key.request_count",
        )?)
        .execute(tx.as_mut())
        .await
        .context("failed to upsert usage_hourly_tenant_api_key row")?;
        Ok(())
    }

    async fn upsert_hourly_tenant_account_row(
        tx: &mut sqlx::Transaction<'_, Postgres>,
        row: crate::usage::HourlyTenantAccountUsageRow,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO usage_hourly_tenant_account (tenant_id, account_id, hour_start, request_count)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (tenant_id, account_id, hour_start) DO UPDATE
            SET request_count = usage_hourly_tenant_account.request_count + EXCLUDED.request_count
            "#,
        )
        .bind(row.tenant_id)
        .bind(row.account_id)
        .bind(row.hour_start)
        .bind(Self::as_i64(
            row.request_count,
            "usage_hourly_tenant_account.request_count",
        )?)
        .execute(tx.as_mut())
        .await
        .context("failed to upsert usage_hourly_tenant_account row")?;
        Ok(())
    }

    async fn fetch_dashboard_summary_row(
        &self,
        start_ts: i64,
        end_ts: i64,
        tenant_id: Option<Uuid>,
        account_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<DashboardSummaryRow> {
        let start_at = Self::timestamp_bound(start_ts, "start_ts")?;
        let end_at = Self::timestamp_bound(end_ts, "end_ts")?;
        let mut builder = QueryBuilder::<Postgres>::new(
            "SELECT COUNT(*) AS total_requests, \
             COALESCE(SUM(COALESCE(input_tokens, 0)), 0) AS input_tokens, \
             COALESCE(SUM(COALESCE(cached_input_tokens, 0)), 0) AS cached_input_tokens, \
             COALESCE(SUM(COALESCE(output_tokens, 0)), 0) AS output_tokens, \
             COALESCE(SUM(COALESCE(reasoning_tokens, 0)), 0) AS reasoning_tokens, \
             COALESCE(SUM(COALESCE(estimated_cost_microusd, 0)), 0) AS estimated_cost_microusd, \
             CASE \
                 WHEN COUNT(first_token_latency_ms) = 0 THEN NULL \
                 ELSE ROUND(AVG(first_token_latency_ms)::numeric, 0)::bigint \
             END AS avg_first_token_latency_ms \
             FROM usage_request_logs \
             WHERE created_at >= ",
        );
        builder.push_bind(start_at);
        builder.push(" AND created_at <= ");
        builder.push_bind(end_at);
        builder.push(" AND (billing_phase IS NULL OR billing_phase != 'streaming_open')");

        if let Some(tenant_id) = tenant_id {
            builder.push(" AND tenant_id = ");
            builder.push_bind(tenant_id);
        }
        if let Some(account_id) = account_id {
            builder.push(" AND account_id = ");
            builder.push_bind(account_id);
        }
        if let Some(api_key_id) = api_key_id {
            builder.push(" AND api_key_id = ");
            builder.push_bind(api_key_id);
        }

        let row = builder
            .build()
            .fetch_one(&self.pool)
            .await
            .context("failed to query postgres dashboard summary")?;

        Ok(DashboardSummaryRow {
            total_requests: Self::as_u64(
                row.try_get::<i64, _>("total_requests")?,
                "total_requests",
            )?,
            input_tokens: Self::as_u64(row.try_get::<i64, _>("input_tokens")?, "input_tokens")?,
            cached_input_tokens: Self::as_u64(
                row.try_get::<i64, _>("cached_input_tokens")?,
                "cached_input_tokens",
            )?,
            output_tokens: Self::as_u64(row.try_get::<i64, _>("output_tokens")?, "output_tokens")?,
            reasoning_tokens: Self::as_u64(
                row.try_get::<i64, _>("reasoning_tokens")?,
                "reasoning_tokens",
            )?,
            estimated_cost_microusd: row.try_get("estimated_cost_microusd")?,
            avg_first_token_latency_ms: row
                .try_get::<Option<i64>, _>("avg_first_token_latency_ms")?
                .map(|value| Self::as_u64(value, "avg_first_token_latency_ms"))
                .transpose()?,
        })
    }

    async fn fetch_dashboard_token_trends(
        &self,
        start_ts: i64,
        end_ts: i64,
        tenant_id: Option<Uuid>,
        account_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<Vec<UsageDashboardTokenTrendPoint>> {
        let start_at = Self::timestamp_bound(start_ts, "start_ts")?;
        let end_at = Self::timestamp_bound(end_ts, "end_ts")?;
        let point_limit = ((end_ts.saturating_sub(start_ts) / 3600) + 2).clamp(1, 10_000);
        let mut builder = QueryBuilder::<Postgres>::new(
            "SELECT date_trunc('hour', created_at) AS hour_start, \
             COUNT(*) AS request_count, \
             COALESCE(SUM(COALESCE(input_tokens, 0)), 0) AS input_tokens, \
             COALESCE(SUM(COALESCE(cached_input_tokens, 0)), 0) AS cached_input_tokens, \
             COALESCE(SUM(COALESCE(output_tokens, 0)), 0) AS output_tokens, \
             COALESCE(SUM(COALESCE(reasoning_tokens, 0)), 0) AS reasoning_tokens, \
             COALESCE(SUM(COALESCE(estimated_cost_microusd, 0)), 0) AS estimated_cost_microusd \
             FROM usage_request_logs \
             WHERE created_at >= ",
        );
        builder.push_bind(start_at);
        builder.push(" AND created_at <= ");
        builder.push_bind(end_at);
        builder.push(" AND (billing_phase IS NULL OR billing_phase != 'streaming_open')");

        if let Some(tenant_id) = tenant_id {
            builder.push(" AND tenant_id = ");
            builder.push_bind(tenant_id);
        }
        if let Some(account_id) = account_id {
            builder.push(" AND account_id = ");
            builder.push_bind(account_id);
        }
        if let Some(api_key_id) = api_key_id {
            builder.push(" AND api_key_id = ");
            builder.push_bind(api_key_id);
        }

        builder.push(" GROUP BY hour_start ORDER BY hour_start ASC LIMIT ");
        builder.push_bind(point_limit);

        let rows = builder
            .build()
            .fetch_all(&self.pool)
            .await
            .context("failed to query postgres dashboard token trends")?;

        rows.into_iter()
            .map(|row| {
                let hour_start = row.try_get::<DateTime<Utc>, _>("hour_start")?.timestamp();
                let request_count =
                    Self::as_u64(row.try_get::<i64, _>("request_count")?, "request_count")?;
                let input_tokens =
                    Self::as_u64(row.try_get::<i64, _>("input_tokens")?, "input_tokens")?;
                let cached_input_tokens = Self::as_u64(
                    row.try_get::<i64, _>("cached_input_tokens")?,
                    "cached_input_tokens",
                )?;
                let output_tokens =
                    Self::as_u64(row.try_get::<i64, _>("output_tokens")?, "output_tokens")?;
                let reasoning_tokens = Self::as_u64(
                    row.try_get::<i64, _>("reasoning_tokens")?,
                    "reasoning_tokens",
                )?;
                Ok(UsageDashboardTokenTrendPoint {
                    hour_start,
                    request_count,
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

    async fn fetch_dashboard_model_distribution(
        &self,
        start_ts: i64,
        end_ts: i64,
        tenant_id: Option<Uuid>,
        account_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
        order_by_tokens: bool,
    ) -> Result<Vec<UsageDashboardModelDistributionItem>> {
        let start_at = Self::timestamp_bound(start_ts, "start_ts")?;
        let end_at = Self::timestamp_bound(end_ts, "end_ts")?;
        let mut builder = QueryBuilder::<Postgres>::new(
            "SELECT COALESCE(NULLIF(model, ''), 'unknown') AS model, \
             COUNT(*) AS request_count, \
             COALESCE(SUM(COALESCE(input_tokens, 0) + COALESCE(cached_input_tokens, 0) + COALESCE(output_tokens, 0) + COALESCE(reasoning_tokens, 0)), 0) AS total_tokens \
             FROM usage_request_logs \
             WHERE created_at >= ",
        );
        builder.push_bind(start_at);
        builder.push(" AND created_at <= ");
        builder.push_bind(end_at);
        builder.push(" AND (billing_phase IS NULL OR billing_phase != 'streaming_open')");

        if let Some(tenant_id) = tenant_id {
            builder.push(" AND tenant_id = ");
            builder.push_bind(tenant_id);
        }
        if let Some(account_id) = account_id {
            builder.push(" AND account_id = ");
            builder.push_bind(account_id);
        }
        if let Some(api_key_id) = api_key_id {
            builder.push(" AND api_key_id = ");
            builder.push_bind(api_key_id);
        }

        builder.push(" GROUP BY model");
        if order_by_tokens {
            builder.push(" ORDER BY total_tokens DESC, request_count DESC, model ASC LIMIT 50");
        } else {
            builder.push(" ORDER BY request_count DESC, total_tokens DESC, model ASC LIMIT 50");
        }

        let rows = builder
            .build()
            .fetch_all(&self.pool)
            .await
            .context("failed to query postgres dashboard model distribution")?;

        rows.into_iter()
            .map(|row| {
                Ok(UsageDashboardModelDistributionItem {
                    model: row.try_get::<String, _>("model")?,
                    request_count: Self::as_u64(
                        row.try_get::<i64, _>("request_count")?,
                        "request_count",
                    )?,
                    total_tokens: Self::as_u64(
                        row.try_get::<i64, _>("total_tokens")?,
                        "total_tokens",
                    )?,
                })
            })
            .collect()
    }
}

#[derive(Debug)]
struct DashboardSummaryRow {
    total_requests: u64,
    input_tokens: u64,
    cached_input_tokens: u64,
    output_tokens: u64,
    reasoning_tokens: u64,
    estimated_cost_microusd: Option<i64>,
    avg_first_token_latency_ms: Option<u64>,
}

#[async_trait]
impl UsageIngestRepository for PostgresUsageRepo {
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
        let usage_rows =
            usage_rows_from_request_log_event(&event, event.tenant_id, event.api_key_id);
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to begin postgres usage ingest transaction")?;

        let inserted = sqlx::query(
            r#"
            INSERT INTO usage_request_logs (
                id,
                account_id,
                tenant_id,
                api_key_id,
                request_id,
                path,
                method,
                model,
                service_tier,
                input_tokens,
                cached_input_tokens,
                output_tokens,
                reasoning_tokens,
                first_token_latency_ms,
                status_code,
                latency_ms,
                is_stream,
                error_code,
                billing_phase,
                authorization_id,
                capture_status,
                estimated_cost_microusd,
                created_at,
                event_version
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
                $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24
            )
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(request_log_row.id)
        .bind(request_log_row.account_id)
        .bind(request_log_row.tenant_id)
        .bind(request_log_row.api_key_id)
        .bind(request_log_row.request_id.as_deref())
        .bind(request_log_row.path.as_str())
        .bind(request_log_row.method.as_str())
        .bind(request_log_row.model.as_deref())
        .bind(request_log_row.service_tier.as_deref())
        .bind(request_log_row.input_tokens)
        .bind(request_log_row.cached_input_tokens)
        .bind(request_log_row.output_tokens)
        .bind(request_log_row.reasoning_tokens)
        .bind(
            request_log_row
                .first_token_latency_ms
                .map(|value| Self::as_i64(value, "first_token_latency_ms"))
                .transpose()?,
        )
        .bind(i32::from(request_log_row.status_code))
        .bind(Self::as_i64(request_log_row.latency_ms, "latency_ms")?)
        .bind(request_log_row.is_stream)
        .bind(request_log_row.error_code.as_deref())
        .bind(request_log_row.billing_phase.as_deref())
        .bind(request_log_row.authorization_id)
        .bind(request_log_row.capture_status.as_deref())
        .bind(request_log_row.estimated_cost_microusd)
        .bind(request_log_row.created_at)
        .bind(i32::from(request_log_row.event_version))
        .execute(tx.as_mut())
        .await
        .context("failed to insert usage_request_logs row")?
        .rows_affected()
            == 1;

        if !inserted {
            tx.commit()
                .await
                .context("failed to commit postgres usage ingest noop transaction")?;
            return Ok(());
        }

        for row in usage_rows.account_rows {
            Self::upsert_hourly_account_row(&mut tx, row).await?;
        }
        for row in usage_rows.tenant_api_key_rows {
            Self::upsert_hourly_tenant_api_key_row(&mut tx, row).await?;
        }
        for row in usage_rows.tenant_account_rows {
            Self::upsert_hourly_tenant_account_row(&mut tx, row).await?;
        }

        tx.commit()
            .await
            .context("failed to commit postgres usage ingest transaction")?;
        Ok(())
    }
}

#[async_trait]
impl UsageQueryRepository for PostgresUsageRepo {
    async fn query_hourly_accounts(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        account_id: Option<Uuid>,
    ) -> Result<Vec<HourlyAccountUsagePoint>> {
        let start_at = Self::hourly_bound(start_ts, "start_ts")?;
        let end_at = Self::hourly_bound(end_ts, "end_ts")?;
        let mut builder = QueryBuilder::<Postgres>::new(
            "SELECT account_id, hour_start, request_count \
             FROM usage_hourly_account \
             WHERE hour_start >= ",
        );
        builder.push_bind(start_at);
        builder.push(" AND hour_start <= ");
        builder.push_bind(end_at);
        if let Some(account_id) = account_id {
            builder.push(" AND account_id = ");
            builder.push_bind(account_id);
        }
        builder.push(" ORDER BY hour_start ASC, account_id ASC LIMIT ");
        builder.push_bind(i64::from(limit));

        let rows = builder
            .build()
            .fetch_all(&self.pool)
            .await
            .context("failed to query postgres hourly account usage")?;

        rows.into_iter()
            .map(|row| {
                Ok(HourlyAccountUsagePoint {
                    account_id: row.try_get("account_id")?,
                    hour_start: row.try_get::<DateTime<Utc>, _>("hour_start")?.timestamp(),
                    request_count: Self::as_u64(
                        row.try_get::<i64, _>("request_count")?,
                        "request_count",
                    )?,
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
        let start_at = Self::hourly_bound(start_ts, "start_ts")?;
        let end_at = Self::hourly_bound(end_ts, "end_ts")?;
        let mut builder = QueryBuilder::<Postgres>::new(
            "SELECT tenant_id, api_key_id, hour_start, request_count \
             FROM usage_hourly_tenant_api_key \
             WHERE hour_start >= ",
        );
        builder.push_bind(start_at);
        builder.push(" AND hour_start <= ");
        builder.push_bind(end_at);
        if let Some(tenant_id) = tenant_id {
            builder.push(" AND tenant_id = ");
            builder.push_bind(tenant_id);
        }
        if let Some(api_key_id) = api_key_id {
            builder.push(" AND api_key_id = ");
            builder.push_bind(api_key_id);
        }
        builder.push(" ORDER BY hour_start ASC, tenant_id ASC, api_key_id ASC LIMIT ");
        builder.push_bind(i64::from(limit));

        let rows = builder
            .build()
            .fetch_all(&self.pool)
            .await
            .context("failed to query postgres hourly tenant api-key usage")?;

        rows.into_iter()
            .map(|row| {
                Ok(HourlyTenantApiKeyUsagePoint {
                    tenant_id: row.try_get("tenant_id")?,
                    api_key_id: row.try_get("api_key_id")?,
                    hour_start: row.try_get::<DateTime<Utc>, _>("hour_start")?.timestamp(),
                    request_count: Self::as_u64(
                        row.try_get::<i64, _>("request_count")?,
                        "request_count",
                    )?,
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
        let start_at = Self::hourly_bound(start_ts, "start_ts")?;
        let end_at = Self::hourly_bound(end_ts, "end_ts")?;
        let mut builder = QueryBuilder::<Postgres>::new(
            "SELECT hour_start, COALESCE(SUM(request_count), 0) AS request_count \
             FROM usage_hourly_account \
             WHERE hour_start >= ",
        );
        builder.push_bind(start_at);
        builder.push(" AND hour_start < ");
        builder.push_bind(end_at + chrono::Duration::hours(1));
        if let Some(account_id) = account_id {
            builder.push(" AND account_id = ");
            builder.push_bind(account_id);
        }
        builder.push(" GROUP BY hour_start ORDER BY hour_start ASC LIMIT ");
        builder.push_bind(i64::from(limit));

        let rows = builder
            .build()
            .fetch_all(&self.pool)
            .await
            .context("failed to query postgres hourly account totals")?;

        rows.into_iter()
            .map(|row| {
                Ok(HourlyUsageTotalPoint {
                    hour_start: row.try_get::<DateTime<Utc>, _>("hour_start")?.timestamp(),
                    request_count: Self::as_u64(
                        row.try_get::<i64, _>("request_count")?,
                        "request_count",
                    )?,
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
        let start_at = Self::hourly_bound(start_ts, "start_ts")?;
        let end_at = Self::hourly_bound(end_ts, "end_ts")?;
        let mut builder = QueryBuilder::<Postgres>::new(
            "SELECT hour_start, COALESCE(SUM(request_count), 0) AS request_count \
             FROM usage_hourly_tenant_api_key \
             WHERE hour_start >= ",
        );
        builder.push_bind(start_at);
        builder.push(" AND hour_start < ");
        builder.push_bind(end_at + chrono::Duration::hours(1));
        if let Some(tenant_id) = tenant_id {
            builder.push(" AND tenant_id = ");
            builder.push_bind(tenant_id);
        }
        if let Some(api_key_id) = api_key_id {
            builder.push(" AND api_key_id = ");
            builder.push_bind(api_key_id);
        }
        builder.push(" GROUP BY hour_start ORDER BY hour_start ASC LIMIT ");
        builder.push_bind(i64::from(limit));

        let rows = builder
            .build()
            .fetch_all(&self.pool)
            .await
            .context("failed to query postgres hourly tenant api-key totals")?;

        rows.into_iter()
            .map(|row| {
                Ok(HourlyUsageTotalPoint {
                    hour_start: row.try_get::<DateTime<Utc>, _>("hour_start")?.timestamp(),
                    request_count: Self::as_u64(
                        row.try_get::<i64, _>("request_count")?,
                        "request_count",
                    )?,
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
        let start_at = Self::hourly_bound(start_ts, "start_ts")?;
        let end_at = Self::hourly_bound(end_ts, "end_ts")?;
        let mut builder = QueryBuilder::<Postgres>::new(
            "SELECT tenant_id, hour_start, COALESCE(SUM(request_count), 0) AS request_count \
             FROM usage_hourly_tenant_api_key \
             WHERE hour_start >= ",
        );
        builder.push_bind(start_at);
        builder.push(" AND hour_start < ");
        builder.push_bind(end_at + chrono::Duration::hours(1));
        if let Some(tenant_id) = tenant_id {
            builder.push(" AND tenant_id = ");
            builder.push_bind(tenant_id);
        }
        if let Some(api_key_id) = api_key_id {
            builder.push(" AND api_key_id = ");
            builder.push_bind(api_key_id);
        }
        builder
            .push(" GROUP BY tenant_id, hour_start ORDER BY hour_start ASC, tenant_id ASC LIMIT ");
        builder.push_bind(i64::from(limit));

        let rows = builder
            .build()
            .fetch_all(&self.pool)
            .await
            .context("failed to query postgres hourly tenant totals")?;

        rows.into_iter()
            .map(|row| {
                Ok(HourlyTenantUsageTotalPoint {
                    tenant_id: row.try_get("tenant_id")?,
                    hour_start: row.try_get::<DateTime<Utc>, _>("hour_start")?.timestamp(),
                    request_count: Self::as_u64(
                        row.try_get::<i64, _>("request_count")?,
                        "request_count",
                    )?,
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
        let hourly_start = Self::hourly_bound(start_ts, "start_ts")?;
        let hourly_end = Self::hourly_bound(end_ts, "end_ts")?;

        let mut account_summary_builder = QueryBuilder::<Postgres>::new(
            "SELECT COALESCE(SUM(request_count), 0) AS account_total_requests, \
             COUNT(DISTINCT account_id) AS unique_account_count \
             FROM usage_hourly_account \
             WHERE hour_start >= ",
        );
        account_summary_builder.push_bind(hourly_start);
        account_summary_builder.push(" AND hour_start <= ");
        account_summary_builder.push_bind(hourly_end);
        if let Some(account_id) = account_id {
            account_summary_builder.push(" AND account_id = ");
            account_summary_builder.push_bind(account_id);
        }

        let account_summary_row = account_summary_builder
            .build()
            .fetch_one(&self.pool)
            .await
            .context("failed to query postgres account usage summary")?;

        let mut tenant_api_key_summary_builder = QueryBuilder::<Postgres>::new(
            "SELECT COALESCE(SUM(request_count), 0) AS tenant_api_key_total_requests, \
             COUNT(DISTINCT (tenant_id, api_key_id)) AS unique_tenant_api_key_count \
             FROM usage_hourly_tenant_api_key \
             WHERE hour_start >= ",
        );
        tenant_api_key_summary_builder.push_bind(hourly_start);
        tenant_api_key_summary_builder.push(" AND hour_start <= ");
        tenant_api_key_summary_builder.push_bind(hourly_end);
        if let Some(tenant_id) = tenant_id {
            tenant_api_key_summary_builder.push(" AND tenant_id = ");
            tenant_api_key_summary_builder.push_bind(tenant_id);
        }
        if let Some(api_key_id) = api_key_id {
            tenant_api_key_summary_builder.push(" AND api_key_id = ");
            tenant_api_key_summary_builder.push_bind(api_key_id);
        }

        let tenant_api_key_summary_row = tenant_api_key_summary_builder
            .build()
            .fetch_one(&self.pool)
            .await
            .context("failed to query postgres tenant api-key usage summary")?;

        let dashboard_summary = match self
            .fetch_dashboard_summary_row(start_ts, end_ts, tenant_id, account_id, api_key_id)
            .await
        {
            Ok(summary) => Some(summary),
            Err(err) => {
                tracing::warn!(
                    error = ?err,
                    ?tenant_id,
                    ?account_id,
                    ?api_key_id,
                    start_ts,
                    end_ts,
                    "postgres dashboard summary query failed; falling back to summary-only response"
                );
                None
            }
        };

        let dashboard_metrics = match dashboard_summary.as_ref() {
            Some(dashboard_summary) => match tokio::try_join!(
                self.fetch_dashboard_token_trends(start_ts, end_ts, tenant_id, account_id, api_key_id),
                self.fetch_dashboard_model_distribution(
                    start_ts, end_ts, tenant_id, account_id, api_key_id, false
                ),
                self.fetch_dashboard_model_distribution(
                    start_ts, end_ts, tenant_id, account_id, api_key_id, true
                ),
            ) {
                Ok((token_trends, model_request_distribution, model_token_distribution)) => {
                    Some(UsageDashboardMetrics {
                        total_requests: dashboard_summary.total_requests,
                        estimated_cost_microusd: dashboard_summary.estimated_cost_microusd,
                        token_breakdown: UsageDashboardTokenBreakdown {
                            input_tokens: dashboard_summary.input_tokens,
                            cached_input_tokens: dashboard_summary.cached_input_tokens,
                            output_tokens: dashboard_summary.output_tokens,
                            reasoning_tokens: dashboard_summary.reasoning_tokens,
                            total_tokens: dashboard_summary
                                .input_tokens
                                .saturating_add(dashboard_summary.cached_input_tokens)
                                .saturating_add(dashboard_summary.output_tokens)
                                .saturating_add(dashboard_summary.reasoning_tokens),
                        },
                        avg_first_token_latency_ms: dashboard_summary.avg_first_token_latency_ms,
                        token_trends,
                        model_request_distribution,
                        model_token_distribution,
                    })
                }
                Err(err) => {
                    tracing::warn!(
                        error = ?err,
                        ?tenant_id,
                        ?account_id,
                        ?api_key_id,
                        start_ts,
                        end_ts,
                        "postgres dashboard detail query failed; falling back to summary-only response"
                    );
                    None
                }
            },
            None => None,
        };

        Ok(UsageSummaryQueryResponse {
            start_ts,
            end_ts,
            account_total_requests: Self::as_u64(
                account_summary_row.try_get::<i64, _>("account_total_requests")?,
                "account_total_requests",
            )?,
            tenant_api_key_total_requests: Self::as_u64(
                tenant_api_key_summary_row.try_get::<i64, _>("tenant_api_key_total_requests")?,
                "tenant_api_key_total_requests",
            )?,
            unique_account_count: Self::as_u64(
                account_summary_row.try_get::<i64, _>("unique_account_count")?,
                "unique_account_count",
            )?,
            unique_tenant_api_key_count: Self::as_u64(
                tenant_api_key_summary_row.try_get::<i64, _>("unique_tenant_api_key_count")?,
                "unique_tenant_api_key_count",
            )?,
            estimated_cost_microusd: dashboard_summary
                .as_ref()
                .and_then(|summary| summary.estimated_cost_microusd),
            dashboard_metrics,
        })
    }

    async fn query_tenant_leaderboard(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        tenant_id: Option<Uuid>,
    ) -> Result<Vec<TenantUsageLeaderboardItem>> {
        let start_at = Self::hourly_bound(start_ts, "start_ts")?;
        let end_at = Self::hourly_bound(end_ts, "end_ts")?;
        let mut builder = QueryBuilder::<Postgres>::new(
            "SELECT tenant_id, COALESCE(SUM(request_count), 0) AS total_requests \
             FROM usage_hourly_tenant_api_key \
             WHERE hour_start >= ",
        );
        builder.push_bind(start_at);
        builder.push(" AND hour_start <= ");
        builder.push_bind(end_at);
        if let Some(tenant_id) = tenant_id {
            builder.push(" AND tenant_id = ");
            builder.push_bind(tenant_id);
        }
        builder.push(" GROUP BY tenant_id ORDER BY total_requests DESC, tenant_id ASC LIMIT ");
        builder.push_bind(i64::from(limit));

        let rows = builder
            .build()
            .fetch_all(&self.pool)
            .await
            .context("failed to query postgres tenant usage leaderboard")?;

        rows.into_iter()
            .map(|row| {
                Ok(TenantUsageLeaderboardItem {
                    tenant_id: row.try_get("tenant_id")?,
                    total_requests: Self::as_u64(
                        row.try_get::<i64, _>("total_requests")?,
                        "total_requests",
                    )?,
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
        let start_at = Self::hourly_bound(start_ts, "start_ts")?;
        let end_at = Self::hourly_bound(end_ts, "end_ts")?;
        let mut builder = QueryBuilder::<Postgres>::new(
            "SELECT account_id, COALESCE(SUM(request_count), 0) AS total_requests \
             FROM usage_hourly_account \
             WHERE hour_start >= ",
        );
        builder.push_bind(start_at);
        builder.push(" AND hour_start <= ");
        builder.push_bind(end_at);
        if let Some(account_id) = account_id {
            builder.push(" AND account_id = ");
            builder.push_bind(account_id);
        }
        builder.push(" GROUP BY account_id ORDER BY total_requests DESC, account_id ASC LIMIT ");
        builder.push_bind(i64::from(limit));

        let rows = builder
            .build()
            .fetch_all(&self.pool)
            .await
            .context("failed to query postgres account usage leaderboard")?;

        rows.into_iter()
            .map(|row| {
                Ok(AccountUsageLeaderboardItem {
                    account_id: row.try_get("account_id")?,
                    total_requests: Self::as_u64(
                        row.try_get::<i64, _>("total_requests")?,
                        "total_requests",
                    )?,
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
        let start_at = Self::hourly_bound(start_ts, "start_ts")?;
        let end_at = Self::hourly_bound(end_ts, "end_ts")?;
        let mut builder = QueryBuilder::<Postgres>::new(
            "SELECT account_id, COALESCE(SUM(request_count), 0) AS total_requests \
             FROM usage_hourly_tenant_account \
             WHERE hour_start >= ",
        );
        builder.push_bind(start_at);
        builder.push(" AND hour_start <= ");
        builder.push_bind(end_at);
        builder.push(" AND tenant_id = ");
        builder.push_bind(tenant_id);
        if let Some(account_id) = account_id {
            builder.push(" AND account_id = ");
            builder.push_bind(account_id);
        }
        builder.push(" GROUP BY account_id ORDER BY total_requests DESC, account_id ASC LIMIT ");
        builder.push_bind(i64::from(limit));

        let rows = builder
            .build()
            .fetch_all(&self.pool)
            .await
            .context("failed to query postgres tenant-scoped account usage leaderboard")?;

        rows.into_iter()
            .map(|row| {
                Ok(AccountUsageLeaderboardItem {
                    account_id: row.try_get("account_id")?,
                    total_requests: Self::as_u64(
                        row.try_get::<i64, _>("total_requests")?,
                        "total_requests",
                    )?,
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
        let start_at = Self::hourly_bound(start_ts, "start_ts")?;
        let end_at = Self::hourly_bound(end_ts, "end_ts")?;
        let mut builder = QueryBuilder::<Postgres>::new(
            "SELECT tenant_id, api_key_id, COALESCE(SUM(request_count), 0) AS total_requests \
             FROM usage_hourly_tenant_api_key \
             WHERE hour_start >= ",
        );
        builder.push_bind(start_at);
        builder.push(" AND hour_start <= ");
        builder.push_bind(end_at);
        if let Some(tenant_id) = tenant_id {
            builder.push(" AND tenant_id = ");
            builder.push_bind(tenant_id);
        }
        if let Some(api_key_id) = api_key_id {
            builder.push(" AND api_key_id = ");
            builder.push_bind(api_key_id);
        }
        builder.push(
            " GROUP BY tenant_id, api_key_id ORDER BY total_requests DESC, tenant_id ASC, api_key_id ASC LIMIT ",
        );
        builder.push_bind(i64::from(limit));

        let rows = builder
            .build()
            .fetch_all(&self.pool)
            .await
            .context("failed to query postgres api-key usage leaderboard")?;

        rows.into_iter()
            .map(|row| {
                Ok(ApiKeyUsageLeaderboardItem {
                    tenant_id: row.try_get("tenant_id")?,
                    api_key_id: row.try_get("api_key_id")?,
                    total_requests: Self::as_u64(
                        row.try_get::<i64, _>("total_requests")?,
                        "total_requests",
                    )?,
                })
            })
            .collect()
    }

    async fn query_request_logs(&self, query: RequestLogQuery) -> Result<Vec<RequestLogRow>> {
        let start_at = Self::timestamp_bound(query.start_ts, "start_ts")?;
        let end_at = Self::timestamp_bound(query.end_ts, "end_ts")?;
        let mut builder = QueryBuilder::<Postgres>::new(
            "SELECT \
                id, account_id, tenant_id, api_key_id, request_id, path, method, model, service_tier, \
                input_tokens, cached_input_tokens, output_tokens, reasoning_tokens, first_token_latency_ms, \
                status_code, latency_ms, is_stream, error_code, billing_phase, authorization_id, \
                capture_status, estimated_cost_microusd, created_at, event_version \
             FROM usage_request_logs \
             WHERE created_at >= ",
        );
        builder.push_bind(start_at);
        builder.push(" AND created_at <= ");
        builder.push_bind(end_at);

        if let Some(tenant_id) = query.tenant_id {
            builder.push(" AND tenant_id = ");
            builder.push_bind(tenant_id);
        }
        if let Some(api_key_id) = query.api_key_id {
            builder.push(" AND api_key_id = ");
            builder.push_bind(api_key_id);
        }
        if let Some(status_code) = query.status_code {
            builder.push(" AND status_code = ");
            builder.push_bind(i32::from(status_code));
        }
        if let Some(request_id) = query.request_id {
            builder.push(" AND request_id = ");
            builder.push_bind(request_id);
        }
        if let Some(keyword) = query.keyword {
            let keyword = format!("%{keyword}%");
            builder.push(" AND (path ILIKE ");
            builder.push_bind(keyword.clone());
            builder.push(" OR method ILIKE ");
            builder.push_bind(keyword.clone());
            builder.push(" OR COALESCE(request_id, '') ILIKE ");
            builder.push_bind(keyword.clone());
            builder.push(" OR COALESCE(error_code, '') ILIKE ");
            builder.push_bind(keyword.clone());
            builder.push(" OR COALESCE(model, '') ILIKE ");
            builder.push_bind(keyword);
            builder.push(")");
        }

        builder.push(" ORDER BY created_at DESC LIMIT ");
        builder.push_bind(i64::from(query.limit));

        let rows = builder
            .build()
            .fetch_all(&self.pool)
            .await
            .context("failed to query postgres request-log rows")?;

        rows.into_iter()
            .map(|row| {
                Ok(RequestLogRow {
                    id: row.try_get("id")?,
                    account_id: row.try_get("account_id")?,
                    tenant_id: row.try_get("tenant_id")?,
                    api_key_id: row.try_get("api_key_id")?,
                    request_id: row.try_get("request_id")?,
                    path: row.try_get("path")?,
                    method: row.try_get("method")?,
                    model: row.try_get("model")?,
                    service_tier: row.try_get("service_tier")?,
                    input_tokens: row.try_get("input_tokens")?,
                    cached_input_tokens: row.try_get("cached_input_tokens")?,
                    output_tokens: row.try_get("output_tokens")?,
                    reasoning_tokens: row.try_get("reasoning_tokens")?,
                    first_token_latency_ms: row
                        .try_get::<Option<i64>, _>("first_token_latency_ms")?
                        .map(|value| Self::as_u64(value, "first_token_latency_ms"))
                        .transpose()?,
                    status_code: Self::as_u16(
                        row.try_get::<i32, _>("status_code")?,
                        "status_code",
                    )?,
                    latency_ms: Self::as_u64(row.try_get::<i64, _>("latency_ms")?, "latency_ms")?,
                    is_stream: row.try_get("is_stream")?,
                    error_code: row.try_get("error_code")?,
                    billing_phase: row.try_get("billing_phase")?,
                    authorization_id: row.try_get("authorization_id")?,
                    capture_status: row.try_get("capture_status")?,
                    estimated_cost_microusd: row.try_get("estimated_cost_microusd")?,
                    created_at: row.try_get("created_at")?,
                    event_version: u16::try_from(row.try_get::<i32, _>("event_version")?)
                        .context("event_version is out of range")?,
                })
            })
            .collect()
    }
}
