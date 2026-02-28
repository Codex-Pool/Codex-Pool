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
        self.oauth_account_status_inner(account_id)
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

    async fn claim_due_probe_accounts(
        &self,
        limit: usize,
        seen_ok_suppress_sec: i64,
        lock_ttl_sec: i64,
        _claimed_by: &str,
    ) -> Result<Vec<ClaimedProbeAccount>> {
        Ok(self.claim_due_probe_accounts_inner(
            limit,
            seen_ok_suppress_sec,
            lock_ttl_sec,
        ))
    }

    async fn release_upstream_op_lock(&self, account_id: Uuid, op_type: &str) -> Result<()> {
        self.release_upstream_op_lock_inner(account_id, op_type);
        Ok(())
    }

    async fn record_upstream_probe(&self, account_id: Uuid, write: UpstreamProbeWrite) -> Result<()> {
        self.record_upstream_probe_inner(account_id, write);
        Ok(())
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
    use super::{ControlPlaneStore, InMemoryStore};
    use crate::crypto::CredentialCipher;
    use crate::oauth::{OAuthTokenClient, OAuthTokenInfo};
    use async_trait::async_trait;
    use base64::Engine;
    use chrono::{Duration, Utc};
    use codex_pool_core::api::{
        CreateApiKeyRequest, CreateTenantRequest, ImportOAuthRefreshTokenRequest,
    };
    use codex_pool_core::model::UpstreamMode;
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
                chatgpt_account_id: Some("acct_demo".to_string()),
                chatgpt_plan_type: Some("pro".to_string()),
            })
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
}
