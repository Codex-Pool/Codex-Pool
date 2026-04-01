const DB_RATE_LIMIT_JOB_STATUS_QUEUED: &str = "queued";
const DB_RATE_LIMIT_JOB_STATUS_RUNNING: &str = "running";
const DB_RATE_LIMIT_JOB_STATUS_COMPLETED: &str = "completed";
const DB_RATE_LIMIT_JOB_STATUS_FAILED: &str = "failed";
const DB_RATE_LIMIT_JOB_STATUS_CANCELLED: &str = "cancelled";

fn parse_rate_limit_snapshots(raw: Option<String>) -> Vec<OAuthRateLimitSnapshot> {
    raw.and_then(|value| serde_json::from_str::<Vec<OAuthRateLimitSnapshot>>(&value).ok())
        .unwrap_or_default()
}

fn rate_limit_refresh_job_status_to_db(status: OAuthRateLimitRefreshJobStatus) -> &'static str {
    match status {
        OAuthRateLimitRefreshJobStatus::Queued => DB_RATE_LIMIT_JOB_STATUS_QUEUED,
        OAuthRateLimitRefreshJobStatus::Running => DB_RATE_LIMIT_JOB_STATUS_RUNNING,
        OAuthRateLimitRefreshJobStatus::Completed => DB_RATE_LIMIT_JOB_STATUS_COMPLETED,
        OAuthRateLimitRefreshJobStatus::Failed => DB_RATE_LIMIT_JOB_STATUS_FAILED,
        OAuthRateLimitRefreshJobStatus::Cancelled => DB_RATE_LIMIT_JOB_STATUS_CANCELLED,
    }
}

fn parse_rate_limit_refresh_job_status(raw: &str) -> Result<OAuthRateLimitRefreshJobStatus> {
    match raw {
        DB_RATE_LIMIT_JOB_STATUS_QUEUED => Ok(OAuthRateLimitRefreshJobStatus::Queued),
        DB_RATE_LIMIT_JOB_STATUS_RUNNING => Ok(OAuthRateLimitRefreshJobStatus::Running),
        DB_RATE_LIMIT_JOB_STATUS_COMPLETED => Ok(OAuthRateLimitRefreshJobStatus::Completed),
        DB_RATE_LIMIT_JOB_STATUS_FAILED => Ok(OAuthRateLimitRefreshJobStatus::Failed),
        DB_RATE_LIMIT_JOB_STATUS_CANCELLED => Ok(OAuthRateLimitRefreshJobStatus::Cancelled),
        _ => Err(anyhow!("unsupported oauth rate-limit refresh job status: {raw}")),
    }
}

fn oauth_refresh_jitter_seconds(account_id: Uuid) -> i64 {
    let mut seed = 0_u64;
    for byte in account_id.as_bytes() {
        seed = seed.wrapping_mul(131).wrapping_add(u64::from(*byte));
    }
    i64::try_from(seed % 121).unwrap_or(0)
}

fn schedule_next_oauth_refresh(expires_at: DateTime<Utc>, account_id: Uuid) -> DateTime<Utc> {
    let jitter = Duration::seconds(oauth_refresh_jitter_seconds(account_id));
    let baseline = expires_at - Duration::minutes(3) - jitter;
    let floor = Utc::now() + Duration::seconds(30);
    if baseline < floor { floor } else { baseline }
}

#[derive(Debug, Clone)]
struct RateLimitRefreshTarget {
    account_id: Uuid,
    auth_provider: UpstreamAuthProvider,
    base_url: String,
    chatgpt_account_id: Option<String>,
    access_token_enc: Option<String>,
    bearer_token: Option<String>,
}

#[derive(Debug, Default)]
struct RateLimitRefreshBatchStats {
    processed: u64,
    success: u64,
    failed: u64,
    error_counts: std::collections::HashMap<String, u64>,
}

impl PostgresStore {
    fn system_event_runtime_inner(
        &self,
    ) -> Option<Arc<crate::system_events::SystemEventLogRuntime>> {
        self.system_event_runtime.read().unwrap().clone()
    }

    fn emit_system_event_inner(&self, event: codex_pool_core::events::SystemEventWrite) {
        if let Some(runtime) = self.system_event_runtime_inner() {
            runtime.emit_best_effort(event);
        }
    }

    fn emit_rate_limit_refresh_batch_summary_event(
        &self,
        started_at: DateTime<Utc>,
        source: &str,
        job_id: Option<Uuid>,
        fetched: usize,
        stats: &RateLimitRefreshBatchStats,
        due_only: bool,
    ) {
        self.emit_system_event_inner(codex_pool_core::events::SystemEventWrite {
            event_id: None,
            ts: Some(started_at),
            category: codex_pool_core::events::SystemEventCategory::Patrol,
            event_type: "rate_limit_refresh_batch_completed".to_string(),
            severity: if stats.failed > 0 {
                codex_pool_core::events::SystemEventSeverity::Warn
            } else {
                codex_pool_core::events::SystemEventSeverity::Info
            },
            source: source.to_string(),
            tenant_id: None,
            account_id: None,
            request_id: None,
            trace_request_id: None,
            job_id,
            account_label: None,
            auth_provider: None,
            operator_state_from: None,
            operator_state_to: None,
            reason_class: None,
            reason_code: Some("rate_limit_refresh_batch_completed".to_string()),
            next_action_at: None,
            path: None,
            method: None,
            model: None,
            selected_account_id: None,
            selected_proxy_id: None,
            routing_decision: None,
            failover_scope: None,
            status_code: None,
            upstream_status_code: None,
            latency_ms: None,
            message: Some(format!(
                "rate-limit refresh scanned {fetched} targets ({} ok / {} failed)",
                stats.success, stats.failed
            )),
            preview_text: None,
            payload_json: Some(serde_json::json!({
                "fetched": fetched,
                "processed": stats.processed,
                "success": stats.success,
                "failed": stats.failed,
                "error_counts": stats.error_counts,
                "due_only": due_only,
            })),
            secret_preview: None,
        });
    }

    async fn refresh_oauth_account_inner(
        &self,
        account_id: Uuid,
        force: bool,
    ) -> Result<OAuthAccountStatusResponse> {
        let row = sqlx::query(
            r#"
            SELECT
                a.id,
                a.enabled,
                a.base_url,
                a.auth_provider,
                c.access_token_enc,
                c.refresh_token_enc,
                c.fallback_access_token_enc,
                c.token_expires_at,
                c.fallback_token_expires_at,
                c.last_refresh_at,
                c.last_refresh_status,
                c.last_refresh_error,
                c.refresh_failure_count,
                c.refresh_backoff_until,
                c.refresh_inflight_until,
                c.next_refresh_at
            FROM upstream_accounts a
            LEFT JOIN upstream_account_oauth_credentials c ON c.account_id = a.id
            WHERE a.id = $1
            "#,
        )
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to query oauth account")?
        .ok_or_else(|| anyhow!("account not found"))?;

        let auth_provider =
            parse_upstream_auth_provider(row.try_get::<String, _>("auth_provider")?.as_str())?;
        if auth_provider != UpstreamAuthProvider::OAuthRefreshToken {
            return self.fetch_oauth_account_status(account_id).await;
        }

        let token_expires_at = row
            .try_get::<Option<DateTime<Utc>>, _>("token_expires_at")?
            .ok_or_else(|| anyhow!("oauth credential not found"))?;
        let refresh_backoff_until =
            row.try_get::<Option<DateTime<Utc>>, _>("refresh_backoff_until")?;
        let refresh_inflight_until =
            row.try_get::<Option<DateTime<Utc>>, _>("refresh_inflight_until")?;
        let next_refresh_at = row.try_get::<Option<DateTime<Utc>>, _>("next_refresh_at")?;
        let now = Utc::now();
        let has_access_token_fallback = row
            .try_get::<Option<String>, _>("fallback_access_token_enc")?
            .is_some();
        let fallback_token_expires_at =
            row.try_get::<Option<DateTime<Utc>>, _>("fallback_token_expires_at")?;

        if refresh_credential_is_terminal_invalid(
            &parse_oauth_refresh_status(
                row.try_get::<Option<String>, _>("last_refresh_status")?
                    .unwrap_or_else(|| "never".to_string())
                    .as_str(),
            )?,
            row.try_get::<Option<bool>, _>("refresh_reused_detected")?
                .unwrap_or(false),
            row.try_get::<Option<String>, _>("last_refresh_error_code")?
                .as_deref(),
        ) && has_usable_access_token_fallback(
            has_access_token_fallback,
            fallback_token_expires_at,
            now,
        ) {
            return self.fetch_oauth_account_status(account_id).await;
        }

        let should_refresh = force
            || next_refresh_at
                .map(|refresh_at| refresh_at <= now)
                .unwrap_or(token_expires_at <= now + Duration::seconds(OAUTH_REFRESH_WINDOW_SEC));
        let in_backoff = refresh_backoff_until.is_some_and(|until| until > now);
        let in_flight = refresh_inflight_until.is_some_and(|until| until > now);
        if !should_refresh || in_backoff || in_flight {
            return self.fetch_oauth_account_status(account_id).await;
        }

        if !self.try_claim_oauth_refresh_slot(account_id, now).await? {
            return self.fetch_oauth_account_status(account_id).await;
        }

        let cipher = self.require_credential_cipher()?;
        let refresh_token_enc = row
            .try_get::<Option<String>, _>("refresh_token_enc")?
            .ok_or_else(|| anyhow!("oauth credential not found"))?;
        let refresh_token = match cipher.decrypt(&refresh_token_enc) {
            Ok(token) => token,
            Err(err) => {
                self.persist_oauth_refresh_failure(
                    account_id,
                    "credential_decrypt_failed".to_string(),
                    err.to_string(),
                )
                .await?;
                return self.fetch_oauth_account_status(account_id).await;
            }
        };
        let base_url = row.try_get::<String, _>("base_url")?;

        match self
            .oauth_client
            .refresh_token(&refresh_token, Some(&base_url))
            .await
        {
            Ok(token_info) => {
                self.persist_oauth_refresh_success(account_id, token_info)
                    .await?;
            }
            Err(err) => {
                self.persist_oauth_refresh_failure(
                    account_id,
                    err.code().as_str().to_string(),
                    err.to_string(),
                )
                .await?;
            }
        }

        self.fetch_oauth_account_status(account_id).await
    }

    async fn try_claim_oauth_refresh_slot(
        &self,
        account_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<bool> {
        let claim_until = now + Duration::seconds(OAUTH_REFRESH_INFLIGHT_TTL_SEC);
        let affected = sqlx::query(
            r#"
            UPDATE upstream_account_oauth_credentials
            SET
                refresh_inflight_until = $2,
                updated_at = $3
            WHERE
                account_id = $1
                AND (refresh_inflight_until IS NULL OR refresh_inflight_until <= $3)
            "#,
        )
        .bind(account_id)
        .bind(claim_until)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("failed to claim oauth refresh slot")?
        .rows_affected();
        Ok(affected > 0)
    }

    async fn persist_oauth_refresh_success(
        &self,
        account_id: Uuid,
        token_info: OAuthTokenInfo,
    ) -> Result<()> {
        let cipher = self.require_credential_cipher()?;
        let access_token_enc = cipher.encrypt(&token_info.access_token)?;
        let refresh_token_enc = cipher.encrypt(&token_info.refresh_token)?;
        let refresh_token_sha256 = refresh_token_sha256(&token_info.refresh_token);
        let now = Utc::now();
        let next_refresh_at = schedule_next_oauth_refresh(token_info.expires_at, account_id);

        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start oauth refresh success transaction")?;

        sqlx::query(
            r#"
            UPDATE upstream_account_oauth_credentials
            SET
                access_token_enc = $2,
                refresh_token_enc = $3,
                refresh_token_sha256 = $4,
                token_expires_at = $5,
                last_refresh_at = $6,
                last_refresh_status = 'ok',
                last_refresh_error_code = NULL,
                last_refresh_error = NULL,
                refresh_failure_count = 0,
                refresh_backoff_until = NULL,
                refresh_reused_detected = false,
                refresh_inflight_until = NULL,
                next_refresh_at = $7,
                token_version = GREATEST(token_version, 0) + 1,
                updated_at = $6
            WHERE account_id = $1
            "#,
        )
        .bind(account_id)
        .bind(access_token_enc)
        .bind(refresh_token_enc)
        .bind(refresh_token_sha256)
        .bind(token_info.expires_at)
        .bind(now)
        .bind(next_refresh_at)
        .execute(tx.as_mut())
        .await
        .context("failed to persist oauth refresh success")?;

        sqlx::query(
            r#"
            UPDATE upstream_accounts
            SET chatgpt_account_id = COALESCE($2, chatgpt_account_id)
            WHERE id = $1
            "#,
        )
        .bind(account_id)
        .bind(token_info.chatgpt_account_id.clone())
        .execute(tx.as_mut())
        .await
        .context("failed to backfill oauth account chatgpt_account_id")?;

        let session_profile = SessionProfileRecord::from_oauth_token_info(
            &token_info,
            SessionCredentialKind::RefreshRotatable,
            token_info.chatgpt_plan_type.clone(),
            None,
        );
        self.upsert_session_profile_tx(tx.as_mut(), account_id, &session_profile)
        .await?;

        self.bump_revision_tx(&mut tx).await?;
        self.append_data_plane_outbox_event_tx(
            &mut tx,
            DataPlaneSnapshotEventType::AccountUpsert,
            account_id,
        )
        .await?;
        tx.commit()
            .await
            .context("failed to commit oauth refresh success transaction")?;
        Ok(())
    }

    async fn persist_oauth_refresh_failure(
        &self,
        account_id: Uuid,
        error_code: String,
        error_message: String,
    ) -> Result<()> {
        let now = Utc::now();
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start oauth refresh failure transaction")?;

        let row = sqlx::query(
            r#"
            SELECT refresh_failure_count
                , token_family_id
            FROM upstream_account_oauth_credentials
            WHERE account_id = $1
            "#,
        )
        .bind(account_id)
        .fetch_one(tx.as_mut())
        .await
        .context("failed to load oauth refresh failure count")?;
        let previous_failures = row.try_get::<i32, _>("refresh_failure_count")?;
        let token_family_id = row.try_get::<Option<String>, _>("token_family_id")?;
        let next_failures = previous_failures.saturating_add(1);
        let backoff = match next_failures {
            0 => Duration::seconds(0),
            1 => Duration::seconds(30),
            2 => Duration::seconds(60),
            3 => Duration::seconds(120),
            _ => Duration::seconds(300),
        };
        let next_refresh_at = now + backoff + Duration::seconds(oauth_refresh_jitter_seconds(account_id));

        sqlx::query(
            r#"
            UPDATE upstream_account_oauth_credentials
            SET
                last_refresh_at = $2,
                last_refresh_status = 'failed',
                last_refresh_error_code = $3,
                last_refresh_error = $4,
                refresh_failure_count = $5,
                refresh_backoff_until = $6,
                next_refresh_at = $7,
                refresh_reused_detected = CASE WHEN $3 IN ('refresh_token_reused', 'refresh_token_revoked') THEN true ELSE refresh_reused_detected END,
                refresh_inflight_until = NULL,
                updated_at = $2
            WHERE account_id = $1
            "#,
        )
        .bind(account_id)
        .bind(now)
        .bind(&error_code)
        .bind(truncate_error_message(error_message))
        .bind(next_failures)
        .bind(now + backoff)
        .bind(next_refresh_at)
        .execute(tx.as_mut())
        .await
        .context("failed to persist oauth refresh failure")?;

        if should_revoke_oauth_token_family(&error_code) {
            if let Some(token_family_id) = token_family_id.as_deref() {
                let rows = sqlx::query(
                    r#"
                    UPDATE upstream_accounts
                    SET enabled = false
                    WHERE id IN (
                        SELECT account_id
                        FROM upstream_account_oauth_credentials
                        WHERE token_family_id = $1
                    )
                    RETURNING id
                    "#,
                )
                .bind(token_family_id)
                .fetch_all(tx.as_mut())
                .await
                .context("failed to disable oauth token family accounts")?;
                for row in rows {
                    self.append_data_plane_outbox_event_tx(
                        &mut tx,
                        DataPlaneSnapshotEventType::AccountUpsert,
                        row.try_get::<Uuid, _>("id")?,
                    )
                    .await?;
                }
            }
        }

        self.bump_revision_tx(&mut tx).await?;
        self.append_data_plane_outbox_event_tx(
            &mut tx,
            DataPlaneSnapshotEventType::AccountUpsert,
            account_id,
        )
        .await?;
        tx.commit()
            .await
            .context("failed to commit oauth refresh failure transaction")?;
        Ok(())
    }

    async fn fetch_live_rate_limits_result(
        &self,
        auth_provider: &UpstreamAuthProvider,
        access_token_enc: Option<String>,
        bearer_token: Option<String>,
        base_url: Option<String>,
        chatgpt_account_id: Option<String>,
    ) -> Result<crate::oauth::OAuthUsageSnapshot, (String, String)> {
        let access_token = match auth_provider {
            UpstreamAuthProvider::OAuthRefreshToken => {
                let Some(access_token_enc) = access_token_enc else {
                    return Ok(crate::oauth::OAuthUsageSnapshot::default());
                };
                let Some(cipher) = self.credential_cipher.as_ref() else {
                    return Err((
                        "credential_cipher_missing".to_string(),
                        "oauth credential cipher is not configured".to_string(),
                    ));
                };
                match cipher.decrypt(&access_token_enc) {
                    Ok(value) => value,
                    Err(err) => {
                        return Err(("credential_decrypt_failed".to_string(), err.to_string()));
                    }
                }
            }
            UpstreamAuthProvider::LegacyBearer => bearer_token.unwrap_or_default(),
        };
        if access_token.trim().is_empty() {
            return Ok(crate::oauth::OAuthUsageSnapshot::default());
        }

        self.oauth_client
            .fetch_usage(
                &access_token,
                base_url.as_deref(),
                chatgpt_account_id.as_deref(),
            )
            .await
            .map_err(|err| (err.code().as_str().to_string(), err.to_string()))
    }
}
