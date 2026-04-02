#[async_trait]
impl ControlPlaneStore for InMemoryStore {
    async fn configure_system_event_runtime(
        &self,
        runtime: Option<Arc<crate::system_events::SystemEventLogRuntime>>,
    ) -> Result<()> {
        *self.system_event_runtime.write().unwrap() = runtime;
        Ok(())
    }

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

    async fn outbound_proxy_pool_settings(&self) -> Result<OutboundProxyPoolSettings> {
        Ok(self.outbound_proxy_pool_settings.read().unwrap().clone())
    }

    async fn update_outbound_proxy_pool_settings(
        &self,
        req: UpdateOutboundProxyPoolSettingsRequest,
    ) -> Result<OutboundProxyPoolSettings> {
        let settings = OutboundProxyPoolSettings {
            enabled: req.enabled,
            fail_mode: req.fail_mode,
            updated_at: Utc::now(),
        };
        *self.outbound_proxy_pool_settings.write().unwrap() = settings.clone();
        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(settings)
    }

    async fn list_outbound_proxy_nodes(&self) -> Result<Vec<OutboundProxyNode>> {
        Ok(self.list_outbound_proxy_nodes_inner())
    }

    async fn create_outbound_proxy_node(
        &self,
        req: CreateOutboundProxyNodeRequest,
    ) -> Result<OutboundProxyNode> {
        Ok(self.create_outbound_proxy_node_inner(req))
    }

    async fn update_outbound_proxy_node(
        &self,
        node_id: Uuid,
        req: UpdateOutboundProxyNodeRequest,
    ) -> Result<OutboundProxyNode> {
        self.update_outbound_proxy_node_inner(node_id, req)
    }

    async fn delete_outbound_proxy_node(&self, node_id: Uuid) -> Result<()> {
        self.delete_outbound_proxy_node_inner(node_id)
    }

    async fn record_outbound_proxy_test_result(
        &self,
        node_id: Uuid,
        last_test_status: Option<String>,
        last_latency_ms: Option<u64>,
        last_error: Option<String>,
        last_tested_at: Option<DateTime<Utc>>,
    ) -> Result<OutboundProxyNode> {
        self.record_outbound_proxy_test_result_inner(
            node_id,
            last_test_status,
            last_latency_ms,
            last_error,
            last_tested_at,
        )
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

    async fn mark_upstream_account_pending_purge(
        &self,
        account_id: Uuid,
        reason: Option<String>,
    ) -> Result<UpstreamAccount> {
        self.mark_upstream_account_pending_purge_inner(account_id, reason)
    }

    async fn purge_pending_upstream_accounts(&self) -> Result<u64> {
        self.purge_pending_upstream_accounts_inner()
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

    async fn queue_oauth_refresh_token(&self, req: ImportOAuthRefreshTokenRequest) -> Result<bool> {
        let created = self.queue_oauth_refresh_token_inner(req.clone())?;
        if let Some(record_id) = self.oauth_vault_record_id_for_request(&req) {
            self.probe_oauth_vault_admission_inner(record_id).await?;
        }
        Ok(created)
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

    async fn reprobe_runtime_oauth_account(&self, account_id: Uuid) -> Result<()> {
        self.reprobe_runtime_oauth_account_inner(account_id).await
    }

    async fn oauth_account_status(&self, account_id: Uuid) -> Result<OAuthAccountStatusResponse> {
        self.oauth_account_status_inner(account_id).await
    }

    async fn oauth_inventory_summary(
        &self,
    ) -> Result<crate::contracts::OAuthInventorySummaryResponse> {
        Ok(self.oauth_inventory_summary_inner())
    }

    async fn oauth_inventory_records(&self) -> Result<Vec<crate::contracts::OAuthInventoryRecord>> {
        Ok(self.oauth_inventory_records_inner())
    }

    async fn mark_oauth_inventory_record_failed(
        &self,
        record_id: Uuid,
        reason: Option<String>,
    ) -> Result<()> {
        self.mark_oauth_inventory_record_failed_inner(record_id, reason)
    }

    async fn mark_oauth_inventory_records_failed(
        &self,
        record_ids: Vec<Uuid>,
        reason: Option<String>,
    ) -> Result<()> {
        self.mark_oauth_inventory_records_failed_inner(&record_ids, reason);
        Ok(())
    }

    async fn delete_oauth_inventory_record(&self, record_id: Uuid) -> Result<()> {
        self.delete_oauth_inventory_record_inner(record_id)
    }

    async fn delete_oauth_inventory_records(&self, record_ids: Vec<Uuid>) -> Result<()> {
        self.delete_oauth_inventory_records_inner(&record_ids);
        Ok(())
    }

    async fn restore_oauth_inventory_record(&self, record_id: Uuid) -> Result<()> {
        self.restore_oauth_inventory_record_inner(record_id)
    }

    async fn restore_oauth_inventory_records(&self, record_ids: Vec<Uuid>) -> Result<()> {
        self.restore_oauth_inventory_records_inner(&record_ids);
        Ok(())
    }

    async fn reprobe_oauth_inventory_record(&self, record_id: Uuid) -> Result<()> {
        self.reprobe_oauth_inventory_record_inner(record_id).await
    }

    async fn reprobe_oauth_inventory_records(&self, record_ids: Vec<Uuid>) -> Result<()> {
        for record_id in record_ids {
            self.reprobe_oauth_inventory_record_inner(record_id).await?;
        }
        Ok(())
    }

    async fn purge_due_oauth_inventory_records(&self) -> Result<u64> {
        Ok(self.purge_due_oauth_inventory_records_inner())
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
        let removed = self
            .model_routing_policies
            .write()
            .unwrap()
            .remove(&policy_id);
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

    async fn claude_code_routing_settings(&self) -> Result<ClaudeCodeRoutingSettings> {
        Ok(self.claude_code_routing_settings.read().unwrap().clone())
    }

    async fn update_claude_code_routing_settings(
        &self,
        req: UpdateClaudeCodeRoutingSettingsRequest,
    ) -> Result<ClaudeCodeRoutingSettings> {
        let existing = self.claude_code_routing_settings.read().unwrap().clone();
        let settings = ClaudeCodeRoutingSettings {
            enabled: req.enabled,
            opus_target_model: normalize_optional_model(req.opus_target_model),
            sonnet_target_model: normalize_optional_model(req.sonnet_target_model),
            haiku_target_model: normalize_optional_model(req.haiku_target_model),
            effort_routing: req.effort_routing.unwrap_or(existing.effort_routing),
            updated_at: Utc::now(),
        };
        *self.claude_code_routing_settings.write().unwrap() = settings.clone();
        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(settings)
    }

    async fn upstream_error_learning_settings(&self) -> Result<AiErrorLearningSettings> {
        Ok(self
            .upstream_error_learning_settings
            .read()
            .unwrap()
            .clone())
    }

    async fn update_upstream_error_learning_settings(
        &self,
        req: UpdateAiErrorLearningSettingsRequest,
    ) -> Result<AiErrorLearningSettings> {
        let settings = AiErrorLearningSettings {
            enabled: req.enabled,
            first_seen_timeout_ms: req.first_seen_timeout_ms,
            review_hit_threshold: req.review_hit_threshold,
            updated_at: Some(Utc::now()),
        };
        *self.upstream_error_learning_settings.write().unwrap() = settings.clone();
        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(settings)
    }

    async fn list_upstream_error_templates(
        &self,
        status: Option<UpstreamErrorTemplateStatus>,
    ) -> Result<Vec<UpstreamErrorTemplateRecord>> {
        let mut templates = self
            .upstream_error_templates
            .read()
            .unwrap()
            .values()
            .filter(|template| status.is_none_or(|item| template.status == item))
            .cloned()
            .collect::<Vec<_>>();
        templates.sort_by(|left, right| {
            right
                .last_seen_at
                .cmp(&left.last_seen_at)
                .then_with(|| right.updated_at.cmp(&left.updated_at))
        });
        Ok(templates)
    }

    async fn upstream_error_template_by_id(
        &self,
        template_id: Uuid,
    ) -> Result<Option<UpstreamErrorTemplateRecord>> {
        Ok(self
            .upstream_error_templates
            .read()
            .unwrap()
            .get(&template_id)
            .cloned())
    }

    async fn upstream_error_template_by_fingerprint(
        &self,
        fingerprint: &str,
    ) -> Result<Option<UpstreamErrorTemplateRecord>> {
        let template_id = self
            .upstream_error_template_index
            .read()
            .unwrap()
            .get(fingerprint)
            .copied();
        Ok(template_id.and_then(|template_id| {
            self.upstream_error_templates
                .read()
                .unwrap()
                .get(&template_id)
                .cloned()
        }))
    }

    async fn save_upstream_error_template(
        &self,
        template: UpstreamErrorTemplateRecord,
    ) -> Result<UpstreamErrorTemplateRecord> {
        self.upstream_error_template_index
            .write()
            .unwrap()
            .insert(template.fingerprint.clone(), template.id);
        self.upstream_error_templates
            .write()
            .unwrap()
            .insert(template.id, template.clone());
        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(template)
    }

    async fn list_builtin_error_template_overrides(
        &self,
    ) -> Result<Vec<BuiltinErrorTemplateOverrideRecord>> {
        let mut records = self
            .builtin_error_template_overrides
            .read()
            .unwrap()
            .values()
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by(|left, right| {
            left.kind
                .cmp(&right.kind)
                .then_with(|| left.code.cmp(&right.code))
        });
        Ok(records)
    }

    async fn save_builtin_error_template_override(
        &self,
        record: BuiltinErrorTemplateOverrideRecord,
    ) -> Result<BuiltinErrorTemplateOverrideRecord> {
        self.builtin_error_template_overrides
            .write()
            .unwrap()
            .insert((record.kind, record.code.clone()), record.clone());
        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(record)
    }

    async fn delete_builtin_error_template_override(
        &self,
        kind: BuiltinErrorTemplateKind,
        code: &str,
    ) -> Result<()> {
        self.builtin_error_template_overrides
            .write()
            .unwrap()
            .remove(&(kind, code.to_string()));
        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn list_builtin_error_templates(&self) -> Result<Vec<BuiltinErrorTemplateRecord>> {
        Ok(self.list_builtin_error_templates_inner())
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

    async fn activate_oauth_refresh_token_vault(&self) -> Result<u64> {
        self.activate_oauth_refresh_token_vault_inner().await
    }

    async fn patrol_active_oauth_accounts(&self) -> Result<u64> {
        self.patrol_active_oauth_accounts_inner().await
    }

    async fn refresh_due_oauth_rate_limit_caches(&self) -> Result<u64> {
        let batch_size = rate_limit_refresh_batch_size_from_env();
        let concurrency = rate_limit_refresh_concurrency_from_env();
        let mut refreshed_total = 0_u64;

        loop {
            let started_at = Utc::now();
            let targets = self.load_rate_limit_refresh_targets(None, batch_size, true);
            if targets.is_empty() {
                break;
            }

            let fetched = targets.len();
            let stats = self
                .refresh_rate_limit_targets_batch(targets, concurrency)
                .await;
            self.emit_rate_limit_refresh_batch_summary_inner(
                started_at,
                "control_plane.rate_limit_refresh",
                None,
                fetched,
                &stats,
                true,
            );
            refreshed_total = refreshed_total.saturating_add(stats.processed);
            if fetched < batch_size {
                break;
            }
        }

        Ok(refreshed_total)
    }

    async fn recover_oauth_rate_limit_refresh_jobs(&self) -> Result<u64> {
        let now = Utc::now();
        let mut recovered = 0_u64;
        let mut jobs = self.oauth_rate_limit_refresh_jobs.write().unwrap();
        for summary in jobs.values_mut() {
            if summary.status != OAuthRateLimitRefreshJobStatus::Running {
                continue;
            }

            summary.status = OAuthRateLimitRefreshJobStatus::Failed;
            summary.finished_at = Some(now);
            if let Some(existing) = summary
                .error_summary
                .iter_mut()
                .find(|item| item.error_code == "job_recovered_after_restart")
            {
                existing.count = existing.count.saturating_add(1);
            } else {
                summary
                    .error_summary
                    .push(OAuthRateLimitRefreshErrorSummary {
                        error_code: "job_recovered_after_restart".to_string(),
                        count: 1,
                    });
            }
            recovered = recovered.saturating_add(1);
        }

        Ok(recovered)
    }

    async fn create_oauth_rate_limit_refresh_job(&self) -> Result<OAuthRateLimitRefreshJobSummary> {
        let mut jobs = self.oauth_rate_limit_refresh_jobs.write().unwrap();
        if let Some(existing) = jobs
            .values()
            .filter(|summary| {
                matches!(
                    summary.status,
                    OAuthRateLimitRefreshJobStatus::Queued
                        | OAuthRateLimitRefreshJobStatus::Running
                )
            })
            .cloned()
            .max_by(|left, right| {
                left.created_at
                    .cmp(&right.created_at)
                    .then_with(|| left.job_id.cmp(&right.job_id))
            })
        {
            return Ok(existing);
        }

        let now = Utc::now();
        let summary = OAuthRateLimitRefreshJobSummary {
            job_id: Uuid::new_v4(),
            status: OAuthRateLimitRefreshJobStatus::Queued,
            total: self.count_rate_limit_refresh_targets(),
            processed: 0,
            success_count: 0,
            failed_count: 0,
            started_at: None,
            finished_at: None,
            created_at: now,
            throughput_per_min: None,
            error_summary: Vec::new(),
        };
        jobs.insert(summary.job_id, summary.clone());

        Ok(summary)
    }

    async fn oauth_rate_limit_refresh_job(
        &self,
        job_id: Uuid,
    ) -> Result<OAuthRateLimitRefreshJobSummary> {
        self.oauth_rate_limit_refresh_jobs
            .read()
            .unwrap()
            .get(&job_id)
            .cloned()
            .ok_or_else(|| anyhow!("job not found"))
    }

    async fn run_oauth_rate_limit_refresh_job(&self, job_id: Uuid) -> Result<()> {
        let total = self.count_rate_limit_refresh_targets();
        {
            let mut jobs = self.oauth_rate_limit_refresh_jobs.write().unwrap();
            let summary = jobs
                .get_mut(&job_id)
                .ok_or_else(|| anyhow!("job not found"))?;
            if summary.status != OAuthRateLimitRefreshJobStatus::Queued {
                return Ok(());
            }

            summary.status = OAuthRateLimitRefreshJobStatus::Running;
            summary.total = total;
            summary.processed = 0;
            summary.success_count = 0;
            summary.failed_count = 0;
            summary.started_at = Some(Utc::now());
            summary.finished_at = None;
            summary.throughput_per_min = None;
            summary.error_summary.clear();
        }

        let run_result: Result<()> = async {
            let batch_size = rate_limit_refresh_batch_size_from_env();
            let concurrency = rate_limit_refresh_concurrency_from_env();
            let mut cursor = None;

            loop {
                let started_at = Utc::now();
                let targets = self.load_rate_limit_refresh_targets(cursor, batch_size, false);
                if targets.is_empty() {
                    break;
                }

                let fetched = targets.len();
                cursor = targets.last().map(|target| target.account_id);
                let stats = self
                    .refresh_rate_limit_targets_batch(targets, concurrency)
                    .await;
                self.emit_rate_limit_refresh_batch_summary_inner(
                    started_at,
                    "control_plane.rate_limit_refresh_job",
                    Some(job_id),
                    fetched,
                    &stats,
                    false,
                );
                self.append_rate_limit_refresh_job_progress(job_id, &stats)?;

                if fetched < batch_size {
                    break;
                }
            }

            self.finish_rate_limit_refresh_job(job_id, OAuthRateLimitRefreshJobStatus::Completed)
        }
        .await;

        if let Err(err) = run_result {
            let _ = self.mark_rate_limit_refresh_job_failed(job_id, "internal_error");
            return Err(err);
        }

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
        Ok(self.mark_account_seen_ok_inner(account_id, seen_ok_at, min_write_interval_sec))
    }

    async fn maybe_refresh_oauth_rate_limit_cache_on_seen_ok(
        &self,
        account_id: Uuid,
    ) -> Result<()> {
        let now = Utc::now();
        let account = self.accounts.read().unwrap().get(&account_id).cloned();
        let provider = self
            .account_auth_providers
            .read()
            .unwrap()
            .get(&account_id)
            .cloned();
        let credential = self
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
        let cache = self
            .oauth_rate_limit_caches
            .read()
            .unwrap()
            .get(&account_id)
            .cloned()
            .unwrap_or_default();

        let (Some(account), Some(provider)) = (account, provider) else {
            return Ok(());
        };
        if !account.enabled {
            return Ok(());
        }
        let (
            token_expires_at,
            last_refresh_status,
            refresh_reused_detected,
            last_refresh_error_code,
        ) = match provider {
            UpstreamAuthProvider::OAuthRefreshToken => {
                let Some(credential) = credential.as_ref() else {
                    return Ok(());
                };
                (
                    Some(credential.token_expires_at),
                    credential.last_refresh_status.clone(),
                    credential.refresh_reused_detected,
                    credential.last_refresh_error_code.clone(),
                )
            }
            UpstreamAuthProvider::LegacyBearer => {
                let Some(profile) = session_profile.as_ref() else {
                    return Ok(());
                };
                if account.mode != UpstreamMode::CodexOauth
                    || profile.credential_kind != SessionCredentialKind::OneTimeAccessToken
                {
                    return Ok(());
                }
                (
                    profile.token_expires_at,
                    OAuthRefreshStatus::Never,
                    false,
                    None,
                )
            }
        };
        if !should_refresh_rate_limit_cache_on_seen_ok(
            now,
            SeenOkRateLimitRefreshContext {
                token_expires_at,
                last_refresh_status: &last_refresh_status,
                refresh_reused_detected,
                last_refresh_error_code: last_refresh_error_code.as_deref(),
                rate_limits_expires_at: cache.expires_at,
                rate_limits_last_error_code: cache.last_error_code.as_deref(),
                rate_limits_last_error: cache.last_error.as_deref(),
            },
        ) {
            return Ok(());
        }

        let Some(target) = self.build_rate_limit_refresh_target_inner(
            &account,
            &provider,
            credential.as_ref(),
            session_profile.as_ref(),
            Some(&cache),
            now,
            false,
        ) else {
            return Ok(());
        };
        let _ = self.refresh_rate_limit_targets_batch(vec![target], 1).await;

        Ok(())
    }

    async fn update_oauth_rate_limit_cache_from_observation(
        &self,
        account_id: Uuid,
        rate_limits: Vec<OAuthRateLimitSnapshot>,
        observed_at: DateTime<Utc>,
    ) -> Result<()> {
        self.persist_rate_limit_cache_success_inner(account_id, rate_limits, observed_at, None);
        Ok(())
    }

    async fn record_upstream_account_live_result(
        &self,
        account_id: Uuid,
        reported_at: DateTime<Utc>,
        status: OAuthLiveResultStatus,
        source: OAuthLiveResultSource,
        status_code: Option<u16>,
        error_code: Option<String>,
        error_message_preview: Option<String>,
    ) -> Result<bool> {
        self.record_upstream_account_live_result_inner(
            account_id,
            reported_at,
            status,
            source,
            status_code,
            error_code,
            error_message_preview,
        )
    }
}

fn normalize_optional_model(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
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
    use super::{
        ControlPlaneStore, InMemoryStore, OAuthCredentialRecord, UpsertOneTimeSessionAccountRequest,
    };
    use crate::contracts::{
        CreateApiKeyRequest, CreateTenantRequest, CreateUpstreamAccountRequest,
        ImportOAuthRefreshTokenRequest, OAuthAccountPoolState, OAuthInventoryFailureStage,
        OAuthLiveResultSource, OAuthLiveResultStatus, OAuthRateLimitRefreshJobStatus,
        OAuthRateLimitSnapshot, OAuthRateLimitWindow, OAuthRefreshStatus, OAuthVaultRecordStatus,
        RefreshCredentialState, SessionCredentialKind, UpsertModelRoutingPolicyRequest,
        UpsertRoutingProfileRequest,
    };
    use crate::crypto::CredentialCipher;
    use crate::oauth::{OAuthTokenClient, OAuthTokenInfo, OAuthUsageSnapshot};
    use crate::system_events::{
        sqlite_repo::SqliteSystemEventRepo, SystemEventLogRuntime, SystemEventQuery,
        SystemEventRepository,
    };
    use async_trait::async_trait;
    use base64::Engine;
    use chrono::{DateTime, Duration, Utc};
    use codex_pool_core::model::{RoutingProfileSelector, UpstreamAuthProvider, UpstreamMode};
    use serde_json::json;
    use sqlx_sqlite::SqlitePool;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use tokio::time::{sleep, Duration as TokioDuration};

    async fn wait_for_system_events(
        repo: &Arc<SqliteSystemEventRepo>,
        expected_min: usize,
    ) -> Vec<crate::contracts::SystemEventRecord> {
        for _ in 0..20 {
            let items = repo
                .list_events(SystemEventQuery {
                    limit: Some(500),
                    ..Default::default()
                })
                .await
                .unwrap()
                .items;
            if items.len() >= expected_min {
                return items;
            }
            sleep(TokioDuration::from_millis(25)).await;
        }
        repo.list_events(SystemEventQuery {
            limit: Some(500),
            ..Default::default()
        })
        .await
        .unwrap()
        .items
    }

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

    #[tokio::test]
    async fn in_memory_create_upstream_account_normalizes_codex_base_url() {
        let store = InMemoryStore::default();

        let account = store
            .create_upstream_account(CreateUpstreamAccountRequest {
                label: "codex-ak".to_string(),
                mode: UpstreamMode::CodexOauth,
                base_url: "https://chatgpt.com".to_string(),
                bearer_token: "access-token".to_string(),
                chatgpt_account_id: Some("acct-demo".to_string()),
                auth_provider: None,
                enabled: Some(true),
                priority: Some(100),
            })
            .await
            .unwrap();

        assert_eq!(account.base_url, "https://chatgpt.com/backend-api/codex");
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
    struct TerminalRefreshFailureOAuthTokenClient;

    #[async_trait]
    impl OAuthTokenClient for TerminalRefreshFailureOAuthTokenClient {
        async fn refresh_token(
            &self,
            _refresh_token: &str,
            _base_url: Option<&str>,
        ) -> Result<OAuthTokenInfo, crate::oauth::OAuthTokenClientError> {
            Err(crate::oauth::OAuthTokenClientError::InvalidRefreshToken {
                code: crate::oauth::OAuthRefreshErrorCode::InvalidRefreshToken,
                message: "refresh token is invalid".to_string(),
            })
        }
    }

    #[derive(Clone)]
    struct ImportOkThenTerminalFailureOAuthTokenClient {
        refresh_calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl OAuthTokenClient for ImportOkThenTerminalFailureOAuthTokenClient {
        async fn refresh_token(
            &self,
            refresh_token: &str,
            base_url: Option<&str>,
        ) -> Result<OAuthTokenInfo, crate::oauth::OAuthTokenClientError> {
            if self.refresh_calls.fetch_add(1, Ordering::Relaxed) == 0 {
                return StaticOAuthTokenClient
                    .refresh_token(refresh_token, base_url)
                    .await;
            }
            Err(crate::oauth::OAuthTokenClientError::InvalidRefreshToken {
                code: crate::oauth::OAuthRefreshErrorCode::InvalidRefreshToken,
                message: "refresh token is invalid".to_string(),
            })
        }
    }

    #[derive(Clone)]
    struct RateLimitAwareOAuthTokenClient;

    #[async_trait]
    impl OAuthTokenClient for RateLimitAwareOAuthTokenClient {
        async fn refresh_token(
            &self,
            _refresh_token: &str,
            _base_url: Option<&str>,
        ) -> Result<OAuthTokenInfo, crate::oauth::OAuthTokenClientError> {
            StaticOAuthTokenClient.refresh_token("rt", None).await
        }

        async fn fetch_rate_limits(
            &self,
            _access_token: &str,
            _base_url: Option<&str>,
            _chatgpt_account_id: Option<&str>,
        ) -> Result<Vec<OAuthRateLimitSnapshot>, crate::oauth::OAuthTokenClientError> {
            Ok(vec![OAuthRateLimitSnapshot {
                limit_id: Some("five_hours".to_string()),
                limit_name: Some("5 hours".to_string()),
                primary: Some(OAuthRateLimitWindow {
                    used_percent: 25.0,
                    window_minutes: Some(300),
                    resets_at: Some(Utc::now() + Duration::minutes(30)),
                }),
                secondary: None,
            }])
        }

        async fn fetch_usage(
            &self,
            access_token: &str,
            base_url: Option<&str>,
            chatgpt_account_id: Option<&str>,
        ) -> Result<OAuthUsageSnapshot, crate::oauth::OAuthTokenClientError> {
            Ok(OAuthUsageSnapshot {
                rate_limits: self
                    .fetch_rate_limits(access_token, base_url, chatgpt_account_id)
                    .await?,
                chatgpt_plan_type: Some("plus".to_string()),
            })
        }
    }

    #[derive(Clone)]
    struct LegacyBearerAuthFailureOAuthTokenClient;

    #[async_trait]
    impl OAuthTokenClient for LegacyBearerAuthFailureOAuthTokenClient {
        async fn refresh_token(
            &self,
            _refresh_token: &str,
            _base_url: Option<&str>,
        ) -> Result<OAuthTokenInfo, crate::oauth::OAuthTokenClientError> {
            StaticOAuthTokenClient.refresh_token("rt", None).await
        }

        async fn fetch_rate_limits(
            &self,
            _access_token: &str,
            _base_url: Option<&str>,
            _chatgpt_account_id: Option<&str>,
        ) -> Result<Vec<OAuthRateLimitSnapshot>, crate::oauth::OAuthTokenClientError> {
            Err(crate::oauth::OAuthTokenClientError::InvalidRefreshToken {
                code: crate::oauth::OAuthRefreshErrorCode::InvalidRefreshToken,
                message: "access token is invalid".to_string(),
            })
        }
    }

    #[derive(Clone)]
    struct AdmissionProbeOAuthTokenClient {
        refresh_calls: Arc<AtomicUsize>,
        rate_limits: Vec<OAuthRateLimitSnapshot>,
    }

    #[async_trait]
    impl OAuthTokenClient for AdmissionProbeOAuthTokenClient {
        async fn refresh_token(
            &self,
            refresh_token: &str,
            base_url: Option<&str>,
        ) -> Result<OAuthTokenInfo, crate::oauth::OAuthTokenClientError> {
            self.refresh_calls.fetch_add(1, Ordering::SeqCst);
            StaticOAuthTokenClient
                .refresh_token(refresh_token, base_url)
                .await
        }

        async fn fetch_rate_limits(
            &self,
            _access_token: &str,
            _base_url: Option<&str>,
            _chatgpt_account_id: Option<&str>,
        ) -> Result<Vec<OAuthRateLimitSnapshot>, crate::oauth::OAuthTokenClientError> {
            Ok(self.rate_limits.clone())
        }
    }

    #[derive(Clone)]
    struct SequentialAdmissionProbeOAuthTokenClient {
        fetch_results: Arc<
            Mutex<
                VecDeque<Result<Vec<OAuthRateLimitSnapshot>, crate::oauth::OAuthTokenClientError>>,
            >,
        >,
    }

    #[async_trait]
    impl OAuthTokenClient for SequentialAdmissionProbeOAuthTokenClient {
        async fn refresh_token(
            &self,
            refresh_token: &str,
            base_url: Option<&str>,
        ) -> Result<OAuthTokenInfo, crate::oauth::OAuthTokenClientError> {
            StaticOAuthTokenClient
                .refresh_token(refresh_token, base_url)
                .await
        }

        async fn fetch_rate_limits(
            &self,
            _access_token: &str,
            _base_url: Option<&str>,
            _chatgpt_account_id: Option<&str>,
        ) -> Result<Vec<OAuthRateLimitSnapshot>, crate::oauth::OAuthTokenClientError> {
            self.fetch_results
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| Ok(Vec::new()))
        }
    }

    #[derive(Clone)]
    struct TransientAdmissionFailureOAuthTokenClient;

    #[async_trait]
    impl OAuthTokenClient for TransientAdmissionFailureOAuthTokenClient {
        async fn refresh_token(
            &self,
            refresh_token: &str,
            base_url: Option<&str>,
        ) -> Result<OAuthTokenInfo, crate::oauth::OAuthTokenClientError> {
            StaticOAuthTokenClient
                .refresh_token(refresh_token, base_url)
                .await
        }

        async fn fetch_rate_limits(
            &self,
            _access_token: &str,
            _base_url: Option<&str>,
            _chatgpt_account_id: Option<&str>,
        ) -> Result<Vec<OAuthRateLimitSnapshot>, crate::oauth::OAuthTokenClientError> {
            Err(crate::oauth::OAuthTokenClientError::Upstream {
                code: crate::oauth::OAuthRefreshErrorCode::UpstreamUnavailable,
                message: "upstream unavailable".to_string(),
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
            let (email, account_user_id, workspace_name) = if refresh_token.contains("workspace-b")
            {
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
            if access_token == "probe-access" && chatgpt_account_id == Some("acct_probe_team") {
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
                fallback_access_token: None,
                fallback_token_expires_at: None,
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
    async fn in_memory_oauth_import_normalizes_codex_base_url() {
        let cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([7_u8; 32]),
        )
        .unwrap();
        let store = InMemoryStore::new_with_oauth(Arc::new(StaticOAuthTokenClient), Some(cipher));

        let account = store
            .import_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-bare-base".to_string(),
                base_url: "https://chatgpt.com".to_string(),
                refresh_token: "rt-bare-base".to_string(),
                fallback_access_token: None,
                fallback_token_expires_at: None,
                chatgpt_account_id: None,
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: None,
                source_type: Some("codex".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(account.base_url, "https://chatgpt.com/backend-api/codex");
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
                fallback_access_token: None,
                fallback_token_expires_at: None,
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
        assert_eq!(status.chatgpt_compute_residency.as_deref(), Some("us"));
        assert_eq!(status.organizations.as_ref().map(Vec::len), Some(1));
        assert_eq!(status.groups.as_ref().map(Vec::len), Some(1));
    }

    #[tokio::test]
    async fn in_memory_oauth_status_reports_refresh_credential_capabilities() {
        let cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([13_u8; 32]),
        )
        .unwrap();
        let store = InMemoryStore::new_with_oauth(Arc::new(StaticOAuthTokenClient), Some(cipher));

        let account = store
            .import_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-refresh-state".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-refresh-state".to_string(),
                fallback_access_token: None,
                fallback_token_expires_at: None,
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

        assert_eq!(
            status.credential_kind,
            Some(SessionCredentialKind::RefreshRotatable)
        );
        assert!(status.has_refresh_credential);
        assert!(!status.has_access_token_fallback);
        assert_eq!(
            status.refresh_credential_state,
            Some(RefreshCredentialState::Healthy)
        );
    }

    #[tokio::test]
    async fn in_memory_oauth_status_reports_access_fallback_when_imported_with_refresh() {
        let key = base64::engine::general_purpose::STANDARD.encode([16_u8; 32]);
        let cipher = CredentialCipher::from_base64_key(&key).unwrap();
        let store = InMemoryStore::new_with_oauth(Arc::new(StaticOAuthTokenClient), Some(cipher));
        let fallback_token_expires_at = Utc::now() + Duration::hours(2);

        let account = store
            .import_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-refresh-with-fallback".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-refresh-with-fallback".to_string(),
                fallback_access_token: Some("ak-fallback".to_string()),
                fallback_token_expires_at: Some(fallback_token_expires_at),
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
        assert!(status.has_refresh_credential);
        assert!(status.has_access_token_fallback);

        let stored = store
            .oauth_credentials
            .read()
            .unwrap()
            .get(&account.id)
            .cloned()
            .expect("oauth credential exists");
        let verifier = CredentialCipher::from_base64_key(&key).unwrap();
        assert_eq!(
            verifier
                .decrypt(
                    stored
                        .fallback_access_token_enc
                        .as_deref()
                        .expect("fallback access token stored"),
                )
                .unwrap(),
            "ak-fallback"
        );
        assert_eq!(
            stored.fallback_token_expires_at,
            Some(fallback_token_expires_at)
        );
    }

    #[tokio::test]
    async fn in_memory_oauth_upsert_preserves_existing_access_fallback_when_reimport_omits_it() {
        let key = base64::engine::general_purpose::STANDARD.encode([17_u8; 32]);
        let cipher = CredentialCipher::from_base64_key(&key).unwrap();
        let store = InMemoryStore::new_with_oauth(Arc::new(StaticOAuthTokenClient), Some(cipher));

        let first = store
            .upsert_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-fallback-preserve".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-fallback-preserve".to_string(),
                fallback_access_token: Some("ak-preserve".to_string()),
                fallback_token_expires_at: Some(Utc::now() + Duration::hours(3)),
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
                label: "oauth-fallback-preserve-updated".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-fallback-preserve".to_string(),
                fallback_access_token: None,
                fallback_token_expires_at: None,
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

        let stored = store
            .oauth_credentials
            .read()
            .unwrap()
            .get(&second.account.id)
            .cloned()
            .expect("oauth credential exists");
        let verifier = CredentialCipher::from_base64_key(&key).unwrap();
        assert_eq!(
            verifier
                .decrypt(
                    stored
                        .fallback_access_token_enc
                        .as_deref()
                        .expect("fallback access token stored"),
                )
                .unwrap(),
            "ak-preserve"
        );
        assert!(
            store
                .oauth_account_status(second.account.id)
                .await
                .unwrap()
                .has_access_token_fallback
        );
    }

    #[tokio::test]
    async fn in_memory_snapshot_uses_access_fallback_when_refresh_token_is_terminally_invalid() {
        let key = base64::engine::general_purpose::STANDARD.encode([18_u8; 32]);
        let cipher = CredentialCipher::from_base64_key(&key).unwrap();
        let store = InMemoryStore::new_with_oauth(Arc::new(StaticOAuthTokenClient), Some(cipher));

        let account = store
            .import_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-fallback-runtime".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-fallback-runtime".to_string(),
                fallback_access_token: Some("ak-runtime".to_string()),
                fallback_token_expires_at: Some(Utc::now() + Duration::hours(2)),
                chatgpt_account_id: None,
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: None,
                source_type: Some("codex".to_string()),
            })
            .await
            .unwrap();

        {
            let mut credentials = store.oauth_credentials.write().unwrap();
            let credential = credentials
                .get_mut(&account.id)
                .expect("oauth credential exists");
            credential.token_expires_at = Utc::now() - Duration::minutes(5);
            credential.last_refresh_status = crate::contracts::OAuthRefreshStatus::Failed;
            credential.last_refresh_error_code = Some("invalid_refresh_token".to_string());
            credential.last_refresh_error = Some("refresh token invalid".to_string());
        }

        let snapshot = store.snapshot().await.unwrap();
        let account = snapshot
            .accounts
            .into_iter()
            .find(|item| item.id == account.id)
            .expect("snapshot account exists");

        assert!(account.enabled);
        assert_eq!(account.bearer_token, "ak-runtime");
    }

    #[tokio::test]
    async fn in_memory_one_time_status_reports_access_only_capabilities() {
        let store = InMemoryStore::default();

        let account = store
            .upsert_one_time_session_account(UpsertOneTimeSessionAccountRequest {
                label: "one-time-refresh-state".to_string(),
                mode: UpstreamMode::CodexOauth,
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                access_token: "one-time-token".to_string(),
                chatgpt_account_id: Some("acct-one-time".to_string()),
                enabled: Some(true),
                priority: Some(100),
                token_expires_at: Some(Utc::now() + Duration::hours(1)),
                chatgpt_plan_type: Some("free".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .unwrap()
            .account;

        let status = store.oauth_account_status(account.id).await.unwrap();

        assert_eq!(
            status.credential_kind,
            Some(SessionCredentialKind::OneTimeAccessToken)
        );
        assert!(!status.has_refresh_credential);
        assert!(!status.has_access_token_fallback);
        assert_eq!(status.refresh_credential_state, None);
    }

    #[tokio::test]
    async fn in_memory_oauth_status_marks_terminal_refresh_failure() {
        let healthy_cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([14_u8; 32]),
        )
        .unwrap();
        let healthy_store =
            InMemoryStore::new_with_oauth(Arc::new(StaticOAuthTokenClient), Some(healthy_cipher));

        let account = healthy_store
            .import_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-terminal-refresh-state".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-terminal-refresh-state".to_string(),
                fallback_access_token: None,
                fallback_token_expires_at: None,
                chatgpt_account_id: None,
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: None,
                source_type: Some("codex".to_string()),
            })
            .await
            .unwrap();

        let failing_cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([15_u8; 32]),
        )
        .unwrap();
        let failing_store = InMemoryStore::new_with_oauth(
            Arc::new(TerminalRefreshFailureOAuthTokenClient),
            Some(failing_cipher),
        );
        failing_store
            .accounts
            .write()
            .unwrap()
            .insert(account.id, account.clone());
        failing_store
            .account_auth_providers
            .write()
            .unwrap()
            .insert(account.id, UpstreamAuthProvider::OAuthRefreshToken);
        let cipher = failing_store.require_credential_cipher().unwrap();
        failing_store.oauth_credentials.write().unwrap().insert(
            account.id,
            OAuthCredentialRecord::from_token_info(
                cipher,
                &OAuthTokenInfo {
                    access_token: "access-1".to_string(),
                    refresh_token: "refresh-1".to_string(),
                    expires_at: Utc::now() + Duration::seconds(10),
                    token_type: Some("Bearer".to_string()),
                    scope: Some("model.read".to_string()),
                    email: Some("demo@example.com".to_string()),
                    oauth_subject: Some("auth0|demo".to_string()),
                    oauth_identity_provider: Some("google-oauth2".to_string()),
                    email_verified: Some(true),
                    chatgpt_account_id: Some("acct_demo".to_string()),
                    chatgpt_user_id: Some("user_demo".to_string()),
                    chatgpt_plan_type: Some("pro".to_string()),
                    chatgpt_subscription_active_start: None,
                    chatgpt_subscription_active_until: None,
                    chatgpt_subscription_last_checked: None,
                    chatgpt_account_user_id: Some("acct_user_demo".to_string()),
                    chatgpt_compute_residency: Some("us".to_string()),
                    workspace_name: None,
                    organizations: None,
                    groups: None,
                },
            )
            .unwrap(),
        );

        let status = failing_store
            .refresh_oauth_account(account.id)
            .await
            .unwrap();

        assert_eq!(
            status.refresh_credential_state,
            Some(RefreshCredentialState::TerminalInvalid)
        );
    }

    #[tokio::test]
    async fn in_memory_refresh_oauth_account_marks_terminal_failure_pending_purge() {
        let cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([16_u8; 32]),
        )
        .unwrap();
        let store = InMemoryStore::new_with_oauth(
            Arc::new(ImportOkThenTerminalFailureOAuthTokenClient {
                refresh_calls: Arc::new(AtomicUsize::new(0)),
            }),
            Some(cipher),
        );

        let account = store
            .import_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-pending-purge".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-pending-purge".to_string(),
                fallback_access_token: None,
                fallback_token_expires_at: None,
                chatgpt_account_id: None,
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: None,
                source_type: Some("codex".to_string()),
            })
            .await
            .unwrap();

        let status = store.refresh_oauth_account(account.id).await.unwrap();
        assert_eq!(status.pool_state, OAuthAccountPoolState::PendingPurge);
        assert_eq!(
            status.pending_purge_reason.as_deref(),
            Some("invalid_refresh_token")
        );
        assert!(status.pending_purge_at.is_some());

        let snapshot = store.snapshot().await.unwrap();
        assert!(
            snapshot.accounts.iter().all(|item| item.id != account.id),
            "pending purge account should be excluded from runtime snapshot"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn in_memory_pending_delete_and_delete_emit_account_pool_events() {
        let store = InMemoryStore::default();
        let event_repo = Arc::new(
            SqliteSystemEventRepo::new(SqlitePool::connect("sqlite::memory:").await.unwrap())
                .await
                .unwrap(),
        );
        store
            .configure_system_event_runtime(Some(Arc::new(SystemEventLogRuntime::new(
                event_repo.clone(),
            ))))
            .await
            .unwrap();

        let account = store
            .create_upstream_account(CreateUpstreamAccountRequest {
                label: "event-account@example.com".to_string(),
                mode: UpstreamMode::CodexOauth,
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                bearer_token: "ak-event".to_string(),
                chatgpt_account_id: None,
                auth_provider: Some(UpstreamAuthProvider::LegacyBearer),
                enabled: Some(true),
                priority: Some(100),
            })
            .await
            .unwrap();

        store
            .mark_upstream_account_pending_purge(
                account.id,
                Some("account_deactivated".to_string()),
            )
            .await
            .unwrap();
        store.delete_upstream_account(account.id).await.unwrap();

        let items = wait_for_system_events(&event_repo, 2).await;
        assert!(
            items.iter().any(|item| {
                item.event_type == "account_pool_state_transition"
                    && item.account_id == Some(account.id)
                    && item.operator_state_to.as_deref() == Some("pending_delete")
                    && item.reason_code.as_deref() == Some("account_deactivated")
            }),
            "pending delete transition event should be present"
        );
        assert!(
            items.iter().any(|item| {
                item.event_type == "account_deleted"
                    && item.account_id == Some(account.id)
                    && item.account_label.as_deref() == Some("event-account@example.com")
            }),
            "account_deleted event should be present"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn in_memory_active_patrol_emits_probe_and_batch_events() {
        let cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([17_u8; 32]),
        )
        .unwrap();
        let store =
            InMemoryStore::new_with_oauth(Arc::new(RateLimitAwareOAuthTokenClient), Some(cipher));
        let event_repo = Arc::new(
            SqliteSystemEventRepo::new(SqlitePool::connect("sqlite::memory:").await.unwrap())
                .await
                .unwrap(),
        );
        store
            .configure_system_event_runtime(Some(Arc::new(SystemEventLogRuntime::new(
                event_repo.clone(),
            ))))
            .await
            .unwrap();

        let account = store
            .import_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-patrol-events".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-patrol-events".to_string(),
                fallback_access_token: None,
                fallback_token_expires_at: None,
                chatgpt_account_id: None,
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: None,
                source_type: Some("codex".to_string()),
            })
            .await
            .unwrap();

        store
            .record_upstream_account_live_result(
                account.id,
                Utc::now(),
                OAuthLiveResultStatus::Failed,
                OAuthLiveResultSource::Passive,
                Some(502),
                Some("transport_error".to_string()),
                Some("stale".to_string()),
            )
            .await
            .unwrap();

        let patrolled = store.patrol_active_oauth_accounts().await.unwrap();
        assert_eq!(patrolled, 1);

        let items = wait_for_system_events(&event_repo, 2).await;
        assert!(
            items.iter().any(|item| {
                item.category == codex_pool_core::events::SystemEventCategory::Patrol
                    && item.event_type == "probe_succeeded"
                    && item.account_id == Some(account.id)
            }),
            "probe_succeeded event should be present"
        );
        assert!(
            items.iter().any(|item| {
                item.category == codex_pool_core::events::SystemEventCategory::Patrol
                    && item.event_type == "active_patrol_batch_completed"
                    && item
                        .payload_json
                        .as_ref()
                        .and_then(|value| value.get("patrolled"))
                        .and_then(|value| value.as_u64())
                        == Some(1)
            }),
            "active_patrol_batch_completed summary should be present"
        );
    }

    #[tokio::test]
    async fn in_memory_store_rate_limit_refresh_job_populates_oauth_status_cache() {
        let cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([7_u8; 32]),
        )
        .unwrap();
        let store =
            InMemoryStore::new_with_oauth(Arc::new(RateLimitAwareOAuthTokenClient), Some(cipher));

        let account = store
            .import_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-rate-limit".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-rate-limit".to_string(),
                fallback_access_token: None,
                fallback_token_expires_at: None,
                chatgpt_account_id: None,
                mode: None,
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: None,
                source_type: None,
            })
            .await
            .expect("import oauth account");

        let created = store
            .create_oauth_rate_limit_refresh_job()
            .await
            .expect("create rate-limit refresh job");
        assert_eq!(created.total, 1);

        store
            .run_oauth_rate_limit_refresh_job(created.job_id)
            .await
            .expect("run rate-limit refresh job");

        let summary = store
            .oauth_rate_limit_refresh_job(created.job_id)
            .await
            .expect("load rate-limit refresh job");
        assert_eq!(summary.status, OAuthRateLimitRefreshJobStatus::Completed);
        assert_eq!(summary.total, 1);
        assert_eq!(summary.processed, 1);
        assert_eq!(summary.success_count, 1);
        assert_eq!(summary.failed_count, 0);

        let status = store
            .oauth_account_status(account.id)
            .await
            .expect("oauth account status");
        assert_eq!(status.rate_limits.len(), 1);
        assert_eq!(
            status.rate_limits[0].limit_id.as_deref(),
            Some("five_hours")
        );
        assert_eq!(status.chatgpt_plan_type.as_deref(), Some("plus"));
        assert!(status.rate_limits_fetched_at.is_some());
        assert!(status.rate_limits_expires_at.is_some());
        assert!(status.rate_limits_last_error_code.is_none());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn in_memory_due_rate_limit_refresh_emits_batch_event() {
        let store = InMemoryStore::new_with_oauth(Arc::new(RateLimitAwareOAuthTokenClient), None);
        let event_repo = Arc::new(
            SqliteSystemEventRepo::new(SqlitePool::connect("sqlite::memory:").await.unwrap())
                .await
                .unwrap(),
        );
        store
            .configure_system_event_runtime(Some(Arc::new(SystemEventLogRuntime::new(
                event_repo.clone(),
            ))))
            .await
            .unwrap();

        store
            .upsert_one_time_session_account(UpsertOneTimeSessionAccountRequest {
                label: "legacy-codex-rate-limit-event".to_string(),
                mode: UpstreamMode::CodexOauth,
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                access_token: "legacy-rate-limit-event-token".to_string(),
                chatgpt_account_id: Some("acct_legacy_rate_limit_event".to_string()),
                enabled: Some(true),
                priority: Some(100),
                token_expires_at: Some(Utc::now() + Duration::hours(2)),
                chatgpt_plan_type: Some("free".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("upsert one-time codex account");

        let refreshed = store
            .refresh_due_oauth_rate_limit_caches()
            .await
            .expect("refresh due rate-limit caches");
        assert_eq!(refreshed, 1);

        let items = wait_for_system_events(&event_repo, 1).await;
        assert!(
            items.iter().any(|item| {
                item.category == codex_pool_core::events::SystemEventCategory::Patrol
                    && item.event_type == "rate_limit_refresh_batch_completed"
                    && item.source == "control_plane.rate_limit_refresh"
                    && item.job_id.is_none()
                    && item
                        .payload_json
                        .as_ref()
                        .and_then(|value| value.get("processed"))
                        .and_then(|value| value.as_u64())
                        == Some(1)
                    && item
                        .payload_json
                        .as_ref()
                        .and_then(|value| value.get("success"))
                        .and_then(|value| value.as_u64())
                        == Some(1)
                    && item
                        .payload_json
                        .as_ref()
                        .and_then(|value| value.get("failed"))
                        .and_then(|value| value.as_u64())
                        == Some(0)
            }),
            "due rate-limit refresh batch event should be present"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn in_memory_rate_limit_refresh_job_emits_batch_event() {
        let store = InMemoryStore::new_with_oauth(Arc::new(RateLimitAwareOAuthTokenClient), None);
        let event_repo = Arc::new(
            SqliteSystemEventRepo::new(SqlitePool::connect("sqlite::memory:").await.unwrap())
                .await
                .unwrap(),
        );
        store
            .configure_system_event_runtime(Some(Arc::new(SystemEventLogRuntime::new(
                event_repo.clone(),
            ))))
            .await
            .unwrap();

        store
            .upsert_one_time_session_account(UpsertOneTimeSessionAccountRequest {
                label: "legacy-codex-rate-limit-job-event".to_string(),
                mode: UpstreamMode::CodexOauth,
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                access_token: "legacy-rate-limit-job-event-token".to_string(),
                chatgpt_account_id: Some("acct_legacy_rate_limit_job_event".to_string()),
                enabled: Some(true),
                priority: Some(100),
                token_expires_at: Some(Utc::now() + Duration::hours(2)),
                chatgpt_plan_type: Some("free".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("upsert one-time codex account");

        let created = store
            .create_oauth_rate_limit_refresh_job()
            .await
            .expect("create rate-limit refresh job");
        store
            .run_oauth_rate_limit_refresh_job(created.job_id)
            .await
            .expect("run rate-limit refresh job");

        let items = wait_for_system_events(&event_repo, 1).await;
        assert!(
            items.iter().any(|item| {
                item.category == codex_pool_core::events::SystemEventCategory::Patrol
                    && item.event_type == "rate_limit_refresh_batch_completed"
                    && item.source == "control_plane.rate_limit_refresh_job"
                    && item.job_id == Some(created.job_id)
                    && item
                        .payload_json
                        .as_ref()
                        .and_then(|value| value.get("processed"))
                        .and_then(|value| value.as_u64())
                        == Some(1)
                    && item
                        .payload_json
                        .as_ref()
                        .and_then(|value| value.get("success"))
                        .and_then(|value| value.as_u64())
                        == Some(1)
                    && item
                        .payload_json
                        .as_ref()
                        .and_then(|value| value.get("failed"))
                        .and_then(|value| value.as_u64())
                        == Some(0)
            }),
            "rate-limit refresh job batch event should be present"
        );
    }

    #[tokio::test]
    async fn in_memory_rate_limit_refresh_job_includes_legacy_bearer_codex_accounts() {
        let store = InMemoryStore::new_with_oauth(Arc::new(RateLimitAwareOAuthTokenClient), None);

        let account = store
            .upsert_one_time_session_account(UpsertOneTimeSessionAccountRequest {
                label: "legacy-codex-rate-limit".to_string(),
                mode: UpstreamMode::CodexOauth,
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                access_token: "legacy-access-token".to_string(),
                chatgpt_account_id: Some("acct_legacy_rate_limit".to_string()),
                enabled: Some(true),
                priority: Some(100),
                token_expires_at: Some(Utc::now() + Duration::hours(2)),
                chatgpt_plan_type: Some("free".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("upsert one-time codex account")
            .account;

        let created = store
            .create_oauth_rate_limit_refresh_job()
            .await
            .expect("create rate-limit refresh job");
        assert_eq!(created.total, 1);

        store
            .run_oauth_rate_limit_refresh_job(created.job_id)
            .await
            .expect("run rate-limit refresh job");

        let status = store
            .oauth_account_status(account.id)
            .await
            .expect("legacy bearer oauth status");
        assert_eq!(status.auth_provider, UpstreamAuthProvider::LegacyBearer);
        assert_eq!(status.rate_limits.len(), 1);
        assert_eq!(
            status.rate_limits[0].limit_id.as_deref(),
            Some("five_hours")
        );
        assert_eq!(status.chatgpt_plan_type.as_deref(), Some("plus"));
        assert!(status.rate_limits_fetched_at.is_some());
    }

    #[tokio::test]
    async fn in_memory_due_rate_limit_refresh_includes_legacy_bearer_codex_accounts() {
        let store = InMemoryStore::new_with_oauth(Arc::new(RateLimitAwareOAuthTokenClient), None);

        let account = store
            .upsert_one_time_session_account(UpsertOneTimeSessionAccountRequest {
                label: "legacy-codex-due-rate-limit".to_string(),
                mode: UpstreamMode::CodexOauth,
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                access_token: "legacy-access-token-due".to_string(),
                chatgpt_account_id: Some("acct_legacy_due_rate_limit".to_string()),
                enabled: Some(true),
                priority: Some(100),
                token_expires_at: Some(Utc::now() + Duration::hours(2)),
                chatgpt_plan_type: Some("free".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("upsert one-time codex account")
            .account;

        let refreshed = store
            .refresh_due_oauth_rate_limit_caches()
            .await
            .expect("refresh due rate-limit caches");
        assert_eq!(refreshed, 1);

        let status = store
            .oauth_account_status(account.id)
            .await
            .expect("legacy bearer oauth status");
        assert_eq!(status.auth_provider, UpstreamAuthProvider::LegacyBearer);
        assert_eq!(status.rate_limits.len(), 1);
        assert_eq!(
            status.rate_limits[0].limit_id.as_deref(),
            Some("five_hours")
        );
        assert_eq!(status.chatgpt_plan_type.as_deref(), Some("plus"));
        assert!(status.rate_limits_fetched_at.is_some());
    }

    #[tokio::test]
    async fn in_memory_seen_ok_refresh_includes_legacy_bearer_codex_accounts() {
        let store = InMemoryStore::new_with_oauth(Arc::new(RateLimitAwareOAuthTokenClient), None);

        let account = store
            .upsert_one_time_session_account(UpsertOneTimeSessionAccountRequest {
                label: "legacy-codex-seen-ok-rate-limit".to_string(),
                mode: UpstreamMode::CodexOauth,
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                access_token: "legacy-access-token-seen-ok".to_string(),
                chatgpt_account_id: Some("acct_legacy_seen_ok_rate_limit".to_string()),
                enabled: Some(true),
                priority: Some(100),
                token_expires_at: Some(Utc::now() + Duration::hours(2)),
                chatgpt_plan_type: Some("free".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("upsert one-time codex account")
            .account;

        store
            .maybe_refresh_oauth_rate_limit_cache_on_seen_ok(account.id)
            .await
            .expect("refresh rate-limit cache on seen_ok");

        let status = store
            .oauth_account_status(account.id)
            .await
            .expect("legacy bearer oauth status");
        assert_eq!(status.auth_provider, UpstreamAuthProvider::LegacyBearer);
        assert_eq!(status.rate_limits.len(), 1);
        assert_eq!(
            status.rate_limits[0].limit_id.as_deref(),
            Some("five_hours")
        );
        assert_eq!(status.chatgpt_plan_type.as_deref(), Some("plus"));
        assert!(status.rate_limits_fetched_at.is_some());
    }

    #[tokio::test]
    async fn in_memory_refresh_oauth_account_returns_status_for_legacy_bearer() {
        let store = InMemoryStore::new_with_oauth(Arc::new(RateLimitAwareOAuthTokenClient), None);

        let account = store
            .upsert_one_time_session_account(UpsertOneTimeSessionAccountRequest {
                label: "legacy-codex-refresh-noop".to_string(),
                mode: UpstreamMode::CodexOauth,
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                access_token: "legacy-access-token-refresh-noop".to_string(),
                chatgpt_account_id: Some("acct_legacy_refresh_noop".to_string()),
                enabled: Some(true),
                priority: Some(100),
                token_expires_at: Some(Utc::now() + Duration::hours(2)),
                chatgpt_plan_type: Some("free".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("upsert one-time codex account")
            .account;

        let status = store
            .refresh_oauth_account(account.id)
            .await
            .expect("legacy bearer refresh should no-op");
        assert_eq!(status.auth_provider, UpstreamAuthProvider::LegacyBearer);
        assert_eq!(
            status.credential_kind,
            Some(SessionCredentialKind::OneTimeAccessToken)
        );
        assert!(!status.has_refresh_credential);
        assert_eq!(status.last_refresh_status, OAuthRefreshStatus::Never);
    }

    #[tokio::test]
    async fn in_memory_live_result_token_invalidated_escalates_oauth_account_on_third_strike() {
        let cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([27_u8; 32]),
        )
        .unwrap();
        let store = InMemoryStore::new_with_oauth(Arc::new(StaticOAuthTokenClient), Some(cipher));

        let account = store
            .import_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-token-invalidated-strikes".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-token-invalidated-strikes".to_string(),
                fallback_access_token: None,
                fallback_token_expires_at: None,
                chatgpt_account_id: None,
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: None,
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("import oauth account");

        for attempt in 0..2 {
            let accepted = store
                .record_upstream_account_live_result(
                    account.id,
                    Utc::now() + Duration::minutes(attempt),
                    OAuthLiveResultStatus::Failed,
                    OAuthLiveResultSource::Passive,
                    Some(401),
                    Some("token_invalidated".to_string()),
                    Some("token invalidated".to_string()),
                )
                .await
                .expect("record token invalidated live-result");
            assert!(accepted);

            let status = store
                .oauth_account_status(account.id)
                .await
                .expect("oauth status after strike");
            assert_eq!(status.pool_state, OAuthAccountPoolState::Quarantine);
            assert_eq!(
                status.quarantine_reason.as_deref(),
                Some("token_invalidated")
            );
        }

        let accepted = store
            .record_upstream_account_live_result(
                account.id,
                Utc::now() + Duration::minutes(2),
                OAuthLiveResultStatus::Failed,
                OAuthLiveResultSource::Passive,
                Some(401),
                Some("token_invalidated".to_string()),
                Some("token invalidated".to_string()),
            )
            .await
            .expect("record third token invalidated live-result");
        assert!(accepted);

        let status = store
            .oauth_account_status(account.id)
            .await
            .expect("oauth status after third strike");
        assert_eq!(status.pool_state, OAuthAccountPoolState::PendingPurge);
        assert_eq!(
            status.pending_purge_reason.as_deref(),
            Some("token_invalidated")
        );
        assert!(status.pending_purge_at.is_some());
    }

    #[tokio::test]
    async fn in_memory_oauth_status_exposes_team_workspace_name() {
        let cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([10_u8; 32]),
        )
        .unwrap();
        let store =
            InMemoryStore::new_with_oauth(Arc::new(SharedAccountIdOAuthTokenClient), Some(cipher));

        let account = store
            .import_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-team-workspace".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-shared-workspace-a".to_string(),
                fallback_access_token: None,
                fallback_token_expires_at: None,
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
                fallback_access_token: None,
                fallback_token_expires_at: None,
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
    async fn in_memory_observed_rate_limits_disable_effective_account_immediately() {
        let cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([12_u8; 32]),
        )
        .unwrap();
        let store = InMemoryStore::new_with_oauth(Arc::new(StaticOAuthTokenClient), Some(cipher));

        let account = store
            .import_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-observed-rate-limit".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-observed-rate-limit".to_string(),
                fallback_access_token: None,
                fallback_token_expires_at: None,
                chatgpt_account_id: None,
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: None,
                source_type: Some("codex".to_string()),
            })
            .await
            .unwrap();

        let observed_at = Utc::now();
        store
            .update_oauth_rate_limit_cache_from_observation(
                account.id,
                vec![OAuthRateLimitSnapshot {
                    limit_id: Some("codex".to_string()),
                    limit_name: Some("Codex".to_string()),
                    primary: Some(OAuthRateLimitWindow {
                        used_percent: 100.0,
                        window_minutes: Some(300),
                        resets_at: Some(observed_at + Duration::minutes(30)),
                    }),
                    secondary: None,
                }],
                observed_at,
            )
            .await
            .unwrap();

        let status = store.oauth_account_status(account.id).await.unwrap();
        assert_eq!(status.rate_limits.len(), 1);
        assert_eq!(status.rate_limits[0].limit_id.as_deref(), Some("codex"));
        assert!(status.rate_limits_fetched_at.is_some());
        assert_eq!(
            status.rate_limits_last_error_code.as_deref(),
            Some("primary_window_exhausted")
        );
        assert!(status
            .rate_limits_last_error
            .as_deref()
            .is_some_and(|message| message.contains("primary")));
        assert!(status
            .rate_limits_expires_at
            .is_some_and(|expires_at| expires_at >= observed_at + Duration::minutes(30)));
        assert!(!status.effective_enabled);
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
                fallback_access_token: None,
                fallback_token_expires_at: None,
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
        let store =
            InMemoryStore::new_with_oauth(Arc::new(SharedAccountIdOAuthTokenClient), Some(cipher));

        let first = store
            .upsert_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-shared-a".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-shared-workspace-a-1".to_string(),
                fallback_access_token: None,
                fallback_token_expires_at: None,
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
                fallback_access_token: None,
                fallback_token_expires_at: None,
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
        let store =
            InMemoryStore::new_with_oauth(Arc::new(SharedAccountIdOAuthTokenClient), Some(cipher));

        let first = store
            .upsert_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-shared-a".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-shared-workspace-a".to_string(),
                fallback_access_token: None,
                fallback_token_expires_at: None,
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
                fallback_access_token: None,
                fallback_token_expires_at: None,
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
    async fn in_memory_one_time_session_upsert_normalizes_codex_base_url() {
        let store = InMemoryStore::default();

        let account = store
            .upsert_one_time_session_account(UpsertOneTimeSessionAccountRequest {
                label: "codex-one-time".to_string(),
                mode: UpstreamMode::CodexOauth,
                base_url: "https://chatgpt.com".to_string(),
                access_token: "one-time-token".to_string(),
                chatgpt_account_id: Some("acct-one-time".to_string()),
                enabled: Some(true),
                priority: Some(100),
                token_expires_at: Some(Utc::now() + Duration::hours(1)),
                chatgpt_plan_type: Some("free".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .unwrap()
            .account;

        assert_eq!(account.base_url, "https://chatgpt.com/backend-api/codex");
    }

    #[tokio::test]
    async fn in_memory_snapshot_prefers_manual_model_routes_over_supported_model_probe_results() {
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
                name: "gpt54-paid".to_string(),
                family: "gpt-5.4".to_string(),
                exact_models: vec!["gpt-5.4".to_string()],
                model_prefixes: Vec::new(),
                fallback_profile_ids: vec![paid_profile.id],
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
                vec!["gpt-5.2-codex".to_string()],
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
        assert_eq!(
            gpt54.fallback_segments[0].account_ids,
            vec![paid_account.id]
        );
    }

    #[tokio::test]
    async fn in_memory_queue_oauth_refresh_token_marks_ready_inventory_without_refresh() {
        let cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([21_u8; 32]),
        )
        .unwrap();
        let refresh_calls = Arc::new(AtomicUsize::new(0));
        let store = InMemoryStore::new_with_oauth(
            Arc::new(AdmissionProbeOAuthTokenClient {
                refresh_calls: refresh_calls.clone(),
                rate_limits: vec![OAuthRateLimitSnapshot {
                    limit_id: Some("five_hours".to_string()),
                    limit_name: Some("5 hours".to_string()),
                    primary: Some(OAuthRateLimitWindow {
                        used_percent: 25.0,
                        window_minutes: Some(300),
                        resets_at: Some(Utc::now() + Duration::minutes(30)),
                    }),
                    secondary: None,
                }],
            }),
            Some(cipher),
        );

        let fallback_token_expires_at = Utc::now() + Duration::hours(2);
        let created = store
            .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-admission-ready".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-admission-ready".to_string(),
                fallback_access_token: Some("ak-ready".to_string()),
                fallback_token_expires_at: Some(fallback_token_expires_at),
                chatgpt_account_id: Some("acct_admission_ready".to_string()),
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: Some("pro".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("queue oauth refresh token");

        assert!(created);
        assert_eq!(refresh_calls.load(Ordering::SeqCst), 0);
        assert!(
            store
                .list_upstream_accounts()
                .await
                .expect("list runtime accounts")
                .is_empty(),
            "inventory-ready items should remain in vault before activation"
        );

        let summary = store
            .oauth_inventory_summary()
            .await
            .expect("inventory summary");
        assert_eq!(summary.total, 1);
        assert_eq!(summary.ready, 1);

        let records = store
            .oauth_inventory_records()
            .await
            .expect("inventory records");
        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record.vault_status, OAuthVaultRecordStatus::Ready);
        assert_eq!(
            record.admission_source.as_deref(),
            Some("fallback_access_token")
        );
        assert!(record.admission_checked_at.is_some());
        assert!(record.admission_rate_limits_expires_at.is_some());
        assert!(record.has_access_token_fallback);
    }

    #[tokio::test]
    async fn in_memory_queue_oauth_refresh_token_marks_no_quota_inventory_with_retry_after() {
        let cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([22_u8; 32]),
        )
        .unwrap();
        let refresh_calls = Arc::new(AtomicUsize::new(0));
        let store = InMemoryStore::new_with_oauth(
            Arc::new(AdmissionProbeOAuthTokenClient {
                refresh_calls: refresh_calls.clone(),
                rate_limits: vec![OAuthRateLimitSnapshot {
                    limit_id: Some("five_hours".to_string()),
                    limit_name: Some("5 hours".to_string()),
                    primary: Some(OAuthRateLimitWindow {
                        used_percent: 100.0,
                        window_minutes: Some(300),
                        resets_at: Some(Utc::now() + Duration::minutes(45)),
                    }),
                    secondary: None,
                }],
            }),
            Some(cipher),
        );

        store
            .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-admission-no-quota".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-admission-no-quota".to_string(),
                fallback_access_token: Some("ak-no-quota".to_string()),
                fallback_token_expires_at: Some(Utc::now() + Duration::hours(2)),
                chatgpt_account_id: Some("acct_admission_no_quota".to_string()),
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: Some("pro".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("queue oauth refresh token");

        assert_eq!(refresh_calls.load(Ordering::SeqCst), 0);

        let records = store
            .oauth_inventory_records()
            .await
            .expect("inventory records");
        let record = records.first().expect("inventory record");
        assert_eq!(record.vault_status, OAuthVaultRecordStatus::NoQuota);
        assert!(record.admission_retry_after.is_some());
    }

    #[tokio::test]
    async fn in_memory_queue_oauth_refresh_token_marks_needs_refresh_when_expiry_unknown() {
        let cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([23_u8; 32]),
        )
        .unwrap();
        let refresh_calls = Arc::new(AtomicUsize::new(0));
        let store = InMemoryStore::new_with_oauth(
            Arc::new(AdmissionProbeOAuthTokenClient {
                refresh_calls: refresh_calls.clone(),
                rate_limits: vec![OAuthRateLimitSnapshot {
                    limit_id: Some("five_hours".to_string()),
                    limit_name: Some("5 hours".to_string()),
                    primary: Some(OAuthRateLimitWindow {
                        used_percent: 12.0,
                        window_minutes: Some(300),
                        resets_at: Some(Utc::now() + Duration::minutes(20)),
                    }),
                    secondary: None,
                }],
            }),
            Some(cipher),
        );

        store
            .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-admission-needs-refresh".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-admission-needs-refresh".to_string(),
                fallback_access_token: Some("opaque-ak-needs-refresh".to_string()),
                fallback_token_expires_at: None,
                chatgpt_account_id: Some("acct_admission_needs_refresh".to_string()),
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: Some("pro".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("queue oauth refresh token");

        assert_eq!(refresh_calls.load(Ordering::SeqCst), 0);

        let records = store
            .oauth_inventory_records()
            .await
            .expect("inventory records");
        let record = records.first().expect("inventory record");
        assert_eq!(record.vault_status, OAuthVaultRecordStatus::NeedsRefresh);
        assert_eq!(
            record.admission_error_code.as_deref(),
            Some("expiry_unknown")
        );
    }

    #[tokio::test]
    async fn in_memory_queue_oauth_refresh_token_marks_transient_probe_failure_retryable() {
        let cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([24_u8; 32]),
        )
        .unwrap();
        let store = InMemoryStore::new_with_oauth(
            Arc::new(TransientAdmissionFailureOAuthTokenClient),
            Some(cipher),
        );

        store
            .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-admission-transient".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-admission-transient".to_string(),
                fallback_access_token: Some("ak-admission-transient".to_string()),
                fallback_token_expires_at: Some(Utc::now() + Duration::hours(2)),
                chatgpt_account_id: Some("acct_admission_transient".to_string()),
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: Some("pro".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("queue oauth refresh token");

        let records = store
            .oauth_inventory_records()
            .await
            .expect("inventory records");
        let record = records.first().expect("inventory record");
        assert_eq!(record.vault_status, OAuthVaultRecordStatus::Failed);
        assert_eq!(
            record.failure_stage,
            Some(OAuthInventoryFailureStage::AdmissionProbe)
        );
        assert_eq!(record.attempt_count, 1);
        assert_eq!(record.transient_retry_count, 1);
        assert!(record.next_retry_at.is_some());
        assert!(record.retryable);
        assert_eq!(record.terminal_reason, None);
    }

    #[tokio::test]
    async fn in_memory_oauth_vault_auth_fatal_retries_then_marks_terminal_failed() {
        let cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([25_u8; 32]),
        )
        .unwrap();
        let store = InMemoryStore::new_with_oauth(
            Arc::new(TerminalRefreshFailureOAuthTokenClient),
            Some(cipher),
        );

        store
            .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-auth-fatal".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-auth-fatal".to_string(),
                fallback_access_token: None,
                fallback_token_expires_at: None,
                chatgpt_account_id: Some("acct_auth_fatal".to_string()),
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: Some("pro".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .expect("queue oauth refresh token");

        let record_id = store
            .oauth_inventory_records()
            .await
            .expect("inventory records")
            .into_iter()
            .next()
            .expect("inventory record")
            .id;

        for attempt in 0..3 {
            store.mark_oauth_vault_activation_failed_inner(
                record_id,
                OAuthVaultRecordStatus::NeedsRefresh,
                "invalid_refresh_token".to_string(),
                "invalid refresh token".to_string(),
            );
            let record = store
                .oauth_inventory_records()
                .await
                .expect("inventory records")
                .into_iter()
                .next()
                .expect("inventory record");

            assert_eq!(
                record.failure_stage,
                Some(OAuthInventoryFailureStage::ActivationRefresh)
            );

            if attempt < 2 {
                assert_eq!(record.vault_status, OAuthVaultRecordStatus::NeedsRefresh);
                assert!(record.retryable);
                assert!(record.next_retry_at.is_some());
                assert_eq!(record.terminal_reason, None);
            } else {
                assert_eq!(record.vault_status, OAuthVaultRecordStatus::Failed);
                assert!(!record.retryable);
                assert_eq!(
                    record.terminal_reason.as_deref(),
                    Some("invalid_refresh_token")
                );
                assert!(record.next_retry_at.is_none());
            }
        }
    }

    #[tokio::test]
    async fn in_memory_activate_ready_record_requires_preflight_probe() {
        let cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([26_u8; 32]),
        )
        .unwrap();
        let store = InMemoryStore::new_with_oauth(
            Arc::new(SequentialAdmissionProbeOAuthTokenClient {
                fetch_results: Arc::new(Mutex::new(VecDeque::from(vec![
                    Ok(vec![OAuthRateLimitSnapshot {
                        limit_id: Some("five_hours".to_string()),
                        limit_name: Some("5 hours".to_string()),
                        primary: Some(OAuthRateLimitWindow {
                            used_percent: 12.0,
                            window_minutes: Some(300),
                            resets_at: Some(Utc::now() + Duration::minutes(20)),
                        }),
                        secondary: None,
                    }]),
                    Err(crate::oauth::OAuthTokenClientError::InvalidRefreshToken {
                        code: crate::oauth::OAuthRefreshErrorCode::InvalidRefreshToken,
                        message: "invalid refresh token".to_string(),
                    }),
                ]))),
            }),
            Some(cipher),
        );

        store
            .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-ready-preflight".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-ready-preflight".to_string(),
                fallback_access_token: Some("ak-ready-preflight".to_string()),
                fallback_token_expires_at: Some(Utc::now() + Duration::hours(2)),
                chatgpt_account_id: Some("acct_ready_preflight".to_string()),
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: Some("pro".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .unwrap();

        let summary_before = store.oauth_inventory_summary().await.unwrap();
        assert_eq!(summary_before.ready, 1);

        let activated = store.activate_oauth_refresh_token_vault().await.unwrap();
        assert_eq!(activated, 0, "fatal preflight probe must block activation");
        assert!(
            store.list_upstream_accounts().await.unwrap().is_empty(),
            "ready inventory should not materialize into runtime when preflight probe fails"
        );

        let record = store
            .oauth_inventory_records()
            .await
            .unwrap()
            .into_iter()
            .next()
            .expect("inventory record");
        assert_eq!(record.vault_status, OAuthVaultRecordStatus::Failed);
        assert_eq!(
            record.terminal_reason.as_deref(),
            Some("invalid_refresh_token")
        );
    }

    #[tokio::test]
    async fn in_memory_active_patrol_rechecks_stale_routable_accounts() {
        let cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([27_u8; 32]),
        )
        .unwrap();
        let store = InMemoryStore::new_with_oauth(
            Arc::new(SequentialAdmissionProbeOAuthTokenClient {
                fetch_results: Arc::new(Mutex::new(VecDeque::from(vec![
                    Ok(vec![OAuthRateLimitSnapshot {
                        limit_id: Some("five_hours".to_string()),
                        limit_name: Some("5 hours".to_string()),
                        primary: Some(OAuthRateLimitWindow {
                            used_percent: 20.0,
                            window_minutes: Some(300),
                            resets_at: Some(Utc::now() + Duration::minutes(40)),
                        }),
                        secondary: None,
                    }]),
                    Ok(vec![OAuthRateLimitSnapshot {
                        limit_id: Some("five_hours".to_string()),
                        limit_name: Some("5 hours".to_string()),
                        primary: Some(OAuthRateLimitWindow {
                            used_percent: 24.0,
                            window_minutes: Some(300),
                            resets_at: Some(Utc::now() + Duration::minutes(35)),
                        }),
                        secondary: None,
                    }]),
                    Err(crate::oauth::OAuthTokenClientError::Upstream {
                        code: crate::oauth::OAuthRefreshErrorCode::UnauthorizedClient,
                        message: "account_deactivated".to_string(),
                    }),
                ]))),
            }),
            Some(cipher),
        );

        let account = store
            .queue_oauth_refresh_token(ImportOAuthRefreshTokenRequest {
                label: "oauth-patrol-stale".to_string(),
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                refresh_token: "rt-patrol-stale".to_string(),
                fallback_access_token: Some("ak-patrol-stale".to_string()),
                fallback_token_expires_at: Some(Utc::now() + Duration::hours(2)),
                chatgpt_account_id: Some("acct_patrol_stale".to_string()),
                mode: Some(UpstreamMode::CodexOauth),
                enabled: Some(true),
                priority: Some(100),
                chatgpt_plan_type: Some("pro".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .unwrap();

        assert!(account);
        let activated = store.activate_oauth_refresh_token_vault().await.unwrap();
        assert_eq!(activated, 1);
        let account = store
            .list_upstream_accounts()
            .await
            .unwrap()
            .into_iter()
            .next()
            .expect("runtime account");

        {
            let mut states = store.account_health_states.write().unwrap();
            let state = states.entry(account.id).or_default();
            state.last_probe_at = Some(Utc::now() - Duration::minutes(30));
            state.last_probe_outcome = Some(crate::contracts::AccountProbeOutcome::Ok);
        }

        let patrolled = store.patrol_active_oauth_accounts().await.unwrap();
        assert_eq!(patrolled, 1);

        let pool_record = store.account_pool_record(account.id).await.unwrap();
        assert_eq!(
            pool_record.operator_state,
            crate::contracts::AccountPoolOperatorState::PendingDelete
        );
        assert_eq!(
            pool_record.reason_code.as_deref(),
            Some("account_deactivated")
        );
    }

    #[tokio::test]
    async fn in_memory_active_patrol_rechecks_stale_legacy_bearer_codex_accounts() {
        let cipher = CredentialCipher::from_base64_key(
            &base64::engine::general_purpose::STANDARD.encode([28_u8; 32]),
        )
        .unwrap();
        let store =
            InMemoryStore::new_with_oauth(Arc::new(RateLimitAwareOAuthTokenClient), Some(cipher));

        let account = store
            .upsert_one_time_session_account(UpsertOneTimeSessionAccountRequest {
                label: "legacy-bearer-patrol-stale".to_string(),
                mode: UpstreamMode::CodexOauth,
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                access_token: "legacy-bearer-patrol-token".to_string(),
                chatgpt_account_id: Some("acct_legacy_patrol".to_string()),
                enabled: Some(true),
                priority: Some(100),
                token_expires_at: Some(Utc::now() + Duration::hours(2)),
                chatgpt_plan_type: Some("pro".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .unwrap()
            .account;

        {
            let mut states = store.account_health_states.write().unwrap();
            let state = states.entry(account.id).or_default();
            state.last_probe_at = Some(Utc::now() - Duration::minutes(30));
            state.last_probe_outcome = Some(crate::contracts::AccountProbeOutcome::Ok);
        }

        let patrolled = store.patrol_active_oauth_accounts().await.unwrap();
        assert_eq!(patrolled, 1);

        let pool_record = store.account_pool_record(account.id).await.unwrap();
        assert_eq!(
            pool_record.operator_state,
            crate::contracts::AccountPoolOperatorState::Routable
        );
        assert_eq!(
            pool_record.health_freshness,
            crate::contracts::AccountHealthFreshness::Fresh
        );
        assert!(pool_record.last_probe_at.is_some());
    }

    #[tokio::test]
    async fn in_memory_active_patrol_labels_legacy_bearer_auth_failure_as_token_invalidated() {
        let store =
            InMemoryStore::new_with_oauth(Arc::new(LegacyBearerAuthFailureOAuthTokenClient), None);

        let account = store
            .upsert_one_time_session_account(UpsertOneTimeSessionAccountRequest {
                label: "legacy-bearer-auth-failure".to_string(),
                mode: UpstreamMode::CodexOauth,
                base_url: "https://chatgpt.com/backend-api/codex".to_string(),
                access_token: "legacy-bearer-invalid-token".to_string(),
                chatgpt_account_id: Some("acct_legacy_invalid".to_string()),
                enabled: Some(true),
                priority: Some(100),
                token_expires_at: Some(Utc::now() + Duration::hours(2)),
                chatgpt_plan_type: Some("pro".to_string()),
                source_type: Some("codex".to_string()),
            })
            .await
            .unwrap()
            .account;

        {
            let mut states = store.account_health_states.write().unwrap();
            let state = states.entry(account.id).or_default();
            state.last_probe_at = Some(Utc::now() - Duration::minutes(30));
            state.last_probe_outcome = Some(crate::contracts::AccountProbeOutcome::Ok);
        }

        let patrolled = store.patrol_active_oauth_accounts().await.unwrap();
        assert_eq!(patrolled, 1);

        let status = store.oauth_account_status(account.id).await.unwrap();
        assert_eq!(
            status.rate_limits_last_error_code.as_deref(),
            Some("token_invalidated")
        );
        assert_eq!(
            status.pending_purge_reason.as_deref(),
            Some("token_invalidated")
        );

        let pool_record = store.account_pool_record(account.id).await.unwrap();
        assert_eq!(
            pool_record.operator_state,
            crate::contracts::AccountPoolOperatorState::PendingDelete
        );
        assert_eq!(
            pool_record.reason_code.as_deref(),
            Some("token_invalidated")
        );
    }
}
