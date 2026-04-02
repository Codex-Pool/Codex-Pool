# Claude Code Routing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add proxy-mode compatible Claude Code support with Anthropic `/v1/messages`, `/v1/messages/count_tokens`, and an admin UI that maps `Opus`, `Sonnet`, and `Haiku` to exactly one internal pool model each.

**Architecture:** The control-plane stores dedicated Claude Code routing settings and delivers them through the snapshot contract. The data-plane resolves Anthropic model families to configured internal models, rewrites Anthropic Messages requests into the existing canonical proxy flow, and translates Responses-style JSON/SSE back into Anthropic Messages semantics. The frontend adds a constrained Claude Code panel inside the existing model routing page.

**Tech Stack:** Rust (`axum`, `serde`, existing control-plane/data-plane crates), React + TypeScript + React Query, existing admin model catalog and routing APIs.

---

### Task 1: Shared Contract And Control-Plane Settings

**Files:**
- Modify: `/Users/wangnov/Codex-Pool/crates/codex-pool-core/src/model.rs`
- Modify: `/Users/wangnov/Codex-Pool/crates/codex-pool-core/src/api.rs`
- Modify: `/Users/wangnov/Codex-Pool/crates/codex-pool-core/src/snapshot.rs`
- Modify: `/Users/wangnov/Codex-Pool/services/control-plane/src/store.rs`
- Modify: `/Users/wangnov/Codex-Pool/services/control-plane/src/store/in_memory_core.rs`
- Modify: `/Users/wangnov/Codex-Pool/services/control-plane/src/store/postgres/helpers_trait.rs`
- Modify: `/Users/wangnov/Codex-Pool/services/control-plane/src/store/postgres/impl_oauth_snapshot/model_routing.rs`
- Modify: `/Users/wangnov/Codex-Pool/services/control-plane/src/app/core_handlers/model_routing_admin.rs`
- Test: `/Users/wangnov/Codex-Pool/services/control-plane/tests/api/model_routing_admin.rs`

- [ ] Add a shared `ClaudeCodeRoutingSettings` contract with three optional family target models.
- [ ] Add request/response structs for admin get/update APIs.
- [ ] Extend snapshot payloads so the control-plane can publish Claude Code routing settings to the data-plane.
- [ ] Add store trait methods for read/update Claude Code routing settings.
- [ ] Implement in-memory store support.
- [ ] Implement Postgres persistence and default loading behavior.
- [ ] Add admin handlers and routes for `GET`/`PUT /api/v1/admin/model-routing/claude-code`.
- [ ] Add control-plane tests that fail before implementation and pass after implementation.

### Task 2: Data-Plane Anthropic Compatibility

**Files:**
- Modify: `/Users/wangnov/Codex-Pool/services/data-plane/src/auth.rs`
- Modify: `/Users/wangnov/Codex-Pool/services/data-plane/src/app/bootstrap.rs`
- Modify: `/Users/wangnov/Codex-Pool/services/data-plane/src/snapshot.rs`
- Modify: `/Users/wangnov/Codex-Pool/services/data-plane/src/router.rs`
- Modify: `/Users/wangnov/Codex-Pool/services/data-plane/src/proxy.rs`
- Modify: `/Users/wangnov/Codex-Pool/services/data-plane/src/proxy/entry.rs`
- Modify: `/Users/wangnov/Codex-Pool/services/data-plane/src/proxy/request_utils.rs`
- Modify: `/Users/wangnov/Codex-Pool/services/data-plane/src/proxy/responses_api.rs`
- Create: `/Users/wangnov/Codex-Pool/services/data-plane/src/proxy/anthropic_api.rs`
- Create: `/Users/wangnov/Codex-Pool/services/data-plane/src/proxy/anthropic_translator.rs`
- Test: `/Users/wangnov/Codex-Pool/services/data-plane/tests/auth_middleware.rs`
- Test: `/Users/wangnov/Codex-Pool/services/data-plane/tests/compatibility.rs`
- Test: `/Users/wangnov/Codex-Pool/services/data-plane/tests/compat_contract.rs`

- [ ] Add failing auth tests for `x-api-key` support on Anthropic routes.
- [ ] Add failing compatibility tests for `/v1/messages` non-stream, streaming SSE, unmapped family errors, and `/v1/messages/count_tokens`.
- [ ] Extend auth extraction so Anthropic routes accept `x-api-key` in addition to bearer auth.
- [ ] Load Claude Code routing settings from snapshots into app state/router state.
- [ ] Implement Anthropic family normalization and target-model resolution.
- [ ] Implement Anthropic request translation into the existing canonical proxy flow.
- [ ] Implement Anthropic JSON response translation.
- [ ] Implement Anthropic SSE translation with correct event ordering and final `message_delta`.
- [ ] Implement `/v1/messages/count_tokens` translation using existing local token estimation.
- [ ] Wire the new handlers into bootstrap routing.
- [ ] Run focused Rust tests and fix regressions.

### Task 3: Admin Frontend Claude Code Panel

**Files:**
- Modify: `/Users/wangnov/Codex-Pool/frontend/src/api/modelRouting.ts`
- Modify: `/Users/wangnov/Codex-Pool/frontend/src/pages/ModelRouting.tsx`
- Modify: `/Users/wangnov/Codex-Pool/frontend/src/locales/en.ts`
- Modify: `/Users/wangnov/Codex-Pool/frontend/src/locales/zh-CN.ts`
- Test: `/Users/wangnov/Codex-Pool/frontend/src/api/parity.test.ts`
- Test: `/Users/wangnov/Codex-Pool/frontend/src/pages/model-routing-archetype.test.ts`
- Test: `/Users/wangnov/Codex-Pool/frontend/src/pages/page-title-docking-adoption.test.ts`

- [ ] Add failing frontend API parity coverage for Claude Code routing methods.
- [ ] Add failing page-level regression coverage for a Claude Code panel on the model routing page.
- [ ] Extend the admin API module with typed get/update Claude Code routing methods.
- [ ] Add a dedicated `Claude Code` panel using existing model selector primitives and antigravity page patterns.
- [ ] Load model catalog data and bind exactly one selected target model per family.
- [ ] Add save/reset UX and notification handling.
- [ ] Add English and Chinese copy for the new panel and error states.
- [ ] Run focused frontend tests and fix regressions.

### Task 4: Integration And Finish

**Files:**
- Modify: `/Users/wangnov/Codex-Pool/services/control-plane/tests/contracts_surface.rs`
- Modify: `/Users/wangnov/Codex-Pool/services/data-plane/tests/integration.rs`
- Modify: `/Users/wangnov/Codex-Pool/frontend/src/backend-parity-regression.test.ts`

- [ ] Add contract-level coverage proving the new control-plane snapshot field reaches the data-plane.
- [ ] Add an end-to-end compatibility test for a Claude Code style request flowing through mapped family resolution.
- [ ] Add a frontend/backend parity regression covering the new admin surface.
- [ ] Run targeted test suites for control-plane, data-plane, and frontend.
- [ ] Document any residual follow-up work discovered during integration.
