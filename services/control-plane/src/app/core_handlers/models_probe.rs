fn provider_label_for_mode(mode: &UpstreamMode) -> &'static str {
    match mode {
        UpstreamMode::ChatGptSession => "chatgpt-session",
        UpstreamMode::CodexOauth => "codex-oauth",
        UpstreamMode::OpenAiApiKey => "openai",
    }
}

fn parse_model_item(
    item: &serde_json::Value,
    fallback_provider: &str,
) -> Option<AdminModelItem> {
    let id = item
        .get("id")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())?
        .to_string();
    let object = item
        .get("object")
        .and_then(|value| value.as_str())
        .unwrap_or("model")
        .to_string();
    let created = item.get("created").and_then(|value| value.as_i64()).unwrap_or(0);
    let owned_by = item
        .get("owned_by")
        .and_then(|value| value.as_str())
        .unwrap_or(fallback_provider)
        .to_string();
    let visibility = item
        .get("visibility")
        .and_then(|value| value.as_str())
        .map(ToString::to_string);

    Some(AdminModelItem {
        id,
        object,
        created,
        owned_by,
        entity_id: None,
        visibility,
        in_catalog: true,
        availability_status: AdminModelAvailabilityStatus::Unknown,
        availability_checked_at: None,
        availability_http_status: None,
        availability_error: None,
    })
}

fn cached_models_catalog_context(state: &AppState, allow_stale: bool) -> Option<ModelsCatalogContext> {
    let now = Utc::now();
    let ttl_sec = state.model_catalog_cache_ttl_sec.max(1);
    let cache = state
        .model_catalog_cache
        .read()
        .expect("model_catalog_cache lock poisoned");
    let context = cache.context.clone()?;
    if allow_stale {
        return Some(context);
    }
    let is_fresh = cache
        .updated_at
        .map(|updated_at| now.signed_duration_since(updated_at).num_seconds() < ttl_sec)
        .unwrap_or(false);
    if is_fresh {
        Some(context)
    } else {
        None
    }
}

fn update_models_catalog_cache(state: &AppState, context: &ModelsCatalogContext) {
    let mut cache = state
        .model_catalog_cache
        .write()
        .expect("model_catalog_cache lock poisoned");
    cache.updated_at = Some(Utc::now());
    cache.source_account_label = Some(context.account_label.clone());
    cache.context = Some(context.clone());
}

fn set_model_catalog_last_error(state: &AppState, error: Option<String>) {
    let mut last_error = state
        .model_catalog_last_error
        .write()
        .expect("model_catalog_last_error lock poisoned");
    *last_error = error;
}

fn preferred_models_account_label(state: &AppState) -> Option<String> {
    let probe_source = state
        .model_probe_cache
        .read()
        .expect("model_probe_cache lock poisoned")
        .source_account_label
        .clone();
    if probe_source.is_some() {
        return probe_source;
    }

    state
        .model_catalog_cache
        .read()
        .expect("model_catalog_cache lock poisoned")
        .source_account_label
        .clone()
}

fn prioritize_and_limit_models_catalog_accounts(
    mut accounts: Vec<UpstreamAccount>,
    preferred_label: Option<&str>,
    max_attempts: usize,
) -> Vec<UpstreamAccount> {
    if let Some(label) = preferred_label {
        if let Some(index) = accounts.iter().position(|account| account.label == label) {
            accounts.swap(0, index);
        }
    }

    let max_attempts = max_attempts.max(1);
    if accounts.len() > max_attempts {
        accounts.truncate(max_attempts);
    }
    accounts
}

async fn fetch_models_catalog_context_from_account(
    client: &reqwest::Client,
    account: UpstreamAccount,
) -> Result<ModelsCatalogContext, String> {
    let models_url = crate::upstream_api::build_upstream_models_url(&account.base_url, &account.mode)
        .map_err(|err| format!("{}: invalid base_url ({err})", account.label))?;
    let account_label = account.label.clone();

    let mut request = client
        .get(models_url)
        .header("authorization", format!("Bearer {}", account.bearer_token));
    if let Some(account_id) = account.chatgpt_account_id.as_deref() {
        request = request.header("chatgpt-account-id", account_id);
    }

    let response = request
        .send()
        .await
        .map_err(|err| format!("{account_label}: {err}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "{}: upstream returned {}",
            account_label,
            response.status()
        ));
    }

    let payload = response
        .json::<serde_json::Value>()
        .await
        .map_err(|err| format!("{account_label}: failed to parse upstream models response: {err}"))?;
    let provider = provider_label_for_mode(&account.mode);
    let normalised = crate::upstream_api::normalise_models_payload(payload, &account.mode);
    let items = normalised
        .get("data")
        .and_then(|value| value.as_array())
        .map(|array| {
            array
                .iter()
                .filter_map(|item| parse_model_item(item, provider))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if items.is_empty() {
        return Err(format!(
            "{}: upstream returned empty models list",
            account_label
        ));
    }

    Ok(ModelsCatalogContext {
        account,
        account_label,
        items,
    })
}

async fn fetch_models_catalog_context_from_upstream(
    state: &AppState,
) -> Result<ModelsCatalogContext, (StatusCode, Json<ErrorEnvelope>)> {
    let snapshot = state.store.snapshot().await.map_err(internal_error)?;
    let enabled_accounts = snapshot
        .accounts
        .into_iter()
        .filter(|account| account.enabled)
        .collect::<Vec<_>>();
    if enabled_accounts.is_empty() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorEnvelope::new(
                "no_upstream_account",
                "no enabled upstream account is available",
            )),
        ));
    }

    let preferred_label = preferred_models_account_label(state);
    let accounts = prioritize_and_limit_models_catalog_accounts(
        enabled_accounts,
        preferred_label.as_deref(),
        state.model_catalog_fetch_attempt_limit,
    );
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(state.model_catalog_request_timeout_sec))
        .build()
        .map_err(|err| internal_error(err.into()))?;
    let mut error_messages = Vec::new();

    let concurrency = state.model_catalog_fetch_concurrency.max(1);
    let mut attempts = futures_util::stream::iter(accounts.into_iter().map(|account| {
        let client = client.clone();
        async move { fetch_models_catalog_context_from_account(&client, account).await }
    }))
    .buffer_unordered(concurrency);

    while let Some(result) = attempts.next().await {
        match result {
            Ok(context) => {
                push_admin_log(
                    state,
                    "info",
                    "admin.models.list",
                    format!(
                        "loaded models from upstream account {}",
                        context.account_label
                    ),
                );
                return Ok(context);
            }
            Err(error) => {
                error_messages.push(error);
            }
        }
    }

    if !error_messages.is_empty() {
        tracing::warn!(
            errors = ?error_messages,
            "failed to fetch models catalog from all available upstream accounts"
        );
    }

    Err((
        StatusCode::BAD_GATEWAY,
        Json(ErrorEnvelope::new(
            "upstream_models_unavailable",
            "failed to fetch models from all available accounts",
        )),
    ))
}

async fn load_models_catalog_context(
    state: &AppState,
    force_refresh: bool,
) -> Result<ModelsCatalogContext, (StatusCode, Json<ErrorEnvelope>)> {
    if !force_refresh {
        if let Some(context) = cached_models_catalog_context(state, false) {
            return Ok(context);
        }
    }

    match fetch_models_catalog_context_from_upstream(state).await {
        Ok(context) => {
            update_models_catalog_cache(state, &context);
            set_model_catalog_last_error(state, None);
            Ok(context)
        }
        Err((status, envelope)) => {
            set_model_catalog_last_error(
                state,
                Some(format!(
                    "{}: {}",
                    envelope.0.error.code, envelope.0.error.message
                )),
            );
            if let Some(context) = cached_models_catalog_context(state, true) {
                let (stale_age_sec, source_account_label) = {
                    let cache = state
                        .model_catalog_cache
                        .read()
                        .expect("model_catalog_cache lock poisoned");
                    let stale_age_sec = cache
                        .updated_at
                        .map(|updated_at| Utc::now().signed_duration_since(updated_at).num_seconds())
                        .unwrap_or_default();
                    (stale_age_sec, cache.source_account_label.clone())
                };
                tracing::warn!(
                    status = %status,
                    code = %envelope.0.error.code,
                    message = %envelope.0.error.message,
                    stale_age_sec,
                    source_account_label = ?source_account_label,
                    "falling back to stale models catalog cache after upstream fetch failure"
                );
                schedule_models_catalog_retry_if_needed(state, "stale_fallback");
                return Ok(context);
            }
            Err((status, envelope))
        }
    }
}

fn schedule_models_catalog_retry_if_needed(state: &AppState, trigger: &str) {
    if state
        .model_catalog_retry_inflight
        .compare_exchange(
            false,
            true,
            std::sync::atomic::Ordering::AcqRel,
            std::sync::atomic::Ordering::Acquire,
        )
        .is_err()
    {
        return;
    }

    let state_cloned = state.clone();
    let trigger = trigger.to_string();
    tokio::spawn(async move {
        let refresh_result = fetch_models_catalog_context_from_upstream(&state_cloned).await;
        match refresh_result {
            Ok(context) => {
                update_models_catalog_cache(&state_cloned, &context);
                set_model_catalog_last_error(&state_cloned, None);
                tracing::info!(
                    trigger = %trigger,
                    source_account_label = %context.account_label,
                    "background models catalog refresh succeeded"
                );
            }
            Err((status, envelope)) => {
                set_model_catalog_last_error(
                    &state_cloned,
                    Some(format!(
                        "{}: {}",
                        envelope.0.error.code, envelope.0.error.message
                    )),
                );
                tracing::warn!(
                    trigger = %trigger,
                    status = %status,
                    code = %envelope.0.error.code,
                    message = %envelope.0.error.message,
                    "background models catalog refresh failed"
                );
            }
        }
        state_cloned
            .model_catalog_retry_inflight
            .store(false, std::sync::atomic::Ordering::Release);
    });
}

fn build_unlisted_model_item(model_id: &str, provider: &str) -> AdminModelItem {
    AdminModelItem {
        id: model_id.to_string(),
        object: "model".to_string(),
        created: 0,
        owned_by: provider.to_string(),
        entity_id: None,
        visibility: None,
        in_catalog: false,
        availability_status: AdminModelAvailabilityStatus::Unknown,
        availability_checked_at: None,
        availability_http_status: None,
        availability_error: None,
    }
}

fn build_admin_models_response(
    state: &AppState,
    context: ModelsCatalogContext,
    model_entities: Vec<crate::tenant::AdminModelEntityItem>,
) -> AdminModelsResponse {
    let provider = provider_label_for_mode(&context.account.mode).to_string();
    let mut model_map: std::collections::BTreeMap<String, AdminModelItem> = context
        .items
        .into_iter()
        .map(|item| (item.id.clone(), item))
        .collect();

    for entity in model_entities {
        let model_id = entity.model.trim();
        if model_id.is_empty() {
            continue;
        }
        let mut item = model_map
            .remove(model_id)
            .unwrap_or_else(|| build_unlisted_model_item(model_id, &entity.provider));
        item.owned_by = entity.provider.clone();
        item.entity_id = Some(entity.id);
        item.visibility = entity.visibility.clone();
        item.in_catalog = true;
        model_map.insert(model_id.to_string(), item);
    }

    let (cache_updated_at, cache_source_label, cache_entries) = {
        let cache = state
            .model_probe_cache
            .read()
            .expect("model_probe_cache lock poisoned");
        (
            cache.updated_at,
            cache.source_account_label.clone(),
            cache.entries.clone(),
        )
    };
    let catalog_cache_source_label = {
        let cache = state
            .model_catalog_cache
            .read()
            .expect("model_catalog_cache lock poisoned");
        cache.source_account_label.clone()
    };
    let catalog_last_error = state
        .model_catalog_last_error
        .read()
        .expect("model_catalog_last_error lock poisoned")
        .clone();

    for model_id in state.model_probe_extra_models.iter() {
        if !model_map.contains_key(model_id) {
            model_map.insert(model_id.clone(), build_unlisted_model_item(model_id, &provider));
        }
    }
    for model_id in cache_entries.keys() {
        if !model_map.contains_key(model_id) {
            model_map.insert(
                model_id.clone(),
                build_unlisted_model_item(model_id, &provider),
            );
        }
    }

    for item in model_map.values_mut() {
        if let Some(probe) = cache_entries.get(&item.id) {
            item.availability_status = probe.status.clone();
            item.availability_checked_at = Some(probe.checked_at);
            item.availability_http_status = probe.http_status;
            item.availability_error = probe.error.clone();
        }
    }

    let mut data = model_map.into_values().collect::<Vec<_>>();
    data.sort_by(|left, right| {
        right
            .in_catalog
            .cmp(&left.in_catalog)
            .then(left.id.cmp(&right.id))
    });

    let now = Utc::now();
    let probe_cache_stale = cache_updated_at
        .map(|checked_at| now.signed_duration_since(checked_at).num_seconds() >= MODEL_PROBE_CACHE_TTL_SEC)
        .unwrap_or(true);

    AdminModelsResponse {
        object: "list".to_string(),
        data,
        meta: AdminModelsMeta {
            probe_cache_ttl_sec: MODEL_PROBE_CACHE_TTL_SEC,
            probe_cache_stale,
            probe_cache_updated_at: cache_updated_at,
            source_account_label: cache_source_label
                .or(catalog_cache_source_label)
                .or(Some(context.account_label)),
            catalog_last_error,
        },
    }
}

fn normalize_requested_model_ids(models: Vec<String>) -> Vec<String> {
    let mut ids = models
        .into_iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    ids
}

fn extract_probe_error_message(body: &str) -> Option<String> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(detail) = value.get("detail").and_then(|item| item.as_str()) {
            let detail = detail.trim();
            if !detail.is_empty() {
                return Some(detail.to_string());
            }
        }
        if let Some(message) = value
            .get("error")
            .and_then(|item| item.get("message"))
            .and_then(|item| item.as_str())
        {
            let message = message.trim();
            if !message.is_empty() {
                return Some(message.to_string());
            }
        }
        if let Some(message) = value.get("message").and_then(|item| item.as_str()) {
            let message = message.trim();
            if !message.is_empty() {
                return Some(message.to_string());
            }
        }
    }
    Some(trimmed.chars().take(240).collect())
}

async fn probe_single_model(
    client: &reqwest::Client,
    responses_url: &str,
    account: &UpstreamAccount,
    model_id: &str,
) -> ModelProbeCacheEntry {
    let payload = serde_json::json!({
        "model": model_id,
        "store": false,
        "stream": true,
        "instructions": "You are concise.",
        "input": [
            {
                "role": "user",
                "content": [
                    { "type": "input_text", "text": "reply with pong" }
                ]
            }
        ]
    });

    let mut request = client
        .post(responses_url)
        .header("authorization", format!("Bearer {}", account.bearer_token))
        .header("content-type", "application/json")
        .json(&payload);
    if let Some(account_id) = account.chatgpt_account_id.as_deref() {
        request = request.header("chatgpt-account-id", account_id);
    }

    let checked_at = Utc::now();
    match request.send().await {
        Ok(response) if response.status().is_success() => ModelProbeCacheEntry {
            status: AdminModelAvailabilityStatus::Available,
            checked_at,
            http_status: Some(response.status().as_u16()),
            error: None,
        },
        Ok(response) => {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            ModelProbeCacheEntry {
                status: AdminModelAvailabilityStatus::Unavailable,
                checked_at,
                http_status: Some(status.as_u16()),
                error: extract_probe_error_message(&body),
            }
        }
        Err(err) => ModelProbeCacheEntry {
            status: AdminModelAvailabilityStatus::Unavailable,
            checked_at,
            http_status: None,
            error: Some(err.to_string()),
        },
    }
}

async fn run_model_probe_cycle(
    state: &AppState,
    requested_models: Vec<String>,
    force: bool,
    trigger: &str,
) -> anyhow::Result<()> {
    let requested_models = normalize_requested_model_ids(requested_models);
    let cache_is_fresh = {
        let cache = state
            .model_probe_cache
            .read()
            .expect("model_probe_cache lock poisoned");
        cache
            .updated_at
            .map(|updated_at| {
                Utc::now()
                    .signed_duration_since(updated_at)
                    .num_seconds()
                    < MODEL_PROBE_CACHE_TTL_SEC
            })
            .unwrap_or(false)
    };
    if !force && requested_models.is_empty() && cache_is_fresh {
        return Ok(());
    }

    let context = load_models_catalog_context(state, false)
        .await
        .map_err(|(status, envelope)| {
            anyhow::anyhow!(
                "failed to load catalog for probing ({}): {}",
                status,
                envelope.error.message
            )
        })?;
    let responses_url = crate::upstream_api::build_upstream_responses_url(
        &context.account.base_url,
        &context.account.mode,
    )?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(MODEL_PROBE_REQUEST_TIMEOUT_SEC))
        .build()?;

    let mut candidate_ids = std::collections::BTreeSet::<String>::new();
    for item in &context.items {
        candidate_ids.insert(item.id.clone());
    }
    for item in state.model_probe_extra_models.iter() {
        candidate_ids.insert(item.clone());
    }
    if let Some(tenant_auth) = state.tenant_auth_service.as_ref() {
        match tenant_auth.admin_list_model_entities().await {
            Ok(entities) => {
                for entity in entities {
                    let model_id = entity.model.trim();
                    if model_id.is_empty() {
                        continue;
                    }
                    candidate_ids.insert(model_id.to_string());
                }
            }
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    "failed to list admin model entities for model probe candidates"
                );
            }
        }
    }
    for item in requested_models {
        candidate_ids.insert(item);
    }
    {
        let cache = state
            .model_probe_cache
            .read()
            .expect("model_probe_cache lock poisoned");
        for model_id in cache.entries.keys() {
            candidate_ids.insert(model_id.clone());
        }
    }

    let mut entries = HashMap::new();
    let mut available = 0usize;
    let tested = candidate_ids.len();
    for model_id in candidate_ids {
        let probe = probe_single_model(&client, &responses_url, &context.account, &model_id).await;
        if probe.status == AdminModelAvailabilityStatus::Available {
            available += 1;
        }
        entries.insert(model_id, probe);
    }

    if !force && tested > 0 && available == 0 {
        let (previous_available, previous_updated_at) = {
            let cache = state
                .model_probe_cache
                .read()
                .expect("model_probe_cache lock poisoned");
            let previous_available = cache
                .entries
                .values()
                .filter(|entry| entry.status == AdminModelAvailabilityStatus::Available)
                .count();
            (previous_available, cache.updated_at)
        };
        let previous_is_recent = previous_updated_at
            .map(|updated_at| {
                Utc::now().signed_duration_since(updated_at).num_seconds()
                    < MODEL_PROBE_CACHE_TTL_SEC.saturating_mul(2)
            })
            .unwrap_or(false);
        if previous_available > 0 && previous_is_recent {
            tracing::warn!(
                trigger = %trigger,
                tested,
                previous_available,
                source_account_label = %context.account_label,
                "model probe produced zero available models; keeping previous probe cache"
            );
            return Ok(());
        }
    }

    {
        let mut cache = state
            .model_probe_cache
            .write()
            .expect("model_probe_cache lock poisoned");
        cache.updated_at = Some(Utc::now());
        cache.source_account_label = Some(context.account_label.clone());
        cache.entries = entries;
    }

    push_admin_log(
        state,
        "info",
        "admin.models.probe",
        format!(
            "model probe ({trigger}) tested {tested} models via account {} (available={available}, unavailable={})",
            context.account_label,
            tested.saturating_sub(available)
        ),
    );
    Ok(())
}

#[cfg_attr(test, allow(dead_code))]
fn spawn_model_probe_loop(state: AppState) {
    let interval_sec = state.model_probe_interval_sec.max(60);
    tokio::spawn(async move {
        if let Err(err) = run_model_probe_cycle(&state, Vec::new(), true, "startup").await {
            tracing::warn!(error = %err, "initial model probe failed");
        }

        let mut ticker = tokio::time::interval(Duration::from_secs(interval_sec));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let _ = ticker.tick().await;
        loop {
            let _ = ticker.tick().await;
            if let Err(err) = run_model_probe_cycle(&state, Vec::new(), false, "auto").await {
                tracing::warn!(error = %err, "scheduled model probe failed");
            }
        }
    });
    tracing::info!(interval_sec, "model probe loop started");
}

#[cfg(test)]
mod model_catalog_helpers_tests {
    use super::prioritize_and_limit_models_catalog_accounts;
    use chrono::Utc;
    use codex_pool_core::model::{UpstreamAccount, UpstreamMode};
    use uuid::Uuid;

    fn account(label: &str) -> UpstreamAccount {
        UpstreamAccount {
            id: Uuid::new_v4(),
            label: label.to_string(),
            mode: UpstreamMode::CodexOauth,
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            bearer_token: "test-token".to_string(),
            chatgpt_account_id: None,
            enabled: true,
            priority: 100,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn prioritize_and_limit_models_catalog_accounts_moves_preferred_account_first() {
        let accounts = vec![account("a"), account("b"), account("c")];
        let ordered =
            prioritize_and_limit_models_catalog_accounts(accounts, Some("c"), usize::MAX);
        assert_eq!(ordered[0].label, "c");
        assert_eq!(ordered.len(), 3);
    }

    #[test]
    fn prioritize_and_limit_models_catalog_accounts_enforces_attempt_limit() {
        let accounts = vec![account("a"), account("b"), account("c"), account("d")];
        let ordered = prioritize_and_limit_models_catalog_accounts(accounts, None, 2);
        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[0].label, "a");
        assert_eq!(ordered[1].label, "b");
    }
}
