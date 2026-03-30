use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Timelike, Utc};
use codex_pool_core::events::RequestLogEvent;
use sqlx_sqlite::{SqlitePool, SqliteRow};
use uuid::Uuid;

use crate::contracts::{
    AccountUsageLeaderboardItem, ApiKeyUsageLeaderboardItem, HourlyAccountUsagePoint,
    HourlyTenantApiKeyUsagePoint, HourlyTenantUsageTotalPoint, HourlyUsageTotalPoint,
    TenantUsageLeaderboardItem, UsageDashboardMetrics, UsageDashboardModelDistributionItem,
    UsageDashboardTokenBreakdown, UsageDashboardTokenTrendPoint, UsageSummaryQueryResponse,
};
use crate::cost::{calculate_estimated_cost_microusd, TokenPriceMicrousd};
use crate::tenant::{
    fetch_openai_model_catalog_items, fetch_openai_model_catalog_items_with_client,
    ModelPricingItem, ModelPricingUpsertRequest, OpenAiModelCatalogItem, OpenAiModelsSyncResponse,
    OPENAI_MODELS_INDEX_URL,
};
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
                owned_by TEXT NULL,
                title TEXT NULL,
                display_name TEXT NULL,
                tagline TEXT NULL,
                family TEXT NULL,
                family_label TEXT NULL,
                description TEXT NULL,
                avatar_remote_url TEXT NULL,
                avatar_local_path TEXT NULL,
                avatar_synced_at TEXT NULL,
                deprecated INTEGER NULL,
                context_window_tokens INTEGER NULL,
                max_input_tokens INTEGER NULL,
                max_output_tokens INTEGER NULL,
                knowledge_cutoff TEXT NULL,
                reasoning_token_support INTEGER NULL,
                input_price_microcredits INTEGER NULL,
                cached_input_price_microcredits INTEGER NULL,
                output_price_microcredits INTEGER NULL,
                pricing_notes TEXT NULL,
                pricing_note_items_json TEXT NULL,
                input_modalities_json TEXT NULL,
                output_modalities_json TEXT NULL,
                endpoints_json TEXT NULL,
                supported_features_json TEXT NULL,
                supported_tools_json TEXT NULL,
                snapshots_json TEXT NULL,
                modality_items_json TEXT NULL,
                endpoint_items_json TEXT NULL,
                feature_items_json TEXT NULL,
                tool_items_json TEXT NULL,
                snapshot_items_json TEXT NULL,
                source_url TEXT NULL,
                raw_text TEXT NULL,
                synced_at TEXT NULL
            )
            "#,
        )
        .execute(pool)
        .await
        .context("failed to create sqlite openai_models_catalog table")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS model_pricing_overrides (
                id TEXT NULL,
                model TEXT NOT NULL,
                service_tier TEXT NOT NULL DEFAULT 'default',
                input_price_microcredits INTEGER NOT NULL,
                cached_input_price_microcredits INTEGER NOT NULL,
                output_price_microcredits INTEGER NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NULL,
                updated_at TEXT NULL,
                PRIMARY KEY (model, service_tier)
            )
            "#,
        )
        .execute(pool)
        .await
        .context("failed to create sqlite model_pricing_overrides table")?;

        Self::ensure_column_exists(pool, "openai_models_catalog", "owned_by", "TEXT NULL").await?;
        Self::ensure_column_exists(pool, "openai_models_catalog", "title", "TEXT NULL").await?;
        Self::ensure_column_exists(pool, "openai_models_catalog", "display_name", "TEXT NULL")
            .await?;
        Self::ensure_column_exists(pool, "openai_models_catalog", "tagline", "TEXT NULL").await?;
        Self::ensure_column_exists(pool, "openai_models_catalog", "family", "TEXT NULL").await?;
        Self::ensure_column_exists(pool, "openai_models_catalog", "family_label", "TEXT NULL")
            .await?;
        Self::ensure_column_exists(pool, "openai_models_catalog", "description", "TEXT NULL")
            .await?;
        Self::ensure_column_exists(
            pool,
            "openai_models_catalog",
            "avatar_remote_url",
            "TEXT NULL",
        )
        .await?;
        Self::ensure_column_exists(
            pool,
            "openai_models_catalog",
            "avatar_local_path",
            "TEXT NULL",
        )
        .await?;
        Self::ensure_column_exists(
            pool,
            "openai_models_catalog",
            "avatar_synced_at",
            "TEXT NULL",
        )
        .await?;
        Self::ensure_column_exists(pool, "openai_models_catalog", "deprecated", "INTEGER NULL")
            .await?;
        Self::ensure_column_exists(
            pool,
            "openai_models_catalog",
            "context_window_tokens",
            "INTEGER NULL",
        )
        .await?;
        Self::ensure_column_exists(
            pool,
            "openai_models_catalog",
            "max_input_tokens",
            "INTEGER NULL",
        )
        .await?;
        Self::ensure_column_exists(
            pool,
            "openai_models_catalog",
            "max_output_tokens",
            "INTEGER NULL",
        )
        .await?;
        Self::ensure_column_exists(
            pool,
            "openai_models_catalog",
            "knowledge_cutoff",
            "TEXT NULL",
        )
        .await?;
        Self::ensure_column_exists(
            pool,
            "openai_models_catalog",
            "reasoning_token_support",
            "INTEGER NULL",
        )
        .await?;
        Self::ensure_column_exists(pool, "openai_models_catalog", "pricing_notes", "TEXT NULL")
            .await?;
        Self::ensure_column_exists(
            pool,
            "openai_models_catalog",
            "pricing_note_items_json",
            "TEXT NULL",
        )
        .await?;
        Self::ensure_column_exists(
            pool,
            "openai_models_catalog",
            "input_modalities_json",
            "TEXT NULL",
        )
        .await?;
        Self::ensure_column_exists(
            pool,
            "openai_models_catalog",
            "output_modalities_json",
            "TEXT NULL",
        )
        .await?;
        Self::ensure_column_exists(pool, "openai_models_catalog", "endpoints_json", "TEXT NULL")
            .await?;
        Self::ensure_column_exists(
            pool,
            "openai_models_catalog",
            "supported_features_json",
            "TEXT NULL",
        )
        .await?;
        Self::ensure_column_exists(
            pool,
            "openai_models_catalog",
            "supported_tools_json",
            "TEXT NULL",
        )
        .await?;
        Self::ensure_column_exists(pool, "openai_models_catalog", "snapshots_json", "TEXT NULL")
            .await?;
        Self::ensure_column_exists(
            pool,
            "openai_models_catalog",
            "modality_items_json",
            "TEXT NULL",
        )
        .await?;
        Self::ensure_column_exists(
            pool,
            "openai_models_catalog",
            "endpoint_items_json",
            "TEXT NULL",
        )
        .await?;
        Self::ensure_column_exists(
            pool,
            "openai_models_catalog",
            "feature_items_json",
            "TEXT NULL",
        )
        .await?;
        Self::ensure_column_exists(
            pool,
            "openai_models_catalog",
            "tool_items_json",
            "TEXT NULL",
        )
        .await?;
        Self::ensure_column_exists(
            pool,
            "openai_models_catalog",
            "snapshot_items_json",
            "TEXT NULL",
        )
        .await?;
        Self::ensure_column_exists(pool, "openai_models_catalog", "source_url", "TEXT NULL")
            .await?;
        Self::ensure_column_exists(pool, "openai_models_catalog", "raw_text", "TEXT NULL").await?;
        Self::ensure_column_exists(pool, "openai_models_catalog", "synced_at", "TEXT NULL").await?;
        Self::ensure_column_exists(pool, "model_pricing_overrides", "id", "TEXT NULL").await?;
        Self::ensure_column_exists(pool, "model_pricing_overrides", "created_at", "TEXT NULL")
            .await?;
        Self::ensure_column_exists(pool, "model_pricing_overrides", "updated_at", "TEXT NULL")
            .await?;
        Self::backfill_openai_catalog_metadata(pool).await?;
        Self::backfill_model_pricing_metadata(pool).await?;

        sqlx::query(
            r#"
            CREATE UNIQUE INDEX IF NOT EXISTS idx_model_pricing_overrides_id
            ON model_pricing_overrides (id)
            "#,
        )
        .execute(pool)
        .await
        .context("failed to create sqlite model_pricing_overrides id index")?;

        Ok(())
    }

    async fn ensure_column_exists(
        pool: &SqlitePool,
        table: &str,
        column: &str,
        definition: &str,
    ) -> Result<()> {
        let pragma = format!("PRAGMA table_info({table})");
        let columns = sqlx::query(&pragma)
            .fetch_all(pool)
            .await
            .with_context(|| format!("failed to read sqlite schema for {table}"))?;
        let exists = columns.into_iter().any(|row| {
            row.try_get::<String, _>("name")
                .map(|name| name == column)
                .unwrap_or(false)
        });
        if exists {
            return Ok(());
        }

        let alter = format!("ALTER TABLE {table} ADD COLUMN {column} {definition}");
        sqlx::query(&alter)
            .execute(pool)
            .await
            .with_context(|| format!("failed to add {table}.{column} column"))?;
        Ok(())
    }

    async fn backfill_openai_catalog_metadata(pool: &SqlitePool) -> Result<()> {
        let rows = sqlx::query(
            r#"
            SELECT model_id, title, owned_by, source_url, synced_at
            FROM openai_models_catalog
            "#,
        )
        .fetch_all(pool)
        .await
        .context("failed to query sqlite openai catalog metadata for backfill")?;

        for row in rows {
            let model_id: String = row.try_get("model_id")?;
            let title: Option<String> = row.try_get("title")?;
            let owned_by: Option<String> = row.try_get("owned_by")?;
            let source_url: Option<String> = row.try_get("source_url")?;
            let synced_at: Option<String> = row.try_get("synced_at")?;

            sqlx::query(
                r#"
                UPDATE openai_models_catalog
                SET
                    title = COALESCE(title, ?),
                    owned_by = COALESCE(owned_by, 'openai'),
                    source_url = COALESCE(source_url, ?),
                    synced_at = COALESCE(synced_at, ?)
                WHERE model_id = ?
                "#,
            )
            .bind(title.unwrap_or_else(|| model_id.clone()))
            .bind(format!("{OPENAI_MODELS_INDEX_URL}/{model_id}"))
            .bind(synced_at.unwrap_or_else(|| Utc::now().to_rfc3339()))
            .bind(model_id)
            .execute(pool)
            .await
            .context("failed to backfill sqlite openai catalog metadata")?;

            if owned_by.is_none() || source_url.is_none() {
                continue;
            }
        }

        Ok(())
    }

    async fn backfill_model_pricing_metadata(pool: &SqlitePool) -> Result<()> {
        let rows = sqlx::query(
            r#"
            SELECT model, service_tier, id, created_at, updated_at
            FROM model_pricing_overrides
            "#,
        )
        .fetch_all(pool)
        .await
        .context("failed to query sqlite model_pricing_overrides metadata for backfill")?;

        for row in rows {
            let model: String = row.try_get("model")?;
            let service_tier: String = row.try_get("service_tier")?;
            let id: Option<String> = row.try_get("id")?;
            let created_at: Option<String> = row.try_get("created_at")?;
            let updated_at: Option<String> = row.try_get("updated_at")?;
            let now = Utc::now().to_rfc3339();

            sqlx::query(
                r#"
                UPDATE model_pricing_overrides
                SET
                    id = COALESCE(id, ?),
                    created_at = COALESCE(created_at, ?),
                    updated_at = COALESCE(updated_at, ?)
                WHERE model = ? AND service_tier = ?
                "#,
            )
            .bind(id.unwrap_or_else(|| Uuid::new_v4().to_string()))
            .bind(created_at.unwrap_or_else(|| now.clone()))
            .bind(updated_at.unwrap_or(now))
            .bind(model)
            .bind(service_tier)
            .execute(pool)
            .await
            .context("failed to backfill sqlite model pricing metadata")?;
        }

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

    pub async fn list_openai_model_catalog(&self) -> Result<Vec<OpenAiModelCatalogItem>> {
        let rows = sqlx::query(
            r#"
            SELECT
                model_id,
                owned_by,
                title,
                display_name,
                tagline,
                family,
                family_label,
                description,
                avatar_remote_url,
                avatar_local_path,
                avatar_synced_at,
                deprecated,
                context_window_tokens,
                max_input_tokens,
                max_output_tokens,
                knowledge_cutoff,
                reasoning_token_support,
                input_price_microcredits,
                cached_input_price_microcredits,
                output_price_microcredits,
                pricing_notes,
                pricing_note_items_json,
                input_modalities_json,
                output_modalities_json,
                endpoints_json,
                supported_features_json,
                supported_tools_json,
                snapshots_json,
                modality_items_json,
                endpoint_items_json,
                feature_items_json,
                tool_items_json,
                snapshot_items_json,
                source_url,
                raw_text,
                synced_at
            FROM openai_models_catalog
            ORDER BY model_id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list sqlite openai models catalog")?;

        rows.into_iter()
            .map(|row| -> Result<OpenAiModelCatalogItem> {
                Ok(OpenAiModelCatalogItem {
                    model_id: row.try_get("model_id")?,
                    owned_by: row
                        .try_get::<Option<String>, _>("owned_by")?
                        .unwrap_or_else(|| "openai".to_string()),
                    title: row
                        .try_get::<Option<String>, _>("title")?
                        .unwrap_or_else(|| row.try_get("model_id").unwrap_or_default()),
                    display_name: row.try_get("display_name")?,
                    tagline: row.try_get("tagline")?,
                    family: row.try_get("family")?,
                    family_label: row.try_get("family_label")?,
                    description: row.try_get("description")?,
                    avatar_remote_url: row.try_get("avatar_remote_url")?,
                    avatar_local_path: row.try_get("avatar_local_path")?,
                    avatar_synced_at: row
                        .try_get::<Option<String>, _>("avatar_synced_at")?
                        .as_deref()
                        .map(Self::parse_created_at)
                        .transpose()?,
                    deprecated: row.try_get("deprecated")?,
                    context_window_tokens: row.try_get("context_window_tokens")?,
                    max_input_tokens: row.try_get("max_input_tokens")?,
                    max_output_tokens: row.try_get("max_output_tokens")?,
                    knowledge_cutoff: row.try_get("knowledge_cutoff")?,
                    reasoning_token_support: row.try_get("reasoning_token_support")?,
                    input_price_microcredits: row.try_get("input_price_microcredits")?,
                    cached_input_price_microcredits: row
                        .try_get("cached_input_price_microcredits")?,
                    output_price_microcredits: row.try_get("output_price_microcredits")?,
                    pricing_notes: row.try_get("pricing_notes")?,
                    pricing_note_items: serde_json::from_str(
                        row.try_get::<Option<String>, _>("pricing_note_items_json")?
                            .as_deref()
                            .unwrap_or("[]"),
                    )
                    .context("failed to decode sqlite pricing note items json")?,
                    input_modalities: serde_json::from_str(
                        row.try_get::<Option<String>, _>("input_modalities_json")?
                            .as_deref()
                            .unwrap_or("[]"),
                    )
                    .context("failed to decode sqlite input modalities json")?,
                    output_modalities: serde_json::from_str(
                        row.try_get::<Option<String>, _>("output_modalities_json")?
                            .as_deref()
                            .unwrap_or("[]"),
                    )
                    .context("failed to decode sqlite output modalities json")?,
                    endpoints: serde_json::from_str(
                        row.try_get::<Option<String>, _>("endpoints_json")?
                            .as_deref()
                            .unwrap_or("[]"),
                    )
                    .context("failed to decode sqlite endpoints json")?,
                    supported_features: serde_json::from_str(
                        row.try_get::<Option<String>, _>("supported_features_json")?
                            .as_deref()
                            .unwrap_or("[]"),
                    )
                    .context("failed to decode sqlite supported features json")?,
                    supported_tools: serde_json::from_str(
                        row.try_get::<Option<String>, _>("supported_tools_json")?
                            .as_deref()
                            .unwrap_or("[]"),
                    )
                    .context("failed to decode sqlite supported tools json")?,
                    snapshots: serde_json::from_str(
                        row.try_get::<Option<String>, _>("snapshots_json")?
                            .as_deref()
                            .unwrap_or("[]"),
                    )
                    .context("failed to decode sqlite snapshots json")?,
                    modality_items: serde_json::from_str(
                        row.try_get::<Option<String>, _>("modality_items_json")?
                            .as_deref()
                            .unwrap_or("[]"),
                    )
                    .context("failed to decode sqlite modality items json")?,
                    endpoint_items: serde_json::from_str(
                        row.try_get::<Option<String>, _>("endpoint_items_json")?
                            .as_deref()
                            .unwrap_or("[]"),
                    )
                    .context("failed to decode sqlite endpoint items json")?,
                    feature_items: serde_json::from_str(
                        row.try_get::<Option<String>, _>("feature_items_json")?
                            .as_deref()
                            .unwrap_or("[]"),
                    )
                    .context("failed to decode sqlite feature items json")?,
                    tool_items: serde_json::from_str(
                        row.try_get::<Option<String>, _>("tool_items_json")?
                            .as_deref()
                            .unwrap_or("[]"),
                    )
                    .context("failed to decode sqlite tool items json")?,
                    snapshot_items: serde_json::from_str(
                        row.try_get::<Option<String>, _>("snapshot_items_json")?
                            .as_deref()
                            .unwrap_or("[]"),
                    )
                    .context("failed to decode sqlite snapshot items json")?,
                    source_url: row
                        .try_get::<Option<String>, _>("source_url")?
                        .unwrap_or_else(|| {
                            format!(
                                "{OPENAI_MODELS_INDEX_URL}/{}",
                                row.try_get::<String, _>("model_id").unwrap_or_default()
                            )
                        }),
                    raw_text: row.try_get("raw_text")?,
                    synced_at: row
                        .try_get::<Option<String>, _>("synced_at")?
                        .as_deref()
                        .map(Self::parse_created_at)
                        .transpose()?
                        .unwrap_or_else(Utc::now),
                })
            })
            .collect()
    }

    pub async fn apply_openai_model_catalog_items(
        &self,
        synced_at: DateTime<Utc>,
        items: &[OpenAiModelCatalogItem],
    ) -> Result<OpenAiModelsSyncResponse> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start sqlite openai catalog sync transaction")?;

        for item in items {
            sqlx::query(
                r#"
                INSERT INTO openai_models_catalog (
                    model_id,
                    owned_by,
                    title,
                    display_name,
                    tagline,
                    family,
                    family_label,
                    description,
                    avatar_remote_url,
                    avatar_local_path,
                    avatar_synced_at,
                    deprecated,
                    context_window_tokens,
                    max_input_tokens,
                    max_output_tokens,
                    knowledge_cutoff,
                    reasoning_token_support,
                    input_price_microcredits,
                    cached_input_price_microcredits,
                    output_price_microcredits,
                    pricing_notes,
                    pricing_note_items_json,
                    input_modalities_json,
                    output_modalities_json,
                    endpoints_json,
                    supported_features_json,
                    supported_tools_json,
                    snapshots_json,
                    modality_items_json,
                    endpoint_items_json,
                    feature_items_json,
                    tool_items_json,
                    snapshot_items_json,
                    source_url,
                    raw_text,
                    synced_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(model_id) DO UPDATE SET
                    owned_by = excluded.owned_by,
                    title = excluded.title,
                    display_name = excluded.display_name,
                    tagline = excluded.tagline,
                    family = excluded.family,
                    family_label = excluded.family_label,
                    description = excluded.description,
                    avatar_remote_url = excluded.avatar_remote_url,
                    avatar_local_path = excluded.avatar_local_path,
                    avatar_synced_at = excluded.avatar_synced_at,
                    deprecated = excluded.deprecated,
                    context_window_tokens = excluded.context_window_tokens,
                    max_input_tokens = excluded.max_input_tokens,
                    max_output_tokens = excluded.max_output_tokens,
                    knowledge_cutoff = excluded.knowledge_cutoff,
                    reasoning_token_support = excluded.reasoning_token_support,
                    input_price_microcredits = excluded.input_price_microcredits,
                    cached_input_price_microcredits = excluded.cached_input_price_microcredits,
                    output_price_microcredits = excluded.output_price_microcredits,
                    pricing_notes = excluded.pricing_notes,
                    pricing_note_items_json = excluded.pricing_note_items_json,
                    input_modalities_json = excluded.input_modalities_json,
                    output_modalities_json = excluded.output_modalities_json,
                    endpoints_json = excluded.endpoints_json,
                    supported_features_json = excluded.supported_features_json,
                    supported_tools_json = excluded.supported_tools_json,
                    snapshots_json = excluded.snapshots_json,
                    modality_items_json = excluded.modality_items_json,
                    endpoint_items_json = excluded.endpoint_items_json,
                    feature_items_json = excluded.feature_items_json,
                    tool_items_json = excluded.tool_items_json,
                    snapshot_items_json = excluded.snapshot_items_json,
                    source_url = excluded.source_url,
                    raw_text = excluded.raw_text,
                    synced_at = excluded.synced_at
                "#,
            )
            .bind(&item.model_id)
            .bind(&item.owned_by)
            .bind(&item.title)
            .bind(&item.display_name)
            .bind(&item.tagline)
            .bind(&item.family)
            .bind(&item.family_label)
            .bind(&item.description)
            .bind(&item.avatar_remote_url)
            .bind(&item.avatar_local_path)
            .bind(item.avatar_synced_at.map(|value| value.to_rfc3339()))
            .bind(item.deprecated)
            .bind(item.context_window_tokens)
            .bind(item.max_input_tokens)
            .bind(item.max_output_tokens)
            .bind(&item.knowledge_cutoff)
            .bind(item.reasoning_token_support)
            .bind(item.input_price_microcredits)
            .bind(item.cached_input_price_microcredits)
            .bind(item.output_price_microcredits)
            .bind(&item.pricing_notes)
            .bind(serde_json::to_string(&item.pricing_note_items)?)
            .bind(serde_json::to_string(&item.input_modalities)?)
            .bind(serde_json::to_string(&item.output_modalities)?)
            .bind(serde_json::to_string(&item.endpoints)?)
            .bind(serde_json::to_string(&item.supported_features)?)
            .bind(serde_json::to_string(&item.supported_tools)?)
            .bind(serde_json::to_string(&item.snapshots)?)
            .bind(serde_json::to_string(&item.modality_items)?)
            .bind(serde_json::to_string(&item.endpoint_items)?)
            .bind(serde_json::to_string(&item.feature_items)?)
            .bind(serde_json::to_string(&item.tool_items)?)
            .bind(serde_json::to_string(&item.snapshot_items)?)
            .bind(&item.source_url)
            .bind(&item.raw_text)
            .bind(item.synced_at.to_rfc3339())
            .execute(tx.as_mut())
            .await
            .with_context(|| format!("failed to upsert sqlite official model {}", item.model_id))?;
        }

        let deleted_catalog_rows = if items.is_empty() {
            sqlx::query("DELETE FROM openai_models_catalog")
                .execute(tx.as_mut())
                .await
                .context("failed to clear sqlite openai catalog")?
                .rows_affected() as usize
        } else {
            let placeholders = std::iter::repeat_n("?", items.len())
                .collect::<Vec<_>>()
                .join(", ");
            let delete_sql =
                format!("DELETE FROM openai_models_catalog WHERE model_id NOT IN ({placeholders})");
            let mut query = sqlx::query(&delete_sql);
            for item in items {
                query = query.bind(&item.model_id);
            }
            query
                .execute(tx.as_mut())
                .await
                .context("failed to delete removed sqlite openai catalog rows")?
                .rows_affected() as usize
        };

        tx.commit()
            .await
            .context("failed to commit sqlite openai catalog sync transaction")?;

        Ok(OpenAiModelsSyncResponse {
            models_total: items.len(),
            created_or_updated: items.len(),
            deleted_catalog_rows,
            cleared_custom_entities: 0,
            cleared_billing_rules: 0,
            deleted_legacy_pricing_rows: 0,
            synced_at,
        })
    }

    pub async fn sync_openai_model_catalog(&self) -> Result<OpenAiModelsSyncResponse> {
        self.sync_openai_model_catalog_with_client(None).await
    }

    pub async fn sync_openai_model_catalog_with_client(
        &self,
        client: Option<reqwest::Client>,
    ) -> Result<OpenAiModelsSyncResponse> {
        let (synced_at, items) = match client {
            Some(client) => fetch_openai_model_catalog_items_with_client(client).await?,
            None => fetch_openai_model_catalog_items().await?,
        };
        self.apply_openai_model_catalog_items(synced_at, &items)
            .await
    }

    pub async fn list_model_pricing(&self) -> Result<Vec<ModelPricingItem>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id,
                model,
                service_tier,
                input_price_microcredits,
                cached_input_price_microcredits,
                output_price_microcredits,
                enabled,
                created_at,
                updated_at
            FROM model_pricing_overrides
            ORDER BY model ASC, service_tier ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list sqlite model pricing overrides")?;

        rows.into_iter()
            .map(|row| -> Result<ModelPricingItem> {
                Ok(ModelPricingItem {
                    id: Uuid::parse_str(&row.try_get::<String, _>("id")?)
                        .context("failed to parse sqlite model pricing id")?,
                    model: row.try_get("model")?,
                    service_tier: row.try_get("service_tier")?,
                    input_price_microcredits: row.try_get("input_price_microcredits")?,
                    cached_input_price_microcredits: row
                        .try_get("cached_input_price_microcredits")?,
                    output_price_microcredits: row.try_get("output_price_microcredits")?,
                    enabled: row.try_get("enabled")?,
                    created_at: Self::parse_created_at(&row.try_get::<String, _>("created_at")?)?,
                    updated_at: Self::parse_created_at(&row.try_get::<String, _>("updated_at")?)?,
                })
            })
            .collect()
    }

    pub async fn upsert_model_pricing(
        &self,
        req: ModelPricingUpsertRequest,
    ) -> Result<ModelPricingItem> {
        let model = req.model.trim();
        if model.is_empty() {
            anyhow::bail!("model must not be empty");
        }
        let exists = sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(*) FROM openai_models_catalog WHERE model_id = ?"#,
        )
        .bind(model)
        .fetch_one(&self.pool)
        .await
        .context("failed to validate sqlite official model id for price override")?;
        if exists == 0 {
            anyhow::bail!("model must exist in official catalog before creating an override");
        }

        let service_tier = Self::normalize_service_tier(Some(req.service_tier.as_str()));
        let cached_input_price_microcredits = req
            .cached_input_price_microcredits
            .unwrap_or_else(|| (req.input_price_microcredits / 10).max(0));
        if req.input_price_microcredits < 0
            || cached_input_price_microcredits < 0
            || req.output_price_microcredits < 0
        {
            anyhow::bail!(
                "input_price_microcredits/cached_input_price_microcredits/output_price_microcredits must be >= 0"
            );
        }

        let existing = sqlx::query(
            r#"
            SELECT id, created_at
            FROM model_pricing_overrides
            WHERE model = ? AND service_tier = ?
            "#,
        )
        .bind(model)
        .bind(&service_tier)
        .fetch_optional(&self.pool)
        .await
        .context("failed to query sqlite model pricing override before upsert")?;
        let now = Utc::now();
        let id = existing
            .as_ref()
            .and_then(|row| row.try_get::<Option<String>, _>("id").ok().flatten())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let created_at = existing
            .as_ref()
            .and_then(|row| {
                row.try_get::<Option<String>, _>("created_at")
                    .ok()
                    .flatten()
            })
            .unwrap_or_else(|| now.to_rfc3339());
        let updated_at = now.to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO model_pricing_overrides (
                id,
                model,
                service_tier,
                input_price_microcredits,
                cached_input_price_microcredits,
                output_price_microcredits,
                enabled,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(model, service_tier) DO UPDATE SET
                id = excluded.id,
                input_price_microcredits = excluded.input_price_microcredits,
                cached_input_price_microcredits = excluded.cached_input_price_microcredits,
                output_price_microcredits = excluded.output_price_microcredits,
                enabled = excluded.enabled,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&id)
        .bind(model)
        .bind(&service_tier)
        .bind(req.input_price_microcredits)
        .bind(cached_input_price_microcredits)
        .bind(req.output_price_microcredits)
        .bind(req.enabled)
        .bind(&created_at)
        .bind(&updated_at)
        .execute(&self.pool)
        .await
        .context("failed to upsert sqlite model pricing override")?;

        Ok(ModelPricingItem {
            id: Uuid::parse_str(&id).context("failed to parse sqlite model pricing id")?,
            model: model.to_string(),
            service_tier,
            input_price_microcredits: req.input_price_microcredits,
            cached_input_price_microcredits,
            output_price_microcredits: req.output_price_microcredits,
            enabled: req.enabled,
            created_at: Self::parse_created_at(&created_at)?,
            updated_at: Self::parse_created_at(&updated_at)?,
        })
    }

    pub async fn delete_model_pricing(&self, pricing_id: Uuid) -> Result<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM model_pricing_overrides
            WHERE id = ?
            "#,
        )
        .bind(pricing_id.to_string())
        .execute(&self.pool)
        .await
        .context("failed to delete sqlite model pricing override")?;
        if result.rows_affected() == 0 {
            anyhow::bail!("model pricing not found");
        }
        Ok(())
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

    #[tokio::test]
    async fn sqlite_usage_repo_supports_admin_models_catalog_and_pricing_overrides() {
        let repo = build_repo("cp-admin-models").await;
        let synced_at = Utc::now();
        repo.apply_openai_model_catalog_items(
            synced_at,
            &[crate::tenant::OpenAiModelCatalogItem {
                model_id: "gpt-5.4".to_string(),
                owned_by: "openai".to_string(),
                title: "GPT-5.4".to_string(),
                display_name: Some("GPT-5.4".to_string()),
                tagline: Some("Latest reasoning flagship".to_string()),
                family: Some("frontier".to_string()),
                family_label: Some("Frontier models".to_string()),
                description: Some("Latest reasoning model".to_string()),
                avatar_remote_url: Some(
                    "https://developers.openai.com/images/api/models/icons/gpt-5.4.png".to_string(),
                ),
                avatar_local_path: Some("gpt-5.4.png".to_string()),
                avatar_synced_at: Some(synced_at),
                deprecated: Some(false),
                context_window_tokens: Some(400_000),
                max_input_tokens: Some(272_000),
                max_output_tokens: Some(128_000),
                knowledge_cutoff: Some("Mar 1, 2025".to_string()),
                reasoning_token_support: Some(true),
                input_price_microcredits: Some(1_250_000),
                cached_input_price_microcredits: Some(125_000),
                output_price_microcredits: Some(10_000_000),
                pricing_notes: None,
                pricing_note_items: vec!["Pricing note".to_string()],
                input_modalities: vec!["text".to_string()],
                output_modalities: vec!["text".to_string()],
                endpoints: vec!["v1/responses".to_string()],
                supported_features: vec!["streaming".to_string()],
                supported_tools: vec!["web_search".to_string()],
                snapshots: vec!["gpt-5.4-2026-03-05".to_string()],
                modality_items: vec![crate::tenant::OpenAiModelSectionItem {
                    key: "text".to_string(),
                    label: "Text".to_string(),
                    detail: Some("Input and output".to_string()),
                    status: Some("input_output".to_string()),
                    icon_svg: None,
                }],
                endpoint_items: vec![crate::tenant::OpenAiModelSectionItem {
                    key: "responses".to_string(),
                    label: "Responses".to_string(),
                    detail: Some("v1/responses".to_string()),
                    status: Some("supported".to_string()),
                    icon_svg: None,
                }],
                feature_items: vec![crate::tenant::OpenAiModelSectionItem {
                    key: "streaming".to_string(),
                    label: "Streaming".to_string(),
                    detail: Some("Supported".to_string()),
                    status: Some("supported".to_string()),
                    icon_svg: None,
                }],
                tool_items: vec![crate::tenant::OpenAiModelSectionItem {
                    key: "web_search".to_string(),
                    label: "Web search".to_string(),
                    detail: Some("Supported".to_string()),
                    status: Some("supported".to_string()),
                    icon_svg: None,
                }],
                snapshot_items: vec![crate::tenant::OpenAiModelSnapshotItem {
                    alias: "gpt-5.4".to_string(),
                    label: "GPT-5.4".to_string(),
                    latest_snapshot: Some("gpt-5.4-2026-03-05".to_string()),
                    versions: vec!["gpt-5.4-2026-03-05".to_string()],
                }],
                source_url: "https://developers.openai.com/api/docs/models/gpt-5.4".to_string(),
                raw_text: Some("model page".to_string()),
                synced_at,
            }],
        )
        .await
        .expect("apply sqlite admin models catalog");

        let catalog = repo
            .list_openai_model_catalog()
            .await
            .expect("list sqlite admin models catalog");
        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog[0].model_id, "gpt-5.4");
        assert_eq!(catalog[0].title, "GPT-5.4");
        assert_eq!(catalog[0].display_name.as_deref(), Some("GPT-5.4"));
        assert_eq!(
            catalog[0].tagline.as_deref(),
            Some("Latest reasoning flagship")
        );
        assert_eq!(catalog[0].family.as_deref(), Some("frontier"));
        assert_eq!(catalog[0].family_label.as_deref(), Some("Frontier models"));
        assert_eq!(catalog[0].avatar_local_path.as_deref(), Some("gpt-5.4.png"));
        assert_eq!(catalog[0].max_input_tokens, Some(272_000));
        assert_eq!(
            catalog[0].pricing_note_items,
            vec!["Pricing note".to_string()]
        );
        assert_eq!(catalog[0].endpoints, vec!["v1/responses".to_string()]);
        assert_eq!(catalog[0].supported_features, vec!["streaming".to_string()]);
        assert_eq!(catalog[0].supported_tools, vec!["web_search".to_string()]);
        assert_eq!(catalog[0].modality_items.len(), 1);
        assert_eq!(catalog[0].endpoint_items.len(), 1);
        assert_eq!(catalog[0].feature_items.len(), 1);
        assert_eq!(catalog[0].tool_items.len(), 1);
        assert_eq!(catalog[0].snapshot_items.len(), 1);
        assert_eq!(catalog[0].snapshots, vec!["gpt-5.4-2026-03-05".to_string()]);

        let pricing = repo
            .upsert_model_pricing(crate::tenant::ModelPricingUpsertRequest {
                model: "gpt-5.4".to_string(),
                service_tier: "default".to_string(),
                input_price_microcredits: 2_000_000,
                cached_input_price_microcredits: Some(200_000),
                output_price_microcredits: 8_000_000,
                enabled: true,
            })
            .await
            .expect("upsert sqlite model pricing");
        let all_pricing = repo
            .list_model_pricing()
            .await
            .expect("list sqlite model pricing");

        assert_eq!(all_pricing.len(), 1);
        assert_eq!(all_pricing[0].id, pricing.id);
        assert_eq!(all_pricing[0].model, "gpt-5.4");
        assert_eq!(all_pricing[0].service_tier, "default");
        assert_eq!(all_pricing[0].output_price_microcredits, 8_000_000);
    }
}
