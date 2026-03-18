#[derive(Debug, Clone)]
struct InMemoryRateLimitRefreshTarget {
    account_id: Uuid,
    base_url: String,
    chatgpt_account_id: Option<String>,
    access_token_enc: String,
}

#[derive(Debug, Default)]
struct InMemoryRateLimitRefreshBatchStats {
    processed: u64,
    success: u64,
    failed: u64,
    error_counts: HashMap<String, u64>,
}

impl InMemoryStore {
    fn canonical_oauth_account_id_by_identity(
        &self,
        target_chatgpt_account_user_id: Option<&str>,
        target_chatgpt_user_id: Option<&str>,
        target_chatgpt_account_id: Option<&str>,
    ) -> Option<Uuid> {
        let normalized_target_account_user_id = target_chatgpt_account_user_id
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let normalized_target_user_id = target_chatgpt_user_id
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let normalized_target_account_id = target_chatgpt_account_id
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if normalized_target_account_user_id.is_none()
            && (normalized_target_user_id.is_none() || normalized_target_account_id.is_none())
        {
            return None;
        }

        let providers = self.account_auth_providers.read().unwrap();
        let accounts = self.accounts.read().unwrap();
        let profiles = self.session_profiles.read().unwrap();

        accounts
            .values()
            .filter(|account| {
                providers
                    .get(&account.id)
                    .is_some_and(|provider| *provider == UpstreamAuthProvider::OAuthRefreshToken)
                    && profiles.get(&account.id).is_some_and(|profile| {
                        if let Some(target_account_user_id) = normalized_target_account_user_id {
                            return profile.chatgpt_account_user_id.as_deref().map(str::trim)
                                == Some(target_account_user_id);
                        }

                        if let (Some(target_user_id), Some(target_account_id)) = (
                            normalized_target_user_id,
                            normalized_target_account_id,
                        ) {
                            return profile.chatgpt_user_id.as_deref().map(str::trim)
                                == Some(target_user_id)
                                && account.chatgpt_account_id.as_deref().map(str::trim)
                                    == Some(target_account_id);
                        }

                        false
                    })
            })
            .max_by(|left, right| {
                left.created_at
                    .cmp(&right.created_at)
                    .then_with(|| left.id.cmp(&right.id))
            })
            .map(|account| account.id)
    }

    fn dedupe_oauth_accounts_by_identity_inner(
        &self,
        target_chatgpt_account_user_id: Option<&str>,
        target_chatgpt_user_id: Option<&str>,
        target_chatgpt_account_id: Option<&str>,
    ) -> u64 {
        let providers = self.account_auth_providers.read().unwrap().clone();
        let accounts = self.accounts.read().unwrap().clone();
        let profiles = self.session_profiles.read().unwrap().clone();

        let target_key = target_chatgpt_account_user_id
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

        let mut grouped: std::collections::HashMap<String, Vec<UpstreamAccount>> =
            std::collections::HashMap::new();
        for account in accounts.values() {
            if !providers
                .get(&account.id)
                .is_some_and(|provider| *provider == UpstreamAuthProvider::OAuthRefreshToken)
            {
                continue;
            }
            let Some(identity_key) = profiles.get(&account.id).and_then(|profile| {
                profile
                    .chatgpt_account_user_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| format!("account_user:{value}"))
                    .or_else(|| {
                        profile
                            .chatgpt_user_id
                            .as_deref()
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .zip(
                                account
                                    .chatgpt_account_id
                                    .as_deref()
                                    .map(str::trim)
                                    .filter(|value| !value.is_empty()),
                            )
                            .map(|(user_id, account_id)| {
                                format!("user_account:{user_id}:{account_id}")
                            })
                    })
            }) else {
                continue;
            };
            if target_key
                .as_deref()
                .is_some_and(|target| target != identity_key.as_str())
            {
                continue;
            }
            grouped.entry(identity_key).or_default().push(account.clone());
        }

        let mut duplicate_ids = Vec::new();
        for items in grouped.values_mut() {
            items.sort_by(|left, right| {
                right
                    .created_at
                    .cmp(&left.created_at)
                    .then_with(|| right.id.cmp(&left.id))
            });
            duplicate_ids.extend(items.iter().skip(1).map(|account| account.id));
        }

        if duplicate_ids.is_empty() {
            return 0;
        }

        {
            let mut accounts = self.accounts.write().unwrap();
            let mut providers = self.account_auth_providers.write().unwrap();
            let mut credentials = self.oauth_credentials.write().unwrap();
            let mut profiles = self.session_profiles.write().unwrap();
            let mut health = self.account_health_states.write().unwrap();
            let mut rate_limit_caches = self.oauth_rate_limit_caches.write().unwrap();

            for account_id in &duplicate_ids {
                accounts.remove(account_id);
                providers.remove(account_id);
                credentials.remove(account_id);
                profiles.remove(account_id);
                health.remove(account_id);
                rate_limit_caches.remove(account_id);
            }
        }

        self.revision.fetch_add(1, Ordering::Relaxed);
        duplicate_ids.len() as u64
    }

    fn load_rate_limit_refresh_targets(
        &self,
        after_id: Option<Uuid>,
        limit: usize,
        due_only: bool,
    ) -> Vec<InMemoryRateLimitRefreshTarget> {
        let now = Utc::now();
        let accounts = self.accounts.read().unwrap().clone();
        let providers = self.account_auth_providers.read().unwrap().clone();
        let credentials = self.oauth_credentials.read().unwrap().clone();
        let caches = self.oauth_rate_limit_caches.read().unwrap().clone();

        let mut targets = accounts
            .values()
            .filter_map(|account| {
                if after_id.is_some_and(|cursor| account.id <= cursor) {
                    return None;
                }
                if !account.enabled {
                    return None;
                }
                if !providers
                    .get(&account.id)
                    .is_some_and(|provider| *provider == UpstreamAuthProvider::OAuthRefreshToken)
                {
                    return None;
                }

                let credential = credentials.get(&account.id)?;
                if credential.token_expires_at <= now + Duration::seconds(OAUTH_MIN_VALID_SEC) {
                    return None;
                }
                if credential.refresh_reused_detected {
                    return None;
                }
                if matches!(credential.last_refresh_status, OAuthRefreshStatus::Failed)
                    && is_fatal_refresh_error_code(
                        credential.last_refresh_error_code.as_deref(),
                    )
                {
                    return None;
                }

                let cache = caches.get(&account.id).cloned().unwrap_or_default();
                if due_only && cache.expires_at.is_some_and(|expires_at| expires_at > now) {
                    return None;
                }

                Some(InMemoryRateLimitRefreshTarget {
                    account_id: account.id,
                    base_url: account.base_url.clone(),
                    chatgpt_account_id: account.chatgpt_account_id.clone(),
                    access_token_enc: credential.access_token_enc.clone(),
                })
            })
            .collect::<Vec<_>>();

        targets.sort_by(|left, right| left.account_id.cmp(&right.account_id));
        targets.truncate(limit);
        targets
    }

    fn count_rate_limit_refresh_targets(&self) -> u64 {
        let now = Utc::now();
        let accounts = self.accounts.read().unwrap().clone();
        let providers = self.account_auth_providers.read().unwrap().clone();
        let credentials = self.oauth_credentials.read().unwrap().clone();

        accounts
            .values()
            .filter(|account| account.enabled)
            .filter(|account| {
                providers
                    .get(&account.id)
                    .is_some_and(|provider| *provider == UpstreamAuthProvider::OAuthRefreshToken)
            })
            .filter_map(|account| credentials.get(&account.id))
            .filter(|credential| {
                credential.token_expires_at > now + Duration::seconds(OAUTH_MIN_VALID_SEC)
                    && !credential.refresh_reused_detected
                    && !(matches!(credential.last_refresh_status, OAuthRefreshStatus::Failed)
                        && is_fatal_refresh_error_code(
                            credential.last_refresh_error_code.as_deref(),
                        ))
            })
            .count() as u64
    }

    async fn fetch_live_rate_limits_result(
        &self,
        target: &InMemoryRateLimitRefreshTarget,
    ) -> Result<Vec<OAuthRateLimitSnapshot>, (String, String)> {
        let Some(cipher) = self.credential_cipher.as_ref() else {
            return Err((
                "credential_cipher_missing".to_string(),
                "oauth credential cipher is not configured".to_string(),
            ));
        };
        let access_token = cipher
            .decrypt(&target.access_token_enc)
            .map_err(|err| ("credential_decrypt_failed".to_string(), err.to_string()))?;
        if access_token.trim().is_empty() {
            return Ok(Vec::new());
        }

        self.oauth_client
            .fetch_rate_limits(
                &access_token,
                Some(&target.base_url),
                target.chatgpt_account_id.as_deref(),
            )
            .await
            .map_err(|err| (err.code().as_str().to_string(), err.to_string()))
    }

    fn persist_rate_limit_cache_success_inner(
        &self,
        account_id: Uuid,
        rate_limits: Vec<OAuthRateLimitSnapshot>,
        fetched_at: DateTime<Utc>,
    ) {
        let (blocked_until, block_reason) = derive_rate_limit_block(&rate_limits, fetched_at);
        let expires_at = blocked_until
            .unwrap_or_else(|| fetched_at + Duration::seconds(rate_limit_cache_ttl_sec_from_env()));
        let last_error = block_reason.as_deref().map(rate_limit_block_message);
        self.oauth_rate_limit_caches.write().unwrap().insert(
            account_id,
            OAuthRateLimitCacheRecord {
                rate_limits,
                fetched_at: Some(fetched_at),
                expires_at: Some(expires_at),
                last_error_code: block_reason,
                last_error,
            },
        );
        self.revision.fetch_add(1, Ordering::Relaxed);
    }

    fn persist_rate_limit_cache_failure_inner(
        &self,
        account_id: Uuid,
        error_code: &str,
        error_message: &str,
    ) {
        let now = Utc::now();
        let backoff_sec = rate_limit_failure_backoff_seconds(error_code, error_message);
        let truncated_message = truncate_error_message(error_message.to_string());
        let mut caches = self.oauth_rate_limit_caches.write().unwrap();
        let cache = caches.entry(account_id).or_default();
        if cache.rate_limits.is_empty() {
            cache.fetched_at = None;
        }
        cache.expires_at = Some(now + Duration::seconds(backoff_sec));
        cache.last_error_code = Some(error_code.to_string());
        cache.last_error = Some(truncated_message);
        drop(caches);

        self.revision.fetch_add(1, Ordering::Relaxed);
    }

    fn append_rate_limit_refresh_job_progress(
        &self,
        job_id: Uuid,
        stats: &InMemoryRateLimitRefreshBatchStats,
    ) -> Result<()> {
        let mut jobs = self.oauth_rate_limit_refresh_jobs.write().unwrap();
        let summary = jobs
            .get_mut(&job_id)
            .ok_or_else(|| anyhow!("job not found"))?;
        summary.processed = summary.processed.saturating_add(stats.processed);
        summary.success_count = summary.success_count.saturating_add(stats.success);
        summary.failed_count = summary.failed_count.saturating_add(stats.failed);
        for (error_code, count) in &stats.error_counts {
            if let Some(existing) = summary
                .error_summary
                .iter_mut()
                .find(|item| item.error_code == *error_code)
            {
                existing.count = existing.count.saturating_add(*count);
            } else {
                summary.error_summary.push(OAuthRateLimitRefreshErrorSummary {
                    error_code: error_code.clone(),
                    count: *count,
                });
            }
        }
        summary
            .error_summary
            .sort_by(|left, right| right.count.cmp(&left.count).then_with(|| left.error_code.cmp(&right.error_code)));
        Ok(())
    }

    fn finish_rate_limit_refresh_job(
        &self,
        job_id: Uuid,
        status: OAuthRateLimitRefreshJobStatus,
    ) -> Result<()> {
        let mut jobs = self.oauth_rate_limit_refresh_jobs.write().unwrap();
        let summary = jobs
            .get_mut(&job_id)
            .ok_or_else(|| anyhow!("job not found"))?;
        let finished_at = Utc::now();
        summary.status = status;
        summary.finished_at = Some(finished_at);
        summary.throughput_per_min = summary.started_at.and_then(|started_at| {
            let elapsed_sec = (finished_at - started_at).num_seconds();
            if elapsed_sec <= 0 {
                return None;
            }
            Some((summary.processed as f64) * 60.0 / (elapsed_sec as f64))
        });
        Ok(())
    }

    fn mark_rate_limit_refresh_job_failed(
        &self,
        job_id: Uuid,
        error_code: &str,
    ) -> Result<()> {
        let mut jobs = self.oauth_rate_limit_refresh_jobs.write().unwrap();
        let summary = jobs
            .get_mut(&job_id)
            .ok_or_else(|| anyhow!("job not found"))?;
        summary.status = OAuthRateLimitRefreshJobStatus::Failed;
        summary.failed_count = summary.failed_count.saturating_add(1);
        if let Some(existing) = summary
            .error_summary
            .iter_mut()
            .find(|item| item.error_code == error_code)
        {
            existing.count = existing.count.saturating_add(1);
        } else {
            summary.error_summary.push(OAuthRateLimitRefreshErrorSummary {
                error_code: error_code.to_string(),
                count: 1,
            });
        }
        let finished_at = Utc::now();
        summary.finished_at = Some(finished_at);
        summary.throughput_per_min = summary.started_at.and_then(|started_at| {
            let elapsed_sec = (finished_at - started_at).num_seconds();
            if elapsed_sec <= 0 {
                return None;
            }
            Some((summary.processed as f64) * 60.0 / (elapsed_sec as f64))
        });
        Ok(())
    }

    async fn refresh_rate_limit_targets_batch(
        &self,
        targets: Vec<InMemoryRateLimitRefreshTarget>,
        concurrency: usize,
    ) -> InMemoryRateLimitRefreshBatchStats {
        let results = futures_util::stream::iter(targets.into_iter())
            .map(|target| async move {
                let fetched_at = Utc::now();
                match self.fetch_live_rate_limits_result(&target).await {
                    Ok(rate_limits) => {
                        self.persist_rate_limit_cache_success_inner(
                            target.account_id,
                            rate_limits,
                            fetched_at,
                        );
                        (true, None)
                    }
                    Err((error_code, error_message)) => {
                        self.persist_rate_limit_cache_failure_inner(
                            target.account_id,
                            &error_code,
                            &error_message,
                        );
                        (false, Some(error_code))
                    }
                }
            })
            .buffer_unordered(concurrency.max(1))
            .collect::<Vec<_>>()
            .await;

        let mut stats = InMemoryRateLimitRefreshBatchStats {
            processed: results.len() as u64,
            ..Default::default()
        };
        for (success, error_code) in results {
            if success {
                stats.success = stats.success.saturating_add(1);
            } else {
                stats.failed = stats.failed.saturating_add(1);
                if let Some(error_code) = error_code {
                    *stats.error_counts.entry(error_code).or_insert(0) += 1;
                }
            }
        }
        stats
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

    async fn import_oauth_refresh_token_inner(
        &self,
        req: ImportOAuthRefreshTokenRequest,
    ) -> Result<UpstreamAccount> {
        let cipher = self.require_credential_cipher()?;
        let resolved_mode = resolve_oauth_import_mode(req.mode.clone(), req.source_type.as_deref());
        let token_info = self
            .oauth_client
            .refresh_token(&req.refresh_token, Some(&req.base_url))
            .await
            .map_err(|err| anyhow!(err.to_string()))?;

        let account = UpstreamAccount {
            id: Uuid::new_v4(),
            label: req.label,
            mode: resolved_mode,
            base_url: req.base_url,
            bearer_token: OAUTH_MANAGED_BEARER_SENTINEL.to_string(),
            chatgpt_account_id: req
                .chatgpt_account_id
                .or(token_info.chatgpt_account_id.clone()),
            enabled: req.enabled.unwrap_or(true),
            priority: req.priority.unwrap_or(100),
            created_at: Utc::now(),
        };

        let oauth_credential = OAuthCredentialRecord::from_token_info(cipher, &token_info)?;

        self.accounts
            .write()
            .unwrap()
            .insert(account.id, account.clone());
        self.account_auth_providers
            .write()
            .unwrap()
            .insert(account.id, UpstreamAuthProvider::OAuthRefreshToken);
        self.upsert_oauth_credential(account.id, oauth_credential);
        self.upsert_session_profile(
            account.id,
            SessionProfileRecord::from_oauth_token_info(
                &token_info,
                SessionCredentialKind::RefreshRotatable,
                req.chatgpt_plan_type,
                req.source_type,
            ),
        );
        self.revision.fetch_add(1, Ordering::Relaxed);

        Ok(account)
    }

    async fn upsert_oauth_refresh_token_inner(
        &self,
        req: ImportOAuthRefreshTokenRequest,
    ) -> Result<OAuthUpsertResult> {
        let cipher = self.require_credential_cipher()?;
        let resolved_mode = resolve_oauth_import_mode(req.mode.clone(), req.source_type.as_deref());
        let token_info = self
            .oauth_client
            .refresh_token(&req.refresh_token, Some(&req.base_url))
            .await
            .map_err(|err| anyhow!(err.to_string()))?;

        let normalized_chatgpt_account_id = req
            .chatgpt_account_id
            .clone()
            .or(token_info.chatgpt_account_id.clone());
        let normalized_refresh_hash = refresh_token_sha256(&token_info.refresh_token);

        let matched_account_id = {
            let accounts = self.accounts.read().unwrap();
            let providers = self.account_auth_providers.read().unwrap();
            let credentials = self.oauth_credentials.read().unwrap();

            accounts.iter().find_map(|(account_id, _account)| {
                let provider = providers
                    .get(account_id)
                    .cloned()
                    .unwrap_or(UpstreamAuthProvider::LegacyBearer);
                if provider != UpstreamAuthProvider::OAuthRefreshToken {
                    return None;
                }

                credentials
                    .get(account_id)
                    .filter(|credential| credential.refresh_token_sha256 == normalized_refresh_hash)
                    .map(|_| *account_id)
            }).or_else(|| {
                self.canonical_oauth_account_id_by_identity(
                    token_info.chatgpt_account_user_id.as_deref(),
                    token_info.chatgpt_user_id.as_deref(),
                    normalized_chatgpt_account_id.as_deref(),
                )
            })
        };

        if let Some(account_id) = matched_account_id {
            let mut accounts = self.accounts.write().unwrap();
            let Some(account) = accounts.get_mut(&account_id) else {
                return Err(anyhow!("matched oauth account is missing"));
            };
            account.label = req.label;
            account.mode = resolved_mode;
            account.base_url = req.base_url;
            account.bearer_token = OAUTH_MANAGED_BEARER_SENTINEL.to_string();
            account.chatgpt_account_id = normalized_chatgpt_account_id.clone();
            account.enabled = req.enabled.unwrap_or(true);
            account.priority = req.priority.unwrap_or(100);

            let mut credential = OAuthCredentialRecord::from_token_info(cipher, &token_info)?;
            if let Some(existing) = self.oauth_credentials.read().unwrap().get(&account_id) {
                credential.token_family_id = existing.token_family_id.clone();
                credential.token_version = existing.token_version.saturating_add(1);
            }
            drop(accounts);
            self.account_auth_providers
                .write()
                .unwrap()
                .insert(account_id, UpstreamAuthProvider::OAuthRefreshToken);
            self.upsert_oauth_credential(account_id, credential);
            let existing_profile = self
                .session_profiles
                .read()
                .unwrap()
                .get(&account_id)
                .cloned();
            self.upsert_session_profile(
                account_id,
                existing_profile
                    .unwrap_or_else(|| {
                        SessionProfileRecord::from_oauth_token_info(
                            &token_info,
                            SessionCredentialKind::RefreshRotatable,
                            None,
                            None,
                        )
                    })
                    .merge_oauth_token_info(
                        &token_info,
                        SessionCredentialKind::RefreshRotatable,
                        req.chatgpt_plan_type,
                        req.source_type,
                    ),
            );
            self.revision.fetch_add(1, Ordering::Relaxed);

            let account = self
                .accounts
                .read()
                .unwrap()
                .get(&account_id)
                .cloned()
                .ok_or_else(|| anyhow!("updated oauth account not found"))?;
            self.dedupe_oauth_accounts_by_identity_inner(
                token_info.chatgpt_account_user_id.as_deref(),
                token_info.chatgpt_user_id.as_deref(),
                normalized_chatgpt_account_id.as_deref(),
            );
            return Ok(OAuthUpsertResult {
                account,
                created: false,
            });
        }

        let account = self.import_oauth_refresh_token_inner(req).await?;
        self.dedupe_oauth_accounts_by_identity_inner(
            token_info.chatgpt_account_user_id.as_deref(),
            token_info.chatgpt_user_id.as_deref(),
            normalized_chatgpt_account_id.as_deref(),
        );
        Ok(OAuthUpsertResult {
            account,
            created: true,
        })
    }

    fn upsert_one_time_session_account_inner(
        &self,
        req: UpsertOneTimeSessionAccountRequest,
    ) -> Result<OAuthUpsertResult> {
        let normalized_chatgpt_account_id = req
            .chatgpt_account_id
            .clone()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let normalized_label = req.label.trim().to_string();
        if normalized_label.is_empty() {
            return Err(anyhow!("label is required"));
        }

        let matched_account_id = {
            let accounts = self.accounts.read().unwrap();
            let providers = self.account_auth_providers.read().unwrap();

            accounts.iter().find_map(|(account_id, account)| {
                let provider = providers
                    .get(account_id)
                    .cloned()
                    .unwrap_or(UpstreamAuthProvider::LegacyBearer);
                if provider != UpstreamAuthProvider::LegacyBearer {
                    return None;
                }
                if let Some(target_account_id) = normalized_chatgpt_account_id.as_deref() {
                    if account.chatgpt_account_id.as_deref() == Some(target_account_id) {
                        return Some(*account_id);
                    }
                }

                if account.mode != req.mode {
                    return None;
                }

                if normalized_chatgpt_account_id.is_none() && account.label == normalized_label {
                    return Some(*account_id);
                }

                None
            })
        };

        if let Some(account_id) = matched_account_id {
            let mut accounts = self.accounts.write().unwrap();
            let Some(account) = accounts.get_mut(&account_id) else {
                return Err(anyhow!("matched one-time account is missing"));
            };
            account.label = normalized_label.clone();
            account.mode = req.mode;
            account.base_url = req.base_url;
            account.bearer_token = req.access_token;
            account.chatgpt_account_id = normalized_chatgpt_account_id.clone();
            account.enabled = req.enabled.unwrap_or(true);
            account.priority = req.priority.unwrap_or(100);
            drop(accounts);

            self.account_auth_providers
                .write()
                .unwrap()
                .insert(account_id, UpstreamAuthProvider::LegacyBearer);
        self.upsert_session_profile(
            account_id,
            SessionProfileRecord::one_time_access_token(
                req.token_expires_at,
                req.chatgpt_plan_type,
                req.source_type,
            ),
        );
            self.revision.fetch_add(1, Ordering::Relaxed);

            let account = self
                .accounts
                .read()
                .unwrap()
                .get(&account_id)
                .cloned()
                .ok_or_else(|| anyhow!("updated one-time account not found"))?;

            return Ok(OAuthUpsertResult {
                account,
                created: false,
            });
        }

        let account = UpstreamAccount {
            id: Uuid::new_v4(),
            label: normalized_label,
            mode: req.mode,
            base_url: req.base_url,
            bearer_token: req.access_token,
            chatgpt_account_id: normalized_chatgpt_account_id,
            enabled: req.enabled.unwrap_or(true),
            priority: req.priority.unwrap_or(100),
            created_at: Utc::now(),
        };

        self.accounts
            .write()
            .unwrap()
            .insert(account.id, account.clone());
        self.account_auth_providers
            .write()
            .unwrap()
            .insert(account.id, UpstreamAuthProvider::LegacyBearer);
        self.upsert_session_profile(
            account.id,
            SessionProfileRecord::one_time_access_token(
                req.token_expires_at,
                req.chatgpt_plan_type,
                req.source_type,
            ),
        );
        self.revision.fetch_add(1, Ordering::Relaxed);

        Ok(OAuthUpsertResult {
            account,
            created: true,
        })
    }

    async fn refresh_oauth_account_inner(
        &self,
        account_id: Uuid,
        force: bool,
    ) -> Result<OAuthAccountStatusResponse> {
        let provider = self.account_auth_provider(account_id);
        if provider != UpstreamAuthProvider::OAuthRefreshToken {
            return Err(anyhow!("account is not an oauth account"));
        }

        let mut account = self
            .accounts
            .read()
            .unwrap()
            .get(&account_id)
            .cloned()
            .ok_or_else(|| anyhow!("account not found"))?;

        let mut oauth_credential = self
            .oauth_credentials
            .read()
            .unwrap()
            .get(&account_id)
            .cloned()
            .ok_or_else(|| anyhow!("oauth credential not found"))?;
        let session_profile = self
            .session_profiles
            .read()
            .unwrap()
            .get(&account_id)
            .cloned();

        let now = Utc::now();
        let should_refresh = force
            || oauth_credential.token_expires_at
                <= now + Duration::seconds(OAUTH_REFRESH_WINDOW_SEC);
        if !should_refresh {
            return Ok(self.oauth_status_from(
                &account,
                UpstreamAuthProvider::OAuthRefreshToken,
                Some(&oauth_credential),
                session_profile.as_ref(),
            ));
        }

        if oauth_credential
            .refresh_backoff_until
            .is_some_and(|until| until > now)
        {
            return Ok(self.oauth_status_from(
                &account,
                UpstreamAuthProvider::OAuthRefreshToken,
                Some(&oauth_credential),
                session_profile.as_ref(),
            ));
        }

        let cipher = self.require_credential_cipher()?;
        let refresh_token = cipher.decrypt(&oauth_credential.refresh_token_enc)?;

        match self
            .oauth_client
            .refresh_token(&refresh_token, Some(&account.base_url))
            .await
        {
            Ok(token_info) => {
                let previous_family_id = oauth_credential.token_family_id.clone();
                let previous_version = oauth_credential.token_version;
                oauth_credential = OAuthCredentialRecord::from_token_info(cipher, &token_info)?;
                oauth_credential.token_family_id = previous_family_id;
                oauth_credential.token_version = previous_version.saturating_add(1);
                self.upsert_oauth_credential(account_id, oauth_credential.clone());
                if token_info.chatgpt_account_id.is_some() {
                    let mut accounts = self.accounts.write().unwrap();
                    if let Some(stored_account) = accounts.get_mut(&account_id) {
                        stored_account.chatgpt_account_id = token_info.chatgpt_account_id.clone();
                        account.chatgpt_account_id = stored_account.chatgpt_account_id.clone();
                    }
                }
                let existing_profile = self
                    .session_profiles
                    .read()
                    .unwrap()
                    .get(&account_id)
                    .cloned();
                if let Some(mut profile) = existing_profile {
                    profile = profile.merge_oauth_token_info(
                        &token_info,
                        SessionCredentialKind::RefreshRotatable,
                        None,
                        None,
                    );
                    self.upsert_session_profile(account_id, profile);
                } else {
                    self.upsert_session_profile(
                        account_id,
                        SessionProfileRecord::from_oauth_token_info(
                            &token_info,
                            SessionCredentialKind::RefreshRotatable,
                            None,
                            None,
                        ),
                    );
                }
                self.revision.fetch_add(1, Ordering::Relaxed);
            }
            Err(err) => {
                oauth_credential.last_refresh_status = OAuthRefreshStatus::Failed;
                oauth_credential.last_refresh_at = Some(now);
                oauth_credential.refresh_reused_detected = matches!(
                    err.code().as_str(),
                    "refresh_token_reused" | "refresh_token_revoked"
                );
                oauth_credential.last_refresh_error_code = Some(err.code().as_str().to_string());
                oauth_credential.last_refresh_error = Some(truncate_error_message(err.to_string()));
                oauth_credential.refresh_failure_count =
                    oauth_credential.refresh_failure_count.saturating_add(1);
                oauth_credential.refresh_backoff_until =
                    Some(now + oauth_credential.backoff_duration());
                self.upsert_oauth_credential(account_id, oauth_credential.clone());
                if oauth_credential.refresh_reused_detected {
                    let family_id = oauth_credential.token_family_id.clone();
                    self.disable_oauth_family_inner(&family_id);
                }
            }
        }

        Ok(self.oauth_status_from(
            &account,
            UpstreamAuthProvider::OAuthRefreshToken,
            Some(&oauth_credential),
            session_profile.as_ref(),
        ))
    }

    async fn oauth_account_status_inner(
        &self,
        account_id: Uuid,
    ) -> Result<OAuthAccountStatusResponse> {
        let account = self
            .accounts
            .read()
            .unwrap()
            .get(&account_id)
            .cloned()
            .ok_or_else(|| anyhow!("account not found"))?;
        let provider = self.account_auth_provider(account_id);
        let oauth_credential = self
            .oauth_credentials
            .read()
            .unwrap()
            .get(&account_id)
            .cloned();
        let session_profile = self
            .session_profiles
            .read()
            .unwrap()
            .get(&account_id)
            .cloned();

        let session_profile = self
            .maybe_backfill_workspace_name_from_probe_inner(
                &account,
                provider.clone(),
                oauth_credential.as_ref(),
                session_profile,
            )
            .await;

        Ok(self.oauth_status_from(
            &account,
            provider,
            oauth_credential.as_ref(),
            session_profile.as_ref(),
        ))
    }

    async fn maybe_backfill_workspace_name_from_probe_inner(
        &self,
        account: &UpstreamAccount,
        provider: UpstreamAuthProvider,
        oauth_credential: Option<&OAuthCredentialRecord>,
        session_profile: Option<SessionProfileRecord>,
    ) -> Option<SessionProfileRecord> {
        let mut profile = session_profile?;
        if provider != UpstreamAuthProvider::OAuthRefreshToken {
            return Some(profile);
        }
        if !profile
            .chatgpt_plan_type
            .as_deref()
            .is_some_and(|plan| plan.trim().eq_ignore_ascii_case("team"))
        {
            return Some(profile);
        }
        if profile
            .workspace_name
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        {
            return Some(profile);
        }
        let Some(chatgpt_account_id) = account
            .chatgpt_account_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Some(profile);
        };
        let Some(credential) = oauth_credential else {
            return Some(profile);
        };
        let cipher = match self.require_credential_cipher() {
            Ok(cipher) => cipher,
            Err(err) => {
                tracing::warn!(
                    account_id = %account.id,
                    error = %err,
                    "workspace name probe skipped because credential cipher is unavailable"
                );
                return Some(profile);
            }
        };
        let access_token = match cipher.decrypt(&credential.access_token_enc) {
            Ok(token) if !token.trim().is_empty() => token,
            Ok(_) => return Some(profile),
            Err(err) => {
                tracing::warn!(
                    account_id = %account.id,
                    error = %err,
                    "workspace name probe skipped because access token decrypt failed"
                );
                return Some(profile);
            }
        };
        let workspace_name = match self
            .oauth_client
            .fetch_workspace_name(
                &access_token,
                Some(&account.base_url),
                Some(chatgpt_account_id),
            )
            .await
        {
            Ok(Some(name)) if !name.trim().is_empty() => name.trim().to_string(),
            Ok(_) => return Some(profile),
            Err(err) => {
                tracing::warn!(
                    account_id = %account.id,
                    error = %err,
                    "workspace name probe failed during oauth status lookup"
                );
                return Some(profile);
            }
        };

        profile.workspace_name = Some(workspace_name);
        self.upsert_session_profile(account.id, profile.clone());
        Some(profile)
    }

    async fn refresh_expiring_oauth_accounts_inner(&self) {
        self.purge_expired_one_time_accounts_inner();
        let batch_size = oauth_refresh_batch_size_from_env();
        let concurrency = oauth_refresh_concurrency_from_env();

        loop {
            let now = Utc::now();
            let account_ids = {
                let accounts = self.accounts.read().unwrap();
                let providers = self.account_auth_providers.read().unwrap();
                let oauth_credentials = self.oauth_credentials.read().unwrap();

                accounts
                    .iter()
                    .filter_map(|(account_id, account)| {
                        if !account.enabled {
                            return None;
                        }

                        let provider = providers
                            .get(account_id)
                            .cloned()
                            .unwrap_or(UpstreamAuthProvider::LegacyBearer);
                        if provider != UpstreamAuthProvider::OAuthRefreshToken {
                            return None;
                        }

                        let credential = oauth_credentials.get(account_id)?;
                        if credential
                            .refresh_backoff_until
                            .is_some_and(|until| until > now)
                        {
                            return None;
                        }

                        if credential.token_expires_at
                            <= now + Duration::seconds(OAUTH_REFRESH_WINDOW_SEC)
                        {
                            return Some(*account_id);
                        }

                        None
                    })
                    .take(batch_size)
                    .collect::<Vec<_>>()
            };

            if account_ids.is_empty() {
                break;
            }

            futures_util::stream::iter(account_ids.clone())
                .for_each_concurrent(Some(concurrency), |account_id| async move {
                    let _ = self.refresh_oauth_account_inner(account_id, false).await;
                })
                .await;

            if account_ids.len() < batch_size {
                break;
            }
        }
    }
}
