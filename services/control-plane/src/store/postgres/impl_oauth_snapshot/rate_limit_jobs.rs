impl PostgresStore {
    async fn fetch_oauth_account_status(
        &self,
        account_id: Uuid,
    ) -> Result<OAuthAccountStatusResponse> {
        let row = sqlx::query(
            r#"
            SELECT
                a.id,
                a.enabled,
                a.mode,
                a.base_url,
                a.chatgpt_account_id,
                a.auth_provider,
                p.credential_kind,
                p.token_expires_at AS profile_token_expires_at,
                p.email,
                p.oauth_subject,
                p.oauth_identity_provider,
                p.email_verified,
                p.chatgpt_plan_type,
                p.chatgpt_user_id,
                p.chatgpt_subscription_active_start,
                p.chatgpt_subscription_active_until,
                p.chatgpt_subscription_last_checked,
                p.chatgpt_account_user_id,
                p.chatgpt_compute_residency,
                p.organizations_json::text AS organizations_json_text,
                p.groups_json::text AS groups_json_text,
                p.source_type,
                c.access_token_enc,
                c.token_family_id,
                c.token_version,
                c.token_expires_at,
                c.last_refresh_at,
                c.last_refresh_status,
                c.refresh_reused_detected,
                c.last_refresh_error_code,
                c.last_refresh_error,
                rl.rate_limits_json::text AS rate_limits_json_text,
                rl.fetched_at AS rate_limits_fetched_at,
                rl.expires_at AS rate_limits_expires_at,
                rl.last_error_code AS rate_limits_last_error_code,
                rl.last_error_message AS rate_limits_last_error
            FROM upstream_accounts a
            LEFT JOIN upstream_account_oauth_credentials c ON c.account_id = a.id
            LEFT JOIN upstream_account_session_profiles p ON p.account_id = a.id
            LEFT JOIN upstream_account_rate_limit_snapshots rl ON rl.account_id = a.id
            WHERE a.id = $1
            "#,
        )
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to query oauth account status")?
        .ok_or_else(|| anyhow!("account not found"))?;

        let auth_provider =
            parse_upstream_auth_provider(row.try_get::<String, _>("auth_provider")?.as_str())?;
        let token_expires_at = row
            .try_get::<Option<DateTime<Utc>>, _>("token_expires_at")?
            .or_else(|| row.try_get::<Option<DateTime<Utc>>, _>("profile_token_expires_at").ok().flatten());
        let last_refresh_status = parse_oauth_refresh_status(
            row.try_get::<Option<String>, _>("last_refresh_status")?
                .unwrap_or_else(|| "never".to_string())
                .as_str(),
        )?;
        let enabled = row.try_get::<bool, _>("enabled")?;
        let mode = parse_upstream_mode(row.try_get::<String, _>("mode")?.as_str())?;
        let credential_kind = row
            .try_get::<Option<String>, _>("credential_kind")?
            .as_deref()
            .map(parse_session_credential_kind)
            .transpose()?
            .or_else(|| match (auth_provider.clone(), mode) {
                (UpstreamAuthProvider::OAuthRefreshToken, _) => {
                    Some(SessionCredentialKind::RefreshRotatable)
                }
                (UpstreamAuthProvider::LegacyBearer, UpstreamMode::ChatGptSession)
                | (UpstreamAuthProvider::LegacyBearer, UpstreamMode::CodexOauth) => {
                    Some(SessionCredentialKind::OneTimeAccessToken)
                }
                _ => None,
            });
        let last_refresh_at = row.try_get::<Option<DateTime<Utc>>, _>("last_refresh_at")?;
        let refresh_reused_detected = row
            .try_get::<Option<bool>, _>("refresh_reused_detected")?
            .unwrap_or(false);
        let last_refresh_error_code = row.try_get::<Option<String>, _>("last_refresh_error_code")?;
        let last_refresh_error = row.try_get::<Option<String>, _>("last_refresh_error")?;
        let rate_limits = parse_rate_limit_snapshots(
            row.try_get::<Option<String>, _>("rate_limits_json_text")?,
        );
        let rate_limits_fetched_at =
            row.try_get::<Option<DateTime<Utc>>, _>("rate_limits_fetched_at")?;
        let rate_limits_expires_at =
            row.try_get::<Option<DateTime<Utc>>, _>("rate_limits_expires_at")?;
        let rate_limits_last_error_code =
            row.try_get::<Option<String>, _>("rate_limits_last_error_code")?;
        let rate_limits_last_error = row.try_get::<Option<String>, _>("rate_limits_last_error")?;

        let now = Utc::now();
        let effective_enabled = oauth_effective_enabled(
            enabled,
            &auth_provider,
            credential_kind.as_ref(),
            token_expires_at,
            &last_refresh_status,
            refresh_reused_detected,
            last_refresh_error_code.as_deref(),
            rate_limits_expires_at,
            rate_limits_last_error_code.as_deref(),
            rate_limits_last_error.as_deref(),
            now,
        );
        let next_refresh_at = match &auth_provider {
            UpstreamAuthProvider::OAuthRefreshToken => {
                token_expires_at.map(|expires_at| expires_at - Duration::seconds(OAUTH_REFRESH_WINDOW_SEC))
            }
            _ => None,
        };

        Ok(OAuthAccountStatusResponse {
            account_id,
            auth_provider,
            credential_kind,
            email: row.try_get::<Option<String>, _>("email")?,
            oauth_subject: row.try_get::<Option<String>, _>("oauth_subject")?,
            oauth_identity_provider: row
                .try_get::<Option<String>, _>("oauth_identity_provider")?,
            email_verified: row.try_get::<Option<bool>, _>("email_verified")?,
            chatgpt_plan_type: row.try_get::<Option<String>, _>("chatgpt_plan_type")?,
            chatgpt_user_id: row.try_get::<Option<String>, _>("chatgpt_user_id")?,
            chatgpt_subscription_active_start: row
                .try_get::<Option<DateTime<Utc>>, _>("chatgpt_subscription_active_start")?,
            chatgpt_subscription_active_until: row
                .try_get::<Option<DateTime<Utc>>, _>("chatgpt_subscription_active_until")?,
            chatgpt_subscription_last_checked: row
                .try_get::<Option<DateTime<Utc>>, _>("chatgpt_subscription_last_checked")?,
            chatgpt_account_user_id: row
                .try_get::<Option<String>, _>("chatgpt_account_user_id")?,
            chatgpt_compute_residency: row
                .try_get::<Option<String>, _>("chatgpt_compute_residency")?,
            organizations: parse_json_array(
                row.try_get::<Option<String>, _>("organizations_json_text")?,
            ),
            groups: parse_json_array(row.try_get::<Option<String>, _>("groups_json_text")?),
            source_type: row.try_get::<Option<String>, _>("source_type")?,
            token_family_id: row.try_get::<Option<String>, _>("token_family_id")?,
            token_version: row
                .try_get::<Option<i64>, _>("token_version")?
                .map(|value| value.max(0) as u64),
            token_expires_at,
            last_refresh_at,
            last_refresh_status,
            refresh_reused_detected,
            last_refresh_error_code,
            last_refresh_error,
            effective_enabled,
            rate_limits,
            rate_limits_fetched_at,
            rate_limits_expires_at,
            rate_limits_last_error_code,
            rate_limits_last_error,
            next_refresh_at,
        })
    }

    async fn fetch_oauth_account_statuses(
        &self,
        account_ids: &[Uuid],
    ) -> Result<Vec<OAuthAccountStatusResponse>> {
        if account_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query(
            r#"
            SELECT
                a.id,
                a.enabled,
                a.mode,
                a.base_url,
                a.chatgpt_account_id,
                a.auth_provider,
                p.credential_kind,
                p.token_expires_at AS profile_token_expires_at,
                p.email,
                p.oauth_subject,
                p.oauth_identity_provider,
                p.email_verified,
                p.chatgpt_plan_type,
                p.chatgpt_user_id,
                p.chatgpt_subscription_active_start,
                p.chatgpt_subscription_active_until,
                p.chatgpt_subscription_last_checked,
                p.chatgpt_account_user_id,
                p.chatgpt_compute_residency,
                p.organizations_json::text AS organizations_json_text,
                p.groups_json::text AS groups_json_text,
                p.source_type,
                c.access_token_enc,
                c.token_family_id,
                c.token_version,
                c.token_expires_at,
                c.last_refresh_at,
                c.last_refresh_status,
                c.refresh_reused_detected,
                c.last_refresh_error_code,
                c.last_refresh_error,
                rl.rate_limits_json::text AS rate_limits_json_text,
                rl.fetched_at AS rate_limits_fetched_at,
                rl.expires_at AS rate_limits_expires_at,
                rl.last_error_code AS rate_limits_last_error_code,
                rl.last_error_message AS rate_limits_last_error
            FROM upstream_accounts a
            LEFT JOIN upstream_account_oauth_credentials c ON c.account_id = a.id
            LEFT JOIN upstream_account_session_profiles p ON p.account_id = a.id
            LEFT JOIN upstream_account_rate_limit_snapshots rl ON rl.account_id = a.id
            WHERE a.id = ANY($1)
            "#,
        )
        .bind(account_ids)
        .fetch_all(&self.pool)
        .await
        .context("failed to query oauth account statuses")?;

        let now = Utc::now();
        struct PendingOAuthStatus {
            account_id: Uuid,
            auth_provider: UpstreamAuthProvider,
            credential_kind: Option<SessionCredentialKind>,
            email: Option<String>,
            oauth_subject: Option<String>,
            oauth_identity_provider: Option<String>,
            email_verified: Option<bool>,
            chatgpt_plan_type: Option<String>,
            chatgpt_user_id: Option<String>,
            chatgpt_subscription_active_start: Option<DateTime<Utc>>,
            chatgpt_subscription_active_until: Option<DateTime<Utc>>,
            chatgpt_subscription_last_checked: Option<DateTime<Utc>>,
            chatgpt_account_user_id: Option<String>,
            chatgpt_compute_residency: Option<String>,
            organizations: Option<Vec<serde_json::Value>>,
            groups: Option<Vec<serde_json::Value>>,
            source_type: Option<String>,
            token_family_id: Option<String>,
            token_version: Option<u64>,
            token_expires_at: Option<DateTime<Utc>>,
            last_refresh_at: Option<DateTime<Utc>>,
            last_refresh_status: OAuthRefreshStatus,
            refresh_reused_detected: bool,
            last_refresh_error_code: Option<String>,
            last_refresh_error: Option<String>,
            effective_enabled: bool,
            next_refresh_at: Option<DateTime<Utc>>,
            rate_limits: Vec<OAuthRateLimitSnapshot>,
            rate_limits_fetched_at: Option<DateTime<Utc>>,
            rate_limits_expires_at: Option<DateTime<Utc>>,
            rate_limits_last_error_code: Option<String>,
            rate_limits_last_error: Option<String>,
        }

        let mut pending = Vec::with_capacity(rows.len());
        for row in rows {
            let account_id: Uuid = row.try_get("id")?;
            let auth_provider =
                parse_upstream_auth_provider(row.try_get::<String, _>("auth_provider")?.as_str())?;
            let token_expires_at = row
                .try_get::<Option<DateTime<Utc>>, _>("token_expires_at")?
                .or_else(|| row.try_get::<Option<DateTime<Utc>>, _>("profile_token_expires_at").ok().flatten());
            let last_refresh_status = parse_oauth_refresh_status(
                row.try_get::<Option<String>, _>("last_refresh_status")?
                    .unwrap_or_else(|| "never".to_string())
                    .as_str(),
            )?;
            let enabled = row.try_get::<bool, _>("enabled")?;
            let mode = parse_upstream_mode(row.try_get::<String, _>("mode")?.as_str())?;
            let credential_kind = row
                .try_get::<Option<String>, _>("credential_kind")?
                .as_deref()
                .map(parse_session_credential_kind)
                .transpose()?
                .or_else(|| match (auth_provider.clone(), mode) {
                    (UpstreamAuthProvider::OAuthRefreshToken, _) => {
                        Some(SessionCredentialKind::RefreshRotatable)
                    }
                    (UpstreamAuthProvider::LegacyBearer, UpstreamMode::ChatGptSession)
                    | (UpstreamAuthProvider::LegacyBearer, UpstreamMode::CodexOauth) => {
                        Some(SessionCredentialKind::OneTimeAccessToken)
                    }
                    _ => None,
                });
            let refresh_reused_detected = row
                .try_get::<Option<bool>, _>("refresh_reused_detected")?
                .unwrap_or(false);
            let last_refresh_error_code = row.try_get::<Option<String>, _>("last_refresh_error_code")?;
            let last_refresh_error = row.try_get::<Option<String>, _>("last_refresh_error")?;
            let rate_limits = parse_rate_limit_snapshots(
                row.try_get::<Option<String>, _>("rate_limits_json_text")?,
            );
            let rate_limits_fetched_at = row
                .try_get::<Option<DateTime<Utc>>, _>("rate_limits_fetched_at")?;
            let rate_limits_expires_at = row
                .try_get::<Option<DateTime<Utc>>, _>("rate_limits_expires_at")?;
            let rate_limits_last_error_code = row
                .try_get::<Option<String>, _>("rate_limits_last_error_code")?;
            let rate_limits_last_error = row
                .try_get::<Option<String>, _>("rate_limits_last_error")?;

            let effective_enabled = oauth_effective_enabled(
                enabled,
                &auth_provider,
                credential_kind.as_ref(),
                token_expires_at,
                &last_refresh_status,
                refresh_reused_detected,
                last_refresh_error_code.as_deref(),
                rate_limits_expires_at,
                rate_limits_last_error_code.as_deref(),
                rate_limits_last_error.as_deref(),
                now,
            );
            let next_refresh_at = match &auth_provider {
                UpstreamAuthProvider::OAuthRefreshToken => {
                    token_expires_at.map(|expires_at| expires_at - Duration::seconds(OAUTH_REFRESH_WINDOW_SEC))
                }
                _ => None,
            };
            pending.push(PendingOAuthStatus {
                account_id,
                auth_provider,
                credential_kind,
                email: row.try_get::<Option<String>, _>("email")?,
                oauth_subject: row.try_get::<Option<String>, _>("oauth_subject")?,
                oauth_identity_provider: row
                    .try_get::<Option<String>, _>("oauth_identity_provider")?,
                email_verified: row.try_get::<Option<bool>, _>("email_verified")?,
                chatgpt_plan_type: row.try_get::<Option<String>, _>("chatgpt_plan_type")?,
                chatgpt_user_id: row.try_get::<Option<String>, _>("chatgpt_user_id")?,
                chatgpt_subscription_active_start: row
                    .try_get::<Option<DateTime<Utc>>, _>("chatgpt_subscription_active_start")?,
                chatgpt_subscription_active_until: row
                    .try_get::<Option<DateTime<Utc>>, _>("chatgpt_subscription_active_until")?,
                chatgpt_subscription_last_checked: row
                    .try_get::<Option<DateTime<Utc>>, _>("chatgpt_subscription_last_checked")?,
                chatgpt_account_user_id: row
                    .try_get::<Option<String>, _>("chatgpt_account_user_id")?,
                chatgpt_compute_residency: row
                    .try_get::<Option<String>, _>("chatgpt_compute_residency")?,
                organizations: parse_json_array(
                    row.try_get::<Option<String>, _>("organizations_json_text")?,
                ),
                groups: parse_json_array(row.try_get::<Option<String>, _>("groups_json_text")?),
                source_type: row.try_get::<Option<String>, _>("source_type")?,
                token_family_id: row.try_get::<Option<String>, _>("token_family_id")?,
                token_version: row
                    .try_get::<Option<i64>, _>("token_version")?
                    .map(|value| value.max(0) as u64),
                token_expires_at,
                last_refresh_at: row.try_get::<Option<DateTime<Utc>>, _>("last_refresh_at")?,
                last_refresh_status,
                refresh_reused_detected,
                last_refresh_error_code,
                last_refresh_error,
                effective_enabled,
                next_refresh_at,
                rate_limits,
                rate_limits_fetched_at,
                rate_limits_expires_at,
                rate_limits_last_error_code,
                rate_limits_last_error,
            });
        }

        let mut status_by_id = std::collections::HashMap::with_capacity(pending.len());
        for item in pending {
            status_by_id.insert(
                item.account_id,
                OAuthAccountStatusResponse {
                    account_id: item.account_id,
                    auth_provider: item.auth_provider,
                    credential_kind: item.credential_kind,
                    email: item.email,
                    oauth_subject: item.oauth_subject,
                    oauth_identity_provider: item.oauth_identity_provider,
                    email_verified: item.email_verified,
                    chatgpt_plan_type: item.chatgpt_plan_type,
                    chatgpt_user_id: item.chatgpt_user_id,
                    chatgpt_subscription_active_start: item.chatgpt_subscription_active_start,
                    chatgpt_subscription_active_until: item.chatgpt_subscription_active_until,
                    chatgpt_subscription_last_checked: item
                        .chatgpt_subscription_last_checked,
                    chatgpt_account_user_id: item.chatgpt_account_user_id,
                    chatgpt_compute_residency: item.chatgpt_compute_residency,
                    organizations: item.organizations,
                    groups: item.groups,
                    source_type: item.source_type,
                    token_family_id: item.token_family_id,
                    token_version: item.token_version,
                    token_expires_at: item.token_expires_at,
                    last_refresh_at: item.last_refresh_at,
                    last_refresh_status: item.last_refresh_status,
                    refresh_reused_detected: item.refresh_reused_detected,
                    last_refresh_error_code: item.last_refresh_error_code,
                    last_refresh_error: item.last_refresh_error,
                    effective_enabled: item.effective_enabled,
                    rate_limits: item.rate_limits,
                    rate_limits_fetched_at: item.rate_limits_fetched_at,
                    rate_limits_expires_at: item.rate_limits_expires_at,
                    rate_limits_last_error_code: item.rate_limits_last_error_code,
                    rate_limits_last_error: item.rate_limits_last_error,
                    next_refresh_at: item.next_refresh_at,
                },
            );
        }

        let mut items = Vec::with_capacity(account_ids.len());
        for account_id in account_ids {
            let status = status_by_id
                .remove(account_id)
                .ok_or_else(|| anyhow!("account not found"))?;
            items.push(status);
        }

        Ok(items)
    }

    async fn load_due_rate_limit_refresh_targets(
        &self,
        limit: usize,
    ) -> Result<Vec<RateLimitRefreshTarget>> {
        let now = Utc::now();
        let rows = sqlx::query(
            r#"
            SELECT
                a.id,
                a.base_url,
                a.chatgpt_account_id,
                c.access_token_enc
            FROM upstream_accounts a
            INNER JOIN upstream_account_oauth_credentials c ON c.account_id = a.id
            LEFT JOIN upstream_account_rate_limit_snapshots rl ON rl.account_id = a.id
            WHERE
                a.auth_provider = $1
                AND a.pool_state = $2
                AND a.enabled = true
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
                AND (rl.expires_at IS NULL OR rl.expires_at <= $4)
            ORDER BY COALESCE(rl.expires_at, 'epoch'::timestamptz) ASC, a.created_at ASC
            LIMIT $5
            "#,
        )
        .bind(AUTH_PROVIDER_OAUTH_REFRESH_TOKEN)
        .bind(POOL_STATE_ACTIVE)
        .bind(now + Duration::seconds(OAUTH_MIN_VALID_SEC))
        .bind(now)
        .bind(i64::try_from(limit).unwrap_or(i64::MAX))
        .fetch_all(&self.pool)
        .await
        .context("failed to list due rate-limit refresh targets")?;

        let mut targets = Vec::with_capacity(rows.len());
        for row in rows {
            targets.push(RateLimitRefreshTarget {
                account_id: row.try_get::<Uuid, _>("id")?,
                base_url: row.try_get::<String, _>("base_url")?,
                chatgpt_account_id: row.try_get::<Option<String>, _>("chatgpt_account_id")?,
                access_token_enc: row.try_get::<Option<String>, _>("access_token_enc")?,
            });
        }
        Ok(targets)
    }

    async fn load_all_rate_limit_refresh_targets_after(
        &self,
        after_id: Option<Uuid>,
        limit: usize,
    ) -> Result<Vec<RateLimitRefreshTarget>> {
        let now = Utc::now();
        let rows = sqlx::query(
            r#"
            SELECT
                a.id,
                a.base_url,
                a.chatgpt_account_id,
                c.access_token_enc
            FROM upstream_accounts a
            INNER JOIN upstream_account_oauth_credentials c ON c.account_id = a.id
            WHERE
                a.auth_provider = $1
                AND a.pool_state = $2
                AND a.enabled = true
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
                AND ($4::uuid IS NULL OR a.id > $4)
            ORDER BY a.id ASC
            LIMIT $5
            "#,
        )
        .bind(AUTH_PROVIDER_OAUTH_REFRESH_TOKEN)
        .bind(POOL_STATE_ACTIVE)
        .bind(now + Duration::seconds(OAUTH_MIN_VALID_SEC))
        .bind(after_id)
        .bind(i64::try_from(limit).unwrap_or(i64::MAX))
        .fetch_all(&self.pool)
        .await
        .context("failed to list all rate-limit refresh targets")?;

        let mut targets = Vec::with_capacity(rows.len());
        for row in rows {
            targets.push(RateLimitRefreshTarget {
                account_id: row.try_get::<Uuid, _>("id")?,
                base_url: row.try_get::<String, _>("base_url")?,
                chatgpt_account_id: row.try_get::<Option<String>, _>("chatgpt_account_id")?,
                access_token_enc: row.try_get::<Option<String>, _>("access_token_enc")?,
            });
        }
        Ok(targets)
    }

    async fn persist_rate_limit_cache_success(
        &self,
        account_id: Uuid,
        rate_limits: Vec<OAuthRateLimitSnapshot>,
        fetched_at: DateTime<Utc>,
    ) -> Result<()> {
        let expires_at = fetched_at + Duration::seconds(rate_limit_cache_ttl_sec_from_env());
        let payload = serde_json::to_string(&rate_limits)
            .context("failed to encode oauth rate-limit snapshots json")?;
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start oauth rate-limit success transaction")?;

        sqlx::query(
            r#"
            INSERT INTO upstream_account_rate_limit_snapshots (
                account_id,
                rate_limits_json,
                fetched_at,
                expires_at,
                last_error_code,
                last_error_message,
                updated_at
            )
            VALUES ($1, $2::jsonb, $3, $4, NULL, NULL, $3)
            ON CONFLICT (account_id) DO UPDATE
            SET
                rate_limits_json = EXCLUDED.rate_limits_json,
                fetched_at = EXCLUDED.fetched_at,
                expires_at = EXCLUDED.expires_at,
                last_error_code = NULL,
                last_error_message = NULL,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(account_id)
        .bind(payload)
        .bind(fetched_at)
        .bind(expires_at)
        .execute(tx.as_mut())
        .await
        .context("failed to upsert oauth rate-limit cache snapshot")?;

        self.append_data_plane_outbox_event_tx(
            &mut tx,
            DataPlaneSnapshotEventType::AccountUpsert,
            account_id,
        )
        .await?;
        self.bump_revision_tx(&mut tx).await?;
        tx.commit()
            .await
            .context("failed to commit oauth rate-limit success transaction")?;

        Ok(())
    }

    async fn persist_rate_limit_cache_failure(
        &self,
        account_id: Uuid,
        error_code: &str,
        error_message: &str,
    ) -> Result<()> {
        let now = Utc::now();
        let backoff_sec = rate_limit_failure_backoff_seconds(error_code, error_message);
        let next_retry_at = now + Duration::seconds(backoff_sec);
        let truncated_message = truncate_error_message(error_message.to_string());
        let blocking_error = is_blocking_rate_limit_error(Some(error_code), Some(error_message));
        let quota_signal = is_quota_error_signal(error_code, error_message);
        let auth_signal = is_auth_error_signal(error_code, error_message);
        let rate_limited_signal = is_rate_limited_signal(error_code, error_message);
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start oauth rate-limit failure transaction")?;

        sqlx::query(
            r#"
            INSERT INTO upstream_account_rate_limit_snapshots (
                account_id,
                rate_limits_json,
                fetched_at,
                expires_at,
                last_error_code,
                last_error_message,
                updated_at
            )
            VALUES ($1, '[]'::jsonb, NULL, $2, $3, $4, $5)
            ON CONFLICT (account_id) DO UPDATE
            SET
                expires_at = EXCLUDED.expires_at,
                last_error_code = EXCLUDED.last_error_code,
                last_error_message = EXCLUDED.last_error_message,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(account_id)
        .bind(next_retry_at)
        .bind(error_code)
        .bind(truncated_message.as_str())
        .bind(now)
        .execute(tx.as_mut())
        .await
        .context("failed to persist oauth rate-limit cache failure")?;

        self.append_data_plane_outbox_event_tx(
            &mut tx,
            DataPlaneSnapshotEventType::AccountUpsert,
            account_id,
        )
        .await?;
        self.bump_revision_tx(&mut tx).await?;
        tx.commit()
            .await
            .context("failed to commit oauth rate-limit failure transaction")?;

        tracing::warn!(
            account_id = %account_id,
            error_code,
            error_message = %truncated_message,
            backoff_sec,
            next_retry_at = %next_retry_at,
            blocking_error,
            quota_signal,
            auth_signal,
            rate_limited_signal,
            "oauth rate-limit refresh failed; applied health-policy backoff window"
        );

        Ok(())
    }

    async fn persist_rate_limit_success_outcome(
        &self,
        account_id: Uuid,
        rate_limits: Vec<OAuthRateLimitSnapshot>,
        fetched_at: DateTime<Utc>,
    ) -> (bool, Option<String>) {
        match self
            .persist_rate_limit_cache_success(account_id, rate_limits, fetched_at)
            .await
        {
            Ok(()) => (true, None),
            Err(err) => {
                let code = "cache_persist_failed".to_string();
                let _ = self
                    .persist_rate_limit_cache_failure(account_id, &code, &err.to_string())
                    .await;
                (false, Some(code))
            }
        }
    }

    async fn load_rate_limit_refresh_target_by_account_id(
        &self,
        account_id: Uuid,
    ) -> Result<RateLimitRefreshTarget> {
        let row = sqlx::query(
            r#"
            SELECT
                a.id,
                a.base_url,
                a.chatgpt_account_id,
                c.access_token_enc
            FROM upstream_accounts a
            INNER JOIN upstream_account_oauth_credentials c ON c.account_id = a.id
            WHERE
                a.id = $1
                AND a.auth_provider = $2
                AND a.pool_state = $3
                AND a.enabled = true
            "#,
        )
        .bind(account_id)
        .bind(AUTH_PROVIDER_OAUTH_REFRESH_TOKEN)
        .bind(POOL_STATE_ACTIVE)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load oauth rate-limit refresh target by account id")?
        .ok_or_else(|| anyhow!("oauth rate-limit refresh target not found after oauth refresh"))?;

        Ok(RateLimitRefreshTarget {
            account_id: row.try_get::<Uuid, _>("id")?,
            base_url: row.try_get::<String, _>("base_url")?,
            chatgpt_account_id: row.try_get::<Option<String>, _>("chatgpt_account_id")?,
            access_token_enc: row.try_get::<Option<String>, _>("access_token_enc")?,
        })
    }

    async fn maybe_refresh_oauth_rate_limit_cache_on_seen_ok_inner(
        &self,
        account_id: Uuid,
    ) -> Result<()> {
        let now = Utc::now();
        let row = sqlx::query(
            r#"
            SELECT
                a.id,
                a.base_url,
                a.chatgpt_account_id,
                c.access_token_enc,
                c.token_expires_at,
                c.last_refresh_status,
                c.refresh_reused_detected,
                c.last_refresh_error_code,
                rl.expires_at AS rate_limits_expires_at,
                rl.last_error_code AS rate_limits_last_error_code,
                rl.last_error_message AS rate_limits_last_error
            FROM upstream_accounts a
            INNER JOIN upstream_account_oauth_credentials c ON c.account_id = a.id
            LEFT JOIN upstream_account_rate_limit_snapshots rl ON rl.account_id = a.id
            WHERE
                a.id = $1
                AND a.auth_provider = $2
                AND a.pool_state = $3
                AND a.enabled = true
            "#,
        )
        .bind(account_id)
        .bind(AUTH_PROVIDER_OAUTH_REFRESH_TOKEN)
        .bind(POOL_STATE_ACTIVE)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load seen_ok oauth rate-limit refresh target")?;

        let Some(row) = row else {
            return Ok(());
        };

        let token_expires_at = row.try_get::<Option<DateTime<Utc>>, _>("token_expires_at")?;
        let refresh_reused_detected = row
            .try_get::<Option<bool>, _>("refresh_reused_detected")?
            .unwrap_or(false);
        let last_refresh_status = parse_oauth_refresh_status(
            row.try_get::<Option<String>, _>("last_refresh_status")?
                .unwrap_or_else(|| "never".to_string())
                .as_str(),
        )?;
        let last_refresh_error_code = row.try_get::<Option<String>, _>("last_refresh_error_code")?;
        let rate_limits_expires_at =
            row.try_get::<Option<DateTime<Utc>>, _>("rate_limits_expires_at")?;
        let rate_limits_last_error_code =
            row.try_get::<Option<String>, _>("rate_limits_last_error_code")?;
        let rate_limits_last_error = row.try_get::<Option<String>, _>("rate_limits_last_error")?;
        if !should_refresh_rate_limit_cache_on_seen_ok(
            now,
            token_expires_at,
            &last_refresh_status,
            refresh_reused_detected,
            last_refresh_error_code.as_deref(),
            rate_limits_expires_at,
            rate_limits_last_error_code.as_deref(),
            rate_limits_last_error.as_deref(),
        ) {
            return Ok(());
        }

        let target = RateLimitRefreshTarget {
            account_id,
            base_url: row.try_get::<String, _>("base_url")?,
            chatgpt_account_id: row.try_get::<Option<String>, _>("chatgpt_account_id")?,
            access_token_enc: row.try_get::<Option<String>, _>("access_token_enc")?,
        };

        let stats = self
            .refresh_rate_limit_targets_batch(vec![target], 1)
            .await;
        if stats.failed > 0 {
            tracing::warn!(
                account_id = %account_id,
                success = stats.success,
                failed = stats.failed,
                error_counts = ?stats.error_counts,
                "seen_ok-triggered oauth rate-limit refresh completed with failures"
            );
        }

        Ok(())
    }

    async fn try_refresh_oauth_account_after_rate_limit_failure(&self, account_id: Uuid) -> bool {
        match self.refresh_oauth_account_inner(account_id, true).await {
            Ok(status) => matches!(status.last_refresh_status, OAuthRefreshStatus::Ok),
            Err(_) => false,
        }
    }

    async fn refresh_rate_limit_targets_batch(
        &self,
        targets: Vec<RateLimitRefreshTarget>,
        concurrency: usize,
    ) -> RateLimitRefreshBatchStats {
        let effective_concurrency = concurrency.max(1);
        let max_rps = rate_limit_refresh_max_rps_from_env();
        let launch_interval = std::time::Duration::from_secs_f64(1.0 / f64::from(max_rps));
        let throttle = std::sync::Arc::new(tokio::sync::Mutex::new(tokio::time::Instant::now()));
        let results = futures_util::stream::iter(targets.into_iter())
            .map(|target| {
                let throttle = throttle.clone();
                async move {
                    throttle_refresh_start(throttle.as_ref(), launch_interval).await;
                    let account_id = target.account_id;
                    let fetched_at = Utc::now();
                    match self
                        .fetch_live_rate_limits_result(
                            &UpstreamAuthProvider::OAuthRefreshToken,
                            target.access_token_enc,
                            Some(target.base_url),
                            target.chatgpt_account_id,
                        )
                        .await
                    {
                        Ok(rate_limits) => {
                            self.persist_rate_limit_success_outcome(
                                account_id,
                                rate_limits,
                                fetched_at,
                            )
                            .await
                        }
                        Err((error_code, error_message)) => {
                            let should_try_refresh = should_trigger_refresh_after_rate_limit_failure(
                                &error_code,
                                &error_message,
                            );
                            if should_try_refresh {
                                tracing::info!(
                                    account_id = %account_id,
                                    error_code = %error_code,
                                    "oauth rate-limit failure matched auth signal; attempting forced oauth refresh"
                                );
                                if self
                                    .try_refresh_oauth_account_after_rate_limit_failure(account_id)
                                    .await
                                {
                                    tracing::info!(
                                        account_id = %account_id,
                                        error_code = %error_code,
                                        "forced oauth refresh recovered account after rate-limit auth failure"
                                    );
                                    let refreshed_target = match self
                                        .load_rate_limit_refresh_target_by_account_id(account_id)
                                        .await
                                    {
                                        Ok(target) => target,
                                        Err(err) => {
                                            let code =
                                                "refresh_recovered_target_load_failed".to_string();
                                            let _ = self
                                                .persist_rate_limit_cache_failure(
                                                    account_id,
                                                    &code,
                                                    &err.to_string(),
                                                )
                                                .await;
                                            return (false, Some(code));
                                        }
                                    };
                                    let refetched_at = Utc::now();
                                    return match self
                                        .fetch_live_rate_limits_result(
                                            &UpstreamAuthProvider::OAuthRefreshToken,
                                            refreshed_target.access_token_enc,
                                            Some(refreshed_target.base_url),
                                            refreshed_target.chatgpt_account_id,
                                        )
                                        .await
                                    {
                                        Ok(rate_limits) => {
                                            self.persist_rate_limit_success_outcome(
                                                account_id,
                                                rate_limits,
                                                refetched_at,
                                            )
                                            .await
                                        }
                                        Err((refetch_error_code, refetch_error_message)) => {
                                            let _ = self
                                                .persist_rate_limit_cache_failure(
                                                    account_id,
                                                    &refetch_error_code,
                                                    &refetch_error_message,
                                                )
                                                .await;
                                            (false, Some(refetch_error_code))
                                        }
                                    };
                                }
                                tracing::warn!(
                                    account_id = %account_id,
                                    error_code = %error_code,
                                    "forced oauth refresh did not recover account after rate-limit auth failure"
                                );
                            }
                            let _ = self
                                .persist_rate_limit_cache_failure(
                                    account_id,
                                    &error_code,
                                    &error_message,
                                )
                                .await;
                            (false, Some(error_code))
                        }
                    }
                }
            })
            .buffer_unordered(effective_concurrency)
            .collect::<Vec<_>>()
            .await;

        let mut stats = RateLimitRefreshBatchStats {
            processed: u64::try_from(results.len()).unwrap_or(u64::MAX),
            ..Default::default()
        };
        for (success, error_code) in results {
            if success {
                stats.success = stats.success.saturating_add(1);
            } else {
                stats.failed = stats.failed.saturating_add(1);
                if let Some(code) = error_code {
                    *stats.error_counts.entry(code).or_insert(0) += 1;
                }
            }
        }
        stats
    }

    async fn refresh_due_oauth_rate_limit_caches_inner(&self) -> Result<u64> {
        let batch_size = rate_limit_refresh_batch_size_from_env();
        let concurrency = rate_limit_refresh_concurrency_from_env();
        let mut refreshed_total = 0_u64;

        loop {
            let targets = self.load_due_rate_limit_refresh_targets(batch_size).await?;
            if targets.is_empty() {
                break;
            }
            let fetched = targets.len();
            let stats = self
                .refresh_rate_limit_targets_batch(targets, concurrency)
                .await;
            refreshed_total = refreshed_total.saturating_add(stats.processed);
            if stats.failed > 0 {
                tracing::warn!(
                    targets = fetched,
                    processed = stats.processed,
                    success = stats.success,
                    failed = stats.failed,
                    error_counts = ?stats.error_counts,
                    "oauth rate-limit refresh batch completed with failures"
                );
            } else {
                tracing::info!(
                    targets = fetched,
                    processed = stats.processed,
                    success = stats.success,
                    failed = stats.failed,
                    "oauth rate-limit refresh batch completed"
                );
            }
            if fetched < batch_size {
                break;
            }
        }

        Ok(refreshed_total)
    }

    async fn list_oauth_rate_limit_refresh_job_error_summary(
        &self,
        job_id: Uuid,
    ) -> Result<Vec<OAuthRateLimitRefreshErrorSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT error_code, count
            FROM oauth_rate_limit_refresh_job_errors
            WHERE job_id = $1
            ORDER BY count DESC, error_code ASC
            "#,
        )
        .bind(job_id)
        .fetch_all(&self.pool)
        .await
        .context("failed to query oauth rate-limit refresh job error summary")?;

        let mut summary = Vec::with_capacity(rows.len());
        for row in rows {
            let count = row.try_get::<i64, _>("count")?;
            summary.push(OAuthRateLimitRefreshErrorSummary {
                error_code: row.try_get::<String, _>("error_code")?,
                count: u64::try_from(count.max(0)).unwrap_or(u64::MAX),
            });
        }
        Ok(summary)
    }

    async fn load_oauth_rate_limit_refresh_job_summary_inner(
        &self,
        job_id: Uuid,
    ) -> Result<OAuthRateLimitRefreshJobSummary> {
        let row = sqlx::query(
            r#"
            SELECT
                id,
                status,
                total,
                processed,
                success_count,
                failed_count,
                started_at,
                finished_at,
                created_at
            FROM oauth_rate_limit_refresh_jobs
            WHERE id = $1
            "#,
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to query oauth rate-limit refresh job summary")?
        .ok_or_else(|| anyhow!("job not found"))?;

        let status =
            parse_rate_limit_refresh_job_status(row.try_get::<String, _>("status")?.as_str())?;
        let total = u64::try_from(row.try_get::<i64, _>("total")?.max(0)).unwrap_or(u64::MAX);
        let processed =
            u64::try_from(row.try_get::<i64, _>("processed")?.max(0)).unwrap_or(u64::MAX);
        let success_count =
            u64::try_from(row.try_get::<i64, _>("success_count")?.max(0)).unwrap_or(u64::MAX);
        let failed_count =
            u64::try_from(row.try_get::<i64, _>("failed_count")?.max(0)).unwrap_or(u64::MAX);
        let started_at = row.try_get::<Option<DateTime<Utc>>, _>("started_at")?;
        let finished_at = row.try_get::<Option<DateTime<Utc>>, _>("finished_at")?;
        let created_at = row.try_get::<DateTime<Utc>, _>("created_at")?;
        let end_at = finished_at.unwrap_or_else(Utc::now);
        let throughput_per_min = started_at.and_then(|started| {
            let elapsed_sec = (end_at - started).num_seconds();
            if elapsed_sec <= 0 {
                return None;
            }
            Some((processed as f64) * 60.0 / (elapsed_sec as f64))
        });
        let error_summary = self
            .list_oauth_rate_limit_refresh_job_error_summary(job_id)
            .await?;

        Ok(OAuthRateLimitRefreshJobSummary {
            job_id,
            status,
            total,
            processed,
            success_count,
            failed_count,
            started_at,
            finished_at,
            created_at,
            throughput_per_min,
            error_summary,
        })
    }

    async fn recover_rate_limit_refresh_jobs_inner(&self) -> Result<u64> {
        let running_ids = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT id
            FROM oauth_rate_limit_refresh_jobs
            WHERE status = $1
            "#,
        )
        .bind(DB_RATE_LIMIT_JOB_STATUS_RUNNING)
        .fetch_all(&self.pool)
        .await
        .context("failed to list running oauth rate-limit refresh jobs for recovery")?;
        if running_ids.is_empty() {
            return Ok(0);
        }

        let now = Utc::now();
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start oauth rate-limit refresh recovery transaction")?;

        sqlx::query(
            r#"
            UPDATE oauth_rate_limit_refresh_jobs
            SET
                status = $2,
                finished_at = COALESCE(finished_at, $3),
                updated_at = $3
            WHERE status = $1
            "#,
        )
        .bind(DB_RATE_LIMIT_JOB_STATUS_RUNNING)
        .bind(DB_RATE_LIMIT_JOB_STATUS_FAILED)
        .bind(now)
        .execute(tx.as_mut())
        .await
        .context("failed to mark running oauth rate-limit refresh jobs as failed")?;

        for job_id in &running_ids {
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
            .bind(*job_id)
            .bind("job_recovered_after_restart")
            .bind(now)
            .execute(tx.as_mut())
            .await
            .with_context(|| {
                format!(
                    "failed to append recovery error summary for oauth rate-limit refresh job {job_id}"
                )
            })?;
        }

        tx.commit()
            .await
            .context("failed to commit oauth rate-limit refresh recovery transaction")?;

        Ok(u64::try_from(running_ids.len()).unwrap_or(u64::MAX))
    }
}

fn parse_json_array(raw: Option<String>) -> Option<Vec<serde_json::Value>> {
    let value = raw
        .as_deref()
        .and_then(|payload| serde_json::from_str::<serde_json::Value>(payload).ok())?;
    match value {
        serde_json::Value::Array(items) => Some(items),
        _ => None,
    }
}
