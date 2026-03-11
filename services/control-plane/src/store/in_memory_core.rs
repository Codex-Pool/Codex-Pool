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
            session_profiles: Arc::new(RwLock::new(HashMap::new())),
            account_health_states: Arc::new(RwLock::new(HashMap::new())),
            account_model_support: Arc::new(RwLock::new(HashMap::new())),
            policies: Arc::new(RwLock::new(HashMap::new())),
            routing_profiles: Arc::new(RwLock::new(HashMap::new())),
            model_routing_policies: Arc::new(RwLock::new(HashMap::new())),
            ai_routing_settings: Arc::new(RwLock::new(AiRoutingSettings {
                enabled: true,
                auto_publish: true,
                planner_model_chain: Vec::new(),
                trigger_mode: AiRoutingTriggerMode::Hybrid,
                kill_switch: false,
                updated_at: Utc::now(),
            })),
            routing_plan_versions: Arc::new(RwLock::new(Vec::new())),
            revision: Arc::new(AtomicU64::new(1)),
            oauth_client,
            credential_cipher,
        }
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
        let account = UpstreamAccount {
            id: Uuid::new_v4(),
            label: req.label,
            mode: req.mode,
            base_url: req.base_url,
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
            for account_id in expired_ids {
                accounts.remove(&account_id);
                providers.remove(&account_id);
                oauth_credentials.remove(&account_id);
                session_profiles.remove(&account_id);
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
        let effective_enabled = match (provider.clone(), credential_kind.clone()) {
            (UpstreamAuthProvider::OAuthRefreshToken, _) => {
                account.enabled && token_expires_at.is_some_and(|expires_at| expires_at > now_guard)
            }
            (_, Some(SessionCredentialKind::OneTimeAccessToken)) => {
                account.enabled
                    && token_expires_at
                        .map(|expires_at| expires_at > now_guard)
                        .unwrap_or(true)
            }
            _ => account.enabled,
        };
        let next_refresh_at = match provider {
            UpstreamAuthProvider::OAuthRefreshToken => token_expires_at
                .map(|expires_at| expires_at - Duration::seconds(OAUTH_REFRESH_WINDOW_SEC)),
            _ => None,
        };

        OAuthAccountStatusResponse {
            account_id: account.id,
            auth_provider: provider,
            credential_kind,
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
            rate_limits: Vec::new(),
            rate_limits_fetched_at: None,
            rate_limits_expires_at: None,
            rate_limits_last_error_code: None,
            rate_limits_last_error: None,
            next_refresh_at,
        }
    }
}
