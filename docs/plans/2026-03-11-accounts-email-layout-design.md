# Accounts Email Layout Design

**Date:** 2026-03-11

**Goal:** Make the Accounts page easier to identify and inspect by prioritizing email over account ID, removing the low-value "next refresh" surface, and restructuring the detail dialog so long content remains readable and scrollable.

## Context

The current Accounts experience already exposes `chatgpt_plan_type`, refresh state, and rate-limit state, but it still emphasizes technical identifiers that are less useful for daily operations. OAuth probe results also showed that `email` is a better human-facing identifier than `chatgpt_account_id`, while `next_refresh_at` is noisy and not actionable for the user.

The detail dialog currently mixes identity, OAuth state, and raw values in one long form layout. When token or raw payload fields become long, the dialog can become hard to scroll and inspect.

## Constraints

- Do not introduce `chatgpt_account_id` as a primary visible identifier in the list.
- Keep the existing rate-limit and raw payload views available.
- Reuse the existing Accounts data flow when possible.
- If `email` is missing, the UI must gracefully fall back to `label`.
- Keep all user-facing text localized.

## Recommended Design

### 1. Data model

- Extend OAuth account status responses with `email`.
- Persist `email` in the OAuth session profile so it survives imports and refresh flows.
- Continue using the existing `oauthStatuses` query as the source of truth for session account display metadata.

### 2. Accounts list

- The identity column should display:
  - Primary line: `email` when available, otherwise `label`
  - Secondary line: `label` when `email` exists and differs, otherwise the internal account UUID
- Keep the existing `plan`, `login status`, `credential type`, and `rate limit` columns.
- Remove the `next refresh` column entirely.
- Search should include `email`.

### 3. Account detail dialog

- Keep the existing tabs, but restructure the content inside them.
- `Profile` tab:
  - Summarize human-facing identity first: email, label, mode, status, base URL, created at
  - Move technical values like internal account ID and bearer token into lower-priority cards
- `OAuth` tab:
  - Group fields into sections instead of one flat grid
  - Hide or remove `Next Refresh At`
  - Keep plan, credential kind, source type, token expiry, refresh status, token family/version
- `Limits` tab:
  - Preserve current behavior
- `Raw` tab:
  - Preserve current behavior

### 4. Scrolling and layout

- Cap dialog height relative to viewport.
- Make the dialog body independently scrollable.
- Give long text blocks such as bearer token and raw payload their own bounded scroll containers.

## Alternatives Considered

### Keep label-first and only add email as a subtitle

- Lower risk, but does not solve the user’s main “which account is this?” problem.

### Put account ID back into the main list to disambiguate

- Strong for uniqueness, but hurts scanability and conflicts with the explicit requirement to prefer email.

## Success Criteria

- The Accounts list can be scanned primarily by email.
- The list no longer shows `next refresh`.
- The detail dialog no longer becomes unwieldy when content is long.
- Session accounts without email still display a sensible fallback.
