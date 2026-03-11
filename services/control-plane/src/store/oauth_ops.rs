impl InMemoryStore {
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

                // Do not collapse distinct workspace logins that reuse the same ChatGPT account id.
                credentials
                    .get(account_id)
                    .filter(|credential| credential.refresh_token_sha256 == normalized_refresh_hash)
                    .map(|_| *account_id)
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
            account.chatgpt_account_id = normalized_chatgpt_account_id;
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
            return Ok(OAuthUpsertResult {
                account,
                created: false,
            });
        }

        let account = self.import_oauth_refresh_token_inner(req).await?;
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

    fn oauth_account_status_inner(&self, account_id: Uuid) -> Result<OAuthAccountStatusResponse> {
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

        Ok(self.oauth_status_from(
            &account,
            provider,
            oauth_credential.as_ref(),
            session_profile.as_ref(),
        ))
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
