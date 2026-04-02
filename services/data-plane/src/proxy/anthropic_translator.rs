#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClaudeCodeFamily {
    Opus,
    Sonnet,
    Haiku,
}

fn resolve_claude_code_target_model(
    state: &AppState,
    requested_model: &str,
) -> Result<String, Response> {
    let family = normalize_claude_code_family(requested_model).ok_or_else(|| {
        anthropic_json_error(
            StatusCode::BAD_REQUEST,
            "invalid_request_error",
            "unsupported Claude family; expected an Opus, Sonnet, or Haiku model",
        )
    })?;
    let settings = state.claude_code_routing_settings();
    if !settings.enabled {
        return Err(anthropic_json_error(
            StatusCode::BAD_REQUEST,
            "invalid_request_error",
            "Claude Code routing is disabled",
        ));
    }

    let target_model = match family {
        ClaudeCodeFamily::Opus => settings.opus_target_model,
        ClaudeCodeFamily::Sonnet => settings.sonnet_target_model,
        ClaudeCodeFamily::Haiku => settings.haiku_target_model,
    }
    .filter(|value| !value.trim().is_empty())
    .ok_or_else(|| {
        anthropic_json_error(
            StatusCode::BAD_REQUEST,
            "invalid_request_error",
            "requested Claude family is not mapped to an internal target model",
        )
    })?;

    Ok(target_model)
}

fn normalize_claude_code_family(model: &str) -> Option<ClaudeCodeFamily> {
    let normalized = model.trim().to_ascii_lowercase();
    if !normalized.starts_with("claude") {
        return None;
    }
    if normalized.contains("opus") {
        return Some(ClaudeCodeFamily::Opus);
    }
    if normalized.contains("sonnet") {
        return Some(ClaudeCodeFamily::Sonnet);
    }
    if normalized.contains("haiku") {
        return Some(ClaudeCodeFamily::Haiku);
    }
    None
}

fn translate_anthropic_messages_request(
    request_value: &Value,
    target_model: &str,
) -> Result<Value, Response> {
    let messages = request_value
        .get("messages")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            anthropic_json_error(
                StatusCode::BAD_REQUEST,
                "invalid_request_error",
                "messages must be an array",
            )
        })?;

    let mut input = Vec::with_capacity(messages.len());
    for message in messages {
        let role = message
            .get("role")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                anthropic_json_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_request_error",
                    "each message must include a role",
                )
            })?;
        let content = translate_anthropic_message_content(message.get("content"))?;
        input.push(serde_json::json!({
            "role": role,
            "content": content,
        }));
    }

    let mut object = serde_json::Map::new();
    object.insert(
        "model".to_string(),
        Value::String(target_model.to_string()),
    );
    object.insert("input".to_string(), Value::Array(input));
    if let Some(instructions) = anthropic_system_to_instructions(request_value.get("system")) {
        object.insert("instructions".to_string(), Value::String(instructions));
    }
    if let Some(max_tokens) = request_value.get("max_tokens").and_then(Value::as_i64) {
        object.insert(
            "max_output_tokens".to_string(),
            Value::Number(serde_json::Number::from(max_tokens.max(0))),
        );
    }
    if let Some(stream) = request_value.get("stream").and_then(Value::as_bool) {
        object.insert("stream".to_string(), Value::Bool(stream));
    }
    if let Some(temperature) = request_value.get("temperature").cloned() {
        object.insert("temperature".to_string(), temperature);
    }
    if let Some(metadata) = request_value.get("metadata").cloned() {
        object.insert("metadata".to_string(), metadata);
    }
    if let Some(thinking) = request_value.get("thinking").cloned() {
        object.insert("reasoning".to_string(), thinking);
    }
    if let Some(context_management) = request_value.get("context_management").cloned() {
        object.insert("context_management".to_string(), context_management);
    }
    if let Some(output_config) = request_value.get("output_config").cloned() {
        object.insert("output_config".to_string(), output_config);
    }
    if let Some(speed) = request_value.get("speed").cloned() {
        object.insert("speed".to_string(), speed);
    }
    if let Some(tools) = request_value.get("tools").and_then(Value::as_array) {
        object.insert(
            "tools".to_string(),
            Value::Array(
                tools
                    .iter()
                    .map(translate_anthropic_tool_definition)
                    .collect(),
            ),
        );
    }
    if let Some(tool_choice) = request_value.get("tool_choice") {
        object.insert(
            "tool_choice".to_string(),
            translate_anthropic_tool_choice(tool_choice),
        );
    }

    Ok(Value::Object(object))
}

fn anthropic_system_to_instructions(system: Option<&Value>) -> Option<String> {
    match system {
        Some(Value::String(text)) => {
            let trimmed = text.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        Some(Value::Array(items)) => {
            let parts = items
                .iter()
                .filter_map(|item| match item {
                    Value::String(text) => Some(text.trim().to_string()),
                    Value::Object(map) => map
                        .get("text")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(ToString::to_string),
                    _ => None,
                })
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>();
            (!parts.is_empty()).then(|| parts.join("\n\n"))
        }
        _ => None,
    }
}

fn translate_anthropic_message_content(content: Option<&Value>) -> Result<Vec<Value>, Response> {
    match content {
        Some(Value::String(text)) => Ok(vec![serde_json::json!({
            "type": "input_text",
            "text": text,
        })]),
        Some(Value::Array(blocks)) => blocks
            .iter()
            .map(translate_anthropic_content_block)
            .collect(),
        Some(Value::Null) | None => Ok(Vec::new()),
        Some(other) => Ok(vec![serde_json::json!({
            "type": "input_text",
            "text": other.to_string(),
        })]),
    }
}

fn translate_anthropic_content_block(block: &Value) -> Result<Value, Response> {
    let Some(block_type) = block.get("type").and_then(Value::as_str) else {
        return Ok(serde_json::json!({
            "type": "input_text",
            "text": block.to_string(),
        }));
    };

    match block_type {
        "text" => Ok(serde_json::json!({
            "type": "input_text",
            "text": block.get("text").and_then(Value::as_str).unwrap_or_default(),
        })),
        "tool_result" => Ok(serde_json::json!({
            "type": "function_call_output",
            "call_id": block.get("tool_use_id").and_then(Value::as_str).unwrap_or_default(),
            "output": block
                .get("content")
                .cloned()
                .unwrap_or(Value::String(String::new())),
        })),
        "image" => Ok(translate_anthropic_image_block(block)),
        _ => Ok(serde_json::json!({
            "type": "input_text",
            "text": block.to_string(),
        })),
    }
}

fn translate_anthropic_image_block(block: &Value) -> Value {
    let Some(source) = block.get("source").and_then(Value::as_object) else {
        return serde_json::json!({
            "type": "input_text",
            "text": block.to_string(),
        });
    };

    match source.get("type").and_then(Value::as_str) {
        Some("base64") => {
            let media_type = source
                .get("media_type")
                .and_then(Value::as_str)
                .unwrap_or("application/octet-stream");
            let data = source.get("data").and_then(Value::as_str).unwrap_or_default();
            serde_json::json!({
                "type": "input_image",
                "image_url": format!("data:{media_type};base64,{data}"),
            })
        }
        Some("url") => serde_json::json!({
            "type": "input_image",
            "image_url": source.get("url").and_then(Value::as_str).unwrap_or_default(),
        }),
        _ => serde_json::json!({
            "type": "input_text",
            "text": block.to_string(),
        }),
    }
}

fn translate_anthropic_tool_definition(tool: &Value) -> Value {
    let mut mapped = serde_json::Map::new();
    mapped.insert("type".to_string(), Value::String("function".to_string()));
    if let Some(name) = tool.get("name").cloned() {
        mapped.insert("name".to_string(), name);
    }
    if let Some(description) = tool.get("description").cloned() {
        mapped.insert("description".to_string(), description);
    }
    if let Some(parameters) = tool.get("input_schema").cloned() {
        mapped.insert("parameters".to_string(), parameters);
    }
    for key in ["strict", "defer_loading", "eager_input_streaming", "cache_control"] {
        if let Some(value) = tool.get(key).cloned() {
            mapped.insert(key.to_string(), value);
        }
    }
    Value::Object(mapped)
}

fn translate_anthropic_tool_choice(choice: &Value) -> Value {
    match choice {
        Value::String(raw) if raw.eq_ignore_ascii_case("any") => {
            Value::String("required".to_string())
        }
        Value::Object(map) if map.get("type").and_then(Value::as_str) == Some("tool") => {
            serde_json::json!({
                "type": "function",
                "name": map.get("name").and_then(Value::as_str).unwrap_or_default(),
            })
        }
        other => other.clone(),
    }
}

fn translate_responses_json_to_anthropic_message(
    response_value: &Value,
    requested_model: &str,
) -> Value {
    let content = anthropic_content_from_responses_output(response_value);
    serde_json::json!({
        "id": response_value
            .get("id")
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .unwrap_or_else(anthropic_generated_message_id),
        "type": "message",
        "role": "assistant",
        "model": requested_model,
        "content": content,
        "stop_reason": anthropic_stop_reason(response_value),
        "stop_sequence": Value::Null,
        "usage": anthropic_usage_value(response_value),
    })
}

fn anthropic_content_from_responses_output(response_value: &Value) -> Vec<Value> {
    let mut content = Vec::new();
    if let Some(output) = response_value.get("output").and_then(Value::as_array) {
        for item in output {
            match item.get("type").and_then(Value::as_str) {
                Some("message") => {
                    if let Some(blocks) = item.get("content").and_then(Value::as_array) {
                        for block in blocks {
                            if let Some(text) = block.get("text").and_then(Value::as_str) {
                                content.push(serde_json::json!({
                                    "type": "text",
                                    "text": text,
                                }));
                            }
                        }
                    }
                }
                Some("function_call") => {
                    let input = item
                        .get("arguments")
                        .map(parse_response_function_arguments)
                        .unwrap_or(Value::Object(serde_json::Map::new()));
                    content.push(serde_json::json!({
                        "type": "tool_use",
                        "id": item
                            .get("call_id")
                            .or_else(|| item.get("id"))
                            .and_then(Value::as_str)
                            .map(ToString::to_string)
                            .unwrap_or_else(|| format!("toolu_{}", uuid::Uuid::new_v4().simple())),
                        "name": item.get("name").and_then(Value::as_str).unwrap_or_default(),
                        "input": input,
                    }));
                }
                _ => {}
            }
        }
    }
    if content.is_empty() {
        if let Some(text) = response_value.get("output_text").and_then(Value::as_str) {
            content.push(serde_json::json!({
                "type": "text",
                "text": text,
            }));
        }
    }
    content
}

fn parse_response_function_arguments(value: &Value) -> Value {
    match value {
        Value::String(raw) => serde_json::from_str(raw)
            .unwrap_or_else(|_| Value::String(raw.to_string())),
        other => other.clone(),
    }
}

fn anthropic_stop_reason(response_value: &Value) -> &'static str {
    if anthropic_content_from_responses_output(response_value)
        .iter()
        .any(|block| block.get("type").and_then(Value::as_str) == Some("tool_use"))
    {
        return "tool_use";
    }
    let incomplete_reason = response_value
        .get("incomplete_details")
        .and_then(|details| details.get("reason"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    if incomplete_reason.contains("max") {
        return "max_tokens";
    }
    "end_turn"
}

fn anthropic_usage_value(response_value: &Value) -> Value {
    let usage = response_value.get("usage");
    serde_json::json!({
        "input_tokens": usage
            .and_then(|value| value.get("input_tokens"))
            .and_then(Value::as_i64)
            .unwrap_or(0),
        "output_tokens": usage
            .and_then(|value| value.get("output_tokens"))
            .and_then(Value::as_i64)
            .unwrap_or(0),
    })
}

fn anthropic_generated_message_id() -> String {
    format!("msg_{}", uuid::Uuid::new_v4().simple())
}

struct AnthropicSseTranslator {
    requested_model: String,
    response_id: Option<String>,
    message_started: bool,
    text_block_started: bool,
    emitted_content_blocks: usize,
}

impl AnthropicSseTranslator {
    fn new(requested_model: String) -> Self {
        Self {
            requested_model,
            response_id: None,
            message_started: false,
            text_block_started: false,
            emitted_content_blocks: 0,
        }
    }

    fn translate_frame(&mut self, payload: &[u8]) -> Vec<Bytes> {
        if payload == b"[DONE]" {
            return Vec::new();
        }
        let Ok(value) = serde_json::from_slice::<Value>(payload) else {
            return Vec::new();
        };
        let event_type = value.get("type").and_then(Value::as_str).unwrap_or_default();
        match event_type {
            "response.created" => self.handle_response_created(&value),
            "response.output_text.delta" => self.handle_output_text_delta(&value),
            "response.completed" => self.handle_response_completed(&value),
            _ => Vec::new(),
        }
    }

    fn handle_response_created(&mut self, value: &Value) -> Vec<Bytes> {
        let response = value.get("response").unwrap_or(value);
        self.response_id = response
            .get("id")
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .or_else(|| self.response_id.clone());
        self.ensure_message_start(response)
    }

    fn handle_output_text_delta(&mut self, value: &Value) -> Vec<Bytes> {
        let mut frames = self.ensure_message_start(value.get("response").unwrap_or(value));
        if !self.text_block_started {
            frames.push(build_sse_frame(
                Some("content_block_start"),
                &serde_json::json!({
                    "type": "content_block_start",
                    "index": 0,
                    "content_block": {
                        "type": "text",
                        "text": "",
                    }
                }),
            ));
            self.text_block_started = true;
            self.emitted_content_blocks = self.emitted_content_blocks.max(1);
        }
        if let Some(delta) = value.get("delta").and_then(Value::as_str) {
            frames.push(build_sse_frame(
                Some("content_block_delta"),
                &serde_json::json!({
                    "type": "content_block_delta",
                    "index": 0,
                    "delta": {
                        "type": "text_delta",
                        "text": delta,
                    }
                }),
            ));
        }
        frames
    }

    fn handle_response_completed(&mut self, value: &Value) -> Vec<Bytes> {
        let response = value.get("response").unwrap_or(value);
        self.response_id = response
            .get("id")
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .or_else(|| self.response_id.clone());

        let translated = translate_responses_json_to_anthropic_message(
            response,
            self.requested_model.as_str(),
        );
        let mut frames = self.ensure_message_start(response);
        let content_blocks = translated
            .get("content")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        if self.text_block_started {
            frames.push(build_sse_frame(
                Some("content_block_stop"),
                &serde_json::json!({
                    "type": "content_block_stop",
                    "index": 0,
                }),
            ));
            for (index, block) in content_blocks.iter().enumerate().skip(1) {
                self.emit_full_content_block(index, block, &mut frames);
            }
        } else {
            for (index, block) in content_blocks.iter().enumerate() {
                self.emit_full_content_block(index, block, &mut frames);
            }
        }

        frames.push(build_sse_frame(
            Some("message_delta"),
            &serde_json::json!({
                "type": "message_delta",
                "delta": {
                    "stop_reason": translated.get("stop_reason").cloned().unwrap_or(Value::Null),
                    "stop_sequence": Value::Null,
                },
                "usage": translated.get("usage").cloned().unwrap_or_else(|| anthropic_usage_value(response)),
            }),
        ));
        frames.push(build_sse_frame(
            Some("message_stop"),
            &serde_json::json!({
                "type": "message_stop"
            }),
        ));
        frames
    }

    fn ensure_message_start(&mut self, response: &Value) -> Vec<Bytes> {
        if self.message_started {
            return Vec::new();
        }
        self.message_started = true;
        let response_id = self
            .response_id
            .clone()
            .unwrap_or_else(anthropic_generated_message_id);
        vec![build_sse_frame(
            Some("message_start"),
            &serde_json::json!({
                "type": "message_start",
                "message": {
                    "id": response_id,
                    "type": "message",
                    "role": "assistant",
                    "model": self.requested_model.clone(),
                    "content": [],
                    "stop_reason": Value::Null,
                    "stop_sequence": Value::Null,
                    "usage": {
                        "input_tokens": response
                            .get("usage")
                            .and_then(|usage| usage.get("input_tokens"))
                            .and_then(Value::as_i64)
                            .unwrap_or(0),
                        "output_tokens": 0,
                    }
                }
            }),
        )]
    }

    fn emit_full_content_block(&mut self, index: usize, block: &Value, frames: &mut Vec<Bytes>) {
        let block_type = block.get("type").and_then(Value::as_str).unwrap_or_default();
        frames.push(build_sse_frame(
            Some("content_block_start"),
            &serde_json::json!({
                "type": "content_block_start",
                "index": index,
                "content_block": match block_type {
                    "text" => serde_json::json!({
                        "type": "text",
                        "text": "",
                    }),
                    _ => block.clone(),
                }
            }),
        ));
        if block_type == "text" {
            if let Some(text) = block.get("text").and_then(Value::as_str) {
                frames.push(build_sse_frame(
                    Some("content_block_delta"),
                    &serde_json::json!({
                        "type": "content_block_delta",
                        "index": index,
                        "delta": {
                            "type": "text_delta",
                            "text": text,
                        }
                    }),
                ));
            }
        }
        frames.push(build_sse_frame(
            Some("content_block_stop"),
            &serde_json::json!({
                "type": "content_block_stop",
                "index": index,
            }),
        ));
        self.emitted_content_blocks = self.emitted_content_blocks.max(index + 1);
    }
}
