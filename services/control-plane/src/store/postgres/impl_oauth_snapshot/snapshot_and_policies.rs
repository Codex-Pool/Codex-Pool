impl PostgresStore {
    async fn create_rate_limit_refresh_job_inner(&self) -> Result<OAuthRateLimitRefreshJobSummary> {
        if let Some(existing_id) = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT id
            FROM oauth_rate_limit_refresh_jobs
            WHERE status IN ($1, $2)
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(DB_RATE_LIMIT_JOB_STATUS_QUEUED)
        .bind(DB_RATE_LIMIT_JOB_STATUS_RUNNING)
        .fetch_optional(&self.pool)
        .await
        .context("failed to check existing oauth rate-limit refresh jobs")?
        {
            return self
                .load_oauth_rate_limit_refresh_job_summary_inner(existing_id)
                .await;
        }

        let job_id = Uuid::new_v4();
        let now = Utc::now();
        sqlx::query(
            r#"
            INSERT INTO oauth_rate_limit_refresh_jobs (
                id,
                status,
                total,
                processed,
                success_count,
                failed_count,
                started_at,
                finished_at,
                created_at,
                updated_at
            )
            VALUES ($1, $2, 0, 0, 0, 0, NULL, NULL, $3, $3)
            "#,
        )
        .bind(job_id)
        .bind(DB_RATE_LIMIT_JOB_STATUS_QUEUED)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("failed to create oauth rate-limit refresh job")?;

        self.load_oauth_rate_limit_refresh_job_summary_inner(job_id).await
    }

    async fn mark_rate_limit_refresh_job_running_inner(&self, job_id: Uuid) -> Result<bool> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start oauth rate-limit refresh job transaction")?;

        let row = sqlx::query(
            r#"
            SELECT status
            FROM oauth_rate_limit_refresh_jobs
            WHERE id = $1
            FOR UPDATE
            "#,
        )
        .bind(job_id)
        .fetch_optional(tx.as_mut())
        .await
        .context("failed to lock oauth rate-limit refresh job")?
        .ok_or_else(|| anyhow!("job not found"))?;

        let status = row.try_get::<String, _>("status")?;
        if status != DB_RATE_LIMIT_JOB_STATUS_QUEUED {
            tx.commit()
                .await
                .context("failed to commit oauth rate-limit refresh lock transaction")?;
            return Ok(false);
        }

        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE oauth_rate_limit_refresh_jobs
            SET
                status = $2,
                started_at = $3,
                updated_at = $3
            WHERE id = $1
            "#,
        )
        .bind(job_id)
        .bind(DB_RATE_LIMIT_JOB_STATUS_RUNNING)
        .bind(now)
        .execute(tx.as_mut())
        .await
        .context("failed to mark oauth rate-limit refresh job running")?;

        tx.commit()
            .await
            .context("failed to commit oauth rate-limit refresh running transaction")?;
        Ok(true)
    }

    async fn set_rate_limit_refresh_job_total_inner(&self, job_id: Uuid, total: u64) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE oauth_rate_limit_refresh_jobs
            SET
                total = $2,
                updated_at = $3
            WHERE id = $1
            "#,
        )
        .bind(job_id)
        .bind(i64::try_from(total).unwrap_or(i64::MAX))
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .context("failed to set oauth rate-limit refresh total")?;
        Ok(())
    }

    async fn append_rate_limit_refresh_job_progress_inner(
        &self,
        job_id: Uuid,
        stats: &RateLimitRefreshBatchStats,
    ) -> Result<()> {
        let now = Utc::now();
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start oauth rate-limit refresh progress transaction")?;

        sqlx::query(
            r#"
            UPDATE oauth_rate_limit_refresh_jobs
            SET
                processed = processed + $2,
                success_count = success_count + $3,
                failed_count = failed_count + $4,
                updated_at = $5
            WHERE id = $1
            "#,
        )
        .bind(job_id)
        .bind(i64::try_from(stats.processed).unwrap_or(i64::MAX))
        .bind(i64::try_from(stats.success).unwrap_or(i64::MAX))
        .bind(i64::try_from(stats.failed).unwrap_or(i64::MAX))
        .bind(now)
        .execute(tx.as_mut())
        .await
        .context("failed to append oauth rate-limit refresh progress")?;

        for (error_code, count) in &stats.error_counts {
            sqlx::query(
                r#"
                INSERT INTO oauth_rate_limit_refresh_job_errors (
                    job_id,
                    error_code,
                    count,
                    updated_at
                )
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (job_id, error_code) DO UPDATE
                SET
                    count = oauth_rate_limit_refresh_job_errors.count + EXCLUDED.count,
                    updated_at = EXCLUDED.updated_at
                "#,
            )
            .bind(job_id)
            .bind(error_code)
            .bind(i64::try_from(*count).unwrap_or(i64::MAX))
            .bind(now)
            .execute(tx.as_mut())
            .await
            .with_context(|| {
                format!(
                    "failed to append oauth rate-limit refresh error count for error_code={error_code}"
                )
            })?;
        }

        tx.commit()
            .await
            .context("failed to commit oauth rate-limit refresh progress transaction")?;
        Ok(())
    }

    async fn finish_rate_limit_refresh_job_inner(
        &self,
        job_id: Uuid,
        status: OAuthRateLimitRefreshJobStatus,
    ) -> Result<()> {
        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE oauth_rate_limit_refresh_jobs
            SET
                status = $2,
                finished_at = $3,
                updated_at = $3
            WHERE id = $1
            "#,
        )
        .bind(job_id)
        .bind(rate_limit_refresh_job_status_to_db(status))
        .bind(now)
        .execute(&self.pool)
        .await
        .context("failed to finish oauth rate-limit refresh job")?;
        Ok(())
    }

    async fn mark_rate_limit_refresh_job_failed_inner(
        &self,
        job_id: Uuid,
        error_message: &str,
    ) -> Result<()> {
        let now = Utc::now();
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start oauth rate-limit refresh failure transaction")?;

        sqlx::query(
            r#"
            INSERT INTO oauth_rate_limit_refresh_job_errors (
                job_id,
                error_code,
                count,
                updated_at
            )
            VALUES ($1, $2, 1, $3)
            ON CONFLICT (job_id, error_code) DO UPDATE
            SET
                count = oauth_rate_limit_refresh_job_errors.count + 1,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(job_id)
        .bind("internal_error")
        .bind(now)
        .execute(tx.as_mut())
        .await
        .context("failed to append oauth rate-limit refresh internal error")?;

        sqlx::query(
            r#"
            UPDATE oauth_rate_limit_refresh_jobs
            SET
                status = $2,
                failed_count = failed_count + 1,
                finished_at = $3,
                updated_at = $3
            WHERE id = $1
            "#,
        )
        .bind(job_id)
        .bind(DB_RATE_LIMIT_JOB_STATUS_FAILED)
        .bind(now)
        .execute(tx.as_mut())
        .await
        .context("failed to mark oauth rate-limit refresh job failed")?;

        tx.commit()
            .await
            .context("failed to commit oauth rate-limit refresh failure transaction")?;

        tracing::warn!(job_id = %job_id, error = %error_message, "oauth rate-limit refresh job failed");
        Ok(())
    }

    async fn run_rate_limit_refresh_job_inner(&self, job_id: Uuid) -> Result<()> {
        let run_result: Result<()> = async {
            let should_run = self.mark_rate_limit_refresh_job_running_inner(job_id).await?;
            if !should_run {
                return Ok(());
            }

            let batch_size = rate_limit_refresh_batch_size_from_env();
            let concurrency = rate_limit_refresh_concurrency_from_env();
            let total = sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(1)
                FROM upstream_accounts a
                LEFT JOIN upstream_account_oauth_credentials c ON c.account_id = a.id
                LEFT JOIN upstream_account_session_profiles p ON p.account_id = a.id
                WHERE
                    a.pool_state = $2
                    AND a.enabled = true
                    AND (
                        (
                            a.auth_provider = $1
                            AND c.token_expires_at > $3
                            AND COALESCE(c.refresh_reused_detected, false) = false
                            AND NOT (
                                c.last_refresh_status = 'failed'
                                AND LOWER(COALESCE(c.last_refresh_error_code, '')) IN (
                                    'refresh_token_reused',
                                    'refresh_token_revoked',
                                    'invalid_refresh_token',
                                    'missing_client_id',
                                    'unauthorized_client'
                                )
                            )
                        )
                        OR (
                            a.auth_provider = $4
                            AND a.mode = $5
                            AND p.credential_kind = $6
                            AND p.token_expires_at > $3
                            AND NULLIF(BTRIM(COALESCE(a.bearer_token, '')), '') IS NOT NULL
                        )
                    )
                "#,
            )
            .bind(AUTH_PROVIDER_OAUTH_REFRESH_TOKEN)
            .bind(POOL_STATE_ACTIVE)
            .bind(Utc::now() + Duration::seconds(OAUTH_MIN_VALID_SEC))
            .bind(AUTH_PROVIDER_LEGACY_BEARER)
            .bind(upstream_mode_to_db(&UpstreamMode::CodexOauth))
            .bind(session_credential_kind_to_db(
                &SessionCredentialKind::OneTimeAccessToken,
            ))
            .fetch_one(&self.pool)
            .await
            .context("failed to count oauth rate-limit refresh targets")?;
            self.set_rate_limit_refresh_job_total_inner(
                job_id,
                u64::try_from(total.max(0)).unwrap_or(u64::MAX),
            )
            .await?;

            let mut cursor: Option<Uuid> = None;
            loop {
                let started_at = Utc::now();
                let targets = self
                    .load_all_rate_limit_refresh_targets_after(cursor, batch_size)
                    .await?;
                if targets.is_empty() {
                    break;
                }

                cursor = targets.last().map(|target| target.account_id);
                let fetched = targets.len();
                let progress_chunk_size = concurrency.max(1);
                for chunk in targets.chunks(progress_chunk_size) {
                    let stats = self
                        .refresh_rate_limit_targets_batch(chunk.to_vec(), concurrency)
                        .await;
                    self.emit_rate_limit_refresh_batch_summary_event(
                        started_at,
                        "control_plane.rate_limit_refresh_job",
                        Some(job_id),
                        chunk.len(),
                        &stats,
                        false,
                    );
                    self.append_rate_limit_refresh_job_progress_inner(job_id, &stats)
                        .await?;
                }

                if fetched < batch_size {
                    break;
                }
            }

            self.finish_rate_limit_refresh_job_inner(
                job_id,
                OAuthRateLimitRefreshJobStatus::Completed,
            )
            .await?;
            Ok(())
        }
        .await;

        if let Err(err) = run_result {
            let _ = self
                .mark_rate_limit_refresh_job_failed_inner(job_id, &err.to_string())
                .await;
            return Err(err);
        }

        Ok(())
    }

    async fn refresh_expiring_oauth_accounts_inner(&self) -> Result<()> {
        let _ = self.purge_expired_one_time_accounts_inner().await?;
        let batch_size = oauth_refresh_batch_size_from_env();
        let concurrency = oauth_refresh_concurrency_from_env();
        let max_rps = oauth_refresh_max_rps_from_env();
        let launch_interval = std::time::Duration::from_secs_f64(1.0 / f64::from(max_rps));
        let throttle = std::sync::Arc::new(tokio::sync::Mutex::new(tokio::time::Instant::now()));

        loop {
            let now = Utc::now();
            let rows = sqlx::query(
                r#"
                SELECT a.id
                FROM upstream_accounts a
                INNER JOIN upstream_account_oauth_credentials c ON c.account_id = a.id
                WHERE
                    a.auth_provider = $1
                    AND a.enabled = true
                    AND a.pool_state = $2
                    AND COALESCE(c.refresh_reused_detected, false) = false
                    AND NOT (
                        c.last_refresh_status = 'failed'
                        AND LOWER(COALESCE(c.last_refresh_error_code, '')) IN (
                            'refresh_token_reused',
                            'refresh_token_revoked',
                            'invalid_refresh_token',
                            'missing_client_id',
                            'unauthorized_client'
                        )
                    )
                    AND (
                        (c.next_refresh_at IS NOT NULL AND c.next_refresh_at <= $3)
                        OR (
                            c.next_refresh_at IS NULL
                            AND c.token_expires_at <= $3
                        )
                    )
                    AND (c.refresh_backoff_until IS NULL OR c.refresh_backoff_until <= $4)
                    AND (c.refresh_inflight_until IS NULL OR c.refresh_inflight_until <= $4)
                ORDER BY COALESCE(c.next_refresh_at, c.token_expires_at) ASC
                LIMIT $5
                "#,
            )
            .bind(AUTH_PROVIDER_OAUTH_REFRESH_TOKEN)
            .bind(POOL_STATE_ACTIVE)
            .bind(now + Duration::seconds(OAUTH_REFRESH_WINDOW_SEC))
            .bind(now)
            .bind(i64::try_from(batch_size).unwrap_or(i64::MAX))
            .fetch_all(&self.pool)
            .await
            .context("failed to list expiring oauth accounts")?;
            if rows.is_empty() {
                break;
            }

            let mut account_ids = Vec::with_capacity(rows.len());
            for row in rows {
                account_ids.push(row.try_get::<Uuid, _>("id")?);
            }
            let fetched = account_ids.len();

            futures_util::stream::iter(account_ids)
                .for_each_concurrent(Some(concurrency), |account_id| {
                    let throttle = throttle.clone();
                    async move {
                        throttle_refresh_start(throttle.as_ref(), launch_interval).await;
                        let _ = self.refresh_oauth_account_inner(account_id, false).await;
                    }
                })
                .await;

            if fetched < batch_size {
                break;
            }
        }

        Ok(())
    }

    async fn set_oauth_family_enabled_inner(
        &self,
        account_id: Uuid,
        enabled: bool,
    ) -> Result<OAuthFamilyActionResponse> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start oauth family action transaction")?;

        let row = sqlx::query(
            r#"
            SELECT
                a.auth_provider,
                c.token_family_id
            FROM upstream_accounts a
            LEFT JOIN upstream_account_oauth_credentials c ON c.account_id = a.id
            WHERE a.id = $1
            FOR UPDATE
            "#,
        )
        .bind(account_id)
        .fetch_optional(tx.as_mut())
        .await
        .context("failed to query oauth family by account")?
        .ok_or_else(|| anyhow!("account not found"))?;

        let auth_provider =
            parse_upstream_auth_provider(row.try_get::<String, _>("auth_provider")?.as_str())?;
        if auth_provider != UpstreamAuthProvider::OAuthRefreshToken {
            return Err(anyhow!("account is not an oauth account"));
        }

        let token_family_id = row
            .try_get::<Option<String>, _>("token_family_id")?
            .ok_or_else(|| anyhow!("oauth token family not found"))?;

        let affected_rows = sqlx::query(
            r#"
            UPDATE upstream_accounts
            SET enabled = $2
            WHERE id IN (
                SELECT c.account_id
                FROM upstream_account_oauth_credentials c
                INNER JOIN upstream_accounts a ON a.id = c.account_id
                WHERE c.token_family_id = $1 AND a.auth_provider = $3
            )
            RETURNING id
            "#,
        )
        .bind(&token_family_id)
        .bind(enabled)
        .bind(AUTH_PROVIDER_OAUTH_REFRESH_TOKEN)
        .fetch_all(tx.as_mut())
        .await
        .context("failed to update oauth family enabled flag")?;

        if enabled {
            sqlx::query(
                r#"
                UPDATE upstream_account_oauth_credentials
                SET
                    refresh_reused_detected = false,
                    refresh_backoff_until = NULL,
                    refresh_inflight_until = NULL,
                    updated_at = $2
                WHERE token_family_id = $1
                "#,
            )
            .bind(&token_family_id)
            .bind(Utc::now())
            .execute(tx.as_mut())
            .await
            .context("failed to clear oauth family recovered flags")?;
        }

        for row in &affected_rows {
            self.append_data_plane_outbox_event_tx(
                &mut tx,
                DataPlaneSnapshotEventType::AccountUpsert,
                row.try_get::<Uuid, _>("id")?,
            )
            .await?;
        }

        self.bump_revision_tx(&mut tx).await?;
        tx.commit()
            .await
            .context("failed to commit oauth family action transaction")?;

        Ok(OAuthFamilyActionResponse {
            account_id,
            token_family_id: Some(token_family_id),
            enabled,
            affected_accounts: u64::try_from(affected_rows.len()).unwrap_or(u64::MAX),
        })
    }

    async fn upsert_routing_policy_inner(
        &self,
        req: UpsertRoutingPolicyRequest,
    ) -> Result<RoutingPolicy> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start routing policy transaction")?;
        let updated_at = Utc::now();
        let strategy = routing_strategy_to_db(&req.strategy);

        let row = sqlx::query(
            r#"
            INSERT INTO routing_policies (
                tenant_id,
                strategy,
                max_retries,
                stream_max_retries,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (tenant_id) DO UPDATE
            SET
                strategy = EXCLUDED.strategy,
                max_retries = EXCLUDED.max_retries,
                stream_max_retries = EXCLUDED.stream_max_retries,
                updated_at = EXCLUDED.updated_at
            RETURNING tenant_id, strategy, max_retries, stream_max_retries, updated_at
            "#,
        )
        .bind(req.tenant_id)
        .bind(strategy)
        .bind(i64::from(req.max_retries))
        .bind(i64::from(req.stream_max_retries))
        .bind(updated_at)
        .fetch_one(tx.as_mut())
        .await
        .context("failed to upsert routing policy")?;

        self.bump_revision_tx(&mut tx).await?;
        tx.commit()
            .await
            .context("failed to commit routing policy transaction")?;

        parse_routing_policy_row(&row)
    }

    async fn upsert_retry_policy_inner(
        &self,
        req: UpsertRetryPolicyRequest,
    ) -> Result<RoutingPolicy> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start retry policy transaction")?;
        let updated_at = Utc::now();

        let row = sqlx::query(
            r#"
            INSERT INTO routing_policies (
                tenant_id,
                strategy,
                max_retries,
                stream_max_retries,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (tenant_id) DO UPDATE
            SET
                max_retries = EXCLUDED.max_retries,
                updated_at = EXCLUDED.updated_at
            RETURNING tenant_id, strategy, max_retries, stream_max_retries, updated_at
            "#,
        )
        .bind(req.tenant_id)
        .bind(routing_strategy_to_db(&RoutingStrategy::RoundRobin))
        .bind(i64::from(req.max_retries))
        .bind(0_i64)
        .bind(updated_at)
        .fetch_one(tx.as_mut())
        .await
        .context("failed to upsert retry policy")?;

        self.bump_revision_tx(&mut tx).await?;
        tx.commit()
            .await
            .context("failed to commit retry policy transaction")?;

        parse_routing_policy_row(&row)
    }

    async fn upsert_stream_retry_policy_inner(
        &self,
        req: UpsertStreamRetryPolicyRequest,
    ) -> Result<RoutingPolicy> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start stream retry policy transaction")?;
        let updated_at = Utc::now();

        let row = sqlx::query(
            r#"
            INSERT INTO routing_policies (
                tenant_id,
                strategy,
                max_retries,
                stream_max_retries,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (tenant_id) DO UPDATE
            SET
                stream_max_retries = EXCLUDED.stream_max_retries,
                updated_at = EXCLUDED.updated_at
            RETURNING tenant_id, strategy, max_retries, stream_max_retries, updated_at
            "#,
        )
        .bind(req.tenant_id)
        .bind(routing_strategy_to_db(&RoutingStrategy::RoundRobin))
        .bind(0_i64)
        .bind(i64::from(req.stream_max_retries))
        .bind(updated_at)
        .fetch_one(tx.as_mut())
        .await
        .context("failed to upsert stream retry policy")?;

        self.bump_revision_tx(&mut tx).await?;
        tx.commit()
            .await
            .context("failed to commit stream retry policy transaction")?;

        parse_routing_policy_row(&row)
    }

    async fn snapshot_inner(&self) -> Result<DataPlaneSnapshot> {
        let revision = self.current_revision().await?;
        let cursor = self.data_plane_outbox_cursor_inner().await?;
        let accounts = self.load_snapshot_accounts_inner().await?;
        let account_traits_map = self.load_account_routing_traits_inner(&accounts).await?;
        let compiled_routing_plan = self
            .build_compiled_routing_plan_inner(
                &accounts,
                &account_traits_map,
                Some("postgres_snapshot".to_string()),
            )
            .await?;
        let ai_error_learning_settings = self.load_upstream_error_learning_settings_inner().await?;
        let outbound_proxy_pool_settings = self.load_outbound_proxy_pool_settings_inner().await?;
        let outbound_proxy_nodes = self.list_outbound_proxy_nodes_inner().await?;
        let approved_upstream_error_templates =
            self.load_approved_upstream_error_templates_inner().await?;
        let builtin_error_templates = self.list_builtin_error_templates().await?;
        let account_traits = accounts
            .iter()
            .filter_map(|account| account_traits_map.get(&account.id).cloned())
            .collect();
        Ok(DataPlaneSnapshot {
            revision,
            cursor,
            accounts,
            account_traits,
            compiled_routing_plan,
            ai_error_learning_settings,
            approved_upstream_error_templates,
            builtin_error_templates,
            outbound_proxy_pool_settings,
            outbound_proxy_nodes,
            issued_at: Utc::now(),
        })
    }

    async fn current_revision(&self) -> Result<u64> {
        let row = sqlx::query(
            r#"
            SELECT revision
            FROM snapshot_state
            WHERE singleton = $1
            "#,
        )
        .bind(SNAPSHOT_SINGLETON_ROW)
        .fetch_one(&self.pool)
        .await
        .context("failed to load snapshot revision")?;
        let revision = row.try_get::<i64, _>("revision")?;
        u64::try_from(revision).context("snapshot revision must be non-negative")
    }

    async fn bump_revision_tx(&self, tx: &mut Transaction<'_, Postgres>) -> Result<u64> {
        let affected = sqlx::query(
            r#"
            UPDATE snapshot_state
            SET dirty = true
            WHERE singleton = $1
              AND dirty = false
            "#,
        )
        .bind(SNAPSHOT_SINGLETON_ROW)
        .execute(tx.as_mut())
        .await
        .context("failed to mark snapshot revision dirty")?
        .rows_affected();
        Ok(affected)
    }

    async fn flush_snapshot_revision_batch_inner(&self, max_batch: usize) -> Result<u32> {
        let mut flushed = 0_u32;
        let capped_batch = max_batch.clamp(1, 10_000);

        for _ in 0..capped_batch {
            let affected = sqlx::query(
                r#"
                UPDATE snapshot_state
                SET revision = revision + 1,
                    dirty = false
                WHERE singleton = $1
                  AND dirty = true
                "#,
            )
            .bind(SNAPSHOT_SINGLETON_ROW)
            .execute(&self.pool)
            .await
            .context("failed to flush snapshot revision")?
            .rows_affected();

            if affected == 0 {
                break;
            }
            flushed = flushed.saturating_add(1);
        }

        Ok(flushed)
    }
}

async fn throttle_refresh_start(
    throttle: &tokio::sync::Mutex<tokio::time::Instant>,
    launch_interval: std::time::Duration,
) {
    if launch_interval.is_zero() {
        return;
    }
    let wait = {
        let mut next = throttle.lock().await;
        let now = tokio::time::Instant::now();
        let scheduled = (*next).max(now);
        *next = scheduled + launch_interval;
        scheduled.saturating_duration_since(now)
    };
    if !wait.is_zero() {
        tokio::time::sleep(wait).await;
    }
}

#[cfg(test)]
mod snapshot_state_lock_tests {
    use super::{PostgresStore, SNAPSHOT_SINGLETON_ROW};

    fn test_db_url() -> Option<String> {
        std::env::var("CONTROL_PLANE_DATABASE_URL")
            .ok()
            .or_else(|| std::env::var("DATABASE_URL").ok())
    }

    #[tokio::test]
    async fn bump_revision_tx_skips_when_snapshot_already_dirty() {
        let Some(db_url) = test_db_url() else {
            eprintln!(
                "skip bump_revision_tx_skips_when_snapshot_already_dirty: set CONTROL_PLANE_DATABASE_URL"
            );
            return;
        };

        let store = PostgresStore::connect(&db_url).await.unwrap();
        let mut tx = store.pool.begin().await.unwrap();
        sqlx::query(
            r#"
            UPDATE snapshot_state
            SET dirty = true
            WHERE singleton = $1
            "#,
        )
        .bind(SNAPSHOT_SINGLETON_ROW)
        .execute(tx.as_mut())
        .await
        .unwrap();

        let affected = store.bump_revision_tx(&mut tx).await.unwrap();
        tx.rollback().await.unwrap();

        assert_eq!(
            affected, 0,
            "snapshot_state 已经 dirty 时，不应再次写入同一行"
        );
    }
}
