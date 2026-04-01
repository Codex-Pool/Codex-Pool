fn parse_oauth_vault_record_row(
    row: &sqlx_postgres::PgRow,
) -> Result<OAuthRefreshTokenVaultRecord> {
    let admission_rate_limits = row
        .try_get::<Option<String>, _>("admission_rate_limits_json_text")?
        .map(|value| {
            serde_json::from_str::<Vec<OAuthRateLimitSnapshot>>(&value)
                .context("failed to decode oauth vault admission rate limits")
        })
        .transpose()?
        .unwrap_or_default();

    Ok(OAuthRefreshTokenVaultRecord {
        id: row.try_get("id")?,
        label: row.try_get("label")?,
        email: row.try_get("email")?,
        base_url: row.try_get("base_url")?,
        refresh_token_enc: row.try_get("refresh_token_enc")?,
        fallback_access_token_enc: row.try_get("fallback_access_token_enc")?,
        fallback_token_expires_at: row.try_get("fallback_token_expires_at")?,
        refresh_token_sha256: row.try_get("refresh_token_sha256")?,
        chatgpt_account_id: row.try_get("chatgpt_account_id")?,
        chatgpt_plan_type: row.try_get("chatgpt_plan_type")?,
        source_type: row.try_get("source_type")?,
        desired_mode: parse_upstream_mode(row.try_get::<String, _>("desired_mode")?.as_str())?,
        desired_enabled: row.try_get("desired_enabled")?,
        desired_priority: row.try_get("desired_priority")?,
        status: parse_oauth_vault_record_status(row.try_get::<String, _>("status")?.as_str())?,
        failure_count: row
            .try_get::<i32, _>("failure_count")?
            .max(0)
            .try_into()
            .unwrap_or(u32::MAX),
        backoff_until: row.try_get("backoff_until")?,
        next_attempt_at: row.try_get("next_attempt_at")?,
        last_error_code: row.try_get("last_error_code")?,
        last_error_message: row.try_get("last_error_message")?,
        admission_source: row.try_get("admission_source")?,
        admission_checked_at: row.try_get("admission_checked_at")?,
        admission_retry_after: row.try_get("admission_retry_after")?,
        admission_error_code: row.try_get("admission_error_code")?,
        admission_error_message: row.try_get("admission_error_message")?,
        admission_rate_limits,
        admission_rate_limits_expires_at: row.try_get("admission_rate_limits_expires_at")?,
        failure_stage: row
            .try_get::<Option<String>, _>("failure_stage")?
            .map(|raw| parse_oauth_inventory_failure_stage(raw.as_str()))
            .transpose()?,
        attempt_count: row
            .try_get::<i32, _>("attempt_count")?
            .max(0)
            .try_into()
            .unwrap_or(u32::MAX),
        transient_retry_count: row
            .try_get::<i32, _>("transient_retry_count")?
            .max(0)
            .try_into()
            .unwrap_or(u32::MAX),
        next_retry_at: row.try_get("next_retry_at")?,
        retryable: row.try_get("retryable")?,
        terminal_reason: row.try_get("terminal_reason")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn parse_oauth_inventory_record_row(row: &sqlx_postgres::PgRow) -> Result<OAuthInventoryRecord> {
    let admission_rate_limits = row
        .try_get::<Option<String>, _>("admission_rate_limits_json_text")?
        .map(|value| {
            serde_json::from_str::<Vec<OAuthRateLimitSnapshot>>(&value)
                .context("failed to decode oauth inventory admission rate limits")
        })
        .transpose()?
        .unwrap_or_default();

    Ok(OAuthInventoryRecord {
        id: row.try_get("id")?,
        label: row.try_get("label")?,
        email: row.try_get("email")?,
        chatgpt_account_id: row.try_get("chatgpt_account_id")?,
        chatgpt_plan_type: row.try_get("chatgpt_plan_type")?,
        source_type: row.try_get("source_type")?,
        vault_status: parse_oauth_vault_record_status(row.try_get::<String, _>("status")?.as_str())?,
        has_refresh_token: true,
        has_access_token_fallback: row
            .try_get::<Option<String>, _>("fallback_access_token_enc")?
            .is_some(),
        admission_source: row.try_get("admission_source")?,
        admission_checked_at: row.try_get("admission_checked_at")?,
        admission_retry_after: row.try_get("admission_retry_after")?,
        admission_error_code: row.try_get("admission_error_code")?,
        admission_error_message: row.try_get("admission_error_message")?,
        admission_rate_limits,
        admission_rate_limits_expires_at: row.try_get("admission_rate_limits_expires_at")?,
        failure_stage: row
            .try_get::<Option<String>, _>("failure_stage")?
            .map(|raw| parse_oauth_inventory_failure_stage(raw.as_str()))
            .transpose()?,
        attempt_count: row
            .try_get::<i32, _>("attempt_count")?
            .max(0)
            .try_into()
            .unwrap_or(u32::MAX),
        transient_retry_count: row
            .try_get::<i32, _>("transient_retry_count")?
            .max(0)
            .try_into()
            .unwrap_or(u32::MAX),
        next_retry_at: row.try_get("next_retry_at")?,
        retryable: row.try_get("retryable")?,
        terminal_reason: row.try_get("terminal_reason")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

impl PostgresStore {
    async fn canonical_oauth_account_id_by_identity(
        &self,
        chatgpt_account_user_id: Option<&str>,
        chatgpt_user_id: Option<&str>,
        chatgpt_account_id: Option<&str>,
    ) -> Result<Option<Uuid>> {
        let normalized_account_user_id = chatgpt_account_user_id
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let normalized_user_id = chatgpt_user_id
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let normalized_account_id = chatgpt_account_id
            .map(str::trim)
            .filter(|value| !value.is_empty());

        if let Some(account_user_id) = normalized_account_user_id {
            return sqlx::query_scalar::<_, Uuid>(
                r#"
                SELECT a.id
                FROM upstream_accounts a
                LEFT JOIN upstream_account_oauth_credentials c ON c.account_id = a.id
                LEFT JOIN upstream_account_session_profiles p ON p.account_id = a.id
                WHERE
                    a.auth_provider = $1
                    AND p.chatgpt_account_user_id = $2
                ORDER BY
                    COALESCE(c.updated_at, p.updated_at, a.created_at) DESC,
                    a.created_at DESC,
                    a.id DESC
                LIMIT 1
                "#,
            )
            .bind(AUTH_PROVIDER_OAUTH_REFRESH_TOKEN)
            .bind(account_user_id)
            .fetch_optional(&self.pool)
            .await
            .context("failed to query oauth account by chatgpt_account_user_id");
        }

        let (Some(user_id), Some(account_id)) = (normalized_user_id, normalized_account_id) else {
            return Ok(None);
        };

        sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT a.id
            FROM upstream_accounts a
            LEFT JOIN upstream_account_oauth_credentials c ON c.account_id = a.id
            LEFT JOIN upstream_account_session_profiles p ON p.account_id = a.id
            WHERE
                a.auth_provider = $1
                AND p.chatgpt_user_id = $2
                AND a.chatgpt_account_id = $3
            ORDER BY
                COALESCE(c.updated_at, p.updated_at, a.created_at) DESC,
                a.created_at DESC,
                a.id DESC
            LIMIT 1
            "#,
        )
        .bind(AUTH_PROVIDER_OAUTH_REFRESH_TOKEN)
        .bind(user_id)
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to query oauth account by chatgpt_user_id + chatgpt_account_id")
    }

    async fn dedupe_oauth_accounts_by_identity_inner(
        &self,
        target_chatgpt_account_user_id: Option<&str>,
        target_chatgpt_user_id: Option<&str>,
        target_chatgpt_account_id: Option<&str>,
    ) -> Result<u64> {
        let normalized_target = target_chatgpt_account_user_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("account_user:{value}"))
            .or_else(|| {
                target_chatgpt_user_id
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .zip(
                        target_chatgpt_account_id
                            .map(str::trim)
                            .filter(|value| !value.is_empty()),
                    )
                    .map(|(user_id, account_id)| format!("user_account:{user_id}:{account_id}"))
            });
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start oauth duplicate cleanup transaction")?;

        let rows = sqlx::query(
            r#"
            WITH ranked AS (
                SELECT
                    a.id,
                    CASE
                        WHEN NULLIF(BTRIM(COALESCE(p.chatgpt_account_user_id, '')), '') IS NOT NULL
                            THEN 'account_user:' || BTRIM(p.chatgpt_account_user_id)
                        WHEN NULLIF(BTRIM(COALESCE(p.chatgpt_user_id, '')), '') IS NOT NULL
                             AND NULLIF(BTRIM(COALESCE(a.chatgpt_account_id, '')), '') IS NOT NULL
                            THEN 'user_account:' || BTRIM(p.chatgpt_user_id) || ':' || BTRIM(a.chatgpt_account_id)
                        ELSE NULL
                    END AS identity_key,
                    ROW_NUMBER() OVER (
                        PARTITION BY
                            CASE
                                WHEN NULLIF(BTRIM(COALESCE(p.chatgpt_account_user_id, '')), '') IS NOT NULL
                                    THEN 'account_user:' || BTRIM(p.chatgpt_account_user_id)
                                WHEN NULLIF(BTRIM(COALESCE(p.chatgpt_user_id, '')), '') IS NOT NULL
                                     AND NULLIF(BTRIM(COALESCE(a.chatgpt_account_id, '')), '') IS NOT NULL
                                    THEN 'user_account:' || BTRIM(p.chatgpt_user_id) || ':' || BTRIM(a.chatgpt_account_id)
                                ELSE NULL
                            END
                        ORDER BY COALESCE(c.updated_at, p.updated_at, a.created_at) DESC,
                                 a.created_at DESC,
                                 a.id DESC
                    ) AS duplicate_rank
                FROM upstream_accounts a
                LEFT JOIN upstream_account_oauth_credentials c ON c.account_id = a.id
                LEFT JOIN upstream_account_session_profiles p ON p.account_id = a.id
                WHERE
                    a.auth_provider = $1
                    AND (
                        NULLIF(BTRIM(COALESCE(p.chatgpt_account_user_id, '')), '') IS NOT NULL
                        OR (
                            NULLIF(BTRIM(COALESCE(p.chatgpt_user_id, '')), '') IS NOT NULL
                            AND NULLIF(BTRIM(COALESCE(a.chatgpt_account_id, '')), '') IS NOT NULL
                        )
                    )
            )
            DELETE FROM upstream_accounts doomed
            USING ranked
            WHERE doomed.id = ranked.id
              AND ranked.identity_key IS NOT NULL
              AND ($2::text IS NULL OR ranked.identity_key = $2)
              AND ranked.duplicate_rank > 1
            RETURNING doomed.id
            "#,
        )
        .bind(AUTH_PROVIDER_OAUTH_REFRESH_TOKEN)
        .bind(normalized_target)
        .fetch_all(tx.as_mut())
        .await
        .context("failed to delete duplicate oauth accounts by identity")?;

        if rows.is_empty() {
            tx.commit()
                .await
                .context("failed to commit oauth duplicate cleanup no-op transaction")?;
            return Ok(0);
        }

        for row in &rows {
            self.append_data_plane_outbox_event_tx(
                &mut tx,
                DataPlaneSnapshotEventType::AccountDelete,
                row.try_get::<Uuid, _>("id")?,
            )
            .await?;
        }
        self.bump_revision_tx(&mut tx).await?;
        tx.commit()
            .await
            .context("failed to commit oauth duplicate cleanup transaction")?;

        Ok(rows.len() as u64)
    }

    async fn prefill_oauth_rate_limits_after_upsert(
        &self,
        account_id: Uuid,
        access_token: &str,
        base_url: &str,
        chatgpt_account_id: Option<String>,
    ) {
        if access_token.trim().is_empty() {
            return;
        }
        let fetched_at = Utc::now();
        let usage = match self
            .oauth_client
            .fetch_usage(
                access_token,
                Some(base_url),
                chatgpt_account_id.as_deref(),
            )
            .await
        {
            Ok(usage) => usage,
            Err(err) => {
                tracing::warn!(
                    account_id = %account_id,
                    error = %err,
                    "best-effort oauth rate-limit prefill failed after upsert"
                );
                return;
            }
        };

        if let Err(err) = self
            .persist_rate_limit_cache_success(
                account_id,
                usage.rate_limits,
                fetched_at,
                usage.chatgpt_plan_type,
            )
            .await
        {
            tracing::warn!(
                account_id = %account_id,
                error = %err,
                "failed to persist oauth rate-limit prefill snapshot"
            );
        }
    }

    async fn upsert_session_profile_tx(
        &self,
        tx: &mut sqlx::PgConnection,
        account_id: Uuid,
        profile: &SessionProfileRecord,
    ) -> Result<()> {
        let organizations_json = profile
            .organizations
            .clone()
            .map(serde_json::Value::Array);
        let groups_json = profile.groups.clone().map(serde_json::Value::Array);

        sqlx::query(
            r#"
            INSERT INTO upstream_account_session_profiles (
                account_id,
                credential_kind,
                token_expires_at,
                email,
                oauth_subject,
                oauth_identity_provider,
                email_verified,
                chatgpt_plan_type,
                chatgpt_user_id,
                chatgpt_subscription_active_start,
                chatgpt_subscription_active_until,
                chatgpt_subscription_last_checked,
                chatgpt_account_user_id,
                chatgpt_compute_residency,
                workspace_name,
                organizations_json,
                groups_json,
                source_type,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
            ON CONFLICT (account_id) DO UPDATE
            SET
                credential_kind = EXCLUDED.credential_kind,
                token_expires_at = EXCLUDED.token_expires_at,
                email = COALESCE(EXCLUDED.email, upstream_account_session_profiles.email),
                oauth_subject = COALESCE(EXCLUDED.oauth_subject, upstream_account_session_profiles.oauth_subject),
                oauth_identity_provider = COALESCE(EXCLUDED.oauth_identity_provider, upstream_account_session_profiles.oauth_identity_provider),
                email_verified = COALESCE(EXCLUDED.email_verified, upstream_account_session_profiles.email_verified),
                chatgpt_plan_type = COALESCE(EXCLUDED.chatgpt_plan_type, upstream_account_session_profiles.chatgpt_plan_type),
                chatgpt_user_id = COALESCE(EXCLUDED.chatgpt_user_id, upstream_account_session_profiles.chatgpt_user_id),
                chatgpt_subscription_active_start = COALESCE(EXCLUDED.chatgpt_subscription_active_start, upstream_account_session_profiles.chatgpt_subscription_active_start),
                chatgpt_subscription_active_until = COALESCE(EXCLUDED.chatgpt_subscription_active_until, upstream_account_session_profiles.chatgpt_subscription_active_until),
                chatgpt_subscription_last_checked = COALESCE(EXCLUDED.chatgpt_subscription_last_checked, upstream_account_session_profiles.chatgpt_subscription_last_checked),
                chatgpt_account_user_id = COALESCE(EXCLUDED.chatgpt_account_user_id, upstream_account_session_profiles.chatgpt_account_user_id),
                chatgpt_compute_residency = COALESCE(EXCLUDED.chatgpt_compute_residency, upstream_account_session_profiles.chatgpt_compute_residency),
                workspace_name = COALESCE(EXCLUDED.workspace_name, upstream_account_session_profiles.workspace_name),
                organizations_json = COALESCE(EXCLUDED.organizations_json, upstream_account_session_profiles.organizations_json),
                groups_json = COALESCE(EXCLUDED.groups_json, upstream_account_session_profiles.groups_json),
                source_type = COALESCE(EXCLUDED.source_type, upstream_account_session_profiles.source_type),
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(account_id)
        .bind(session_credential_kind_to_db(&profile.credential_kind))
        .bind(profile.token_expires_at.as_ref().cloned())
        .bind(profile.email.clone())
        .bind(profile.oauth_subject.clone())
        .bind(profile.oauth_identity_provider.clone())
        .bind(profile.email_verified)
        .bind(profile.chatgpt_plan_type.clone())
        .bind(profile.chatgpt_user_id.clone())
        .bind(profile.chatgpt_subscription_active_start.as_ref().cloned())
        .bind(profile.chatgpt_subscription_active_until.as_ref().cloned())
        .bind(profile.chatgpt_subscription_last_checked.as_ref().cloned())
        .bind(profile.chatgpt_account_user_id.clone())
        .bind(profile.chatgpt_compute_residency.clone())
        .bind(profile.workspace_name.clone())
        .bind(organizations_json)
        .bind(groups_json)
        .bind(profile.source_type.clone())
        .bind(Utc::now())
        .execute(tx)
        .await
        .context("failed to upsert session profile")?;

        Ok(())
    }

    async fn purge_expired_one_time_accounts_inner(&self) -> Result<u64> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start one-time account purge transaction")?;
        let now = Utc::now() + Duration::seconds(OAUTH_MIN_VALID_SEC);
        let rows = sqlx::query(
            r#"
            DELETE FROM upstream_accounts a
            USING upstream_account_session_profiles p
            WHERE
                a.id = p.account_id
                AND p.credential_kind = $1
                AND p.token_expires_at IS NOT NULL
                AND p.token_expires_at <= $2
            RETURNING a.id
            "#,
        )
        .bind(SESSION_CREDENTIAL_KIND_ONE_TIME_ACCESS_TOKEN)
        .bind(now)
        .fetch_all(tx.as_mut())
        .await
        .context("failed to purge expired one-time accounts")?;
        let deleted = u64::try_from(rows.len()).unwrap_or(u64::MAX);
        if deleted > 0 {
            for row in rows {
                self.append_data_plane_outbox_event_tx(
                    &mut tx,
                    DataPlaneSnapshotEventType::AccountDelete,
                    row.try_get::<Uuid, _>("id")?,
                )
                .await?;
            }
            self.bump_revision_tx(&mut tx).await?;
        }
        tx.commit()
            .await
            .context("failed to commit one-time account purge transaction")?;
        Ok(deleted)
    }

    async fn validate_oauth_refresh_token_inner(
        &self,
        req: ValidateOAuthRefreshTokenRequest,
    ) -> Result<ValidateOAuthRefreshTokenResponse> {
        let token_info = self
            .oauth_client
            .refresh_token(&req.refresh_token, req.base_url.as_deref())
            .await
            .map_err(|err| anyhow!(err.to_string()))?;

        Ok(ValidateOAuthRefreshTokenResponse {
            expires_at: token_info.expires_at,
            token_type: token_info.token_type,
            scope: token_info.scope,
            chatgpt_account_id: token_info.chatgpt_account_id,
            chatgpt_user_id: token_info.chatgpt_user_id,
            chatgpt_account_user_id: token_info.chatgpt_account_user_id,
        })
    }

    async fn fetch_oauth_vault_record_inner(
        &self,
        record_id: Uuid,
    ) -> Result<Option<OAuthRefreshTokenVaultRecord>> {
        let row = sqlx::query(
            r#"
            SELECT
                id,
                label,
                email,
                base_url,
                refresh_token_enc,
                fallback_access_token_enc,
                fallback_token_expires_at,
                refresh_token_sha256,
                chatgpt_account_id,
                chatgpt_plan_type,
                source_type,
                desired_mode,
                desired_enabled,
                desired_priority,
                status,
                failure_count,
                backoff_until,
                next_attempt_at,
                last_error_code,
                last_error_message,
                admission_source,
                admission_checked_at,
                admission_retry_after,
                admission_error_code,
                admission_error_message,
                admission_rate_limits_json::text AS admission_rate_limits_json_text,
                admission_rate_limits_expires_at,
                failure_stage,
                attempt_count,
                transient_retry_count,
                next_retry_at,
                retryable,
                terminal_reason,
                created_at,
                updated_at
            FROM oauth_refresh_token_vault
            WHERE id = $1
            "#,
        )
        .bind(record_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to fetch oauth vault record")?;

        row.as_ref().map(parse_oauth_vault_record_row).transpose()
    }

    #[allow(clippy::too_many_arguments)]
    async fn update_oauth_vault_admission_result(
        &self,
        record_id: Uuid,
        status: OAuthVaultRecordStatus,
        checked_at: DateTime<Utc>,
        admission_source: Option<String>,
        retry_after: Option<DateTime<Utc>>,
        error_code: Option<String>,
        error_message: Option<String>,
        fallback_token_expires_at: Option<DateTime<Utc>>,
        rate_limits: Vec<OAuthRateLimitSnapshot>,
        rate_limits_expires_at: Option<DateTime<Utc>>,
        failure_stage: Option<OAuthInventoryFailureStage>,
        attempt_count_delta: u32,
        transient_retry_count_delta: u32,
        next_retry_at: Option<DateTime<Utc>>,
        retryable: bool,
        terminal_reason: Option<String>,
    ) -> Result<()> {
        let rate_limits_json = if rate_limits.is_empty() {
            None
        } else {
            Some(
                serde_json::to_string(&rate_limits)
                    .context("failed to encode oauth vault admission rate limits")?,
            )
        };
        sqlx::query(
            r#"
            UPDATE oauth_refresh_token_vault
            SET
                status = $2,
                admission_source = $3,
                admission_checked_at = $4,
                admission_retry_after = $5,
                admission_error_code = $6,
                admission_error_message = $7,
                fallback_token_expires_at = COALESCE($8, fallback_token_expires_at),
                admission_rate_limits_json = $9::jsonb,
                admission_rate_limits_expires_at = $10,
                failure_stage = $11,
                attempt_count = attempt_count + $12,
                transient_retry_count = transient_retry_count + $13,
                next_retry_at = $14,
                retryable = $15,
                terminal_reason = $16,
                updated_at = $4
            WHERE id = $1
            "#,
        )
        .bind(record_id)
        .bind(oauth_vault_status_to_db(status))
        .bind(admission_source)
        .bind(checked_at)
        .bind(retry_after)
        .bind(error_code)
        .bind(error_message.map(truncate_error_message))
        .bind(fallback_token_expires_at)
        .bind(rate_limits_json)
        .bind(rate_limits_expires_at)
        .bind(failure_stage.map(oauth_inventory_failure_stage_to_db))
        .bind(i32::try_from(attempt_count_delta).unwrap_or(i32::MAX))
        .bind(i32::try_from(transient_retry_count_delta).unwrap_or(i32::MAX))
        .bind(next_retry_at)
        .bind(retryable)
        .bind(terminal_reason)
        .execute(&self.pool)
        .await
        .context("failed to update oauth vault admission result")?;
        Ok(())
    }

    async fn mark_oauth_inventory_record_failed_inner(
        &self,
        record_id: Uuid,
        reason: Option<String>,
    ) -> Result<()> {
        let existing = self.fetch_oauth_vault_record_inner(record_id).await?;
        let Some(existing) = existing else {
            return Err(anyhow!("oauth inventory record not found"));
        };
        let checked_at = Utc::now();
        let delete_due_at = checked_at + Duration::seconds(pending_purge_delay_sec_from_env());
        let normalized_reason = reason
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .or(existing.terminal_reason)
            .or(existing.admission_error_code);

        sqlx::query(
            r#"
            UPDATE oauth_refresh_token_vault
            SET
                status = $2,
                admission_checked_at = $3,
                admission_retry_after = NULL,
                next_retry_at = $4,
                retryable = FALSE,
                terminal_reason = $5,
                updated_at = $3
            WHERE id = $1
            "#,
        )
        .bind(record_id)
        .bind(oauth_vault_status_to_db(OAuthVaultRecordStatus::Failed))
        .bind(checked_at)
        .bind(delete_due_at)
        .bind(normalized_reason)
        .execute(&self.pool)
        .await
        .context("failed to mark oauth inventory record failed")?;
        Ok(())
    }

    async fn delete_oauth_inventory_record_inner(&self, record_id: Uuid) -> Result<()> {
        let deleted = sqlx::query(
            r#"
            DELETE FROM oauth_refresh_token_vault
            WHERE id = $1
            "#,
        )
        .bind(record_id)
        .execute(&self.pool)
        .await
        .context("failed to delete oauth inventory record")?
        .rows_affected();

        if deleted == 0 {
            return Err(anyhow!("oauth inventory record not found"));
        }
        Ok(())
    }

    async fn restore_oauth_inventory_record_inner(&self, record_id: Uuid) -> Result<()> {
        let now = Utc::now();
        let updated = sqlx::query(
            r#"
            UPDATE oauth_refresh_token_vault
            SET
                status = $2,
                failure_count = 0,
                backoff_until = NULL,
                next_attempt_at = $3,
                last_error_code = NULL,
                last_error_message = NULL,
                admission_source = NULL,
                admission_checked_at = NULL,
                admission_retry_after = NULL,
                admission_error_code = NULL,
                admission_error_message = NULL,
                admission_rate_limits_json = NULL,
                admission_rate_limits_expires_at = NULL,
                failure_stage = NULL,
                attempt_count = 0,
                transient_retry_count = 0,
                next_retry_at = $3,
                retryable = TRUE,
                terminal_reason = NULL,
                updated_at = $3
            WHERE id = $1
            "#,
        )
        .bind(record_id)
        .bind(oauth_vault_status_to_db(OAuthVaultRecordStatus::Queued))
        .bind(now)
        .execute(&self.pool)
        .await
        .context("failed to restore oauth inventory record")?
        .rows_affected();

        if updated == 0 {
            return Err(anyhow!("oauth inventory record not found"));
        }
        Ok(())
    }

    async fn reprobe_oauth_inventory_record_inner(&self, record_id: Uuid) -> Result<()> {
        self.restore_oauth_inventory_record_inner(record_id).await?;
        self.probe_oauth_vault_admission_inner(record_id).await
    }

    async fn purge_due_oauth_inventory_records_inner(&self) -> Result<u64> {
        let deleted = sqlx::query(
            r#"
            DELETE FROM oauth_refresh_token_vault
            WHERE status = $1
              AND retryable = FALSE
              AND COALESCE(next_retry_at, updated_at + make_interval(secs => $2)) <= NOW()
            "#,
        )
        .bind(oauth_vault_status_to_db(OAuthVaultRecordStatus::Failed))
        .bind(pending_purge_delay_sec_from_env())
        .execute(&self.pool)
        .await
        .context("failed to purge due oauth inventory records")?
        .rows_affected();
        Ok(deleted)
    }

    async fn probe_oauth_vault_admission_inner(&self, record_id: Uuid) -> Result<()> {
        let Some(record) = self.fetch_oauth_vault_record_inner(record_id).await? else {
            return Ok(());
        };
        let checked_at = Utc::now();

        let Some(cipher) = self.credential_cipher.as_ref() else {
            self.update_oauth_vault_admission_result(
                record_id,
                OAuthVaultRecordStatus::Failed,
                checked_at,
                None,
                None,
                Some("credential_cipher_missing".to_string()),
                Some("oauth credential cipher is not configured".to_string()),
                None,
                Vec::new(),
                None,
                Some(OAuthInventoryFailureStage::AdmissionProbe),
                1,
                0,
                None,
                false,
                Some("credential_cipher_missing".to_string()),
            )
            .await?;
            return Ok(());
        };

        let Some(fallback_token_enc) = record.fallback_access_token_enc.as_deref() else {
            self.update_oauth_vault_admission_result(
                record_id,
                OAuthVaultRecordStatus::NeedsRefresh,
                checked_at,
                None,
                None,
                Some("missing_access_token_fallback".to_string()),
                Some("fallback access token is not available".to_string()),
                None,
                Vec::new(),
                None,
                Some(OAuthInventoryFailureStage::AdmissionProbe),
                1,
                0,
                None,
                true,
                None,
            )
            .await?;
            return Ok(());
        };

        let fallback_access_token = match cipher.decrypt(fallback_token_enc) {
            Ok(value) if !value.trim().is_empty() => value,
            Ok(_) => {
                self.update_oauth_vault_admission_result(
                    record_id,
                    OAuthVaultRecordStatus::Failed,
                    checked_at,
                    Some("fallback_access_token".to_string()),
                    None,
                    Some("credential_decrypt_failed".to_string()),
                    Some("fallback access token is empty".to_string()),
                    None,
                    Vec::new(),
                    None,
                    Some(OAuthInventoryFailureStage::AdmissionProbe),
                    1,
                    0,
                    None,
                    false,
                    Some("credential_decrypt_failed".to_string()),
                )
                .await?;
                return Ok(());
            }
            Err(err) => {
                self.update_oauth_vault_admission_result(
                    record_id,
                    OAuthVaultRecordStatus::Failed,
                    checked_at,
                    Some("fallback_access_token".to_string()),
                    None,
                    Some("credential_decrypt_failed".to_string()),
                    Some(err.to_string()),
                    None,
                    Vec::new(),
                    None,
                    Some(OAuthInventoryFailureStage::AdmissionProbe),
                    1,
                    0,
                    None,
                    false,
                    Some("credential_decrypt_failed".to_string()),
                )
                .await?;
                return Ok(());
            }
        };

        let fallback_expires_at = record
            .fallback_token_expires_at
            .or_else(|| parse_jwt_exp_from_access_token(&fallback_access_token));

        match self
            .oauth_client
            .fetch_rate_limits(
                &fallback_access_token,
                Some(&record.base_url),
                record.chatgpt_account_id.as_deref(),
            )
            .await
        {
            Ok(rate_limits) => {
                let rate_limits_expires_at =
                    derive_admission_rate_limits_expires_at(&rate_limits, checked_at);
                let (blocked_until, block_reason) =
                    derive_rate_limit_block(&rate_limits, checked_at);
                if fallback_expires_at.is_none() {
                    self.update_oauth_vault_admission_result(
                        record_id,
                        OAuthVaultRecordStatus::NeedsRefresh,
                        checked_at,
                        Some("fallback_access_token".to_string()),
                        None,
                        Some("expiry_unknown".to_string()),
                        Some("fallback access token expiry is unknown".to_string()),
                        None,
                        rate_limits,
                        rate_limits_expires_at,
                        Some(OAuthInventoryFailureStage::AdmissionProbe),
                        1,
                        0,
                        None,
                        true,
                        None,
                    )
                    .await?;
                    return Ok(());
                }
                if let Some(block_reason) = block_reason {
                    self.update_oauth_vault_admission_result(
                        record_id,
                        OAuthVaultRecordStatus::NoQuota,
                        checked_at,
                        Some("fallback_access_token".to_string()),
                        blocked_until,
                        Some(block_reason.clone()),
                        Some(rate_limit_block_message(&block_reason)),
                        fallback_expires_at,
                        rate_limits,
                        rate_limits_expires_at,
                        Some(OAuthInventoryFailureStage::AdmissionProbe),
                        1,
                        0,
                        blocked_until,
                        true,
                        None,
                    )
                    .await?;
                    return Ok(());
                }

                self.update_oauth_vault_admission_result(
                    record_id,
                    OAuthVaultRecordStatus::Ready,
                    checked_at,
                    Some("fallback_access_token".to_string()),
                    None,
                    None,
                    None,
                    fallback_expires_at,
                    rate_limits,
                    rate_limits_expires_at,
                    None,
                    1,
                    0,
                    None,
                    false,
                    None,
                )
                .await?;
                Ok(())
            }
            Err(err) => {
                let error_code = err.code().as_str().to_string();
                let error_message = truncate_error_message(err.to_string());
                let transient_signal =
                    is_transient_upstream_error_signal(&error_code, &error_message);
                let current_transient_retry_count = record.transient_retry_count;
                let retry_after = if transient_signal
                    && can_retry_transient_admission_failure(current_transient_retry_count)
                {
                    admission_probe_retry_after_with_budget(
                        checked_at,
                        &error_code,
                        &error_message,
                        current_transient_retry_count,
                    )
                } else if transient_signal {
                    None
                } else {
                    admission_probe_retry_after_with_budget(
                        checked_at,
                        &error_code,
                        &error_message,
                        current_transient_retry_count,
                    )
                };
                let fatal_auth = is_fatal_refresh_error_code(Some(error_code.as_str()));
                let status = if fatal_auth {
                    OAuthVaultRecordStatus::Failed
                } else if retry_after.is_some()
                    && (is_quota_error_signal(&error_code, &error_message)
                        || is_rate_limited_signal(&error_code, &error_message))
                {
                    OAuthVaultRecordStatus::NoQuota
                } else if is_auth_error_signal(&error_code, &error_message) {
                    OAuthVaultRecordStatus::NeedsRefresh
                } else {
                    OAuthVaultRecordStatus::Failed
                };
                let retryable = match status {
                    OAuthVaultRecordStatus::Ready => false,
                    OAuthVaultRecordStatus::NeedsRefresh => true,
                    OAuthVaultRecordStatus::NoQuota => retry_after.is_some(),
                    OAuthVaultRecordStatus::Failed => retry_after.is_some() && !fatal_auth,
                    OAuthVaultRecordStatus::Queued => false,
                };
                let terminal_reason = if retryable {
                    None
                } else {
                    Some(error_code.clone())
                };
                self.update_oauth_vault_admission_result(
                    record_id,
                    status,
                    checked_at,
                    Some("fallback_access_token".to_string()),
                    retry_after,
                    Some(error_code),
                    Some(error_message),
                    fallback_expires_at,
                    Vec::new(),
                    retry_after,
                    Some(OAuthInventoryFailureStage::AdmissionProbe),
                    1,
                    if transient_signal && retry_after.is_some() { 1 } else { 0 },
                    retry_after,
                    retryable,
                    terminal_reason,
                )
                .await?;
                Ok(())
            }
        }
    }

    async fn oauth_inventory_summary_inner(&self) -> Result<OAuthInventorySummaryResponse> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*)::BIGINT AS total,
                COUNT(*) FILTER (WHERE status = $1)::BIGINT AS queued,
                COUNT(*) FILTER (WHERE status = $2)::BIGINT AS ready,
                COUNT(*) FILTER (WHERE status = $3)::BIGINT AS needs_refresh,
                COUNT(*) FILTER (WHERE status = $4)::BIGINT AS no_quota,
                COUNT(*) FILTER (WHERE status = $5)::BIGINT AS failed
            FROM oauth_refresh_token_vault
            "#,
        )
        .bind(oauth_vault_status_to_db(OAuthVaultRecordStatus::Queued))
        .bind(oauth_vault_status_to_db(OAuthVaultRecordStatus::Ready))
        .bind(oauth_vault_status_to_db(OAuthVaultRecordStatus::NeedsRefresh))
        .bind(oauth_vault_status_to_db(OAuthVaultRecordStatus::NoQuota))
        .bind(oauth_vault_status_to_db(OAuthVaultRecordStatus::Failed))
        .fetch_one(&self.pool)
        .await
        .context("failed to load oauth inventory summary")?;

        Ok(OAuthInventorySummaryResponse {
            total: row
                .try_get::<i64, _>("total")?
                .max(0)
                .try_into()
                .unwrap_or(u64::MAX),
            queued: row
                .try_get::<i64, _>("queued")?
                .max(0)
                .try_into()
                .unwrap_or(u64::MAX),
            ready: row
                .try_get::<i64, _>("ready")?
                .max(0)
                .try_into()
                .unwrap_or(u64::MAX),
            needs_refresh: row
                .try_get::<i64, _>("needs_refresh")?
                .max(0)
                .try_into()
                .unwrap_or(u64::MAX),
            no_quota: row
                .try_get::<i64, _>("no_quota")?
                .max(0)
                .try_into()
                .unwrap_or(u64::MAX),
            failed: row
                .try_get::<i64, _>("failed")?
                .max(0)
                .try_into()
                .unwrap_or(u64::MAX),
        })
    }

    async fn oauth_inventory_records_inner(&self) -> Result<Vec<OAuthInventoryRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id,
                label,
                email,
                fallback_access_token_enc,
                chatgpt_account_id,
                chatgpt_plan_type,
                source_type,
                status,
                admission_source,
                admission_checked_at,
                admission_retry_after,
                admission_error_code,
                admission_error_message,
                admission_rate_limits_json::text AS admission_rate_limits_json_text,
                admission_rate_limits_expires_at,
                failure_stage,
                attempt_count,
                transient_retry_count,
                next_retry_at,
                retryable,
                terminal_reason,
                created_at,
                updated_at
            FROM oauth_refresh_token_vault
            ORDER BY created_at ASC, id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to load oauth inventory records")?;

        rows.iter().map(parse_oauth_inventory_record_row).collect()
    }

    async fn queue_oauth_refresh_token_vault_inner(
        &self,
        req: ImportOAuthRefreshTokenRequest,
    ) -> Result<bool> {
        let cipher = self.require_credential_cipher()?;
        let refresh_token = req.refresh_token.trim();
        if refresh_token.is_empty() {
            return Err(anyhow!("refresh token is empty"));
        }
        let desired_mode = resolve_oauth_import_mode(req.mode.clone(), req.source_type.as_deref());
        let normalized_base_url = normalize_upstream_account_base_url(&desired_mode, &req.base_url);
        let refresh_token_enc = cipher.encrypt(refresh_token)?;
        let fallback_access_token_enc = req
            .fallback_access_token
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| cipher.encrypt(value))
            .transpose()?;
        let refresh_token_sha256 = refresh_token_sha256(refresh_token);
        let now = Utc::now();
        let resolved_chatgpt_account_id = req
            .chatgpt_account_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned);

        let row = sqlx::query(
            r#"
            INSERT INTO oauth_refresh_token_vault (
                id,
                refresh_token_enc,
                refresh_token_sha256,
                fallback_access_token_enc,
                fallback_token_expires_at,
                base_url,
                label,
                email,
                chatgpt_account_id,
                chatgpt_plan_type,
                source_type,
                desired_mode,
                desired_enabled,
                desired_priority,
                status,
                failure_count,
                backoff_until,
                next_attempt_at,
                last_error_code,
                last_error_message,
                admission_source,
                admission_checked_at,
                admission_retry_after,
                admission_error_code,
                admission_error_message,
                admission_rate_limits_json,
                admission_rate_limits_expires_at,
                failure_stage,
                attempt_count,
                transient_retry_count,
                next_retry_at,
                retryable,
                terminal_reason,
                created_at,
                updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, NULL, $8, $9, $10, $11, $12, $13, $14, 0, NULL, $15, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, $15, true, NULL, $15, $15
            )
            ON CONFLICT (refresh_token_sha256) DO UPDATE
            SET
                refresh_token_enc = EXCLUDED.refresh_token_enc,
                fallback_access_token_enc = COALESCE(
                    EXCLUDED.fallback_access_token_enc,
                    oauth_refresh_token_vault.fallback_access_token_enc
                ),
                fallback_token_expires_at = COALESCE(
                    EXCLUDED.fallback_token_expires_at,
                    oauth_refresh_token_vault.fallback_token_expires_at
                ),
                base_url = EXCLUDED.base_url,
                label = EXCLUDED.label,
                chatgpt_account_id = COALESCE(EXCLUDED.chatgpt_account_id, oauth_refresh_token_vault.chatgpt_account_id),
                chatgpt_plan_type = COALESCE(EXCLUDED.chatgpt_plan_type, oauth_refresh_token_vault.chatgpt_plan_type),
                source_type = COALESCE(EXCLUDED.source_type, oauth_refresh_token_vault.source_type),
                desired_mode = EXCLUDED.desired_mode,
                desired_enabled = EXCLUDED.desired_enabled,
                desired_priority = EXCLUDED.desired_priority,
                status = $14,
                failure_count = 0,
                backoff_until = NULL,
                next_attempt_at = EXCLUDED.next_attempt_at,
                last_error_code = NULL,
                last_error_message = NULL,
                admission_source = NULL,
                admission_checked_at = NULL,
                admission_retry_after = NULL,
                admission_error_code = NULL,
                admission_error_message = NULL,
                admission_rate_limits_json = NULL,
                admission_rate_limits_expires_at = NULL,
                failure_stage = NULL,
                attempt_count = 0,
                transient_retry_count = 0,
                next_retry_at = EXCLUDED.next_retry_at,
                retryable = true,
                terminal_reason = NULL,
                updated_at = EXCLUDED.updated_at
            RETURNING id, (xmax = 0) AS inserted
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(refresh_token_enc)
        .bind(refresh_token_sha256)
        .bind(fallback_access_token_enc)
        .bind(req.fallback_token_expires_at)
        .bind(&normalized_base_url)
        .bind(&req.label)
        .bind(resolved_chatgpt_account_id)
        .bind(req.chatgpt_plan_type.clone())
        .bind(req.source_type.clone())
        .bind(upstream_mode_to_db(&desired_mode))
        .bind(req.enabled.unwrap_or(true))
        .bind(req.priority.unwrap_or(100))
        .bind(oauth_vault_status_to_db(OAuthVaultRecordStatus::Queued))
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .context("failed to queue oauth refresh token into vault")?;

        let record_id = row
            .try_get::<Uuid, _>("id")
            .context("failed to read oauth vault record id")?;
        let inserted = row
            .try_get::<bool, _>("inserted")
            .context("failed to read vault inserted flag")?;
        self.probe_oauth_vault_admission_inner(record_id).await?;
        Ok(inserted)
    }

    async fn matched_oauth_account_id_for_vault_record(
        &self,
        record: &OAuthRefreshTokenVaultRecord,
    ) -> Result<Option<Uuid>> {
        sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT c.account_id
            FROM upstream_account_oauth_credentials c
            INNER JOIN upstream_accounts a ON a.id = c.account_id
            WHERE
                a.auth_provider = $1
                AND c.refresh_token_sha256 = $2
            ORDER BY a.created_at DESC, a.id DESC
            LIMIT 1
            "#,
        )
        .bind(AUTH_PROVIDER_OAUTH_REFRESH_TOKEN)
        .bind(&record.refresh_token_sha256)
        .fetch_optional(&self.pool)
        .await
        .context("failed to query oauth account by vault refresh token hash")
    }

    fn ready_session_profile_from_vault_record(
        record: &OAuthRefreshTokenVaultRecord,
        token_expires_at: DateTime<Utc>,
    ) -> SessionProfileRecord {
        SessionProfileRecord {
            credential_kind: SessionCredentialKind::RefreshRotatable,
            token_expires_at: Some(token_expires_at),
            email: record.email.clone(),
            oauth_subject: None,
            oauth_identity_provider: None,
            email_verified: None,
            chatgpt_plan_type: record.chatgpt_plan_type.clone(),
            chatgpt_user_id: None,
            chatgpt_subscription_active_start: None,
            chatgpt_subscription_active_until: None,
            chatgpt_subscription_last_checked: None,
            chatgpt_account_user_id: None,
            chatgpt_compute_residency: None,
            workspace_name: None,
            organizations: None,
            groups: None,
            source_type: record.source_type.clone(),
        }
    }

    async fn materialize_ready_oauth_vault_record_inner(
        &self,
        record: &OAuthRefreshTokenVaultRecord,
    ) -> Result<()> {
        let cipher = self.require_credential_cipher()?;
        let access_token_enc = record
            .fallback_access_token_enc
            .clone()
            .ok_or_else(|| anyhow!("ready access token is missing"))?;
        let token_expires_at = match record.fallback_token_expires_at {
            Some(expires_at) => expires_at,
            None => {
                let fallback_access_token = cipher
                    .decrypt(&access_token_enc)
                    .map_err(|err| anyhow!(truncate_error_message(err.to_string())))?;
                parse_jwt_exp_from_access_token(&fallback_access_token)
                    .ok_or_else(|| anyhow!("ready access token expiry is unknown"))?
            }
        };
        let now = Utc::now();
        if token_expires_at <= now + Duration::seconds(OAUTH_MIN_VALID_SEC) {
            return Err(anyhow!("ready access token is already expired"));
        }

        let account_id = self
            .matched_oauth_account_id_for_vault_record(record)
            .await?
            .unwrap_or_else(Uuid::new_v4);
        let existing = sqlx::query(
            r#"
            SELECT id, created_at
            FROM upstream_accounts
            WHERE id = $1
            "#,
        )
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to query existing oauth account for ready materialization")?;
        let created_at = existing
            .as_ref()
            .map(|row| row.try_get::<DateTime<Utc>, _>("created_at"))
            .transpose()?
            .unwrap_or(now);
        let enabled = record.desired_enabled;
        let priority = record.desired_priority;
        let mode = upstream_mode_to_db(&record.desired_mode);
        let next_refresh_at = schedule_next_oauth_refresh(token_expires_at, account_id);
        let token_family_id = stable_vault_token_family_id(&record.refresh_token_sha256);

        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start ready oauth vault materialization transaction")?;

        sqlx::query(
            r#"
            INSERT INTO upstream_accounts (
                id,
                label,
                mode,
                base_url,
                bearer_token,
                chatgpt_account_id,
                auth_provider,
                enabled,
                pool_state,
                priority,
                created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (id) DO UPDATE
            SET
                label = EXCLUDED.label,
                mode = EXCLUDED.mode,
                base_url = EXCLUDED.base_url,
                bearer_token = EXCLUDED.bearer_token,
                chatgpt_account_id = EXCLUDED.chatgpt_account_id,
                auth_provider = EXCLUDED.auth_provider,
                enabled = EXCLUDED.enabled,
                pool_state = EXCLUDED.pool_state,
                priority = EXCLUDED.priority
            "#,
        )
        .bind(account_id)
        .bind(&record.label)
        .bind(mode)
        .bind(&record.base_url)
        .bind(OAUTH_MANAGED_BEARER_SENTINEL)
        .bind(record.chatgpt_account_id.clone())
        .bind(AUTH_PROVIDER_OAUTH_REFRESH_TOKEN)
        .bind(enabled)
        .bind(POOL_STATE_ACTIVE)
        .bind(priority)
        .bind(created_at)
        .execute(tx.as_mut())
        .await
        .context("failed to upsert ready oauth upstream account")?;

        sqlx::query(
            r#"
            INSERT INTO upstream_account_oauth_credentials (
                account_id,
                access_token_enc,
                fallback_access_token_enc,
                refresh_token_enc,
                refresh_token_sha256,
                token_family_id,
                token_version,
                token_expires_at,
                fallback_token_expires_at,
                last_refresh_at,
                last_refresh_status,
                last_refresh_error_code,
                last_refresh_error,
                refresh_failure_count,
                refresh_backoff_until,
                refresh_reused_detected,
                refresh_inflight_until,
                next_refresh_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $7, NULL, 'never', NULL, NULL, 0, NULL, false, NULL, $8, $9)
            ON CONFLICT (account_id) DO UPDATE
            SET
                access_token_enc = EXCLUDED.access_token_enc,
                fallback_access_token_enc = EXCLUDED.fallback_access_token_enc,
                refresh_token_enc = EXCLUDED.refresh_token_enc,
                refresh_token_sha256 = EXCLUDED.refresh_token_sha256,
                token_family_id = COALESCE(
                    upstream_account_oauth_credentials.token_family_id,
                    EXCLUDED.token_family_id
                ),
                token_version = GREATEST(upstream_account_oauth_credentials.token_version, 0),
                token_expires_at = EXCLUDED.token_expires_at,
                fallback_token_expires_at = EXCLUDED.fallback_token_expires_at,
                last_refresh_at = NULL,
                last_refresh_status = 'never',
                last_refresh_error_code = NULL,
                last_refresh_error = NULL,
                refresh_failure_count = 0,
                refresh_backoff_until = NULL,
                refresh_reused_detected = false,
                refresh_inflight_until = NULL,
                next_refresh_at = EXCLUDED.next_refresh_at,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(account_id)
        .bind(access_token_enc.clone())
        .bind(Some(access_token_enc.clone()))
        .bind(record.refresh_token_enc.clone())
        .bind(&record.refresh_token_sha256)
        .bind(token_family_id)
        .bind(token_expires_at)
        .bind(next_refresh_at)
        .bind(now)
        .execute(tx.as_mut())
        .await
        .context("failed to upsert ready oauth credential row")?;

        let profile = Self::ready_session_profile_from_vault_record(record, token_expires_at);
        self.upsert_session_profile_tx(tx.as_mut(), account_id, &profile)
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
            .context("failed to commit ready oauth vault materialization transaction")?;

        if !record.admission_rate_limits.is_empty() {
            if let Err(err) = self
                .persist_rate_limit_cache_success(
                    account_id,
                    record.admission_rate_limits.clone(),
                    record.admission_checked_at.unwrap_or(now),
                    record.chatgpt_plan_type.clone(),
                )
                .await
            {
                tracing::warn!(
                    account_id = %account_id,
                    error = %err,
                    "failed to persist ready oauth admission rate-limit cache"
                );
            }
        }

        Ok(())
    }

    async fn insert_oauth_account(
        &self,
        req: ImportOAuthRefreshTokenRequest,
    ) -> Result<UpstreamAccount> {
        let cipher = self.require_credential_cipher()?;
        let resolved_mode = resolve_oauth_import_mode(req.mode.clone(), req.source_type.as_deref());
        let normalized_base_url =
            normalize_upstream_account_base_url(&resolved_mode, &req.base_url);
        let token_info = self
            .oauth_client
            .refresh_token(&req.refresh_token, Some(&normalized_base_url))
            .await
            .map_err(|err| anyhow!(err.to_string()))?;

        let access_token_enc = cipher.encrypt(&token_info.access_token)?;
        let fallback_access_token_enc = req
            .fallback_access_token
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| cipher.encrypt(value))
            .transpose()?;
        let refresh_token_enc = cipher.encrypt(&token_info.refresh_token)?;
        let refresh_token_sha256 = refresh_token_sha256(&token_info.refresh_token);
        let token_family_id = Uuid::new_v4().to_string();
        let resolved_chatgpt_account_id = req
            .chatgpt_account_id
            .clone()
            .or(token_info.chatgpt_account_id.clone());
        let resolved_chatgpt_plan_type = req
            .chatgpt_plan_type
            .clone()
            .or(token_info.chatgpt_plan_type.clone());
        let base_url_for_rate_limit = normalized_base_url.clone();
        let account_id = Uuid::new_v4();
        let enabled = req.enabled.unwrap_or(true);
        let priority = req.priority.unwrap_or(100);
        let created_at = Utc::now();
        let updated_at = Utc::now();
        let mode = upstream_mode_to_db(&resolved_mode);

        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start oauth account transaction")?;

        let row = sqlx::query(
            r#"
            INSERT INTO upstream_accounts (
                id,
                label,
                mode,
                base_url,
                bearer_token,
                chatgpt_account_id,
                auth_provider,
                enabled,
                pool_state,
                priority,
                created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id, label, mode, base_url, bearer_token, chatgpt_account_id, enabled, priority, created_at
            "#,
        )
        .bind(account_id)
        .bind(req.label)
        .bind(mode)
        .bind(&normalized_base_url)
        .bind(OAUTH_MANAGED_BEARER_SENTINEL)
        .bind(resolved_chatgpt_account_id.clone())
        .bind(AUTH_PROVIDER_OAUTH_REFRESH_TOKEN)
        .bind(enabled)
        .bind(POOL_STATE_ACTIVE)
        .bind(priority)
        .bind(created_at)
        .fetch_one(tx.as_mut())
        .await
        .context("failed to insert oauth upstream account")?;

        sqlx::query(
            r#"
            INSERT INTO upstream_account_oauth_credentials (
                account_id,
                access_token_enc,
                fallback_access_token_enc,
                refresh_token_enc,
                refresh_token_sha256,
                token_family_id,
                token_version,
                token_expires_at,
                fallback_token_expires_at,
                last_refresh_at,
                last_refresh_status,
                last_refresh_error_code,
                last_refresh_error,
                refresh_failure_count,
                refresh_backoff_until,
                refresh_reused_detected,
                refresh_inflight_until,
                next_refresh_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, 1, $7, $8, $9, 'ok', NULL, NULL, 0, NULL, false, NULL, $10, $11)
            "#,
        )
        .bind(account_id)
        .bind(access_token_enc)
        .bind(fallback_access_token_enc)
        .bind(refresh_token_enc)
        .bind(refresh_token_sha256)
        .bind(token_family_id)
        .bind(token_info.expires_at)
        .bind(req.fallback_token_expires_at)
        .bind(Utc::now())
        .bind(schedule_next_oauth_refresh(token_info.expires_at, account_id))
        .bind(updated_at)
        .execute(tx.as_mut())
        .await
        .context("failed to insert oauth credential")?;

        let session_profile = SessionProfileRecord::from_oauth_token_info(
            &token_info,
            SessionCredentialKind::RefreshRotatable,
            resolved_chatgpt_plan_type.clone(),
            req.source_type.clone(),
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
            .context("failed to commit oauth account transaction")?;
        self.prefill_oauth_rate_limits_after_upsert(
            account_id,
            &token_info.access_token,
            &base_url_for_rate_limit,
            resolved_chatgpt_account_id,
        )
        .await;

        Ok(UpstreamAccount {
            id: row.try_get("id")?,
            label: row.try_get("label")?,
            mode: parse_upstream_mode(row.try_get::<String, _>("mode")?.as_str())?,
            base_url: row.try_get("base_url")?,
            bearer_token: row.try_get("bearer_token")?,
            chatgpt_account_id: row.try_get("chatgpt_account_id")?,
            enabled: row.try_get("enabled")?,
            priority: row.try_get("priority")?,
            created_at: row.try_get::<DateTime<Utc>, _>("created_at")?,
        })
    }

    async fn upsert_oauth_account(
        &self,
        req: ImportOAuthRefreshTokenRequest,
    ) -> Result<OAuthUpsertResult> {
        let cipher = self.require_credential_cipher()?;
        let resolved_mode = resolve_oauth_import_mode(req.mode.clone(), req.source_type.as_deref());
        let normalized_base_url =
            normalize_upstream_account_base_url(&resolved_mode, &req.base_url);
        let token_info = self
            .oauth_client
            .refresh_token(&req.refresh_token, Some(&normalized_base_url))
            .await
            .map_err(|err| anyhow!(err.to_string()))?;

        let refresh_token_sha256 = refresh_token_sha256(&token_info.refresh_token);
        let target_chatgpt_account_id = req
            .chatgpt_account_id
            .clone()
            .or(token_info.chatgpt_account_id.clone());
        let target_chatgpt_plan_type = req
            .chatgpt_plan_type
            .clone()
            .or(token_info.chatgpt_plan_type.clone());
        // Only exact refresh-token reuse or a stable account identity should collapse into the
        // same upstream account. Bare chatgpt_account_id is not unique across workspaces.
        let matched_account_id = sqlx::query(
            r#"
            SELECT c.account_id
            FROM upstream_account_oauth_credentials c
            INNER JOIN upstream_accounts a ON a.id = c.account_id
            WHERE a.auth_provider = $1 AND c.refresh_token_sha256 = $2
            LIMIT 1
            "#,
        )
        .bind(AUTH_PROVIDER_OAUTH_REFRESH_TOKEN)
        .bind(&refresh_token_sha256)
        .fetch_optional(&self.pool)
        .await
        .context("failed to query oauth account by refresh token hash")?
        .map(|row| row.try_get::<Uuid, _>("account_id"))
        .transpose()?;
        let matched_account_id = match matched_account_id {
            Some(account_id) => Some(account_id),
            None => {
                self.canonical_oauth_account_id_by_identity(
                    token_info.chatgpt_account_user_id.as_deref(),
                    token_info.chatgpt_user_id.as_deref(),
                    target_chatgpt_account_id.as_deref(),
                )
                .await?
            }
        };

        if let Some(account_id) = matched_account_id {
            let mut tx = self
                .pool
                .begin()
                .await
                .context("failed to start oauth upsert update transaction")?;

            let mode = upstream_mode_to_db(&resolved_mode);
            let enabled = req.enabled.unwrap_or(true);
            let priority = req.priority.unwrap_or(100);
            let access_token_enc = cipher.encrypt(&token_info.access_token)?;
            let fallback_access_token_enc = req
                .fallback_access_token
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| cipher.encrypt(value))
                .transpose()?;
            let refresh_token_enc = cipher.encrypt(&token_info.refresh_token)?;
            let token_family_id = Uuid::new_v4().to_string();
            let now = Utc::now();

            sqlx::query(
                r#"
                UPDATE upstream_accounts
                SET
                    label = $2,
                    mode = $3,
                    base_url = $4,
                    bearer_token = $5,
                    chatgpt_account_id = $6,
                    auth_provider = $7,
                    enabled = $8,
                    pool_state = $9,
                    priority = $10
                WHERE id = $1
                "#,
            )
            .bind(account_id)
            .bind(&req.label)
            .bind(mode)
            .bind(&normalized_base_url)
            .bind(OAUTH_MANAGED_BEARER_SENTINEL)
            .bind(target_chatgpt_account_id.clone())
            .bind(AUTH_PROVIDER_OAUTH_REFRESH_TOKEN)
            .bind(enabled)
            .bind(POOL_STATE_ACTIVE)
            .bind(priority)
            .execute(tx.as_mut())
            .await
            .context("failed to update oauth upstream account")?;

            sqlx::query(
                r#"
                INSERT INTO upstream_account_oauth_credentials (
                    account_id,
                    access_token_enc,
                    fallback_access_token_enc,
                    refresh_token_enc,
                    refresh_token_sha256,
                    token_family_id,
                    token_version,
                    token_expires_at,
                    fallback_token_expires_at,
                    last_refresh_at,
                    last_refresh_status,
                    last_refresh_error_code,
                    last_refresh_error,
                    refresh_failure_count,
                    refresh_backoff_until,
                    refresh_reused_detected,
                    refresh_inflight_until,
                    next_refresh_at,
                    updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, 1, $7, $8, $9, 'ok', NULL, NULL, 0, NULL, false, NULL, $10, $11)
                ON CONFLICT (account_id) DO UPDATE
                SET
                    access_token_enc = EXCLUDED.access_token_enc,
                    fallback_access_token_enc = COALESCE(
                        EXCLUDED.fallback_access_token_enc,
                        upstream_account_oauth_credentials.fallback_access_token_enc
                    ),
                    refresh_token_enc = EXCLUDED.refresh_token_enc,
                    refresh_token_sha256 = EXCLUDED.refresh_token_sha256,
                    token_family_id = COALESCE(upstream_account_oauth_credentials.token_family_id, EXCLUDED.token_family_id),
                    token_version = GREATEST(upstream_account_oauth_credentials.token_version, 0) + 1,
                    token_expires_at = EXCLUDED.token_expires_at,
                    fallback_token_expires_at = COALESCE(
                        EXCLUDED.fallback_token_expires_at,
                        upstream_account_oauth_credentials.fallback_token_expires_at
                    ),
                    last_refresh_at = EXCLUDED.last_refresh_at,
                    last_refresh_status = 'ok',
                    last_refresh_error_code = NULL,
                    last_refresh_error = NULL,
                    refresh_failure_count = 0,
                    refresh_backoff_until = NULL,
                    refresh_reused_detected = false,
                    refresh_inflight_until = NULL,
                    next_refresh_at = EXCLUDED.next_refresh_at,
                    updated_at = EXCLUDED.updated_at
                "#,
            )
            .bind(account_id)
            .bind(access_token_enc)
            .bind(fallback_access_token_enc)
            .bind(refresh_token_enc)
            .bind(refresh_token_sha256)
            .bind(token_family_id)
            .bind(token_info.expires_at)
            .bind(req.fallback_token_expires_at)
            .bind(now)
            .bind(schedule_next_oauth_refresh(token_info.expires_at, account_id))
            .bind(now)
            .execute(tx.as_mut())
            .await
            .context("failed to upsert oauth credential row")?;

            let session_profile = SessionProfileRecord::from_oauth_token_info(
                &token_info,
                SessionCredentialKind::RefreshRotatable,
                target_chatgpt_plan_type,
                req.source_type.clone(),
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
                .context("failed to commit oauth upsert update transaction")?;
            self.prefill_oauth_rate_limits_after_upsert(
                account_id,
                &token_info.access_token,
                &normalized_base_url,
                target_chatgpt_account_id.clone(),
            )
            .await;

            let row = sqlx::query(
                r#"
                SELECT id, label, mode, base_url, bearer_token, chatgpt_account_id, enabled, priority, created_at
                FROM upstream_accounts
                WHERE id = $1
                "#,
            )
            .bind(account_id)
            .fetch_one(&self.pool)
            .await
            .context("failed to fetch updated oauth account")?;

            let account = UpstreamAccount {
                id: row.try_get("id")?,
                label: row.try_get("label")?,
                mode: parse_upstream_mode(row.try_get::<String, _>("mode")?.as_str())?,
                base_url: row.try_get("base_url")?,
                bearer_token: row.try_get("bearer_token")?,
                chatgpt_account_id: row.try_get("chatgpt_account_id")?,
                enabled: row.try_get("enabled")?,
                priority: row.try_get("priority")?,
                created_at: row.try_get::<DateTime<Utc>, _>("created_at")?,
            };
            let _ = self
                .dedupe_oauth_accounts_by_identity_inner(
                    token_info.chatgpt_account_user_id.as_deref(),
                    token_info.chatgpt_user_id.as_deref(),
                    target_chatgpt_account_id.as_deref(),
                )
                .await?;

            return Ok(OAuthUpsertResult {
                account,
                created: false,
            });
        }

        let account = self.insert_oauth_account(req).await?;
        let _ = self
            .dedupe_oauth_accounts_by_identity_inner(
                token_info.chatgpt_account_user_id.as_deref(),
                token_info.chatgpt_user_id.as_deref(),
                target_chatgpt_account_id.as_deref(),
            )
            .await?;
        Ok(OAuthUpsertResult {
            account,
            created: true,
        })
    }

    async fn upsert_one_time_session_account_inner(
        &self,
        req: UpsertOneTimeSessionAccountRequest,
    ) -> Result<OAuthUpsertResult> {
        let normalized_label = req.label.trim().to_string();
        if normalized_label.is_empty() {
            return Err(anyhow!("label is required"));
        }

        let normalized_chatgpt_account_id = req
            .chatgpt_account_id
            .clone()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let normalized_base_url = normalize_upstream_account_base_url(&req.mode, &req.base_url);
        let enabled = req.enabled.unwrap_or(true);
        let priority = req.priority.unwrap_or(100);
        let mode = upstream_mode_to_db(&req.mode);

        let matched_account_id = if let Some(chatgpt_account_id) = normalized_chatgpt_account_id.as_deref() {
            sqlx::query(
                r#"
                SELECT id
                FROM upstream_accounts
                WHERE auth_provider = $1 AND mode = $2 AND chatgpt_account_id = $3
                ORDER BY created_at ASC
                LIMIT 1
                "#,
            )
            .bind(AUTH_PROVIDER_LEGACY_BEARER)
            .bind(mode)
            .bind(chatgpt_account_id)
            .fetch_optional(&self.pool)
            .await
            .context("failed to query one-time account by chatgpt_account_id")?
            .map(|row| row.try_get::<Uuid, _>("id"))
            .transpose()?
        } else {
            sqlx::query(
                r#"
                SELECT id
                FROM upstream_accounts
                WHERE auth_provider = $1 AND mode = $2 AND label = $3
                ORDER BY created_at ASC
                LIMIT 1
                "#,
            )
            .bind(AUTH_PROVIDER_LEGACY_BEARER)
            .bind(mode)
            .bind(&normalized_label)
            .fetch_optional(&self.pool)
            .await
            .context("failed to query one-time account by label")?
            .map(|row| row.try_get::<Uuid, _>("id"))
            .transpose()?
        };

        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start one-time account transaction")?;

        let account_id = if let Some(account_id) = matched_account_id {
            sqlx::query(
                r#"
                UPDATE upstream_accounts
                SET
                    label = $2,
                    mode = $3,
                    base_url = $4,
                    bearer_token = $5,
                    chatgpt_account_id = $6,
                    auth_provider = $7,
                    enabled = $8,
                    priority = $9
                WHERE id = $1
                "#,
            )
            .bind(account_id)
            .bind(&normalized_label)
            .bind(mode)
            .bind(&normalized_base_url)
            .bind(&req.access_token)
            .bind(&normalized_chatgpt_account_id)
            .bind(AUTH_PROVIDER_LEGACY_BEARER)
            .bind(enabled)
            .bind(priority)
            .execute(tx.as_mut())
            .await
            .context("failed to update one-time upstream account")?;
            account_id
        } else {
            let account_id = Uuid::new_v4();
            let created_at = Utc::now();
            sqlx::query(
                r#"
                INSERT INTO upstream_accounts (
                    id,
                    label,
                    mode,
                    base_url,
                    bearer_token,
                    chatgpt_account_id,
                    auth_provider,
                    enabled,
                    priority,
                    created_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                "#,
            )
            .bind(account_id)
            .bind(&normalized_label)
            .bind(mode)
            .bind(&normalized_base_url)
            .bind(&req.access_token)
            .bind(&normalized_chatgpt_account_id)
            .bind(AUTH_PROVIDER_LEGACY_BEARER)
            .bind(enabled)
            .bind(priority)
            .bind(created_at)
            .execute(tx.as_mut())
            .await
            .context("failed to insert one-time upstream account")?;
            account_id
        };

        let session_profile = SessionProfileRecord::one_time_access_token(
            req.token_expires_at,
            req.chatgpt_plan_type,
            req.source_type,
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
            .context("failed to commit one-time account transaction")?;

        let row = sqlx::query(
            r#"
            SELECT id, label, mode, base_url, bearer_token, chatgpt_account_id, enabled, priority, created_at
            FROM upstream_accounts
            WHERE id = $1
            "#,
        )
        .bind(account_id)
        .fetch_one(&self.pool)
        .await
        .context("failed to fetch one-time account after upsert")?;

        let account = UpstreamAccount {
            id: row.try_get("id")?,
            label: row.try_get("label")?,
            mode: parse_upstream_mode(row.try_get::<String, _>("mode")?.as_str())?,
            base_url: row.try_get("base_url")?,
            bearer_token: row.try_get("bearer_token")?,
            chatgpt_account_id: row.try_get("chatgpt_account_id")?,
            enabled: row.try_get("enabled")?,
            priority: row.try_get("priority")?,
            created_at: row.try_get::<DateTime<Utc>, _>("created_at")?,
        };

        Ok(OAuthUpsertResult {
            account,
            created: matched_account_id.is_none(),
        })
    }

}
