impl ClickHouseUsageRepo {
    async fn fetch_hourly_account_rows(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        account_id: Option<Uuid>,
    ) -> Result<Vec<HourlyAccountUsagePoint>> {
        let mut sql = format!(
            "SELECT account_id, hour_start, request_count FROM {} WHERE hour_start >= ? AND hour_start < ?",
            self.account_table
        );

        if account_id.is_some() {
            sql.push_str(" AND account_id = ?");
        }

        sql.push_str(" ORDER BY hour_start ASC, account_id ASC LIMIT ?");

        let mut query = self.ch_client.query(&sql).bind(start_ts).bind(end_ts);

        if let Some(account_id) = account_id {
            query = query.bind(account_id.to_string());
        }

        let rows = query
            .bind(limit as u64)
            .fetch_all::<ClickHouseHourlyAccountUsageRow>()
            .await
            .context("failed to query clickhouse account usage rows")?;

        rows.into_iter()
            .map(HourlyAccountUsagePoint::try_from)
            .collect::<Result<Vec<_>>>()
            .context("failed to decode clickhouse account usage rows")
    }

    async fn fetch_hourly_tenant_api_key_rows(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        tenant_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<Vec<HourlyTenantApiKeyUsagePoint>> {
        let mut sql = format!(
            "SELECT tenant_id, api_key_id, hour_start, request_count FROM {} WHERE hour_start >= ? AND hour_start < ?",
            self.tenant_api_key_table
        );

        if tenant_id.is_some() {
            sql.push_str(" AND tenant_id = ?");
        }

        if api_key_id.is_some() {
            sql.push_str(" AND api_key_id = ?");
        }

        sql.push_str(" ORDER BY hour_start ASC, tenant_id ASC, api_key_id ASC LIMIT ?");

        let mut query = self.ch_client.query(&sql).bind(start_ts).bind(end_ts);

        if let Some(tenant_id) = tenant_id {
            query = query.bind(tenant_id.to_string());
        }

        if let Some(api_key_id) = api_key_id {
            query = query.bind(api_key_id.to_string());
        }

        let rows = query
            .bind(limit as u64)
            .fetch_all::<ClickHouseHourlyTenantApiKeyUsageRow>()
            .await
            .context("failed to query clickhouse tenant api-key usage rows")?;

        rows.into_iter()
            .map(HourlyTenantApiKeyUsagePoint::try_from)
            .collect::<Result<Vec<_>>>()
            .context("failed to decode clickhouse tenant api-key usage rows")
    }

    async fn fetch_hourly_account_totals(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        account_id: Option<Uuid>,
    ) -> Result<Vec<HourlyUsageTotalPoint>> {
        let mut sql = format!(
            "SELECT hour_start, toUInt64(ifNull(sum(request_count), 0)) AS request_count FROM {} WHERE hour_start >= ? AND hour_start < ?",
            self.account_table
        );

        if account_id.is_some() {
            sql.push_str(" AND account_id = ?");
        }

        sql.push_str(" GROUP BY hour_start ORDER BY hour_start ASC LIMIT ?");

        let mut query = self.ch_client.query(&sql).bind(start_ts).bind(end_ts);

        if let Some(account_id) = account_id {
            query = query.bind(account_id.to_string());
        }

        let rows = query
            .bind(limit as u64)
            .fetch_all::<ClickHouseHourlyUsageTotalRow>()
            .await
            .context("failed to query clickhouse hourly account usage totals")?;

        Ok(rows
            .into_iter()
            .map(|row| HourlyUsageTotalPoint {
                hour_start: row.hour_start,
                request_count: row.request_count,
            })
            .collect())
    }

    async fn fetch_hourly_tenant_api_key_totals(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        tenant_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<Vec<HourlyUsageTotalPoint>> {
        let mut sql = format!(
            "SELECT hour_start, toUInt64(ifNull(sum(request_count), 0)) AS request_count FROM {} WHERE hour_start >= ? AND hour_start < ?",
            self.tenant_api_key_table
        );

        if tenant_id.is_some() {
            sql.push_str(" AND tenant_id = ?");
        }

        if api_key_id.is_some() {
            sql.push_str(" AND api_key_id = ?");
        }

        sql.push_str(" GROUP BY hour_start ORDER BY hour_start ASC LIMIT ?");

        let mut query = self.ch_client.query(&sql).bind(start_ts).bind(end_ts);

        if let Some(tenant_id) = tenant_id {
            query = query.bind(tenant_id.to_string());
        }

        if let Some(api_key_id) = api_key_id {
            query = query.bind(api_key_id.to_string());
        }

        let rows = query
            .bind(limit as u64)
            .fetch_all::<ClickHouseHourlyUsageTotalRow>()
            .await
            .context("failed to query clickhouse hourly tenant api-key usage totals")?;

        Ok(rows
            .into_iter()
            .map(|row| HourlyUsageTotalPoint {
                hour_start: row.hour_start,
                request_count: row.request_count,
            })
            .collect())
    }

    async fn fetch_hourly_tenant_totals(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        tenant_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<Vec<HourlyTenantUsageTotalPoint>> {
        let mut sql = format!(
            "SELECT tenant_id, hour_start, toUInt64(ifNull(sum(request_count), 0)) AS request_count FROM {} WHERE hour_start >= ? AND hour_start < ?",
            self.tenant_api_key_table
        );

        if tenant_id.is_some() {
            sql.push_str(" AND tenant_id = ?");
        }

        if api_key_id.is_some() {
            sql.push_str(" AND api_key_id = ?");
        }

        sql.push_str(
            " GROUP BY tenant_id, hour_start ORDER BY hour_start ASC, tenant_id ASC LIMIT ?",
        );

        let mut query = self.ch_client.query(&sql).bind(start_ts).bind(end_ts);

        if let Some(tenant_id) = tenant_id {
            query = query.bind(tenant_id.to_string());
        }

        if let Some(api_key_id) = api_key_id {
            query = query.bind(api_key_id.to_string());
        }

        let rows = query
            .bind(limit as u64)
            .fetch_all::<ClickHouseHourlyTenantUsageTotalRow>()
            .await
            .context("failed to query clickhouse hourly tenant usage totals")?;

        rows.into_iter()
            .map(|row| {
                let tenant_id = Uuid::parse_str(&row.tenant_id).with_context(|| {
                    format!("invalid tenant_id in clickhouse row: {}", row.tenant_id)
                })?;

                Ok(HourlyTenantUsageTotalPoint {
                    tenant_id,
                    hour_start: row.hour_start,
                    request_count: row.request_count,
                })
            })
            .collect()
    }

    async fn fetch_dashboard_summary_row(
        &self,
        start_ts: i64,
        end_ts: i64,
        tenant_id: Option<Uuid>,
        account_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<ClickHouseDashboardSummaryRow> {
        let mut sql = format!(
            "SELECT toUInt64(count()) AS total_requests, toUInt64(ifNull(sum(ifNull(input_tokens, 0)), 0)) AS input_tokens, toUInt64(ifNull(sum(ifNull(cached_input_tokens, 0)), 0)) AS cached_input_tokens, toUInt64(ifNull(sum(ifNull(output_tokens, 0)), 0)) AS output_tokens, toUInt64(ifNull(sum(ifNull(reasoning_tokens, 0)), 0)) AS reasoning_tokens, if(countIf(first_token_latency_ms IS NOT NULL) = 0, NULL, toUInt64(round(avgIf(toFloat64(first_token_latency_ms), first_token_latency_ms IS NOT NULL), 0))) AS avg_first_token_latency_ms FROM {} WHERE created_at >= ? AND created_at <= ? AND (billing_phase IS NULL OR billing_phase != 'streaming_open')",
            self.request_log_table
        );
        if tenant_id.is_some() {
            sql.push_str(" AND tenant_id = ?");
        }
        if account_id.is_some() {
            sql.push_str(" AND account_id = ?");
        }
        if api_key_id.is_some() {
            sql.push_str(" AND api_key_id = ?");
        }

        let mut query = self.ch_client.query(&sql).bind(start_ts).bind(end_ts);
        if let Some(tenant_id) = tenant_id {
            query = query.bind(tenant_id.to_string());
        }
        if let Some(account_id) = account_id {
            query = query.bind(account_id.to_string());
        }
        if let Some(api_key_id) = api_key_id {
            query = query.bind(api_key_id.to_string());
        }

        query
            .fetch_one::<ClickHouseDashboardSummaryRow>()
            .await
            .context("failed to query clickhouse dashboard summary")
    }

    async fn fetch_dashboard_token_trends(
        &self,
        start_ts: i64,
        end_ts: i64,
        tenant_id: Option<Uuid>,
        account_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<Vec<UsageDashboardTokenTrendPoint>> {
        let mut sql = format!(
            "SELECT intDiv(created_at, 3600) * 3600 AS hour_start, toUInt64(count()) AS request_count, toUInt64(ifNull(sum(ifNull(input_tokens, 0)), 0)) AS input_tokens, toUInt64(ifNull(sum(ifNull(cached_input_tokens, 0)), 0)) AS cached_input_tokens, toUInt64(ifNull(sum(ifNull(output_tokens, 0)), 0)) AS output_tokens, toUInt64(ifNull(sum(ifNull(reasoning_tokens, 0)), 0)) AS reasoning_tokens FROM {} WHERE created_at >= ? AND created_at <= ? AND (billing_phase IS NULL OR billing_phase != 'streaming_open')",
            self.request_log_table
        );
        if tenant_id.is_some() {
            sql.push_str(" AND tenant_id = ?");
        }
        if account_id.is_some() {
            sql.push_str(" AND account_id = ?");
        }
        if api_key_id.is_some() {
            sql.push_str(" AND api_key_id = ?");
        }
        sql.push_str(" GROUP BY hour_start ORDER BY hour_start ASC LIMIT ?");

        let point_limit = ((end_ts.saturating_sub(start_ts) / 3600) + 2).clamp(1, 10_000) as u64;
        let mut query = self.ch_client.query(&sql).bind(start_ts).bind(end_ts);
        if let Some(tenant_id) = tenant_id {
            query = query.bind(tenant_id.to_string());
        }
        if let Some(account_id) = account_id {
            query = query.bind(account_id.to_string());
        }
        if let Some(api_key_id) = api_key_id {
            query = query.bind(api_key_id.to_string());
        }

        let rows = query
            .bind(point_limit)
            .fetch_all::<ClickHouseDashboardTokenTrendRow>()
            .await
            .context("failed to query clickhouse dashboard token trends")?;

        Ok(rows
            .into_iter()
            .map(|row| UsageDashboardTokenTrendPoint {
                hour_start: row.hour_start,
                request_count: row.request_count,
                input_tokens: row.input_tokens,
                cached_input_tokens: row.cached_input_tokens,
                output_tokens: row.output_tokens,
                reasoning_tokens: row.reasoning_tokens,
                total_tokens: row
                    .input_tokens
                    .saturating_add(row.cached_input_tokens)
                    .saturating_add(row.output_tokens)
                    .saturating_add(row.reasoning_tokens),
                estimated_cost_microusd: None,
            })
            .collect())
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
        let mut sql = format!(
            "SELECT ifNull(nullIf(model, ''), 'unknown') AS model, toUInt64(count()) AS request_count, toUInt64(ifNull(sum(ifNull(input_tokens, 0) + ifNull(cached_input_tokens, 0) + ifNull(output_tokens, 0) + ifNull(reasoning_tokens, 0)), 0)) AS total_tokens FROM {} WHERE created_at >= ? AND created_at <= ? AND (billing_phase IS NULL OR billing_phase != 'streaming_open')",
            self.request_log_table
        );
        if tenant_id.is_some() {
            sql.push_str(" AND tenant_id = ?");
        }
        if account_id.is_some() {
            sql.push_str(" AND account_id = ?");
        }
        if api_key_id.is_some() {
            sql.push_str(" AND api_key_id = ?");
        }
        sql.push_str(" GROUP BY model");
        if order_by_tokens {
            sql.push_str(" ORDER BY total_tokens DESC, request_count DESC, model ASC LIMIT 50");
        } else {
            sql.push_str(" ORDER BY request_count DESC, total_tokens DESC, model ASC LIMIT 50");
        }

        let mut query = self.ch_client.query(&sql).bind(start_ts).bind(end_ts);
        if let Some(tenant_id) = tenant_id {
            query = query.bind(tenant_id.to_string());
        }
        if let Some(account_id) = account_id {
            query = query.bind(account_id.to_string());
        }
        if let Some(api_key_id) = api_key_id {
            query = query.bind(api_key_id.to_string());
        }

        let rows = query
            .fetch_all::<ClickHouseDashboardModelDistributionRow>()
            .await
            .context("failed to query clickhouse dashboard model distribution")?;

        Ok(rows
            .into_iter()
            .map(|row| UsageDashboardModelDistributionItem {
                model: row.model,
                request_count: row.request_count,
                total_tokens: row.total_tokens,
            })
            .collect())
    }

    async fn fetch_summary(
        &self,
        start_ts: i64,
        end_ts: i64,
        tenant_id: Option<Uuid>,
        account_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<UsageSummaryQueryResponse> {
        let mut account_summary_sql = format!(
            "SELECT toUInt64(ifNull(sum(request_count), 0)) AS account_total_requests, toUInt64(uniqExact(account_id)) AS unique_account_count FROM {} WHERE hour_start >= ? AND hour_start <= ?",
            self.account_table
        );

        if account_id.is_some() {
            account_summary_sql.push_str(" AND account_id = ?");
        }

        let mut tenant_api_key_summary_sql = format!(
            "SELECT toUInt64(ifNull(sum(request_count), 0)) AS tenant_api_key_total_requests, toUInt64(uniqExact(tuple(tenant_id, api_key_id))) AS unique_tenant_api_key_count FROM {} WHERE hour_start >= ? AND hour_start <= ?",
            self.tenant_api_key_table
        );

        if tenant_id.is_some() {
            tenant_api_key_summary_sql.push_str(" AND tenant_id = ?");
        }

        if api_key_id.is_some() {
            tenant_api_key_summary_sql.push_str(" AND api_key_id = ?");
        }

        let mut account_summary_query = self
            .ch_client
            .query(&account_summary_sql)
            .bind(start_ts)
            .bind(end_ts);

        if let Some(account_id) = account_id {
            account_summary_query = account_summary_query.bind(account_id.to_string());
        }

        let account_summary = account_summary_query
            .fetch_one::<ClickHouseAccountUsageSummaryRow>()
            .await
            .context("failed to query clickhouse account usage summary")?;

        let mut tenant_api_key_summary_query = self
            .ch_client
            .query(&tenant_api_key_summary_sql)
            .bind(start_ts)
            .bind(end_ts);

        if let Some(tenant_id) = tenant_id {
            tenant_api_key_summary_query = tenant_api_key_summary_query.bind(tenant_id.to_string());
        }

        if let Some(api_key_id) = api_key_id {
            tenant_api_key_summary_query =
                tenant_api_key_summary_query.bind(api_key_id.to_string());
        }

        let tenant_api_key_summary = tenant_api_key_summary_query
            .fetch_one::<ClickHouseTenantApiKeyUsageSummaryRow>()
            .await
            .context("failed to query clickhouse tenant api-key usage summary")?;

        let dashboard_metrics = match tokio::try_join!(
            self.fetch_dashboard_summary_row(start_ts, end_ts, tenant_id, account_id, api_key_id),
            self.fetch_dashboard_token_trends(start_ts, end_ts, tenant_id, account_id, api_key_id),
            self.fetch_dashboard_model_distribution(
                start_ts,
                end_ts,
                tenant_id,
                account_id,
                api_key_id,
                false
            ),
            self.fetch_dashboard_model_distribution(
                start_ts,
                end_ts,
                tenant_id,
                account_id,
                api_key_id,
                true
            ),
        ) {
            Ok((
                dashboard_summary,
                token_trends,
                model_request_distribution,
                model_token_distribution,
            )) => {
                let token_breakdown = UsageDashboardTokenBreakdown {
                    input_tokens: dashboard_summary.input_tokens,
                    cached_input_tokens: dashboard_summary.cached_input_tokens,
                    output_tokens: dashboard_summary.output_tokens,
                    reasoning_tokens: dashboard_summary.reasoning_tokens,
                    total_tokens: dashboard_summary
                        .input_tokens
                        .saturating_add(dashboard_summary.cached_input_tokens)
                        .saturating_add(dashboard_summary.output_tokens)
                        .saturating_add(dashboard_summary.reasoning_tokens),
                };

                Some(UsageDashboardMetrics {
                    total_requests: dashboard_summary.total_requests,
                    estimated_cost_microusd: None,
                    token_breakdown,
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
                    "dashboard metrics query failed; falling back to summary-only response"
                );
                None
            }
        };

        Ok(UsageSummaryQueryResponse {
            start_ts,
            end_ts,
            account_total_requests: account_summary.account_total_requests,
            tenant_api_key_total_requests: tenant_api_key_summary.tenant_api_key_total_requests,
            unique_account_count: account_summary.unique_account_count,
            unique_tenant_api_key_count: tenant_api_key_summary.unique_tenant_api_key_count,
            estimated_cost_microusd: None,
            dashboard_metrics,
        })
    }

    async fn fetch_tenant_leaderboard(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        tenant_id: Option<Uuid>,
    ) -> Result<Vec<TenantUsageLeaderboardItem>> {
        let mut sql = format!(
            "SELECT tenant_id, toUInt64(ifNull(sum(request_count), 0)) AS total_requests FROM {} WHERE hour_start >= ? AND hour_start <= ?",
            self.tenant_api_key_table
        );

        if tenant_id.is_some() {
            sql.push_str(" AND tenant_id = ?");
        }

        sql.push_str(" GROUP BY tenant_id ORDER BY total_requests DESC, tenant_id ASC LIMIT ?");

        let mut query = self.ch_client.query(&sql).bind(start_ts).bind(end_ts);

        if let Some(tenant_id) = tenant_id {
            query = query.bind(tenant_id.to_string());
        }

        let rows = query
            .bind(limit as u64)
            .fetch_all::<ClickHouseTenantUsageLeaderboardRow>()
            .await
            .context("failed to query clickhouse tenant usage leaderboard")?;

        rows.into_iter()
            .map(|row| {
                let tenant_id = Uuid::parse_str(&row.tenant_id).with_context(|| {
                    format!("invalid tenant_id in clickhouse row: {}", row.tenant_id)
                })?;

                Ok(TenantUsageLeaderboardItem {
                    tenant_id,
                    total_requests: row.total_requests,
                })
            })
            .collect()
    }

    async fn fetch_account_leaderboard(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        account_id: Option<Uuid>,
    ) -> Result<Vec<AccountUsageLeaderboardItem>> {
        let mut sql = format!(
            "SELECT account_id, toUInt64(ifNull(sum(request_count), 0)) AS total_requests FROM {} WHERE hour_start >= ? AND hour_start <= ?",
            self.account_table
        );

        if account_id.is_some() {
            sql.push_str(" AND account_id = ?");
        }

        sql.push_str(" GROUP BY account_id ORDER BY total_requests DESC, account_id ASC LIMIT ?");

        let mut query = self.ch_client.query(&sql).bind(start_ts).bind(end_ts);

        if let Some(account_id) = account_id {
            query = query.bind(account_id.to_string());
        }

        let rows = query
            .bind(limit as u64)
            .fetch_all::<ClickHouseAccountUsageLeaderboardRow>()
            .await
            .context("failed to query clickhouse account usage leaderboard")?;

        rows.into_iter()
            .map(|row| {
                let account_id = Uuid::parse_str(&row.account_id).with_context(|| {
                    format!("invalid account_id in clickhouse row: {}", row.account_id)
                })?;

                Ok(AccountUsageLeaderboardItem {
                    account_id,
                    total_requests: row.total_requests,
                })
            })
            .collect()
    }

    async fn fetch_tenant_scoped_account_leaderboard(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        tenant_id: Uuid,
        account_id: Option<Uuid>,
    ) -> Result<Vec<AccountUsageLeaderboardItem>> {
        let mut sql = format!(
            "SELECT account_id, toUInt64(ifNull(sum(request_count), 0)) AS total_requests FROM {} WHERE hour_start >= ? AND hour_start <= ? AND tenant_id = ?",
            self.tenant_account_table
        );

        if account_id.is_some() {
            sql.push_str(" AND account_id = ?");
        }

        sql.push_str(" GROUP BY account_id ORDER BY total_requests DESC, account_id ASC LIMIT ?");

        let mut query = self
            .ch_client
            .query(&sql)
            .bind(start_ts)
            .bind(end_ts)
            .bind(tenant_id.to_string());

        if let Some(account_id) = account_id {
            query = query.bind(account_id.to_string());
        }

        let rows = query
            .bind(limit as u64)
            .fetch_all::<ClickHouseTenantScopedAccountUsageLeaderboardRow>()
            .await
            .context("failed to query clickhouse tenant-scoped account usage leaderboard")?;

        rows.into_iter()
            .map(|row| {
                let account_id = Uuid::parse_str(&row.account_id).with_context(|| {
                    format!(
                        "invalid account_id in tenant-scoped clickhouse row: {}",
                        row.account_id
                    )
                })?;

                Ok(AccountUsageLeaderboardItem {
                    account_id,
                    total_requests: row.total_requests,
                })
            })
            .collect()
    }

    async fn fetch_api_key_leaderboard(
        &self,
        start_ts: i64,
        end_ts: i64,
        limit: u32,
        tenant_id: Option<Uuid>,
        api_key_id: Option<Uuid>,
    ) -> Result<Vec<ApiKeyUsageLeaderboardItem>> {
        let mut sql = format!(
            "SELECT tenant_id, api_key_id, toUInt64(ifNull(sum(request_count), 0)) AS total_requests FROM {} WHERE hour_start >= ? AND hour_start <= ?",
            self.tenant_api_key_table
        );

        if tenant_id.is_some() {
            sql.push_str(" AND tenant_id = ?");
        }

        if api_key_id.is_some() {
            sql.push_str(" AND api_key_id = ?");
        }

        sql.push_str(
            " GROUP BY tenant_id, api_key_id ORDER BY total_requests DESC, tenant_id ASC, api_key_id ASC LIMIT ?",
        );

        let mut query = self.ch_client.query(&sql).bind(start_ts).bind(end_ts);

        if let Some(tenant_id) = tenant_id {
            query = query.bind(tenant_id.to_string());
        }

        if let Some(api_key_id) = api_key_id {
            query = query.bind(api_key_id.to_string());
        }

        let rows = query
            .bind(limit as u64)
            .fetch_all::<ClickHouseApiKeyUsageLeaderboardRow>()
            .await
            .context("failed to query clickhouse api-key usage leaderboard")?;

        rows.into_iter()
            .map(|row| {
                let tenant_id = Uuid::parse_str(&row.tenant_id).with_context(|| {
                    format!("invalid tenant_id in clickhouse row: {}", row.tenant_id)
                })?;
                let api_key_id = Uuid::parse_str(&row.api_key_id).with_context(|| {
                    format!("invalid api_key_id in clickhouse row: {}", row.api_key_id)
                })?;

                Ok(ApiKeyUsageLeaderboardItem {
                    tenant_id,
                    api_key_id,
                    total_requests: row.total_requests,
                })
            })
            .collect()
    }
}
