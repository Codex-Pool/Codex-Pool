# WS / Billing / Failover Hardening Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Eliminate billing idempotency leaks, make websocket logical-request cleanup reliable, and strengthen continuation-aware routing without breaking existing proxy compatibility.

**Architecture:** Separate tracing IDs from billing IDs, treat websocket logical requests as explicit stateful units, and promote `previous_response_id` into a continuation/routing anchor rather than a billing key. Implement the hardening in two phases: first stop high-risk leaks, then improve continuation-aware routing and attribution.

**Tech Stack:** Rust, Axum, Tokio, reqwest, tokio-tungstenite, SQLx, Postgres, existing `data-plane` / `control-plane` tests.

---

## Progress

- [x] Task 1: Define ID Semantics and Billing Key Boundaries
- [x] Task 2: Fix WebSocket Close / Failed / Incomplete Cleanup
- [x] Task 3: Make WebSocket Replay / Rebind Billing Safe
- [x] Task 4: Improve Continuation-Aware Routing and Weak-ID Handling
- [x] Task 5: Fix Stream Failover Attribution and Non-Stream Settle Semantics
- [x] Task 6: Full Verification and Docs Sync

### Task 1: Define ID Semantics and Billing Key Boundaries

**Files:**
- Modify: `services/data-plane/src/proxy/billing_stream.rs`
- Modify: `services/data-plane/src/proxy/entry.rs`
- Modify: `services/data-plane/src/proxy/ws_utils.rs`
- Modify: `services/control-plane/src/tenant/types_and_runtime.rs`
- Modify: `services/control-plane/src/tenant/billing_reconcile.rs`
- Test: `services/data-plane/tests/billing_compact_pricing.rs`
- Test: `services/control-plane/tests/api/base_and_core_part2.rs`

**Step 1: Write failing tests for reused client request ids**

- Add a data-plane regression where two distinct billable requests reuse the same client `x-request-id`, and assert both requests still produce distinct internal billing operations.
- Add a control-plane regression where an already `released` authorization must not be returned as the active authorization for a new logical request.

**Step 2: Run tests to verify failure**

Run: `cargo test -p data-plane billing_compact_pricing -- --nocapture`

Run: `cargo test -p control-plane base_and_core_part2 -- --nocapture`

Expected: existing code either reuses old authorization or fails new assertions.

**Step 3: Implement logical billing key separation**

- Add a server-generated logical billing key for every billable request.
- Keep `x-request-id` as trace/correlation only.
- Ensure websocket logical requests also generate per-message billing keys instead of falling back to handshake-level `x-request-id`.

**Step 4: Tighten control-plane authorize semantics**

- Update authorize lookup logic so only non-terminal authorizations are reusable.
- Ensure `released` / `captured` rows cannot silently satisfy a fresh logical request.

**Step 5: Re-run targeted tests**

Run: `cargo test -p data-plane billing_compact_pricing -- --nocapture`

Run: `cargo test -p control-plane base_and_core_part2 -- --nocapture`

Expected: PASS.

**Step 6: Commit**

```bash
git add services/data-plane/src/proxy/billing_stream.rs services/data-plane/src/proxy/entry.rs services/data-plane/src/proxy/ws_utils.rs services/control-plane/src/tenant/types_and_runtime.rs services/control-plane/src/tenant/billing_reconcile.rs services/data-plane/tests/billing_compact_pricing.rs services/control-plane/tests/api/base_and_core_part2.rs
git commit -m "fix(core): harden billing idempotency keys" -m "Separate billing logical keys from client request ids and block terminal authorization reuse."
```

### Task 2: Fix WebSocket Close / Failed / Incomplete Cleanup

**Files:**
- Modify: `services/data-plane/src/proxy/ws_utils.rs`
- Test: `services/data-plane/tests/compatibility_ws.rs`

**Step 1: Write failing websocket cleanup tests**

- Add a test where upstream sends `Close` before a logical request completes, and assert pending holds are released.
- Add a test where upstream emits `response.failed`, and assert the request is classified as failed/released without waiting for connection close.

**Step 2: Run tests to verify failure**

Run: `cargo test -p data-plane compatibility_ws -- --nocapture`

Expected: new tests fail on current early-return / missing event handling.

**Step 3: Refactor websocket exit path into unified cleanup**

- Remove early-return behavior that bypasses final cleanup for lingering billing sessions.
- Explicitly classify `response.failed` as a terminal failed logical response.
- Guarantee every exit path drains pending billing actions and releases unfinished sessions.

**Step 4: Re-run websocket tests**

Run: `cargo test -p data-plane compatibility_ws -- --nocapture`

Expected: PASS.

**Step 5: Commit**

```bash
git add services/data-plane/src/proxy/ws_utils.rs services/data-plane/tests/compatibility_ws.rs
git commit -m "fix(data-plane): unify websocket terminal cleanup" -m "Release pending billing holds on close and handle response.failed as an explicit terminal path."
```

### Task 3: Make WebSocket Replay / Rebind Billing Safe

**Files:**
- Modify: `services/data-plane/src/proxy/ws_utils.rs`
- Modify: `services/control-plane/src/tenant/billing_reconcile.rs`
- Test: `services/data-plane/tests/compatibility_ws.rs`

**Step 1: Write failing replay billing regression**

- Add a test for same-request replay on a new account before any output.
- Assert the replayed request receives a fresh valid authorization and capture is not satisfied by a previously released authorization.

**Step 2: Run test to verify failure**

Run: `cargo test -p data-plane compatibility_ws ws_session_retries_same_request_on_new_account_before_any_output -- --nocapture`

Expected: new assertion fails against current reuse path.

**Step 3: Implement replay-safe authorization behavior**

- Ensure replay/rebind issues a fresh logical billing operation.
- Preserve traceability to the original request via metadata rather than raw authorization reuse.

**Step 4: Re-run targeted replay test**

Run: `cargo test -p data-plane compatibility_ws ws_session_retries_same_request_on_new_account_before_any_output -- --nocapture`

Expected: PASS.

**Step 5: Commit**

```bash
git add services/data-plane/src/proxy/ws_utils.rs services/control-plane/src/tenant/billing_reconcile.rs services/data-plane/tests/compatibility_ws.rs
git commit -m "fix(core): make websocket replay billing-safe" -m "Issue fresh billing operations for websocket rebinds instead of reusing released authorizations."
```

### Task 4: Improve Continuation-Aware Routing and Weak-ID Handling

**Files:**
- Modify: `services/data-plane/src/proxy/request_utils.rs`
- Modify: `services/data-plane/src/proxy/ws_utils.rs`
- Test: `services/data-plane/tests/compatibility_ws.rs`
- Test: `services/data-plane/tests/stream_consistency.rs`

**Step 1: Write failing continuation and weak-ID tests**

- Add a websocket test for multiple logical requests without message-level `request_id`, asserting they do not collapse onto one billing key.
- Add a test proving `previous_response_id` is preferred as a continuation anchor for routing decisions when available.

**Step 2: Run tests to verify failure**

Run: `cargo test -p data-plane compatibility_ws stream_consistency -- --nocapture`

Expected: current weak-ID heuristics or sticky behavior fail the assertions.

**Step 3: Implement continuation-aware improvements**

- Promote `previous_response_id` from weak sticky hint into explicit continuation routing input.
- Reduce heuristic fallback in websocket tracker where possible.
- If identifiers are absent, generate deterministic per-connection sequence-based logical IDs.

**Step 4: Re-run targeted tests**

Run: `cargo test -p data-plane compatibility_ws stream_consistency -- --nocapture`

Expected: PASS.

**Step 5: Commit**

```bash
git add services/data-plane/src/proxy/request_utils.rs services/data-plane/src/proxy/ws_utils.rs services/data-plane/tests/compatibility_ws.rs services/data-plane/tests/stream_consistency.rs
git commit -m "feat(data-plane): strengthen continuation-aware websocket routing" -m "Use previous_response_id and generated logical ids to reduce weak-ID websocket misbinding."
```

### Task 5: Fix Stream Failover Attribution and Non-Stream Settle Semantics

**Files:**
- Modify: `services/data-plane/src/proxy/entry.rs`
- Modify: `services/data-plane/src/proxy/billing_stream.rs`
- Test: `services/data-plane/tests/stream_consistency.rs`
- Test: `services/control-plane/tests/request_logs_api.rs`

**Step 1: Write failing attribution and settle tests**

- Add a stream failover regression asserting final request log / billing event account attribution follows the actual capture account.
- Add a non-stream regression asserting an upstream-success response is not spuriously converted into a client-visible hard failure solely because post-response settle failed.

**Step 2: Run tests to verify failure**

Run: `cargo test -p data-plane stream_consistency -- --nocapture`

Run: `cargo test -p control-plane request_logs_api -- --nocapture`

Expected: current behavior fails new assertions.

**Step 3: Implement attribution and settle handling changes**

- Update stream billing finalization to record the actual account that captured the request.
- Revisit post-success non-stream settle failure handling so internal billing failure does not automatically masquerade as upstream request failure.

**Step 4: Re-run tests**

Run: `cargo test -p data-plane stream_consistency -- --nocapture`

Run: `cargo test -p control-plane request_logs_api -- --nocapture`

Expected: PASS.

**Step 5: Commit**

```bash
git add services/data-plane/src/proxy/entry.rs services/data-plane/src/proxy/billing_stream.rs services/data-plane/tests/stream_consistency.rs services/control-plane/tests/request_logs_api.rs
git commit -m "fix(data-plane): correct stream attribution and settle behavior" -m "Align billing attribution with actual capture account and stop misreporting post-success settle failures."
```

### Task 6: Full Verification and Docs Sync

**Files:**
- Modify: `README.md`
- Modify: `docs/plans/2026-03-06-ws-billing-failover-hardening-design.md`
- Modify: `docs/plans/2026-03-06-ws-billing-failover-hardening.md`

**Step 1: Update docs**

- Document the new separation between tracing request ids, billing logical ids, continuation keys, and sticky routing keys.

**Step 2: Run focused validation**

Run: `cargo test -p data-plane compatibility_ws -- --nocapture`

Run: `cargo test -p data-plane stream_consistency -- --nocapture`

Run: `cargo test -p data-plane billing_compact_pricing -- --nocapture`

Run: `cargo test -p control-plane base_and_core_part2 -- --nocapture`

Run: `cargo test -p control-plane request_logs_api -- --nocapture`

**Step 3: Run compile checks**

Run: `cargo check -p control-plane`

Run: `cargo check -p data-plane`

Expected: all pass.

**Step 4: Final commit**

```bash
git add README.md docs/plans/2026-03-06-ws-billing-failover-hardening-design.md docs/plans/2026-03-06-ws-billing-failover-hardening.md
git commit -m "docs(repo): record ws billing hardening design" -m "Document the staged hardening plan and verification path for websocket, billing, and failover fixes."
```

