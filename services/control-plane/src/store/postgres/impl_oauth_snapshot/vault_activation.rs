fn classify_vault_activation_error_code(message: &str) -> &'static str {
    let lowered = message.to_ascii_lowercase();
    if lowered.contains("refresh token reused") {
        return "refresh_token_reused";
    }
    if lowered.contains("refresh token revoked") {
        return "refresh_token_revoked";
    }
    if lowered.contains("invalid refresh token") {
        return "invalid_refresh_token";
    }
    if lowered.contains("missing client id") {
        return "missing_client_id";
    }
    if lowered.contains("unauthorized client") {
        return "unauthorized_client";
    }
    if lowered.contains("rate_limited")
        || lowered.contains("rate limit")
        || lowered.contains("too many requests")
    {
        return "rate_limited";
    }
    if lowered.contains("upstream unavailable")
        || lowered.contains("service unavailable")
        || lowered.contains("temporarily unavailable")
    {
        return "upstream_unavailable";
    }
    "vault_activation_failed"
}

fn vault_activation_backoff(failure_count: i32) -> Duration {
    match failure_count {
        i32::MIN..=0 => Duration::seconds(30),
        1 => Duration::seconds(60),
        2 => Duration::seconds(120),
        _ => Duration::seconds(300),
    }
}

impl PostgresStore {
    async fn load_due_oauth_vault_admission_reprobe_candidates(
        &self,
        now: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<Uuid>> {
        let rows = sqlx::query(
            r#"
            SELECT id
            FROM oauth_refresh_token_vault
            WHERE
                status IN ($2, $3)
                AND admission_retry_after IS NOT NULL
                AND admission_retry_after <= $1
            ORDER BY admission_retry_after ASC, created_at ASC, id ASC
            LIMIT $4
            "#,
        )
        .bind(now)
        .bind(oauth_vault_status_to_db(OAuthVaultRecordStatus::NoQuota))
        .bind(oauth_vault_status_to_db(OAuthVaultRecordStatus::Failed))
        .bind(i64::try_from(limit).unwrap_or(i64::MAX))
        .fetch_all(&self.pool)
        .await
        .context("failed to load due oauth vault admission reprobe candidates")?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            items.push(row.try_get("id")?);
        }
        Ok(items)
    }

    async fn load_oauth_vault_activation_candidates(
        &self,
        now: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<OAuthRefreshTokenVaultRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id,
                email,
                label,
                base_url,
                refresh_token_enc,
                refresh_token_sha256,
                fallback_access_token_enc,
                fallback_token_expires_at,
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
            WHERE
                status IN ($2, $3, $4)
                AND (backoff_until IS NULL OR backoff_until <= $1)
                AND (next_attempt_at IS NULL OR next_attempt_at <= $1)
            ORDER BY created_at ASC
            LIMIT $5
            "#,
        )
        .bind(now)
        .bind(oauth_vault_status_to_db(OAuthVaultRecordStatus::Ready))
        .bind(oauth_vault_status_to_db(OAuthVaultRecordStatus::NeedsRefresh))
        .bind(oauth_vault_status_to_db(OAuthVaultRecordStatus::Queued))
        .bind(i64::try_from(limit).unwrap_or(i64::MAX))
        .fetch_all(&self.pool)
        .await
        .context("failed to load oauth vault activation candidates")?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            items.push(parse_oauth_vault_record_row(&row)?);
        }
        items.sort_by(|left, right| {
            oauth_vault_activation_priority(left.status)
                .cmp(&oauth_vault_activation_priority(right.status))
                .then_with(|| {
                    left.admission_checked_at
                        .unwrap_or(left.created_at)
                        .cmp(&right.admission_checked_at.unwrap_or(right.created_at))
                })
                .then_with(|| left.created_at.cmp(&right.created_at))
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(items)
    }

    async fn mark_oauth_vault_activation_failed(
        &self,
        item_id: Uuid,
        current_status: OAuthVaultRecordStatus,
        error_code: &str,
        error_message: &str,
    ) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to begin oauth vault failure transaction")?;
        let current_failure_count = sqlx::query_scalar::<_, Option<i32>>(
            r#"
            SELECT failure_count
            FROM oauth_refresh_token_vault
            WHERE id = $1
            FOR UPDATE
            "#,
        )
        .bind(item_id)
        .fetch_optional(tx.as_mut())
        .await
        .context("failed to load oauth vault failure count")?
        .flatten();

        let Some(current_failure_count) = current_failure_count else {
            tx.commit()
                .await
                .context("failed to commit oauth vault no-op failure transaction")?;
            return Ok(());
        };

        let next_failure_count = current_failure_count.saturating_add(1);
        let fatal_auth = is_fatal_refresh_error_code(Some(error_code));
        let terminal_config_error = matches!(
            normalize_health_error_code(error_code).as_str(),
            "credential_cipher_missing" | "credential_decrypt_failed"
        );
        let now = Utc::now();
        let allow_retry = if terminal_config_error {
            false
        } else if fatal_auth {
            can_retry_fatal_activation_failure(next_failure_count as u32)
        } else {
            true
        };
        let backoff = if allow_retry {
            Some(vault_activation_backoff(next_failure_count as u32))
        } else {
            None
        };
        let next_attempt_at = backoff.map(|value| now + value);
        let status = if allow_retry {
            oauth_vault_activation_fallback_status(current_status)
        } else {
            OAuthVaultRecordStatus::Failed
        };

        sqlx::query(
            r#"
            UPDATE oauth_refresh_token_vault
            SET
                status = $2,
                failure_count = $3,
                backoff_until = $4,
                next_attempt_at = $5,
                last_error_code = $6,
                last_error_message = $7,
                failure_stage = $8,
                attempt_count = attempt_count + 1,
                next_retry_at = $9,
                retryable = $10,
                terminal_reason = $11,
                updated_at = $12
            WHERE id = $1
            "#,
        )
        .bind(item_id)
        .bind(oauth_vault_status_to_db(status))
        .bind(next_failure_count)
        .bind(next_attempt_at)
        .bind(next_attempt_at)
        .bind(error_code)
        .bind(truncate_error_message(error_message.to_string()))
        .bind(oauth_inventory_failure_stage_to_db(
            OAuthInventoryFailureStage::ActivationRefresh,
        ))
        .bind(next_attempt_at)
        .bind(allow_retry)
        .bind(if allow_retry {
            None
        } else {
            Some(error_code.to_string())
        })
        .bind(now)
        .execute(tx.as_mut())
        .await
        .context("failed to update oauth vault activation failure")?;

        tx.commit()
            .await
            .context("failed to commit oauth vault failure transaction")?;
        Ok(())
    }

    async fn delete_oauth_vault_item(&self, item_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM oauth_refresh_token_vault
            WHERE id = $1
            "#,
        )
        .bind(item_id)
        .execute(&self.pool)
        .await
        .context("failed to delete oauth vault item after activation")?;
        Ok(())
    }

    async fn activate_oauth_refresh_token_vault_inner(&self) -> Result<u64> {
        let reprobe_limit = oauth_vault_activate_batch_size_from_env();
        for record_id in self
            .load_due_oauth_vault_admission_reprobe_candidates(Utc::now(), reprobe_limit)
            .await?
        {
            self.probe_oauth_vault_admission_inner(record_id).await?;
        }
        let _ = self.purge_expired_one_time_accounts_inner().await?;
        let runtime_count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)::BIGINT
            FROM upstream_accounts
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .context("failed to count runtime upstream accounts")?;
        let runtime_count = usize::try_from(runtime_count.max(0)).unwrap_or_default();
        let runtime_cap = runtime_pool_cap_from_env();
        if runtime_count >= runtime_cap {
            tracing::warn!(
                runtime_count,
                runtime_cap,
                "postgres runtime pool reached configured cap; skipping oauth vault activation"
            );
            return Ok(0);
        }

        let active_count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)::BIGINT
            FROM upstream_accounts
            WHERE enabled = true
              AND pool_state = $1
            "#,
        )
        .bind(POOL_STATE_ACTIVE)
        .fetch_one(&self.pool)
        .await
        .context("failed to count active upstream accounts")?;

        let active_count = usize::try_from(active_count.max(0)).unwrap_or_default();
        let target = active_pool_target_from_env();
        let active_min = active_pool_min_from_env().min(target);
        if active_count >= target {
            return Ok(0);
        }
        if active_count < active_min {
            tracing::warn!(
                active_count,
                active_min,
                target,
                "active oauth pool dropped below configured minimum"
            );
        }

        let batch_size = oauth_vault_activate_batch_size_from_env();
        let needed = target.saturating_sub(active_count);
        let headroom = runtime_cap.saturating_sub(runtime_count);
        let limit = needed.min(batch_size).min(headroom);
        if limit == 0 {
            return Ok(0);
        }

        let candidates = self
            .load_oauth_vault_activation_candidates(Utc::now(), limit)
            .await?;
        if candidates.is_empty() {
            return Ok(0);
        }

        let concurrency = oauth_vault_activate_concurrency_from_env();
        let max_rps = oauth_vault_activate_max_rps_from_env();
        let launch_interval = std::time::Duration::from_secs_f64(1.0 / f64::from(max_rps));
        let throttle = std::sync::Arc::new(tokio::sync::Mutex::new(tokio::time::Instant::now()));

        let results = futures_util::stream::iter(candidates.into_iter())
            .map(|item| {
                let throttle = throttle.clone();
                async move {
                    throttle_refresh_start(throttle.as_ref(), launch_interval).await;
                    let activation_result = match item.status {
                        OAuthVaultRecordStatus::Ready => {
                            self.materialize_ready_oauth_vault_record_inner(&item).await
                        }
                        OAuthVaultRecordStatus::NeedsRefresh | OAuthVaultRecordStatus::Queued => {
                            let cipher = match self.require_credential_cipher() {
                                Ok(cipher) => cipher,
                                Err(err) => {
                                    let _ = self
                                        .mark_oauth_vault_activation_failed(
                                            item.id,
                                            item.status,
                                            "credential_cipher_missing",
                                            &err.to_string(),
                                        )
                                        .await;
                                    return false;
                                }
                            };
                            let refresh_token = match cipher.decrypt(&item.refresh_token_enc) {
                                Ok(value) if !value.trim().is_empty() => value,
                                Ok(_) => {
                                    let _ = self
                                        .mark_oauth_vault_activation_failed(
                                            item.id,
                                            item.status,
                                            "invalid_refresh_token",
                                            "refresh token is empty",
                                        )
                                        .await;
                                    return false;
                                }
                                Err(err) => {
                                    let _ = self
                                        .mark_oauth_vault_activation_failed(
                                            item.id,
                                            item.status,
                                            "credential_decrypt_failed",
                                            &err.to_string(),
                                        )
                                        .await;
                                    return false;
                                }
                            };
                            let fallback_access_token = match item.fallback_access_token_enc.as_deref() {
                                Some(token_enc) => match cipher.decrypt(token_enc) {
                                    Ok(value) if !value.trim().is_empty() => Some(value),
                                    Ok(_) => None,
                                    Err(err) => {
                                        let _ = self
                                            .mark_oauth_vault_activation_failed(
                                                item.id,
                                                item.status,
                                                "credential_decrypt_failed",
                                                &err.to_string(),
                                            )
                                            .await;
                                        return false;
                                    }
                                },
                                None => None,
                            };

                            let req = ImportOAuthRefreshTokenRequest {
                                label: item.label.clone(),
                                base_url: item.base_url.clone(),
                                refresh_token,
                                fallback_access_token,
                                fallback_token_expires_at: item.fallback_token_expires_at,
                                chatgpt_account_id: item.chatgpt_account_id.clone(),
                                mode: Some(item.desired_mode.clone()),
                                enabled: Some(item.desired_enabled),
                                priority: Some(item.desired_priority),
                                chatgpt_plan_type: item.chatgpt_plan_type.clone(),
                                source_type: item.source_type.clone(),
                            };

                            self.upsert_oauth_account(req).await.map(|_| ())
                        }
                        OAuthVaultRecordStatus::NoQuota | OAuthVaultRecordStatus::Failed => {
                            return false;
                        }
                    };

                    match activation_result {
                        Ok(_) => {
                            let _ = self.delete_oauth_vault_item(item.id).await;
                            true
                        }
                        Err(err) => {
                            let message = err.to_string();
                            let error_code = classify_vault_activation_error_code(&message);
                            let _ = self
                                .mark_oauth_vault_activation_failed(
                                    item.id,
                                    item.status,
                                    error_code,
                                    &message,
                                )
                                .await;
                            false
                        }
                    }
                }
            })
            .buffer_unordered(concurrency.max(1))
            .collect::<Vec<_>>()
            .await;

        Ok(u64::try_from(results.into_iter().filter(|ok| *ok).count()).unwrap_or(u64::MAX))
    }
}
