impl PostgresStore {
    pub async fn insert_tenant(&self, req: CreateTenantRequest) -> Result<Tenant> {
        let id = Uuid::new_v4();
        let created_at = Utc::now();

        let row = sqlx::query(
            r#"
            INSERT INTO tenants (id, name, created_at)
            VALUES ($1, $2, $3)
            RETURNING id, name, created_at
            "#,
        )
        .bind(id)
        .bind(req.name)
        .bind(created_at)
        .fetch_one(&self.pool)
        .await
        .context("failed to insert tenant")?;

        Ok(Tenant {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            created_at: row.try_get::<DateTime<Utc>, _>("created_at")?,
        })
    }

    pub async fn fetch_tenants(&self) -> Result<Vec<Tenant>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, created_at
            FROM tenants
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list tenants")?;

        let mut tenants = Vec::with_capacity(rows.len());
        for row in rows {
            tenants.push(Tenant {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                created_at: row.try_get::<DateTime<Utc>, _>("created_at")?,
            });
        }

        Ok(tenants)
    }

    async fn insert_api_key(&self, req: CreateApiKeyRequest) -> Result<CreateApiKeyResponse> {
        let plaintext_key = format!("cp_{}", Uuid::new_v4().simple());
        let token_hash = hash_api_key_token(&plaintext_key);
        let key_hash = token_hash.clone();
        let key_prefix: String = plaintext_key.chars().take(12).collect();
        let id = Uuid::new_v4();
        let created_at = Utc::now();
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start api key transaction")?;

        let row = sqlx::query(
            r#"
            INSERT INTO api_keys (id, tenant_id, name, key_prefix, key_hash, enabled, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, tenant_id, name, key_prefix, key_hash, enabled, created_at
            "#,
        )
        .bind(id)
        .bind(req.tenant_id)
        .bind(req.name)
        .bind(key_prefix)
        .bind(key_hash)
        .bind(true)
        .bind(created_at)
        .fetch_one(tx.as_mut())
        .await
        .context("failed to insert api key")?;

        sqlx::query(
            r#"
            INSERT INTO api_key_tokens (token, api_key_id)
            VALUES ($1, $2)
            "#,
        )
        .bind(token_hash)
        .bind(id)
        .execute(tx.as_mut())
        .await
        .context("failed to insert api key token")?;
        tx.commit()
            .await
            .context("failed to commit api key transaction")?;

        Ok(CreateApiKeyResponse {
            record: ApiKey {
                id: row.try_get("id")?,
                tenant_id: row.try_get("tenant_id")?,
                name: row.try_get("name")?,
                key_prefix: row.try_get("key_prefix")?,
                key_hash: row.try_get("key_hash")?,
                enabled: row.try_get("enabled")?,
                created_at: row.try_get::<DateTime<Utc>, _>("created_at")?,
            },
            plaintext_key,
        })
    }

    async fn fetch_api_keys(&self) -> Result<Vec<ApiKey>> {
        let rows = sqlx::query(
            r#"
            SELECT id, tenant_id, name, key_prefix, key_hash, enabled, created_at
            FROM api_keys
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list api keys")?;

        let mut api_keys = Vec::with_capacity(rows.len());
        for row in rows {
            api_keys.push(ApiKey {
                id: row.try_get("id")?,
                tenant_id: row.try_get("tenant_id")?,
                name: row.try_get("name")?,
                key_prefix: row.try_get("key_prefix")?,
                key_hash: row.try_get("key_hash")?,
                enabled: row.try_get("enabled")?,
                created_at: row.try_get::<DateTime<Utc>, _>("created_at")?,
            });
        }

        Ok(api_keys)
    }

    async fn set_api_key_enabled_inner(&self, api_key_id: Uuid, enabled: bool) -> Result<ApiKey> {
        let row = sqlx::query(
            r#"
            UPDATE api_keys
            SET enabled = $2
            WHERE id = $1
            RETURNING id, tenant_id, name, key_prefix, key_hash, enabled, created_at
            "#,
        )
        .bind(api_key_id)
        .bind(enabled)
        .fetch_optional(&self.pool)
        .await
        .context("failed to update api key enabled flag")?
        .ok_or_else(|| anyhow!("api key not found"))?;

        Ok(ApiKey {
            id: row.try_get("id")?,
            tenant_id: row.try_get("tenant_id")?,
            name: row.try_get("name")?,
            key_prefix: row.try_get("key_prefix")?,
            key_hash: row.try_get("key_hash")?,
            enabled: row.try_get("enabled")?,
            created_at: row.try_get::<DateTime<Utc>, _>("created_at")?,
        })
    }

    async fn fetch_validated_principal_by_token(
        &self,
        token: &str,
    ) -> Result<Option<ValidatedPrincipal>> {
        let token_hash = hash_api_key_token(token);
        let token_hash_candidates = crate::security::api_key_token_hash_candidates(token);
        let row = sqlx::query(
            r#"
            SELECT
                k.tenant_id,
                k.id AS api_key_id,
                k.enabled,
                k.ip_allowlist,
                g.id AS group_id,
                g.name AS group_name,
                g.enabled AS group_enabled,
                g.allow_all_models AS group_allow_all_models,
                g.deleted_at AS group_deleted_at,
                ten.status AS tenant_status,
                ten.expires_at AS tenant_expires_at,
                c.balance_microcredits,
                tok.token AS stored_token
            FROM api_key_tokens tok
            INNER JOIN api_keys k ON k.id = tok.api_key_id
            INNER JOIN api_key_groups g ON g.id = k.group_id
            INNER JOIN tenants ten ON ten.id = k.tenant_id
            LEFT JOIN tenant_credit_accounts c ON c.tenant_id = k.tenant_id
            WHERE tok.token = ANY($1)
            "#,
        )
        .bind(&token_hash_candidates)
        .fetch_optional(&self.pool)
        .await
        .context("failed to validate api key token")?;

        let Some(row) = row else {
            return Ok(None);
        };
        let stored_token: String = row.try_get("stored_token")?;
        let api_key_id: Uuid = row.try_get("api_key_id")?;
        if stored_token != token_hash {
            if let Err(err) = self
                .upgrade_api_key_token_hash(api_key_id, &stored_token, token, &token_hash)
                .await
            {
                tracing::warn!(
                    api_key_id = %api_key_id,
                    error = %err,
                    "failed to upgrade api key token hash to active hmac key"
                );
            }
        }

        let ip_allowlist = row
            .try_get::<serde_json::Value, _>("ip_allowlist")?
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let api_key_group_id: Uuid = row.try_get("group_id")?;
        let api_key_group_name: String = row.try_get("group_name")?;
        let api_key_group_invalid = row
            .try_get::<Option<DateTime<Utc>>, _>("group_deleted_at")?
            .is_some()
            || !row.try_get::<bool, _>("group_enabled")?;
        let group_allow_all_models: bool = row.try_get("group_allow_all_models")?;
        let key_model_allowlist = if api_key_group_invalid {
            Vec::new()
        } else if group_allow_all_models {
            let rows = sqlx::query(
                r#"
                WITH catalog_models AS (
                    SELECT model_id FROM openai_models_catalog
                    UNION
                    SELECT model_id FROM admin_model_entities
                ),
                denied_models AS (
                    SELECT model_id
                    FROM api_key_group_model_policies
                    WHERE group_id = $1 AND enabled = false
                )
                SELECT model_id
                FROM catalog_models
                WHERE model_id NOT IN (SELECT model_id FROM denied_models)
                ORDER BY model_id ASC
                "#,
            )
            .bind(api_key_group_id)
            .fetch_all(&self.pool)
            .await
            .context("failed to query allow-all group model allowlist")?;
            rows.into_iter()
                .map(|item| item.try_get::<String, _>("model_id"))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            let rows = sqlx::query(
                r#"
                SELECT model_id
                FROM api_key_group_model_policies
                WHERE group_id = $1 AND enabled = true
                ORDER BY model_id ASC
                "#,
            )
            .bind(api_key_group_id)
            .fetch_all(&self.pool)
            .await
            .context("failed to query api key group model allowlist")?;
            rows.into_iter()
                .map(|item| item.try_get::<String, _>("model_id"))
                .collect::<Result<Vec<_>, _>>()?
        };
        Ok(Some(ValidatedPrincipal {
            tenant_id: row.try_get("tenant_id")?,
            api_key_id,
            api_key_group_id,
            api_key_group_name,
            api_key_group_invalid,
            enabled: row.try_get("enabled")?,
            key_ip_allowlist: ip_allowlist,
            key_model_allowlist,
            tenant_status: row.try_get::<Option<String>, _>("tenant_status")?,
            tenant_expires_at: row.try_get::<Option<DateTime<Utc>>, _>("tenant_expires_at")?,
            balance_microcredits: row.try_get::<Option<i64>, _>("balance_microcredits")?,
        }))
    }

    async fn upgrade_api_key_token_hash(
        &self,
        api_key_id: Uuid,
        stored_token: &str,
        plaintext_token: &str,
        token_hash: &str,
    ) -> Result<bool> {
        if stored_token == token_hash {
            return Ok(false);
        }

        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start api key hash upgrade transaction")?;
        let token_update = sqlx::query(
            r#"
            UPDATE api_key_tokens
            SET token = $1
            WHERE api_key_id = $2 AND token = $3
            "#,
        )
        .bind(token_hash)
        .bind(api_key_id)
        .bind(stored_token)
        .execute(tx.as_mut())
        .await
        .context("failed to upgrade api_key_tokens token hash")?;

        let mut key_hash_candidates = crate::security::api_key_token_hash_candidates(plaintext_token);
        key_hash_candidates.push(format!("plaintext:{plaintext_token}"));
        key_hash_candidates.push(stored_token.to_string());
        key_hash_candidates.sort_unstable();
        key_hash_candidates.dedup();

        let key_hash_update = sqlx::query(
            r#"
            UPDATE api_keys
            SET key_hash = $1
            WHERE id = $2 AND key_hash = ANY($3) AND key_hash <> $1
            "#,
        )
        .bind(token_hash)
        .bind(api_key_id)
        .bind(&key_hash_candidates)
        .execute(tx.as_mut())
        .await
        .context("failed to upgrade api_keys key_hash")?;

        tx.commit()
            .await
            .context("failed to commit api key hash upgrade transaction")?;

        Ok(token_update.rows_affected() > 0 || key_hash_update.rows_affected() > 0)
    }

    async fn insert_upstream_account(
        &self,
        req: CreateUpstreamAccountRequest,
    ) -> Result<UpstreamAccount> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start upstream account transaction")?;
        let id = Uuid::new_v4();
        let enabled = req.enabled.unwrap_or(true);
        let priority = req.priority.unwrap_or(100);
        let created_at = Utc::now();
        let mode = upstream_mode_to_db(&req.mode);
        let base_url = normalize_upstream_account_base_url(&req.mode, &req.base_url);
        let auth_provider = upstream_auth_provider_to_db(&UpstreamAuthProvider::LegacyBearer);

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
                priority,
                created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, label, mode, base_url, bearer_token, chatgpt_account_id, enabled, priority, created_at
            "#,
        )
        .bind(id)
        .bind(req.label)
        .bind(mode)
        .bind(base_url)
        .bind(req.bearer_token)
        .bind(req.chatgpt_account_id)
        .bind(auth_provider)
        .bind(enabled)
        .bind(priority)
        .bind(created_at)
        .fetch_one(tx.as_mut())
        .await
        .context("failed to insert upstream account")?;

        self.bump_revision_tx(&mut tx).await?;
        self.append_data_plane_outbox_event_tx(
            &mut tx,
            DataPlaneSnapshotEventType::AccountUpsert,
            id,
        )
        .await?;
        tx.commit()
            .await
            .context("failed to commit upstream account transaction")?;

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

    async fn fetch_upstream_accounts(&self) -> Result<Vec<UpstreamAccount>> {
        let rows = sqlx::query(
            r#"
            SELECT id, label, mode, base_url, bearer_token, chatgpt_account_id, enabled, priority, created_at
            FROM upstream_accounts
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list upstream accounts")?;

        let mut accounts = Vec::with_capacity(rows.len());
        for row in rows {
            let mode = parse_upstream_mode(row.try_get::<String, _>("mode")?.as_str())?;
            accounts.push(UpstreamAccount {
                id: row.try_get("id")?,
                label: row.try_get("label")?,
                mode,
                base_url: row.try_get("base_url")?,
                bearer_token: row.try_get("bearer_token")?,
                chatgpt_account_id: row.try_get("chatgpt_account_id")?,
                enabled: row.try_get("enabled")?,
                priority: row.try_get("priority")?,
                created_at: row.try_get::<DateTime<Utc>, _>("created_at")?,
            });
        }

        Ok(accounts)
    }

    async fn set_upstream_account_enabled_inner(
        &self,
        account_id: Uuid,
        enabled: bool,
    ) -> Result<UpstreamAccount> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start upstream account update transaction")?;

        let row = sqlx::query(
            r#"
            UPDATE upstream_accounts
            SET enabled = $2
            WHERE id = $1
            RETURNING id, label, mode, base_url, bearer_token, chatgpt_account_id, enabled, priority, created_at
            "#,
        )
        .bind(account_id)
        .bind(enabled)
        .fetch_optional(tx.as_mut())
        .await
        .context("failed to update upstream account enabled flag")?
        .ok_or_else(|| anyhow!("upstream account not found"))?;

        self.bump_revision_tx(&mut tx).await?;
        self.append_data_plane_outbox_event_tx(
            &mut tx,
            DataPlaneSnapshotEventType::AccountUpsert,
            account_id,
        )
        .await?;
        tx.commit()
            .await
            .context("failed to commit upstream account update transaction")?;

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

    async fn delete_upstream_account_inner(&self, account_id: Uuid) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start upstream account delete transaction")?;

        let deleted = sqlx::query(
            r#"
            DELETE FROM upstream_accounts
            WHERE id = $1
            "#,
        )
        .bind(account_id)
        .execute(tx.as_mut())
        .await
        .context("failed to delete upstream account")?
        .rows_affected();

        if deleted == 0 {
            return Err(anyhow!("upstream account not found"));
        }

        self.bump_revision_tx(&mut tx).await?;
        self.append_data_plane_outbox_event_tx(
            &mut tx,
            DataPlaneSnapshotEventType::AccountDelete,
            account_id,
        )
        .await?;
        tx.commit()
            .await
            .context("failed to commit upstream account delete transaction")?;

        Ok(())
    }

    fn require_credential_cipher(&self) -> Result<&CredentialCipher> {
        self.credential_cipher.as_ref().ok_or_else(|| {
            anyhow!("oauth credential encryption key is missing: set CREDENTIALS_ENCRYPTION_KEY")
        })
    }
}
