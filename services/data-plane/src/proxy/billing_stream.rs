const BILLING_PREAUTH_ERROR_RATIO_RECENT_WINDOW: usize = 512;
const BILLING_PREAUTH_ERROR_RATIO_MODEL_WINDOW: usize = 128;
const BILLING_PREAUTH_ERROR_RATIO_MODEL_MAX: usize = 64;
const PRICING_PER_MILLION_TOKENS_SCALE: f64 = 1_000_000.0;
const STREAM_PRELUDE_BUFFER_MAX_BYTES: usize = 32 * 1024;
const STREAM_PRELUDE_BUFFER_MAX_CHUNKS: usize = 16;

async fn build_pending_billing_session(
    state: &AppState,
    principal: Option<&ApiPrincipal>,
    headers: &HeaderMap,
    context: &ParsedRequestPolicyContext,
    path: &str,
    method: &str,
) -> std::result::Result<Option<PendingBillingSession>, Box<Response>> {
    if !is_billable_path(path, method) {
        return Ok(None);
    }

    let Some(principal) = principal else {
        return Ok(None);
    };
    if principal.balance_microcredits.is_none() {
        return Ok(None);
    }
    if context.stream && !state.enable_metered_stream_billing {
        return Err(Box::new(json_error(
            StatusCode::BAD_REQUEST,
            "billing_streaming_not_supported",
            "streaming request is not enabled for metered billing mode",
        )));
    }
    if context.stream && !state.billing_authorize_required_for_stream {
        return Ok(None);
    }

    let Some(tenant_id) = principal.tenant_id else {
        return Ok(None);
    };
    let Some(api_key_id) = principal.api_key_id else {
        return Ok(None);
    };
    let Some(model) = context.model.as_deref() else {
        return Err(Box::new(json_error(
            StatusCode::BAD_REQUEST,
            "billing_model_missing",
            "request model is required for metered billing",
        )));
    };
    let estimated_input_tokens = context.estimated_input_tokens.unwrap_or(0);
    let trace_request_id = resolve_trace_request_id(headers, context);
    let request_id = generate_billing_request_id();
    let session_key = context
        .session_key_hint
        .clone()
        .or(trace_request_id.clone())
        .unwrap_or_else(|| request_id.clone());
    let reserved_microcredits =
        estimate_authorize_reserve_microcredits(
            state,
            api_key_id,
            model,
            context.requested_service_tier.as_deref(),
            estimated_input_tokens,
        )
        .await;

    Ok(Some(PendingBillingSession {
        tenant_id,
        api_key_id,
        request_id,
        trace_request_id,
        model: model.to_string(),
        requested_service_tier: context.requested_service_tier.clone(),
        session_key,
        request_kind: billing_request_kind(path).to_string(),
        is_stream: context.stream,
        estimated_input_tokens,
        reserved_microcredits,
    }))
}

fn billing_request_kind(path: &str) -> &'static str {
    match path {
        "/v1/responses" | "/backend-api/codex/responses" => "response",
        "/v1/responses/compact" | "/backend-api/codex/responses/compact" => "compact",
        "/v1/chat/completions" => "chat",
        _ => "unknown",
    }
}

async fn estimate_authorize_reserve_microcredits(
    state: &AppState,
    api_key_id: Uuid,
    model: &str,
    service_tier: Option<&str>,
    estimated_input_tokens: i64,
) -> i64 {
    if !state.billing_dynamic_preauth_enabled {
        return mark_fallback_preauth_reserve(state, state.stream_billing_reserve_microcredits);
    }

    if let Some(pricing) =
        resolve_model_pricing_for_preauth(state, api_key_id, model, service_tier).await
    {
        if let Some(reserve) = estimate_reserve_with_model_pricing(
            state,
            estimated_input_tokens,
            pricing.input_price_microcredits,
            pricing.output_price_microcredits,
        ) {
            state
                .billing_preauth_dynamic_total
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            state.billing_preauth_amount_microcredits_sum.fetch_add(
                metric_amount_from_microcredits(reserve),
                std::sync::atomic::Ordering::Relaxed,
            );
            return reserve;
        }
    }

    let reserve = estimate_reserve_with_unit_price(state, estimated_input_tokens)
        .unwrap_or(state.stream_billing_reserve_microcredits);
    mark_fallback_preauth_reserve(state, reserve)
}

async fn resolve_model_pricing_for_preauth(
    state: &AppState,
    api_key_id: Uuid,
    model: &str,
    service_tier: Option<&str>,
) -> Option<InternalBillingPricingResponse> {
    let normalized_service_tier = service_tier
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
        .unwrap_or_else(|| "default".to_string());
    let model_key = format!(
        "{}:{}:{}",
        api_key_id,
        model.trim().to_ascii_lowercase(),
        normalized_service_tier
    );
    if model_key.is_empty() {
        return None;
    }
    let now = Instant::now();
    if let Ok(cache) = state.billing_pricing_cache.read() {
        if let Some(cached) = cache.get(&model_key) {
            if cached.expires_at > now {
                return Some(InternalBillingPricingResponse {
                    input_price_microcredits: cached.input_price_microcredits,
                    cached_input_price_microcredits: cached.cached_input_price_microcredits,
                    output_price_microcredits: cached.output_price_microcredits,
                    source: cached.source.to_string(),
                });
            }
        }
    }

    let fetched =
        fetch_model_pricing_from_control_plane(state, api_key_id, model, service_tier).await?;
    if let Ok(mut cache) = state.billing_pricing_cache.write() {
        cache.insert(
            model_key,
            crate::app::CachedBillingPricing {
                input_price_microcredits: fetched.input_price_microcredits,
                cached_input_price_microcredits: fetched.cached_input_price_microcredits,
                output_price_microcredits: fetched.output_price_microcredits,
                source: Arc::<str>::from(fetched.source.clone()),
                expires_at: now + Duration::from_secs(BILLING_PRICING_CACHE_TTL_SEC),
            },
        );
    }
    Some(fetched)
}

async fn fetch_model_pricing_from_control_plane(
    state: &AppState,
    api_key_id: Uuid,
    model: &str,
    service_tier: Option<&str>,
) -> Option<InternalBillingPricingResponse> {
    let base_url = state.control_plane_base_url.as_deref()?;
    let endpoint = format!(
        "{}/internal/v1/billing/pricing",
        base_url.trim_end_matches('/')
    );
    let response = state
        .http_client
        .post(endpoint)
        .bearer_auth(state.control_plane_internal_auth_token.as_ref())
        .json(&InternalBillingPricingPayload {
            model: model.to_string(),
            api_key_id: Some(api_key_id),
            service_tier: service_tier.map(ToString::to_string),
        })
        .timeout(Duration::from_secs(INTERNAL_BILLING_PRICING_TIMEOUT_SEC))
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    response.json::<InternalBillingPricingResponse>().await.ok()
}

fn estimate_reserve_with_model_pricing(
    state: &AppState,
    estimated_input_tokens: i64,
    input_price_microcredits: i64,
    output_price_microcredits: i64,
) -> Option<i64> {
    let estimated_input_tokens = estimated_input_tokens.max(0) as f64;
    let expected_output_tokens = state.billing_preauth_expected_output_tokens.max(0) as f64;
    let input_price_microcredits = input_price_microcredits.max(0) as f64;
    let output_price_microcredits = output_price_microcredits.max(0) as f64;
    let priced_tokens = ((estimated_input_tokens * input_price_microcredits)
        + (expected_output_tokens * output_price_microcredits))
        / PRICING_PER_MILLION_TOKENS_SCALE;
    let raw_reserve = priced_tokens * state.billing_preauth_safety_factor;
    clamp_estimated_reserve(state, raw_reserve)
}

fn estimate_reserve_with_unit_price(state: &AppState, estimated_input_tokens: i64) -> Option<i64> {
    let estimated_input_tokens = estimated_input_tokens.max(0) as f64;
    let expected_output_tokens = state.billing_preauth_expected_output_tokens.max(0) as f64;
    let unit_price_microcredits = state.billing_preauth_unit_price_microcredits.max(0) as f64;
    let priced_tokens = ((estimated_input_tokens + expected_output_tokens) * unit_price_microcredits)
        / PRICING_PER_MILLION_TOKENS_SCALE;
    let raw_reserve = priced_tokens * state.billing_preauth_safety_factor;
    clamp_estimated_reserve(state, raw_reserve)
}

fn clamp_estimated_reserve(state: &AppState, raw_reserve: f64) -> Option<i64> {
    if !raw_reserve.is_finite() {
        return None;
    }
    let min_reserve = state
        .billing_preauth_min_microcredits
        .min(state.billing_preauth_max_microcredits);
    let max_reserve = state
        .billing_preauth_min_microcredits
        .max(state.billing_preauth_max_microcredits);
    Some((raw_reserve.ceil() as i64).clamp(min_reserve, max_reserve))
}

fn mark_fallback_preauth_reserve(state: &AppState, reserve: i64) -> i64 {
    state
        .billing_preauth_fallback_total
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    state
        .billing_preauth_amount_microcredits_sum
        .fetch_add(metric_amount_from_microcredits(reserve), std::sync::atomic::Ordering::Relaxed);
    reserve
}

fn metric_amount_from_microcredits(value: i64) -> u64 {
    value.max(0) as u64
}

fn preauth_error_ratio_ppm(reserved_microcredits: i64, charged_microcredits: i64) -> u64 {
    let reserved = reserved_microcredits.max(0) as f64;
    let charged = charged_microcredits.max(0) as f64;
    let denominator = charged.max(1.0);
    let ratio = (reserved - charged).abs() / denominator;
    if !ratio.is_finite() {
        return 0;
    }
    (ratio * 1_000_000.0).round().max(0.0).min(u64::MAX as f64) as u64
}

fn record_preauth_error_ratio_sample(
    state: &AppState,
    model: &str,
    reserved_microcredits: i64,
    charged_microcredits: Option<i64>,
) {
    let Some(charged_microcredits) = charged_microcredits else {
        state
            .billing_preauth_capture_missing_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        return;
    };
    let ratio_ppm = preauth_error_ratio_ppm(reserved_microcredits, charged_microcredits);
    state
        .billing_preauth_error_ratio_ppm_sum_total
        .fetch_add(ratio_ppm, std::sync::atomic::Ordering::Relaxed);
    state
        .billing_preauth_error_ratio_count_total
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    if let Ok(mut recent) = state.billing_preauth_error_ratio_recent_ppm.write() {
        recent.push_back(ratio_ppm);
        if recent.len() > BILLING_PREAUTH_ERROR_RATIO_RECENT_WINDOW {
            recent.pop_front();
        }
    }

    let model_key = model.trim().to_ascii_lowercase();
    if model_key.is_empty() {
        return;
    }
    if let Ok(mut by_model) = state.billing_preauth_error_ratio_by_model_ppm.write() {
        if !by_model.contains_key(&model_key) && by_model.len() >= BILLING_PREAUTH_ERROR_RATIO_MODEL_MAX {
            if let Some(first_key) = by_model.keys().next().cloned() {
                by_model.remove(&first_key);
            }
        }
        let samples = by_model.entry(model_key).or_default();
        samples.push_back(ratio_ppm);
        if samples.len() > BILLING_PREAUTH_ERROR_RATIO_MODEL_WINDOW {
            samples.pop_front();
        }
    }
}

fn resolve_trace_request_id(headers: &HeaderMap, context: &ParsedRequestPolicyContext) -> Option<String> {
    context.request_id.clone().or_else(|| {
        headers
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
    })
}

fn generate_billing_request_id() -> String {
    Uuid::new_v4().to_string()
}

async fn authorize_billing_session(
    state: &AppState,
    pending: &PendingBillingSession,
    account_id: Uuid,
    path: &str,
    method: &str,
    started: Instant,
    is_stream: bool,
) -> std::result::Result<BillingSession, Box<Response>> {
    state
        .billing_authorize_total
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    match trigger_internal_billing_authorize(
        state,
        InternalBillingAuthorizePayload {
            tenant_id: pending.tenant_id,
            api_key_id: Some(pending.api_key_id),
            request_id: pending.request_id.clone(),
            trace_request_id: pending.trace_request_id.clone(),
            model: pending.model.clone(),
            service_tier: pending.requested_service_tier.clone(),
            session_key: Some(pending.session_key.clone()),
            request_kind: Some(pending.request_kind.clone()),
            reserved_microcredits: pending.reserved_microcredits,
            ttl_sec: Some(BILLING_AUTHORIZATION_TTL_SEC),
            is_stream,
        },
    )
    .await
    {
        Ok(authorize) => {
            if authorize.status != "authorized" {
                state
                    .billing_idempotent_hit_total
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            Ok(BillingSession {
                account_id,
                tenant_id: pending.tenant_id,
                api_key_id: pending.api_key_id,
                request_path: path.to_string(),
                request_method: method.to_string(),
                request_started: started,
                request_id: pending.request_id.clone(),
                trace_request_id: pending.trace_request_id.clone(),
                model: pending.model.clone(),
                requested_service_tier: pending.requested_service_tier.clone(),
                effective_service_tier: None,
                session_key: pending.session_key.clone(),
                request_kind: pending.request_kind.clone(),
                is_stream,
                first_token_latency_ms: None,
                estimated_input_tokens: pending.estimated_input_tokens,
                authorization_id: authorize.authorization_id,
                reserved_microcredits: authorize.reserved_microcredits,
            })
        }
        Err(err) => {
            state
                .billing_authorize_failed_total
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Err(err)
        }
    }
}

async fn settle_billing_if_needed(
    state: Arc<AppState>,
    billing_session: Option<&BillingSession>,
    status: StatusCode,
    body: &bytes::Bytes,
) -> std::result::Result<BillingSettleOutcome, Box<Response>> {
    let Some(billing_session) = billing_session else {
        return Ok(BillingSettleOutcome::NotNeeded);
    };
    if !status.is_success() {
        return Ok(BillingSettleOutcome::NotNeeded);
    }
    let Some(usage_tokens) = extract_usage_tokens(body) else {
        let estimated_input_tokens = billing_session.estimated_input_tokens.max(0);
        let estimated_output_tokens = estimate_response_output_tokens(body).unwrap_or(0).max(0);
        let estimated_usage = UsageTokens {
            input_tokens: estimated_input_tokens,
            cached_input_tokens: 0,
            output_tokens: estimated_output_tokens,
            reasoning_tokens: 0,
        };
        let billing_session = billing_session.clone();
        let authorization_id = billing_session.authorization_id;
        let billing_session_for_task = billing_session.clone();
        tokio::spawn(async move {
            warn!(
                request_id = %billing_session_for_task.request_id,
                authorization_id = %billing_session_for_task.authorization_id,
                estimated_input_tokens,
                estimated_output_tokens,
                usage_confidence = "low",
                "non-stream usage missing; applying low-confidence token estimate and deferring billing capture"
            );
            if estimated_input_tokens == 0 && estimated_output_tokens == 0 {
                release_billing_hold_best_effort(
                    state,
                    Some(billing_session_for_task),
                    Some(BillingReleaseFailureContext {
                        release_reason: Some("usage_missing_zero_estimate".to_string()),
                        failover_action: Some("return_success".to_string()),
                        failover_reason_class: Some("usage_missing_zero_estimate".to_string()),
                        ..Default::default()
                    }),
                )
                .await;
                return;
            }
            if let Err(err) = settle_authorized_billing(
                state.as_ref(),
                &billing_session_for_task,
                estimated_usage,
                billing_session_for_task.effective_service_tier.as_deref(),
            )
            .await
            {
                warn!(
                    status = ?err.status(),
                    request_id = %billing_session_for_task.request_id,
                    authorization_id = %billing_session_for_task.authorization_id,
                    reserved_microcredits = billing_session_for_task.reserved_microcredits,
                    "non-stream deferred billing settle failed"
                );
            }
        });
        return Ok(BillingSettleOutcome::DeferredEstimated {
            authorization_id,
            usage: estimated_usage,
        });
    };

    let settle_result = settle_authorized_billing(
        state.as_ref(),
        billing_session,
        usage_tokens,
        billing_session.effective_service_tier.as_deref(),
    )
    .await?;
    Ok(BillingSettleOutcome::Settled(settle_result))
}

async fn settle_authorized_billing(
    state: &AppState,
    billing_session: &BillingSession,
    usage_tokens: UsageTokens,
    effective_service_tier: Option<&str>,
) -> std::result::Result<BillingSettleResult, Box<Response>> {
    state
        .billing_capture_total
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let capture = match trigger_internal_billing_capture_with_retry(
        state,
        InternalBillingCapturePayload {
            tenant_id: billing_session.tenant_id,
            api_key_id: Some(billing_session.api_key_id),
            request_id: billing_session.request_id.clone(),
            model: billing_session.model.clone(),
            service_tier: effective_service_tier
                .map(ToString::to_string)
                .or_else(|| billing_session.requested_service_tier.clone()),
            session_key: Some(billing_session.session_key.clone()),
            request_kind: Some(billing_session.request_kind.clone()),
            input_tokens: usage_tokens.input_tokens,
            cached_input_tokens: usage_tokens.cached_input_tokens,
            output_tokens: usage_tokens.output_tokens,
            reasoning_tokens: usage_tokens.reasoning_tokens,
            is_stream: billing_session.is_stream,
        },
    )
    .await
    {
        Ok(value) => value,
        Err(err) => {
            state
                .billing_capture_failed_total
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            state
                .billing_release_total
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            state
                .billing_release_without_capture_total
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let _ = trigger_internal_billing_release(
                state,
                InternalBillingReleasePayload {
                    tenant_id: billing_session.tenant_id,
                    request_id: billing_session.request_id.clone(),
                    is_stream: billing_session.is_stream,
                    release_reason: None,
                    upstream_status_code: None,
                    upstream_error_code: None,
                    failover_action: None,
                    failover_reason_class: None,
                    recovery_action: None,
                    recovery_outcome: None,
                    cross_account_failover_attempted: None,
                },
            )
            .await;
            return Err(err);
        }
    };
    let capture_status = capture.status;
    if capture_status != "captured" {
        state
            .billing_idempotent_hit_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    state
        .billing_release_total
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let release = trigger_internal_billing_release(
        state,
        InternalBillingReleasePayload {
            tenant_id: billing_session.tenant_id,
            request_id: billing_session.request_id.clone(),
            is_stream: billing_session.is_stream,
            release_reason: None,
            upstream_status_code: None,
            upstream_error_code: None,
            failover_action: None,
            failover_reason_class: None,
            recovery_action: None,
            recovery_outcome: None,
            cross_account_failover_attempted: None,
        },
    )
    .await?;
    if release.status != "released" {
        state
            .billing_idempotent_hit_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    record_preauth_error_ratio_sample(
        state,
        &billing_session.model,
        billing_session.reserved_microcredits,
        capture.charged_microcredits,
    );
    state
        .billing_settle_complete_total
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    Ok(BillingSettleResult {
        authorization_id: billing_session.authorization_id,
        capture_status,
        input_tokens: usage_tokens.input_tokens,
        cached_input_tokens: usage_tokens.cached_input_tokens,
        output_tokens: usage_tokens.output_tokens,
        reasoning_tokens: usage_tokens.reasoning_tokens,
    })
}

async fn trigger_internal_billing_capture_with_retry(
    state: &AppState,
    payload: InternalBillingCapturePayload,
) -> std::result::Result<InternalBillingCaptureResponse, Box<Response>> {
    let mut attempt: u32 = 0;
    loop {
        match trigger_internal_billing_capture(state, &payload).await {
            Ok(response) => return Ok(response),
            Err(error_response) => {
                let retryable = is_retryable_billing_error(error_response.as_ref());
                attempt = attempt.saturating_add(1);
                if !retryable || attempt >= state.billing_capture_retry_max {
                    return Err(error_response);
                }
                tokio::time::sleep(state.billing_capture_retry_backoff).await;
            }
        }
    }
}

fn is_retryable_billing_error(response: &Response) -> bool {
    response.status() == StatusCode::BAD_GATEWAY
        || response.status() == StatusCode::SERVICE_UNAVAILABLE
}

fn is_billable_path(path: &str, method: &str) -> bool {
    if !method.eq_ignore_ascii_case("POST") {
        return false;
    }
    matches!(
        path,
        "/v1/responses"
            | "/v1/responses/compact"
            | "/backend-api/codex/responses"
            | "/backend-api/codex/responses/compact"
            | "/v1/chat/completions"
    )
}

async fn trigger_internal_billing_authorize(
    state: &AppState,
    payload: InternalBillingAuthorizePayload,
) -> std::result::Result<InternalBillingAuthorizeResponse, Box<Response>> {
    post_internal_billing_json(state, "authorize", &payload).await
}

async fn trigger_internal_billing_capture(
    state: &AppState,
    payload: &InternalBillingCapturePayload,
) -> std::result::Result<InternalBillingCaptureResponse, Box<Response>> {
    post_internal_billing_json(state, "capture", payload).await
}

async fn trigger_internal_billing_release(
    state: &AppState,
    payload: InternalBillingReleasePayload,
) -> std::result::Result<InternalBillingReleaseResponse, Box<Response>> {
    post_internal_billing_json(state, "release", &payload).await
}

async fn post_internal_billing_json<TReq, TResp>(
    state: &AppState,
    operation: &str,
    payload: &TReq,
) -> std::result::Result<TResp, Box<Response>>
where
    TReq: Serialize + ?Sized,
    TResp: serde::de::DeserializeOwned,
{
    let Some(base_url) = state.control_plane_base_url.as_deref() else {
        return Err(Box::new(json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "billing_service_unavailable",
            "control plane base url is not configured for billing",
        )));
    };
    let endpoint = format!(
        "{}/internal/v1/billing/{operation}",
        base_url.trim_end_matches('/')
    );
    let billing_context = internal_billing_log_context(operation, payload);
    let response = state
        .http_client
        .post(endpoint.clone())
        .bearer_auth(state.control_plane_internal_auth_token.as_ref())
        .json(payload)
        .timeout(Duration::from_secs(INTERNAL_BILLING_TIMEOUT_SEC))
        .send()
        .await
        .map_err(|err| {
            warn!(
                operation = operation,
                endpoint = %endpoint,
                error = %err,
                error_chain = %format!("{err:#}"),
                error_kind = classify_reqwest_error(&err),
                billing = %billing_context,
                "failed to call internal billing endpoint"
            );
            Box::new(json_error(
                StatusCode::BAD_GATEWAY,
                "billing_service_error",
                "failed to call billing service",
            ))
        })?;

    if response.status().is_success() {
        return response.json::<TResp>().await.map_err(|err| {
            warn!(
                operation = operation,
                endpoint = %endpoint,
                error = %err,
                error_chain = %format!("{err:#}"),
                error_kind = classify_reqwest_error(&err),
                billing = %billing_context,
                "failed to decode internal billing response payload"
            );
            Box::new(json_error(
                StatusCode::BAD_GATEWAY,
                "billing_service_error",
                "invalid billing service response payload",
            ))
        });
    }

    let status = response.status();
    let envelope = response.json::<InternalBillingErrorEnvelope>().await.ok();
    if let Some(item) = envelope.as_ref() {
        warn!(
            operation = operation,
            endpoint = %endpoint,
            status = status.as_u16(),
            code = %item.error.code,
            message = %item.error.message,
            billing = %billing_context,
            "internal billing endpoint returned error"
        );
    } else {
        warn!(
            operation = operation,
            endpoint = %endpoint,
            status = status.as_u16(),
            billing = %billing_context,
            "internal billing endpoint returned non-json error"
        );
    }
    let (mapped_status, code, message) = match status {
        StatusCode::BAD_REQUEST => envelope
            .as_ref()
            .map(|item| {
                (
                    StatusCode::BAD_REQUEST,
                    item.error.code.as_str(),
                    item.error.message.as_str(),
                )
            })
            .unwrap_or((
                StatusCode::BAD_REQUEST,
                "billing_rejected",
                "billing request rejected",
            )),
        StatusCode::PAYMENT_REQUIRED => (
            StatusCode::PAYMENT_REQUIRED,
            "insufficient_credits",
            "insufficient credits",
        ),
        StatusCode::NOT_FOUND => (
            StatusCode::BAD_GATEWAY,
            "billing_authorization_not_found",
            "billing authorization not found",
        ),
        _ => (
            StatusCode::BAD_GATEWAY,
            "billing_service_error",
            "billing service error",
        ),
    };

    Err(Box::new(json_error(mapped_status, code, message)))
}

fn internal_billing_log_context<TReq>(operation: &str, payload: &TReq) -> Value
where
    TReq: Serialize + ?Sized,
{
    let mut context = serde_json::Map::new();
    context.insert("operation".to_string(), Value::String(operation.to_string()));

    let Some(payload) = serde_json::to_value(payload).ok() else {
        return Value::Object(context);
    };
    let Some(payload) = payload.as_object() else {
        return Value::Object(context);
    };

    for key in [
        "tenant_id",
        "api_key_id",
        "request_id",
        "trace_request_id",
        "session_key",
        "model",
        "service_tier",
        "request_kind",
        "reserved_microcredits",
        "ttl_sec",
        "input_tokens",
        "cached_input_tokens",
        "output_tokens",
        "reasoning_tokens",
        "is_stream",
        "release_reason",
        "upstream_status_code",
        "upstream_error_code",
        "failover_action",
        "failover_reason_class",
        "recovery_action",
        "recovery_outcome",
        "cross_account_failover_attempted",
    ] {
        if let Some(value) = payload.get(key).filter(|value| !value.is_null()) {
            context.insert(key.to_string(), value.clone());
        }
    }

    Value::Object(context)
}

fn classify_reqwest_error(err: &reqwest::Error) -> &'static str {
    if err.is_timeout() {
        return "timeout";
    }
    if err.is_connect() {
        return "connect";
    }
    if err.is_request() {
        return "request";
    }
    if err.is_body() {
        return "body";
    }
    if err.is_decode() {
        return "decode";
    }
    if err.is_redirect() {
        return "redirect";
    }
    if err.is_builder() {
        return "builder";
    }
    "other"
}

async fn release_billing_hold_best_effort(
    state: Arc<AppState>,
    billing_session: Option<BillingSession>,
    failure_context: Option<BillingReleaseFailureContext>,
) {
    let Some(billing_session) = billing_session else {
        return;
    };

    state
        .billing_release_total
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    state
        .billing_release_without_capture_total
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if let Err(err) = trigger_internal_billing_release(
        state.as_ref(),
        InternalBillingReleasePayload {
            tenant_id: billing_session.tenant_id,
            request_id: billing_session.request_id,
            is_stream: billing_session.is_stream,
            release_reason: failure_context
                .as_ref()
                .and_then(|context| context.release_reason.clone()),
            upstream_status_code: failure_context
                .as_ref()
                .and_then(|context| context.upstream_status_code),
            upstream_error_code: failure_context
                .as_ref()
                .and_then(|context| context.upstream_error_code.clone()),
            failover_action: failure_context
                .as_ref()
                .and_then(|context| context.failover_action.clone()),
            failover_reason_class: failure_context
                .as_ref()
                .and_then(|context| context.failover_reason_class.clone()),
            recovery_action: failure_context
                .as_ref()
                .and_then(|context| context.recovery_action.clone()),
            recovery_outcome: failure_context
                .as_ref()
                .and_then(|context| context.recovery_outcome.clone()),
            cross_account_failover_attempted: failure_context
                .as_ref()
                .and_then(|context| context.cross_account_failover_attempted),
        },
    )
    .await
    {
        warn!(
            status = ?err.status(),
            "failed to release billing authorization on best effort path"
        );
    }
}

async fn stream_response_with_first_chunk(
    state: Arc<AppState>,
    status: StatusCode,
    headers: &HeaderMap,
    upstream_response: reqwest::Response,
    billing_session: Option<BillingSession>,
    stream_log_context: Option<StreamRequestLogContext>,
) -> Result<Response, StreamPreludeError> {
    let mut upstream_stream: UpstreamByteStream = Box::pin(upstream_response.bytes_stream());
    let mut prelude_chunks = Vec::new();
    let mut prelude_bytes = Vec::new();
    loop {
        match upstream_stream.next().await {
            Some(Ok(chunk)) if !chunk.is_empty() => {
                prelude_bytes.extend_from_slice(chunk.as_ref());
                prelude_chunks.push(chunk);
            }
            Some(Ok(_)) => continue,
            Some(Err(err)) => return Err(StreamPreludeError::UpstreamReadFailed(err.to_string())),
            None => return Err(StreamPreludeError::EndOfStreamBeforeCommit),
        }

        if let Some(error_context) =
            parse_stream_prelude_error_context(status, headers, &prelude_bytes)
        {
            return Err(StreamPreludeError::UpstreamErrorResponse(error_context));
        }

        if prelude_bytes.len() >= STREAM_PRELUDE_BUFFER_MAX_BYTES
            || prelude_chunks.len() >= STREAM_PRELUDE_BUFFER_MAX_CHUNKS
            || !should_continue_stream_prelude(&prelude_bytes)
        {
            break;
        }
    }

    let first_token_latency_ms = started_first_token_latency_ms(&billing_session, &stream_log_context);
    let mut billing_session = billing_session;
    if let Some(session) = billing_session.as_mut() {
        session.first_token_latency_ms = first_token_latency_ms;
    }

    let (tx, rx) =
        mpsc::channel::<std::result::Result<bytes::Bytes, io::Error>>(STREAM_PROXY_BUFFER_SIZE);
    let state_for_task = state.clone();
    tokio::spawn(async move {
        let mut tracker = SseUsageTracker::default();
        let mut sender_closed = false;
        for chunk in prelude_chunks {
            tracker.observe_chunk(&chunk);
            if tx.send(Ok(chunk)).await.is_err() {
                sender_closed = true;
                break;
            }
        }

        while !sender_closed {
            match upstream_stream.next().await {
                Some(Ok(chunk)) => {
                    tracker.observe_chunk(&chunk);
                    if tx.send(Ok(chunk)).await.is_err() {
                        sender_closed = true;
                        break;
                    }
                }
                Some(Err(err)) => {
                    let _ = tx.send(Err(io::Error::other(err))).await;
                    break;
                }
                None => break,
            }
        }

        if sender_closed {
            drain_upstream_stream_until_timeout(
                state_for_task.as_ref(),
                &mut upstream_stream,
                &mut tracker,
            )
            .await;
        }
        drop(tx);

        let observation = tracker.finish_usage();
        if let Some(billing_session) = billing_session {
            finalize_stream_billing(state_for_task, billing_session, observation).await;
        } else if let Some(stream_log_context) = stream_log_context {
            emit_stream_usage_request_log_event(
                state_for_task.as_ref(),
                stream_log_context,
                first_token_latency_ms,
                observation,
            )
            .await;
        }
    });

    let body = Body::from_stream(ReceiverStream::new(rx));
    Ok(response_with_body(status, headers, body))
}

fn should_continue_stream_prelude(buffered: &[u8]) -> bool {
    let mut line_start = 0usize;

    for (index, byte) in buffered.iter().enumerate() {
        if *byte != b'\n' {
            continue;
        }

        let mut line = &buffered[line_start..index];
        if line.last() == Some(&b'\r') {
            line = &line[..line.len() - 1];
        }
        let line = trim_ascii(line);
        line_start = index + 1;

        if line.is_empty()
            || line.starts_with(b":")
            || line.starts_with(b"event:")
            || line.starts_with(b"id:")
            || line.starts_with(b"retry:")
        {
            continue;
        }

        let Some(payload) = line.strip_prefix(b"data:") else {
            return false;
        };
        let payload = trim_ascii(payload);
        if payload.is_empty() || payload == b"[DONE]" {
            continue;
        }

        let Ok(value) = serde_json::from_slice::<Value>(payload) else {
            return false;
        };
        if !is_stream_prelude_metadata_payload(&value) {
            return false;
        }
    }

    !buffered.is_empty()
}

fn is_stream_prelude_metadata_payload(value: &Value) -> bool {
    value
        .get("type")
        .and_then(Value::as_str)
        .map(|kind| matches!(kind, "response.created" | "response.in_progress"))
        .unwrap_or(false)
}

fn started_first_token_latency_ms(
    billing_session: &Option<BillingSession>,
    stream_log_context: &Option<StreamRequestLogContext>,
) -> Option<u64> {
    if let Some(session) = billing_session.as_ref() {
        return Some(session.request_started.elapsed().as_millis() as u64);
    }
    stream_log_context
        .as_ref()
        .map(|context| context.request_started.elapsed().as_millis() as u64)
}

async fn emit_stream_usage_request_log_event(
    state: &AppState,
    stream_log_context: StreamRequestLogContext,
    first_token_latency_ms: Option<u64>,
    observation: StreamUsageObservation,
) {
    let usage = observation.usage.or_else(|| {
        let estimated_output_tokens = observation.estimated_output_tokens.unwrap_or(0).max(0);
        let estimated_input_tokens = stream_log_context
            .estimated_input_tokens
            .unwrap_or(0)
            .max(0);
        if estimated_input_tokens == 0 && estimated_output_tokens == 0 {
            return None;
        }
        Some(UsageTokens {
            input_tokens: estimated_input_tokens,
            cached_input_tokens: 0,
            output_tokens: estimated_output_tokens,
            reasoning_tokens: 0,
        })
    });

    state
        .event_sink
        .emit_request_log(RequestLogEvent {
            id: Uuid::new_v4(),
            account_id: stream_log_context.account_id,
            tenant_id: stream_log_context.tenant_id,
            api_key_id: stream_log_context.api_key_id,
            event_version: 2,
            path: stream_log_context.request_path,
            method: stream_log_context.request_method,
            status_code: StatusCode::OK.as_u16(),
            latency_ms: stream_log_context.request_started.elapsed().as_millis() as u64,
            is_stream: true,
            error_code: None,
            request_id: stream_log_context.request_id,
            model: stream_log_context.model,
            service_tier: observation
                .service_tier
                .or(stream_log_context.requested_service_tier),
            input_tokens: usage.as_ref().map(|item| item.input_tokens),
            cached_input_tokens: usage.as_ref().map(|item| item.cached_input_tokens),
            output_tokens: usage.as_ref().map(|item| item.output_tokens),
            reasoning_tokens: usage.as_ref().map(|item| item.reasoning_tokens),
            first_token_latency_ms,
            billing_phase: Some("stream_observed".to_string()),
            authorization_id: None,
            capture_status: None,
            created_at: chrono::Utc::now(),
        })
        .await;
}

fn parse_stream_prelude_error_context(
    status: StatusCode,
    headers: &HeaderMap,
    chunk: &[u8],
) -> Option<UpstreamErrorContext> {
    if let Some(context) = parse_json_error_payload(status, headers, chunk) {
        return Some(context);
    }

    let text = std::str::from_utf8(chunk).ok()?;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let payload = match line.strip_prefix("data:") {
            Some(value) => value.trim_start(),
            None => continue,
        };
        if payload == "[DONE]" || payload.is_empty() {
            continue;
        }
        if let Some(context) = parse_json_error_payload(status, headers, payload.as_bytes()) {
            return Some(context);
        }
        if let Some(context) = build_upstream_error_context(status, headers, payload.as_bytes()) {
            return Some(context);
        }
    }

    build_upstream_error_context(status, headers, chunk)
}

fn parse_json_error_payload(
    status: StatusCode,
    headers: &HeaderMap,
    payload: &[u8],
) -> Option<UpstreamErrorContext> {
    let value = serde_json::from_slice::<Value>(payload).ok()?;
    if !is_stream_error_payload(&value) {
        return None;
    }
    let normalized = normalize_stream_error_payload(&value);
    build_upstream_error_context(status, headers, &normalized)
}

fn normalize_stream_error_payload(value: &Value) -> Vec<u8> {
    let mut normalized = value.clone();
    let incomplete_reason = normalized
        .get("response")
        .and_then(|response| response.get("incomplete_details"))
        .and_then(|details| details.get("reason"))
        .and_then(Value::as_str)
        .map(ToString::to_string);
    if normalized.get("error").is_none() {
        if let Some(error) = normalized
            .get("response")
            .and_then(|response| response.get("error"))
            .cloned()
        {
            normalized["error"] = error;
        } else if let Some(reason) = incomplete_reason {
            normalized["code"] = Value::String(reason.clone());
            normalized["message"] =
                Value::String(format!("Incomplete response returned, reason: {reason}"));
        }
    }

    serde_json::to_vec(&normalized).unwrap_or_default()
}

fn is_stream_error_payload(value: &Value) -> bool {
    if value.get("error").is_some() {
        return true;
    }

    if value.get("code").is_some() && value.get("message").is_some() {
        return true;
    }

    value
        .get("type")
        .and_then(Value::as_str)
        .map(|kind| {
            matches!(
                kind,
                "error" | "response.error" | "response.failed" | "response.incomplete"
            )
        })
        .unwrap_or(false)
}

async fn drain_upstream_stream_until_timeout(
    state: &AppState,
    upstream_stream: &mut UpstreamByteStream,
    tracker: &mut SseUsageTracker,
) {
    let deadline = Instant::now() + state.stream_billing_drain_timeout;
    loop {
        let now = Instant::now();
        if now >= deadline {
            state
                .stream_drain_timeout_total
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            break;
        }
        let remaining = deadline.saturating_duration_since(now);
        match tokio::time::timeout(remaining, upstream_stream.next()).await {
            Ok(Some(Ok(chunk))) => tracker.observe_chunk(&chunk),
            Ok(Some(Err(_))) => break,
            Ok(None) => break,
            Err(_) => {
                state
                    .stream_drain_timeout_total
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                break;
            }
        }
    }
}

async fn finalize_stream_billing(
    state: Arc<AppState>,
    mut billing_session: BillingSession,
    observation: StreamUsageObservation,
) {
    if observation.used_json_line_fallback {
        state
            .stream_usage_json_line_fallback_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    if billing_session.effective_service_tier.is_none() {
        billing_session.effective_service_tier = observation
            .service_tier
            .clone()
            .or_else(|| billing_session.requested_service_tier.clone());
    }

    let mut usage = observation.usage;
    if usage.is_none() && STREAM_USAGE_ESTIMATE_FALLBACK_ENABLED {
        if let Some(estimated_output_tokens) = observation.estimated_output_tokens {
            let estimated_input_tokens = billing_session.estimated_input_tokens.max(0);
            usage = Some(UsageTokens {
                input_tokens: estimated_input_tokens,
                cached_input_tokens: 0,
                output_tokens: estimated_output_tokens.max(0),
                reasoning_tokens: 0,
            });
            state
                .stream_usage_estimated_total
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            warn!(
                request_id = %billing_session.request_id,
                authorization_id = %billing_session.authorization_id,
                estimated_input_tokens,
                estimated_output_tokens,
                usage_confidence = "low",
                "stream usage missing; applying low-confidence token estimate fallback"
            );
        }
    }

    let Some(usage_tokens) = usage else {
        emit_stream_request_log_event(
            state.as_ref(),
            &billing_session,
            "released_missing_usage",
            None,
            None,
            None,
            None,
            None,
        )
        .await;
        state
            .stream_usage_missing_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        release_billing_hold_best_effort(
            state,
            Some(billing_session),
            Some(BillingReleaseFailureContext {
                release_reason: Some("stream_usage_missing".to_string()),
                failover_action: Some("return_failure".to_string()),
                failover_reason_class: Some("stream_usage_missing".to_string()),
                ..Default::default()
            }),
        )
        .await;
        return;
    };

    match settle_authorized_billing(
        state.as_ref(),
        &billing_session,
        usage_tokens,
        billing_session.effective_service_tier.as_deref(),
    )
    .await
    {
        Ok(settle_result) => {
            emit_stream_request_log_event(
                state.as_ref(),
                &billing_session,
                "released",
                Some(settle_result.capture_status.as_str()),
                Some(settle_result.input_tokens),
                Some(settle_result.cached_input_tokens),
                Some(settle_result.output_tokens),
                Some(settle_result.reasoning_tokens),
            )
            .await;
        }
        Err(err) => {
            warn!(
                status = ?err.status(),
                request_id = %billing_session.request_id,
                authorization_id = %billing_session.authorization_id,
                reserved_microcredits = billing_session.reserved_microcredits,
                "stream billing finalize failed"
            );
            emit_stream_request_log_event(
                state.as_ref(),
                &billing_session,
                "released_failed",
                None,
                Some(usage_tokens.input_tokens),
                Some(usage_tokens.cached_input_tokens),
                Some(usage_tokens.output_tokens),
                Some(usage_tokens.reasoning_tokens),
            )
            .await;
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn emit_stream_request_log_event(
    state: &AppState,
    billing_session: &BillingSession,
    billing_phase: &str,
    capture_status: Option<&str>,
    input_tokens: Option<i64>,
    cached_input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    reasoning_tokens: Option<i64>,
) {
    state
        .event_sink
        .emit_request_log(RequestLogEvent {
            id: Uuid::new_v4(),
            account_id: billing_session.account_id,
            tenant_id: Some(billing_session.tenant_id),
            api_key_id: Some(billing_session.api_key_id),
            event_version: 2,
            path: billing_session.request_path.clone(),
            method: billing_session.request_method.clone(),
            status_code: StatusCode::OK.as_u16(),
            latency_ms: billing_session.request_started.elapsed().as_millis() as u64,
            is_stream: true,
            error_code: None,
            request_id: billing_session
                .trace_request_id
                .clone()
                .or_else(|| Some(billing_session.request_id.clone())),
            model: Some(billing_session.model.clone()),
            service_tier: billing_session
                .effective_service_tier
                .clone()
                .or_else(|| billing_session.requested_service_tier.clone()),
            input_tokens,
            cached_input_tokens,
            output_tokens,
            reasoning_tokens,
            first_token_latency_ms: billing_session.first_token_latency_ms,
            billing_phase: Some(billing_phase.to_string()),
            authorization_id: Some(billing_session.authorization_id),
            capture_status: capture_status.map(ToString::to_string),
            created_at: chrono::Utc::now(),
        })
        .await;
}

impl SseUsageTracker {
    fn observe_chunk(&mut self, chunk: &bytes::Bytes) {
        self.line_buffer.extend_from_slice(chunk);
        while let Some(position) = self.line_buffer.iter().position(|item| *item == b'\n') {
            let mut line = self.line_buffer.drain(..=position).collect::<Vec<_>>();
            if line.last() == Some(&b'\n') {
                line.pop();
            }
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            self.observe_line(&line);
        }
    }

    fn finish_usage(mut self) -> StreamUsageObservation {
        if !self.line_buffer.is_empty() {
            let line = std::mem::take(&mut self.line_buffer);
            self.observe_line(trim_ascii(&line));
        }
        let estimated_output_tokens = if self.output_text_chars == 0 {
            None
        } else {
            Some(rough_token_estimate_from_char_count(self.output_text_chars))
        };
        StreamUsageObservation {
            usage: self.usage,
            service_tier: self.service_tier,
            estimated_output_tokens,
            used_json_line_fallback: self.used_json_line_fallback,
        }
    }

    fn observe_line(&mut self, line: &[u8]) {
        if line.is_empty() {
            return;
        }
        let (payload, from_data_line) = if let Some(raw_payload) = line.strip_prefix(b"data:") {
            (trim_ascii(raw_payload), true)
        } else {
            // Fallback for non-standard stream payloads that carry plain JSON lines.
            (trim_ascii(line), false)
        };
        if payload.is_empty() || payload == b"[DONE]" {
            return;
        }
        if let Ok(value) = serde_json::from_slice::<Value>(payload) {
            if self.service_tier.is_none() {
                self.service_tier = extract_service_tier_from_value(&value);
            }
            if let Some(tokens) = extract_usage_tokens_from_value(&value) {
                self.usage = Some(tokens);
                if !from_data_line {
                    self.used_json_line_fallback = true;
                }
            }
            if let Some(event_type) = value.get("type").and_then(Value::as_str) {
                if event_type == "response.output_text.delta" {
                    if let Some(delta) = value.get("delta").and_then(Value::as_str) {
                        self.output_text_chars =
                            self.output_text_chars.saturating_add(delta.chars().count());
                        self.saw_output_text_delta = true;
                    }
                } else if event_type == "response.output_text.done" && !self.saw_output_text_delta {
                    if let Some(text) = value.get("text").and_then(Value::as_str) {
                        self.output_text_chars =
                            self.output_text_chars.saturating_add(text.chars().count());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod billing_stream_logging_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn internal_billing_log_context_includes_authorize_identifiers() {
        let payload = InternalBillingAuthorizePayload {
            tenant_id: Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap(),
            api_key_id: Some(Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap()),
            request_id: "internal-billing-request".to_string(),
            trace_request_id: Some("external-request".to_string()),
            model: "gpt-5.4".to_string(),
            service_tier: Some("default".to_string()),
            session_key: Some("session-key".to_string()),
            request_kind: Some("response".to_string()),
            reserved_microcredits: 1234,
            ttl_sec: Some(900),
            is_stream: true,
        };

        let context = internal_billing_log_context("authorize", &payload);

        assert_eq!(
            context,
            json!({
                "operation": "authorize",
                "tenant_id": "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
                "api_key_id": "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
                "request_id": "internal-billing-request",
                "trace_request_id": "external-request",
                "session_key": "session-key",
                "model": "gpt-5.4",
                "service_tier": "default",
                "request_kind": "response",
                "reserved_microcredits": 1234,
                "ttl_sec": 900,
                "is_stream": true
            })
        );
    }
}
