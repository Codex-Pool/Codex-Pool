# Claude Code Compat Gap Closure

## Goal

Close the highest-impact compatibility gaps between Codex-Pool's Anthropic-compatible
surface and Claude Code proxy-mode, using Claude Code source and CLIProxyAPI as the
reference for real-world behavior.

## Phase 1: Stability Gaps

### 1. Anthropic auth error envelopes

- Scope: `/v1/messages`, `/v1/messages/count_tokens`
- Outcome: auth failures return Anthropic-shaped errors instead of generic control-plane envelopes
- Acceptance:
  - `x-api-key` and `Authorization: Bearer` both still work
  - missing/invalid/disabled key paths return Anthropic `type=error`

### 2. Streaming failure completeness

- Scope: Anthropic SSE translation and stream bridging
- Outcome: upstream stream failures no longer silently truncate
- Acceptance:
  - `response.failed` or equivalent upstream failure produces a deterministic Anthropic failure path
  - broken SSE frame delivery is covered by tests

### 3. Tool output-side compatibility

- Scope: Responses -> Anthropic translation, streaming and non-streaming
- Outcome: tool-calling turns are emitted in a shape Claude Code consumes reliably
- Acceptance:
  - non-stream `function_call` becomes Anthropic `tool_use`
  - stream tool calls emit `content_block_start` + `input_json_delta` + `content_block_stop`
  - `stop_reason=tool_use` is preserved

### 4. Regression coverage

- Scope: integration tests for Anthropic compatibility
- Outcome: critical happy/failure paths are locked down
- Acceptance:
  - upstream 401/429/5xx error mapping covered
  - split-frame SSE covered
  - tool output-side translation covered
  - Opus/Sonnet/Haiku family matrix expanded

## Phase 2: Protocol Parity Enhancements

### 5. tool_reference and thinking safety gaps

- Scope: Anthropic Messages request translation
- Outcome: Claude Code specific blocks no longer degrade into stray JSON text
- Acceptance:
  - `tool_reference` blocks are handled explicitly instead of stringified
  - empty `tool_result.content[]` after filtering gets a short placeholder fallback
  - `thinking` maps with explicit rules instead of a single coarse bucket
  - `thinking` / `redacted_thinking` history blocks are safely dropped rather than stringified

### 6. context_management and output_config

- Scope: request translation from Anthropic Messages to Responses
- Outcome: reduce semantic loss for real Claude Code requests
- Candidates:
  - preserve or emulate safe subsets of `context_management`
  - carry `output_config.format` only when there is a low-risk equivalent
  - keep `output_config.task_budget` accepted but ignored unless a safe mapping appears

### 7. non-first-party host edges

- Scope: Claude Code optional paths and proxy-specific quirks
- Outcome: narrow the gap with CLIProxyAPI where evidence shows real value
- Candidates:
  - optional non-first-party request headers/fingerprint shims
  - cache-related compatibility edges only if they affect real Claude Code behavior

## Out of Scope For Phase 1

- Full first-party Anthropic host emulation
- Bootstrap/OAuth endpoints outside `ANTHROPIC_BASE_URL`
- Anthropic `/v1/models` parity unless a concrete Claude Code path starts depending on it

## Execution Order

1. Fix Anthropic auth error envelopes
2. Fix stream/tool output compatibility
3. Expand regression matrix
4. Fix `tool_reference` / `thinking` safety gaps
5. Reassess remaining Claude Code gaps with live runs
6. Start deeper Phase 2 parity only after the earlier items are green

## Explicit Deferrals

- Do not emulate Claude Code microcompact or `context_management` history rewriting in this batch
- Do not map `output_config.task_budget` to unrelated Responses fields
- Do not add first-party Anthropic fingerprint/attestation shims on the current Responses route
