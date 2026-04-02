# Browser Language Detection Design

## Summary

This change aligns the front-end language bootstrap behavior with a browser-first experience while keeping explicit user choice sticky.

The core decision is:

- Use the browser language on first load when the user has not chosen a language yet.
- Continue to prefer the saved manual selection when one exists.
- Fall back to English when the detected language is not supported.

## Current State

The front-end already initializes `i18next` with `i18next-browser-languagedetector` in `/Users/wangnov/Codex-Pool/frontend/src/i18n.ts`.

Observed behavior:

- Supported UI languages are `en` and `zh-CN`.
- Detection order already checks local storage before browser language.
- The fallback language is currently `zh-CN`.
- Unsupported browser locales therefore normalize to Chinese instead of English.

## Problem Statement

The product already has automatic browser-language detection, but its final fallback behavior does not match the desired UX.

For users whose browser language is neither English nor Simplified Chinese, the UI currently lands in Chinese because the fallback baseline is `zh-CN`.

That makes first-run language selection feel surprising and increases the chance of an unreadable default experience.

## Goals

- Keep automatic browser-language selection on first load.
- Preserve manual language selection through local storage.
- Normalize supported English variants to `en`.
- Normalize supported Chinese variants to `zh-CN`.
- Default unsupported or missing locales to `en`.

## Non-Goals

- Do not add more UI languages in this change.
- Do not redesign the language switcher UI.
- Do not introduce URL-driven locale routing.

## Recommended Architecture

### Detection priority

Keep the existing detection order:

1. Saved language in `localStorage`
2. Browser language via `navigator`
3. `<html lang>` as a final ambient hint

This already matches the desired "manual choice wins" rule.

### Normalization

Keep the language normalization logic in `/Users/wangnov/Codex-Pool/frontend/src/i18n.ts`, but change its default baseline from `zh-CN` to `en`.

Expected normalization:

- `en`, `en-US`, `en-GB` -> `en`
- `zh`, `zh-CN`, `zh-Hans`, `zh-TW`, `zh-Hant` -> `zh-CN`
- unsupported or empty values -> `en`

### Persistence

Keep storing the resolved supported language in `codex-ui-language`.

This ensures:

- first load respects the browser language
- subsequent manual changes remain sticky

## Testing Strategy

Add a regression test around `/Users/wangnov/Codex-Pool/frontend/src/i18n.ts` that proves:

- unsupported locales fall back to English
- English variants normalize to `en`
- Chinese variants normalize to `zh-CN`

## Acceptance Criteria

- A browser with `navigator.language = fr-FR` lands in English when no saved language exists.
- A browser with `navigator.language = zh-TW` lands in Simplified Chinese when no saved language exists.
- A user who manually selects English keeps English on later visits even if the browser language is Chinese.
