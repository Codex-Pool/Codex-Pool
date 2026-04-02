# Claude Code Routing Design

## Goal

Add proxy-mode compatible Anthropic Messages support so Claude Code can work stably against Codex-Pool, while giving operators a dedicated admin UI to map the three Claude Code families (`Opus`, `Sonnet`, `Haiku`) onto exactly one internal pool model each.

## Scope

This design covers:

- Anthropic-compatible `/v1/messages`
- Anthropic-compatible `/v1/messages/count_tokens`
- Claude Code family-to-model mapping in the admin frontend
- Control-plane storage and snapshot delivery for Claude Code routing settings
- Data-plane request rewriting and SSE/JSON response translation

This design does not try to emulate Anthropic first-party host behavior. It targets Claude Code running through `ANTHROPIC_BASE_URL=<proxy>`.

## Product Behavior

### Admin UI

The existing admin model routing page gets a dedicated `Claude Code` panel with three fixed slots:

- `Opus`
- `Sonnet`
- `Haiku`

Each slot selects exactly one internal target model from the existing model catalog. There is no multi-model fallback chain inside this panel.

If a family is unmapped, Claude Code requests for that family fail with a clear Anthropic-style error. There is no hidden fallback.

### Runtime Behavior

When the data-plane receives an Anthropic Messages request from Claude Code:

1. Determine the requested Claude family from the Anthropic model string.
2. Resolve the configured target internal model for that family.
3. Rewrite the request to the internal model before normal routing.
4. Route to upstream accounts using existing routing infrastructure.
5. Translate upstream Responses-style JSON/SSE into Anthropic Messages JSON/SSE.

## Architecture

### Frontend

The frontend reuses the existing model routing page and model selector primitives. A new `Claude Code` panel reads and writes dedicated Claude Code routing settings through admin APIs. The panel is a constrained operator view, not a replacement for the general routing editor.

### Control Plane

The control-plane stores a dedicated `ClaudeCodeRoutingSettings` object. That object is included in the control-plane snapshot payload consumed by the data-plane.

This keeps Claude family mapping explicit instead of trying to overload generic routing policies with model substitution logic.

### Data Plane

The data-plane adds Anthropic-compatible handlers alongside the existing OpenAI/Codex handlers.

The new path is:

- parse Anthropic request
- resolve Claude family mapping
- translate to canonical Responses request
- execute existing upstream proxy flow
- translate upstream response back to Anthropic shape

## Data Model

### Claude Code routing settings

The control-plane and data-plane share a small contract:

- `enabled: bool`
- `opus_target_model: Option<String>`
- `sonnet_target_model: Option<String>`
- `haiku_target_model: Option<String>`
- `updated_at: DateTime<Utc>`

Family resolution uses normalized Anthropic model names:

- `claude-opus-*` -> `Opus`
- `claude-sonnet-*` -> `Sonnet`
- `claude-haiku-*` -> `Haiku`

Unknown Anthropic families are rejected.

## API Surface

### New admin APIs

Add admin endpoints for Claude Code routing settings:

- `GET /api/v1/admin/model-routing/claude-code`
- `PUT /api/v1/admin/model-routing/claude-code`

These endpoints are separate from general model routing settings because they control model substitution rather than planner behavior.

### New data-plane APIs

Add:

- `POST /v1/messages`
- `POST /v1/messages/count_tokens`

The handlers must accept `x-api-key` and `Authorization: Bearer ...`, plus standard Anthropic headers like `anthropic-version` and `anthropic-beta`.

## Anthropic Compatibility

### Request compatibility

Support the Claude Code proxy-mode request surface:

- `model`
- `messages`
- `system`
- `tools`
- `tool_choice`
- `metadata`
- `thinking`
- `stream`
- `max_tokens`
- `temperature`
- `context_management`
- `output_config`
- `speed`
- tool schema extras such as `strict`, `defer_loading`, `eager_input_streaming`, and `cache_control`

Unsupported first-party-only behavior should degrade safely rather than 400 when possible.

### Streaming compatibility

Emit Anthropic raw message stream events in this order:

1. `message_start`
2. zero or more `content_block_start`
3. zero or more `content_block_delta`
4. matching `content_block_stop`
5. `message_delta`
6. `message_stop`

`message_delta` must carry final `usage` and `stop_reason` because Claude Code depends on that ordering.

### Tool compatibility

Tool blocks must preserve Anthropic semantics:

- `tool_use.id` is stable and unique
- `tool_use.input` is emitted as string/object in valid Anthropic form
- `tool_result.tool_use_id` matches the originating tool use

## Error Handling

Return Anthropic-style errors for:

- missing Claude Code family mapping
- mapped target model unavailable or unroutable
- unsupported Anthropic family
- malformed Anthropic Messages payload

These should be explicit operator-facing failures, not silent fallback.

## Testing

### Control-plane

- Admin API tests for get/update Claude Code routing settings
- Snapshot/export tests proving settings flow into data-plane payloads

### Data-plane

- Auth tests covering `x-api-key`
- `/v1/messages` non-stream response translation tests
- `/v1/messages` SSE translation tests
- `/v1/messages/count_tokens` translation tests
- unmapped family error tests

### Frontend

- API surface parity test for new Claude Code routing methods
- Model routing page regression test for the new Claude Code panel
- interaction tests for selecting and saving family mappings

## Non-Goals

- Anthropic first-party host emulation
- `/v1/models` parity work
- Anthropic bootstrap endpoints
- multi-target fallback chains inside Claude Code family settings
