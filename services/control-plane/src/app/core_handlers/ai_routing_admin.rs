async fn list_admin_routing_profiles(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<RoutingProfilesResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    let profiles = state
        .store
        .list_routing_profiles()
        .await
        .map_err(map_tenant_error)?;
    Ok(Json(RoutingProfilesResponse { profiles }))
}

async fn upsert_admin_routing_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpsertRoutingProfileRequest>,
) -> Result<Json<RoutingProfile>, (StatusCode, Json<ErrorEnvelope>)> {
    let principal = require_admin_principal(&state, &headers)?;
    let request_name = req.name.clone();
    let response = state
        .store
        .upsert_routing_profile(req)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: None,
            action: "admin.ai_routing.profile.upsert".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("routing_profile".to_string()),
            target_id: Some(response.id.to_string()),
            payload_json: json!({
                "name": request_name,
                "enabled": response.enabled,
                "priority": response.priority,
            }),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(Json(response))
}

async fn delete_admin_routing_profile(
    Path(profile_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorEnvelope>)> {
    let principal = require_admin_principal(&state, &headers)?;
    state
        .store
        .delete_routing_profile(profile_id)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: None,
            action: "admin.ai_routing.profile.delete".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("routing_profile".to_string()),
            target_id: Some(profile_id.to_string()),
            payload_json: json!({}),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_admin_model_routing_policies(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ModelRoutingPoliciesResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    let policies = state
        .store
        .list_model_routing_policies()
        .await
        .map_err(map_tenant_error)?;
    Ok(Json(ModelRoutingPoliciesResponse { policies }))
}

async fn upsert_admin_model_routing_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpsertModelRoutingPolicyRequest>,
) -> Result<Json<ModelRoutingPolicy>, (StatusCode, Json<ErrorEnvelope>)> {
    let principal = require_admin_principal(&state, &headers)?;
    let request_name = req.name.clone();
    let request_family = req.family.clone();
    let response = state
        .store
        .upsert_model_routing_policy(req)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: None,
            action: "admin.ai_routing.model_policy.upsert".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("model_routing_policy".to_string()),
            target_id: Some(response.id.to_string()),
            payload_json: json!({
                "name": request_name,
                "family": request_family,
                "enabled": response.enabled,
                "priority": response.priority,
            }),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(Json(response))
}

async fn delete_admin_model_routing_policy(
    Path(policy_id): Path<Uuid>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorEnvelope>)> {
    let principal = require_admin_principal(&state, &headers)?;
    state
        .store
        .delete_model_routing_policy(policy_id)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: None,
            action: "admin.ai_routing.model_policy.delete".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("model_routing_policy".to_string()),
            target_id: Some(policy_id.to_string()),
            payload_json: json!({}),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_admin_ai_routing_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AiRoutingSettingsResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    let settings = state
        .store
        .ai_routing_settings()
        .await
        .map_err(map_tenant_error)?;
    Ok(Json(AiRoutingSettingsResponse { settings }))
}

async fn update_admin_ai_routing_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpdateAiRoutingSettingsRequest>,
) -> Result<Json<AiRoutingSettingsResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let principal = require_admin_principal(&state, &headers)?;
    let response = state
        .store
        .update_ai_routing_settings(req)
        .await
        .map_err(map_tenant_error)?;
    write_audit_log_best_effort(
        &state,
        crate::tenant::AuditLogWriteRequest {
            actor_type: "admin_user".to_string(),
            actor_id: Some(principal.user_id),
            tenant_id: None,
            action: "admin.ai_routing.settings.update".to_string(),
            reason: None,
            request_ip: crate::tenant::extract_client_ip(&headers),
            user_agent: extract_user_agent(&headers),
            target_type: Some("ai_routing_settings".to_string()),
            target_id: Some("singleton".to_string()),
            payload_json: json!({
                "enabled": response.enabled,
                "auto_publish": response.auto_publish,
                "planner_model_chain": response.planner_model_chain,
                "trigger_mode": response.trigger_mode,
                "kill_switch": response.kill_switch,
            }),
            result_status: "ok".to_string(),
        },
    )
    .await;
    Ok(Json(AiRoutingSettingsResponse { settings: response }))
}

async fn list_admin_routing_plan_versions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<RoutingPlanVersionsResponse>, (StatusCode, Json<ErrorEnvelope>)> {
    let _principal = require_admin_principal(&state, &headers)?;
    let versions = state
        .store
        .list_routing_plan_versions()
        .await
        .map_err(map_tenant_error)?;
    Ok(Json(RoutingPlanVersionsResponse { versions }))
}
