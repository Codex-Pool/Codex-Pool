const AI_ROUTING_SETTINGS_SINGLETON_ROW: bool = true;
const PRIMARY_RATE_LIMIT_WINDOW_MINUTES: i64 = 300;
const SECONDARY_RATE_LIMIT_WINDOW_MINUTES: i64 = 10_080;
const ROUTING_PLAN_REASON_PROFILE_UPSERT: &str = "routing_profile_upsert";
const ROUTING_PLAN_REASON_PROFILE_DELETE: &str = "routing_profile_delete";
const ROUTING_PLAN_REASON_MODEL_POLICY_UPSERT: &str = "model_routing_policy_upsert";
const ROUTING_PLAN_REASON_MODEL_POLICY_DELETE: &str = "model_routing_policy_delete";
const ROUTING_PLAN_REASON_MODEL_SUPPORT_REFRESH: &str = "account_model_support_refresh";

#[derive(Debug, Clone)]
struct SnapshotRoutingRecord {
    account: UpstreamAccount,
    traits: AccountRoutingTraits,
}

#[derive(Debug, Clone)]
struct RoutingPlanVersionMetadata {
    id: Uuid,
    reason: Option<String>,
    published_at: DateTime<Utc>,
}

impl PostgresStore {
    async fn list_routing_profiles_inner(&self) -> Result<Vec<RoutingProfile>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id,
                name,
                description,
                enabled,
                priority,
                selector_json::text AS selector_json_text,
                created_at,
                updated_at
            FROM routing_profiles
            ORDER BY priority DESC, created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list routing profiles")?;

        rows.iter().map(parse_routing_profile_row).collect()
    }

    async fn upsert_routing_profile_inner(
        &self,
        req: UpsertRoutingProfileRequest,
    ) -> Result<RoutingProfile> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start routing profile transaction")?;
        let now = Utc::now();
        let profile_id = req.id.unwrap_or_else(Uuid::new_v4);
        let selector_json = serde_json::to_string(&req.selector)
            .context("failed to encode routing profile selector")?;

        let row = sqlx::query(
            r#"
            INSERT INTO routing_profiles (
                id,
                name,
                description,
                enabled,
                priority,
                selector_json,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7, $7)
            ON CONFLICT (id) DO UPDATE
            SET
                name = EXCLUDED.name,
                description = EXCLUDED.description,
                enabled = EXCLUDED.enabled,
                priority = EXCLUDED.priority,
                selector_json = EXCLUDED.selector_json,
                updated_at = EXCLUDED.updated_at
            RETURNING
                id,
                name,
                description,
                enabled,
                priority,
                selector_json::text AS selector_json_text,
                created_at,
                updated_at
            "#,
        )
        .bind(profile_id)
        .bind(req.name)
        .bind(req.description)
        .bind(req.enabled)
        .bind(req.priority)
        .bind(selector_json)
        .bind(now)
        .fetch_one(tx.as_mut())
        .await
        .context("failed to upsert routing profile")?;

        self.bump_revision_tx(&mut tx).await?;
        self.append_data_plane_outbox_event_tx(
            &mut tx,
            DataPlaneSnapshotEventType::RoutingPlanRefresh,
            Uuid::nil(),
        )
        .await?;
        tx.commit()
            .await
            .context("failed to commit routing profile transaction")?;

        let _ = self
            .publish_current_routing_plan_version_inner(Some(ROUTING_PLAN_REASON_PROFILE_UPSERT))
            .await?;

        parse_routing_profile_row(&row)
    }

    async fn delete_routing_profile_inner(&self, profile_id: Uuid) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start delete routing profile transaction")?;

        let deleted = sqlx::query(
            r#"
            DELETE FROM routing_profiles
            WHERE id = $1
            "#,
        )
        .bind(profile_id)
        .execute(tx.as_mut())
        .await
        .context("failed to delete routing profile")?
        .rows_affected();

        if deleted == 0 {
            tx.rollback()
                .await
                .context("failed to rollback missing routing profile delete")?;
            return Err(anyhow!("routing profile not found"));
        }

        self.bump_revision_tx(&mut tx).await?;
        self.append_data_plane_outbox_event_tx(
            &mut tx,
            DataPlaneSnapshotEventType::RoutingPlanRefresh,
            Uuid::nil(),
        )
        .await?;
        tx.commit()
            .await
            .context("failed to commit delete routing profile transaction")?;

        let _ = self
            .publish_current_routing_plan_version_inner(Some(ROUTING_PLAN_REASON_PROFILE_DELETE))
            .await?;
        Ok(())
    }

    async fn list_model_routing_policies_inner(&self) -> Result<Vec<ModelRoutingPolicy>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id,
                name,
                family,
                exact_models_json::text AS exact_models_json_text,
                model_prefixes_json::text AS model_prefixes_json_text,
                fallback_profile_ids_json::text AS fallback_profile_ids_json_text,
                enabled,
                priority,
                created_at,
                updated_at
            FROM model_routing_policies
            ORDER BY priority DESC, created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list model routing policies")?;

        rows.iter().map(parse_model_routing_policy_row).collect()
    }

    async fn upsert_model_routing_policy_inner(
        &self,
        req: UpsertModelRoutingPolicyRequest,
    ) -> Result<ModelRoutingPolicy> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start model routing policy transaction")?;
        let now = Utc::now();
        let policy_id = req.id.unwrap_or_else(Uuid::new_v4);
        let exact_models_json = serde_json::to_string(&normalize_string_list(req.exact_models))
            .context("failed to encode exact model list")?;
        let model_prefixes_json = serde_json::to_string(&normalize_string_list(req.model_prefixes))
            .context("failed to encode model prefix list")?;
        let fallback_profile_ids_json = serde_json::to_string(&req.fallback_profile_ids)
            .context("failed to encode fallback profile ids")?;

        let row = sqlx::query(
            r#"
            INSERT INTO model_routing_policies (
                id,
                name,
                family,
                exact_models_json,
                model_prefixes_json,
                fallback_profile_ids_json,
                enabled,
                priority,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4::jsonb, $5::jsonb, $6::jsonb, $7, $8, $9, $9)
            ON CONFLICT (id) DO UPDATE
            SET
                name = EXCLUDED.name,
                family = EXCLUDED.family,
                exact_models_json = EXCLUDED.exact_models_json,
                model_prefixes_json = EXCLUDED.model_prefixes_json,
                fallback_profile_ids_json = EXCLUDED.fallback_profile_ids_json,
                enabled = EXCLUDED.enabled,
                priority = EXCLUDED.priority,
                updated_at = EXCLUDED.updated_at
            RETURNING
                id,
                name,
                family,
                exact_models_json::text AS exact_models_json_text,
                model_prefixes_json::text AS model_prefixes_json_text,
                fallback_profile_ids_json::text AS fallback_profile_ids_json_text,
                enabled,
                priority,
                created_at,
                updated_at
            "#,
        )
        .bind(policy_id)
        .bind(req.name)
        .bind(req.family)
        .bind(exact_models_json)
        .bind(model_prefixes_json)
        .bind(fallback_profile_ids_json)
        .bind(req.enabled)
        .bind(req.priority)
        .bind(now)
        .fetch_one(tx.as_mut())
        .await
        .context("failed to upsert model routing policy")?;

        self.bump_revision_tx(&mut tx).await?;
        self.append_data_plane_outbox_event_tx(
            &mut tx,
            DataPlaneSnapshotEventType::RoutingPlanRefresh,
            Uuid::nil(),
        )
        .await?;
        tx.commit()
            .await
            .context("failed to commit model routing policy transaction")?;

        let _ = self
            .publish_current_routing_plan_version_inner(Some(
                ROUTING_PLAN_REASON_MODEL_POLICY_UPSERT,
            ))
            .await?;

        parse_model_routing_policy_row(&row)
    }

    async fn delete_model_routing_policy_inner(&self, policy_id: Uuid) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start delete model routing policy transaction")?;

        let deleted = sqlx::query(
            r#"
            DELETE FROM model_routing_policies
            WHERE id = $1
            "#,
        )
        .bind(policy_id)
        .execute(tx.as_mut())
        .await
        .context("failed to delete model routing policy")?
        .rows_affected();

        if deleted == 0 {
            tx.rollback()
                .await
                .context("failed to rollback missing model routing policy delete")?;
            return Err(anyhow!("model routing policy not found"));
        }

        self.bump_revision_tx(&mut tx).await?;
        self.append_data_plane_outbox_event_tx(
            &mut tx,
            DataPlaneSnapshotEventType::RoutingPlanRefresh,
            Uuid::nil(),
        )
        .await?;
        tx.commit()
            .await
            .context("failed to commit delete model routing policy transaction")?;

        let _ = self
            .publish_current_routing_plan_version_inner(Some(
                ROUTING_PLAN_REASON_MODEL_POLICY_DELETE,
            ))
            .await?;
        Ok(())
    }

    async fn load_ai_routing_settings_inner(&self) -> Result<AiRoutingSettings> {
        let row = sqlx::query(
            r#"
            SELECT
                enabled,
                auto_publish,
                planner_model_chain_json::text AS planner_model_chain_json_text,
                trigger_mode,
                kill_switch,
                updated_at
            FROM ai_routing_settings
            WHERE singleton = $1
            "#,
        )
        .bind(AI_ROUTING_SETTINGS_SINGLETON_ROW)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load ai routing settings")?;

        let Some(row) = row else {
            return Ok(default_ai_routing_settings(Utc::now()));
        };

        Ok(AiRoutingSettings {
            enabled: row.try_get("enabled")?,
            auto_publish: row.try_get("auto_publish")?,
            planner_model_chain: parse_json_string_array(
                row.try_get::<Option<String>, _>("planner_model_chain_json_text")?,
            ),
            trigger_mode: parse_ai_routing_trigger_mode(
                row.try_get::<String, _>("trigger_mode")?.as_str(),
            )?,
            kill_switch: row.try_get("kill_switch")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    async fn update_ai_routing_settings_inner(
        &self,
        req: UpdateAiRoutingSettingsRequest,
    ) -> Result<AiRoutingSettings> {
        let updated_at = Utc::now();
        let planner_model_chain_json = serde_json::to_string(&normalize_string_list(
            req.planner_model_chain.clone(),
        ))
        .context("failed to encode ai routing planner model chain")?;
        sqlx::query(
            r#"
            INSERT INTO ai_routing_settings (
                singleton,
                enabled,
                auto_publish,
                planner_model_chain_json,
                trigger_mode,
                kill_switch,
                updated_at
            )
            VALUES ($1, $2, $3, $4::jsonb, $5, $6, $7)
            ON CONFLICT (singleton) DO UPDATE
            SET
                enabled = EXCLUDED.enabled,
                auto_publish = EXCLUDED.auto_publish,
                planner_model_chain_json = EXCLUDED.planner_model_chain_json,
                trigger_mode = EXCLUDED.trigger_mode,
                kill_switch = EXCLUDED.kill_switch,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(AI_ROUTING_SETTINGS_SINGLETON_ROW)
        .bind(req.enabled)
        .bind(req.auto_publish)
        .bind(planner_model_chain_json)
        .bind(ai_routing_trigger_mode_to_db(&req.trigger_mode))
        .bind(req.kill_switch)
        .bind(updated_at)
        .execute(&self.pool)
        .await
        .context("failed to update ai routing settings")?;

        Ok(AiRoutingSettings {
            enabled: req.enabled,
            auto_publish: req.auto_publish,
            planner_model_chain: normalize_string_list(req.planner_model_chain),
            trigger_mode: req.trigger_mode,
            kill_switch: req.kill_switch,
            updated_at,
        })
    }

    async fn list_routing_plan_versions_inner(&self) -> Result<Vec<RoutingPlanVersion>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id,
                reason,
                published_at,
                compiled_plan_json::text AS compiled_plan_json_text
            FROM routing_plan_versions
            ORDER BY published_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list routing plan versions")?;

        rows.iter().map(parse_routing_plan_version_row).collect()
    }

    async fn record_account_model_support_inner(
        &self,
        account_id: Uuid,
        supported_models: Vec<String>,
        checked_at: DateTime<Utc>,
    ) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start account model support transaction")?;
        let now = Utc::now();
        let supported_models_json = serde_json::to_string(&normalize_string_list(supported_models))
            .context("failed to encode supported model list")?;

        sqlx::query(
            r#"
            INSERT INTO upstream_account_model_support (
                account_id,
                supported_models_json,
                checked_at,
                updated_at
            )
            VALUES ($1, $2::jsonb, $3, $4)
            ON CONFLICT (account_id) DO UPDATE
            SET
                supported_models_json = EXCLUDED.supported_models_json,
                checked_at = EXCLUDED.checked_at,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(account_id)
        .bind(supported_models_json)
        .bind(checked_at)
        .bind(now)
        .execute(tx.as_mut())
        .await
        .context("failed to upsert account model support")?;

        self.bump_revision_tx(&mut tx).await?;
        self.append_data_plane_outbox_event_tx(
            &mut tx,
            DataPlaneSnapshotEventType::RoutingPlanRefresh,
            Uuid::nil(),
        )
        .await?;
        tx.commit()
            .await
            .context("failed to commit account model support transaction")?;

        let _ = self
            .publish_current_routing_plan_version_inner(Some(
                ROUTING_PLAN_REASON_MODEL_SUPPORT_REFRESH,
            ))
            .await?;
        Ok(())
    }

    async fn load_snapshot_accounts_inner(&self) -> Result<Vec<UpstreamAccount>> {
        Ok(self
            .load_active_snapshot_accounts_and_traits_inner(None)
            .await?
            .into_iter()
            .map(|record| record.account)
            .collect())
    }

    async fn load_account_routing_traits_inner(
        &self,
        accounts: &[UpstreamAccount],
    ) -> Result<HashMap<Uuid, AccountRoutingTraits>> {
        let account_ids = accounts.iter().map(|account| account.id).collect::<std::collections::HashSet<_>>();
        Ok(self
            .load_active_snapshot_accounts_and_traits_inner(None)
            .await?
            .into_iter()
            .filter(|record| account_ids.contains(&record.account.id))
            .map(|record| (record.account.id, record.traits))
            .collect())
    }

    async fn build_compiled_routing_plan_inner(
        &self,
        accounts: &[UpstreamAccount],
        account_traits: &HashMap<Uuid, AccountRoutingTraits>,
        trigger_reason: Option<String>,
    ) -> Result<Option<CompiledRoutingPlan>> {
        let profiles = self.list_routing_profiles_inner().await?;
        let policies = self.list_model_routing_policies_inner().await?;
        let metadata = self.load_latest_routing_plan_version_metadata_inner().await?;
        let version_id = metadata
            .as_ref()
            .map(|value| value.id)
            .unwrap_or_else(Uuid::new_v4);
        let published_at = metadata
            .as_ref()
            .map(|value| value.published_at)
            .unwrap_or_else(Utc::now);
        let compiled_reason = trigger_reason.or_else(|| metadata.and_then(|value| value.reason));
        Ok(compile_routing_plan_from_state(
            accounts,
            account_traits,
            &profiles,
            &policies,
            version_id,
            published_at,
            compiled_reason,
        ))
    }

    async fn publish_current_routing_plan_version_inner(
        &self,
        reason: Option<&str>,
    ) -> Result<Option<RoutingPlanVersion>> {
        let snapshot_records = self.load_active_snapshot_accounts_and_traits_inner(None).await?;
        let accounts = snapshot_records
            .iter()
            .map(|record| record.account.clone())
            .collect::<Vec<_>>();
        let account_traits = snapshot_records
            .iter()
            .map(|record| (record.account.id, record.traits.clone()))
            .collect::<HashMap<_, _>>();
        let profiles = self.list_routing_profiles_inner().await?;
        let policies = self.list_model_routing_policies_inner().await?;
        let version_id = Uuid::new_v4();
        let published_at = Utc::now();
        let compiled_plan = compile_routing_plan_from_state(
            &accounts,
            &account_traits,
            &profiles,
            &policies,
            version_id,
            published_at,
            reason.map(ToString::to_string),
        );
        let Some(compiled_plan) = compiled_plan else {
            return Ok(None);
        };
        let version = RoutingPlanVersion {
            id: version_id,
            reason: reason.map(ToString::to_string),
            published_at,
            compiled_plan: compiled_plan.clone(),
        };
        let payload = serde_json::to_string(&compiled_plan)
            .context("failed to encode compiled routing plan version")?;

        sqlx::query(
            r#"
            INSERT INTO routing_plan_versions (
                id,
                reason,
                published_at,
                compiled_plan_json
            )
            VALUES ($1, $2, $3, $4::jsonb)
            "#,
        )
        .bind(version_id)
        .bind(version.reason.clone())
        .bind(published_at)
        .bind(payload)
        .execute(&self.pool)
        .await
        .context("failed to insert routing plan version")?;

        Ok(Some(version))
    }

    async fn load_latest_routing_plan_version_metadata_inner(
        &self,
    ) -> Result<Option<RoutingPlanVersionMetadata>> {
        let row = sqlx::query(
            r#"
            SELECT id, reason, published_at
            FROM routing_plan_versions
            ORDER BY published_at DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await
        .context("failed to load latest routing plan version metadata")?;

        row.map(|row| {
            Ok(RoutingPlanVersionMetadata {
                id: row.try_get("id")?,
                reason: row.try_get("reason")?,
                published_at: row.try_get("published_at")?,
            })
        })
        .transpose()
    }

    async fn load_compiled_routing_plan_inner(&self) -> Result<Option<CompiledRoutingPlan>> {
        let snapshot_records = self.load_active_snapshot_accounts_and_traits_inner(None).await?;
        self.load_compiled_routing_plan_inner_from_records(&snapshot_records)
            .await
    }

    async fn load_compiled_routing_plan_inner_from_records(
        &self,
        snapshot_records: &[SnapshotRoutingRecord],
    ) -> Result<Option<CompiledRoutingPlan>> {
        let accounts = snapshot_records
            .iter()
            .map(|record| record.account.clone())
            .collect::<Vec<_>>();
        let account_traits = snapshot_records
            .iter()
            .map(|record| (record.account.id, record.traits.clone()))
            .collect::<HashMap<_, _>>();
        let profiles = self.list_routing_profiles_inner().await?;
        let policies = self.list_model_routing_policies_inner().await?;
        let metadata = self.load_latest_routing_plan_version_metadata_inner().await?;
        let version_id = metadata
            .as_ref()
            .map(|value| value.id)
            .unwrap_or_else(Uuid::new_v4);
        let published_at = metadata
            .as_ref()
            .map(|value| value.published_at)
            .unwrap_or_else(Utc::now);
        let trigger_reason = metadata
            .as_ref()
            .and_then(|value| value.reason.clone())
            .or_else(|| Some("runtime_snapshot".to_string()));

        Ok(compile_routing_plan_from_state(
            &accounts,
            &account_traits,
            &profiles,
            &policies,
            version_id,
            published_at,
            trigger_reason,
        ))
    }

    async fn load_active_snapshot_accounts_and_traits_inner(
        &self,
        only_account_id: Option<Uuid>,
    ) -> Result<Vec<SnapshotRoutingRecord>> {
        let rows = if let Some(account_id) = only_account_id {
            sqlx::query(
                r#"
                SELECT
                    a.id,
                    a.label,
                    a.mode,
                    a.base_url,
                    a.bearer_token,
                    a.chatgpt_account_id,
                    a.auth_provider,
                    a.enabled,
                    a.priority,
                    a.created_at,
                    c.access_token_enc,
                    c.token_expires_at,
                    c.last_refresh_status,
                    c.refresh_reused_detected,
                    c.last_refresh_error_code,
                    rl.rate_limits_json::text AS rate_limits_json_text,
                    rl.expires_at AS rate_limits_expires_at,
                    rl.last_error_code AS rate_limits_last_error_code,
                    rl.last_error_message AS rate_limits_last_error,
                    p.chatgpt_plan_type,
                    ms.supported_models_json::text AS supported_models_json_text
                FROM upstream_accounts a
                LEFT JOIN upstream_account_oauth_credentials c ON c.account_id = a.id
                LEFT JOIN upstream_account_rate_limit_snapshots rl ON rl.account_id = a.id
                LEFT JOIN upstream_account_session_profiles p ON p.account_id = a.id
                LEFT JOIN upstream_account_model_support ms ON ms.account_id = a.id
                WHERE a.pool_state = $1
                  AND a.id = $2
                ORDER BY a.created_at ASC
                "#,
            )
            .bind(POOL_STATE_ACTIVE)
            .bind(account_id)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query(
                r#"
                SELECT
                    a.id,
                    a.label,
                    a.mode,
                    a.base_url,
                    a.bearer_token,
                    a.chatgpt_account_id,
                    a.auth_provider,
                    a.enabled,
                    a.priority,
                    a.created_at,
                    c.access_token_enc,
                    c.token_expires_at,
                    c.last_refresh_status,
                    c.refresh_reused_detected,
                    c.last_refresh_error_code,
                    rl.rate_limits_json::text AS rate_limits_json_text,
                    rl.expires_at AS rate_limits_expires_at,
                    rl.last_error_code AS rate_limits_last_error_code,
                    rl.last_error_message AS rate_limits_last_error,
                    p.chatgpt_plan_type,
                    ms.supported_models_json::text AS supported_models_json_text
                FROM upstream_accounts a
                LEFT JOIN upstream_account_oauth_credentials c ON c.account_id = a.id
                LEFT JOIN upstream_account_rate_limit_snapshots rl ON rl.account_id = a.id
                LEFT JOIN upstream_account_session_profiles p ON p.account_id = a.id
                LEFT JOIN upstream_account_model_support ms ON ms.account_id = a.id
                WHERE a.pool_state = $1
                ORDER BY a.created_at ASC
                "#,
            )
            .bind(POOL_STATE_ACTIVE)
            .fetch_all(&self.pool)
            .await
        }
        .context("failed to load snapshot routing records")?;

        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            let now = Utc::now();
            let auth_provider =
                parse_upstream_auth_provider(row.try_get::<String, _>("auth_provider")?.as_str())?;
            let mode = parse_upstream_mode(row.try_get::<String, _>("mode")?.as_str())?;
            let token_expires_at = row.try_get::<Option<DateTime<Utc>>, _>("token_expires_at")?;
            let last_refresh_status = parse_oauth_refresh_status(
                row.try_get::<Option<String>, _>("last_refresh_status")?
                    .unwrap_or_else(|| "never".to_string())
                    .as_str(),
            )?;
            let refresh_reused_detected = row
                .try_get::<Option<bool>, _>("refresh_reused_detected")?
                .unwrap_or(false);
            let last_refresh_error_code = row.try_get::<Option<String>, _>("last_refresh_error_code")?;
            let rate_limits_expires_at =
                row.try_get::<Option<DateTime<Utc>>, _>("rate_limits_expires_at")?;
            let rate_limits_last_error_code =
                row.try_get::<Option<String>, _>("rate_limits_last_error_code")?;
            let rate_limits_last_error = row.try_get::<Option<String>, _>("rate_limits_last_error")?;
            let credential_kind = match (auth_provider.clone(), mode.clone()) {
                (UpstreamAuthProvider::OAuthRefreshToken, _) => {
                    Some(SessionCredentialKind::RefreshRotatable)
                }
                (UpstreamAuthProvider::LegacyBearer, UpstreamMode::ChatGptSession)
                | (UpstreamAuthProvider::LegacyBearer, UpstreamMode::CodexOauth) => {
                    Some(SessionCredentialKind::OneTimeAccessToken)
                }
                _ => None,
            };

            let mut enabled = oauth_effective_enabled(
                row.try_get::<bool, _>("enabled")?,
                &auth_provider,
                credential_kind.as_ref(),
                token_expires_at,
                &last_refresh_status,
                refresh_reused_detected,
                last_refresh_error_code.as_deref(),
                rate_limits_expires_at,
                rate_limits_last_error_code.as_deref(),
                rate_limits_last_error.as_deref(),
                now,
            );
            let mut bearer_token = row.try_get::<String, _>("bearer_token")?;

            if auth_provider == UpstreamAuthProvider::OAuthRefreshToken {
                let access_token_enc = row.try_get::<Option<String>, _>("access_token_enc")?;
                if let (Some(cipher), Some(access_token_enc)) =
                    (&self.credential_cipher, access_token_enc)
                {
                    match cipher.decrypt(&access_token_enc) {
                        Ok(access_token) => bearer_token = access_token,
                        Err(_) => {
                            enabled = false;
                            bearer_token.clear();
                        }
                    }
                } else {
                    enabled = false;
                    bearer_token.clear();
                }
            }

            let rate_limit_snapshots = parse_rate_limit_snapshots(
                row.try_get::<Option<String>, _>("rate_limits_json_text")?,
            );
            let (blocked_until, hard_block_reason) =
                derive_rate_limit_block(&rate_limit_snapshots, now);

            records.push(SnapshotRoutingRecord {
                account: UpstreamAccount {
                    id: row.try_get("id")?,
                    label: row.try_get("label")?,
                    mode,
                    base_url: row.try_get("base_url")?,
                    bearer_token,
                    chatgpt_account_id: row.try_get("chatgpt_account_id")?,
                    enabled,
                    priority: row.try_get("priority")?,
                    created_at: row.try_get::<DateTime<Utc>, _>("created_at")?,
                },
                traits: AccountRoutingTraits {
                    account_id: row.try_get("id")?,
                    plan_type: row.try_get("chatgpt_plan_type")?,
                    auth_provider: Some(auth_provider),
                    supported_models: parse_json_string_array(
                        row.try_get::<Option<String>, _>("supported_models_json_text")?,
                    ),
                    blocked_until,
                    hard_block_reason,
                },
            });
        }

        Ok(records)
    }
}

fn default_ai_routing_settings(updated_at: DateTime<Utc>) -> AiRoutingSettings {
    AiRoutingSettings {
        enabled: true,
        auto_publish: true,
        planner_model_chain: Vec::new(),
        trigger_mode: AiRoutingTriggerMode::Hybrid,
        kill_switch: false,
        updated_at,
    }
}

fn normalize_string_list(values: Vec<String>) -> Vec<String> {
    let mut normalized = values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
}

fn parse_json_string_array(raw: Option<String>) -> Vec<String> {
    normalize_string_list(
        raw.and_then(|value| serde_json::from_str::<Vec<String>>(&value).ok())
            .unwrap_or_default(),
    )
}

fn parse_json_uuid_array(raw: Option<String>) -> Vec<Uuid> {
    let mut values = raw
        .and_then(|value| serde_json::from_str::<Vec<Uuid>>(&value).ok())
        .unwrap_or_default();
    values.sort();
    values.dedup();
    values
}

fn parse_ai_routing_trigger_mode(raw: &str) -> Result<AiRoutingTriggerMode> {
    match raw {
        "hybrid" => Ok(AiRoutingTriggerMode::Hybrid),
        "scheduled_only" => Ok(AiRoutingTriggerMode::ScheduledOnly),
        "event_only" => Ok(AiRoutingTriggerMode::EventOnly),
        _ => Err(anyhow!("unsupported ai routing trigger mode: {raw}")),
    }
}

fn ai_routing_trigger_mode_to_db(mode: &AiRoutingTriggerMode) -> &'static str {
    match mode {
        AiRoutingTriggerMode::Hybrid => "hybrid",
        AiRoutingTriggerMode::ScheduledOnly => "scheduled_only",
        AiRoutingTriggerMode::EventOnly => "event_only",
    }
}

fn parse_routing_profile_row(row: &sqlx_postgres::PgRow) -> Result<RoutingProfile> {
    let selector = row
        .try_get::<Option<String>, _>("selector_json_text")?
        .and_then(|value| serde_json::from_str::<RoutingProfileSelector>(&value).ok())
        .unwrap_or_default();
    Ok(RoutingProfile {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        enabled: row.try_get("enabled")?,
        priority: row.try_get("priority")?,
        selector,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn parse_model_routing_policy_row(row: &sqlx_postgres::PgRow) -> Result<ModelRoutingPolicy> {
    Ok(ModelRoutingPolicy {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        family: row.try_get("family")?,
        exact_models: parse_json_string_array(
            row.try_get::<Option<String>, _>("exact_models_json_text")?,
        ),
        model_prefixes: parse_json_string_array(
            row.try_get::<Option<String>, _>("model_prefixes_json_text")?,
        ),
        fallback_profile_ids: parse_json_uuid_array(
            row.try_get::<Option<String>, _>("fallback_profile_ids_json_text")?,
        ),
        enabled: row.try_get("enabled")?,
        priority: row.try_get("priority")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn parse_routing_plan_version_row(row: &sqlx_postgres::PgRow) -> Result<RoutingPlanVersion> {
    let compiled_plan = row
        .try_get::<Option<String>, _>("compiled_plan_json_text")?
        .and_then(|value| serde_json::from_str::<CompiledRoutingPlan>(&value).ok())
        .ok_or_else(|| anyhow!("routing plan version compiled_plan_json is invalid"))?;
    Ok(RoutingPlanVersion {
        id: row.try_get("id")?,
        reason: row.try_get("reason")?,
        published_at: row.try_get("published_at")?,
        compiled_plan,
    })
}

fn derive_rate_limit_block(
    snapshots: &[OAuthRateLimitSnapshot],
    now: DateTime<Utc>,
) -> (Option<DateTime<Utc>>, Option<String>) {
    if let Some(blocked_until) =
        find_blocked_until_for_window(snapshots, true, Some(SECONDARY_RATE_LIMIT_WINDOW_MINUTES), now)
    {
        return (
            Some(blocked_until),
            Some("secondary_window_exhausted".to_string()),
        );
    }
    if let Some(blocked_until) =
        find_blocked_until_for_window(snapshots, false, Some(PRIMARY_RATE_LIMIT_WINDOW_MINUTES), now)
    {
        return (
            Some(blocked_until),
            Some("primary_window_exhausted".to_string()),
        );
    }
    (None, None)
}

fn find_blocked_until_for_window(
    snapshots: &[OAuthRateLimitSnapshot],
    secondary: bool,
    window_minutes: Option<i64>,
    now: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    snapshots
        .iter()
        .filter_map(|snapshot| {
            let window = if secondary {
                snapshot.secondary.as_ref()
            } else {
                snapshot.primary.as_ref()
            }?;
            if window.used_percent < 100.0 {
                return None;
            }
            if let Some(expected_minutes) = window_minutes {
                if let Some(actual_minutes) = window.window_minutes {
                    if actual_minutes != expected_minutes {
                        return None;
                    }
                }
            }
            let resets_at = window.resets_at?;
            (resets_at > now).then_some(resets_at)
        })
        .max()
}

fn compile_routing_plan_from_state(
    accounts: &[UpstreamAccount],
    account_traits: &HashMap<Uuid, AccountRoutingTraits>,
    profiles: &[RoutingProfile],
    source_policies: &[ModelRoutingPolicy],
    version_id: Uuid,
    published_at: DateTime<Utc>,
    trigger_reason: Option<String>,
) -> Option<CompiledRoutingPlan> {
    let now = published_at;
    let enabled_profiles = profiles
        .iter()
        .filter(|profile| profile.enabled)
        .cloned()
        .collect::<Vec<_>>();
    let compiled_profiles = enabled_profiles
        .iter()
        .map(|profile| {
            let mut matched = accounts
                .iter()
                .filter(|account| {
                    profile_matches_account(profile, account, account_traits)
                        && !account_is_routing_blocked(account_traits, account.id, now)
                })
                .cloned()
                .collect::<Vec<_>>();
            matched.sort_by(|left, right| {
                right
                    .priority
                    .cmp(&left.priority)
                    .then_with(|| left.created_at.cmp(&right.created_at))
            });
            (
                profile.id,
                CompiledRoutingProfile {
                    id: profile.id,
                    name: profile.name.clone(),
                    account_ids: matched.into_iter().map(|account| account.id).collect(),
                },
            )
        })
        .collect::<HashMap<_, _>>();

    let enabled_policies = source_policies
        .iter()
        .filter(|policy| policy.enabled)
        .cloned()
        .collect::<Vec<_>>();
    let default_policy = enabled_policies
        .iter()
        .find(|policy| policy.exact_models.is_empty() && policy.model_prefixes.is_empty())
        .cloned();
    let explicit_policies = enabled_policies
        .into_iter()
        .filter(|policy| !(policy.exact_models.is_empty() && policy.model_prefixes.is_empty()))
        .collect::<Vec<_>>();

    let mut known_models = account_traits
        .values()
        .flat_map(|traits| traits.supported_models.iter().cloned())
        .collect::<Vec<_>>();
    for policy in &explicit_policies {
        known_models.extend(policy.exact_models.iter().cloned());
    }
    known_models.sort();
    known_models.dedup();

    let mut routed_models = std::collections::HashSet::new();
    let mut policies = Vec::new();
    for policy in explicit_policies {
        let mut exact_models = policy.exact_models.clone();
        exact_models.extend(
            known_models
                .iter()
                .filter(|model: &&String| {
                    policy
                        .model_prefixes
                        .iter()
                        .any(|prefix| model.starts_with(prefix))
                })
                .cloned(),
        );
        exact_models.sort();
        exact_models.dedup();

        for model in exact_models {
            if routed_models.contains(&model) {
                continue;
            }
            let fallback_segments = build_compiled_fallback_segments(
                &policy.fallback_profile_ids,
                &compiled_profiles,
                Some(model.as_str()),
                account_traits,
            );
            if fallback_segments.is_empty() {
                continue;
            }
            routed_models.insert(model.clone());
            policies.push(CompiledModelRoutingPolicy {
                id: policy.id,
                name: policy.name.clone(),
                family: policy.family.clone(),
                exact_models: vec![model],
                model_prefixes: Vec::new(),
                fallback_segments,
            });
        }
    }

    let default_route = default_policy
        .as_ref()
        .map(|policy| {
            build_compiled_fallback_segments(
                &policy.fallback_profile_ids,
                &compiled_profiles,
                None,
                account_traits,
            )
        })
        .unwrap_or_default();

    if let Some(default_policy) = default_policy {
        for model in known_models {
            if routed_models.contains(&model) {
                continue;
            }
            let fallback_segments = build_compiled_fallback_segments(
                &default_policy.fallback_profile_ids,
                &compiled_profiles,
                Some(model.as_str()),
                account_traits,
            );
            if fallback_segments.is_empty() {
                continue;
            }
            policies.push(CompiledModelRoutingPolicy {
                id: default_policy.id,
                name: default_policy.name.clone(),
                family: default_policy.family.clone(),
                exact_models: vec![model],
                model_prefixes: Vec::new(),
                fallback_segments,
            });
        }
    }

    if default_route.is_empty() && policies.is_empty() {
        return None;
    }

    Some(CompiledRoutingPlan {
        version_id,
        published_at,
        trigger_reason,
        default_route,
        policies,
    })
}

fn build_compiled_fallback_segments(
    profile_ids: &[Uuid],
    compiled_profiles: &HashMap<Uuid, CompiledRoutingProfile>,
    model: Option<&str>,
    account_traits: &HashMap<Uuid, AccountRoutingTraits>,
) -> Vec<CompiledRoutingProfile> {
    profile_ids
        .iter()
        .filter_map(|profile_id| compiled_profiles.get(profile_id).cloned())
        .map(|profile| {
            let account_ids = match model {
                Some(model) => profile
                    .account_ids
                    .iter()
                    .copied()
                    .filter(|account_id| account_supports_model(account_traits, *account_id, model))
                    .collect::<Vec<_>>(),
                None => profile.account_ids.clone(),
            };
            CompiledRoutingProfile {
                account_ids,
                ..profile
            }
        })
        .filter(|profile| !profile.account_ids.is_empty())
        .collect()
}

fn profile_matches_account(
    profile: &RoutingProfile,
    account: &UpstreamAccount,
    account_traits: &HashMap<Uuid, AccountRoutingTraits>,
) -> bool {
    if profile.selector.exclude_account_ids.contains(&account.id) {
        return false;
    }
    if profile.selector.include_account_ids.contains(&account.id) {
        return true;
    }

    let Some(traits) = account_traits.get(&account.id) else {
        return false;
    };

    if !profile.selector.plan_types.is_empty()
        && !traits.plan_type.as_ref().is_some_and(|plan_type| {
            profile
                .selector
                .plan_types
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(plan_type))
        })
    {
        return false;
    }

    if !profile.selector.modes.is_empty() && !profile.selector.modes.contains(&account.mode) {
        return false;
    }

    if !profile.selector.auth_providers.is_empty()
        && !traits
            .auth_provider
            .as_ref()
            .is_some_and(|provider| profile.selector.auth_providers.contains(provider))
    {
        return false;
    }

    true
}

fn account_is_routing_blocked(
    account_traits: &HashMap<Uuid, AccountRoutingTraits>,
    account_id: Uuid,
    now: DateTime<Utc>,
) -> bool {
    account_traits
        .get(&account_id)
        .and_then(|traits| traits.blocked_until)
        .is_some_and(|blocked_until| blocked_until > now)
}

fn account_supports_model(
    account_traits: &HashMap<Uuid, AccountRoutingTraits>,
    account_id: Uuid,
    model: &str,
) -> bool {
    account_traits
        .get(&account_id)
        .map(|traits| {
            traits.supported_models.is_empty()
                || traits.supported_models.iter().any(|candidate| candidate == model)
        })
        .unwrap_or(false)
}
