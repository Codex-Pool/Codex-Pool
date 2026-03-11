# OAuth Profile Claims Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Persist a richer structured OAuth profile snapshot for imported/refreshed accounts and expose it through `oauth status` responses so the Accounts page can retain higher-value identity/subscription metadata.

**Architecture:** Extend the OAuth parsing layer to normalize ID-token and access-token claims into `OAuthTokenInfo`, expand the session-profile persistence model in both in-memory and Postgres stores, and surface the new fields from `OAuthAccountStatusResponse`. Frontend only needs type updates because the raw tab already serializes the returned status object.

**Tech Stack:** Rust + Axum + sqlx on the backend, TypeScript + React on the frontend API client.

---

### Task 1: Define the shared structured OAuth profile schema

**Files:**
- Modify: `crates/codex-pool-core/src/api.rs`
- Modify: `frontend/src/api/accounts.ts`

**Steps:**
1. Add typed API structs for OAuth organizations/groups.
2. Extend `OAuthAccountStatusResponse` with the approved P0/P1/P2 fields.
3. Mirror the new response fields in the frontend type so raw JSON remains typed.

**Todo:**
- [x] Add raw OAuth organization/group array fields to the shared API schema.
- [x] Extend `OAuthAccountStatusResponse` with P0/P1/P2 fields.
- [x] Extend the frontend account status type.

### Task 2: Add failing parser tests first

**Files:**
- Modify: `services/control-plane/src/oauth.rs`

**Steps:**
1. Add an ID-token parsing test for `sub`, `auth_provider`, `email_verified`, subscription timing, `chatgpt_user_id`, `organizations`, and `groups`.
2. Add an access-token parsing test for `chatgpt_account_user_id` and `chatgpt_compute_residency`.
3. Run the focused OAuth parser tests and verify they fail before implementation.

**Todo:**
- [x] Add failing ID-token claim parser coverage.
- [x] Add failing access-token claim parser coverage.
- [x] Run focused parser tests and confirm RED.

### Task 3: Implement normalized OAuth claim parsing

**Files:**
- Modify: `services/control-plane/src/oauth.rs`

**Steps:**
1. Expand `OAuthIdTokenClaims` with the structured fields sourced from the ID token.
2. Add a parsed access-token claims struct for the account-instance/runtime fields.
3. Update `refresh_token()` to merge ID-token and access-token claims into `OAuthTokenInfo`.

**Todo:**
- [x] Expand `OAuthIdTokenClaims`.
- [x] Add access-token claim parsing.
- [x] Merge normalized claims into `OAuthTokenInfo`.

### Task 4: Add failing store persistence tests

**Files:**
- Modify: `services/control-plane/src/store/trait_impl.rs`
- Modify: `services/control-plane/tests/postgres_repo.rs`

**Steps:**
1. Extend the in-memory OAuth status regression test to assert the new structured fields are returned.
2. Extend the Postgres repository regression test to assert the new structured fields are returned.
3. Run the focused store tests and verify they fail before persistence changes.

**Todo:**
- [x] Add failing in-memory structured OAuth profile test.
- [x] Add failing Postgres structured OAuth profile test.
- [x] Run focused store tests and confirm RED.

### Task 5: Persist the structured OAuth profile snapshot

**Files:**
- Modify: `services/control-plane/src/store/defs.rs`
- Modify: `services/control-plane/src/store/in_memory_core.rs`
- Modify: `services/control-plane/src/store/oauth_ops.rs`
- Modify: `services/control-plane/src/store/postgres/impl_crud/bootstrap_schema.rs`
- Modify: `services/control-plane/src/store/postgres/impl_crud/oauth_upsert.rs`
- Modify: `services/control-plane/src/store/postgres/impl_oauth_snapshot/prelude.rs`
- Modify: `services/control-plane/src/store/postgres/impl_oauth_snapshot/rate_limit_jobs.rs`

**Steps:**
1. Expand the in-memory session profile record with the structured fields.
2. Expand the Postgres session profile schema with scalar columns and JSONB columns for org/group arrays.
3. Persist the normalized fields on import/upsert/refresh-success paths.
4. Return the fields from single-account and batch OAuth status queries.

**Todo:**
- [x] Expand the in-memory session profile model.
- [x] Expand the Postgres session profile schema.
- [x] Persist the structured fields on import/upsert/refresh.
- [x] Return the structured fields from OAuth status queries.

### Task 6: Verification

**Files:**
- Modify: `docs/plans/2026-03-11-oauth-profile-claims.md`

**Steps:**
1. Re-run focused parser tests.
2. Re-run focused store tests.
3. Run `cargo check -p control-plane`.
4. Run frontend type/build verification.
5. Update this plan with checked todos.

**Todo:**
- [x] Run focused parser tests.
- [x] Run focused store tests.
- [x] Run `cargo check -p control-plane`.
- [x] Run `npm run lint`.
- [x] Run `npm run build`.

**Verification Notes:**
- `cargo test -p control-plane parse_id_token_claims_reads_structured_identity_and_subscription_fields -- --nocapture`
- `cargo test -p control-plane parse_access_token_claims_reads_account_instance_fields -- --nocapture`
- `cargo test -p control-plane in_memory_oauth_status_exposes_email -- --nocapture`
- `cargo test -p control-plane --test integration postgres_repo_oauth_status_exposes_email -- --nocapture`
  - In the current environment this test followed the existing skip path because `CONTROL_PLANE_DATABASE_URL` was not set.
- `cargo check -p control-plane`
- `npm run lint`
- `npm run build`
