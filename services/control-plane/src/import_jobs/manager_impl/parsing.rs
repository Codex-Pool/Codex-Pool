fn parse_file_records(
    file: &ImportUploadFile,
    options: &CreateOAuthImportJobOptions,
) -> Result<Vec<PersistedImportItem>> {
    let file_name = if file.file_name.trim().is_empty() {
        "uploaded.json".to_string()
    } else {
        file.file_name.clone()
    };
    let lower_name = file_name.to_ascii_lowercase();

    if lower_name.ends_with(".jsonl") {
        parse_jsonl_records(&file_name, &file.content, options)
    } else if lower_name.ends_with(".json") {
        parse_json_records(&file_name, &file.content, options)
    } else {
        Err(anyhow!(
            "unsupported file extension for {} (only .json/.jsonl)",
            file_name
        ))
    }
}

fn parse_jsonl_records(
    file_name: &str,
    content: &[u8],
    options: &CreateOAuthImportJobOptions,
) -> Result<Vec<PersistedImportItem>> {
    let raw = std::str::from_utf8(content).context("file is not valid utf-8")?;
    let mut items = Vec::new();

    for (idx, line) in raw.lines().enumerate() {
        let line_no = (idx + 1) as u64;
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(value) => items.push(build_item_state(file_name, line_no, value, options)),
            Err(err) => {
                tracing::debug!(
                    file_name = %file_name,
                    line_no,
                    error = %err,
                    "failed to parse JSONL record"
                );
                items.push(build_failed_line_item(
                    file_name,
                    line_no,
                    "invalid_record",
                    "invalid JSON record".to_string(),
                    Some(Value::String(line.to_string())),
                ));
            }
        }
    }

    Ok(items)
}

fn parse_json_records(
    file_name: &str,
    content: &[u8],
    options: &CreateOAuthImportJobOptions,
) -> Result<Vec<PersistedImportItem>> {
    let value = serde_json::from_slice::<serde_json::Value>(content)
        .context("invalid json file content")?;

    let mut items = Vec::new();
    match value {
        serde_json::Value::Array(entries) => {
            for (idx, entry) in entries.into_iter().enumerate() {
                items.push(build_item_state(
                    file_name,
                    (idx + 1) as u64,
                    entry,
                    options,
                ));
            }
        }
        entry @ serde_json::Value::Object(_) => {
            items.push(build_item_state(file_name, 1, entry, options));
        }
        _ => {
            return Err(anyhow!(
                "json file {} must be either an object or an array of objects",
                file_name
            ));
        }
    }

    Ok(items)
}

fn build_item_state(
    source_file: &str,
    line_no: u64,
    value: serde_json::Value,
    options: &CreateOAuthImportJobOptions,
) -> PersistedImportItem {
    let normalized_raw = normalize_record_aliases(value.clone());
    let parsed = serde_json::from_value::<CredentialRecord>(normalized_raw.clone());

    let mut item = OAuthImportJobItem {
        item_id: 0,
        source_file: source_file.to_string(),
        line_no,
        status: OAuthImportItemStatus::Pending,
        label: String::new(),
        email: None,
        chatgpt_account_id: None,
        account_id: None,
        error_code: None,
        error_message: None,
        admission_status: None,
        admission_source: None,
        admission_reason: None,
        failure_stage: None,
        attempt_count: 0,
        transient_retry_count: 0,
        next_retry_at: None,
        retryable: false,
        terminal_reason: None,
    };

    let request = match parsed {
        Ok(record) => normalize_record(record, options, &mut item),
        Err(err) => {
            tracing::debug!(
                source_file = %source_file,
                line_no,
                error = %err,
                "failed to parse credential record"
            );
            item.status = OAuthImportItemStatus::Failed;
            item.error_code = Some("invalid_record".to_string());
            item.error_message = Some("record payload is invalid".to_string());
            item.admission_status = Some(OAuthVaultRecordStatus::Failed);
            item.admission_reason = Some("invalid_record".to_string());
            None
        }
    };

    let normalized_record = request
        .as_ref()
        .and_then(|payload| serde_json::to_value(payload).ok());

    PersistedImportItem {
        item,
        request,
        raw_record: Some(value),
        normalized_record,
        retry_count: 0,
    }
}

fn normalize_record_aliases(value: Value) -> Value {
    let Value::Object(mut object) = value else {
        return value;
    };

    let get_nested_string = |root: &serde_json::Map<String, Value>, path: &[&str]| -> Option<String> {
        if path.is_empty() {
            return None;
        }
        let mut current = root.get(path[0])?;
        for key in &path[1..] {
            let Value::Object(map) = current else {
                return None;
            };
            current = map.get(*key)?;
        }
        current
            .as_str()
            .map(|raw| raw.trim().to_string())
            .filter(|raw| !raw.is_empty())
    };

    let get_string = |root: &serde_json::Map<String, Value>, keys: &[&str]| -> Option<String> {
        keys.iter().find_map(|key| {
            root.get(*key)
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
        })
    };

    if !object.contains_key("refresh_token") {
        if let Some(value) = get_string(&object, &["refreshToken", "rt", "refreshTokenPlaintext"]) {
            object.insert("refresh_token".to_string(), Value::String(value));
        } else if let Some(value) = get_nested_string(
            &object,
            &["token_info", "refresh_token"],
        )
        .or_else(|| get_nested_string(&object, &["tokens", "refresh_token"]))
        .or_else(|| get_nested_string(&object, &["oauth", "refresh_token"]))
        .or_else(|| get_nested_string(&object, &["auth", "refresh_token"]))
        {
            object.insert("refresh_token".to_string(), Value::String(value));
        }
    }

    if !object.contains_key("access_token") {
        if let Some(value) = get_string(&object, &["accessToken", "token", "bearer_token"]) {
            object.insert("access_token".to_string(), Value::String(value));
        } else if let Some(value) = get_nested_string(&object, &["token_info", "access_token"])
            .or_else(|| get_nested_string(&object, &["tokens", "access_token"]))
            .or_else(|| get_nested_string(&object, &["oauth", "access_token"]))
            .or_else(|| get_nested_string(&object, &["auth", "access_token"]))
        {
            object.insert("access_token".to_string(), Value::String(value));
        }
    }

    if !object.contains_key("type") && !object.contains_key("typo") {
        if let Some(value) = get_string(&object, &["type", "typo", "provider_type"]) {
            object.insert("type".to_string(), Value::String(value));
        }
    }

    if !object.contains_key("chatgpt_account_id") {
        if let Some(value) = get_string(
            &object,
            &[
                "chatgptAccountId",
                "accountId",
                "openai_account_id",
                "chatgpt_account",
            ],
        ) {
            object.insert("chatgpt_account_id".to_string(), Value::String(value));
        }
    }

    if !object.contains_key("account_id") {
        if let Some(value) = get_string(
            &object,
            &[
                "accountId",
                "chatgptAccountId",
                "openai_account_id",
                "chatgpt_account",
            ],
        ) {
            object.insert("account_id".to_string(), Value::String(value));
        }
    }

    if !object.contains_key("email") {
        if let Some(value) = get_string(&object, &["mail", "username", "user_email"]) {
            object.insert("email".to_string(), Value::String(value));
        }
    }

    if !object.contains_key("label") {
        if let Some(value) = get_string(&object, &["name", "account_name"]) {
            object.insert("label".to_string(), Value::String(value));
        }
    }

    if !object.contains_key("base_url") {
        if let Some(value) = get_string(&object, &["baseUrl", "endpoint", "upstream_base_url"]) {
            object.insert("base_url".to_string(), Value::String(value));
        }
    }

    if !object.contains_key("priority") {
        if let Some(raw_priority) = get_string(&object, &["weight", "rank", "prio"]) {
            if let Ok(priority) = raw_priority.parse::<i64>() {
                object.insert("priority".to_string(), Value::Number(priority.into()));
            }
        }
    }

    if !object.contains_key("enabled") {
        if let Some(raw_enabled) = get_string(&object, &["is_enabled", "active", "status_enabled"]) {
            if let Some(enabled) = parse_bool_like(raw_enabled.as_str()) {
                object.insert("enabled".to_string(), Value::Bool(enabled));
            }
        }
    }

    Value::Object(object)
}

fn parse_bool_like(raw: &str) -> Option<bool> {
    if matches!(raw.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on") {
        return Some(true);
    }
    if matches!(raw.to_ascii_lowercase().as_str(), "0" | "false" | "no" | "off") {
        return Some(false);
    }
    None
}

fn build_failed_line_item(
    source_file: &str,
    line_no: u64,
    error_code: &str,
    error_message: String,
    raw_record: Option<Value>,
) -> PersistedImportItem {
    PersistedImportItem {
        item: OAuthImportJobItem {
            item_id: 0,
            source_file: source_file.to_string(),
            line_no,
            status: OAuthImportItemStatus::Failed,
            label: String::new(),
            email: None,
            chatgpt_account_id: None,
            account_id: None,
            error_code: Some(error_code.to_string()),
            error_message: Some(error_message),
            admission_status: Some(OAuthVaultRecordStatus::Failed),
            admission_source: None,
            admission_reason: Some(error_code.to_string()),
            failure_stage: None,
            attempt_count: 0,
            transient_retry_count: 0,
            next_retry_at: None,
            retryable: false,
            terminal_reason: Some(error_code.to_string()),
        },
        request: None,
        raw_record,
        normalized_record: None,
        retry_count: 0,
    }
}

fn normalize_record(
    record: CredentialRecord,
    options: &CreateOAuthImportJobOptions,
    item: &mut OAuthImportJobItem,
) -> Option<ImportTaskRequest> {
    let token_info_chatgpt_account_id = record
        .token_info
        .and_then(|token_info| token_info.chatgpt_account_id)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let refresh_token = record
        .refresh_token
        .as_deref()
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(ToString::to_string);
    let access_token = record
        .access_token
        .or(record.bearer_token)
        .as_deref()
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(ToString::to_string);
    let fallback_token_expires_at =
        derive_one_time_expires_at(record.exp, record.expired.as_deref(), access_token.as_deref());

    let chatgpt_account_id = record
        .chatgpt_account_id
        .or(record.account_id)
        .or(token_info_chatgpt_account_id)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let email = record
        .email
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let record_type = record
        .record_type
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let mode = parse_import_mode(
        record.mode.as_deref(),
        record_type.as_deref(),
        &options.default_mode,
    );
    let chatgpt_plan_type = record
        .chatgpt_plan_type
        .or_else(|| {
            record
                .openai_auth
                .and_then(|openai_auth| openai_auth.chatgpt_plan_type)
        })
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let derived_label = record
        .label
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            email
                .as_deref()
                .map(|value| format!("oauth-{}", value.to_ascii_lowercase()))
        })
        .or_else(|| {
            chatgpt_account_id
                .as_deref()
                .map(|value| format!("oauth-{value}"))
        })
        .unwrap_or_else(|| {
            let hash = refresh_token
                .as_ref()
                .or(access_token.as_ref())
                .map(|token| short_hash(token))
                .unwrap_or_else(|| short_hash("empty"));
            format!("oauth-{hash}")
        });

    item.label = derived_label.clone();
    item.email = email;
    item.chatgpt_account_id = chatgpt_account_id.clone();
    let base_url = record
        .base_url
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| options.base_url.clone());

    match options.credential_mode {
        ImportCredentialMode::Auto => {
            if let Some(refresh_token) = refresh_token {
                return Some(ImportTaskRequest::OAuthRefresh(
                    ImportOAuthRefreshTokenRequest {
                        label: derived_label,
                        base_url: base_url.clone(),
                        refresh_token,
                        fallback_access_token: access_token.clone(),
                        fallback_token_expires_at,
                        chatgpt_account_id: chatgpt_account_id.clone(),
                        mode: Some(mode.clone()),
                        enabled: Some(record.enabled.unwrap_or(options.default_enabled)),
                        priority: Some(record.priority.unwrap_or(options.default_priority)),
                        chatgpt_plan_type: chatgpt_plan_type.clone(),
                        source_type: record_type.clone(),
                    },
                ));
            }

            if let Some(access_token) = access_token {
                let token_expires_at = derive_one_time_expires_at(
                    record.exp,
                    record.expired.as_deref(),
                    Some(access_token.as_str()),
                );
                return Some(ImportTaskRequest::OneTimeAccessToken(
                    UpsertOneTimeSessionAccountRequest {
                        label: derived_label,
                        mode,
                        base_url,
                        access_token,
                        chatgpt_account_id,
                        enabled: Some(record.enabled.unwrap_or(options.default_enabled)),
                        priority: Some(record.priority.unwrap_or(options.default_priority)),
                        token_expires_at,
                        chatgpt_plan_type,
                        source_type: record_type,
                    },
                ));
            }

            item.status = OAuthImportItemStatus::Failed;
            item.error_code = Some("missing_credentials".to_string());
            item.error_message =
                Some("record does not contain refresh_token or access_token".to_string());
            None
        }
        ImportCredentialMode::RefreshToken => {
            if let Some(refresh_token) = refresh_token {
                return Some(ImportTaskRequest::OAuthRefresh(
                    ImportOAuthRefreshTokenRequest {
                        label: derived_label,
                        base_url,
                        refresh_token,
                        fallback_access_token: access_token,
                        fallback_token_expires_at,
                        chatgpt_account_id,
                        mode: Some(mode),
                        enabled: Some(record.enabled.unwrap_or(options.default_enabled)),
                        priority: Some(record.priority.unwrap_or(options.default_priority)),
                        chatgpt_plan_type,
                        source_type: record_type,
                    },
                ));
            }

            item.status = OAuthImportItemStatus::Failed;
            item.error_code = Some("missing_refresh_token".to_string());
            item.error_message =
                Some("record does not contain refresh_token for selected import mode".to_string());
            None
        }
        ImportCredentialMode::AccessToken => {
            if let Some(access_token) = access_token {
                let token_expires_at = derive_one_time_expires_at(
                    record.exp,
                    record.expired.as_deref(),
                    Some(access_token.as_str()),
                );
                return Some(ImportTaskRequest::OneTimeAccessToken(
                    UpsertOneTimeSessionAccountRequest {
                        label: derived_label,
                        mode,
                        base_url,
                        access_token,
                        chatgpt_account_id,
                        enabled: Some(record.enabled.unwrap_or(options.default_enabled)),
                        priority: Some(record.priority.unwrap_or(options.default_priority)),
                        token_expires_at,
                        chatgpt_plan_type,
                        source_type: record_type,
                    },
                ));
            }

            item.status = OAuthImportItemStatus::Failed;
            item.error_code = Some("missing_access_token".to_string());
            item.error_message =
                Some("record does not contain access_token for selected import mode".to_string());
            None
        }
    }
}

fn parse_import_mode(raw_mode: Option<&str>, raw_type: Option<&str>, fallback: &UpstreamMode) -> UpstreamMode {
    if let Some(mode) = raw_mode {
        match mode.trim().to_ascii_lowercase().as_str() {
            "open_ai_api_key" | "openai" | "api_key" => return UpstreamMode::OpenAiApiKey,
            "chat_gpt_session" | "chat_gpt_oauth" | "chatgpt" | "chatgpt_oauth" => {
                return UpstreamMode::ChatGptSession
            }
            "codex_oauth" | "codex_session" | "codex" => return UpstreamMode::CodexOauth,
            _ => {}
        }
    }

    if let Some(record_type) = raw_type {
        if record_type.trim().eq_ignore_ascii_case("codex") {
            return UpstreamMode::CodexOauth;
        }
    }

    fallback.clone()
}

fn derive_one_time_expires_at(
    exp_epoch_sec: Option<i64>,
    expired_raw: Option<&str>,
    access_token: Option<&str>,
) -> Option<DateTime<Utc>> {
    if let Some(epoch) = exp_epoch_sec.filter(|value| *value > 0) {
        if let Some(ts) = DateTime::<Utc>::from_timestamp(epoch, 0) {
            return Some(ts);
        }
    }

    if let Some(ts) = expired_raw
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc))
    {
        return Some(ts);
    }

    access_token.and_then(parse_jwt_exp_from_access_token)
}

fn parse_jwt_exp_from_access_token(access_token: &str) -> Option<DateTime<Utc>> {
    let mut segments = access_token.split('.');
    let _header = segments.next()?;
    let payload = segments.next()?;
    let _signature = segments.next()?;

    let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(payload))
        .ok()?;

    let payload_json = serde_json::from_slice::<Value>(&payload_bytes).ok()?;
    let exp = payload_json.get("exp").and_then(|value| {
        value.as_i64().or_else(|| {
            value
                .as_u64()
                .and_then(|number| i64::try_from(number).ok())
        })
    })?;
    if exp <= 0 {
        return None;
    }

    DateTime::<Utc>::from_timestamp(exp, 0)
}

fn short_hash(raw: &str) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    let hash = hex::encode(hasher.finalize());
    hash.chars().take(12).collect()
}

fn refresh_summary_counts(
    summary: &mut OAuthImportJobSummary,
    items: &[PersistedImportItem],
    cancel_requested: bool,
) {
    let mut processed = 0_u64;
    let mut created = 0_u64;
    let mut updated = 0_u64;
    let mut failed = 0_u64;
    let mut skipped = 0_u64;
    let mut errors = HashMap::<String, u64>::new();
    let mut admission_counts = crate::contracts::OAuthImportAdmissionCounts::default();

    for item in items {
        match item.item.status {
            OAuthImportItemStatus::Created => {
                processed = processed.saturating_add(1);
                created = created.saturating_add(1);
            }
            OAuthImportItemStatus::Updated => {
                processed = processed.saturating_add(1);
                updated = updated.saturating_add(1);
            }
            OAuthImportItemStatus::Failed => {
                processed = processed.saturating_add(1);
                failed = failed.saturating_add(1);
                let code = item
                    .item
                    .error_code
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string());
                *errors.entry(code).or_insert(0) += 1;
            }
            OAuthImportItemStatus::Skipped => {
                skipped = skipped.saturating_add(1);
            }
            _ => {}
        }

        match item.item.admission_status {
            Some(OAuthVaultRecordStatus::Ready) => {
                admission_counts.ready = admission_counts.ready.saturating_add(1)
            }
            Some(OAuthVaultRecordStatus::NeedsRefresh) => {
                admission_counts.needs_refresh =
                    admission_counts.needs_refresh.saturating_add(1)
            }
            Some(OAuthVaultRecordStatus::NoQuota) => {
                admission_counts.no_quota = admission_counts.no_quota.saturating_add(1)
            }
            Some(OAuthVaultRecordStatus::Failed) => {
                admission_counts.failed = admission_counts.failed.saturating_add(1)
            }
            Some(OAuthVaultRecordStatus::Queued) | None => {}
        }
    }

    summary.total = items.len() as u64;
    summary.processed = processed;
    summary.created_count = created;
    summary.updated_count = updated;
    summary.failed_count = failed;
    summary.skipped_count = skipped;

    let mut error_summary = errors
        .into_iter()
        .map(|(error_code, count)| OAuthImportErrorSummary { error_code, count })
        .collect::<Vec<_>>();
    error_summary.sort_by(|left, right| right.count.cmp(&left.count));
    summary.error_summary = error_summary;
    summary.admission_counts = admission_counts;

    if summary.status == OAuthImportJobStatus::Running {
        summary.throughput_per_min =
            compute_throughput_per_min(summary.started_at, summary.finished_at, summary.processed);
    }

    if cancel_requested && matches!(summary.status, OAuthImportJobStatus::Queued) {
        summary.status = OAuthImportJobStatus::Cancelled;
    }
}

fn compute_throughput_per_min(
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    processed: u64,
) -> Option<f64> {
    let started_at = started_at?;
    if processed == 0 {
        return None;
    }

    let end = finished_at.unwrap_or_else(Utc::now);
    let elapsed_ms = (end - started_at).num_milliseconds().max(1) as f64;
    let per_min = processed as f64 / (elapsed_ms / 60_000.0);
    Some((per_min * 100.0).round() / 100.0)
}

fn truncate_error_message(raw: String) -> String {
    const MAX_LEN: usize = 512;
    if raw.len() <= MAX_LEN {
        return raw;
    }

    raw.chars().take(MAX_LEN).collect()
}

fn classify_import_failure_code(message: &str) -> &'static str {
    let lowered = message.to_ascii_lowercase();

    if lowered.contains("refresh_token_reused") {
        return "refresh_token_reused";
    }
    if lowered.contains("invalid refresh token") || lowered.contains("invalid_refresh_token") {
        return "invalid_refresh_token";
    }
    if lowered.contains("missing_client_id")
        || lowered.contains("oauth token endpoint is not configured")
    {
        return "oauth_provider_not_configured";
    }
    if lowered.contains("429")
        || lowered.contains("rate limit")
        || lowered.contains("too many requests")
    {
        return "rate_limited";
    }
    if lowered.contains("timeout")
        || lowered.contains("timed out")
        || lowered.contains("connection reset")
        || lowered.contains("connection refused")
        || lowered.contains("connection closed")
        || lowered.contains("transport error")
    {
        return "upstream_network_error";
    }
    if lowered.contains("503")
        || lowered.contains("502")
        || lowered.contains("500")
        || lowered.contains("temporarily unavailable")
        || lowered.contains("service unavailable")
    {
        return "upstream_unavailable";
    }

    "import_failed"
}
