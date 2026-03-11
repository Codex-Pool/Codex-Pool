async fn build_admin_models_response(
    state: &AppState,
    official_catalog: Vec<crate::tenant::OpenAiModelCatalogItem>,
    pricing_overrides: Vec<crate::tenant::ModelPricingItem>,
) -> AdminModelsResponse {
    let pricing_overrides_by_model = pricing_overrides
        .into_iter()
        .map(|item| (item.model.clone(), item))
        .collect::<std::collections::HashMap<String, crate::tenant::ModelPricingItem>>();

    let (cache_updated_at, cache_source_label, mut cache_entries) = {
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
    overlay_recent_successful_request_models(state, &mut cache_entries).await;

    let catalog_last_error = state
        .model_catalog_last_error
        .read()
        .expect("model_catalog_last_error lock poisoned")
        .clone();

    let catalog_synced_at = official_catalog.iter().map(|item| item.synced_at).max();
    let data = build_admin_model_items(
        official_catalog,
        &pricing_overrides_by_model,
        &cache_entries,
    );

    let effective_cache_updated_at = cache_entries
        .values()
        .map(|entry| entry.checked_at)
        .max()
        .or(cache_updated_at);
    let now = Utc::now();
    let probe_cache_stale = effective_cache_updated_at
        .map(|checked_at| {
            now.signed_duration_since(checked_at).num_seconds() >= MODEL_PROBE_CACHE_TTL_SEC
        })
        .unwrap_or(true);

    AdminModelsResponse {
        object: "list".to_string(),
        data,
        meta: AdminModelsMeta {
            probe_cache_ttl_sec: MODEL_PROBE_CACHE_TTL_SEC,
            probe_cache_stale,
            probe_cache_updated_at: effective_cache_updated_at,
            probe_source_account_label: cache_source_label,
            catalog_synced_at,
            catalog_sync_required: catalog_synced_at.is_none(),
            catalog_last_error,
        },
    }
}

fn build_admin_model_items(
    official_catalog: Vec<crate::tenant::OpenAiModelCatalogItem>,
    pricing_overrides_by_model: &std::collections::HashMap<String, crate::tenant::ModelPricingItem>,
    cache_entries: &std::collections::HashMap<String, ModelProbeCacheEntry>,
) -> Vec<AdminModelItem> {
    let official_model_ids = official_catalog
        .iter()
        .map(|item| item.model_id.clone())
        .collect::<std::collections::BTreeSet<_>>();

    let mut data = official_catalog
        .into_iter()
        .map(|item| {
            let override_pricing = pricing_overrides_by_model.get(&item.model_id).cloned();
            let effective_pricing = if let Some(override_item) = override_pricing.as_ref() {
                AdminModelPricingView {
                    input_price_microcredits: Some(override_item.input_price_microcredits),
                    cached_input_price_microcredits: Some(
                        override_item.cached_input_price_microcredits,
                    ),
                    output_price_microcredits: Some(override_item.output_price_microcredits),
                    source: "manual_override".to_string(),
                }
            } else {
                AdminModelPricingView {
                    input_price_microcredits: item.input_price_microcredits,
                    cached_input_price_microcredits: item.cached_input_price_microcredits,
                    output_price_microcredits: item.output_price_microcredits,
                    source: "official_sync".to_string(),
                }
            };
            let probe = cache_entries.get(&item.model_id);
            AdminModelItem {
                id: item.model_id.clone(),
                owned_by: probe
                    .and_then(|entry| entry.owned_by.clone())
                    .unwrap_or_else(|| item.owned_by.clone()),
                availability_status: probe
                    .map(|entry| entry.status.clone())
                    .unwrap_or(AdminModelAvailabilityStatus::Unknown),
                availability_checked_at: probe.map(|entry| entry.checked_at),
                availability_http_status: probe.and_then(|entry| entry.http_status),
                availability_error: probe.and_then(|entry| entry.error.clone()),
                official: AdminModelOfficialInfo {
                    title: item.title,
                    description: item.description,
                    context_window_tokens: item.context_window_tokens,
                    max_output_tokens: item.max_output_tokens,
                    knowledge_cutoff: item.knowledge_cutoff,
                    reasoning_token_support: item.reasoning_token_support,
                    pricing_notes: item.pricing_notes,
                    input_modalities: item.input_modalities,
                    output_modalities: item.output_modalities,
                    endpoints: item.endpoints,
                    source_url: item.source_url,
                    synced_at: item.synced_at,
                    raw_text: item.raw_text,
                },
                override_pricing,
                effective_pricing,
            }
        })
        .collect::<Vec<_>>();

    for (model_id, entry) in cache_entries {
        if official_model_ids.contains(model_id)
            || entry.status != AdminModelAvailabilityStatus::Available
        {
            continue;
        }

        let override_pricing = pricing_overrides_by_model.get(model_id).cloned();
        let effective_pricing = if let Some(override_item) = override_pricing.as_ref() {
            AdminModelPricingView {
                input_price_microcredits: Some(override_item.input_price_microcredits),
                cached_input_price_microcredits: Some(
                    override_item.cached_input_price_microcredits,
                ),
                output_price_microcredits: Some(override_item.output_price_microcredits),
                source: "manual_override".to_string(),
            }
        } else {
            AdminModelPricingView {
                input_price_microcredits: None,
                cached_input_price_microcredits: None,
                output_price_microcredits: None,
                source: "probe_only".to_string(),
            }
        };

        data.push(AdminModelItem {
            id: model_id.clone(),
            owned_by: entry
                .owned_by
                .clone()
                .unwrap_or_else(|| "upstream".to_string()),
            availability_status: entry.status.clone(),
            availability_checked_at: Some(entry.checked_at),
            availability_http_status: entry.http_status,
            availability_error: entry.error.clone(),
            official: AdminModelOfficialInfo {
                title: model_id.clone(),
                description: None,
                context_window_tokens: None,
                max_output_tokens: None,
                knowledge_cutoff: None,
                reasoning_token_support: None,
                pricing_notes: None,
                input_modalities: Vec::new(),
                output_modalities: Vec::new(),
                endpoints: vec!["v1/models".to_string()],
                source_url: String::new(),
                synced_at: entry.checked_at,
                raw_text: None,
            },
            override_pricing,
            effective_pricing,
        });
    }

    data.sort_by(|left, right| left.id.cmp(&right.id));
    data
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct UpstreamListedModel {
    id: String,
    owned_by: Option<String>,
}

#[derive(Debug, Clone)]
struct UpstreamModelsFetchResult {
    checked_at: DateTime<Utc>,
    http_status: Option<u16>,
    error: Option<String>,
    models: Vec<UpstreamListedModel>,
}

fn is_successful_models_fetch(result: &UpstreamModelsFetchResult) -> bool {
    result
        .http_status
        .map(|status| (200..300).contains(&status))
        .unwrap_or(false)
        && result.error.is_none()
}

fn missing_upstream_model_error(model_id: &str) -> String {
    format!(
        "Model '{model_id}' is not listed by the upstream /models endpoint for the selected account."
    )
}

fn extract_upstream_listed_models(
    payload: serde_json::Value,
    mode: &codex_pool_core::model::UpstreamMode,
) -> anyhow::Result<Vec<UpstreamListedModel>> {
    let payload = crate::upstream_api::normalise_models_payload(payload, mode);
    let data = payload
        .get("data")
        .and_then(|value| value.as_array())
        .ok_or_else(|| anyhow::anyhow!("normalized /models payload is missing data array"))?;

    Ok(data
        .iter()
        .filter_map(|item| {
            let id = item.get("id").and_then(|value| value.as_str())?.trim();
            if id.is_empty() {
                return None;
            }
            Some(UpstreamListedModel {
                id: id.to_string(),
                owned_by: item
                    .get("owned_by")
                    .and_then(|value| value.as_str())
                    .map(ToString::to_string),
            })
        })
        .collect())
}

#[cfg_attr(not(test), allow(dead_code))]
fn build_probe_entries_from_upstream_models(
    official_catalog: &[crate::tenant::OpenAiModelCatalogItem],
    requested_models: &[String],
    fetch_result: &UpstreamModelsFetchResult,
) -> std::collections::HashMap<String, ModelProbeCacheEntry> {
    let mut target_ids = official_catalog
        .iter()
        .map(|item| item.model_id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    for model_id in requested_models {
        target_ids.insert(model_id.clone());
    }

    let mut upstream_models_by_id = std::collections::BTreeMap::new();
    for model in &fetch_result.models {
        target_ids.insert(model.id.clone());
        upstream_models_by_id
            .entry(model.id.clone())
            .or_insert_with(|| model.clone());
    }

    let successful_fetch = is_successful_models_fetch(fetch_result);
    let fallback_error = fetch_result
        .error
        .clone()
        .or_else(|| Some("failed to fetch upstream /models list".to_string()));

    target_ids
        .into_iter()
        .map(|model_id| {
            let entry = if let Some(listed_model) = upstream_models_by_id.get(&model_id) {
                ModelProbeCacheEntry {
                    status: AdminModelAvailabilityStatus::Available,
                    checked_at: fetch_result.checked_at,
                    http_status: fetch_result.http_status,
                    error: None,
                    owned_by: listed_model.owned_by.clone(),
                }
            } else if successful_fetch {
                ModelProbeCacheEntry {
                    status: AdminModelAvailabilityStatus::Unavailable,
                    checked_at: fetch_result.checked_at,
                    http_status: fetch_result.http_status,
                    error: Some(missing_upstream_model_error(&model_id)),
                    owned_by: None,
                }
            } else {
                ModelProbeCacheEntry {
                    status: AdminModelAvailabilityStatus::Unavailable,
                    checked_at: fetch_result.checked_at,
                    http_status: fetch_result.http_status,
                    error: fallback_error.clone(),
                    owned_by: None,
                }
            };
            (model_id, entry)
        })
        .collect()
}

fn build_probe_entries_from_account_results(
    official_catalog: &[crate::tenant::OpenAiModelCatalogItem],
    requested_models: &[String],
    account_results: &[(UpstreamAccount, UpstreamModelsFetchResult)],
) -> std::collections::HashMap<String, ModelProbeCacheEntry> {
    let mut target_ids = official_catalog
        .iter()
        .map(|item| item.model_id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    for model_id in requested_models {
        target_ids.insert(model_id.clone());
    }

    let mut available_entries = std::collections::BTreeMap::<String, ModelProbeCacheEntry>::new();
    let mut first_fallback_result: Option<&UpstreamModelsFetchResult> = None;
    let any_success_with_models = account_results
        .iter()
        .any(|(_, result)| is_successful_models_fetch(result) && !result.models.is_empty());

    for (_, result) in account_results {
        if first_fallback_result.is_none()
            || (first_fallback_result.is_some_and(|current| current.http_status.is_none())
                && result.http_status.is_some())
        {
            first_fallback_result = Some(result);
        }

        if !is_successful_models_fetch(result) {
            continue;
        }

        for model in &result.models {
            target_ids.insert(model.id.clone());
            available_entries
                .entry(model.id.clone())
                .and_modify(|entry| {
                    if result.checked_at > entry.checked_at {
                        entry.checked_at = result.checked_at;
                        entry.http_status = result.http_status;
                    }
                    if entry.owned_by.is_none() {
                        entry.owned_by = model.owned_by.clone();
                    }
                })
                .or_insert_with(|| ModelProbeCacheEntry {
                    status: AdminModelAvailabilityStatus::Available,
                    checked_at: result.checked_at,
                    http_status: result.http_status,
                    error: None,
                    owned_by: model.owned_by.clone(),
                });
        }
    }

    let fallback_http_status = first_fallback_result.and_then(|result| result.http_status);
    let fallback_error = first_fallback_result
        .and_then(|result| result.error.clone())
        .or_else(|| Some("failed to fetch upstream /models list from account pool".to_string()));
    let fallback_checked_at = first_fallback_result
        .map(|result| result.checked_at)
        .unwrap_or_else(Utc::now);

    for model_id in target_ids {
        available_entries
            .entry(model_id.clone())
            .or_insert_with(|| {
                if any_success_with_models {
                    ModelProbeCacheEntry {
                        status: AdminModelAvailabilityStatus::Unavailable,
                        checked_at: fallback_checked_at,
                        http_status: Some(200),
                        error: Some(missing_upstream_model_error(&model_id)),
                        owned_by: None,
                    }
                } else {
                    ModelProbeCacheEntry {
                        status: AdminModelAvailabilityStatus::Unavailable,
                        checked_at: fallback_checked_at,
                        http_status: fallback_http_status,
                        error: fallback_error.clone(),
                        owned_by: None,
                    }
                }
            });
    }

    available_entries.into_iter().collect()
}

fn summarize_probe_source_label(
    account_results: &[(UpstreamAccount, UpstreamModelsFetchResult)],
) -> String {
    let total = account_results.len();
    let listed = account_results
        .iter()
        .filter(|(_, result)| is_successful_models_fetch(result) && !result.models.is_empty())
        .count();
    let auth_invalid = account_results
        .iter()
        .filter(|(_, result)| is_auth_invalid_probe_result(result))
        .count();
    let first_label = account_results
        .iter()
        .find(|(_, result)| is_successful_models_fetch(result) && !result.models.is_empty())
        .map(|(account, _)| account.label.as_str())
        .or_else(|| {
            account_results
                .first()
                .map(|(account, _)| account.label.as_str())
        })
        .unwrap_or("none");
    format!(
        "pool-union ({listed}/{total} listed; auth_invalid={auth_invalid}; first={first_label})"
    )
}

fn mark_model_available_in_probe_cache(
    cache: &mut ModelProbeCache,
    model_id: &str,
    checked_at: DateTime<Utc>,
    http_status: u16,
) {
    let model_id = model_id.trim();
    if model_id.is_empty() {
        return;
    }

    cache.updated_at = Some(
        cache
            .updated_at
            .map(|previous| previous.max(checked_at))
            .unwrap_or(checked_at),
    );

    cache
        .entries
        .entry(model_id.to_string())
        .and_modify(|entry| {
            if entry.status != AdminModelAvailabilityStatus::Available
                || checked_at >= entry.checked_at
            {
                entry.status = AdminModelAvailabilityStatus::Available;
                entry.checked_at = checked_at;
                entry.http_status = Some(http_status);
                entry.error = None;
            }
        })
        .or_insert_with(|| ModelProbeCacheEntry {
            status: AdminModelAvailabilityStatus::Available,
            checked_at,
            http_status: Some(http_status),
            error: None,
            owned_by: None,
        });
}

fn apply_recent_successful_request_models(
    entries: &mut std::collections::HashMap<String, ModelProbeCacheEntry>,
    rows: &[crate::usage::RequestLogRow],
) {
    for row in rows {
        let Some(model_id) = row.model.as_ref() else {
            continue;
        };
        if row.status_code < 200 || row.status_code >= 300 {
            continue;
        }
        if !matches!(
            row.path.as_str(),
            "/v1/responses"
                | "/backend-api/codex/responses"
                | "/v1/chat/completions"
                | "/backend-api/codex/responses/compact"
                | "/v1/responses/compact"
        ) {
            continue;
        }

        let checked_at = row.created_at;
        entries
            .entry(model_id.clone())
            .and_modify(|entry| {
                if entry.status != AdminModelAvailabilityStatus::Available
                    || checked_at > entry.checked_at
                {
                    entry.status = AdminModelAvailabilityStatus::Available;
                    entry.checked_at = checked_at;
                    entry.http_status = Some(row.status_code);
                    entry.error = None;
                }
            })
            .or_insert_with(|| ModelProbeCacheEntry {
                status: AdminModelAvailabilityStatus::Available,
                checked_at,
                http_status: Some(row.status_code),
                error: None,
                owned_by: None,
            });
    }
}

async fn overlay_recent_successful_request_models(
    state: &AppState,
    entries: &mut std::collections::HashMap<String, ModelProbeCacheEntry>,
) {
    let Some(usage_repo) = state.usage_repo.as_ref() else {
        return;
    };

    let now_ts = Utc::now().timestamp();
    let query = crate::usage::RequestLogQuery {
        start_ts: now_ts.saturating_sub(MODEL_PROBE_CACHE_TTL_SEC.saturating_mul(2)),
        end_ts: now_ts,
        limit: MAX_USAGE_QUERY_LIMIT,
        tenant_id: None,
        api_key_id: None,
        status_code: Some(200),
        request_id: None,
        keyword: None,
    };

    match usage_repo.query_request_logs(query).await {
        Ok(rows) => apply_recent_successful_request_models(entries, &rows),
        Err(err) => {
            tracing::warn!(error = %err, "failed to overlay recent successful request models onto probe cache");
        }
    }
}

async fn fetch_upstream_models_for_accounts(
    client: &reqwest::Client,
    accounts: Vec<UpstreamAccount>,
) -> anyhow::Result<Vec<(UpstreamAccount, UpstreamModelsFetchResult)>> {
    let mut results =
        futures_util::stream::iter(accounts.into_iter().enumerate().map(|(index, account)| {
            let client = client.clone();
            async move {
                let result = fetch_upstream_models_for_account(&client, &account).await?;
                Ok::<_, anyhow::Error>((index, account, result))
            }
        }))
        .buffer_unordered(MODEL_PROBE_ACCOUNT_FETCH_CONCURRENCY)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<anyhow::Result<Vec<_>>>()?;

    results.sort_by_key(|(index, _, _)| *index);
    Ok(results
        .into_iter()
        .map(|(_, account, result)| (account, result))
        .collect())
}

fn ordered_probe_accounts(
    mut accounts: Vec<UpstreamAccount>,
    preferred_label: Option<&str>,
) -> Vec<UpstreamAccount> {
    if let Some(preferred_label) = preferred_label {
        if let Some(index) = accounts
            .iter()
            .position(|account| account.label == preferred_label)
        {
            accounts.rotate_left(index);
        }
    }
    accounts
}

fn dedupe_probe_accounts(accounts: Vec<UpstreamAccount>) -> Vec<UpstreamAccount> {
    let mut seen = std::collections::HashSet::new();
    let mut deduped = Vec::new();

    for account in accounts {
        let dedupe_key = account
            .chatgpt_account_id
            .as_ref()
            .map(|chatgpt_account_id| {
                format!(
                    "{:?}:{}:{}",
                    account.mode, account.base_url, chatgpt_account_id
                )
            });
        if let Some(key) = dedupe_key {
            if !seen.insert(key) {
                continue;
            }
        }
        deduped.push(account);
    }

    deduped
}

fn is_auth_invalid_probe_result(entry: &UpstreamModelsFetchResult) -> bool {
    entry.http_status == Some(401)
}

async fn fetch_upstream_models_for_account(
    client: &reqwest::Client,
    account: &UpstreamAccount,
) -> anyhow::Result<UpstreamModelsFetchResult> {
    let models_url =
        crate::upstream_api::build_upstream_models_url(&account.base_url, &account.mode)?;
    let mut request = client
        .get(&models_url)
        .header("authorization", format!("Bearer {}", account.bearer_token))
        .header("accept", "application/json");
    if let Some(account_id) = account.chatgpt_account_id.as_deref() {
        request = request.header("chatgpt-account-id", account_id);
    }

    let checked_at = Utc::now();
    let result = match request.send().await {
        Ok(response) if response.status().is_success() => {
            let status = response.status().as_u16();
            match response.json::<serde_json::Value>().await {
                Ok(payload) => match extract_upstream_listed_models(payload, &account.mode) {
                    Ok(models) => UpstreamModelsFetchResult {
                        checked_at,
                        http_status: Some(status),
                        error: None,
                        models,
                    },
                    Err(err) => UpstreamModelsFetchResult {
                        checked_at,
                        http_status: Some(status),
                        error: Some(err.to_string()),
                        models: Vec::new(),
                    },
                },
                Err(err) => UpstreamModelsFetchResult {
                    checked_at,
                    http_status: Some(status),
                    error: Some(err.to_string()),
                    models: Vec::new(),
                },
            }
        }
        Ok(response) => {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            UpstreamModelsFetchResult {
                checked_at,
                http_status: Some(status.as_u16()),
                error: extract_probe_error_message(&body),
                models: Vec::new(),
            }
        }
        Err(err) => UpstreamModelsFetchResult {
            checked_at,
            http_status: None,
            error: Some(err.to_string()),
            models: Vec::new(),
        },
    };

    Ok(result)
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
                Utc::now().signed_duration_since(updated_at).num_seconds()
                    < MODEL_PROBE_CACHE_TTL_SEC
            })
            .unwrap_or(false)
    };
    if !force && requested_models.is_empty() && cache_is_fresh {
        return Ok(());
    }

    let tenant_auth = state
        .tenant_auth_service
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("tenant auth service is not available"))?;
    let official_catalog = tenant_auth
        .admin_list_openai_model_catalog()
        .await
        .context("failed to load official models catalog for probing")?;
    if official_catalog.is_empty() {
        return Err(anyhow::anyhow!(
            "official models catalog is empty; sync OpenAI catalog first"
        ));
    }

    let snapshot = state
        .store
        .snapshot()
        .await
        .context("failed to load upstream account snapshot")?;
    let accounts = snapshot
        .accounts
        .into_iter()
        .filter(|account| account.enabled)
        .collect::<Vec<_>>();
    let preferred_label = state
        .model_probe_cache
        .read()
        .expect("model_probe_cache lock poisoned")
        .source_account_label
        .clone();
    let accounts =
        dedupe_probe_accounts(ordered_probe_accounts(accounts, preferred_label.as_deref()));
    if accounts.is_empty() {
        anyhow::bail!("no enabled upstream account is available for probe");
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(MODEL_PROBE_REQUEST_TIMEOUT_SEC))
        .build()?;
    let account_results = fetch_upstream_models_for_accounts(&client, accounts).await?;
    if account_results.is_empty() {
        anyhow::bail!("no enabled upstream account is available for probe");
    }

    let source_label = summarize_probe_source_label(&account_results);
    let mut entries = build_probe_entries_from_account_results(
        &official_catalog,
        &requested_models,
        &account_results,
    );
    overlay_recent_successful_request_models(state, &mut entries).await;
    let tested = entries.len();
    let available = entries
        .values()
        .filter(|entry| entry.status == AdminModelAvailabilityStatus::Available)
        .count();

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
                source_account_label = %source_label,
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
        cache.source_account_label = Some(source_label.clone());
        cache.entries = entries;
    }

    push_admin_log(
        state,
        "info",
        "admin.models.probe",
        format!(
            "model probe ({trigger}) tested {tested} models via {} (available={available}, unavailable={})",
            source_label,
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
mod models_probe_tests {
    use super::*;

    fn test_account(label: &str) -> UpstreamAccount {
        UpstreamAccount {
            id: Uuid::new_v4(),
            label: label.to_string(),
            mode: codex_pool_core::model::UpstreamMode::CodexOauth,
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            bearer_token: format!("token-{label}"),
            chatgpt_account_id: Some(format!("acct-{label}")),
            enabled: true,
            priority: 100,
            created_at: Utc::now(),
        }
    }

    fn test_probe_entry(
        status: AdminModelAvailabilityStatus,
        http_status: Option<u16>,
        error: Option<&str>,
        owned_by: Option<&str>,
    ) -> ModelProbeCacheEntry {
        ModelProbeCacheEntry {
            status,
            checked_at: Utc::now(),
            http_status,
            error: error.map(ToString::to_string),
            owned_by: owned_by.map(ToString::to_string),
        }
    }

    fn test_models_fetch_result(
        http_status: Option<u16>,
        error: Option<&str>,
        models: &[(&str, Option<&str>)],
    ) -> UpstreamModelsFetchResult {
        UpstreamModelsFetchResult {
            checked_at: Utc::now(),
            http_status,
            error: error.map(ToString::to_string),
            models: models
                .iter()
                .map(|(id, owned_by)| UpstreamListedModel {
                    id: (*id).to_string(),
                    owned_by: owned_by.map(|value| value.to_string()),
                })
                .collect(),
        }
    }

    fn test_catalog_item(model_id: &str) -> crate::tenant::OpenAiModelCatalogItem {
        crate::tenant::OpenAiModelCatalogItem {
            model_id: model_id.to_string(),
            owned_by: "openai".to_string(),
            title: model_id.to_string(),
            description: None,
            context_window_tokens: None,
            max_output_tokens: None,
            knowledge_cutoff: None,
            reasoning_token_support: None,
            input_price_microcredits: None,
            cached_input_price_microcredits: None,
            output_price_microcredits: None,
            pricing_notes: None,
            input_modalities: Vec::new(),
            output_modalities: Vec::new(),
            endpoints: vec!["responses".to_string()],
            source_url: format!("https://example.com/{model_id}"),
            raw_text: None,
            synced_at: Utc::now(),
        }
    }

    #[test]
    fn dedupe_probe_accounts_collapses_same_chatgpt_account() {
        let mut first = test_account("first");
        first.chatgpt_account_id = Some("acct-shared".to_string());
        let mut second = test_account("second");
        second.chatgpt_account_id = Some("acct-shared".to_string());
        let third = test_account("third");

        let labels = dedupe_probe_accounts(vec![first, second, third])
            .into_iter()
            .map(|account| account.label)
            .collect::<Vec<_>>();

        assert_eq!(labels, vec!["first", "third"]);
    }

    #[test]
    fn ordered_probe_accounts_rotates_preferred_without_resorting() {
        let labels = ordered_probe_accounts(
            vec![
                test_account("bravo"),
                test_account("alpha"),
                test_account("charlie"),
            ],
            Some("charlie"),
        )
        .into_iter()
        .map(|account| account.label)
        .collect::<Vec<_>>();

        assert_eq!(labels, vec!["charlie", "bravo", "alpha"]);
    }

    #[test]
    fn build_probe_entries_from_account_results_uses_pool_union_for_availability() {
        let official_catalog = vec![test_catalog_item("gpt-5.4"), test_catalog_item("o3")];
        let requested_models = vec!["codex-mini-latest".to_string()];
        let account_results = vec![
            (
                test_account("probe-source"),
                test_models_fetch_result(Some(200), None, &[("o3", Some("codex-oauth"))]),
            ),
            (
                test_account("actual-success"),
                test_models_fetch_result(
                    Some(200),
                    None,
                    &[
                        ("gpt-5.4", Some("codex-oauth")),
                        ("codex-mini-latest", Some("codex-oauth")),
                    ],
                ),
            ),
        ];

        let entries = build_probe_entries_from_account_results(
            &official_catalog,
            &requested_models,
            &account_results,
        );

        assert_eq!(
            entries.get("gpt-5.4").map(|entry| entry.status.clone()),
            Some(AdminModelAvailabilityStatus::Available)
        );
        assert_eq!(
            entries
                .get("codex-mini-latest")
                .map(|entry| entry.status.clone()),
            Some(AdminModelAvailabilityStatus::Available)
        );
        assert_eq!(
            entries.get("o3").map(|entry| entry.status.clone()),
            Some(AdminModelAvailabilityStatus::Available)
        );
    }

    #[test]
    fn apply_recent_successful_request_models_promotes_model_to_available() {
        let mut entries = std::collections::HashMap::from([(
            "gpt-5.4".to_string(),
            test_probe_entry(
                AdminModelAvailabilityStatus::Unavailable,
                Some(200),
                Some("not listed by /models"),
                None,
            ),
        )]);
        let rows = vec![crate::usage::RequestLogRow {
            id: Uuid::new_v4(),
            account_id: Uuid::new_v4(),
            tenant_id: None,
            api_key_id: None,
            request_id: Some("req-1".to_string()),
            path: "/v1/responses".to_string(),
            method: "POST".to_string(),
            model: Some("gpt-5.4".to_string()),
            input_tokens: None,
            cached_input_tokens: None,
            output_tokens: None,
            reasoning_tokens: None,
            first_token_latency_ms: None,
            status_code: 200,
            latency_ms: 1234,
            is_stream: true,
            error_code: None,
            billing_phase: Some("released".to_string()),
            authorization_id: None,
            capture_status: Some("captured".to_string()),
            created_at: Utc::now(),
            event_version: 2,
        }];

        apply_recent_successful_request_models(&mut entries, &rows);
        let entry = entries.get("gpt-5.4").expect("entry must exist");

        assert_eq!(entry.status, AdminModelAvailabilityStatus::Available);
        assert_eq!(entry.http_status, Some(200));
        assert_eq!(entry.error, None);
    }

    #[test]
    fn mark_model_available_in_probe_cache_promotes_entry_and_refreshes_timestamp() {
        let checked_at = Utc::now();
        let mut cache = ModelProbeCache {
            updated_at: Some(checked_at - chrono::Duration::minutes(5)),
            source_account_label: Some("pool-union".to_string()),
            entries: std::collections::HashMap::from([(
                "gpt-5.3-codex".to_string(),
                test_probe_entry(
                    AdminModelAvailabilityStatus::Unavailable,
                    Some(200),
                    Some("not listed by /models"),
                    None,
                ),
            )]),
        };

        mark_model_available_in_probe_cache(&mut cache, "gpt-5.3-codex", checked_at, 200);

        let entry = cache
            .entries
            .get("gpt-5.3-codex")
            .expect("entry must exist");
        assert_eq!(entry.status, AdminModelAvailabilityStatus::Available);
        assert_eq!(entry.http_status, Some(200));
        assert_eq!(entry.error, None);
        assert_eq!(cache.updated_at, Some(checked_at));
    }

    #[test]
    fn build_probe_entries_from_account_results_falls_back_when_all_accounts_fail() {
        let official_catalog = vec![test_catalog_item("gpt-5.4")];
        let account_results = vec![
            (
                test_account("broken-a"),
                test_models_fetch_result(Some(401), Some("invalidated-a"), &[]),
            ),
            (
                test_account("broken-b"),
                test_models_fetch_result(Some(401), Some("invalidated-b"), &[]),
            ),
        ];

        let entries = build_probe_entries_from_account_results(
            &official_catalog,
            &Vec::new(),
            &account_results,
        );
        let entry = entries.get("gpt-5.4").expect("entry must exist");

        assert_eq!(entry.status, AdminModelAvailabilityStatus::Unavailable);
        assert_eq!(entry.http_status, Some(401));
        assert!(entry
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("invalidated"));
    }

    #[test]
    fn build_probe_entries_marks_supported_and_unsupported_models_from_upstream_list() {
        let official_catalog = vec![test_catalog_item("gpt-4"), test_catalog_item("o3")];
        let requested_models = vec!["codex-mini-latest".to_string()];
        let fetch_result = test_models_fetch_result(
            Some(200),
            None,
            &[
                ("o3", Some("codex-oauth")),
                ("codex-mini-latest", Some("codex-oauth")),
            ],
        );

        let entries = build_probe_entries_from_upstream_models(
            &official_catalog,
            &requested_models,
            &fetch_result,
        );

        assert_eq!(
            entries.get("o3").map(|entry| entry.status.clone()),
            Some(AdminModelAvailabilityStatus::Available)
        );
        assert_eq!(
            entries
                .get("o3")
                .and_then(|entry| entry.owned_by.as_deref()),
            Some("codex-oauth")
        );
        assert_eq!(
            entries
                .get("codex-mini-latest")
                .map(|entry| entry.status.clone()),
            Some(AdminModelAvailabilityStatus::Available)
        );
        let gpt4 = entries.get("gpt-4").expect("gpt-4 entry");
        assert_eq!(gpt4.status, AdminModelAvailabilityStatus::Unavailable);
        assert_eq!(gpt4.http_status, Some(200));
        assert!(gpt4
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("upstream /models endpoint"));
    }

    trait ResponseMaybeFuture {
        type Future: std::future::Future<Output = AdminModelsResponse>;

        fn into_response_future(self) -> Self::Future;
    }

    impl ResponseMaybeFuture for AdminModelsResponse {
        type Future = std::future::Ready<AdminModelsResponse>;

        fn into_response_future(self) -> Self::Future {
            std::future::ready(self)
        }
    }

    impl<Fut> ResponseMaybeFuture for Fut
    where
        Fut: std::future::Future<Output = AdminModelsResponse>,
    {
        type Future = Fut;

        fn into_response_future(self) -> Self::Future {
            self
        }
    }

    #[derive(Clone)]
    struct TestUsageRepo {
        rows: Vec<crate::usage::RequestLogRow>,
    }

    #[async_trait::async_trait]
    impl crate::usage::clickhouse_repo::UsageQueryRepository for TestUsageRepo {
        async fn query_hourly_accounts(
            &self,
            _start_ts: i64,
            _end_ts: i64,
            _limit: u32,
            _account_id: Option<Uuid>,
        ) -> anyhow::Result<Vec<codex_pool_core::api::HourlyAccountUsagePoint>> {
            Ok(Vec::new())
        }

        async fn query_hourly_tenant_api_keys(
            &self,
            _start_ts: i64,
            _end_ts: i64,
            _limit: u32,
            _tenant_id: Option<Uuid>,
            _api_key_id: Option<Uuid>,
        ) -> anyhow::Result<Vec<codex_pool_core::api::HourlyTenantApiKeyUsagePoint>> {
            Ok(Vec::new())
        }

        async fn query_hourly_account_totals(
            &self,
            _start_ts: i64,
            _end_ts: i64,
            _limit: u32,
            _account_id: Option<Uuid>,
        ) -> anyhow::Result<Vec<codex_pool_core::api::HourlyUsageTotalPoint>> {
            Ok(Vec::new())
        }

        async fn query_hourly_tenant_api_key_totals(
            &self,
            _start_ts: i64,
            _end_ts: i64,
            _limit: u32,
            _tenant_id: Option<Uuid>,
            _api_key_id: Option<Uuid>,
        ) -> anyhow::Result<Vec<codex_pool_core::api::HourlyUsageTotalPoint>> {
            Ok(Vec::new())
        }

        async fn query_hourly_tenant_totals(
            &self,
            _start_ts: i64,
            _end_ts: i64,
            _limit: u32,
            _tenant_id: Option<Uuid>,
            _api_key_id: Option<Uuid>,
        ) -> anyhow::Result<Vec<codex_pool_core::api::HourlyTenantUsageTotalPoint>> {
            Ok(Vec::new())
        }

        async fn query_summary(
            &self,
            start_ts: i64,
            end_ts: i64,
            _tenant_id: Option<Uuid>,
            _account_id: Option<Uuid>,
            _api_key_id: Option<Uuid>,
        ) -> anyhow::Result<codex_pool_core::api::UsageSummaryQueryResponse> {
            Ok(codex_pool_core::api::UsageSummaryQueryResponse {
                start_ts,
                end_ts,
                account_total_requests: 0,
                tenant_api_key_total_requests: 0,
                unique_account_count: 0,
                unique_tenant_api_key_count: 0,
                dashboard_metrics: None,
            })
        }

        async fn query_tenant_leaderboard(
            &self,
            _start_ts: i64,
            _end_ts: i64,
            _limit: u32,
            _tenant_id: Option<Uuid>,
        ) -> anyhow::Result<Vec<codex_pool_core::api::TenantUsageLeaderboardItem>> {
            Ok(Vec::new())
        }

        async fn query_account_leaderboard(
            &self,
            _start_ts: i64,
            _end_ts: i64,
            _limit: u32,
            _account_id: Option<Uuid>,
        ) -> anyhow::Result<Vec<codex_pool_core::api::AccountUsageLeaderboardItem>> {
            Ok(Vec::new())
        }

        async fn query_api_key_leaderboard(
            &self,
            _start_ts: i64,
            _end_ts: i64,
            _limit: u32,
            _tenant_id: Option<Uuid>,
            _api_key_id: Option<Uuid>,
        ) -> anyhow::Result<Vec<codex_pool_core::api::ApiKeyUsageLeaderboardItem>> {
            Ok(Vec::new())
        }

        async fn query_request_logs(
            &self,
            _query: crate::usage::RequestLogQuery,
        ) -> anyhow::Result<Vec<crate::usage::RequestLogRow>> {
            Ok(self.rows.clone())
        }
    }

    fn ensure_test_admin_env() {
        std::env::set_var("ADMIN_USERNAME", "admin");
        std::env::set_var("ADMIN_PASSWORD", "admin123456");
        std::env::set_var("ADMIN_JWT_SECRET", "control-plane-test-jwt-secret");
    }

    fn test_app_state_with_usage_rows(rows: Vec<crate::usage::RequestLogRow>) -> AppState {
        ensure_test_admin_env();
        let store: std::sync::Arc<dyn crate::store::ControlPlaneStore> =
            std::sync::Arc::new(crate::store::InMemoryStore::default());

        AppState {
            store: store.clone(),
            usage_repo: Some(std::sync::Arc::new(TestUsageRepo { rows })),
            tenant_auth_service: None,
            auth_validate_cache_ttl_sec: DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC,
            admin_auth: crate::admin_auth::AdminAuthService::from_env()
                .expect("admin auth env must be set"),
            internal_auth_token: std::sync::Arc::<str>::from("test-internal-auth-token"),
            import_job_manager: crate::import_jobs::OAuthImportJobManager::new(
                store,
                std::sync::Arc::new(crate::import_jobs::InMemoryOAuthImportJobStore::default()),
                1,
                1,
            ),
            started_at: Utc::now(),
            runtime_config: std::sync::Arc::new(std::sync::RwLock::new(
                build_runtime_config_from_env(DEFAULT_AUTH_VALIDATE_CACHE_TTL_SEC),
            )),
            admin_logs: std::sync::Arc::new(std::sync::RwLock::new(
                std::collections::VecDeque::new(),
            )),
            admin_proxies: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
            model_catalog_last_error: std::sync::Arc::new(std::sync::RwLock::new(None)),
            model_probe_cache: std::sync::Arc::new(std::sync::RwLock::new(
                ModelProbeCache::default(),
            )),
            oauth_login_sessions: std::sync::Arc::new(std::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
            oauth_probe_sessions: std::sync::Arc::new(std::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
            codex_oauth_callback_listen_mode: CodexOAuthCallbackListenMode::Off,
            codex_oauth_callback_listen_addr: None,
            codex_oauth_callback_listener: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
            model_probe_interval_sec: MODEL_PROBE_DEFAULT_INTERVAL_SEC,
        }
    }

    #[tokio::test]
    async fn build_admin_models_response_overlays_recent_successful_request_models() {
        let state = test_app_state_with_usage_rows(vec![crate::usage::RequestLogRow {
            id: Uuid::new_v4(),
            account_id: Uuid::new_v4(),
            tenant_id: None,
            api_key_id: None,
            request_id: Some("req-recent-success".to_string()),
            path: "/v1/responses".to_string(),
            method: "POST".to_string(),
            model: Some("gpt-5.4".to_string()),
            input_tokens: None,
            cached_input_tokens: None,
            output_tokens: None,
            reasoning_tokens: None,
            first_token_latency_ms: None,
            status_code: 200,
            latency_ms: 120,
            is_stream: true,
            error_code: None,
            billing_phase: Some("released".to_string()),
            authorization_id: None,
            capture_status: Some("captured".to_string()),
            created_at: Utc::now(),
            event_version: 2,
        }]);

        let response = build_admin_models_response(&state, Vec::new(), Vec::new())
            .into_response_future()
            .await;
        let row = response
            .data
            .iter()
            .find(|item| item.id == "gpt-5.4")
            .expect("recently successful model must be present in admin model list");

        assert_eq!(
            row.availability_status,
            AdminModelAvailabilityStatus::Available
        );
        assert_eq!(row.availability_http_status, Some(200));
        assert_eq!(row.availability_error, None);
    }

    #[test]
    fn build_admin_model_items_appends_probe_only_available_models() {
        let official_catalog = vec![test_catalog_item("gpt-4")];
        let pricing_overrides = std::collections::HashMap::new();
        let cache_entries = std::collections::HashMap::from([
            (
                "gpt-4".to_string(),
                test_probe_entry(
                    AdminModelAvailabilityStatus::Available,
                    Some(200),
                    None,
                    Some("openai"),
                ),
            ),
            (
                "codex-mini-latest".to_string(),
                test_probe_entry(
                    AdminModelAvailabilityStatus::Available,
                    Some(200),
                    None,
                    Some("codex-oauth"),
                ),
            ),
        ]);

        let items = build_admin_model_items(official_catalog, &pricing_overrides, &cache_entries);
        let probe_only = items
            .iter()
            .find(|item| item.id == "codex-mini-latest")
            .expect("probe-only item should be appended");

        assert_eq!(items.len(), 2);
        assert_eq!(probe_only.owned_by, "codex-oauth");
        assert_eq!(probe_only.official.title, "codex-mini-latest");
        assert_eq!(probe_only.official.endpoints, vec!["v1/models".to_string()]);
        assert!(probe_only.official.source_url.is_empty());
        assert_eq!(
            probe_only.availability_status,
            AdminModelAvailabilityStatus::Available
        );
    }
}
