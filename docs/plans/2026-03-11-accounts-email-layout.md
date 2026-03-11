# Accounts Email Layout Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Prioritize email in the Accounts UI, remove the next-refresh surface, and refactor the detail dialog layout/scrolling so session accounts are easier to inspect.

**Architecture:** Extend the OAuth status model with persisted `email`, then let the existing `oauthStatuses` query drive both list identity rendering and detail-dialog identity content. Keep the tab structure intact while reorganizing the detail dialog into clearer grouped sections with bounded scrolling containers.

**Tech Stack:** Rust + Axum + sqlx on the backend, React + TanStack Query + existing shadcn-style UI components on the frontend.

---

### Task 1: Persist email in OAuth account status

**Files:**
- Modify: `crates/codex-pool-core/src/api.rs`
- Modify: `services/control-plane/src/oauth.rs`
- Modify: `services/control-plane/src/store/defs.rs`
- Modify: `services/control-plane/src/store/in_memory_core.rs`
- Modify: `services/control-plane/src/store/postgres/impl_crud/bootstrap_schema.rs`
- Modify: `services/control-plane/src/store/postgres/impl_crud/oauth_upsert.rs`
- Modify: `services/control-plane/src/store/postgres/impl_oauth_snapshot/prelude.rs`
- Modify: `services/control-plane/src/store/postgres/impl_oauth_snapshot/rate_limit_jobs.rs`

**Steps:**
1. Add `email` to the shared OAuth status response and token info/session profile structs.
2. Add an `email` column to `upstream_account_session_profiles` and include it in all session-profile upserts.
3. Persist `email` on OAuth import and refresh-success paths.
4. Return `email` from both single-account and batch OAuth status queries.

**Todo:**
- [x] Add `email` to shared OAuth API models.
- [x] Persist `email` in session profile state for in-memory and Postgres stores.
- [x] Return `email` from OAuth status queries.

### Task 2: Add the minimal regression coverage first

**Files:**
- Modify: `services/control-plane/src/store/trait_impl.rs`
- Modify: `services/control-plane/tests/postgres_repo.rs`

**Steps:**
1. Add an in-memory store test showing imported OAuth status exposes `email`.
2. Add a Postgres store integration test showing OAuth status exposes `email`.
3. Run the focused tests and confirm they fail before implementation if needed, then pass after implementation.

**Todo:**
- [x] Add in-memory OAuth status email regression test.
- [x] Add Postgres OAuth status email regression test.
- [x] Run focused backend tests.

### Task 3: Update Accounts list identity rendering

**Files:**
- Modify: `frontend/src/api/accounts.ts`
- Modify: `frontend/src/features/accounts/use-accounts-columns.tsx`
- Modify: `frontend/src/features/accounts/utils.ts`

**Steps:**
1. Add `email` to the frontend OAuth status type.
2. Change the identity column to prefer `email` and use `label` as secondary text when available.
3. Remove the `nextRefreshAt` column.
4. Extend account search to include `email`.

**Todo:**
- [x] Extend frontend account status typing with `email`.
- [x] Refactor identity cell to prefer `email`.
- [x] Remove the next-refresh column.
- [x] Include `email` in Accounts search.

### Task 4: Refactor detail dialog layout and scrolling

**Files:**
- Modify: `frontend/src/features/accounts/account-detail-dialog.tsx`

**Steps:**
1. Constrain dialog height and make the content area scroll independently.
2. Reorganize the profile tab into clearer identity and technical sections.
3. Reorganize the OAuth tab into grouped cards and remove the next-refresh field.
4. Give bearer-token/raw areas dedicated scroll containers.

**Todo:**
- [x] Add bounded dialog/content scrolling.
- [x] Restructure the Profile tab layout.
- [x] Restructure the OAuth tab layout and remove next-refresh.
- [x] Bound long technical text blocks with internal scroll.

### Task 5: i18n and verification

**Files:**
- Modify: `frontend/src/locales/en.ts`
- Modify: `frontend/src/locales/zh-CN.ts`
- Modify: `frontend/src/locales/zh-TW.ts`
- Modify: `frontend/src/locales/ja.ts`
- Modify: `frontend/src/locales/ru.ts`

**Steps:**
1. Add any new field/group labels needed by the refactored detail dialog.
2. Run focused backend tests.
3. Run `cargo check -p control-plane`.
4. Run `npm run i18n:check`, `npm run i18n:hardcode -- --no-baseline`, `node scripts/i18n/check-missing-runtime-keys.mjs`, `npm run lint`, and `npm run build`.

**Todo:**
- [x] Add any new i18n labels used by the refactored UI.
- [x] Run focused backend tests.
- [x] Run `cargo check -p control-plane`.
- [x] Run `npm run i18n:check`.
- [x] Run `npm run i18n:hardcode -- --no-baseline`.
- [x] Run `node scripts/i18n/check-missing-runtime-keys.mjs`.
- [x] Run `npm run lint`.
- [x] Run `npm run build`.
