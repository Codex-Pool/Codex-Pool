impl InMemoryStore {
    pub fn new_with_oauth(
        oauth_client: Arc<dyn OAuthTokenClient>,
        credential_cipher: Option<CredentialCipher>,
    ) -> Self {
        Self {
            tenants: Arc::new(RwLock::new(HashMap::new())),
            api_keys: Arc::new(RwLock::new(HashMap::new())),
            api_key_tokens: Arc::new(RwLock::new(HashMap::new())),
            accounts: Arc::new(RwLock::new(HashMap::new())),
            account_auth_providers: Arc::new(RwLock::new(HashMap::new())),
            oauth_credentials: Arc::new(RwLock::new(HashMap::new())),
            oauth_refresh_token_vault: Arc::new(RwLock::new(HashMap::new())),
            session_profiles: Arc::new(RwLock::new(HashMap::new())),
            account_health_states: Arc::new(RwLock::new(HashMap::new())),
            account_model_support: Arc::new(RwLock::new(HashMap::new())),
            oauth_rate_limit_caches: Arc::new(RwLock::new(HashMap::new())),
            oauth_rate_limit_refresh_jobs: Arc::new(RwLock::new(HashMap::new())),
            outbound_proxy_pool_settings: Arc::new(RwLock::new(
                OutboundProxyPoolSettings::default(),
            )),
            outbound_proxy_nodes: Arc::new(RwLock::new(HashMap::new())),
            policies: Arc::new(RwLock::new(HashMap::new())),
            routing_profiles: Arc::new(RwLock::new(HashMap::new())),
            model_routing_policies: Arc::new(RwLock::new(HashMap::new())),
            model_routing_settings: Arc::new(RwLock::new(ModelRoutingSettings {
                enabled: true,
                auto_publish: true,
                planner_model_chain: Vec::new(),
                trigger_mode: ModelRoutingTriggerMode::Hybrid,
                kill_switch: false,
                updated_at: Utc::now(),
            })),
            upstream_error_learning_settings: Arc::new(RwLock::new(
                AiErrorLearningSettings::default(),
            )),
            upstream_error_templates: Arc::new(RwLock::new(HashMap::new())),
            upstream_error_template_index: Arc::new(RwLock::new(HashMap::new())),
            builtin_error_template_overrides: Arc::new(RwLock::new(HashMap::new())),
            routing_plan_versions: Arc::new(RwLock::new(Vec::new())),
            revision: Arc::new(AtomicU64::new(1)),
            oauth_client,
            credential_cipher,
        }
    }

    fn list_builtin_error_templates_inner(&self) -> Vec<BuiltinErrorTemplateRecord> {
        let mut templates = default_builtin_error_templates();
        let overrides = self.builtin_error_template_overrides.read().unwrap();
        for template in &mut templates {
            if let Some(record) = overrides.get(&(template.kind, template.code.clone())) {
                template.templates =
                    merge_localized_error_templates(&template.default_templates, &record.templates);
                template.is_overridden = true;
                template.updated_at = Some(record.updated_at);
            }
        }
        templates.sort_by(|left, right| {
            left.kind
                .cmp(&right.kind)
                .then_with(|| left.code.cmp(&right.code))
        });
        templates
    }

    fn create_tenant_inner(&self, req: CreateTenantRequest) -> Tenant {
        let tenant = Tenant {
            id: Uuid::new_v4(),
            name: req.name,
            created_at: Utc::now(),
        };
        self.tenants
            .write()
            .unwrap()
            .insert(tenant.id, tenant.clone());
        tenant
    }

    fn list_tenants_inner(&self) -> Vec<Tenant> {
        self.tenants
            .read()
            .unwrap()
            .values()
            .cloned()
            .collect::<Vec<_>>()
    }

    fn create_api_key_inner(&self, req: CreateApiKeyRequest) -> CreateApiKeyResponse {
        let plaintext = format!("cp_{}", Uuid::new_v4().simple());
        let key_hash = hash_api_key_token(&plaintext);

        let record = ApiKey {
            id: Uuid::new_v4(),
            tenant_id: req.tenant_id,
            name: req.name,
            key_prefix: plaintext.chars().take(12).collect(),
            key_hash,
            enabled: true,
            created_at: Utc::now(),
        };

        self.api_keys
            .write()
            .unwrap()
            .insert(record.id, record.clone());
        self.api_key_tokens
            .write()
            .unwrap()
            .insert(hash_api_key_token(&plaintext), record.id);

        CreateApiKeyResponse {
            record,
            plaintext_key: plaintext,
        }
    }

    fn list_api_keys_inner(&self) -> Vec<ApiKey> {
        self.api_keys
            .read()
            .unwrap()
            .values()
            .cloned()
            .collect::<Vec<_>>()
    }

    fn list_outbound_proxy_nodes_inner(&self) -> Vec<OutboundProxyNode> {
        let mut nodes = self
            .outbound_proxy_nodes
            .read()
            .unwrap()
            .values()
            .cloned()
            .collect::<Vec<_>>();
        nodes.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.cmp(&right.id))
        });
        nodes
    }

    fn create_outbound_proxy_node_inner(
        &self,
        req: CreateOutboundProxyNodeRequest,
    ) -> OutboundProxyNode {
        let now = Utc::now();
        let node = OutboundProxyNode {
            id: Uuid::new_v4(),
            label: req.label,
            proxy_url: req.proxy_url,
            enabled: req.enabled.unwrap_or(true),
            weight: req.weight.unwrap_or(1),
            last_test_status: None,
            last_latency_ms: None,
            last_error: None,
            last_tested_at: None,
            created_at: now,
            updated_at: now,
        };
        self.outbound_proxy_nodes
            .write()
            .unwrap()
            .insert(node.id, node.clone());
        self.revision.fetch_add(1, Ordering::Relaxed);
        node
    }

    fn update_outbound_proxy_node_inner(
        &self,
        node_id: Uuid,
        req: UpdateOutboundProxyNodeRequest,
    ) -> Result<OutboundProxyNode> {
        let mut nodes = self.outbound_proxy_nodes.write().unwrap();
        let Some(node) = nodes.get_mut(&node_id) else {
            return Err(anyhow!("outbound proxy node not found"));
        };

        if let Some(label) = req.label {
            node.label = label;
        }
        if let Some(proxy_url) = req.proxy_url {
            node.proxy_url = proxy_url;
        }
        if let Some(enabled) = req.enabled {
            node.enabled = enabled;
        }
        if let Some(weight) = req.weight {
            node.weight = weight;
        }
        node.updated_at = Utc::now();
        let updated = node.clone();
        drop(nodes);
        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(updated)
    }

    fn delete_outbound_proxy_node_inner(&self, node_id: Uuid) -> Result<()> {
        let removed = self.outbound_proxy_nodes.write().unwrap().remove(&node_id);
        if removed.is_none() {
            return Err(anyhow!("outbound proxy node not found"));
        }
        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn record_outbound_proxy_test_result_inner(
        &self,
        node_id: Uuid,
        last_test_status: Option<String>,
        last_latency_ms: Option<u64>,
        last_error: Option<String>,
        last_tested_at: Option<DateTime<Utc>>,
    ) -> Result<OutboundProxyNode> {
        let mut nodes = self.outbound_proxy_nodes.write().unwrap();
        let Some(node) = nodes.get_mut(&node_id) else {
            return Err(anyhow!("outbound proxy node not found"));
        };
        node.last_test_status = last_test_status;
        node.last_latency_ms = last_latency_ms;
        node.last_error = last_error;
        node.last_tested_at = last_tested_at;
        node.updated_at = Utc::now();
        let updated = node.clone();
        drop(nodes);
        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(updated)
    }

    fn set_api_key_enabled_inner(&self, api_key_id: Uuid, enabled: bool) -> Result<ApiKey> {
        let mut keys = self.api_keys.write().unwrap();
        let key = keys
            .get_mut(&api_key_id)
            .ok_or_else(|| anyhow!("api key not found"))?;
        key.enabled = enabled;
        Ok(key.clone())
    }

    fn validate_api_key_inner(&self, token: &str) -> Option<ValidatedPrincipal> {
        let token_hash = hash_api_key_token(token);
        let api_key_id = self
            .api_key_tokens
            .read()
            .unwrap()
            .get(&token_hash)
            .copied()?;
        let key = self.api_keys.read().unwrap().get(&api_key_id)?.clone();
        Some(ValidatedPrincipal {
            tenant_id: key.tenant_id,
            api_key_id: key.id,
            api_key_group_id: Uuid::nil(),
            api_key_group_name: "default".to_string(),
            api_key_group_invalid: false,
            enabled: key.enabled,
            key_ip_allowlist: Vec::new(),
            key_model_allowlist: Vec::new(),
            tenant_status: Some("active".to_string()),
            tenant_expires_at: None,
            balance_microcredits: None,
        })
    }

    fn create_upstream_account_inner(&self, req: CreateUpstreamAccountRequest) -> UpstreamAccount {
        let base_url = normalize_upstream_account_base_url(&req.mode, &req.base_url);
        let account = UpstreamAccount {
            id: Uuid::new_v4(),
            label: req.label,
            mode: req.mode,
            base_url,
            bearer_token: req.bearer_token,
            chatgpt_account_id: req.chatgpt_account_id,
            enabled: req.enabled.unwrap_or(true),
            priority: req.priority.unwrap_or(100),
            created_at: Utc::now(),
        };

        let auth_provider = UpstreamAuthProvider::LegacyBearer;

        self.accounts
            .write()
            .unwrap()
            .insert(account.id, account.clone());
        self.account_auth_providers
            .write()
            .unwrap()
            .insert(account.id, auth_provider);
        self.revision.fetch_add(1, Ordering::Relaxed);

        account
    }

    fn list_upstream_accounts_inner(&self) -> Vec<UpstreamAccount> {
        self.purge_expired_one_time_accounts_inner();
        self.accounts
            .read()
            .unwrap()
            .values()
            .cloned()
            .collect::<Vec<_>>()
    }

    fn set_upstream_account_enabled_inner(
        &self,
        account_id: Uuid,
        enabled: bool,
    ) -> Result<UpstreamAccount> {
        let mut accounts = self.accounts.write().unwrap();
        let account = accounts
            .get_mut(&account_id)
            .ok_or_else(|| anyhow!("upstream account not found"))?;
        account.enabled = enabled;
        let updated = account.clone();
        drop(accounts);

        if enabled {
            self.set_account_pool_state_active_inner(account_id, Utc::now());
        }

        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(updated)
    }

    fn delete_upstream_account_inner(&self, account_id: Uuid) -> Result<()> {
        let mut accounts = self.accounts.write().unwrap();
        if accounts.remove(&account_id).is_none() {
            return Err(anyhow!("upstream account not found"));
        }
        drop(accounts);

        self.account_auth_providers
            .write()
            .unwrap()
            .remove(&account_id);
        self.oauth_credentials.write().unwrap().remove(&account_id);
        self.session_profiles.write().unwrap().remove(&account_id);
        self.account_health_states
            .write()
            .unwrap()
            .remove(&account_id);
        self.oauth_rate_limit_caches
            .write()
            .unwrap()
            .remove(&account_id);

        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn upsert_routing_policy_inner(&self, req: UpsertRoutingPolicyRequest) -> RoutingPolicy {
        let policy = RoutingPolicy {
            tenant_id: req.tenant_id,
            strategy: req.strategy,
            max_retries: req.max_retries,
            stream_max_retries: req.stream_max_retries,
            updated_at: Utc::now(),
        };

        self.policies
            .write()
            .unwrap()
            .insert(policy.tenant_id, policy.clone());
        self.revision.fetch_add(1, Ordering::Relaxed);

        policy
    }

    fn upsert_retry_policy_inner(&self, req: UpsertRetryPolicyRequest) -> RoutingPolicy {
        let mut policies = self.policies.write().unwrap();
        let policy = policies
            .entry(req.tenant_id)
            .and_modify(|policy| {
                policy.max_retries = req.max_retries;
                policy.updated_at = Utc::now();
            })
            .or_insert_with(|| RoutingPolicy {
                tenant_id: req.tenant_id,
                strategy: RoutingStrategy::RoundRobin,
                max_retries: req.max_retries,
                stream_max_retries: 0,
                updated_at: Utc::now(),
            })
            .clone();
        drop(policies);

        self.revision.fetch_add(1, Ordering::Relaxed);
        policy
    }

    fn upsert_stream_retry_policy_inner(
        &self,
        req: UpsertStreamRetryPolicyRequest,
    ) -> RoutingPolicy {
        let mut policies = self.policies.write().unwrap();
        let policy = policies
            .entry(req.tenant_id)
            .and_modify(|policy| {
                policy.stream_max_retries = req.stream_max_retries;
                policy.updated_at = Utc::now();
            })
            .or_insert_with(|| RoutingPolicy {
                tenant_id: req.tenant_id,
                strategy: RoutingStrategy::RoundRobin,
                max_retries: 0,
                stream_max_retries: req.stream_max_retries,
                updated_at: Utc::now(),
            })
            .clone();
        drop(policies);

        self.revision.fetch_add(1, Ordering::Relaxed);
        policy
    }

    fn mark_account_seen_ok_inner(
        &self,
        account_id: Uuid,
        seen_ok_at: DateTime<Utc>,
        min_write_interval_sec: i64,
    ) -> bool {
        let mut states = self.account_health_states.write().unwrap();
        let state = states.entry(account_id).or_default();
        let min_interval = Duration::seconds(min_write_interval_sec.max(0));
        if state
            .seen_ok_at
            .is_some_and(|previous| previous >= seen_ok_at - min_interval)
        {
            return false;
        }
        state.seen_ok_at = Some(
            state
                .seen_ok_at
                .map_or(seen_ok_at, |previous| previous.max(seen_ok_at)),
        );
        true
    }

    fn runtime_pool_account_count_inner(&self) -> usize {
        self.accounts.read().unwrap().len()
    }

    fn set_account_pool_state_active_inner(&self, account_id: Uuid, at: DateTime<Utc>) {
        let mut states = self.account_health_states.write().unwrap();
        let state = states.entry(account_id).or_default();
        state.pool_state = AccountPoolState::Active;
        state.quarantine_until = None;
        state.quarantine_reason = None;
        state.pending_purge_at = None;
        state.pending_purge_reason = None;
        state.last_pool_transition_at = Some(at);
    }

    fn set_account_pool_state_quarantine_inner(
        &self,
        account_id: Uuid,
        at: DateTime<Utc>,
        until: Option<DateTime<Utc>>,
        reason: Option<String>,
    ) -> Result<()> {
        if !self.accounts.read().unwrap().contains_key(&account_id) {
            return Err(anyhow!("upstream account not found"));
        }
        let normalized_reason = reason.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
        let mut states = self.account_health_states.write().unwrap();
        let state = states.entry(account_id).or_default();
        state.pool_state = AccountPoolState::Quarantine;
        state.quarantine_until = until;
        state.quarantine_reason = normalized_reason;
        state.pending_purge_at = None;
        state.pending_purge_reason = None;
        state.last_pool_transition_at = Some(at);
        drop(states);

        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn record_upstream_account_live_result_inner(
        &self,
        account_id: Uuid,
        reported_at: DateTime<Utc>,
        status: OAuthLiveResultStatus,
        source: OAuthLiveResultSource,
        status_code: Option<u16>,
        error_code: Option<String>,
        error_message_preview: Option<String>,
    ) -> Result<bool> {
        if !self.accounts.read().unwrap().contains_key(&account_id) {
            return Err(anyhow!("upstream account not found"));
        }

        let normalized_error_code = error_code.and_then(|value| {
            let normalized = value.trim().to_ascii_lowercase().replace([' ', '-'], "_");
            if normalized.is_empty() {
                None
            } else {
                Some(normalized)
            }
        });
        let normalized_error_message = error_message_preview.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(truncate_error_message(trimmed.to_string()))
            }
        });

        {
            let mut states = self.account_health_states.write().unwrap();
            let state = states.entry(account_id).or_default();
            state.last_live_result_at = Some(reported_at);
            state.last_live_result_status = Some(status.clone());
            state.last_live_result_source = Some(source);
            state.last_live_result_status_code =
                status_code.filter(|code| (200..600).contains(code));
            if status == OAuthLiveResultStatus::Failed {
                state.last_live_error_code = normalized_error_code.clone();
                state.last_live_error_message_preview = normalized_error_message.clone();
            } else {
                state.last_live_error_code = None;
                state.last_live_error_message_preview = None;
            }
        }

        let accepted = match status {
            OAuthLiveResultStatus::Ok => {
                self.mark_account_seen_ok_inner(account_id, reported_at, 0);
                self.set_account_pool_state_active_inner(account_id, reported_at);
                true
            }
            OAuthLiveResultStatus::Failed => match normalized_error_code.as_deref() {
                Some("account_deactivated") => {
                    self.mark_upstream_account_pending_purge_inner(
                        account_id,
                        Some("account_deactivated".to_string()),
                    )?;
                    true
                }
                Some("rate_limited") | Some("quota_exhausted") => {
                    let reason = normalized_error_code
                        .clone()
                        .unwrap_or_else(|| "rate_limited".to_string());
                    let until = Some(
                        reported_at
                            + Duration::seconds(rate_limit_failure_backoff_seconds(&reason, "")),
                    );
                    self.set_account_pool_state_quarantine_inner(
                        account_id,
                        reported_at,
                        until,
                        Some(reason),
                    )?;
                    true
                }
                Some("auth_expired") | Some("token_invalidated") => {
                    self.set_account_pool_state_quarantine_inner(
                        account_id,
                        reported_at,
                        Some(reported_at + Duration::minutes(30)),
                        normalized_error_code.clone(),
                    )?;
                    true
                }
                _ => false,
            },
        };

        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(accepted)
    }

    fn mark_upstream_account_pending_purge_inner(
        &self,
        account_id: Uuid,
        reason: Option<String>,
    ) -> Result<UpstreamAccount> {
        let now = Utc::now();
        let pending_purge_at = now + Duration::seconds(pending_purge_delay_sec_from_env());
        let normalized_reason = reason.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });

        let mut accounts = self.accounts.write().unwrap();
        let account = accounts
            .get_mut(&account_id)
            .ok_or_else(|| anyhow!("upstream account not found"))?;
        account.enabled = false;
        let updated = account.clone();
        drop(accounts);

        let mut states = self.account_health_states.write().unwrap();
        let state = states.entry(account_id).or_default();
        let scheduled_at = state
            .pending_purge_at
            .filter(|existing| *existing > now)
            .unwrap_or(pending_purge_at);
        state.pool_state = AccountPoolState::PendingPurge;
        state.quarantine_until = None;
        state.quarantine_reason = None;
        state.pending_purge_at = Some(scheduled_at);
        state.pending_purge_reason = normalized_reason.or_else(|| state.pending_purge_reason.clone());
        state.last_pool_transition_at = Some(now);
        drop(states);

        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(updated)
    }

    fn pending_purge_due_account_ids_inner(&self, now: DateTime<Utc>, limit: usize) -> Vec<Uuid> {
        let mut items = self
            .account_health_states
            .read()
            .unwrap()
            .iter()
            .filter_map(|(account_id, state)| {
                if state.pool_state != AccountPoolState::PendingPurge {
                    return None;
                }
                match state.pending_purge_at {
                    Some(pending_purge_at) if pending_purge_at > now => None,
                    _ => Some((*account_id, state.pending_purge_at, state.last_pool_transition_at)),
                }
            })
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            left.1
                .cmp(&right.1)
                .then_with(|| left.2.cmp(&right.2))
                .then_with(|| left.0.cmp(&right.0))
        });
        items.truncate(limit);
        items.into_iter().map(|item| item.0).collect()
    }

    fn purge_pending_upstream_accounts_inner(&self) -> Result<u64> {
        let candidates =
            self.pending_purge_due_account_ids_inner(Utc::now(), pending_purge_batch_size_from_env());
        let mut purged = 0_u64;
        for account_id in candidates {
            if self.delete_upstream_account_inner(account_id).is_ok() {
                purged = purged.saturating_add(1);
            }
        }
        Ok(purged)
    }

    fn account_pool_state_record_inner(&self, account_id: Uuid) -> UpstreamAccountHealthStateRecord {
        let mut states = self.account_health_states.write().unwrap();
        let now = Utc::now();
        let state = states.entry(account_id).or_default();
        if state.pool_state == AccountPoolState::Quarantine
            && state
                .quarantine_until
                .is_some_and(|quarantine_until| quarantine_until <= now)
        {
            state.pool_state = AccountPoolState::Active;
            state.quarantine_until = None;
            state.quarantine_reason = None;
            state.last_pool_transition_at = Some(now);
        }
        state.clone()
    }

    fn require_credential_cipher(&self) -> Result<&CredentialCipher> {
        self.credential_cipher.as_ref().ok_or_else(|| {
            anyhow!("oauth credential encryption key is missing: set CREDENTIALS_ENCRYPTION_KEY")
        })
    }

    fn account_auth_provider(&self, account_id: Uuid) -> UpstreamAuthProvider {
        self.account_auth_providers
            .read()
            .unwrap()
            .get(&account_id)
            .cloned()
            .unwrap_or(UpstreamAuthProvider::LegacyBearer)
    }

    fn upsert_oauth_credential(&self, account_id: Uuid, credential: OAuthCredentialRecord) {
        self.oauth_credentials
            .write()
            .unwrap()
            .insert(account_id, credential);
    }

    fn upsert_session_profile(&self, account_id: Uuid, profile: SessionProfileRecord) {
        self.session_profiles
            .write()
            .unwrap()
            .insert(account_id, profile);
    }

    fn purge_expired_one_time_accounts_inner(&self) {
        let now = Utc::now() + Duration::seconds(OAUTH_MIN_VALID_SEC);
        let expired_ids = {
            let profiles = self.session_profiles.read().unwrap();
            profiles
                .iter()
                .filter_map(|(account_id, profile)| {
                    if profile.credential_kind != SessionCredentialKind::OneTimeAccessToken {
                        return None;
                    }
                    profile
                        .token_expires_at
                        .filter(|expires_at| *expires_at <= now)
                        .map(|_| *account_id)
                })
                .collect::<Vec<_>>()
        };

        if expired_ids.is_empty() {
            return;
        }

        {
            let mut accounts = self.accounts.write().unwrap();
            let mut providers = self.account_auth_providers.write().unwrap();
            let mut oauth_credentials = self.oauth_credentials.write().unwrap();
            let mut session_profiles = self.session_profiles.write().unwrap();
            let mut rate_limit_caches = self.oauth_rate_limit_caches.write().unwrap();
            for account_id in expired_ids {
                accounts.remove(&account_id);
                providers.remove(&account_id);
                oauth_credentials.remove(&account_id);
                session_profiles.remove(&account_id);
                rate_limit_caches.remove(&account_id);
            }
        }

        self.revision.fetch_add(1, Ordering::Relaxed);
    }

    fn oauth_status_from(
        &self,
        account: &UpstreamAccount,
        provider: UpstreamAuthProvider,
        credential: Option<&OAuthCredentialRecord>,
        session_profile: Option<&SessionProfileRecord>,
    ) -> OAuthAccountStatusResponse {
        let token_expires_at = credential
            .map(|item| item.token_expires_at)
            .or_else(|| session_profile.and_then(|item| item.token_expires_at));
        let credential_kind = session_profile
            .map(|item| item.credential_kind.clone())
            .or_else(|| match (provider.clone(), account.mode.clone()) {
                (UpstreamAuthProvider::OAuthRefreshToken, _) => {
                    Some(SessionCredentialKind::RefreshRotatable)
                }
                (UpstreamAuthProvider::LegacyBearer, UpstreamMode::ChatGptSession)
                | (UpstreamAuthProvider::LegacyBearer, UpstreamMode::CodexOauth) => {
                    Some(SessionCredentialKind::OneTimeAccessToken)
                }
                _ => None,
            });
        let now_guard = Utc::now() + Duration::seconds(OAUTH_MIN_VALID_SEC);
        let rate_limit_cache = self
            .oauth_rate_limit_caches
            .read()
            .unwrap()
            .get(&account.id)
            .cloned()
            .unwrap_or_default();
        let last_refresh_status = credential
            .map(|item| item.last_refresh_status.clone())
            .unwrap_or(OAuthRefreshStatus::Never);
        let effective_enabled = oauth_effective_enabled(
            account.enabled,
            &provider,
            credential_kind.as_ref(),
            token_expires_at,
            credential
                .map(|item| item.has_access_token_fallback())
                .unwrap_or(false),
            credential.and_then(|item| item.fallback_token_expires_at),
            &last_refresh_status,
            credential
                .map(|item| item.refresh_reused_detected)
                .unwrap_or(false),
            credential
                .and_then(|item| item.last_refresh_error_code.as_deref()),
            rate_limit_cache.expires_at,
            rate_limit_cache.last_error_code.as_deref(),
            rate_limit_cache.last_error.as_deref(),
            now_guard,
        );
        let has_refresh_credential = has_refresh_credential(&provider);
        let refresh_credential_state = refresh_credential_state(
            &provider,
            &last_refresh_status,
            credential
                .map(|item| item.refresh_reused_detected)
                .unwrap_or(false),
            credential
                .and_then(|item| item.last_refresh_error_code.as_deref()),
        );
        let next_refresh_at = match provider {
            UpstreamAuthProvider::OAuthRefreshToken => token_expires_at
                .map(|expires_at| expires_at - Duration::seconds(OAUTH_REFRESH_WINDOW_SEC)),
            _ => None,
        };
        let supported_models = self
            .account_model_support
            .read()
            .unwrap()
            .get(&account.id)
            .map(|item| item.supported_models.clone())
            .unwrap_or_default();
        let pool_state_record = self.account_pool_state_record_inner(account.id);
        let pool_state = match pool_state_record.pool_state {
            AccountPoolState::Active => OAuthAccountPoolState::Active,
            AccountPoolState::Quarantine => OAuthAccountPoolState::Quarantine,
            AccountPoolState::PendingPurge => OAuthAccountPoolState::PendingPurge,
        };

        OAuthAccountStatusResponse {
            account_id: account.id,
            auth_provider: provider,
            credential_kind,
            has_refresh_credential,
            has_access_token_fallback: credential
                .map(|item| item.has_access_token_fallback())
                .unwrap_or(false),
            refresh_credential_state,
            email: session_profile.and_then(|item| item.email.clone()),
            oauth_subject: session_profile.and_then(|item| item.oauth_subject.clone()),
            oauth_identity_provider: session_profile
                .and_then(|item| item.oauth_identity_provider.clone()),
            email_verified: session_profile.and_then(|item| item.email_verified),
            chatgpt_plan_type: session_profile.and_then(|item| item.chatgpt_plan_type.clone()),
            chatgpt_user_id: session_profile.and_then(|item| item.chatgpt_user_id.clone()),
            chatgpt_subscription_active_start: session_profile
                .and_then(|item| item.chatgpt_subscription_active_start),
            chatgpt_subscription_active_until: session_profile
                .and_then(|item| item.chatgpt_subscription_active_until),
            chatgpt_subscription_last_checked: session_profile
                .and_then(|item| item.chatgpt_subscription_last_checked),
            chatgpt_account_user_id: session_profile
                .and_then(|item| item.chatgpt_account_user_id.clone()),
            chatgpt_compute_residency: session_profile
                .and_then(|item| item.chatgpt_compute_residency.clone()),
            workspace_name: session_profile.and_then(|item| item.workspace_name.clone()),
            organizations: session_profile.and_then(|item| item.organizations.clone()),
            groups: session_profile.and_then(|item| item.groups.clone()),
            source_type: session_profile.and_then(|item| item.source_type.clone()),
            token_family_id: credential.map(|item| item.token_family_id.clone()),
            token_version: credential.map(|item| item.token_version),
            token_expires_at,
            last_refresh_at: credential.and_then(|item| item.last_refresh_at),
            last_refresh_status: credential
                .map(|item| item.last_refresh_status.clone())
                .unwrap_or(OAuthRefreshStatus::Never),
            refresh_reused_detected: credential
                .map(|item| item.refresh_reused_detected)
                .unwrap_or(false),
            last_refresh_error_code: credential
                .and_then(|item| item.last_refresh_error_code.clone()),
            last_refresh_error: credential.and_then(|item| item.last_refresh_error.clone()),
            effective_enabled,
            pool_state,
            quarantine_until: pool_state_record.quarantine_until,
            quarantine_reason: pool_state_record.quarantine_reason,
            pending_purge_at: pool_state_record.pending_purge_at,
            pending_purge_reason: pool_state_record.pending_purge_reason,
            last_live_result_at: pool_state_record.last_live_result_at,
            last_live_result_status: pool_state_record.last_live_result_status,
            last_live_result_source: pool_state_record.last_live_result_source,
            last_live_result_status_code: pool_state_record.last_live_result_status_code,
            last_live_error_code: pool_state_record.last_live_error_code,
            last_live_error_message_preview: pool_state_record.last_live_error_message_preview,
            supported_models,
            rate_limits: rate_limit_cache.rate_limits,
            rate_limits_fetched_at: rate_limit_cache.fetched_at,
            rate_limits_expires_at: rate_limit_cache.expires_at,
            rate_limits_last_error_code: rate_limit_cache.last_error_code,
            rate_limits_last_error: rate_limit_cache.last_error,
            next_refresh_at,
        }
    }
}

#[cfg(test)]
mod oauth_status_tests {
    use super::*;

    #[test]
    fn oauth_status_includes_supported_models_snapshot() {
        let store = InMemoryStore::default();
        let account = UpstreamAccount {
            id: Uuid::new_v4(),
            label: "acc".to_string(),
            mode: UpstreamMode::CodexOauth,
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            bearer_token: "token".to_string(),
            chatgpt_account_id: None,
            enabled: true,
            priority: 100,
            created_at: Utc::now(),
        };
        store.account_model_support.write().unwrap().insert(
            account.id,
            AccountModelSupportRecord {
                supported_models: vec!["gpt-5.4".to_string(), "o3".to_string()],
            },
        );

        let status = store.oauth_status_from(
            &account,
            UpstreamAuthProvider::LegacyBearer,
            None,
            None,
        );

        assert_eq!(status.supported_models, vec!["gpt-5.4".to_string(), "o3".to_string()]);
    }
}
