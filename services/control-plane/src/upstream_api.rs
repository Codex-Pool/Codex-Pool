use codex_pool_core::model::UpstreamMode;

pub fn build_upstream_models_url(base_url: &str, mode: &UpstreamMode) -> anyhow::Result<String> {
    let mut url = reqwest::Url::parse(base_url)?;
    let base_path = url.path().trim_end_matches('/').to_string();

    let target_path = match mode {
        UpstreamMode::ChatGptSession | UpstreamMode::CodexOauth => {
            if base_path.ends_with("/backend-api/codex") || base_path.ends_with("/v1") {
                format!("{base_path}/models")
            } else {
                format!("{base_path}/backend-api/codex/models")
            }
        }
        UpstreamMode::OpenAiApiKey => {
            if base_path.ends_with("/v1") {
                format!("{base_path}/models")
            } else {
                format!("{base_path}/v1/models")
            }
        }
    };

    url.set_path(&target_path);

    if matches!(mode, UpstreamMode::ChatGptSession | UpstreamMode::CodexOauth) {
        url.query_pairs_mut()
            .append_pair("client_version", env!("CARGO_PKG_VERSION"));
    }

    Ok(url.to_string())
}

pub fn build_upstream_responses_url(
    base_url: &str,
    mode: &UpstreamMode,
) -> anyhow::Result<String> {
    let mut url = reqwest::Url::parse(base_url)?;
    let base_path = url.path().trim_end_matches('/').to_string();

    let target_path = match mode {
        UpstreamMode::ChatGptSession | UpstreamMode::CodexOauth => {
            if base_path.ends_with("/backend-api/codex") || base_path.ends_with("/v1") {
                format!("{base_path}/responses")
            } else {
                format!("{base_path}/backend-api/codex/responses")
            }
        }
        UpstreamMode::OpenAiApiKey => {
            if base_path.ends_with("/v1") {
                format!("{base_path}/responses")
            } else {
                format!("{base_path}/v1/responses")
            }
        }
    };

    url.set_path(&target_path);
    Ok(url.to_string())
}

pub fn normalise_models_payload(
    payload: serde_json::Value,
    mode: &UpstreamMode,
) -> serde_json::Value {
    if payload.get("data").is_some() {
        return payload;
    }

    let models = match payload.get("models").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return payload,
    };

    let provider = match mode {
        UpstreamMode::ChatGptSession => "chatgpt-session",
        UpstreamMode::CodexOauth => "codex-oauth",
        UpstreamMode::OpenAiApiKey => "openai",
    };

    let data: Vec<serde_json::Value> = models
        .iter()
        .map(|m| {
            let id = m.get("slug").and_then(|v| v.as_str()).unwrap_or("unknown");
            let visibility = m
                .get("visibility")
                .and_then(|value| value.as_str())
                .map(ToString::to_string);
            serde_json::json!({
                "id": id,
                "object": "model",
                "created": 0,
                "owned_by": provider,
                "visibility": visibility,
            })
        })
        .collect();

    serde_json::json!({
        "object": "list",
        "data": data,
    })
}
