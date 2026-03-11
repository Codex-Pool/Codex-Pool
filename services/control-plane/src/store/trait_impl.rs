#[async_trait]
impl ControlPlaneStore for InMemoryStore {
    async fn create_tenant(&self, req: CreateTenantRequest) -> Result<Tenant> {
        Ok(self.create_tenant_inner(req))
    }

    async fn list_tenants(&self) -> Result<Vec<Tenant>> {
        Ok(self.list_tenants_inner())
    }

    async fn create_api_key(&self, req: CreateApiKeyRequest) -> Result<CreateApiKeyResponse> {
        Ok(self.create_api_key_inner(req))
    }

    async fn list_api_keys(&self) -> Result<Vec<ApiKey>> {
        Ok(self.list_api_keys_inner())
    }

    async fn set_api_key_enabled(&self, api_key_id: Uuid, enabled: bool) -> Result<ApiKey> {
        self.set_api_key_enabled_inner(api_key_id, enabled)
    }

    async fn validate_api_key(&self, token: &str) -> Result<Option<ValidatedPrincipal>> {
        Ok(self.validate_api_key_inner(token))
    }

    async fn create_upstream_account(
        &self,
        req: CreateUpstreamAccountRequest,
    ) -> Result<UpstreamAccount> {
        Ok(self.create_upstream_account_inner(req))
    }

    async fn list_upstream_accounts(&self) -> Result<Vec<UpstreamAccount>> {
        Ok(self.list_upstream_accounts_inner())
    }

    async fn set_upstream_account_enabled(
        &self,
        account_id: Uuid,
        enabled: bool,
    ) -> Result<UpstreamAccount> {
        self.set_upstream_account_enabled_inner(account_id, enabled)
    }

    async fn delete_upstream_account(&self, account_id: Uuid) -> Result<()> {
        self.delete_upstream_account_inner(account_id)
    }

    async fn validate_oauth_refresh_token(
        &self,
        req: ValidateOAuthRefreshTokenRequest,
    ) -> Result<ValidateOAuthRefreshTokenResponse> {
        self.validate_oauth_refresh_token_inner(req).await
    }

    async fn import_oauth_refresh_token(
        &self,
        req: ImportOAuthRefreshTokenRequest,
    ) -> Result<UpstreamAccount> {
        self.import_oauth_refresh_token_inner(req).await
    }

    async fn upsert_oauth_refresh_token(
        &self,
        req: ImportOAuthRefreshTokenRequest,
    ) -> Result<OAuthUpsertResult> {
        self.upsert_oauth_refresh_token_inner(req).await
    }

    async fn dedupe_oauth_accounts_by_identity(&self) -> Result<u64> {
        Ok(self.dedupe_oauth_accounts_by_identity_inner(None, None, None))
    }

    async fn upsert_one_time_session_account(
        &self,
        req: UpsertOneTimeSessionAccountRequest,
    ) -> Result<OAuthUpsertResult> {
        self.upsert_one_time_session_account_inner(req)
    }

    async fn refresh_oauth_account(&self, account_id: Uuid) -> Result<OAuthAccountStatusResponse> {
        self.refresh_oauth_account_inner(account_id, true).await
    }

    async fn oauth_account_status(&self, account_id: Uuid) -> Result<OAuthAccountStatusResponse> {
        self.oauth_account_status_inner(account_id).await
    }

    async fn upsert_routing_policy(
        &self,
        req: UpsertRoutingPolicyRequest,
    ) -> Result<RoutingPolicy> {
        Ok(self.upsert_routing_policy_inner(req))
    }

    async fn upsert_retry_policy(&self, req: UpsertRetryPolicyRequest) -> Result<RoutingPolicy> {
        Ok(self.upsert_retry_policy_inner(req))
    }

    async fn upsert_stream_retry_policy(
        &self,
        req: UpsertStreamRetryPolicyRequest,
    ) -> Result<RoutingPolicy> {
        Ok(self.upsert_stream_retry_policy_inner(req))
    }

    async fn list_routing_profiles(&self) -> Result<Vec<RoutingProfile>> {
        let mut profiles = self
            .routing_profiles
            .read()
            .unwrap()
            .values()
            .cloned()
            .collect::<Vec<_>>();
        profiles.sort_by(|left, right| {
            right
                .priority
                .cmp(&left.priority)
                .then_with(|| left.created_at.cmp(&right.created_at))
        });
        Ok(profiles)
    }

    async fn upsert_routing_profile(
        &self,
        req: UpsertRoutingProfileRequest,
    ) -> Result<RoutingProfile> {
        let now = Utc::now();
        let profile = RoutingProfile {
            id: req.id.unwrap_or_else(Uuid::new_v4),
            name: req.name,
            description: req.description,
            enabled: req.enabled,
            priority: req.priority,
            selector: req.selector,
            created_at: now,
            updated_at: now,
        };
        self.routing_profiles
            .write()
            .unwrap()
            .entry(profile.id)
            .and_modify(|existing| {
                existing.name = profile.name.clone();
                existing.description = profile.description.clone();
                existing.enabled = profile.enabled;
                existing.priority = profile.priority;
                existing.selector = profile.selector.clone();
                existing.updated_at = now;
            })
            .or_insert_with(|| profile.clone());
        self.revision.fetch_add(1, Ordering::Relaxed);
        self.routing_profiles
            .read()
            .unwrap()
            .get(&profile.id)
            .cloned()
            .ok_or_else(|| anyhow!("routing profile not found after upsert"))
    }

    async fn delete_routing_profile(&self, profile_id: Uuid) -> Result<()> {
        let removed = self.routing_profiles.write().unwrap().remove(&profile_id);
        if removed.is_none() {
            return Err(anyhow!("routing profile not found"));
        }
        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn list_model_routing_policies(&self) -> Result<Vec<ModelRoutingPolicy>> {
        let mut policies = self
            .model_routing_policies
            .read()
            .unwrap()
            .values()
            .cloned()
            .collect::<Vec<_>>();
        policies.sort_by(|left, right| {
            right
                .priority
                .cmp(&left.priority)
                .then_with(|| left.created_at.cmp(&right.created_at))
        });
        Ok(policies)
    }

    async fn upsert_model_routing_policy(
        &self,
        req: UpsertModelRoutingPolicyRequest,
    ) -> Result<ModelRoutingPolicy> {
        let now = Utc::now();
        let policy = ModelRoutingPolicy {
            id: req.id.unwrap_or_else(Uuid::new_v4),
            name: req.name,
            family: req.family,
            exact_models: req.exact_models,
            model_prefixes: req.model_prefixes,
            fallback_profile_ids: req.fallback_profile_ids,
            enabled: req.enabled,
            priority: req.priority,
            created_at: now,
            updated_at: now,
        };
        self.model_routing_policies
            .write()
            .unwrap()
            .entry(policy.id)
            .and_modify(|existing| {
                existing.name = policy.name.clone();
                existing.family = policy.family.clone();
                existing.exact_models = policy.exact_models.clone();
                existing.model_prefixes = policy.model_prefixes.clone();
                existing.fallback_profile_ids = policy.fallback_profile_ids.clone();
                existing.enabled = policy.enabled;
                existing.priority = policy.priority;
                existing.updated_at = now;
            })
            .or_insert_with(|| policy.clone());
        self.revision.fetch_add(1, Ordering::Relaxed);
        self.model_routing_policies
            .read()
            .unwrap()
            .get(&policy.id)
            .cloned()
            .ok_or_else(|| anyhow!("model routing policy not found after upsert"))
    }

    async fn delete_model_routing_policy(&self, policy_id: Uuid) -> Result<()> {
        let removed = self.model_routing_policies.write().unwrap().remove(&policy_id);
        if removed.is_none() {
            return Err(anyhow!("model routing policy not found"));
        }
        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn model_routing_settings(&self) -> Result<ModelRoutingSettings> {
        Ok(self.model_routing_settings.read().unwrap().clone())
    }

    async fn update_model_routing_settings(
        &self,
        req: UpdateModelRoutingSettingsRequest,
    ) -> Result<ModelRoutingSettings> {
        let settings = ModelRoutingSettings {
            enabled: req.enabled,
            auto_publish: req.auto_publish,
            planner_model_chain: req.planner_model_chain,
            trigger_mode: req.trigger_mode,
            kill_switch: req.kill_switch,
            updated_at: Utc::now(),
        };
        *self.model_routing_settings.write().unwrap() = settings.clone();
        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(settings)
    }

    async fn list_routing_plan_versions(&self) -> Result<Vec<RoutingPlanVersion>> {
        Ok(self.routing_plan_versions.read().unwrap().clone())
    }

    async fn record_account_model_support(
        &self,
        account_id: Uuid,
        supported_models: Vec<String>,
        _checked_at: DateTime<Utc>,
    ) -> Result<()> {
        let mut normalized = supported_models
            .into_iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>();
        normalized.sort();
        normalized.dedup();

        self.account_model_support.write().unwrap().insert(
            account_id,
            AccountModelSupportRecord {
                supported_models: normalized,
            },
        );
        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn refresh_expiring_oauth_accounts(&self) -> Result<()> {
        self.refresh_expiring_oauth_accounts_inner().await;
        Ok(())
    }

    async fn set_oauth_family_enabled(
        &self,
        account_id: Uuid,
        enabled: bool,
    ) -> Result<OAuthFamilyActionResponse> {
        self.set_oauth_family_enabled_inner(account_id, enabled)
    }

    async fn snapshot(&self) -> Result<DataPlaneSnapshot> {
        self.snapshot_inner()
    }

    async fn cleanup_data_plane_outbox(&self, _retention: chrono::Duration) -> Result<u64> {
        Ok(0)
    }

    async fn data_plane_snapshot_events(
        &self,
        after: u64,
        _limit: u32,
    ) -> Result<DataPlaneSnapshotEventsResponse> {
        Ok(DataPlaneSnapshotEventsResponse {
            cursor: after,
            high_watermark: after,
            events: Vec::new(),
        })
    }

    async fn mark_account_seen_ok(
        &self,
        account_id: Uuid,
        seen_ok_at: DateTime<Utc>,
        min_write_interval_sec: i64,
    ) -> Result<bool> {
        Ok(self.mark_account_seen_ok_inner(
            account_id,
            seen_ok_at,
            min_write_interval_sec,
        ))
    }
}

fn truncate_error_message(raw: String) -> String {
    const MAX_LEN: usize = 256;
    if raw.len() <= MAX_LEN {
        return raw;
    }

    raw.chars().take(MAX_LEN).collect()
}

fn hash_api_key_token(token: &str) -> String {
    crate::security::hash_api_key_token(token)
}

fn refresh_token_sha256(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::{ControlPlaneStore, InMemoryStore, UpsertOneTimeSessionAccountRequest};
    use crate::crypto::CredentialCipher;
    use crate::oauth::{OAuthTokenClient, OAuthTokenInfo};
    use async_trait::async_trait;
    use base64::Engine;
    use chrono::{DateTime, Duration, Utc};
    use codex_pool_core::api::{
        CreateApiKeyRequest, CreateTenantRequest, ImportOAuthRefreshTokenRequest,
        UpsertModelRoutingPolicyRequest, UpsertRoutingProfileRequest,
    };
    use codex_pool_core::model::{RoutingProfileSelector, UpstreamMode};
    use serde_json::json;
    use std::sync::Arc;

    #[tokio::test]
    async fn in_memory_store_validates_plaintext_api_key() {
        let store = InMemoryStore::default();
        let tenant = store
            .create_tenant(CreateTenantRequest {
                name: "team-auth".to_string(),
            })
            .await
            .unwrap();
        let created = store
            .create_api_key(CreateApiKeyRequest {
                tenant_id: tenant.id,
                name: "primary".to_string(),
            })
            .await
            .unwrap();

        let principal = store
            .validate_api_key(&created.plaintext_key)
            .await
            .unwrap()
            .expect("principal should exist");

        assert_eq!(principal.tenant_id, tenant.id);
        assert_eq!(principal.api_key_id, created.record.id);
        assert!(principal.enabled);
    }

    #[tokio::test]
    async fn in_memory_store_does_not_expose_plaintext_api_key_hash() {
        let store = InMemoryStore::default();
        let tenant = store
            .create_tenant(CreateTenantRequest {
                name: "team-auth-hash".to_string(),
            })
            .await
            .unwrap();
        let created = store
            .create_api_key(CreateApiKeyRequest {
                tenant_id: tenant.id,
                name: "primary".to_string(),
            })
            .await
            .unwrap();

        assert!(
            !created.record.key_hash.starts_with("plaintext:"),
            "api key hash must not use plaintext prefix"
        );
        assert!(
            !created.record.key_hash.contains(&created.plaintext_key),
            "api key hash must not contain plaintext token"
        );
        assert!(
            created.record.key_hash.starts_with("hmac-sha256:"),
            "api key hash should use hmac-sha256 format"
        );
    }

    #[derive(Clone)]
    struct StaticOAuthTokenClient;

    #[async_trait]
    impl OAuthTokenClient for StaticOAuthTokenClient {
        async fn refresh_token(
            &self,
            _refresh_token: &str,
            _base_url: Option<&str>,
        ) -> Result<OAuthTokenInfo, crate::oauth::OAuthTokenClientError> {
            Ok(OAuthTokenInfo {
                access_token: "access-1".to_string(),
                refresh_token: "refresh-1".to_string(),
                expires_at: Utc::now() + Duration::seconds(3600),
                token_type: Some("Bearer".to_string()),
                scope: Some("model.read".to_string()),
                email: Some("demo@example.com".to_string()),
                oauth_subject: Some("auth0|demo".to_string()),
                oauth_identity_provider: Some("google-oauth2".to_string()),
                email_verified: Some(true),
                chatgpt_account_id: Some("acct_demo".to_string()),
                chatgpt_user_id: Some("user_demo".to_string()),
                chatgpt_plan_type: Some("pro".to_string()),
                chatgpt_subscription_active_start: Some(
                    DateTime::parse_from_rfc3339("2026-03-01T00:00:00Z")
                        .unwrap()
                        .with_timezone(&Utc),
                ),
                chatgpt_subscription_active_until: Some(
                    DateTime::parse_from_rfc3339("2026-04-01T00:00:00Z")
                        .unwrap()
                        .with_timezone(&Utc),
                ),
                chatgpt_subscription_last_checked: Some(
                    DateTime::parse_from_rfc3339("2026-03-11T00:00:00Z")
                        .unwrap()
                        .with_timezone(&Utc),
                ),
                chatgpt_account_user_id: Some("acct_user_demo".to_string()),
                chatgpt_compute_residency: Some("us".to_string()),
                workspace_name: None,
                organizations: Some(vec![json!({
                    "id": "org_demo",
                    "title": "Personal",
                })]),
                groups: Some(vec![json!({
                    "id": "grp_demo",
                    "name": "Demo Group",
                })]),
            })
        }
    }

    #[derive(Clone)]
    struct SharedAccountIdOAuthTokenClient;

    #[async_trait]
    impl OAuthTokenClient for SharedAccountIdOAuthTokenClient {
        async fn refresh_token(
            &self,
            refresh_token: &str,
            _base_url: Option<&str>,
        ) -> Result<OAuthTokenInfo, crate::oauth::OAuthTokenClientError> {
            let (email, account_user_id, workspace_name) = if refresh_token.contains("workspace-b") {
                (
                    "shared-workspace-b@example.com",
                    "acct_user_shared_workspace_b",
                    "OAI-07.11",
                )
            } else {
                (
                    "shared-workspace-a@example.com",
                    "acct_user_shared_workspace_a",
                    "OAI-03.09",
                )
            };
            Ok(OAuthTokenInfo {
                access_token: format!("access-{refresh_token}"),
                refresh_token: refresh_token.to_string(),
                expires_at: Utc::now() + Duration::seconds(3600),
                token_type: Some("Bearer".to_string()),
                scope: Some("model.read".to_string()),
                email: Some(email.to_string()),
                oauth_subject: Some("auth0|shared".to_string()),
                oauth_identity_provider: Some("google-oauth2".to_string()),
                email_verified: Some(true),
                chatgpt_account_id: Some("acct_shared".to_string()),
                chatgpt_user_id: Some("user_shared".to_string()),
                chatgpt_plan_type: Some("team".to_string()),
                chatgpt_subscription_active_start: None,
                chatgpt_subscription_active_until: None,
                chatgpt_subscription_last_checked: None,
                chatgpt_account_user_id: Some(account_user_id.to_string()),
                chatgpt_compute_residency: Some("us".to_string()),
                workspace_name: Some(workspace_name.to_string()),
                organizations: Some(vec![json!({
                    "id": "org_shared",
                    "title": "Personal",
                })]),
                groups: Some(vec![]),
            })
        }
    }

    #[derive(Clone)]
    struct TeamWorkspaceProbeOAuthTokenClient;

    #[async_trait]
    impl OAuthTokenClient for TeamWorkspaceProbeOAuthTokenClient {
        async fn refresh_token(
            &self,
            _refresh_token: &str,
            _base_url: Option<&str>,
        ) -> Result<OAuthTokenInfo, crate::oauth::OAuthTokenClientError> {
            Ok(OAuthTokenInfo {
                access_token: "probe-access".to_string(),
                refresh_token: "probe-refresh".to_string(),
                expires_at: Utc::now() + Duration::seconds(3600),
                token_type: Some("Bearer".to_string()),
                scope: Some("model.read".to_string()),
                email: Some("team-probe@example.com".to_string()),
                oauth_subject: Some("auth0|team-probe".to_string()),
                oauth_identity_provider: Some("google-oauth2".to_string()),
                email_verified: Some(true),
                chatgpt_account_id: Some("acct_probe_team".to_string()),
                chatgpt_user_id: Some("user_probe_team".to_string()),
                chatgpt_plan_type: Some("team".to_string()),
                chatgpt_subscription_active_start: None,
                chatgpt_subscription_active_until: None,
                chatgpt_subscription_last_checked: None,
                chatgpt_account_user_id: Some("acct_user_probe_team".to_string()),
                chatgpt_compute_residency: Some("us".to_string()),
                workspace_name: None,
                organizations: Some(vec![json!({
                    "id": "org_probe_team",
                    "title": "Personal",
                })]),
                groups: Some(vec![]),
            })
        }

        async fn fetch_workspace_name(
            &self,
            access_token: &str,
            _base_url: Option<&str>,
            chatgpt_account_id: Option<&str>,
        ) -> Result<Option<String>, crate::oauth::OAuthTokenClientError> {
            if access_token == "probe-access"
                && chatgpt_account_id == Some("acct_probe_team")
            {
                return Ok(Some("OAI-03.09".to_string()));
            }

            Ok(None)
        }
    }

    #[tokio::test]
    async fn in_memory_oauth_import_is_visible_in_snapshot() {
        let cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([1_u8; 32]),
        )
        .unwrap();
        let store = InMemoryStore::new_with_oauth(Arc::new(StaticOAuthTokenClient), Some(cipher));

        let account = store
            .import_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-a".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-1".to_string(),
                chatgpt_account_id: None,
                mode: None,
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: None,
                source_type: None,
            })
            .await
            .unwrap();

        let snapshot = store.snapshot().await.unwrap();
        let snapshot_account = snapshot
            .accounts
            .into_iter()
            .find(|item| item.id == account.id)
            .expect("snapshot account");

        assert_eq!(snapshot_account.bearer_token, "access-1");
        assert_eq!(
            snapshot_account.chatgpt_account_id.as_deref(),
            Some("acct_demo")
        );
    }

    #[tokio::test]
    async fn in_memory_oauth_status_exposes_email() {
        let cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([9_u8; 32]),
        )
        .unwrap();
        let store = InMemoryStore::new_with_oauth(Arc::new(StaticOAuthTokenClient), Some(cipher));

        let account = store
            .import_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-email".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-email".to_string(),
                chatgpt_account_id: None,
                mode: None,
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: None,
                source_type: Some("codex".to_string()),
            })
            .await
            .unwrap();

        let status = store.oauth_account_status(account.id).await.unwrap();
        assert_eq!(status.email.as_deref(), Some("demo@example.com"));
        assert_eq!(status.oauth_subject.as_deref(), Some("auth0|demo"));
        assert_eq!(
            status.oauth_identity_provider.as_deref(),
            Some("google-oauth2")
        );
        assert_eq!(status.email_verified, Some(true));
        assert_eq!(status.chatgpt_user_id.as_deref(), Some("user_demo"));
        assert_eq!(
            status.chatgpt_account_user_id.as_deref(),
            Some("acct_user_demo")
        );
        assert_eq!(
            status.chatgpt_compute_residency.as_deref(),
            Some("us")
        );
        assert_eq!(status.organizations.as_ref().map(Vec::len), Some(1));
        assert_eq!(status.groups.as_ref().map(Vec::len), Some(1));
    }

    #[tokio::test]
    async fn in_memory_oauth_status_exposes_team_workspace_name() {
        let cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([10_u8; 32]),
        )
        .unwrap();
        let store = InMemoryStore::new_with_oauth(
            Arc::new(SharedAccountIdOAuthTokenClient),
            Some(cipher),
        );

        let account = store
            .import_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-team-workspace".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-shared-workspace-a".to_string(),
                chatgpt_account_id: None,
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: None,
                source_type: Some("codex".to_string()),
            })
            .await
            .unwrap();

        let status = store.oauth_account_status(account.id).await.unwrap();
        assert_eq!(status.chatgpt_plan_type.as_deref(), Some("team"));
        assert_eq!(status.workspace_name.as_deref(), Some("OAI-03.09"));
    }

    #[tokio::test]
    async fn in_memory_oauth_status_backfills_team_workspace_name_from_probe() {
        let cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([11_u8; 32]),
        )
        .unwrap();
        let store = InMemoryStore::new_with_oauth(
            Arc::new(TeamWorkspaceProbeOAuthTokenClient),
            Some(cipher),
        );

        let account = store
            .import_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-team-probe".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-team-probe".to_string(),
                chatgpt_account_id: None,
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: None,
                source_type: Some("codex".to_string()),
            })
            .await
            .unwrap();

        let status = store.oauth_account_status(account.id).await.unwrap();
        assert_eq!(status.chatgpt_plan_type.as_deref(), Some("team"));
        assert_eq!(status.workspace_name.as_deref(), Some("OAI-03.09"));
    }

    #[tokio::test]
    async fn in_memory_oauth_import_infers_codex_mode_from_source_type() {
        let cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([2_u8; 32]),
        )
        .unwrap();
        let store = InMemoryStore::new_with_oauth(Arc::new(StaticOAuthTokenClient), Some(cipher));

        let account = store
            .import_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-codex".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-codex-1".to_string(),
                chatgpt_account_id: None,
                mode: None,
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: None,
                source_type: Some("codex".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(account.mode, UpstreamMode::CodexOauth);
    }

    #[tokio::test]
    async fn in_memory_oauth_upsert_dedupes_by_chatgpt_account_user_id() {
        let cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([3_u8; 32]),
        )
        .unwrap();
        let store = InMemoryStore::new_with_oauth(
            Arc::new(SharedAccountIdOAuthTokenClient),
            Some(cipher),
        );

        let first = store
            .upsert_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-shared-a".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-shared-workspace-a-1".to_string(),
                chatgpt_account_id: None,
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: None,
                source_type: Some("codex".to_string()),
            })
            .await
            .unwrap();
        let second = store
            .upsert_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-shared-b".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-shared-workspace-a-2".to_string(),
                chatgpt_account_id: None,
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: None,
                source_type: Some("codex".to_string()),
            })
            .await
            .unwrap();

        assert!(first.created);
        assert!(!second.created);
        assert_eq!(first.account.id, second.account.id);

        let snapshot = store.snapshot().await.unwrap();
        let shared_accounts = snapshot
            .accounts
            .into_iter()
            .filter(|account| account.chatgpt_account_id.as_deref() == Some("acct_shared"))
            .collect::<Vec<_>>();

        assert_eq!(shared_accounts.len(), 1);
    }

    #[tokio::test]
    async fn in_memory_oauth_upsert_keeps_distinct_accounts_with_shared_chatgpt_account_id_but_different_account_user_id(
    ) {
        let cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([4_u8; 32]),
        )
        .unwrap();
        let store = InMemoryStore::new_with_oauth(
            Arc::new(SharedAccountIdOAuthTokenClient),
            Some(cipher),
        );

        let first = store
            .upsert_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-shared-a".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-shared-workspace-a".to_string(),
                chatgpt_account_id: None,
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: None,
                source_type: Some("codex".to_string()),
            })
            .await
            .unwrap();
        let second = store
            .upsert_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-shared-b".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-shared-workspace-b".to_string(),
                chatgpt_account_id: None,
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: None,
                source_type: Some("codex".to_string()),
            })
            .await
            .unwrap();

        assert!(first.created);
        assert!(second.created);
        assert_ne!(first.account.id, second.account.id);

        let snapshot = store.snapshot().await.unwrap();
        let shared_accounts = snapshot
            .accounts
            .into_iter()
            .filter(|account| account.chatgpt_account_id.as_deref() == Some("acct_shared"))
            .collect::<Vec<_>>();

        assert_eq!(shared_accounts.len(), 2);
    }

    #[tokio::test]
    async fn in_memory_snapshot_compiles_exact_model_routes_from_profiles_and_support_matrix() {
        let store = InMemoryStore::default();

        let free_account = store
            .upsert_one_time_session_account(UpsertOneTimeSessionAccountRequest {
                label: "free-codex".to_string(),
                mode: UpstreamMode::CodexOauth,
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                access_token: "free-token".to_string(),
                chatgpt_account_id: Some("acct-free".to_string()),
                enabled: Some(true),
                priority: Some(100),
                token_expires_at: Some(Utc::now() + Duration::hours(4)),
                chatgpt_plan_type: Some("free".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .unwrap()
            .account;
        let paid_account = store
            .upsert_one_time_session_account(UpsertOneTimeSessionAccountRequest {
                label: "paid-codex".to_string(),
                mode: UpstreamMode::CodexOauth,
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                access_token: "paid-token".to_string(),
                chatgpt_account_id: Some("acct-paid".to_string()),
                enabled: Some(true),
                priority: Some(90),
                token_expires_at: Some(Utc::now() + Duration::hours(4)),
                chatgpt_plan_type: Some("plus".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .unwrap()
            .account;

        let free_profile = store
            .upsert_routing_profile(UpsertRoutingProfileRequest {
                id: None,
                name: "free-first".to_string(),
                description: None,
                enabled: true,
                priority: 100,
                selector: RoutingProfileSelector {
                    plan_types: vec!["free".to_string()],
                    ..Default::default()
                },
            })
            .await
            .unwrap();
        let paid_profile = store
            .upsert_routing_profile(UpsertRoutingProfileRequest {
                id: None,
                name: "paid-fallback".to_string(),
                description: None,
                enabled: true,
                priority: 90,
                selector: RoutingProfileSelector {
                    plan_types: vec!["plus".to_string(), "team".to_string()],
                    ..Default::default()
                },
            })
            .await
            .unwrap();

        store
            .upsert_model_routing_policy(UpsertModelRoutingPolicyRequest {
                id: None,
                name: "default".to_string(),
                family: "default".to_string(),
                exact_models: Vec::new(),
                model_prefixes: Vec::new(),
                fallback_profile_ids: vec![free_profile.id, paid_profile.id],
                enabled: true,
                priority: 100,
            })
            .await
            .unwrap();
        store
            .upsert_model_routing_policy(UpsertModelRoutingPolicyRequest {
                id: None,
                name: "gpt5-family".to_string(),
                family: "gpt-5".to_string(),
                exact_models: vec!["gpt-5.4".to_string()],
                model_prefixes: vec!["gpt-5".to_string()],
                fallback_profile_ids: vec![free_profile.id, paid_profile.id],
                enabled: true,
                priority: 80,
            })
            .await
            .unwrap();

        store
            .record_account_model_support(
                free_account.id,
                vec!["gpt-5.2-codex".to_string()],
                Utc::now(),
            )
            .await
            .unwrap();
        store
            .record_account_model_support(
                paid_account.id,
                vec!["gpt-5.4".to_string(), "gpt-5.2-codex".to_string()],
                Utc::now(),
            )
            .await
            .unwrap();

        let snapshot = store.snapshot().await.unwrap();
        let compiled = snapshot
            .compiled_routing_plan
            .expect("compiled routing plan should exist");

        assert_eq!(compiled.default_route.len(), 2);
        assert_eq!(compiled.default_route[0].account_ids, vec![free_account.id]);
        assert_eq!(compiled.default_route[1].account_ids, vec![paid_account.id]);

        let gpt54 = compiled
            .policies
            .iter()
            .find(|policy| policy.exact_models == vec!["gpt-5.4".to_string()])
            .expect("compiled exact route for gpt-5.4");
        assert_eq!(gpt54.fallback_segments.len(), 1);
        assert_eq!(gpt54.fallback_segments[0].account_ids, vec![paid_account.id]);
    }
}
