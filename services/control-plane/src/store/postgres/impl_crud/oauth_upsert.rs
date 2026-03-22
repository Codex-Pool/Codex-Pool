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
        let rate_limits = match self
            .oauth_client
            .fetch_rate_limits(
                access_token,
                Some(base_url),
                chatgpt_account_id.as_deref(),
            )
            .await
        {
            Ok(rate_limits) => rate_limits,
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
            .persist_rate_limit_cache_success(account_id, rate_limits, fetched_at)
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
        let validated = self
            .validate_oauth_refresh_token_inner(ValidateOAuthRefreshTokenRequest {
                refresh_token: refresh_token.to_string(),
                base_url: Some(normalized_base_url.clone()),
            })
            .await?;
        let resolved_chatgpt_account_id = req
            .chatgpt_account_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .or(validated.chatgpt_account_id.clone());
        if self
            .canonical_oauth_account_id_by_identity(
                validated.chatgpt_account_user_id.as_deref(),
                validated.chatgpt_user_id.as_deref(),
                resolved_chatgpt_account_id.as_deref(),
            )
            .await?
            .is_some()
        {
            self.upsert_oauth_account(ImportOAuthRefreshTokenRequest {
                chatgpt_account_id: resolved_chatgpt_account_id.clone(),
                ..req
            })
            .await?;
            return Ok(false);
        }
        let refresh_token_enc = cipher.encrypt(refresh_token)?;
        let refresh_token_sha256 = refresh_token_sha256(refresh_token);
        let now = Utc::now();

        let row = sqlx::query(
            r#"
            INSERT INTO oauth_refresh_token_vault (
                id,
                refresh_token_enc,
                refresh_token_sha256,
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
                created_at,
                updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, NULL, $6, $7, $8, $9, $10, $11, 'queued', 0, NULL, $12, NULL, NULL, $12, $12
            )
            ON CONFLICT (refresh_token_sha256) DO UPDATE
            SET
                refresh_token_enc = EXCLUDED.refresh_token_enc,
                base_url = EXCLUDED.base_url,
                label = EXCLUDED.label,
                chatgpt_account_id = COALESCE(EXCLUDED.chatgpt_account_id, oauth_refresh_token_vault.chatgpt_account_id),
                chatgpt_plan_type = COALESCE(EXCLUDED.chatgpt_plan_type, oauth_refresh_token_vault.chatgpt_plan_type),
                source_type = COALESCE(EXCLUDED.source_type, oauth_refresh_token_vault.source_type),
                desired_mode = EXCLUDED.desired_mode,
                desired_enabled = EXCLUDED.desired_enabled,
                desired_priority = EXCLUDED.desired_priority,
                status = 'queued',
                failure_count = 0,
                backoff_until = NULL,
                next_attempt_at = EXCLUDED.next_attempt_at,
                last_error_code = NULL,
                last_error_message = NULL,
                updated_at = EXCLUDED.updated_at
            RETURNING (xmax = 0) AS inserted
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(refresh_token_enc)
        .bind(refresh_token_sha256)
        .bind(&normalized_base_url)
        .bind(&req.label)
        .bind(resolved_chatgpt_account_id)
        .bind(req.chatgpt_plan_type.clone())
        .bind(req.source_type.clone())
        .bind(upstream_mode_to_db(&desired_mode))
        .bind(req.enabled.unwrap_or(true))
        .bind(req.priority.unwrap_or(100))
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .context("failed to queue oauth refresh token into vault")?;

        row.try_get::<bool, _>("inserted")
            .context("failed to read vault inserted flag")
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
                refresh_token_enc,
                refresh_token_sha256,
                token_family_id,
                token_version,
                token_expires_at,
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
            VALUES ($1, $2, $3, $4, $5, 1, $6, $7, 'ok', NULL, NULL, 0, NULL, false, NULL, $8, $9)
            "#,
        )
        .bind(account_id)
        .bind(access_token_enc)
        .bind(refresh_token_enc)
        .bind(refresh_token_sha256)
        .bind(token_family_id)
        .bind(token_info.expires_at)
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
                    refresh_token_enc,
                    refresh_token_sha256,
                    token_family_id,
                    token_version,
                    token_expires_at,
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
                VALUES ($1, $2, $3, $4, $5, 1, $6, $7, 'ok', NULL, NULL, 0, NULL, false, NULL, $8, $7)
                ON CONFLICT (account_id) DO UPDATE
                SET
                    access_token_enc = EXCLUDED.access_token_enc,
                    refresh_token_enc = EXCLUDED.refresh_token_enc,
                    refresh_token_sha256 = EXCLUDED.refresh_token_sha256,
                    token_family_id = COALESCE(upstream_account_oauth_credentials.token_family_id, EXCLUDED.token_family_id),
                    token_version = GREATEST(upstream_account_oauth_credentials.token_version, 0) + 1,
                    token_expires_at = EXCLUDED.token_expires_at,
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
            .bind(refresh_token_enc)
            .bind(refresh_token_sha256)
            .bind(token_family_id)
            .bind(token_info.expires_at)
            .bind(now)
            .bind(schedule_next_oauth_refresh(token_info.expires_at, account_id))
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
