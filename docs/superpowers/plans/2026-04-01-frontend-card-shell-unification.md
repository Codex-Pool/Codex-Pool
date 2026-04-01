# Front-End Card Shell Unification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Unify page-level primary card shells across the front-end around a single shared standard while preserving secondary and semantic surfaces.

**Architecture:** Extract the primary and secondary panel shell recipes into one shared helper, wire `PagePanel`, `Card`, and `SurfaceCard` to that helper, then migrate page-level top cards away from raw HeroUI `Card` imports so the standard shell is consumed through shared primitives instead of handwritten classes.

**Tech Stack:** React 19, TypeScript, HeroUI 2.8, Tailwind utility classes, Vite

---

### Task 1: Unify Panel Shell Recipes

**Files:**
- Create: `/Users/wangnov/Codex-Pool/.worktrees/codex/frontend-card-shell-unification/frontend/src/lib/panel-shell.ts`
- Modify: `/Users/wangnov/Codex-Pool/.worktrees/codex/frontend-card-shell-unification/frontend/src/components/layout/page-archetypes.tsx`
- Modify: `/Users/wangnov/Codex-Pool/.worktrees/codex/frontend-card-shell-unification/frontend/src/components/ui/card.tsx`
- Modify: `/Users/wangnov/Codex-Pool/.worktrees/codex/frontend-card-shell-unification/frontend/src/components/ui/surface.tsx`

- [ ] Extract shared `primary` and `secondary` shell class recipes into a single helper.
- [ ] Point `PagePanel` at the shared helper.
- [ ] Point the local `Card` wrapper at the same helper for default shells.
- [ ] Point `SurfaceCard` at the same helper for overlapping `default` and `muted` tones.

### Task 2: Migrate Page-Level Cards To Shared Primitives

**Files:**
- Modify: `/Users/wangnov/Codex-Pool/.worktrees/codex/frontend-card-shell-unification/frontend/src/pages/Accounts.tsx`
- Modify: `/Users/wangnov/Codex-Pool/.worktrees/codex/frontend-card-shell-unification/frontend/src/pages/AdminApiKeys.tsx`
- Modify: `/Users/wangnov/Codex-Pool/.worktrees/codex/frontend-card-shell-unification/frontend/src/pages/Billing.tsx`
- Modify: `/Users/wangnov/Codex-Pool/.worktrees/codex/frontend-card-shell-unification/frontend/src/pages/Config.tsx`
- Modify: `/Users/wangnov/Codex-Pool/.worktrees/codex/frontend-card-shell-unification/frontend/src/pages/Dashboard.tsx`
- Modify: `/Users/wangnov/Codex-Pool/.worktrees/codex/frontend-card-shell-unification/frontend/src/pages/Groups.tsx`
- Modify: `/Users/wangnov/Codex-Pool/.worktrees/codex/frontend-card-shell-unification/frontend/src/pages/ImportJobs.tsx`
- Modify: `/Users/wangnov/Codex-Pool/.worktrees/codex/frontend-card-shell-unification/frontend/src/pages/Logs.tsx`
- Modify: `/Users/wangnov/Codex-Pool/.worktrees/codex/frontend-card-shell-unification/frontend/src/pages/Models.tsx`
- Modify: `/Users/wangnov/Codex-Pool/.worktrees/codex/frontend-card-shell-unification/frontend/src/pages/OAuthImport.tsx`
- Modify: `/Users/wangnov/Codex-Pool/.worktrees/codex/frontend-card-shell-unification/frontend/src/pages/System.tsx`
- Modify: `/Users/wangnov/Codex-Pool/.worktrees/codex/frontend-card-shell-unification/frontend/src/pages/Usage.tsx`
- Modify: `/Users/wangnov/Codex-Pool/.worktrees/codex/frontend-card-shell-unification/frontend/src/features/billing/admin-cost-report.tsx`

- [ ] Replace page-level HeroUI `Card` imports with the shared local `Card` primitive where the panel is a primary business card.
- [ ] Remove duplicated handwritten main-shell class strings from migrated cards.
- [ ] Keep secondary, inset, semantic, dashed, and special-case surfaces at their lighter or semantic layers instead of forcing them into the primary shell.
- [ ] Normalize any obvious top-level primary-card drift that remains after the import migration.

### Task 3: Add A Lightweight Guardrail

**Files:**
- Modify: `/Users/wangnov/Codex-Pool/.worktrees/codex/frontend-card-shell-unification/frontend/package.json`
- Create: `/Users/wangnov/Codex-Pool/.worktrees/codex/frontend-card-shell-unification/frontend/scripts/check-page-card-shells.mjs`

- [ ] Add a small static check that warns on raw page-level HeroUI `Card` imports in `frontend/src/pages`, `frontend/src/features`, and `frontend/src/tenant` where the shared primitive should be used.
- [ ] Exclude approved special cases such as auth or intentionally branded surfaces if needed.
- [ ] Wire the check into a script that can be run alongside front-end verification.

### Task 4: Verify And Commit

**Files:**
- Modify: `/Users/wangnov/Codex-Pool/.worktrees/codex/frontend-card-shell-unification/docs/superpowers/plans/2026-04-01-frontend-card-shell-unification.md`

- [ ] Run: `cd /Users/wangnov/Codex-Pool/.worktrees/codex/frontend-card-shell-unification/frontend && npm run build`
- [ ] Run: `cd /Users/wangnov/Codex-Pool/.worktrees/codex/frontend-card-shell-unification/frontend && node scripts/check-page-card-shells.mjs`
- [ ] Review the changed pages for any shell regressions or semantic flattening.
- [ ] Commit with a Conventional Commit message.
