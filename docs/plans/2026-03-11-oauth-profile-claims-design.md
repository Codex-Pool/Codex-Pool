# OAuth Profile Claims Design

**Date:** 2026-03-11

**Goal:** Persist the most valuable OpenAI OAuth identity, subscription, and account-instance claims so Accounts can reliably distinguish human identity from account instance and retain higher-value metadata beyond email/plan.

## Context

Current OAuth account persistence only stores a small subset of profile metadata:

- `email`
- `chatgpt_plan_type`
- `source_type`

Probe samples showed that the real OAuth payload exposes a richer structure:

- Human identity: `sub`, `email_verified`, `chatgpt_user_id`
- Account instance: `chatgpt_account_id`, `chatgpt_account_user_id`
- Subscription timing: `chatgpt_subscription_active_start`, `chatgpt_subscription_active_until`, `chatgpt_subscription_last_checked`
- Runtime/account traits: `chatgpt_compute_residency`
- Workspace-ish metadata: `organizations`, `groups`

The same email/user can legitimately have multiple account instances, so the model needs to preserve both “same person” and “different account instance” identifiers.

## Constraints

- Keep existing OAuth import/refresh semantics intact.
- Continue using the `oauth status` response as the frontend source of truth.
- P0/P1/P2 fields should all be stored structurally.
- P2 (`organizations`, `groups`) should not get dedicated UI yet; they only need to appear in the raw payload view.
- Changes must work for both in-memory and Postgres stores.

## Recommended Design

### 1. Introduce a structured OAuth profile snapshot

Expand the persisted session profile from a minimal trio into a richer OAuth profile snapshot containing:

- P0
  - `oauth_subject` (`sub`)
  - `oauth_identity_provider` (claim `auth_provider`, e.g. `google`)
  - `email_verified`
  - `chatgpt_user_id`
  - `chatgpt_subscription_active_start`
  - `chatgpt_subscription_active_until`
  - `chatgpt_subscription_last_checked`
- P1
  - `chatgpt_account_user_id`
  - `chatgpt_compute_residency`
- P2
  - `organizations`
  - `groups`

These fields should live alongside the existing `email`, `chatgpt_plan_type`, `source_type`, and token-expiry metadata.

### 2. Parse claims from both ID token and access token

The current parser only extracts a few fields from the ID token. The samples show:

- ID token is the right source for:
  - `sub`
  - `auth_provider`
  - `email`
  - `email_verified`
  - `chatgpt_account_id`
  - `chatgpt_plan_type`
  - `chatgpt_user_id`
  - subscription timing
  - `organizations`
  - `groups`
- Access token is the right source for:
  - `chatgpt_account_user_id`
  - `chatgpt_compute_residency`

The merged `OAuthTokenInfo` should expose a normalized, typed view of all of these claims.

### 3. Persist the snapshot on import and refresh

Wherever we currently write session-profile data:

- OAuth import
- OAuth upsert
- successful refresh-token rotation

we should write the expanded OAuth profile snapshot as part of the same update. Missing values should preserve existing stored values where appropriate.

### 4. Return the structured profile through `OAuthAccountStatusResponse`

Extend the shared API response so Accounts can receive the structured fields directly from:

- `GET /upstream-accounts/{account_id}/oauth/status`
- `POST /upstream-accounts/oauth/statuses`

This automatically makes the new fields visible in the frontend raw tab without requiring a dedicated UI section.

## Alternatives Considered

### Store the entire raw OAuth payload

- Pro: quickest way to keep everything
- Con: poor contract shape, noisy frontend payloads, harder future UI work, unclear stable schema

### Only persist P0/P1 and leave P2 in probe-only storage

- Pro: smaller schema
- Con: loses structured org/group data even though the user explicitly wants all P0/P1/P2 stored

## Success Criteria

- Imported/refreshed OAuth accounts persist P0/P1/P2 claims structurally.
- `oauth status` responses expose the new structured fields.
- Existing Accounts raw tab shows the new fields without additional frontend presentation work.
- Existing import/refresh flows continue to work unchanged.
