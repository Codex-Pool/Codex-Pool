async fn get_admin_tenant_credit_balance(
    Path(tenant_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<crate::tenant::TenantCreditBalanceResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let principal = require_admin_principal(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    let response = tenant_auth
        .get_credit_balance(tenant_id)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: Some(tenant_id),
            action: "admin.tenant.credits.balance.get".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("tenant_credit_account".to_string()),
            target_id: Some(tenant_id.to_string()),
            payload_json: json!({
                "balance_microcredits": response.balance_microcredits,
                "updated_at": response.updated_at,
            }),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(Json(response))
}

async fn get_admin_tenant_credit_summary(
    Path(tenant_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<crate::tenant::TenantCreditSummaryResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let principal = require_admin_principal(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    let response = tenant_auth
        .get_credit_summary(tenant_id)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: Some(tenant_id),
            action: "admin.tenant.credits.summary.get".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("tenant_credit_summary".to_string()),
            target_id: Some(tenant_id.to_string()),
            payload_json: json!({
                "balance_microcredits": response.balance_microcredits,
                "today_consumed_microcredits": response.today_consumed_microcredits,
                "month_consumed_microcredits": response.month_consumed_microcredits,
                "updated_at": response.updated_at,
            }),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(Json(response))
}

async fn list_admin_tenant_credit_ledger(
    Path(tenant_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<TenantCreditLedgerQuery>,
) -> Result<Json<crate::tenant::TenantCreditLedgerResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let principal = require_admin_principal(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    let limit = query.limit.unwrap_or(100);
    let response = tenant_auth
        .list_credit_ledger(tenant_id, limit)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: Some(tenant_id),
            action: "admin.tenant.credits.ledger.list".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("tenant_credit_ledger".to_string()),
            target_id: Some(tenant_id.to_string()),
            payload_json: json!({
                "limit": limit,
                "item_count": response.items.len(),
            }),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(Json(response))
}

async fn claim_tenant_daily_checkin(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<crate::tenant::TenantDailyCheckinResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let tenant_auth = require_tenant_auth_service(&state)?;
    let principal = require_tenant_principal(&state, &headers).await?;
    let response = tenant_auth
        .daily_checkin(principal.tenant_id)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "tenant_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: Some(principal.tenant_id),
            action: "tenant.credits.checkin".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("tenant_credit_account".to_string()),
            target_id: Some(principal.tenant_id.to_string()),
            payload_json: json!({
                "local_date": response.local_date,
                "reward_microcredits": response.reward_microcredits,
                "balance_microcredits": response.balance_microcredits,
            }),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(Json(response))
}

async fn list_admin_tenants(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<crate::tenant::AdminTenantItem>>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    tenant_auth
        .admin_list_tenants()
        .await
        .map(Json)
        .map_err(map_tenant_error)
}

async fn create_admin_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<crate::tenant::AdminTenantCreateRequest>,
) -> Result<Json<crate::tenant::AdminTenantItem>, (StatusCode, Json<ErrorEnvelope>)> {
    let principal = require_admin_principal(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    let response = tenant_auth
        .admin_create_tenant(req)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: Some(response.id),
            action: "admin.tenant.create".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("tenant".to_string()),
            target_id: Some(response.id.to_string()),
            payload_json: json!({
                "name": response.name.clone(),
                "status": response.status.clone(),
                "plan": response.plan.clone(),
                "expires_at": response.expires_at,
            }),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(Json(response))
}

async fn ensure_default_admin_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<crate::tenant::AdminTenantItem>, (StatusCode, Json<ErrorEnvelope>)> {
    let principal = require_admin_principal(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    let response = tenant_auth
        .admin_ensure_default_tenant()
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: Some(response.id),
            action: "admin.tenant.ensure_default".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("tenant".to_string()),
            target_id: Some(response.id.to_string()),
            payload_json: json!({
                "name": response.name.clone(),
            }),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(Json(response))
}

async fn patch_admin_tenant(
    Path(tenant_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<crate::tenant::AdminTenantPatchRequest>,
) -> Result<Json<crate::tenant::AdminTenantItem>, (StatusCode, Json<ErrorEnvelope>)> {
    let principal = require_admin_principal(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    let response = tenant_auth
        .admin_patch_tenant(tenant_id, req)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: Some(tenant_id),
            action: "admin.tenant.patch".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("tenant".to_string()),
            target_id: Some(tenant_id.to_string()),
            payload_json: json!({
                "status": response.status.clone(),
                "plan": response.plan.clone(),
                "expires_at": response.expires_at,
            }),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(Json(response))
}

async fn recharge_admin_tenant_credits(
    Path(tenant_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<crate::tenant::AdminRechargeRequest>,
) -> Result<Json<crate::tenant::AdminRechargeResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let principal = require_admin_principal(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    let reason = req.reason.clone();
    let amount_microcredits = req.amount_microcredits;
    let response = tenant_auth
        .admin_recharge_tenant(tenant_id, req)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: Some(tenant_id),
            action: "admin.tenant.credits.recharge".to_string(),
            reason,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("tenant_credit_account".to_string()),
            target_id: Some(tenant_id.to_string()),
            payload_json: json!({
                "amount_microcredits": amount_microcredits,
                "balance_microcredits": response.balance_microcredits,
            }),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(Json(response))
}

async fn list_admin_model_pricing(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<crate::tenant::ModelPricingItem>>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    if let Some(tenant_auth) = state.tenant_auth_service.as_ref() {
        tenant_auth
            .admin_list_model_pricing()
            .await
            .map(Json)
            .map_err(map_tenant_error)
    } else if let Some(sqlite_repo) = state.sqlite_usage_repo.as_ref() {
        sqlite_repo
            .list_model_pricing()
            .await
            .map(Json)
            .map_err(internal_error)
    } else {
        Err(tenant_service_unavailable_error())
    }
}

async fn upsert_admin_model_pricing(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<crate::tenant::ModelPricingUpsertRequest>,
) -> Result<Json<crate::tenant::ModelPricingItem>, (StatusCode, Json<ErrorEnvelope>)> {
    let principal = require_admin_principal(&state, &headers)?;
    let request_model = req.model.clone();
    let response = if let Some(tenant_auth) = state.tenant_auth_service.as_ref() {
        tenant_auth
            .admin_upsert_model_pricing(req)
            .await
            .map_err(map_tenant_error)?
    } else if let Some(sqlite_repo) = state.sqlite_usage_repo.as_ref() {
        sqlite_repo
            .upsert_model_pricing(req)
            .await
            .map_err(internal_error)?
    } else {
        return Err(tenant_service_unavailable_error());
    };
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: None,
            action: "admin.model_pricing.upsert".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("model_pricing".to_string()),
            target_id: Some(response.id.to_string()),
            payload_json: json!({
                "model": request_model,
                "input_price_microcredits": response.input_price_microcredits,
                "cached_input_price_microcredits": response.cached_input_price_microcredits,
                "output_price_microcredits": response.output_price_microcredits,
                "enabled": response.enabled,
            }),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(Json(response))
}

async fn delete_admin_model_pricing(
    Path(pricing_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorEnvelope>)> {
    let principal = require_admin_principal(&state, &headers)?;
    if let Some(tenant_auth) = state.tenant_auth_service.as_ref() {
        tenant_auth
            .admin_delete_model_pricing(pricing_id)
            .await
            .map_err(map_tenant_error)?;
    } else if let Some(sqlite_repo) = state.sqlite_usage_repo.as_ref() {
        sqlite_repo
            .delete_model_pricing(pricing_id)
            .await
            .map_err(internal_error)?;
    } else {
        return Err(tenant_service_unavailable_error());
    }
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: None,
            action: "admin.model_pricing.delete".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("model_pricing".to_string()),
            target_id: Some(pricing_id.to_string()),
            payload_json: json!({}),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_admin_api_key_groups(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<crate::tenant::ApiKeyGroupAdminListResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    tenant_auth
        .admin_list_api_key_groups()
        .await
        .map(Json)
        .map_err(map_tenant_error)
}

async fn upsert_admin_api_key_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<crate::tenant::ApiKeyGroupUpsertRequest>,
) -> Result<Json<crate::tenant::ApiKeyGroupItem>, (StatusCode, Json<ErrorEnvelope>)> {
    let principal = require_admin_principal(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    let request_name = req.name.clone();
    let response = tenant_auth
        .admin_upsert_api_key_group(req)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: None,
            action: "admin.api_key_group.upsert".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("api_key_group".to_string()),
            target_id: Some(response.id.to_string()),
            payload_json: json!({
                "name": request_name,
                "is_default": response.is_default,
                "enabled": response.enabled,
                "allow_all_models": response.allow_all_models,
                "input_multiplier_ppm": response.input_multiplier_ppm,
                "cached_input_multiplier_ppm": response.cached_input_multiplier_ppm,
                "output_multiplier_ppm": response.output_multiplier_ppm,
            }),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(Json(response))
}

async fn delete_admin_api_key_group(
    Path(group_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorEnvelope>)> {
    let principal = require_admin_principal(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    tenant_auth
        .admin_delete_api_key_group(group_id)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: None,
            action: "admin.api_key_group.delete".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("api_key_group".to_string()),
            target_id: Some(group_id.to_string()),
            payload_json: json!({}),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

async fn upsert_admin_api_key_group_model_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<crate::tenant::ApiKeyGroupModelPolicyUpsertRequest>,
) -> Result<Json<crate::tenant::ApiKeyGroupModelPolicyItem>, (StatusCode, Json<ErrorEnvelope>)> {
    let principal = require_admin_principal(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    let request_group_id = req.group_id;
    let request_model = req.model.clone();
    let response = tenant_auth
        .admin_upsert_api_key_group_model_policy(req)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: None,
            action: "admin.api_key_group_model_policy.upsert".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("api_key_group_model_policy".to_string()),
            target_id: Some(response.id.to_string()),
            payload_json: json!({
                "group_id": request_group_id,
                "model": request_model,
                "enabled": response.enabled,
                "input_multiplier_ppm": response.input_multiplier_ppm,
                "cached_input_multiplier_ppm": response.cached_input_multiplier_ppm,
                "output_multiplier_ppm": response.output_multiplier_ppm,
                "input_price_microcredits": response.input_price_microcredits,
                "cached_input_price_microcredits": response.cached_input_price_microcredits,
                "output_price_microcredits": response.output_price_microcredits,
            }),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(Json(response))
}

async fn delete_admin_api_key_group_model_policy(
    Path(policy_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorEnvelope>)> {
    let principal = require_admin_principal(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    tenant_auth
        .admin_delete_api_key_group_model_policy(policy_id)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: None,
            action: "admin.api_key_group_model_policy.delete".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("api_key_group_model_policy".to_string()),
            target_id: Some(policy_id.to_string()),
            payload_json: json!({}),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

#[allow(dead_code)]
async fn list_admin_billing_pricing_rules(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<crate::tenant::BillingPricingRuleItem>>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    tenant_auth
        .admin_list_billing_pricing_rules()
        .await
        .map(Json)
        .map_err(map_tenant_error)
}

#[allow(dead_code)]
async fn upsert_admin_billing_pricing_rule(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<crate::tenant::BillingPricingRuleUpsertRequest>,
) -> Result<Json<crate::tenant::BillingPricingRuleItem>, (StatusCode, Json<ErrorEnvelope>)> {
    let principal = require_admin_principal(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    let request_model_pattern = req.model_pattern.clone();
    let response = tenant_auth
        .admin_upsert_billing_pricing_rule(req)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: None,
            action: "admin.billing_pricing_rule.upsert".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("billing_pricing_rule".to_string()),
            target_id: Some(response.id.to_string()),
            payload_json: json!({
                "model_pattern": request_model_pattern,
                "request_kind": response.request_kind.clone(),
                "scope": response.scope.clone(),
                "threshold_input_tokens": response.threshold_input_tokens,
                "input_multiplier_ppm": response.input_multiplier_ppm,
                "cached_input_multiplier_ppm": response.cached_input_multiplier_ppm,
                "output_multiplier_ppm": response.output_multiplier_ppm,
                "priority": response.priority,
                "enabled": response.enabled,
            }),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(Json(response))
}

#[allow(dead_code)]
async fn delete_admin_billing_pricing_rule(
    Path(rule_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorEnvelope>)> {
    let principal = require_admin_principal(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    tenant_auth
        .admin_delete_billing_pricing_rule(rule_id)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: None,
            action: "admin.billing_pricing_rule.delete".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("billing_pricing_rule".to_string()),
            target_id: Some(rule_id.to_string()),
            payload_json: json!({}),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

#[allow(dead_code)]
async fn list_admin_model_entities(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<crate::tenant::AdminModelEntityItem>>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    tenant_auth
        .admin_list_model_entities()
        .await
        .map(Json)
        .map_err(map_tenant_error)
}

#[allow(dead_code)]
async fn upsert_admin_model_entity(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<crate::tenant::AdminModelEntityUpsertRequest>,
) -> Result<Json<crate::tenant::AdminModelEntityItem>, (StatusCode, Json<ErrorEnvelope>)> {
    let principal = require_admin_principal(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    let request_model = req.model.clone();
    let response = tenant_auth
        .admin_upsert_model_entity(req)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: None,
            action: "admin.model_entity.upsert".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("model_entity".to_string()),
            target_id: Some(response.id.to_string()),
            payload_json: json!({
                "model": request_model,
                "provider": response.provider.clone(),
                "visibility": response.visibility.clone(),
            }),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(Json(response))
}

#[allow(dead_code)]
async fn delete_admin_model_entity(
    Path(entity_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorEnvelope>)> {
    let principal = require_admin_principal(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    tenant_auth
        .admin_delete_model_entity(entity_id)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: None,
            action: "admin.model_entity.delete".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("model_entity".to_string()),
            target_id: Some(entity_id.to_string()),
            payload_json: json!({}),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

async fn create_admin_impersonation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<crate::tenant::AdminImpersonateRequest>,
) -> Result<Json<crate::tenant::AdminImpersonateResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let admin = require_admin_principal(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    let request_reason = req.reason.clone();
    let request_tenant_id = req.tenant_id;
    let response = tenant_auth
        .admin_impersonate(admin.user_id, req)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(admin.user_id),
            tenant_id: Some(request_tenant_id),
            action: "admin.impersonation.create".to_string(),
            reason: Some(request_reason),
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("admin_impersonation_session".to_string()),
            target_id: Some(response.session_id.to_string()),
            payload_json: json!({
                "expires_in": response.expires_in,
            }),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(Json(response))
}

async fn delete_admin_impersonation(
    Path(session_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorEnvelope>)> {
    let admin = require_admin_principal(&state, &headers)?;
    let tenant_auth = require_tenant_auth_service(&state)?;
    tenant_auth
        .admin_revoke_impersonation(session_id)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(admin.user_id),
            tenant_id: None,
            action: "admin.impersonation.delete".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("admin_impersonation_session".to_string()),
            target_id: Some(session_id.to_string()),
            payload_json: json!({}),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}
