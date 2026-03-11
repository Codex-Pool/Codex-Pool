# OAuth Probe Page Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a temporary standalone OAuth probe page that performs Codex OAuth login, captures the returned exchange payload in memory, and lets the admin download the probe JSON without importing any account into the pool.

**Architecture:** Reuse the existing Codex OAuth callback listener and PKCE flow, but create a separate in-memory probe-session store and separate probe endpoints/pages so the probe path never calls `upsert_oauth_refresh_token`. Extend OAuth code-exchange parsing to preserve raw token payload and raw ID token claims for display/download.

**Tech Stack:** Rust + Axum backend, React + TanStack Query frontend, existing i18n locale bundles.

---

### Task 1: Backend probe session plumbing

**Files:**
- Modify: `services/control-plane/src/app.rs`
- Modify: `services/control-plane/src/app/core_handlers/account_access.rs`

**Steps:**
1. Add probe session/result structs and `oauth_probe_sessions` state storage.
2. Add separate probe callback routes and on-demand listener routes.
3. Ensure listener idle shutdown checks both login and probe session stores.

**Todo:**
- [x] Add probe session/result structs and `oauth_probe_sessions` state storage.
- [x] Add separate probe callback routes and on-demand listener routes.
- [x] Ensure listener idle shutdown checks both login and probe session stores.

### Task 2: OAuth probe payload capture

**Files:**
- Modify: `services/control-plane/src/oauth.rs`
- Modify: `services/control-plane/src/app/core_handlers/account_access.rs`

**Steps:**
1. Preserve raw token endpoint payload and raw ID token claims during authorization-code exchange.
2. Implement probe-only session create/get/manual-callback/auto-callback handlers.
3. Store probe results in memory and never write upstream accounts.

**Todo:**
- [x] Preserve raw token endpoint payload and raw ID token claims during authorization-code exchange.
- [x] Implement probe-only session create/get/manual-callback/auto-callback handlers.
- [x] Store probe results in memory and never write upstream accounts.

### Task 3: Frontend standalone probe page

**Files:**
- Create: `frontend/src/api/oauthProbe.ts`
- Create: `frontend/src/pages/OAuthProbe.tsx`
- Modify: `frontend/src/App.tsx`
- Modify: `frontend/src/components/layout/AppLayout.tsx`

**Steps:**
1. Add frontend API bindings for probe session create/get/manual callback.
2. Build a minimal page that starts OAuth, polls the session, renders probe JSON, and downloads it.
3. Add a temporary route and sidebar entry for easy access.

**Todo:**
- [x] Add frontend API bindings for probe session create/get/manual callback.
- [x] Build a minimal page that starts OAuth, polls the session, renders probe JSON, and downloads it.
- [x] Add a temporary route and sidebar entry for easy access.

### Task 4: i18n and verification

**Files:**
- Modify: `frontend/src/locales/en.ts`
- Modify: `frontend/src/locales/zh-CN.ts`
- Modify: `frontend/src/locales/zh-TW.ts`
- Modify: `frontend/src/locales/ja.ts`
- Modify: `frontend/src/locales/ru.ts`

**Steps:**
1. Add the minimal `oauthProbe` and `nav.oauthProbe` locale keys in all supported languages.
2. Run `cargo check -p control-plane`.
3. Run `npm run i18n:check`, `npm run i18n:hardcode -- --no-baseline`, `node scripts/i18n/check-missing-runtime-keys.mjs`, `npm run lint`, and `npm run build`.

**Todo:**
- [x] Add the minimal `oauthProbe` and `nav.oauthProbe` locale keys in all supported languages.
- [x] Run `cargo check -p control-plane`.
- [x] Run `npm run i18n:check`.
- [x] Run `npm run i18n:hardcode -- --no-baseline`.
- [x] Run `node scripts/i18n/check-missing-runtime-keys.mjs`.
- [x] Run `npm run lint`.
- [x] Run `npm run build`.
