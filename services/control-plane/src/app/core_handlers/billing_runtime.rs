async fn internal_billing_precheck(
    Path(tenant_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<crate::tenant::BillingPrecheckResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    require_internal_service_token(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    tenant_auth
        .billing_precheck(tenant_id)
        .await
        .map(Json)
        .map_err(map_tenant_error)
}

async fn internal_billing_authorize(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<crate::tenant::BillingAuthorizeRequest>,
) -> Result<Json<crate::tenant::BillingAuthorizeResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    require_internal_service_token(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    tenant_auth
        .billing_authorize(req)
        .await
        .map(Json)
        .map_err(map_internal_billing_error)
}

async fn internal_billing_capture(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<crate::tenant::BillingCaptureRequest>,
) -> Result<Json<crate::tenant::BillingCaptureResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    require_internal_service_token(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    tenant_auth
        .billing_capture(req)
        .await
        .map(Json)
        .map_err(map_internal_billing_error)
}

async fn internal_billing_pricing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<crate::tenant::BillingPricingRequest>,
) -> Result<Json<crate::tenant::BillingPricingResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    require_internal_service_token(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    tenant_auth
        .billing_pricing(req)
        .await
        .map(Json)
        .map_err(map_internal_billing_error)
}

async fn internal_billing_release(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<crate::tenant::BillingReleaseRequest>,
) -> Result<Json<crate::tenant::BillingReleaseResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    require_internal_service_token(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    tenant_auth
        .billing_release(req)
        .await
        .map(Json)
        .map_err(map_internal_billing_error)
}

fn map_internal_billing_error(err: anyhow::Error) -> (StatusCode, Json<ErrorEnvelope>) {
    let lowered = err.to_string().to_ascii_lowercase();
    if lowered.contains("insufficient credits") {
        return (
            StatusCode::PAYMENT_REQUIRED,
            Json(ErrorEnvelope::new(
                "insufficient_credits",
                "insufficient credits",
            )),
        );
    }
    if lowered.contains("request_id must not be empty") {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorEnvelope::new(
                "billing_request_id_missing",
                "billing request id is required",
            )),
        );
    }
    if lowered.contains("model pricing is not configured")
        || lowered.contains("billing_model_missing")
        || lowered.contains("model must not be empty")
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorEnvelope::new(
                "billing_model_missing",
                "billing model missing",
            )),
        );
    }
    if lowered.contains("reserved_microcredits must be positive") {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorEnvelope::new(
                "billing_reserve_invalid",
                "reserved microcredits must be positive",
            )),
        );
    }
    if lowered.contains("api key group is unavailable")
        || lowered.contains("api key group is deleted")
    {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorEnvelope::new(
                "api_key_group_invalid",
                "api key group is unavailable",
            )),
        );
    }
    if lowered.contains("model is not allowed for api key group") {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorEnvelope::new(
                "model_not_allowed",
                "requested model is not allowed",
            )),
        );
    }
    if lowered.contains("billing authorization is in invalid status") {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorEnvelope::new(
                "billing_authorization_invalid_status",
                "billing authorization is in invalid status",
            )),
        );
    }
    if lowered.contains("authorization not found") {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorEnvelope::new(
                "billing_authorization_not_found",
                "billing authorization not found",
            )),
        );
    }
    tracing::warn!(
        error = %err,
        "falling back to generic tenant error mapping for internal billing error"
    );
    map_tenant_error(err)
}

async fn admin_system_state(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminSystemStateResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    let config = {
        state
            .runtime_config
            .read()
            .expect("runtime_config lock poisoned")
            .clone()
    };

    let accounts = state
        .store
        .list_upstream_accounts()
        .await
        .map_err(internal_error)?;
    let tenants = state.store.list_tenants().await.map_err(internal_error)?;
    let api_keys = state.store.list_api_keys().await.map_err(internal_error)?;

    let (data_plane_debug, data_plane_error) =
        fetch_data_plane_debug_state(&config.data_plane_base_url).await;

    let now = Utc::now();
    let counts = AdminSystemCounts {
        total_accounts: accounts.len(),
        enabled_accounts: accounts.iter().filter(|account| account.enabled).count(),
        oauth_accounts: accounts
            .iter()
            .filter(|account| {
                matches!(
                    account.mode,
                    codex_pool_core::model::UpstreamMode::ChatGptSession
                        | codex_pool_core::model::UpstreamMode::CodexOauth
                )
            })
            .count(),
        api_keys: api_keys.len(),
        tenants: tenants.len(),
    };

    push_admin_log(
        &state,
        "info",
        "admin.system.state",
        format!("queried system state: {} accounts", counts.total_accounts),
    );

    Ok(Json(AdminSystemStateResponse {
        generated_at: now,
        started_at: state.started_at,
        uptime_sec: now.signed_duration_since(state.started_at).num_seconds(),
        usage_repo_available: state.usage_repo.is_some(),
        config,
        counts,
        control_plane_debug: crate::tenant::billing_reconcile_runtime_snapshot(),
        data_plane_debug,
        data_plane_error,
    }))
}

async fn get_admin_runtime_config(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<RuntimeConfigSnapshot>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    let config = {
        state
            .runtime_config
            .read()
            .expect("runtime_config lock poisoned")
            .clone()
    };
    Ok(Json(config))
}

async fn update_admin_runtime_config(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RuntimeConfigUpdateRequest>,
) -> Result<Json<RuntimeConfigSnapshot>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    let mut config = state
        .runtime_config
        .write()
        .expect("runtime_config lock poisoned");
    if let Some(value) = req
        .data_plane_base_url
        .filter(|value| !value.trim().is_empty())
    {
        config.data_plane_base_url = value;
    }
    if let Some(value) = req
        .auth_validate_url
        .filter(|value| !value.trim().is_empty())
    {
        config.auth_validate_url = value;
    }
    if let Some(value) = req.oauth_refresh_enabled {
        config.oauth_refresh_enabled = value;
    }
    if let Some(value) = req.oauth_refresh_interval_sec {
        config.oauth_refresh_interval_sec = value.max(1);
    }
    if let Some(value) = req.notes {
        config.notes = if value.trim().is_empty() {
            None
        } else {
            Some(value)
        };
    }
    let updated = config.clone();
    drop(config);

    push_admin_log(
        &state,
        "warn",
        "admin.config.update",
        "updated runtime config snapshot in-memory",
    );

    Ok(Json(updated))
}

async fn list_admin_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminLogsQuery>,
) -> Result<Json<AdminLogsResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    let limit = query.limit.unwrap_or(200).min(ADMIN_LOG_CAPACITY);
    let logs = state.admin_logs.read().expect("admin_logs lock poisoned");
    let items = logs.iter().rev().take(limit).cloned().collect::<Vec<_>>();
    Ok(Json(AdminLogsResponse { items }))
}

async fn list_admin_proxies(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AdminProxyItem>>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    let proxies = state
        .admin_proxies
        .read()
        .expect("admin_proxies lock poisoned")
        .clone();
    Ok(Json(proxies))
}

async fn test_admin_proxies(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminProxyTestRequest>,
) -> Result<Json<AdminProxyTestResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    let requested_id = query.proxy_id;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|err| internal_error(err.into()))?;

    let mut tested = 0_usize;
    let mut next_proxies = {
        state
            .admin_proxies
            .read()
            .expect("admin_proxies lock poisoned")
            .clone()
    };
    for proxy in next_proxies.iter_mut() {
        if let Some(proxy_id) = requested_id.as_deref() {
            if proxy.id != proxy_id {
                continue;
            }
        }
        tested = tested.saturating_add(1);

        if !proxy.enabled {
            proxy.last_test_status = Some("skipped".to_string());
            proxy.last_error = Some("proxy disabled".to_string());
            proxy.last_latency_ms = None;
            proxy.updated_at = Utc::now();
            continue;
        }

        let test_url = format!("{}/health", proxy.base_url.trim_end_matches('/'));
        let started = std::time::Instant::now();
        match client.get(&test_url).send().await {
            Ok(response) if response.status().is_success() => {
                proxy.last_test_status = Some("ok".to_string());
                proxy.last_error = None;
                proxy.last_latency_ms = Some(started.elapsed().as_millis() as u64);
                proxy.updated_at = Utc::now();
            }
            Ok(response) => {
                proxy.last_test_status = Some("error".to_string());
                proxy.last_error = Some(format!("http {}", response.status()));
                proxy.last_latency_ms = Some(started.elapsed().as_millis() as u64);
                proxy.updated_at = Utc::now();
            }
            Err(err) => {
                proxy.last_test_status = Some("error".to_string());
                proxy.last_error = Some(err.to_string());
                proxy.last_latency_ms = Some(started.elapsed().as_millis() as u64);
                proxy.updated_at = Utc::now();
            }
        }
    }

    {
        let mut proxies = state
            .admin_proxies
            .write()
            .expect("admin_proxies lock poisoned");
        *proxies = next_proxies.clone();
    }
    let results = next_proxies;

    push_admin_log(
        &state,
        "info",
        "admin.proxies.test",
        format!("tested {tested} proxy nodes"),
    );

    Ok(Json(AdminProxyTestResponse { tested, results }))
}

async fn list_admin_api_keys(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ApiKey>>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    state
        .store
        .list_api_keys()
        .await
        .map(Json)
        .map_err(internal_error)
}

async fn create_admin_api_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<AdminKeyCreateRequest>,
) -> Result<Json<codex_pool_core::api::CreateApiKeyResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    let tenant_id = if let Some(tenant_id) = req.tenant_id {
        tenant_id
    } else if let Some(tenant_name) = req.tenant_name.filter(|value| !value.trim().is_empty()) {
        state
            .store
            .create_tenant(CreateTenantRequest { name: tenant_name })
            .await
            .map_err(internal_error)?
            .id
    } else {
        let tenants = state.store.list_tenants().await.map_err(internal_error)?;
        if let Some(existing) = tenants.into_iter().find(|tenant| tenant.name == "default") {
            existing.id
        } else {
            state
                .store
                .create_tenant(CreateTenantRequest {
                    name: "default".to_string(),
                })
                .await
                .map_err(internal_error)?
                .id
        }
    };

    let response = state
        .store
        .create_api_key(CreateApiKeyRequest {
            tenant_id,
            name: req.name,
        })
        .await
        .map_err(internal_error)?;

    push_admin_log(
        &state,
        "info",
        "admin.keys.create",
        format!("created api key {}", response.record.id),
    );

    Ok(Json(response))
}

async fn update_admin_api_key_enabled(
    Path(key_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<AdminKeyPatchRequest>,
) -> Result<Json<ApiKey>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    let key = state
        .store
        .set_api_key_enabled(key_id, req.enabled)
        .await
        .map_err(internal_error)?;
    push_admin_log(
        &state,
        "warn",
        "admin.keys.patch",
        format!("set api key {} enabled={}", key_id, req.enabled),
    );
    Ok(Json(key))
}

async fn get_admin_usage_overview(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AdminUsageOverviewQuery>,
) -> Result<Json<AdminUsageOverviewResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    validate_usage_range(query.start_ts, query.end_ts)?;
    let usage_repo = state
        .usage_repo
        .as_ref()
        .ok_or_else(usage_repo_unavailable_error)?;
    let limit = query.limit.min(MAX_USAGE_LEADERBOARD_LIMIT);

    let (summary, tenants, accounts, api_keys) = tokio::try_join!(
        usage_repo.query_summary(query.start_ts, query.end_ts, None, None, None),
        usage_repo.query_tenant_leaderboard(query.start_ts, query.end_ts, limit, None),
        usage_repo.query_account_leaderboard(query.start_ts, query.end_ts, limit, None),
        usage_repo.query_api_key_leaderboard(query.start_ts, query.end_ts, limit, None, None),
    )
    .map_err(internal_error)?;

    Ok(Json(AdminUsageOverviewResponse {
        start_ts: query.start_ts,
        end_ts: query.end_ts,
        summary,
        tenants,
        accounts,
        api_keys,
    }))
}

async fn list_admin_models(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AdminModelsResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    let official_catalog = tenant_auth
        .admin_list_openai_model_catalog()
        .await
        .map_err(map_tenant_error)?;
    let pricing_overrides = tenant_auth
        .admin_list_model_pricing()
        .await
        .map_err(map_tenant_error)?;
    Ok(Json(
        build_admin_models_response(&state, official_catalog, pricing_overrides).await,
    ))
}

async fn probe_admin_models(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<AdminModelsProbeRequest>,
) -> Result<Json<AdminModelsResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    run_model_probe_cycle(&state, req.models, req.force, "manual")
        .await
        .map_err(|err| {
            tracing::warn!(error = %err, "manual model probe failed");
            let code = if err.to_string().contains("sync OpenAI catalog first") {
                "official_catalog_missing"
            } else {
                "model_probe_failed"
            };
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorEnvelope::new(code, "model probe failed")),
            )
        })?;
    let official_catalog = tenant_auth
        .admin_list_openai_model_catalog()
        .await
        .map_err(map_tenant_error)?;
    let pricing_overrides = tenant_auth
        .admin_list_model_pricing()
        .await
        .map_err(map_tenant_error)?;
    Ok(Json(
        build_admin_models_response(&state, official_catalog, pricing_overrides).await,
    ))
}

async fn sync_openai_admin_models_catalog(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<crate::tenant::OpenAiModelsSyncResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let principal = require_admin_principal(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    let response = tenant_auth
        .admin_sync_openai_models_catalog()
        .await
        .map_err(|err| {
            tracing::warn!(error = %err, "openai catalog sync failed");
            {
                let mut last_error = state
                    .model_catalog_last_error
                    .write()
                    .expect("model_catalog_last_error lock poisoned");
                *last_error = Some(err.to_string());
            }
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorEnvelope::new(
                    "openai_catalog_sync_failed",
                    "openai catalog sync failed",
                )),
            )
        })?;
    {
        let mut last_error = state
            .model_catalog_last_error
            .write()
            .expect("model_catalog_last_error lock poisoned");
        *last_error = None;
    }
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: None,
            action: "admin.models.sync_openai".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("openai_models_catalog".to_string()),
            target_id: None,
            payload_json: json!({
                "models_total": response.models_total,
                "created_or_updated": response.created_or_updated,
                "deleted_catalog_rows": response.deleted_catalog_rows,
                "cleared_custom_entities": response.cleared_custom_entities,
                "cleared_billing_rules": response.cleared_billing_rules,
                "deleted_legacy_pricing_rows": response.deleted_legacy_pricing_rows,
                "synced_at": response.synced_at,
            }),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(Json(response))
}

#[cfg(test)]
mod billing_runtime_tests {
    use super::*;

    #[test]
    fn map_internal_billing_error_maps_model_missing_precisely() {
        let (status, Json(envelope)) =
            map_internal_billing_error(anyhow::anyhow!("model must not be empty"));

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(envelope.error.code, "billing_model_missing");
        assert_eq!(envelope.error.message, "billing model missing");
    }

    #[test]
    fn map_internal_billing_error_maps_invalid_authorization_status_precisely() {
        let (status, Json(envelope)) = map_internal_billing_error(anyhow::anyhow!(
            "billing authorization is in invalid status: released"
        ));

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(envelope.error.code, "billing_authorization_invalid_status");
        assert_eq!(
            envelope.error.message,
            "billing authorization is in invalid status"
        );
    }
}
