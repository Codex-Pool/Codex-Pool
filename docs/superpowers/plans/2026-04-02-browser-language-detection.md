# Browser Language Detection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the front-end automatically follow the browser language on first load while falling back to English and preserving manual language selection.

**Architecture:** Keep the existing `i18next-browser-languagedetector` flow, tighten verification around the normalization behavior, and switch the fallback baseline in the shared i18n bootstrap so unsupported locales resolve to English instead of Chinese.

**Tech Stack:** React 19, TypeScript, i18next, react-i18next, Vite, Node test runner

---

### Task 1: Add A Regression Test For Language Resolution

**Files:**
- Create: `/Users/wangnov/Codex-Pool/frontend/src/i18n.test.ts`
- Modify: `/Users/wangnov/Codex-Pool/frontend/src/i18n.ts`

- [ ] **Step 1: Write the failing test**
  Assert that unsupported locales resolve to English while supported English and Chinese variants normalize to the expected bundled languages.
- [ ] **Step 2: Run test to verify it fails**
  Run: `cd /Users/wangnov/Codex-Pool/frontend && node --test src/i18n.test.ts`
  Expected: FAIL because the current fallback path still resolves unsupported locales to `zh-CN`.
- [ ] **Step 3: Write minimal implementation**
  Change the fallback language baseline from `zh-CN` to `en` without disturbing detection order or manual override behavior.
- [ ] **Step 4: Run test to verify it passes**
  Run: `cd /Users/wangnov/Codex-Pool/frontend && node --test src/i18n.test.ts`
  Expected: PASS

### Task 2: Run Full Front-End Verification

**Files:**
- Modify: `/Users/wangnov/Codex-Pool/frontend/src/i18n.ts`
- Modify: `/Users/wangnov/Codex-Pool/frontend/src/i18n.test.ts`

- [ ] **Step 1: Run the focused test suite**
  Run: `cd /Users/wangnov/Codex-Pool/frontend && node --test src/i18n.test.ts`
- [ ] **Step 2: Run the full front-end test suite**
  Run: `cd /Users/wangnov/Codex-Pool/frontend && npm test`
- [ ] **Step 3: Run the front-end build**
  Run: `cd /Users/wangnov/Codex-Pool/frontend && npm run build`
- [ ] **Step 4: Review behavior against the spec**
  Confirm the implementation still keeps local-storage preference ahead of browser detection and that unsupported languages now land on English.
