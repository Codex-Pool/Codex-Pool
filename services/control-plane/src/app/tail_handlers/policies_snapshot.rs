async fn set_routing_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpsertRoutingPolicyRequest>,
) -> Result<Json<PolicyResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    state
        .store
        .upsert_routing_policy(req)
        .await
        .map(|policy| Json(PolicyResponse { policy }))
        .map_err(internal_error)
}

async fn set_retry_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpsertRetryPolicyRequest>,
) -> Result<Json<PolicyResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    state
        .store
        .upsert_retry_policy(req)
        .await
        .map(|policy| Json(PolicyResponse { policy }))
        .map_err(internal_error)
}

async fn set_stream_retry_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpsertStreamRetryPolicyRequest>,
) -> Result<Json<PolicyResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    state
        .store
        .upsert_stream_retry_policy(req)
        .await
        .map(|policy| Json(PolicyResponse { policy }))
        .map_err(internal_error)
}

async fn data_plane_snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<codex_pool_core::api::DataPlaneSnapshot>, (StatusCode, Json<ErrorEnvelope>)> {
    require_internal_service_token(&state, &headers)?;
    state
        .store
        .snapshot()
        .await
        .map(Json)
        .map_err(internal_error)
}

#[derive(Debug, Deserialize)]
struct DataPlaneSnapshotEventsQuery {
    after: Option<u64>,
    limit: Option<u32>,
    wait_ms: Option<u64>,
}

async fn data_plane_snapshot_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<DataPlaneSnapshotEventsQuery>,
) -> Result<Json<codex_pool_core::api::DataPlaneSnapshotEventsResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    require_internal_service_token(&state, &headers)?;

    let after = query.after.unwrap_or(0);
    let limit = query.limit.unwrap_or(500).clamp(1, 5_000);
    let wait_ms = query.wait_ms.unwrap_or(30_000).clamp(0, 60_000);
    let started = std::time::Instant::now();
    let mut latest_cursor = after;
    let mut latest_high_watermark = after;

    loop {
        match state.store.data_plane_snapshot_events(after, limit).await {
            Ok(response) => {
                latest_cursor = latest_cursor.max(response.cursor);
                latest_high_watermark = latest_high_watermark.max(response.high_watermark);
                if wait_ms == 0 || !response.events.is_empty() {
                    return Ok(Json(response));
                }
                if started.elapsed() >= Duration::from_millis(wait_ms) {
                    return Ok(Json(codex_pool_core::api::DataPlaneSnapshotEventsResponse {
                        cursor: latest_cursor,
                        high_watermark: latest_high_watermark,
                        events: Vec::new(),
                    }));
                }
            }
            Err(err) => {
                let message = err.to_string();
                if message.contains("cursor_gone") {
                    return Err((
                        StatusCode::GONE,
                        Json(ErrorEnvelope::new(
                            "cursor_gone",
                            "snapshot event cursor is no longer available",
                        )),
                    ));
                }
                return Err(internal_error(err));
            }
        }

        let elapsed_ms = started.elapsed().as_millis() as u64;
        let remaining_ms = wait_ms.saturating_sub(elapsed_ms);
        if remaining_ms == 0 {
            return Ok(Json(codex_pool_core::api::DataPlaneSnapshotEventsResponse {
                cursor: latest_cursor,
                high_watermark: latest_high_watermark,
                events: Vec::new(),
            }));
        }
        let sleep_ms = remaining_ms.min(200);
        tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
    }
}

async fn internal_refresh_oauth_account(
    Path(account_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<OAuthAccountStatusResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    require_internal_service_token(&state, &headers)?;
    state
        .store
        .refresh_oauth_account(account_id)
        .await
        .map(Json)
        .map_err(internal_error)
}

async fn internal_disable_upstream_account(
    Path(account_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<codex_pool_core::model::UpstreamAccount>, (StatusCode, Json<ErrorEnvelope>)> {
    require_internal_service_token(&state, &headers)?;
    state
        .store
        .set_upstream_account_enabled(account_id, false)
        .await
        .map(Json)
        .map_err(internal_error)
}

async fn validate_api_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ValidateApiKeyRequest>,
) -> Result<Json<ValidateApiKeyResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    require_internal_service_token(&state, &headers)?;
    let principal = state
        .store
        .validate_api_key(&req.token)
        .await
        .map_err(internal_error)?;
    let principal = principal.ok_or_else(unauthorized_error)?;

    Ok(Json(ValidateApiKeyResponse {
        tenant_id: principal.tenant_id,
        api_key_id: principal.api_key_id,
        enabled: principal.enabled,
        group: codex_pool_core::api::ApiKeyGroupStatus {
            id: principal.api_key_group_id,
            name: principal.api_key_group_name,
            invalid: principal.api_key_group_invalid,
        },
        policy: codex_pool_core::api::ApiKeyPolicy {
            ip_allowlist: principal.key_ip_allowlist,
            model_allowlist: principal.key_model_allowlist,
        },
        tenant_status: principal.tenant_status,
        tenant_expires_at: principal.tenant_expires_at,
        balance_microcredits: state
            .system_capabilities
            .visible_balance_microcredits(principal.balance_microcredits),
        cache_ttl_sec: state.auth_validate_cache_ttl_sec,
    }))
}

fn normalize_optional_query_value(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn resolve_request_logs_query(query: RequestLogsQuery) -> Result<crate::usage::RequestLogQuery, (StatusCode, Json<ErrorEnvelope>)> {
    let end_ts = query.end_ts.unwrap_or_else(|| Utc::now().timestamp());
    let start_ts = query
        .start_ts
        .unwrap_or_else(|| end_ts.saturating_sub(24 * 60 * 60));
    validate_usage_range(start_ts, end_ts)?;

    Ok(crate::usage::RequestLogQuery {
        start_ts,
        end_ts,
        limit: query
            .limit
            .unwrap_or(DEFAULT_USAGE_QUERY_LIMIT)
            .min(MAX_USAGE_QUERY_LIMIT),
        tenant_id: query.tenant_id,
        api_key_id: query.api_key_id,
        status_code: query.status_code,
        request_id: normalize_optional_query_value(query.request_id),
        keyword: normalize_optional_query_value(query.keyword),
    })
}

fn resolve_audit_logs_query(
    query: AuditLogsQuery,
) -> Result<crate::tenant::AuditLogListQuery, (StatusCode, Json<ErrorEnvelope>)> {
    let end_ts = query.end_ts.unwrap_or_else(|| Utc::now().timestamp());
    let start_ts = query
        .start_ts
        .unwrap_or_else(|| end_ts.saturating_sub(24 * 60 * 60));
    validate_usage_range(start_ts, end_ts)?;
    let start_at = DateTime::<Utc>::from_timestamp(start_ts, 0)
        .ok_or_else(|| invalid_request_error("invalid start_ts"))?;
    let end_at =
        DateTime::<Utc>::from_timestamp(end_ts, 0).ok_or_else(|| invalid_request_error("invalid end_ts"))?;

    Ok(crate::tenant::AuditLogListQuery {
        start_at,
        end_at,
        limit: query
            .limit
            .unwrap_or(DEFAULT_USAGE_QUERY_LIMIT)
            .min(MAX_USAGE_QUERY_LIMIT) as usize,
        tenant_id: query.tenant_id,
        actor_type: normalize_optional_query_value(query.actor_type),
        actor_id: query.actor_id,
        action: normalize_optional_query_value(query.action),
        result_status: normalize_optional_query_value(query.result_status),
        keyword: normalize_optional_query_value(query.keyword),
    })
}

fn map_request_log_row(row: crate::usage::RequestLogRow) -> RequestLogItemResponse {
    RequestLogItemResponse {
        id: row.id,
        account_id: row.account_id,
        tenant_id: row.tenant_id,
        api_key_id: row.api_key_id,
        request_id: row.request_id,
        path: row.path,
        method: row.method,
        model: row.model,
        service_tier: row.service_tier,
        input_tokens: row.input_tokens,
        cached_input_tokens: row.cached_input_tokens,
        output_tokens: row.output_tokens,
        reasoning_tokens: row.reasoning_tokens,
        first_token_latency_ms: row.first_token_latency_ms,
        status_code: row.status_code,
        latency_ms: row.latency_ms,
        is_stream: row.is_stream,
        error_code: row.error_code,
        billing_phase: row.billing_phase,
        authorization_id: row.authorization_id,
        capture_status: row.capture_status,
        estimated_cost_microusd: row.estimated_cost_microusd,
        created_at: row.created_at,
        event_version: row.event_version,
    }
}
