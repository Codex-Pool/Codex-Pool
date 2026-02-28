#[derive(Debug, Clone)]
struct VaultActivationItem {
    id: Uuid,
    label: String,
    base_url: String,
    refresh_token_enc: String,
    chatgpt_account_id: Option<String>,
    chatgpt_plan_type: Option<String>,
    source_type: Option<String>,
    desired_mode: UpstreamMode,
    desired_enabled: bool,
    desired_priority: i32,
}

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
    async fn load_oauth_vault_activation_candidates(
        &self,
        now: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<VaultActivationItem>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id,
                label,
                base_url,
                refresh_token_enc,
                chatgpt_account_id,
                chatgpt_plan_type,
                source_type,
                desired_mode,
                desired_enabled,
                desired_priority
            FROM oauth_refresh_token_vault
            WHERE
                status = 'queued'
                AND (backoff_until IS NULL OR backoff_until <= $1)
                AND (next_attempt_at IS NULL OR next_attempt_at <= $1)
            ORDER BY created_at ASC
            LIMIT $2
            "#,
        )
        .bind(now)
        .bind(i64::try_from(limit).unwrap_or(i64::MAX))
        .fetch_all(&self.pool)
        .await
        .context("failed to load oauth vault activation candidates")?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            items.push(VaultActivationItem {
                id: row.try_get::<Uuid, _>("id")?,
                label: row.try_get::<String, _>("label")?,
                base_url: row.try_get::<String, _>("base_url")?,
                refresh_token_enc: row.try_get::<String, _>("refresh_token_enc")?,
                chatgpt_account_id: row.try_get::<Option<String>, _>("chatgpt_account_id")?,
                chatgpt_plan_type: row.try_get::<Option<String>, _>("chatgpt_plan_type")?,
                source_type: row.try_get::<Option<String>, _>("source_type")?,
                desired_mode: parse_upstream_mode(
                    row.try_get::<String, _>("desired_mode")?.as_str(),
                )?,
                desired_enabled: row.try_get::<bool, _>("desired_enabled")?,
                desired_priority: row.try_get::<i32, _>("desired_priority")?,
            });
        }
        Ok(items)
    }

    async fn mark_oauth_vault_activation_failed(
        &self,
        item_id: Uuid,
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
        let fatal = is_fatal_refresh_error_code(Some(error_code));
        let now = Utc::now();
        let backoff = if fatal {
            None
        } else {
            Some(vault_activation_backoff(next_failure_count))
        };
        let next_attempt_at = backoff.map(|value| now + value);
        let status = if fatal { "failed" } else { "queued" };

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
                updated_at = $8
            WHERE id = $1
            "#,
        )
        .bind(item_id)
        .bind(status)
        .bind(next_failure_count)
        .bind(next_attempt_at)
        .bind(next_attempt_at)
        .bind(error_code)
        .bind(truncate_error_message(error_message.to_string()))
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
        let limit = needed.min(batch_size);
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
                    let cipher = match self.require_credential_cipher() {
                        Ok(cipher) => cipher,
                        Err(err) => {
                            let _ = self
                                .mark_oauth_vault_activation_failed(
                                    item.id,
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
                                    "credential_decrypt_failed",
                                    &err.to_string(),
                                )
                                .await;
                            return false;
                        }
                    };

                    let req = ImportOAuthRefreshTokenRequest {
                        label: item.label,
                        base_url: item.base_url,
                        refresh_token,
                        chatgpt_account_id: item.chatgpt_account_id,
                        mode: Some(item.desired_mode),
                        enabled: Some(item.desired_enabled),
                        priority: Some(item.desired_priority),
                        chatgpt_plan_type: item.chatgpt_plan_type,
                        source_type: item.source_type,
                    };

                    match self.upsert_oauth_account(req).await {
                        Ok(_) => {
                            let _ = self.delete_oauth_vault_item(item.id).await;
                            true
                        }
                        Err(err) => {
                            let message = err.to_string();
                            let error_code = classify_vault_activation_error_code(&message);
                            let _ = self
                                .mark_oauth_vault_activation_failed(item.id, error_code, &message)
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
