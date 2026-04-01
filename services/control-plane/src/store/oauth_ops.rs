use base64::Engine;

#[derive(Debug, Clone)]
struct InMemoryRateLimitRefreshTarget {
    account_id: Uuid,
    base_url: String,
    chatgpt_account_id: Option<String>,
    token_source: InMemoryRateLimitRefreshTokenSource,
}

#[derive(Debug, Clone)]
enum InMemoryRateLimitRefreshTokenSource {
    EncryptedAccessToken(String),
    BearerToken(String),
}

#[derive(Debug, Default)]
pub(crate) struct InMemoryRateLimitRefreshBatchStats {
    processed: u64,
    success: u64,
    failed: u64,
    error_counts: HashMap<String, u64>,
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

fn vault_activation_backoff(failure_count: u32) -> Duration {
    match failure_count {
        0 => Duration::seconds(30),
        1 => Duration::seconds(60),
        2 => Duration::seconds(120),
        _ => Duration::seconds(300),
    }
}

fn derive_admission_rate_limits_expires_at(
    snapshots: &[OAuthRateLimitSnapshot],
    checked_at: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    if snapshots.is_empty() {
        return None;
    }
    let (blocked_until, _) = derive_rate_limit_block(snapshots, checked_at);
    Some(
        blocked_until.unwrap_or_else(|| {
            checked_at + Duration::seconds(rate_limit_cache_ttl_sec_from_env())
        }),
    )
}

fn parse_jwt_exp_from_access_token(access_token: &str) -> Option<DateTime<Utc>> {
    let mut parts = access_token.split('.');
    let _header = parts.next()?;
    let payload = parts.next()?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(payload))
        .ok()?;
    let payload_json = serde_json::from_slice::<Value>(&decoded).ok()?;
    let exp = payload_json.get("exp").and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
    })?;
    (exp > 0)
        .then_some(exp)
        .and_then(|exp| DateTime::<Utc>::from_timestamp(exp, 0))
}

impl InMemoryStore {
    fn record_account_probe_result_inner(
        &self,
        account_id: Uuid,
        checked_at: DateTime<Utc>,
        outcome: AccountProbeOutcome,
    ) {
        let mut states = self.account_health_states.write().unwrap();
        let state = states.entry(account_id).or_default();
        state.last_probe_at = Some(checked_at);
        state.last_probe_outcome = Some(outcome);
    }

    fn emit_probe_result_event_inner(
        &self,
        source: &str,
        account_id: Uuid,
        checked_at: DateTime<Utc>,
        outcome: AccountProbeOutcome,
        reason_code: Option<String>,
        next_action_at: Option<DateTime<Utc>>,
        message: Option<String>,
    ) {
        let account = self.accounts.read().unwrap().get(&account_id).cloned();
        let auth_provider = account
            .as_ref()
            .map(|_| self.account_auth_provider(account_id))
            .map(auth_provider_name)
            .map(str::to_string);
        let (event_type, severity, reason_class) = match outcome {
            AccountProbeOutcome::Ok => (
                "probe_succeeded".to_string(),
                codex_pool_core::events::SystemEventSeverity::Info,
                AccountPoolReasonClass::Healthy,
            ),
            AccountProbeOutcome::Quota => (
                "probe_failed".to_string(),
                codex_pool_core::events::SystemEventSeverity::Warn,
                AccountPoolReasonClass::Quota,
            ),
            AccountProbeOutcome::Fatal => (
                "probe_failed".to_string(),
                codex_pool_core::events::SystemEventSeverity::Error,
                AccountPoolReasonClass::Fatal,
            ),
            AccountProbeOutcome::Transient => (
                "probe_failed".to_string(),
                codex_pool_core::events::SystemEventSeverity::Warn,
                AccountPoolReasonClass::Transient,
            ),
        };
        self.emit_system_event_inner(codex_pool_core::events::SystemEventWrite {
            event_id: None,
            ts: Some(checked_at),
            category: codex_pool_core::events::SystemEventCategory::Patrol,
            event_type,
            severity,
            source: source.to_string(),
            tenant_id: None,
            account_id: Some(account_id),
            request_id: None,
            trace_request_id: None,
            job_id: None,
            account_label: account.as_ref().map(|item| item.label.clone()),
            auth_provider,
            operator_state_from: None,
            operator_state_to: None,
            reason_class: Some(account_pool_reason_class_name(reason_class).to_string()),
            reason_code,
            next_action_at,
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
            message,
            preview_text: None,
            payload_json: None,
            secret_preview: None,
        });
    }

    fn emit_active_patrol_batch_summary_inner(
        &self,
        started_at: DateTime<Utc>,
        patrolled: u64,
        ok_count: u64,
        quota_count: u64,
        fatal_count: u64,
        transient_count: u64,
    ) {
        self.emit_system_event_inner(codex_pool_core::events::SystemEventWrite {
            event_id: None,
            ts: Some(started_at),
            category: codex_pool_core::events::SystemEventCategory::Patrol,
            event_type: "active_patrol_batch_completed".to_string(),
            severity: if fatal_count > 0 {
                codex_pool_core::events::SystemEventSeverity::Warn
            } else {
                codex_pool_core::events::SystemEventSeverity::Info
            },
            source: "control_plane.active_patrol".to_string(),
            tenant_id: None,
            account_id: None,
            request_id: None,
            trace_request_id: None,
            job_id: None,
            account_label: None,
            auth_provider: None,
            operator_state_from: None,
            operator_state_to: None,
            reason_class: None,
            reason_code: None,
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
                "active patrol scanned {patrolled} accounts ({ok_count} ok / {quota_count} quota / {fatal_count} fatal / {transient_count} transient)"
            )),
            preview_text: None,
            payload_json: Some(serde_json::json!({
                "patrolled": patrolled,
                "ok": ok_count,
                "quota": quota_count,
                "fatal": fatal_count,
                "transient": transient_count,
            })),
            secret_preview: None,
        });
    }

    async fn probe_runtime_oauth_account_inner(
        &self,
        account_id: Uuid,
        source: &str,
        success_message: &str,
        quota_message: &str,
        decrypt_failure_message: &str,
    ) -> Result<AccountProbeOutcome> {
        let account = self
            .accounts
            .read()
            .unwrap()
            .get(&account_id)
            .cloned()
            .ok_or_else(|| anyhow!("account not found"))?;
        let provider = self.account_auth_provider(account_id);
        let credential = self.oauth_credentials.read().unwrap().get(&account_id).cloned();
        let session_profile = self
            .session_profiles
            .read()
            .unwrap()
            .get(&account_id)
            .cloned();
        let Some(token_source) = self.build_live_rate_limit_token_source_inner(
            &account,
            &provider,
            credential.as_ref(),
            session_profile.as_ref(),
            Utc::now(),
        ) else {
            return Err(anyhow!("unsupported account pool action"));
        };

        let checked_at = Utc::now();
        let access_token = match token_source {
            InMemoryRateLimitRefreshTokenSource::EncryptedAccessToken(access_token_enc) => {
                let cipher = self.require_credential_cipher()?;
                match cipher.decrypt(&access_token_enc) {
                    Ok(access_token) => access_token,
                    Err(err) => {
                        self.record_account_probe_result_inner(
                            account_id,
                            checked_at,
                            AccountProbeOutcome::Fatal,
                        );
                        self.emit_probe_result_event_inner(
                            source,
                            account_id,
                            checked_at,
                            AccountProbeOutcome::Fatal,
                            Some("credential_decrypt_failed".to_string()),
                            None,
                            Some(decrypt_failure_message.to_string()),
                        );
                        self.mark_upstream_account_pending_purge_inner(
                            account_id,
                            Some("credential_decrypt_failed".to_string()),
                        )?;
                        let _ = self.persist_rate_limit_cache_failure_inner(
                            account_id,
                            "credential_decrypt_failed",
                            &truncate_error_message(err.to_string()),
                        );
                        return Ok(AccountProbeOutcome::Fatal);
                    }
                }
            }
            InMemoryRateLimitRefreshTokenSource::BearerToken(access_token) => access_token,
        };

        match self
            .oauth_client
            .fetch_usage(
                &access_token,
                Some(&account.base_url),
                account.chatgpt_account_id.as_deref(),
            )
            .await
        {
            Ok(usage) => {
                let (blocked_until, block_reason) =
                    derive_rate_limit_block(&usage.rate_limits, checked_at);
                self.persist_rate_limit_cache_success_inner(
                    account_id,
                    usage.rate_limits,
                    checked_at,
                    usage.chatgpt_plan_type,
                );

                if let Some(reason_code) = block_reason {
                    self.record_account_probe_result_inner(
                        account_id,
                        checked_at,
                        AccountProbeOutcome::Quota,
                    );
                    self.emit_probe_result_event_inner(
                        source,
                        account_id,
                        checked_at,
                        AccountProbeOutcome::Quota,
                        Some(reason_code.clone()),
                        blocked_until,
                        Some(quota_message.to_string()),
                    );
                    self.set_account_pool_state_quarantine_inner(
                        account_id,
                        checked_at,
                        blocked_until,
                        Some(reason_code),
                    )?;
                    Ok(AccountProbeOutcome::Quota)
                } else {
                    self.record_account_probe_result_inner(
                        account_id,
                        checked_at,
                        AccountProbeOutcome::Ok,
                    );
                    self.emit_probe_result_event_inner(
                        source,
                        account_id,
                        checked_at,
                        AccountProbeOutcome::Ok,
                        None,
                        None,
                        Some(success_message.to_string()),
                    );
                    self.set_account_pool_state_active_inner(account_id, checked_at);
                    Ok(AccountProbeOutcome::Ok)
                }
            }
            Err(err) => {
                let error_code = err.code().as_str().to_string();
                let error_message = truncate_error_message(err.to_string());
                let _ = self.persist_rate_limit_cache_failure_inner(
                    account_id,
                    &error_code,
                    &error_message,
                );
                let (outcome, reason_code, retry_after) = self.classify_runtime_probe_failure(
                    &provider,
                    checked_at,
                    &error_code,
                    &error_message,
                );
                self.record_account_probe_result_inner(account_id, checked_at, outcome);
                self.emit_probe_result_event_inner(
                    source,
                    account_id,
                    checked_at,
                    outcome,
                    Some(reason_code.clone()),
                    retry_after,
                    Some(error_message.clone()),
                );
                match outcome {
                    AccountProbeOutcome::Ok => {
                        self.set_account_pool_state_active_inner(account_id, checked_at);
                    }
                    AccountProbeOutcome::Fatal => {
                        self.mark_upstream_account_pending_purge_inner(
                            account_id,
                            Some(reason_code),
                        )?;
                    }
                    AccountProbeOutcome::Quota | AccountProbeOutcome::Transient => {
                        self.set_account_pool_state_quarantine_inner(
                            account_id,
                            checked_at,
                            retry_after,
                            Some(reason_code),
                        )?;
                    }
                }
                Ok(outcome)
            }
        }
    }

    pub(super) async fn reprobe_runtime_oauth_account_inner(&self, account_id: Uuid) -> Result<()> {
        let _ = self
            .probe_runtime_oauth_account_inner(
                account_id,
                "control_plane.account_pool_reprobe",
                "manual reprobe succeeded",
                "manual reprobe observed a quota block",
                "failed to decrypt oauth access token for reprobe",
            )
            .await?;
        Ok(())
    }

    pub(crate) fn emit_rate_limit_refresh_batch_summary_inner(
        &self,
        started_at: DateTime<Utc>,
        source: &str,
        job_id: Option<Uuid>,
        fetched: usize,
        stats: &InMemoryRateLimitRefreshBatchStats,
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

    fn classify_runtime_probe_failure(
        &self,
        auth_provider: &UpstreamAuthProvider,
        checked_at: DateTime<Utc>,
        error_code: &str,
        error_message: &str,
    ) -> (AccountProbeOutcome, String, Option<DateTime<Utc>>) {
        let normalized_code = normalize_health_error_code(error_code);
        let normalized_message = error_message.to_ascii_lowercase();
        if normalized_code == "account_deactivated" || normalized_message.contains("account_deactivated")
        {
            return (
                AccountProbeOutcome::Fatal,
                "account_deactivated".to_string(),
                None,
            );
        }
        if normalized_code == "token_invalidated" || normalized_message.contains("token_invalidated")
        {
            return (
                AccountProbeOutcome::Fatal,
                "token_invalidated".to_string(),
                None,
            );
        }
        if is_quota_error_signal(error_code, error_message) {
            let reason = if normalized_code.is_empty() {
                "quota_exhausted".to_string()
            } else {
                normalized_code
            };
            let retry_after = Some(
                checked_at + Duration::seconds(rate_limit_failure_backoff_seconds(&reason, error_message)),
            );
            return (AccountProbeOutcome::Quota, reason, retry_after);
        }
        if is_rate_limited_signal(error_code, error_message) {
            let reason = if normalized_code.is_empty() {
                "rate_limited".to_string()
            } else {
                normalized_code
            };
            let retry_after = Some(
                checked_at + Duration::seconds(rate_limit_failure_backoff_seconds(&reason, error_message)),
            );
            return (AccountProbeOutcome::Quota, reason, retry_after);
        }
        if is_fatal_refresh_error_code(Some(error_code)) || is_auth_error_signal(error_code, error_message) {
            return (
                AccountProbeOutcome::Fatal,
                normalized_auth_failure_reason_code(auth_provider, error_code, error_message),
                None,
            );
        }
        if is_transient_upstream_error_signal(error_code, error_message) {
            return (
                AccountProbeOutcome::Transient,
                if normalized_code.is_empty() {
                    "upstream_unavailable".to_string()
                } else {
                    normalized_code
                },
                Some(checked_at + Duration::minutes(5)),
            );
        }

        (
            AccountProbeOutcome::Transient,
            if normalized_code.is_empty() {
                "upstream_unavailable".to_string()
            } else {
                normalized_code
            },
            Some(checked_at + Duration::minutes(5)),
        )
    }

    fn oauth_active_patrol_candidates_inner(&self, now: DateTime<Utc>, limit: usize) -> Vec<Uuid> {
        let accounts = self.accounts.read().unwrap().clone();
        let states = self.account_health_states.read().unwrap().clone();
        let providers = self.account_auth_providers.read().unwrap().clone();
        let credentials = self.oauth_credentials.read().unwrap().clone();
        let session_profiles = self.session_profiles.read().unwrap().clone();
        let mut items = accounts
            .into_values()
            .filter(|account| account.enabled)
            .filter_map(|account| {
                let provider = providers.get(&account.id)?;
                self.build_live_rate_limit_token_source_inner(
                    &account,
                    provider,
                    credentials.get(&account.id),
                    session_profiles.get(&account.id),
                    now,
                )?;
                let state = states.get(&account.id).cloned().unwrap_or_default();
                if state.pool_state != AccountPoolState::Active {
                    return None;
                }
                let freshness = account_health_freshness_from_signals(
                    now,
                    state.seen_ok_at,
                    state.last_probe_at,
                    state.last_probe_outcome,
                    state.last_live_result_at,
                    state.last_live_result_status.as_ref(),
                );
                if matches!(freshness, AccountHealthFreshness::Fresh) {
                    return None;
                }
                Some((
                    account.id,
                    matches!(freshness, AccountHealthFreshness::Unknown),
                    state.last_probe_at,
                    state.seen_ok_at,
                    account.created_at,
                ))
            })
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .1
                .cmp(&left.1)
                .then_with(|| left.2.cmp(&right.2))
                .then_with(|| left.3.cmp(&right.3))
                .then_with(|| left.4.cmp(&right.4))
                .then_with(|| left.0.cmp(&right.0))
        });
        items.truncate(limit);
        items.into_iter().map(|item| item.0).collect()
    }

    fn build_live_rate_limit_token_source_inner(
        &self,
        account: &UpstreamAccount,
        provider: &UpstreamAuthProvider,
        credential: Option<&OAuthCredentialRecord>,
        session_profile: Option<&SessionProfileRecord>,
        now: DateTime<Utc>,
    ) -> Option<InMemoryRateLimitRefreshTokenSource> {
        if !account.enabled {
            return None;
        }

        match provider {
            UpstreamAuthProvider::OAuthRefreshToken => {
                let credential = credential?;
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

                Some(InMemoryRateLimitRefreshTokenSource::EncryptedAccessToken(
                    credential.access_token_enc.clone(),
                ))
            }
            UpstreamAuthProvider::LegacyBearer => {
                let session_profile = session_profile?;
                if account.mode != UpstreamMode::CodexOauth {
                    return None;
                }
                if session_profile.credential_kind != SessionCredentialKind::OneTimeAccessToken {
                    return None;
                }
                let token_expires_at = session_profile.token_expires_at?;
                if token_expires_at <= now + Duration::seconds(OAUTH_MIN_VALID_SEC) {
                    return None;
                }
                let bearer_token = account.bearer_token.trim();
                if bearer_token.is_empty() {
                    return None;
                }

                Some(InMemoryRateLimitRefreshTokenSource::BearerToken(
                    bearer_token.to_string(),
                ))
            }
        }
    }

    fn build_rate_limit_refresh_target_inner(
        &self,
        account: &UpstreamAccount,
        provider: &UpstreamAuthProvider,
        credential: Option<&OAuthCredentialRecord>,
        session_profile: Option<&SessionProfileRecord>,
        cache: Option<&OAuthRateLimitCacheRecord>,
        now: DateTime<Utc>,
        due_only: bool,
    ) -> Option<InMemoryRateLimitRefreshTarget> {
        let token_source = self.build_live_rate_limit_token_source_inner(
            account,
            provider,
            credential,
            session_profile,
            now,
        )?;

        if due_only && cache.is_some_and(|entry| entry.expires_at.is_some_and(|expires_at| expires_at > now)) {
            return None;
        }

        Some(InMemoryRateLimitRefreshTarget {
            account_id: account.id,
            base_url: account.base_url.clone(),
            chatgpt_account_id: account.chatgpt_account_id.clone(),
            token_source,
        })
    }

    fn oauth_inventory_summary_inner(&self) -> OAuthInventorySummaryResponse {
        let mut summary = OAuthInventorySummaryResponse::default();
        let vault = self.oauth_refresh_token_vault.read().unwrap();
        summary.total = vault.len() as u64;
        for item in vault.values() {
            match item.status {
                OAuthVaultRecordStatus::Queued => summary.queued = summary.queued.saturating_add(1),
                OAuthVaultRecordStatus::Ready => summary.ready = summary.ready.saturating_add(1),
                OAuthVaultRecordStatus::NeedsRefresh => {
                    summary.needs_refresh = summary.needs_refresh.saturating_add(1)
                }
                OAuthVaultRecordStatus::NoQuota => {
                    summary.no_quota = summary.no_quota.saturating_add(1)
                }
                OAuthVaultRecordStatus::Failed => summary.failed = summary.failed.saturating_add(1),
            }
        }
        summary
    }

    fn oauth_inventory_records_inner(&self) -> Vec<OAuthInventoryRecord> {
        let mut items = self
            .oauth_refresh_token_vault
            .read()
            .unwrap()
            .values()
            .cloned()
            .map(|item| OAuthInventoryRecord {
                id: item.id,
                label: item.label,
                email: item.email,
                chatgpt_account_id: item.chatgpt_account_id,
                chatgpt_plan_type: item.chatgpt_plan_type,
                source_type: item.source_type,
                vault_status: item.status,
                has_refresh_token: true,
                has_access_token_fallback: item.fallback_access_token_enc.is_some(),
                admission_source: item.admission_source,
                admission_checked_at: item.admission_checked_at,
                admission_retry_after: item.admission_retry_after,
                admission_error_code: item.admission_error_code,
                admission_error_message: item.admission_error_message,
                admission_rate_limits: item.admission_rate_limits,
                admission_rate_limits_expires_at: item.admission_rate_limits_expires_at,
                failure_stage: item.failure_stage,
                attempt_count: item.attempt_count,
                transient_retry_count: item.transient_retry_count,
                next_retry_at: item.next_retry_at,
                retryable: item.retryable,
                terminal_reason: item.terminal_reason,
                created_at: item.created_at,
                updated_at: item.updated_at,
            })
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.cmp(&right.id))
        });
        items
    }

    fn mark_oauth_inventory_record_failed_inner(
        &self,
        record_id: Uuid,
        reason: Option<String>,
    ) -> Result<()> {
        let now = Utc::now();
        let delete_due_at = now + Duration::seconds(pending_purge_delay_sec_from_env());
        let reason = reason
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let mut vault = self.oauth_refresh_token_vault.write().unwrap();
        let Some(item) = vault.get_mut(&record_id) else {
            return Err(anyhow!("oauth inventory record not found"));
        };
        item.status = OAuthVaultRecordStatus::Failed;
        item.admission_checked_at = Some(now);
        item.admission_retry_after = None;
        item.next_retry_at = Some(delete_due_at);
        item.retryable = false;
        if item.terminal_reason.is_none() || reason.is_some() {
            item.terminal_reason = reason
                .or_else(|| item.terminal_reason.clone())
                .or_else(|| item.admission_error_code.clone());
        }
        item.updated_at = now;
        drop(vault);
        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn mark_oauth_inventory_records_failed_inner(
        &self,
        record_ids: &[Uuid],
        reason: Option<String>,
    ) {
        let now = Utc::now();
        let delete_due_at = now + Duration::seconds(pending_purge_delay_sec_from_env());
        let reason = reason
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let mut vault = self.oauth_refresh_token_vault.write().unwrap();
        let mut changed = false;
        for record_id in record_ids {
            let Some(item) = vault.get_mut(record_id) else {
                continue;
            };
            item.status = OAuthVaultRecordStatus::Failed;
            item.admission_checked_at = Some(now);
            item.admission_retry_after = None;
            item.next_retry_at = Some(delete_due_at);
            item.retryable = false;
            if item.terminal_reason.is_none() || reason.is_some() {
                item.terminal_reason = reason
                    .clone()
                    .or_else(|| item.terminal_reason.clone())
                    .or_else(|| item.admission_error_code.clone());
            }
            item.updated_at = now;
            changed = true;
        }
        drop(vault);
        if changed {
            self.revision.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn delete_oauth_inventory_record_inner(&self, record_id: Uuid) -> Result<()> {
        let removed = self
            .oauth_refresh_token_vault
            .write()
            .unwrap()
            .remove(&record_id);
        if removed.is_none() {
            return Err(anyhow!("oauth inventory record not found"));
        }
        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn delete_oauth_inventory_records_inner(&self, record_ids: &[Uuid]) {
        let mut vault = self.oauth_refresh_token_vault.write().unwrap();
        let mut changed = false;
        for record_id in record_ids {
            if vault.remove(record_id).is_some() {
                changed = true;
            }
        }
        drop(vault);
        if changed {
            self.revision.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn restore_oauth_inventory_record_inner(&self, record_id: Uuid) -> Result<()> {
        let now = Utc::now();
        let mut vault = self.oauth_refresh_token_vault.write().unwrap();
        let Some(item) = vault.get_mut(&record_id) else {
            return Err(anyhow!("oauth inventory record not found"));
        };
        item.status = OAuthVaultRecordStatus::Queued;
        item.failure_count = 0;
        item.backoff_until = None;
        item.next_attempt_at = Some(now);
        item.last_error_code = None;
        item.last_error_message = None;
        item.admission_source = None;
        item.admission_checked_at = None;
        item.admission_retry_after = None;
        item.admission_error_code = None;
        item.admission_error_message = None;
        item.admission_rate_limits = Vec::new();
        item.admission_rate_limits_expires_at = None;
        item.failure_stage = None;
        item.attempt_count = 0;
        item.transient_retry_count = 0;
        item.next_retry_at = Some(now);
        item.retryable = true;
        item.terminal_reason = None;
        item.updated_at = now;
        drop(vault);
        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn restore_oauth_inventory_records_inner(&self, record_ids: &[Uuid]) {
        let now = Utc::now();
        let mut vault = self.oauth_refresh_token_vault.write().unwrap();
        let mut changed = false;
        for record_id in record_ids {
            let Some(item) = vault.get_mut(record_id) else {
                continue;
            };
            item.status = OAuthVaultRecordStatus::Queued;
            item.failure_count = 0;
            item.backoff_until = None;
            item.next_attempt_at = Some(now);
            item.last_error_code = None;
            item.last_error_message = None;
            item.admission_source = None;
            item.admission_checked_at = None;
            item.admission_retry_after = None;
            item.admission_error_code = None;
            item.admission_error_message = None;
            item.admission_rate_limits = Vec::new();
            item.admission_rate_limits_expires_at = None;
            item.failure_stage = None;
            item.attempt_count = 0;
            item.transient_retry_count = 0;
            item.next_retry_at = Some(now);
            item.retryable = true;
            item.terminal_reason = None;
            item.updated_at = now;
            changed = true;
        }
        drop(vault);
        if changed {
            self.revision.fetch_add(1, Ordering::Relaxed);
        }
    }

    async fn reprobe_oauth_inventory_record_inner(&self, record_id: Uuid) -> Result<()> {
        self.restore_oauth_inventory_record_inner(record_id)?;
        self.probe_oauth_vault_admission_inner(record_id).await
    }

    fn purge_due_oauth_inventory_records_inner(&self) -> u64 {
        let now = Utc::now();
        let delete_delay = Duration::seconds(pending_purge_delay_sec_from_env());
        let due_record_ids = self
            .oauth_refresh_token_vault
            .read()
            .unwrap()
            .values()
            .filter(|item| {
                matches!(item.status, OAuthVaultRecordStatus::Failed)
                    && !item.retryable
                    && item
                        .next_retry_at
                        .unwrap_or(item.updated_at + delete_delay)
                        <= now
            })
            .map(|item| item.id)
            .collect::<Vec<_>>();
        let deleted = due_record_ids.len() as u64;
        self.delete_oauth_inventory_records_inner(&due_record_ids);
        deleted
    }

    fn oauth_vault_record_id_for_request(
        &self,
        req: &ImportOAuthRefreshTokenRequest,
    ) -> Option<Uuid> {
        let refresh_hash = refresh_token_sha256(&req.refresh_token);
        let vault = self.oauth_refresh_token_vault.read().unwrap();
        vault.iter().find_map(|(id, item)| {
            if item.refresh_token_sha256 == refresh_hash {
                Some(*id)
            } else {
                None
            }
        })
    }

    fn update_oauth_vault_admission_result(
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
        retryable: bool,
        terminal_reason: Option<String>,
        increment_transient_retry_count: bool,
    ) {
        if let Some(item) = self.oauth_refresh_token_vault.write().unwrap().get_mut(&record_id) {
            item.status = status;
            item.admission_source = admission_source;
            item.admission_checked_at = Some(checked_at);
            item.admission_retry_after = retry_after;
            item.admission_error_code = error_code;
            item.admission_error_message = error_message;
            if fallback_token_expires_at.is_some() {
                item.fallback_token_expires_at = fallback_token_expires_at;
            }
            item.admission_rate_limits = rate_limits;
            item.admission_rate_limits_expires_at = rate_limits_expires_at;
            item.failure_stage = failure_stage;
            item.attempt_count = item.attempt_count.saturating_add(1);
            if increment_transient_retry_count {
                item.transient_retry_count = item.transient_retry_count.saturating_add(1);
            }
            item.next_retry_at = retry_after;
            item.retryable = retryable;
            item.terminal_reason = terminal_reason;
            item.updated_at = checked_at;
        }
        self.revision.fetch_add(1, Ordering::Relaxed);
    }

    async fn probe_oauth_vault_admission_inner(&self, record_id: Uuid) -> Result<()> {
        let Some(record) = self
            .oauth_refresh_token_vault
            .read()
            .unwrap()
            .get(&record_id)
            .cloned()
        else {
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
                false,
                Some("credential_cipher_missing".to_string()),
                false,
            );
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
                true,
                None,
                false,
            );
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
                    false,
                    Some("credential_decrypt_failed".to_string()),
                    false,
                );
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
                    Some(truncate_error_message(err.to_string())),
                    None,
                    Vec::new(),
                    None,
                    Some(OAuthInventoryFailureStage::AdmissionProbe),
                    false,
                    Some("credential_decrypt_failed".to_string()),
                    false,
                );
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
                let (blocked_until, block_reason) = derive_rate_limit_block(&rate_limits, checked_at);
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
                        true,
                        None,
                        false,
                    );
                    return Ok(());
                }
                if let Some(block_reason) = block_reason {
                    let block_message = rate_limit_block_message(&block_reason);
                    self.update_oauth_vault_admission_result(
                        record_id,
                        OAuthVaultRecordStatus::NoQuota,
                        checked_at,
                        Some("fallback_access_token".to_string()),
                        blocked_until,
                        Some(block_reason),
                        Some(block_message),
                        fallback_expires_at,
                        rate_limits,
                        rate_limits_expires_at,
                        Some(OAuthInventoryFailureStage::AdmissionProbe),
                        true,
                        None,
                        false,
                    );
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
                    false,
                    None,
                    false,
                );
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
                    retryable,
                    terminal_reason,
                    transient_signal && retry_after.is_some(),
                );
                Ok(())
            }
        }
    }

    fn stable_vault_token_family_id(refresh_token_sha256: &str) -> String {
        format!("vault:{refresh_token_sha256}")
    }

    fn oauth_vault_activation_priority(status: OAuthVaultRecordStatus) -> u8 {
        match status {
            OAuthVaultRecordStatus::Ready => 0,
            OAuthVaultRecordStatus::NeedsRefresh => 1,
            OAuthVaultRecordStatus::Queued => 2,
            OAuthVaultRecordStatus::NoQuota => 3,
            OAuthVaultRecordStatus::Failed => 4,
        }
    }

    fn oauth_vault_activation_fallback_status(status: OAuthVaultRecordStatus) -> OAuthVaultRecordStatus {
        match status {
            OAuthVaultRecordStatus::Ready => OAuthVaultRecordStatus::NeedsRefresh,
            other => other,
        }
    }

    fn mark_oauth_vault_activation_failed_inner(
        &self,
        record_id: Uuid,
        candidate_status: OAuthVaultRecordStatus,
        error_code: String,
        error_message: String,
    ) {
        let now = Utc::now();
        let fatal_auth = is_fatal_refresh_error_code(Some(error_code.as_str()));
        let terminal_config_error = matches!(
            normalize_health_error_code(&error_code).as_str(),
            "credential_cipher_missing" | "credential_decrypt_failed"
        );

        let mut vault = self.oauth_refresh_token_vault.write().unwrap();
        if let Some(item) = vault.get_mut(&record_id) {
            let next_failure_count = item.failure_count.saturating_add(1);
            let allow_retry = if terminal_config_error {
                false
            } else if fatal_auth {
                can_retry_fatal_activation_failure(next_failure_count)
            } else {
                true
            };

            item.status = if allow_retry {
                Self::oauth_vault_activation_fallback_status(candidate_status)
            } else {
                OAuthVaultRecordStatus::Failed
            };
            item.failure_count = next_failure_count;
            item.last_error_code = Some(error_code.clone());
            item.last_error_message = Some(error_message);
            item.failure_stage = Some(OAuthInventoryFailureStage::ActivationRefresh);
            item.attempt_count = item.attempt_count.saturating_add(1);
            item.updated_at = now;
            item.retryable = allow_retry;
            item.terminal_reason = if allow_retry {
                None
            } else {
                Some(error_code)
            };

            if allow_retry {
                let backoff = vault_activation_backoff(item.failure_count);
                let retry_at = now + backoff;
                item.backoff_until = Some(retry_at);
                item.next_attempt_at = Some(retry_at);
                item.next_retry_at = Some(retry_at);
            } else {
                item.backoff_until = None;
                item.next_attempt_at = None;
                item.next_retry_at = None;
            }
        }
    }

    fn matched_oauth_account_id_for_vault_record(
        &self,
        record: &OAuthRefreshTokenVaultRecord,
    ) -> Option<Uuid> {
        let accounts = self.accounts.read().unwrap();
        let providers = self.account_auth_providers.read().unwrap();
        let credentials = self.oauth_credentials.read().unwrap();
        let normalized_chatgpt_account_id = record
            .chatgpt_account_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());

        accounts
            .values()
            .filter(|account| {
                providers.get(&account.id) == Some(&UpstreamAuthProvider::OAuthRefreshToken)
                    && credentials
                        .get(&account.id)
                        .is_some_and(|credential| {
                            credential.refresh_token_sha256 == record.refresh_token_sha256
                        })
            })
            .max_by(|left, right| {
                left.created_at
                    .cmp(&right.created_at)
                    .then_with(|| left.id.cmp(&right.id))
            })
            .map(|account| account.id)
            .or_else(|| {
                normalized_chatgpt_account_id.and_then(|target_account_id| {
                    accounts
                        .values()
                        .filter(|account| {
                            providers.get(&account.id)
                                == Some(&UpstreamAuthProvider::OAuthRefreshToken)
                                && account.chatgpt_account_id.as_deref().map(str::trim)
                                    == Some(target_account_id)
                        })
                        .max_by(|left, right| {
                            left.created_at
                                .cmp(&right.created_at)
                                .then_with(|| left.id.cmp(&right.id))
                        })
                        .map(|account| account.id)
                })
            })
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

    fn materialize_ready_oauth_vault_record_inner(
        &self,
        record: &OAuthRefreshTokenVaultRecord,
    ) -> Result<UpstreamAccount> {
        let now = Utc::now();
        let Some(access_token_enc) = record.fallback_access_token_enc.clone() else {
            return Err(anyhow!("ready access token is missing"));
        };
        let token_expires_at = match record.fallback_token_expires_at {
            Some(expires_at) => expires_at,
            None => {
                let cipher = self.require_credential_cipher()?;
                let fallback_access_token = cipher
                    .decrypt(&access_token_enc)
                    .map_err(|err| anyhow!(truncate_error_message(err.to_string())))?;
                parse_jwt_exp_from_access_token(&fallback_access_token)
                    .ok_or_else(|| anyhow!("ready access token expiry is unknown"))?
            }
        };
        if token_expires_at <= now + Duration::seconds(OAUTH_MIN_VALID_SEC) {
            return Err(anyhow!("ready access token is already expired"));
        }

        let account_id = self.matched_oauth_account_id_for_vault_record(record);
        let existing_credential = account_id.and_then(|account_id| {
            self.oauth_credentials
                .read()
                .unwrap()
                .get(&account_id)
                .cloned()
        });
        let credential = OAuthCredentialRecord {
            access_token_enc: access_token_enc.clone(),
            refresh_token_enc: record.refresh_token_enc.clone(),
            fallback_access_token_enc: Some(access_token_enc),
            refresh_token_sha256: record.refresh_token_sha256.clone(),
            token_family_id: existing_credential
                .as_ref()
                .map(|item| item.token_family_id.clone())
                .unwrap_or_else(|| Self::stable_vault_token_family_id(&record.refresh_token_sha256)),
            token_version: existing_credential
                .as_ref()
                .map(|item| item.token_version)
                .unwrap_or(0),
            token_expires_at,
            fallback_token_expires_at: Some(token_expires_at),
            last_refresh_at: None,
            last_refresh_status: OAuthRefreshStatus::Never,
            refresh_reused_detected: false,
            last_refresh_error_code: None,
            last_refresh_error: None,
            refresh_failure_count: 0,
            refresh_backoff_until: None,
        };

        let account = if let Some(account_id) = account_id {
            let updated = {
                let mut accounts = self.accounts.write().unwrap();
                let account = accounts
                    .get_mut(&account_id)
                    .ok_or_else(|| anyhow!("matched oauth account is missing"))?;
                account.label = record.label.clone();
                account.mode = record.desired_mode.clone();
                account.base_url = record.base_url.clone();
                account.bearer_token = OAUTH_MANAGED_BEARER_SENTINEL.to_string();
                account.chatgpt_account_id = record.chatgpt_account_id.clone();
                account.enabled = record.desired_enabled;
                account.priority = record.desired_priority;
                account.clone()
            };
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
            let profile = existing_profile
                .map(|mut profile| {
                    profile.credential_kind = SessionCredentialKind::RefreshRotatable;
                    profile.token_expires_at = Some(token_expires_at);
                    profile.email = record.email.clone().or(profile.email);
                    profile.chatgpt_plan_type =
                        record.chatgpt_plan_type.clone().or(profile.chatgpt_plan_type);
                    profile.source_type = record.source_type.clone().or(profile.source_type);
                    profile
                })
                .unwrap_or_else(|| Self::ready_session_profile_from_vault_record(record, token_expires_at));
            self.upsert_session_profile(account_id, profile);
            updated
        } else {
            let account = UpstreamAccount {
                id: Uuid::new_v4(),
                label: record.label.clone(),
                mode: record.desired_mode.clone(),
                base_url: record.base_url.clone(),
                bearer_token: OAUTH_MANAGED_BEARER_SENTINEL.to_string(),
                chatgpt_account_id: record.chatgpt_account_id.clone(),
                enabled: record.desired_enabled,
                priority: record.desired_priority,
                created_at: now,
            };
            self.accounts
                .write()
                .unwrap()
                .insert(account.id, account.clone());
            self.account_auth_providers
                .write()
                .unwrap()
                .insert(account.id, UpstreamAuthProvider::OAuthRefreshToken);
            self.upsert_oauth_credential(account.id, credential);
            self.upsert_session_profile(
                account.id,
                Self::ready_session_profile_from_vault_record(record, token_expires_at),
            );
            account
        };

        if !record.admission_rate_limits.is_empty() {
            self.persist_rate_limit_cache_success_inner(
                account.id,
                record.admission_rate_limits.clone(),
                record.admission_checked_at.unwrap_or(now),
                None,
            );
        }
        self.record_account_probe_result_inner(
            account.id,
            record.admission_checked_at.unwrap_or(now),
            AccountProbeOutcome::Ok,
        );
        self.set_account_pool_state_active_inner(account.id, now);
        self.revision.fetch_add(1, Ordering::Relaxed);

        Ok(account)
    }

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
        let session_profiles = self.session_profiles.read().unwrap().clone();
        let caches = self.oauth_rate_limit_caches.read().unwrap().clone();

        let mut targets = accounts
            .values()
            .filter_map(|account| {
                if after_id.is_some_and(|cursor| account.id <= cursor) {
                    return None;
                }
                let provider = providers.get(&account.id)?;
                self.build_rate_limit_refresh_target_inner(
                    account,
                    provider,
                    credentials.get(&account.id),
                    session_profiles.get(&account.id),
                    caches.get(&account.id),
                    now,
                    due_only,
                )
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
        let session_profiles = self.session_profiles.read().unwrap().clone();

        accounts
            .values()
            .filter_map(|account| {
                let provider = providers.get(&account.id)?;
                self.build_rate_limit_refresh_target_inner(
                    account,
                    provider,
                    credentials.get(&account.id),
                    session_profiles.get(&account.id),
                    None,
                    now,
                    false,
                )
            })
            .count() as u64
    }

    async fn fetch_live_rate_limits_result(
        &self,
        target: &InMemoryRateLimitRefreshTarget,
    ) -> Result<crate::oauth::OAuthUsageSnapshot, (String, String)> {
        let access_token = match &target.token_source {
            InMemoryRateLimitRefreshTokenSource::EncryptedAccessToken(access_token_enc) => {
                let Some(cipher) = self.credential_cipher.as_ref() else {
                    return Err((
                        "credential_cipher_missing".to_string(),
                        "oauth credential cipher is not configured".to_string(),
                    ));
                };
                cipher
                    .decrypt(access_token_enc)
                    .map_err(|err| ("credential_decrypt_failed".to_string(), err.to_string()))?
            }
            InMemoryRateLimitRefreshTokenSource::BearerToken(access_token) => {
                access_token.clone()
            }
        };
        if access_token.trim().is_empty() {
            return Ok(crate::oauth::OAuthUsageSnapshot::default());
        }

        self.oauth_client
            .fetch_usage(
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
        chatgpt_plan_type: Option<String>,
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
        if let Some(plan_type) = chatgpt_plan_type
            .map(|raw| raw.trim().to_string())
            .filter(|raw| !raw.is_empty())
        {
            if let Some(profile) = self.session_profiles.write().unwrap().get_mut(&account_id) {
                profile.chatgpt_plan_type = Some(plan_type);
            }
        }
        self.revision.fetch_add(1, Ordering::Relaxed);
    }

    fn persist_rate_limit_cache_failure_inner(
        &self,
        account_id: Uuid,
        error_code: &str,
        error_message: &str,
    ) -> String {
        let now = Utc::now();
        let backoff_sec = rate_limit_failure_backoff_seconds(error_code, error_message);
        let truncated_message = truncate_error_message(error_message.to_string());
        let normalized_error_code = if is_auth_error_signal(error_code, error_message) {
            normalized_auth_failure_reason_code(
                &self.account_auth_provider(account_id),
                error_code,
                error_message,
            )
        } else {
            error_code.to_string()
        };
        let mut caches = self.oauth_rate_limit_caches.write().unwrap();
        let cache = caches.entry(account_id).or_default();
        if cache.rate_limits.is_empty() {
            cache.fetched_at = None;
        }
        cache.expires_at = Some(now + Duration::seconds(backoff_sec));
        cache.last_error_code = Some(normalized_error_code.clone());
        cache.last_error = Some(truncated_message);
        drop(caches);

        self.revision.fetch_add(1, Ordering::Relaxed);
        normalized_error_code
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
                    Ok(usage) => {
                        self.persist_rate_limit_cache_success_inner(
                            target.account_id,
                            usage.rate_limits,
                            fetched_at,
                            usage.chatgpt_plan_type,
                        );
                        (true, None)
                    }
                    Err((error_code, error_message)) => {
                        let stored_error_code = self.persist_rate_limit_cache_failure_inner(
                            target.account_id,
                            &error_code,
                            &error_message,
                        );
                        (false, Some(stored_error_code))
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

    pub(super) fn queue_oauth_refresh_token_inner(
        &self,
        req: ImportOAuthRefreshTokenRequest,
    ) -> Result<bool> {
        let cipher = self.require_credential_cipher()?;
        let desired_mode = resolve_oauth_import_mode(req.mode.clone(), req.source_type.as_deref());
        let normalized_base_url = normalize_upstream_account_base_url(&desired_mode, &req.base_url);
        let normalized_refresh_hash = refresh_token_sha256(&req.refresh_token);
        let now = Utc::now();

        let existing_id = {
            let vault = self.oauth_refresh_token_vault.read().unwrap();
            vault.iter().find_map(|(id, item)| {
                if item.refresh_token_sha256 == normalized_refresh_hash {
                    Some(*id)
                } else {
                    None
                }
            })
        };

        let fallback_access_token_enc = req
            .fallback_access_token
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|token| cipher.encrypt(token))
            .transpose()?;

        let record_id = existing_id.unwrap_or_else(Uuid::new_v4);
        let created_at = if let Some(existing_id) = existing_id {
            self.oauth_refresh_token_vault
                .read()
                .unwrap()
                .get(&existing_id)
                .map(|item| item.created_at)
                .unwrap_or(now)
        } else {
            now
        };

        let record = OAuthRefreshTokenVaultRecord {
            id: record_id,
            label: req.label,
            email: None,
            base_url: normalized_base_url,
            refresh_token_enc: cipher.encrypt(&req.refresh_token)?,
            fallback_access_token_enc,
            fallback_token_expires_at: req.fallback_token_expires_at,
            refresh_token_sha256: normalized_refresh_hash,
            chatgpt_account_id: req.chatgpt_account_id,
            chatgpt_plan_type: req.chatgpt_plan_type,
            source_type: req.source_type,
            desired_mode,
            desired_enabled: req.enabled.unwrap_or(true),
            desired_priority: req.priority.unwrap_or(100),
            status: OAuthVaultRecordStatus::Queued,
            failure_count: 0,
            backoff_until: None,
            next_attempt_at: Some(now),
            last_error_code: None,
            last_error_message: None,
            admission_source: None,
            admission_checked_at: None,
            admission_retry_after: None,
            admission_error_code: None,
            admission_error_message: None,
            admission_rate_limits: Vec::new(),
            admission_rate_limits_expires_at: None,
            failure_stage: None,
            attempt_count: 0,
            transient_retry_count: 0,
            next_retry_at: Some(now),
            retryable: true,
            terminal_reason: None,
            created_at,
            updated_at: now,
        };

        self.oauth_refresh_token_vault
            .write()
            .unwrap()
            .insert(record.id, record);

        Ok(existing_id.is_none())
    }

    fn oauth_active_account_count_inner(&self) -> usize {
        let accounts = self.accounts.read().unwrap();
        let providers = self.account_auth_providers.read().unwrap().clone();
        let states = self.account_health_states.read().unwrap().clone();
        let now = Utc::now();

        accounts
            .iter()
            .filter(|(account_id, account)| {
                if !account.enabled {
                    return false;
                }
                if providers.get(account_id) != Some(&UpstreamAuthProvider::OAuthRefreshToken) {
                    return false;
                }
                let state = states.get(account_id).cloned().unwrap_or_default();
                state.pool_state == AccountPoolState::Active
                    || (state.pool_state == AccountPoolState::Quarantine
                        && state
                            .quarantine_until
                            .is_some_and(|quarantine_until| quarantine_until <= now))
            })
            .count()
    }

    fn oauth_vault_activation_candidates_inner(
        &self,
        now: DateTime<Utc>,
        limit: usize,
    ) -> Vec<OAuthRefreshTokenVaultRecord> {
        let mut items = self
            .oauth_refresh_token_vault
            .read()
            .unwrap()
            .values()
            .filter(|item| {
                matches!(
                    item.status,
                    OAuthVaultRecordStatus::Ready
                        | OAuthVaultRecordStatus::NeedsRefresh
                        | OAuthVaultRecordStatus::Queued
                )
                    && item.backoff_until.is_none_or(|backoff_until| backoff_until <= now)
                    && item.next_attempt_at.is_none_or(|next_attempt_at| next_attempt_at <= now)
            })
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            Self::oauth_vault_activation_priority(left.status)
                .cmp(&Self::oauth_vault_activation_priority(right.status))
                .then_with(|| {
                    left.created_at
                .cmp(&right.created_at)
                })
                .then_with(|| left.id.cmp(&right.id))
        });
        items.truncate(limit);
        items
    }

    fn oauth_vault_admission_reprobe_candidates_inner(
        &self,
        now: DateTime<Utc>,
        limit: usize,
    ) -> Vec<Uuid> {
        let mut items = self
            .oauth_refresh_token_vault
            .read()
            .unwrap()
            .values()
            .filter(|item| {
                matches!(
                    item.status,
                    OAuthVaultRecordStatus::NoQuota | OAuthVaultRecordStatus::Failed
                ) && item
                    .admission_retry_after
                    .is_some_and(|retry_after| retry_after <= now)
            })
            .map(|item| (item.admission_retry_after, item.created_at, item.id))
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
                .then_with(|| left.2.cmp(&right.2))
        });
        items.truncate(limit);
        items.into_iter().map(|item| item.2).collect()
    }

    pub(super) async fn patrol_active_oauth_accounts_inner(&self) -> Result<u64> {
        self.purge_expired_one_time_accounts_inner();

        let now = Utc::now();
        let batch_started_at = now;
        let candidate_ids =
            self.oauth_active_patrol_candidates_inner(now, active_patrol_batch_size_from_env());
        if candidate_ids.is_empty() {
            return Ok(0);
        }

        let mut patrolled = 0_u64;
        let mut ok_count = 0_u64;
        let mut quota_count = 0_u64;
        let mut fatal_count = 0_u64;
        let mut transient_count = 0_u64;

        for account_id in candidate_ids {
            let outcome = match self
                .probe_runtime_oauth_account_inner(
                    account_id,
                    "control_plane.active_patrol",
                    "active patrol probe succeeded",
                    "active patrol observed a quota block",
                    "failed to decrypt oauth access token for patrol",
                )
                .await
            {
                Ok(outcome) => outcome,
                Err(_) => continue,
            };
            match outcome {
                AccountProbeOutcome::Ok => ok_count = ok_count.saturating_add(1),
                AccountProbeOutcome::Quota => quota_count = quota_count.saturating_add(1),
                AccountProbeOutcome::Fatal => fatal_count = fatal_count.saturating_add(1),
                AccountProbeOutcome::Transient => {
                    transient_count = transient_count.saturating_add(1)
                }
            }
            patrolled = patrolled.saturating_add(1);
        }

        self.emit_active_patrol_batch_summary_inner(
            batch_started_at,
            patrolled,
            ok_count,
            quota_count,
            fatal_count,
            transient_count,
        );

        Ok(patrolled)
    }

    pub(super) async fn activate_oauth_refresh_token_vault_inner(&self) -> Result<u64> {
        self.purge_expired_one_time_accounts_inner();
        let reprobe_limit = oauth_vault_activate_batch_size_from_env();
        for record_id in self.oauth_vault_admission_reprobe_candidates_inner(Utc::now(), reprobe_limit) {
            self.probe_oauth_vault_admission_inner(record_id).await?;
        }
        let active_count = self.oauth_active_account_count_inner();
        let target = active_pool_target_from_env();
        let active_min = active_pool_min_from_env().min(target);
        let runtime_cap = runtime_pool_cap_from_env();
        let runtime_count = self.runtime_pool_account_count_inner();
        if runtime_count >= runtime_cap {
            tracing::warn!(
                runtime_count,
                runtime_cap,
                active_count,
                target,
                "sqlite runtime pool reached configured cap; skipping oauth vault activation"
            );
            return Ok(0);
        }
        if active_count >= target {
            return Ok(0);
        }
        if active_count < active_min {
            tracing::warn!(
                active_count,
                active_min,
                target,
                "sqlite active oauth pool dropped below configured minimum"
            );
        }

        let needed = target.saturating_sub(active_count);
        let batch_size = oauth_vault_activate_batch_size_from_env();
        let headroom = runtime_cap.saturating_sub(runtime_count);
        let limit = needed.min(batch_size).min(headroom);
        if limit == 0 {
            return Ok(0);
        }

        let candidates = self.oauth_vault_activation_candidates_inner(Utc::now(), limit);
        if candidates.is_empty() {
            return Ok(0);
        }

        let _concurrency = oauth_vault_activate_concurrency_from_env();
        let max_rps = oauth_vault_activate_max_rps_from_env();
        let launch_interval = std::time::Duration::from_secs_f64(1.0 / f64::from(max_rps));

        let mut activated = 0_u64;
        for (index, mut candidate) in candidates.into_iter().enumerate() {
            if index > 0 {
                tokio::time::sleep(launch_interval).await;
            }

            if matches!(candidate.status, OAuthVaultRecordStatus::Ready) {
                self.probe_oauth_vault_admission_inner(candidate.id).await?;
                let Some(updated_candidate) = self
                    .oauth_refresh_token_vault
                    .read()
                    .unwrap()
                    .get(&candidate.id)
                    .cloned()
                else {
                    continue;
                };
                if !matches!(updated_candidate.status, OAuthVaultRecordStatus::Ready) {
                    continue;
                }
                candidate = updated_candidate;
            }

            let activation_result = match candidate.status {
                OAuthVaultRecordStatus::Ready => self
                    .materialize_ready_oauth_vault_record_inner(&candidate)
                    .map(|account| OAuthUpsertResult {
                        account,
                        created: false,
                    }),
                OAuthVaultRecordStatus::NeedsRefresh | OAuthVaultRecordStatus::Queued => {
                    let cipher = self.require_credential_cipher()?;
                    let refresh_token = match cipher.decrypt(&candidate.refresh_token_enc) {
                        Ok(refresh_token) => refresh_token,
                        Err(err) => {
                            let error_message = truncate_error_message(err.to_string());
                            let error_code = "vault_decrypt_failed".to_string();
                            self.mark_oauth_vault_activation_failed_inner(
                                candidate.id,
                                candidate.status,
                                error_code,
                                error_message,
                            );
                            continue;
                        }
                    };

                    let fallback_access_token = candidate
                        .fallback_access_token_enc
                        .as_deref()
                        .map(|token| cipher.decrypt(token))
                        .transpose()?;

                    let req = ImportOAuthRefreshTokenRequest {
                        label: candidate.label.clone(),
                        base_url: candidate.base_url.clone(),
                        refresh_token,
                        fallback_access_token,
                        fallback_token_expires_at: candidate.fallback_token_expires_at,
                        chatgpt_account_id: candidate.chatgpt_account_id.clone(),
                        mode: Some(candidate.desired_mode.clone()),
                        enabled: Some(candidate.desired_enabled),
                        priority: Some(candidate.desired_priority),
                        chatgpt_plan_type: candidate.chatgpt_plan_type.clone(),
                        source_type: candidate.source_type.clone(),
                    };

                    self.upsert_oauth_refresh_token_inner(req).await
                }
                OAuthVaultRecordStatus::NoQuota | OAuthVaultRecordStatus::Failed => continue,
            };

            match activation_result {
                Ok(upserted) => {
                    self.set_account_pool_state_active_inner(upserted.account.id, Utc::now());
                    self.oauth_refresh_token_vault
                        .write()
                        .unwrap()
                        .remove(&candidate.id);
                    activated = activated.saturating_add(1);
                }
                Err(err) => {
                    let error_message = truncate_error_message(err.to_string());
                    let error_code =
                        classify_vault_activation_error_code(error_message.as_str()).to_string();
                    self.mark_oauth_vault_activation_failed_inner(
                        candidate.id,
                        candidate.status,
                        error_code,
                        error_message,
                    );
                }
            }
        }

        Ok(activated)
    }

    fn import_oauth_refresh_token_with_token_info_inner(
        &self,
        req: ImportOAuthRefreshTokenRequest,
        resolved_mode: UpstreamMode,
        normalized_base_url: String,
        token_info: &OAuthTokenInfo,
    ) -> Result<UpstreamAccount> {
        let cipher = self.require_credential_cipher()?;
        let account = UpstreamAccount {
            id: Uuid::new_v4(),
            label: req.label,
            mode: resolved_mode,
            base_url: normalized_base_url,
            bearer_token: OAUTH_MANAGED_BEARER_SENTINEL.to_string(),
            chatgpt_account_id: req
                .chatgpt_account_id
                .or(token_info.chatgpt_account_id.clone()),
            enabled: req.enabled.unwrap_or(true),
            priority: req.priority.unwrap_or(100),
            created_at: Utc::now(),
        };

        let oauth_credential = OAuthCredentialRecord::from_token_info(cipher, &token_info)?;
        let mut oauth_credential = oauth_credential;
        oauth_credential.set_fallback_access_token(
            cipher,
            req.fallback_access_token.as_deref(),
            req.fallback_token_expires_at,
        )?;

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

    async fn import_oauth_refresh_token_inner(
        &self,
        req: ImportOAuthRefreshTokenRequest,
    ) -> Result<UpstreamAccount> {
        let resolved_mode = resolve_oauth_import_mode(req.mode.clone(), req.source_type.as_deref());
        let normalized_base_url =
            normalize_upstream_account_base_url(&resolved_mode, &req.base_url);
        let token_info = self
            .oauth_client
            .refresh_token(&req.refresh_token, Some(&normalized_base_url))
            .await
            .map_err(|err| anyhow!(err.to_string()))?;

        self.import_oauth_refresh_token_with_token_info_inner(
            req,
            resolved_mode,
            normalized_base_url,
            &token_info,
        )
    }

    async fn upsert_oauth_refresh_token_inner(
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
            account.base_url = normalized_base_url;
            account.bearer_token = OAUTH_MANAGED_BEARER_SENTINEL.to_string();
            account.chatgpt_account_id = normalized_chatgpt_account_id.clone();
            account.enabled = req.enabled.unwrap_or(true);
            account.priority = req.priority.unwrap_or(100);

            let existing_credential = self
                .oauth_credentials
                .read()
                .unwrap()
                .get(&account_id)
                .cloned();
            let mut credential = OAuthCredentialRecord::from_token_info(cipher, &token_info)?;
            if let Some(existing) = existing_credential.as_ref() {
                credential.token_family_id = existing.token_family_id.clone();
                credential.token_version = existing.token_version.saturating_add(1);
            }
            if req
                .fallback_access_token
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty())
            {
                credential.set_fallback_access_token(
                    cipher,
                    req.fallback_access_token.as_deref(),
                    req.fallback_token_expires_at,
                )?;
            } else if let Some(existing) = existing_credential.as_ref() {
                credential.fallback_access_token_enc = existing.fallback_access_token_enc.clone();
                credential.fallback_token_expires_at = existing.fallback_token_expires_at;
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

        let account = self.import_oauth_refresh_token_with_token_info_inner(
            req,
            resolved_mode,
            normalized_base_url,
            &token_info,
        )?;
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
        let normalized_base_url = normalize_upstream_account_base_url(&req.mode, &req.base_url);

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
            account.base_url = normalized_base_url.clone();
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
            base_url: normalized_base_url,
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
            return self.oauth_account_status_inner(account_id).await;
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
        if refresh_credential_is_terminal_invalid(
            &oauth_credential.last_refresh_status,
            oauth_credential.refresh_reused_detected,
            oauth_credential.last_refresh_error_code.as_deref(),
        ) && has_usable_access_token_fallback(
            oauth_credential.has_access_token_fallback(),
            oauth_credential.fallback_token_expires_at,
            now,
        ) {
            return Ok(self.oauth_status_from(
                &account,
                UpstreamAuthProvider::OAuthRefreshToken,
                Some(&oauth_credential),
                session_profile.as_ref(),
            ));
        }
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
                let error_code = err.code().as_str().to_string();
                oauth_credential.last_refresh_status = OAuthRefreshStatus::Failed;
                oauth_credential.last_refresh_at = Some(now);
                oauth_credential.refresh_reused_detected = matches!(
                    error_code.as_str(),
                    "refresh_token_reused" | "refresh_token_revoked"
                );
                oauth_credential.last_refresh_error_code = Some(error_code.clone());
                oauth_credential.last_refresh_error = Some(truncate_error_message(err.to_string()));
                oauth_credential.refresh_failure_count =
                    oauth_credential.refresh_failure_count.saturating_add(1);
                oauth_credential.refresh_backoff_until =
                    Some(now + oauth_credential.backoff_duration());
                self.upsert_oauth_credential(account_id, oauth_credential.clone());
                if oauth_credential.refresh_reused_detected {
                    let family_id = oauth_credential.token_family_id.clone();
                    self.mark_oauth_family_pending_purge_inner(&family_id, Some(error_code));
                } else if is_fatal_refresh_error_code(Some(error_code.as_str()))
                    && !has_usable_access_token_fallback(
                        oauth_credential.has_access_token_fallback(),
                        oauth_credential.fallback_token_expires_at,
                        now,
                    )
                {
                    let _ =
                        self.mark_upstream_account_pending_purge_inner(account_id, Some(error_code));
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
                        if self.account_pool_state_record_inner(*account_id).pool_state
                            != AccountPoolState::Active
                        {
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
