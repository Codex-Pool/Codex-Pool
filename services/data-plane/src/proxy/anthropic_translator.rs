use codex_pool_core::model::{
    ClaudeCodeEffortFallbackMode, ClaudeCodeEffortRoutingSettings, ClaudeCodeRoutingSettings,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClaudeCodeFamily {
    Opus,
    Sonnet,
    Haiku,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedClaudeCodeTarget {
    family: ClaudeCodeFamily,
    target_model: String,
}

fn resolve_claude_code_target_model(
    settings: &ClaudeCodeRoutingSettings,
    requested_model: &str,
) -> Result<ResolvedClaudeCodeTarget, Response> {
    let family = normalize_claude_code_family(requested_model).ok_or_else(|| {
        anthropic_json_error(
            StatusCode::BAD_REQUEST,
            "invalid_request_error",
            "unsupported Claude family; expected an Opus, Sonnet, or Haiku model",
        )
    })?;
    if !settings.enabled {
        return Err(anthropic_json_error(
            StatusCode::BAD_REQUEST,
            "invalid_request_error",
            "Claude Code routing is disabled",
        ));
    }

    let target_model = match family {
        ClaudeCodeFamily::Opus => settings.opus_target_model.clone(),
        ClaudeCodeFamily::Sonnet => settings.sonnet_target_model.clone(),
        ClaudeCodeFamily::Haiku => settings.haiku_target_model.clone(),
    }
    .filter(|value| !value.trim().is_empty())
    .ok_or_else(|| {
        anthropic_json_error(
            StatusCode::BAD_REQUEST,
            "invalid_request_error",
            "requested Claude family is not mapped to an internal target model",
        )
    })?;

    Ok(ResolvedClaudeCodeTarget {
        family,
        target_model,
    })
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
    family: ClaudeCodeFamily,
    target_model: &str,
    effort_routing: &ClaudeCodeEffortRoutingSettings,
) -> Result<TranslatedAnthropicRequest, Response> {
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

    let (effective_messages, context_management) =
        apply_anthropic_context_management(request_value, messages)?;
    let object = build_translated_responses_request(
        request_value,
        family,
        target_model,
        effort_routing,
        effective_messages.as_slice(),
    )?;
    let estimated_input_tokens =
        estimate_anthropic_request_input_tokens(request_value, effective_messages.as_slice()).max(0);

    Ok(TranslatedAnthropicRequest {
        body: Value::Object(object),
        estimated_input_tokens,
        context_management,
    })
}

#[derive(Debug, Clone)]
struct TranslatedAnthropicRequest {
    body: Value,
    estimated_input_tokens: i64,
    context_management: AnthropicContextManagementOutcome,
}

#[derive(Debug, Clone, Default)]
struct AnthropicContextManagementOutcome {
    requested: bool,
    original_input_tokens: i64,
    applied_edits: Vec<Value>,
}

impl AnthropicContextManagementOutcome {
    fn response_value(&self) -> Option<Value> {
        self.requested.then(|| {
            serde_json::json!({
                "applied_edits": self.applied_edits.clone(),
            })
        })
    }

    fn count_tokens_value(&self) -> Option<Value> {
        self.requested.then(|| {
            serde_json::json!({
                "original_input_tokens": self.original_input_tokens.max(0),
            })
        })
    }
}

const CONTEXT_TOOL_RESULT_CLEARED_MESSAGE: &str = "[Old tool result content cleared]";
const DEFAULT_CONTEXT_EDITING_INPUT_TOKENS_TRIGGER: i64 = 100_000;
const DEFAULT_CONTEXT_EDITING_KEEP_TOOL_USES: usize = 3;
const ROUGH_IMAGE_TOKEN_ESTIMATE: i64 = 2_000;
const CLEARED_TOOL_INPUT_JSON: &str = "{}";

fn build_translated_responses_request(
    request_value: &Value,
    family: ClaudeCodeFamily,
    target_model: &str,
    effort_routing: &ClaudeCodeEffortRoutingSettings,
    messages: &[Value],
) -> Result<serde_json::Map<String, Value>, Response> {

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
        input.extend(translate_anthropic_message_items(
            role,
            message.get("content"),
        )?);
    }

    let mut object = serde_json::Map::new();
    object.insert("model".to_string(), Value::String(target_model.to_string()));
    object.insert("input".to_string(), Value::Array(input));
    // Claude Code expects stateless messages semantics, while the Codex upstream
    // rejects persisted responses for this compatibility path.
    object.insert("store".to_string(), Value::Bool(false));
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
    // Claude Code sends temperature for Anthropic requests, but the GPT-5
    // Responses backends behind this compatibility path may reject it.
    // Keeping the Anthropic default is more compatible than forwarding it.
    if let Some(reasoning) = translate_anthropic_reasoning(
        family,
        target_model,
        effort_routing,
        request_value.get("thinking"),
        request_value.get("output_config"),
    ) {
        object.insert("reasoning".to_string(), reasoning);
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

    Ok(object)
}

fn apply_anthropic_context_management(
    request_value: &Value,
    messages: &[Value],
) -> Result<(Vec<Value>, AnthropicContextManagementOutcome), Response> {
    let Some(context_management) = request_value.get("context_management") else {
        return Ok((messages.to_vec(), AnthropicContextManagementOutcome::default()));
    };
    if context_management.is_null() {
        return Ok((messages.to_vec(), AnthropicContextManagementOutcome::default()));
    }

    let edits = context_management
        .get("edits")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    validate_context_management_edit_order(edits.as_slice())?;

    let mut effective_messages = messages.to_vec();
    let mut outcome = AnthropicContextManagementOutcome {
        requested: true,
        original_input_tokens: estimate_anthropic_request_input_tokens(request_value, messages),
        applied_edits: Vec::new(),
    };

    for edit in edits.iter() {
        let Some(edit_type) = edit.get("type").and_then(Value::as_str) else {
            return Err(anthropic_json_error(
                StatusCode::BAD_REQUEST,
                "invalid_request_error",
                "context_management edits must include a type",
            ));
        };
        let applied = match edit_type {
            "clear_thinking_20251015" => {
                apply_clear_thinking_context_edit(effective_messages.as_mut_slice(), edit)
            }
            "clear_tool_uses_20250919" => apply_clear_tool_uses_context_edit(
                request_value,
                effective_messages.as_mut_slice(),
                edit,
            ),
            _ => {
                return Err(anthropic_json_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_request_error",
                    "unsupported context management edit type",
                ))
            }
        };
        if let Some(applied) = applied {
            outcome.applied_edits.push(applied);
        }
    }

    Ok((effective_messages, outcome))
}

fn validate_context_management_edit_order(edits: &[Value]) -> Result<(), Response> {
    if edits.len() < 2 {
        return Ok(());
    }
    let first_type = edits
        .first()
        .and_then(|value| value.get("type"))
        .and_then(Value::as_str);
    let has_clear_thinking = edits.iter().any(|value| {
        value
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|item| item == "clear_thinking_20251015")
    });
    if has_clear_thinking && first_type != Some("clear_thinking_20251015") {
        return Err(anthropic_json_error(
            StatusCode::BAD_REQUEST,
            "invalid_request_error",
            "clear_thinking_20251015 must be listed first in context_management.edits",
        ));
    }
    Ok(())
}

enum AnthropicInputItem {
    MessageContent(Value),
    Standalone(Value),
    Dropped,
}

fn apply_clear_thinking_context_edit(messages: &mut [Value], edit: &Value) -> Option<Value> {
    let keep = parse_clear_thinking_keep(edit);
    if keep.is_none() {
        return None;
    }
    let keep = keep.unwrap();
    let mut thinking_turn_indices = Vec::new();
    for (message_index, message) in messages.iter().enumerate() {
        let role = message.get("role").and_then(Value::as_str).unwrap_or_default();
        let Some(content) = message.get("content").and_then(Value::as_array) else {
            continue;
        };
        if !role.eq_ignore_ascii_case("assistant") {
            continue;
        }
        if content.iter().any(is_thinking_block) {
            thinking_turn_indices.push(message_index);
        }
    }

    if thinking_turn_indices.len() <= keep {
        return None;
    }

    let preserve_from = thinking_turn_indices.len().saturating_sub(keep);
    let mut cleared_turns = 0usize;
    let mut cleared_tokens = 0i64;
    for message_index in thinking_turn_indices.into_iter().take(preserve_from) {
        let Some(content) = messages
            .get_mut(message_index)
            .and_then(Value::as_object_mut)
            .and_then(|message| message.get_mut("content"))
            .and_then(Value::as_array_mut)
        else {
            continue;
        };
        let before_len = content.len();
        let mut removed_tokens = 0i64;
        content.retain(|block| {
            if is_thinking_block(block) {
                removed_tokens += estimate_anthropic_content_block_tokens(block);
                return false;
            }
            true
        });
        if content.len() != before_len {
            cleared_turns += 1;
            cleared_tokens += removed_tokens.max(0);
        }
    }

    (cleared_turns > 0).then(|| {
        serde_json::json!({
            "type": "clear_thinking_20251015",
            "cleared_thinking_turns": cleared_turns,
            "cleared_input_tokens": cleared_tokens.max(0),
        })
    })
}

fn parse_clear_thinking_keep(edit: &Value) -> Option<usize> {
    match edit.get("keep") {
        Some(Value::String(value)) if value.eq_ignore_ascii_case("all") => None,
        Some(Value::Object(map)) => map
            .get("value")
            .and_then(Value::as_u64)
            .map(|value| value.max(1) as usize)
            .or(Some(1)),
        _ => Some(1),
    }
}

fn apply_clear_tool_uses_context_edit(
    request_value: &Value,
    messages: &mut [Value],
    edit: &Value,
) -> Option<Value> {
    let tool_history = collect_tool_history_entries(messages);
    if tool_history.is_empty() {
        return None;
    }

    let exclude_tools = parse_context_management_tool_names(edit.get("exclude_tools"));
    let clearable_entries = tool_history
        .into_iter()
        .filter(|entry| !exclude_tools.contains(entry.name.as_str()))
        .collect::<Vec<_>>();

    if clearable_entries.is_empty() {
        return None;
    }

    if !tool_clear_trigger_fired(request_value, messages, edit, clearable_entries.as_slice()) {
        return None;
    }

    let keep = parse_tool_use_keep(edit);
    let clear_inputs_for = parse_clear_tool_inputs(edit.get("clear_tool_inputs"));
    let clear_up_to = clearable_entries.len().saturating_sub(keep);
    let minimum_tokens_to_clear = parse_clear_at_least_tokens(edit);
    let mut cleared_tool_uses = 0usize;
    let mut cleared_tokens = 0i64;

    for entry in clearable_entries.iter().take(clear_up_to) {
        let mut changed = false;
        if let Some(saved) = clear_tool_result_content(messages, entry) {
            if saved > 0 {
                cleared_tokens += saved;
                changed = true;
            }
        }
        if should_clear_tool_inputs_for_name(clear_inputs_for.as_ref(), entry.name.as_str()) {
            if let Some(saved) = clear_tool_use_input(messages, entry) {
                if saved > 0 {
                    cleared_tokens += saved;
                    changed = true;
                }
            }
        }
        if changed {
            cleared_tool_uses += 1;
        }
        if minimum_tokens_to_clear > 0 && cleared_tokens >= minimum_tokens_to_clear {
            break;
        }
    }

    (cleared_tool_uses > 0).then(|| {
        serde_json::json!({
            "type": "clear_tool_uses_20250919",
            "cleared_tool_uses": cleared_tool_uses,
            "cleared_input_tokens": cleared_tokens.max(0),
        })
    })
}

#[derive(Debug, Clone)]
struct ToolHistoryEntry {
    name: String,
    use_message_index: usize,
    use_block_index: usize,
    result_message_index: usize,
    result_block_index: usize,
}

fn collect_tool_history_entries(messages: &[Value]) -> Vec<ToolHistoryEntry> {
    let mut entries = Vec::new();
    let mut tool_use_by_id = std::collections::BTreeMap::new();

    for (message_index, message) in messages.iter().enumerate() {
        let role = message.get("role").and_then(Value::as_str).unwrap_or_default();
        let Some(content) = message.get("content").and_then(Value::as_array) else {
            continue;
        };
        if role.eq_ignore_ascii_case("assistant") {
            for (block_index, block) in content.iter().enumerate() {
                if block.get("type").and_then(Value::as_str) != Some("tool_use") {
                    continue;
                }
                let id = block
                    .get("id")
                    .or_else(|| block.get("tool_use_id"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                if id.is_empty() {
                    continue;
                }
                let entry_index = entries.len();
                entries.push(ToolHistoryEntry {
                    name: block
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    use_message_index: message_index,
                    use_block_index: block_index,
                    result_message_index: usize::MAX,
                    result_block_index: usize::MAX,
                });
                tool_use_by_id.insert(id, entry_index);
            }
        } else if role.eq_ignore_ascii_case("user") {
            for (block_index, block) in content.iter().enumerate() {
                if block.get("type").and_then(Value::as_str) != Some("tool_result") {
                    continue;
                }
                let Some(tool_use_id) = block.get("tool_use_id").and_then(Value::as_str) else {
                    continue;
                };
                let Some(entry_index) = tool_use_by_id.get(tool_use_id).copied() else {
                    continue;
                };
                if entries[entry_index].result_message_index != usize::MAX {
                    continue;
                }
                entries[entry_index].result_message_index = message_index;
                entries[entry_index].result_block_index = block_index;
            }
        }
    }

    entries
        .into_iter()
        .filter(|entry| entry.result_message_index != usize::MAX)
        .collect()
}

fn parse_context_management_tool_names(value: Option<&Value>) -> std::collections::BTreeSet<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone)]
enum ClearToolInputsSetting {
    All,
    Named(std::collections::BTreeSet<String>),
}

fn parse_clear_tool_inputs(value: Option<&Value>) -> Option<ClearToolInputsSetting> {
    match value {
        Some(Value::Bool(true)) => Some(ClearToolInputsSetting::All),
        Some(Value::Array(items)) => {
            let names = items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect::<std::collections::BTreeSet<_>>();
            (!names.is_empty()).then_some(ClearToolInputsSetting::Named(names))
        }
        _ => None,
    }
}

fn should_clear_tool_inputs_for_name(
    setting: Option<&ClearToolInputsSetting>,
    name: &str,
) -> bool {
    match setting {
        Some(ClearToolInputsSetting::All) => true,
        Some(ClearToolInputsSetting::Named(names)) => names.contains(name),
        None => false,
    }
}

fn parse_tool_use_keep(edit: &Value) -> usize {
    edit.get("keep")
        .and_then(Value::as_object)
        .and_then(|keep| keep.get("value"))
        .and_then(Value::as_u64)
        .map(|value| value.max(1) as usize)
        .unwrap_or(DEFAULT_CONTEXT_EDITING_KEEP_TOOL_USES)
}

fn parse_clear_at_least_tokens(edit: &Value) -> i64 {
    edit.get("clear_at_least")
        .and_then(Value::as_object)
        .and_then(|value| value.get("value"))
        .and_then(Value::as_i64)
        .unwrap_or_default()
        .max(0)
}

fn tool_clear_trigger_fired(
    request_value: &Value,
    messages: &[Value],
    edit: &Value,
    clearable_entries: &[ToolHistoryEntry],
) -> bool {
    match edit
        .get("trigger")
        .and_then(Value::as_object)
        .and_then(|trigger| trigger.get("type"))
        .and_then(Value::as_str)
    {
        Some("tool_uses") => {
            let threshold = edit
                .get("trigger")
                .and_then(Value::as_object)
                .and_then(|trigger| trigger.get("value"))
                .and_then(Value::as_u64)
                .unwrap_or(DEFAULT_CONTEXT_EDITING_KEEP_TOOL_USES as u64)
                .max(1) as usize;
            clearable_entries.len() >= threshold
        }
        _ => {
            let threshold = edit
                .get("trigger")
                .and_then(Value::as_object)
                .and_then(|trigger| trigger.get("value"))
                .and_then(Value::as_i64)
                .unwrap_or(DEFAULT_CONTEXT_EDITING_INPUT_TOKENS_TRIGGER)
                .max(0);
            estimate_anthropic_request_input_tokens(request_value, messages) >= threshold
        }
    }
}

fn clear_tool_result_content(messages: &mut [Value], entry: &ToolHistoryEntry) -> Option<i64> {
    let block = messages
        .get(entry.result_message_index)
        .and_then(Value::as_object)
        .and_then(|message| message.get("content"))
        .and_then(Value::as_array)
        .and_then(|content| content.get(entry.result_block_index))?;
    if block.get("content").and_then(Value::as_str) == Some(CONTEXT_TOOL_RESULT_CLEARED_MESSAGE) {
        return None;
    }
    let old_tokens = estimate_tool_result_content_tokens(block.get("content"));
    let new_tokens = rough_token_count_estimation(CONTEXT_TOOL_RESULT_CLEARED_MESSAGE);

    let block = messages
        .get_mut(entry.result_message_index)
        .and_then(Value::as_object_mut)
        .and_then(|message| message.get_mut("content"))
        .and_then(Value::as_array_mut)
        .and_then(|content| content.get_mut(entry.result_block_index))
        .and_then(Value::as_object_mut)?;
    block.insert(
        "content".to_string(),
        Value::String(CONTEXT_TOOL_RESULT_CLEARED_MESSAGE.to_string()),
    );
    Some((old_tokens - new_tokens).max(0))
}

fn clear_tool_use_input(messages: &mut [Value], entry: &ToolHistoryEntry) -> Option<i64> {
    let block = messages
        .get(entry.use_message_index)
        .and_then(Value::as_object)
        .and_then(|message| message.get("content"))
        .and_then(Value::as_array)
        .and_then(|content| content.get(entry.use_block_index))?;
    let old_tokens = block
        .get("input")
        .map(estimate_jsonish_tokens)
        .unwrap_or_default();
    let new_tokens = rough_token_count_estimation(CLEARED_TOOL_INPUT_JSON);
    if old_tokens <= new_tokens {
        return None;
    }

    let block = messages
        .get_mut(entry.use_message_index)
        .and_then(Value::as_object_mut)
        .and_then(|message| message.get_mut("content"))
        .and_then(Value::as_array_mut)
        .and_then(|content| content.get_mut(entry.use_block_index))
        .and_then(Value::as_object_mut)?;
    block.insert(
        "input".to_string(),
        Value::Object(serde_json::Map::new()),
    );
    Some((old_tokens - new_tokens).max(0))
}

fn estimate_anthropic_request_input_tokens(request_value: &Value, messages: &[Value]) -> i64 {
    let mut total = 0i64;
    total += estimate_system_tokens(request_value.get("system"));
    total += request_value
        .get("tools")
        .and_then(Value::as_array)
        .map(|tools| tools.iter().map(estimate_jsonish_tokens).sum::<i64>())
        .unwrap_or_default();
    total += messages
        .iter()
        .map(estimate_anthropic_message_tokens)
        .sum::<i64>();
    (((total.max(0)) as f64) * (4.0 / 3.0)).ceil() as i64
}

fn estimate_system_tokens(system: Option<&Value>) -> i64 {
    match system {
        Some(Value::String(text)) => rough_token_count_estimation(text),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| match item {
                Value::String(text) => rough_token_count_estimation(text),
                Value::Object(map) => map
                    .get("text")
                    .and_then(Value::as_str)
                    .map(rough_token_count_estimation)
                    .unwrap_or_else(|| estimate_jsonish_tokens(item)),
                _ => estimate_jsonish_tokens(item),
            })
            .sum(),
        Some(other) => estimate_jsonish_tokens(other),
        None => 0,
    }
}

fn estimate_anthropic_message_tokens(message: &Value) -> i64 {
    match message.get("content") {
        Some(Value::String(text)) => rough_token_count_estimation(text),
        Some(Value::Array(items)) => items.iter().map(estimate_anthropic_content_block_tokens).sum(),
        Some(other) => estimate_jsonish_tokens(other),
        None => 0,
    }
}

fn estimate_anthropic_content_block_tokens(block: &Value) -> i64 {
    match block {
        Value::String(text) => rough_token_count_estimation(text),
        Value::Object(map) => match map.get("type").and_then(Value::as_str) {
            Some("text") => map
                .get("text")
                .and_then(Value::as_str)
                .map(rough_token_count_estimation)
                .unwrap_or_default(),
            Some("thinking") => map
                .get("thinking")
                .and_then(Value::as_str)
                .map(rough_token_count_estimation)
                .unwrap_or_default(),
            Some("redacted_thinking") => map
                .get("data")
                .and_then(Value::as_str)
                .map(rough_token_count_estimation)
                .unwrap_or_default(),
            Some("tool_use") => rough_token_count_estimation(
                &format!(
                    "{}{}",
                    map.get("name").and_then(Value::as_str).unwrap_or_default(),
                    map.get("input")
                        .map(|value| serde_json::to_string(value).unwrap_or_default())
                        .unwrap_or_default()
                ),
            ),
            Some("tool_result") => estimate_tool_result_content_tokens(map.get("content")),
            Some("image") | Some("document") => ROUGH_IMAGE_TOKEN_ESTIMATE,
            Some("tool_reference") => {
                rough_token_count_estimation(&anthropic_tool_reference_text(block))
            }
            _ => estimate_jsonish_tokens(block),
        },
        _ => estimate_jsonish_tokens(block),
    }
}

fn estimate_tool_result_content_tokens(content: Option<&Value>) -> i64 {
    match content {
        Some(Value::String(text)) => rough_token_count_estimation(text),
        Some(Value::Array(items)) => items.iter().map(estimate_anthropic_content_block_tokens).sum(),
        Some(other) => estimate_jsonish_tokens(other),
        None => 0,
    }
}

fn estimate_jsonish_tokens(value: &Value) -> i64 {
    rough_token_count_estimation(&serde_json::to_string(value).unwrap_or_default())
}

fn rough_token_count_estimation(text: &str) -> i64 {
    ((text.chars().count() as f64) / 4.0).ceil() as i64
}

fn is_thinking_block(block: &Value) -> bool {
    matches!(
        block.get("type").and_then(Value::as_str),
        Some("thinking") | Some("redacted_thinking")
    )
}

fn translate_anthropic_reasoning(
    family: ClaudeCodeFamily,
    target_model: &str,
    effort_routing: &ClaudeCodeEffortRoutingSettings,
    thinking: Option<&Value>,
    output_config: Option<&Value>,
) -> Option<Value> {
    let explicit_effort = output_config
        .and_then(|value| value.get("effort"))
        .and_then(Value::as_str)
        .and_then(normalize_source_effort);
    let source_effort = match anthropic_thinking_mode(thinking) {
        Some("disabled") => None,
        Some("adaptive") => explicit_effort.or_else(|| Some("medium".to_string())),
        Some("enabled") => infer_reasoning_effort_from_thinking(thinking).or(explicit_effort),
        _ => explicit_effort.or_else(|| infer_reasoning_effort_from_thinking(thinking)),
    }?;
    let configured_effort = resolve_configured_target_effort(
        family,
        source_effort.as_str(),
        effort_routing,
    )?;
    let effective_effort = negotiate_target_reasoning_effort(
        target_model,
        configured_effort.as_str(),
        effort_routing.fallback_mode,
    )?;

    Some(serde_json::json!({ "effort": effective_effort }))
}

fn normalize_source_effort(value: &str) -> Option<String> {
    let normalized = value.trim().to_ascii_lowercase();
    (!normalized.is_empty()).then_some(normalized)
}

fn infer_reasoning_effort_from_thinking(thinking: Option<&Value>) -> Option<String> {
    let thinking = thinking?.as_object()?;
    match thinking.get("type").and_then(Value::as_str) {
        Some("adaptive") => Some("medium".to_string()),
        Some("enabled") => {
            let budget = thinking
                .get("budget_tokens")
                .and_then(Value::as_i64)
                .unwrap_or_default()
                .max(0);
            Some(
                match budget {
                    0..=2048 => "low",
                    2049..=8192 => "medium",
                    _ => "high",
                }
                .to_string(),
            )
        }
        _ => None,
    }
}

fn resolve_configured_target_effort(
    family: ClaudeCodeFamily,
    source_effort: &str,
    effort_routing: &ClaudeCodeEffortRoutingSettings,
) -> Option<String> {
    let family_routing = match family {
        ClaudeCodeFamily::Opus => &effort_routing.opus,
        ClaudeCodeFamily::Sonnet => &effort_routing.sonnet,
        ClaudeCodeFamily::Haiku => &effort_routing.haiku,
    };

    match family_routing.source_to_target.get(source_effort) {
        Some(mapped) => mapped.clone(),
        None => family_routing.default_target_effort.clone(),
    }
}

fn negotiate_target_reasoning_effort(
    target_model: &str,
    requested_effort: &str,
    fallback_mode: ClaudeCodeEffortFallbackMode,
) -> Option<String> {
    let normalized_effort = normalize_source_effort(requested_effort)?;
    let Some(supported) = supported_target_reasoning_efforts(target_model) else {
        return Some(normalized_effort);
    };
    if supported.contains(&normalized_effort.as_str()) {
        return Some(normalized_effort);
    }

    match fallback_mode {
        ClaudeCodeEffortFallbackMode::ClampDown => clamp_down_reasoning_effort(
            normalized_effort.as_str(),
            supported,
        )
        .map(str::to_string),
        ClaudeCodeEffortFallbackMode::Omit => None,
    }
}

fn supported_target_reasoning_efforts(target_model: &str) -> Option<&'static [&'static str]> {
    let normalized = target_model.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    if normalized.starts_with("gpt-5") {
        if normalized.contains("-mini") || normalized.contains("-nano") {
            return Some(&["low", "medium", "high"]);
        }
        return Some(&["low", "medium", "high", "xhigh"]);
    }
    None
}

fn clamp_down_reasoning_effort<'a>(
    requested_effort: &str,
    supported: &'a [&'a str],
) -> Option<&'a str> {
    let requested_rank = reasoning_effort_rank(requested_effort)?;
    supported
        .iter()
        .copied()
        .filter_map(|effort| Some((effort, reasoning_effort_rank(effort)?)))
        .filter(|(_, rank)| *rank <= requested_rank)
        .max_by_key(|(_, rank)| *rank)
        .map(|(effort, _)| effort)
}

fn reasoning_effort_rank(value: &str) -> Option<u8> {
    match value {
        "low" => Some(0),
        "medium" => Some(1),
        "high" => Some(2),
        "xhigh" => Some(3),
        "max" => Some(3),
        _ => None,
    }
}

fn anthropic_thinking_mode(thinking: Option<&Value>) -> Option<&str> {
    thinking?
        .as_object()?
        .get("type")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
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

fn translate_anthropic_message_items(
    role: &str,
    content: Option<&Value>,
) -> Result<Vec<Value>, Response> {
    match content {
        Some(Value::String(text)) => Ok(vec![message_input_item(
            role,
            vec![translate_text_block_for_role(role, text)],
        )]),
        Some(Value::Array(blocks)) => {
            let mut items = Vec::new();
            let mut pending_message_content = Vec::new();
            for block in blocks {
                match translate_anthropic_content_block(role, block)? {
                    AnthropicInputItem::MessageContent(content) => {
                        pending_message_content.push(content);
                    }
                    AnthropicInputItem::Standalone(item) => {
                        flush_pending_message_item(role, &mut pending_message_content, &mut items);
                        items.push(item);
                    }
                    AnthropicInputItem::Dropped => {}
                }
            }
            flush_pending_message_item(role, &mut pending_message_content, &mut items);
            Ok(items)
        }
        Some(Value::Null) | None => Ok(Vec::new()),
        Some(other) => Ok(vec![message_input_item(
            role,
            vec![translate_text_block_for_role(role, other.to_string())],
        )]),
    }
}

fn flush_pending_message_item(
    role: &str,
    pending_content: &mut Vec<Value>,
    items: &mut Vec<Value>,
) {
    if pending_content.is_empty() {
        return;
    }
    let content = std::mem::take(pending_content);
    items.push(message_input_item(role, content));
}

fn message_input_item(role: &str, content: Vec<Value>) -> Value {
    serde_json::json!({
        "role": role,
        "content": content,
    })
}

fn translate_anthropic_content_block(
    role: &str,
    block: &Value,
) -> Result<AnthropicInputItem, Response> {
    let Some(block_type) = block.get("type").and_then(Value::as_str) else {
        return Ok(AnthropicInputItem::MessageContent(
            translate_text_block_for_role(role, block.to_string()),
        ));
    };

    match block_type {
        "text" => Ok(AnthropicInputItem::MessageContent(
            translate_text_block_for_role(
                role,
                block
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            ),
        )),
        "thinking" | "redacted_thinking" => Ok(AnthropicInputItem::Dropped),
        "tool_result" => Ok(AnthropicInputItem::Standalone(serde_json::json!({
            "type": "function_call_output",
            "call_id": block.get("tool_use_id").and_then(Value::as_str).unwrap_or_default(),
            "output": translate_anthropic_tool_result_output(block.get("content")),
        }))),
        "tool_use" => Ok(AnthropicInputItem::Standalone(
            translate_assistant_tool_use_block(block),
        )),
        "tool_reference" => Ok(AnthropicInputItem::MessageContent(
            translate_text_block_for_role(role, anthropic_tool_reference_text(block)),
        )),
        "image" => Ok(AnthropicInputItem::MessageContent(
            translate_anthropic_image_block(role, block),
        )),
        _ => Ok(AnthropicInputItem::MessageContent(
            translate_text_block_for_role(role, block.to_string()),
        )),
    }
}

fn translate_anthropic_tool_result_output(content: Option<&Value>) -> Value {
    match content {
        Some(Value::Array(blocks)) => {
            let parts = blocks
                .iter()
                .filter_map(translate_anthropic_tool_result_block_to_text)
                .collect::<Vec<_>>();
            if parts.is_empty() {
                Value::String("Tool result omitted.".to_string())
            } else {
                Value::String(parts.join("\n\n"))
            }
        }
        Some(other) => other.clone(),
        None => Value::String(String::new()),
    }
}

fn translate_anthropic_tool_result_block_to_text(block: &Value) -> Option<String> {
    match block {
        Value::String(text) => (!text.trim().is_empty()).then(|| text.to_string()),
        Value::Object(_) => match block.get("type").and_then(Value::as_str) {
            Some("text") => block
                .get("text")
                .and_then(Value::as_str)
                .filter(|text| !text.trim().is_empty())
                .map(ToString::to_string),
            Some("thinking") | Some("redacted_thinking") => None,
            Some("tool_reference") => Some(anthropic_tool_reference_text(block)),
            _ => None,
        },
        _ => None,
    }
}

fn anthropic_tool_reference_text(block: &Value) -> String {
    let label = block
        .get("tool_name")
        .and_then(Value::as_str)
        .or_else(|| block.get("tool_use_id").and_then(Value::as_str))
        .or_else(|| block.get("id").and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match label {
        Some(value) => format!("Tool reference: {value}"),
        None => "Tool reference".to_string(),
    }
}

fn translate_text_block_for_role(role: &str, text: impl Into<String>) -> Value {
    let text = text.into();
    if role.eq_ignore_ascii_case("assistant") {
        serde_json::json!({
            "type": "output_text",
            "text": text,
        })
    } else {
        serde_json::json!({
            "type": "input_text",
            "text": text,
        })
    }
}

fn translate_assistant_tool_use_block(block: &Value) -> Value {
    let input = block
        .get("input")
        .cloned()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    let arguments = serde_json::to_string(&input).unwrap_or_else(|_| "{}".to_string());
    serde_json::json!({
        "type": "function_call",
        "call_id": block
            .get("id")
            .or_else(|| block.get("tool_use_id"))
            .and_then(Value::as_str)
            .unwrap_or_default(),
        "name": block.get("name").and_then(Value::as_str).unwrap_or_default(),
        "arguments": arguments,
    })
}

fn translate_anthropic_image_block(role: &str, block: &Value) -> Value {
    let Some(source) = block.get("source").and_then(Value::as_object) else {
        return translate_text_block_for_role(role, block.to_string());
    };

    match source.get("type").and_then(Value::as_str) {
        Some("base64") => {
            let media_type = source
                .get("media_type")
                .and_then(Value::as_str)
                .unwrap_or("application/octet-stream");
            let data = source
                .get("data")
                .and_then(Value::as_str)
                .unwrap_or_default();
            serde_json::json!({
                "type": "input_image",
                "image_url": format!("data:{media_type};base64,{data}"),
            })
        }
        Some("url") => serde_json::json!({
            "type": "input_image",
            "image_url": source.get("url").and_then(Value::as_str).unwrap_or_default(),
        }),
        _ => translate_text_block_for_role(role, block.to_string()),
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
    let parameters = tool
        .get("input_schema")
        .map(sanitize_openai_tool_schema)
        .unwrap_or_else(default_openai_tool_schema);
    mapped.insert("parameters".to_string(), parameters);
    // Keep only the OpenAI-compatible function tool surface here.
    // Claude Code-specific fields like defer_loading/eager_input_streaming
    // are local hints and can cause downstream schema rejections.
    Value::Object(mapped)
}

fn default_openai_tool_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {}
    })
}

fn sanitize_openai_tool_schema(schema: &Value) -> Value {
    match schema {
        Value::Object(map) => {
            let mut sanitized = serde_json::Map::new();
            for (key, value) in map {
                let next_value = match key.as_str() {
                    "properties" => value
                        .as_object()
                        .map(|properties| {
                            Value::Object(
                                properties
                                    .iter()
                                    .map(|(name, schema)| {
                                        (name.clone(), sanitize_openai_tool_schema(schema))
                                    })
                                    .collect(),
                            )
                        })
                        .unwrap_or_else(|| Value::Object(serde_json::Map::new())),
                    "items" | "additionalProperties" | "not" => sanitize_openai_tool_schema(value),
                    "allOf" | "anyOf" | "oneOf" | "prefixItems" => value
                        .as_array()
                        .map(|entries| {
                            Value::Array(
                                entries
                                    .iter()
                                    .map(sanitize_openai_tool_schema)
                                    .collect(),
                            )
                        })
                        .unwrap_or_else(|| value.clone()),
                    _ => value.clone(),
                };
                sanitized.insert(key.clone(), next_value);
            }

            if sanitized.get("type").and_then(Value::as_str) == Some("object")
                && !matches!(sanitized.get("properties"), Some(Value::Object(_)))
            {
                sanitized.insert(
                    "properties".to_string(),
                    Value::Object(serde_json::Map::new()),
                );
            }

            Value::Object(sanitized)
        }
        Value::Array(items) => Value::Array(items.iter().map(sanitize_openai_tool_schema).collect()),
        other => other.clone(),
    }
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
        Value::String(raw) => {
            serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_string()))
        }
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
    tool_use_blocks: std::collections::BTreeMap<usize, ToolUseStreamState>,
    terminal_event_seen: bool,
    context_management: Option<Value>,
}

#[derive(Debug, Clone, Default)]
struct ToolUseStreamState {
    id: String,
    name: String,
    started: bool,
    delta_emitted: bool,
    stopped: bool,
}

impl AnthropicSseTranslator {
    fn new(requested_model: String, context_management: Option<Value>) -> Self {
        Self {
            requested_model,
            response_id: None,
            message_started: false,
            text_block_started: false,
            emitted_content_blocks: 0,
            tool_use_blocks: std::collections::BTreeMap::new(),
            terminal_event_seen: false,
            context_management,
        }
    }

    fn translate_frame(&mut self, payload: &[u8]) -> Vec<Bytes> {
        if self.terminal_event_seen {
            return Vec::new();
        }
        if payload == b"[DONE]" {
            return Vec::new();
        }
        let Ok(value) = serde_json::from_slice::<Value>(payload) else {
            self.terminal_event_seen = true;
            return vec![anthropic_stream_error_frame(
                None,
                None,
                "upstream stream emitted invalid JSON",
            )];
        };
        let event_type = value
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match event_type {
            "response.created" => self.handle_response_created(&value),
            "response.output_text.delta" => self.handle_output_text_delta(&value),
            "response.output_item.added" => self.handle_response_output_item_added(&value),
            "response.function_call_arguments.delta" => {
                self.handle_function_call_arguments_delta(&value)
            }
            "response.function_call_arguments.done" => {
                self.handle_function_call_arguments_done(&value)
            }
            "response.output_item.done" => self.handle_response_output_item_done(&value),
            "response.failed" | "response.error" | "error" => self.handle_response_failed(&value),
            "response.completed" => self.handle_response_completed(&value),
            _ => Vec::new(),
        }
    }

    fn finish_on_upstream_eof(&mut self) -> Option<Bytes> {
        if self.terminal_event_seen {
            return None;
        }
        self.terminal_event_seen = true;
        Some(anthropic_stream_error_frame(
            None,
            None,
            "upstream response stream ended before a terminal event",
        ))
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

    fn handle_response_output_item_added(&mut self, value: &Value) -> Vec<Bytes> {
        let Some(item) = value.get("item") else {
            return Vec::new();
        };
        if item.get("type").and_then(Value::as_str) != Some("function_call") {
            return Vec::new();
        }
        let index = value
            .get("output_index")
            .and_then(Value::as_u64)
            .unwrap_or(self.emitted_content_blocks as u64) as usize;
        self.ensure_tool_use_block_started(index, item, value.get("response").unwrap_or(value))
    }

    fn handle_function_call_arguments_delta(&mut self, value: &Value) -> Vec<Bytes> {
        let index = value
            .get("output_index")
            .and_then(Value::as_u64)
            .unwrap_or(self.emitted_content_blocks as u64) as usize;
        let mut frames = self.ensure_tool_use_block_started(
            index,
            value.get("item").unwrap_or(&Value::Null),
            value.get("response").unwrap_or(value),
        );
        let Some(delta) = value.get("delta").and_then(Value::as_str) else {
            return frames;
        };
        frames.push(build_sse_frame(
            Some("content_block_delta"),
            &serde_json::json!({
                "type": "content_block_delta",
                "index": index,
                "delta": {
                    "type": "input_json_delta",
                    "partial_json": delta,
                }
            }),
        ));
        if let Some(state) = self.tool_use_blocks.get_mut(&index) {
            state.delta_emitted = true;
        }
        frames
    }

    fn handle_function_call_arguments_done(&mut self, value: &Value) -> Vec<Bytes> {
        let index = value
            .get("output_index")
            .and_then(Value::as_u64)
            .unwrap_or(self.emitted_content_blocks as u64) as usize;
        self.close_tool_use_block(
            index,
            value.get("arguments").and_then(Value::as_str),
            value.get("item"),
            value.get("response").unwrap_or(value),
        )
    }

    fn handle_response_output_item_done(&mut self, value: &Value) -> Vec<Bytes> {
        let Some(item) = value.get("item") else {
            return Vec::new();
        };
        if item.get("type").and_then(Value::as_str) != Some("function_call") {
            return Vec::new();
        }
        let index = value
            .get("output_index")
            .and_then(Value::as_u64)
            .unwrap_or(self.emitted_content_blocks as u64) as usize;
        self.close_tool_use_block(
            index,
            item.get("arguments").and_then(Value::as_str),
            Some(item),
            value.get("response").unwrap_or(value),
        )
    }

    fn handle_response_failed(&mut self, value: &Value) -> Vec<Bytes> {
        self.terminal_event_seen = true;
        let (error_code, message) = upstream_stream_error_details(value);
        vec![anthropic_stream_error_frame(
            None,
            error_code.as_deref(),
            message.as_deref().unwrap_or("request failed"),
        )]
    }

    fn handle_response_completed(&mut self, value: &Value) -> Vec<Bytes> {
        self.terminal_event_seen = true;
        let response = value.get("response").unwrap_or(value);
        self.response_id = response
            .get("id")
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .or_else(|| self.response_id.clone());

        let translated =
            translate_responses_json_to_anthropic_message(response, self.requested_model.as_str());
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
                self.maybe_emit_or_close_content_block(index, block, &mut frames);
            }
        } else {
            for (index, block) in content_blocks.iter().enumerate() {
                self.maybe_emit_or_close_content_block(index, block, &mut frames);
            }
        }

        let mut message_delta = serde_json::json!({
            "type": "message_delta",
            "delta": {
                "stop_reason": translated.get("stop_reason").cloned().unwrap_or(Value::Null),
                "stop_sequence": Value::Null,
            },
            "usage": translated.get("usage").cloned().unwrap_or_else(|| anthropic_usage_value(response)),
        });
        if let Some(context_management) = self.context_management.clone() {
            if let Some(object) = message_delta.as_object_mut() {
                object.insert("context_management".to_string(), context_management);
            }
        }
        frames.push(build_sse_frame(Some("message_delta"), &message_delta));
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

    fn maybe_emit_or_close_content_block(
        &mut self,
        index: usize,
        block: &Value,
        frames: &mut Vec<Bytes>,
    ) {
        if block.get("type").and_then(Value::as_str) == Some("tool_use") {
            let input = block
                .get("input")
                .map(|value| serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string()));
            if self.tool_use_blocks.contains_key(&index) {
                let final_input = input.as_deref();
                frames.extend(self.close_tool_use_block(
                    index,
                    final_input,
                    Some(block),
                    &Value::Null,
                ));
                return;
            }
        }
        self.emit_full_content_block(index, block, frames);
    }

    fn ensure_tool_use_block_started(
        &mut self,
        index: usize,
        item: &Value,
        response: &Value,
    ) -> Vec<Bytes> {
        let mut frames = self.ensure_message_start(response);
        let state = self.tool_use_blocks.entry(index).or_default();
        if state.id.is_empty() {
            state.id = item
                .get("call_id")
                .or_else(|| item.get("id"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
        }
        if state.name.is_empty() {
            state.name = item
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
        }
        if state.started || state.stopped {
            return frames;
        }
        frames.push(build_sse_frame(
            Some("content_block_start"),
            &serde_json::json!({
                "type": "content_block_start",
                "index": index,
                "content_block": {
                    "type": "tool_use",
                    "id": state.id.clone(),
                    "name": state.name.clone(),
                    "input": {},
                }
            }),
        ));
        state.started = true;
        self.emitted_content_blocks = self.emitted_content_blocks.max(index + 1);
        frames
    }

    fn close_tool_use_block(
        &mut self,
        index: usize,
        final_arguments: Option<&str>,
        item: Option<&Value>,
        response: &Value,
    ) -> Vec<Bytes> {
        let mut frames =
            self.ensure_tool_use_block_started(index, item.unwrap_or(&Value::Null), response);
        let state = self.tool_use_blocks.entry(index).or_default();
        if !state.delta_emitted {
            if let Some(arguments) = final_arguments.filter(|value| !value.is_empty()) {
                frames.push(build_sse_frame(
                    Some("content_block_delta"),
                    &serde_json::json!({
                        "type": "content_block_delta",
                        "index": index,
                        "delta": {
                            "type": "input_json_delta",
                            "partial_json": arguments,
                        }
                    }),
                ));
                state.delta_emitted = true;
            }
        }
        if !state.stopped {
            frames.push(build_sse_frame(
                Some("content_block_stop"),
                &serde_json::json!({
                    "type": "content_block_stop",
                    "index": index,
                }),
            ));
            state.stopped = true;
        }
        frames
    }

    fn emit_full_content_block(&mut self, index: usize, block: &Value, frames: &mut Vec<Bytes>) {
        let block_type = block
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
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
                    "tool_use" => serde_json::json!({
                        "type": "tool_use",
                        "id": block.get("id").cloned().unwrap_or_else(|| Value::String(String::new())),
                        "name": block.get("name").cloned().unwrap_or_else(|| Value::String(String::new())),
                        "input": {},
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
        } else if block_type == "tool_use" {
            let input = block
                .get("input")
                .cloned()
                .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
            let partial_json = serde_json::to_string(&input).unwrap_or_else(|_| "{}".to_string());
            frames.push(build_sse_frame(
                Some("content_block_delta"),
                &serde_json::json!({
                    "type": "content_block_delta",
                    "index": index,
                    "delta": {
                        "type": "input_json_delta",
                        "partial_json": partial_json,
                    }
                }),
            ));
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

fn upstream_stream_error_details(value: &Value) -> (Option<String>, Option<String>) {
    let error = value.get("error").or_else(|| {
        value
            .get("response")
            .and_then(|response| response.get("error"))
    });
    let code = error
        .and_then(|error| error.get("code"))
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let message = error
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .or_else(|| value.get("message").and_then(Value::as_str))
        .map(ToString::to_string);
    (code, message)
}
