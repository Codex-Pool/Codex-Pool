async fn internal_debug_auth_cache(
    State(state): State<Arc<AppState>>,
) -> axum::Json<InternalDebugAuthCacheResponse> {
    let (cached_principal_total, negative_cached_token_total) = match state.auth_validator.as_ref()
    {
        Some(auth_validator) => (
            auth_validator.cached_principal_total().await,
            auth_validator.cached_negative_total().await,
        ),
        None => (0, 0),
    };

    axum::Json(InternalDebugAuthCacheResponse {
        auth_validator_enabled: state.auth_validator.is_some(),
        cached_principal_total,
        negative_cached_token_total,
        auth_fail_open: state.auth_fail_open,
        allowlist_api_key_total: state.allowed_api_keys.len(),
    })
}

async fn internal_debug_clear_auth_cache(
    State(state): State<Arc<AppState>>,
) -> axum::Json<InternalDebugAuthCacheClearResponse> {
    let (auth_validator_enabled, cleared, cached_principal_total) =
        match state.auth_validator.as_ref() {
            Some(auth_validator) => {
                let cleared = auth_validator.clear_cache().await;
                let cached_principal_total = auth_validator.cached_principal_total().await;
                (true, cleared, cached_principal_total)
            }
            None => (false, 0, 0),
        };

    axum::Json(InternalDebugAuthCacheClearResponse {
        auth_validator_enabled,
        cleared,
        cached_principal_total,
    })
}

async fn internal_debug_auth_cache_stats(
    State(state): State<Arc<AppState>>,
) -> axum::Json<InternalDebugAuthCacheStatsResponse> {
    let (auth_validator_enabled, cached_principal_total, stats) =
        match state.auth_validator.as_ref() {
            Some(auth_validator) => (
                true,
                auth_validator.cached_principal_total().await,
                auth_validator.cache_stats(),
            ),
            None => (false, 0, AuthCacheStatsSnapshot::default()),
        };

    axum::Json(InternalDebugAuthCacheStatsResponse {
        auth_validator_enabled,
        cached_principal_total,
        cache_hit_count: stats.cache_hit_count,
        cache_miss_count: stats.cache_miss_count,
        remote_validate_count: stats.remote_validate_count,
        negative_cache_hit_count: stats.negative_cache_hit_count,
        negative_cache_store_count: stats.negative_cache_store_count,
    })
}

async fn internal_debug_reset_auth_cache_stats(
    State(state): State<Arc<AppState>>,
) -> axum::Json<InternalDebugAuthCacheStatsResetResponse> {
    let (auth_validator_enabled, before, after) = match state.auth_validator.as_ref() {
        Some(auth_validator) => {
            let before = auth_validator.reset_cache_stats();
            let after = auth_validator.cache_stats();
            (true, before, after)
        }
        None => (
            false,
            AuthCacheStatsSnapshot::default(),
            AuthCacheStatsSnapshot::default(),
        ),
    };

    axum::Json(InternalDebugAuthCacheStatsResetResponse {
        auth_validator_enabled,
        cache_hit_count_before: before.cache_hit_count,
        cache_miss_count_before: before.cache_miss_count,
        remote_validate_count_before: before.remote_validate_count,
        negative_cache_hit_count_before: before.negative_cache_hit_count,
        negative_cache_store_count_before: before.negative_cache_store_count,
        cache_hit_count_after: after.cache_hit_count,
        cache_miss_count_after: after.cache_miss_count,
        remote_validate_count_after: after.remote_validate_count,
        negative_cache_hit_count_after: after.negative_cache_hit_count,
        negative_cache_store_count_after: after.negative_cache_store_count,
    })
}

async fn internal_debug_lookup_auth_cache(
    State(state): State<Arc<AppState>>,
    request: Result<axum::Json<InternalDebugAuthCacheLookupRequest>, JsonRejection>,
) -> Result<axum::Json<InternalDebugAuthCacheLookupResponse>, (StatusCode, axum::Json<ErrorEnvelope>)>
{
    let axum::Json(request) = request.map_err(|_| invalid_auth_cache_lookup_request())?;
    if request.token.trim().is_empty() {
        return Err(invalid_auth_cache_lookup_request());
    }

    let Some(auth_validator) = state.auth_validator.as_ref() else {
        return Ok(axum::Json(InternalDebugAuthCacheLookupResponse {
            auth_validator_enabled: false,
            hit: false,
            cached_negative: false,
            lookup_status: "validator_disabled".to_string(),
            tenant_id: None,
            api_key_id: None,
            enabled: None,
            cached_principal_total: 0,
        }));
    };

    let cache_lookup_result = auth_validator.lookup_cached_token(&request.token).await;
    let cached_principal_total = auth_validator.cached_principal_total().await;

    Ok(axum::Json(match cache_lookup_result {
        AuthCacheLookupResult::PositiveHit(principal) => InternalDebugAuthCacheLookupResponse {
            auth_validator_enabled: true,
            hit: true,
            cached_negative: false,
            lookup_status: "positive_hit".to_string(),
            tenant_id: principal.tenant_id,
            api_key_id: principal.api_key_id,
            enabled: Some(principal.enabled),
            cached_principal_total,
        },
        AuthCacheLookupResult::NegativeHit => InternalDebugAuthCacheLookupResponse {
            auth_validator_enabled: true,
            hit: false,
            cached_negative: true,
            lookup_status: "negative_hit".to_string(),
            tenant_id: None,
            api_key_id: None,
            enabled: None,
            cached_principal_total,
        },
        AuthCacheLookupResult::Miss => InternalDebugAuthCacheLookupResponse {
            auth_validator_enabled: true,
            hit: false,
            cached_negative: false,
            lookup_status: "miss".to_string(),
            tenant_id: None,
            api_key_id: None,
            enabled: None,
            cached_principal_total,
        },
    }))
}

async fn internal_debug_evict_auth_cache(
    State(state): State<Arc<AppState>>,
    request: Result<axum::Json<InternalDebugAuthCacheEvictRequest>, JsonRejection>,
) -> Result<axum::Json<InternalDebugAuthCacheEvictResponse>, (StatusCode, axum::Json<ErrorEnvelope>)>
{
    let axum::Json(request) = request.map_err(|_| invalid_auth_cache_evict_request())?;
    if request.token.trim().is_empty() {
        return Err(invalid_auth_cache_evict_request());
    }

    let (
        auth_validator_enabled,
        evicted,
        positive_evicted,
        negative_evicted,
        cached_principal_total,
    ) = match state.auth_validator.as_ref() {
        Some(auth_validator) => {
            let evict_result = auth_validator.evict_token(&request.token).await;
            let cached_principal_total = auth_validator.cached_principal_total().await;
            (
                true,
                evict_result.evicted(),
                evict_result.positive_evicted,
                evict_result.negative_evicted,
                cached_principal_total,
            )
        }
        None => (false, false, false, false, 0),
    };

    Ok(axum::Json(InternalDebugAuthCacheEvictResponse {
        auth_validator_enabled,
        evicted,
        positive_evicted,
        negative_evicted,
        cached_principal_total,
    }))
}

async fn internal_debug_accounts(
    State(state): State<Arc<AppState>>,
) -> axum::Json<InternalDebugAccountsResponse> {
    axum::Json(InternalDebugAccountsResponse {
        accounts: state
            .router
            .list_account_diagnostics()
            .into_iter()
            .map(internal_debug_account_from_diagnostics)
            .collect(),
    })
}

async fn internal_debug_unhealthy_accounts(
    State(state): State<Arc<AppState>>,
) -> axum::Json<InternalDebugAccountsResponse> {
    axum::Json(InternalDebugAccountsResponse {
        accounts: state
            .router
            .list_account_diagnostics()
            .into_iter()
            .filter(|account| account.temporarily_unhealthy)
            .map(internal_debug_account_from_diagnostics)
            .collect(),
    })
}

async fn internal_debug_account_by_id(
    Path(account_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<axum::Json<InternalDebugAccount>, (StatusCode, axum::Json<ErrorEnvelope>)> {
    let diagnostics = state
        .router
        .account_diagnostics(account_id)
        .ok_or_else(|| internal_debug_account_not_found(account_id))?;

    Ok(axum::Json(internal_debug_account_from_diagnostics(
        diagnostics,
    )))
}

async fn internal_debug_clear_unhealthy(
    State(state): State<Arc<AppState>>,
) -> axum::Json<InternalDebugClearUnhealthyResponse> {
    axum::Json(InternalDebugClearUnhealthyResponse {
        cleared: state.router.clear_all_unhealthy(),
    })
}

async fn internal_debug_mark_unhealthy(
    Path(account_id): Path<Uuid>,
    Query(query): Query<InternalDebugMarkUnhealthyQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<axum::Json<InternalDebugAccount>, (StatusCode, axum::Json<ErrorEnvelope>)> {
    if state.router.account_diagnostics(account_id).is_none() {
        return Err(internal_debug_account_not_found(account_id));
    }

    let ttl_sec = query
        .ttl_sec
        .unwrap_or(DEFAULT_MARK_UNHEALTHY_TTL_SEC)
        .min(MAX_MARK_UNHEALTHY_TTL_SEC);
    state
        .router
        .mark_unhealthy(account_id, Duration::from_secs(ttl_sec));

    let diagnostics = state
        .router
        .account_diagnostics(account_id)
        .ok_or_else(|| internal_debug_account_not_found(account_id))?;

    Ok(axum::Json(internal_debug_account_from_diagnostics(
        diagnostics,
    )))
}

async fn internal_debug_mark_healthy(
    Path(account_id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<axum::Json<InternalDebugAccount>, (StatusCode, axum::Json<ErrorEnvelope>)> {
    if !state.router.clear_unhealthy(account_id) {
        return Err(internal_debug_account_not_found(account_id));
    }

    let diagnostics = state
        .router
        .account_diagnostics(account_id)
        .ok_or_else(|| internal_debug_account_not_found(account_id))?;

    Ok(axum::Json(internal_debug_account_from_diagnostics(
        diagnostics,
    )))
}

fn internal_debug_account_from_diagnostics(
    account: crate::router::AccountDiagnostics,
) -> InternalDebugAccount {
    InternalDebugAccount {
        id: account.id,
        label: account.label,
        mode: account.mode,
        enabled: account.enabled,
        priority: account.priority,
        base_url: account.base_url,
        chatgpt_account_id: account.chatgpt_account_id,
        temporarily_unhealthy: account.temporarily_unhealthy,
    }
}

fn internal_debug_account_not_found(account_id: Uuid) -> (StatusCode, axum::Json<ErrorEnvelope>) {
    (
        StatusCode::NOT_FOUND,
        axum::Json(ErrorEnvelope::new(
            "not_found",
            format!("account {account_id} not found"),
        )),
    )
}

fn invalid_auth_cache_lookup_request() -> (StatusCode, axum::Json<ErrorEnvelope>) {
    (
        StatusCode::BAD_REQUEST,
        axum::Json(ErrorEnvelope::new(
            "invalid_request",
            "token must be a non-empty string",
        )),
    )
}

fn invalid_auth_cache_evict_request() -> (StatusCode, axum::Json<ErrorEnvelope>) {
    (
        StatusCode::BAD_REQUEST,
        axum::Json(ErrorEnvelope::new(
            "invalid_request",
            "token must be a non-empty string",
        )),
    )
}

#[cfg(test)]
mod tests {
    use super::{build_app, max_request_body_bytes_from_env};
    use crate::config::DataPlaneConfig;
    use crate::test_support::{set_env, ENV_LOCK};
    use codex_pool_core::model::RoutingStrategy;

    fn empty_config() -> DataPlaneConfig {
        DataPlaneConfig {
            listen_addr: "127.0.0.1:0".parse().expect("listen addr"),
            routing_strategy: RoutingStrategy::RoundRobin,
            upstream_accounts: Vec::new(),
            account_ejection_ttl_sec: 30,
            enable_request_failover: true,
            same_account_quick_retry_max: 1,
            request_failover_wait_ms: 2_000,
            retry_poll_interval_ms: 100,
            sticky_prefer_non_conflicting: true,
            shared_routing_cache_enabled: true,
            enable_metered_stream_billing: true,
            billing_authorize_required_for_stream: true,
            stream_billing_reserve_microcredits: 2_000_000,
            billing_dynamic_preauth_enabled: true,
            billing_preauth_expected_output_tokens: 256,
            billing_preauth_safety_factor: 1.3,
            billing_preauth_min_microcredits: 1_000,
            billing_preauth_max_microcredits: 1_000_000_000_000,
            billing_preauth_unit_price_microcredits: 10_000,
            stream_billing_drain_timeout_ms: 5_000,
            billing_capture_retry_max: 3,
            billing_capture_retry_backoff_ms: 200,
            redis_url: None,
            auth_validate_url: None,
            auth_validate_cache_ttl_sec: 30,
            auth_validate_negative_cache_ttl_sec: 5,
            auth_fail_open: false,
            enable_internal_debug_routes: false,
        }
    }

    #[test]
    fn build_app_fails_when_internal_auth_token_missing() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old_internal = set_env("CONTROL_PLANE_INTERNAL_AUTH_TOKEN", None);

        let runtime = tokio::runtime::Runtime::new().expect("create runtime");
        let result = runtime.block_on(build_app(empty_config()));
        assert!(result.is_err());

        set_env("CONTROL_PLANE_INTERNAL_AUTH_TOKEN", old_internal.as_deref());
    }

    #[test]
    fn max_request_body_bytes_defaults_and_clamps() {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let old = set_env("DATA_PLANE_MAX_REQUEST_BODY_BYTES", None);
        assert_eq!(max_request_body_bytes_from_env(), 10 * 1024 * 1024);

        set_env("DATA_PLANE_MAX_REQUEST_BODY_BYTES", Some("256"));
        assert_eq!(max_request_body_bytes_from_env(), 1024);

        set_env("DATA_PLANE_MAX_REQUEST_BODY_BYTES", Some("268435456"));
        assert_eq!(max_request_body_bytes_from_env(), 64 * 1024 * 1024);

        set_env("DATA_PLANE_MAX_REQUEST_BODY_BYTES", old.as_deref());
    }
}
