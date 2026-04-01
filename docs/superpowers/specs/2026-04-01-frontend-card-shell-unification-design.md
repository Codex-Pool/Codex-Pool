# Front-End Card Shell Unification Design

## Summary

This spec standardizes the front-end card system around one primary page-level card shell while preserving lower-level semantic surfaces for alerts, inset sections, and empty states.

The core decision is:

- All page-level top cards must use the same primary shell.
- Nested blocks must keep a lighter secondary or semantic treatment.
- Directly hand-authoring page-level HeroUI `Card` shells should stop.

This keeps the product visually coherent without flattening information hierarchy.

## Current State

The front-end already has a design language and shared shell primitives, but they are not the only entry points:

- Design recipes exist in `/Users/wangnov/Codex-Pool/frontend/src/lib/design-system.ts`.
- The standard page panel shell already exists in `/Users/wangnov/Codex-Pool/frontend/src/components/layout/page-archetypes.tsx`.
- A separate surface abstraction exists in `/Users/wangnov/Codex-Pool/frontend/src/components/ui/surface.tsx`.
- Many pages still render HeroUI `Card` directly with handwritten shell classes.

Observed result:

- Theme values are present.
- Shared primitives are present.
- Enforcement is missing.
- Visual drift accumulates page by page.

## Problem Statement

The product currently mixes at least three shell entry points:

1. `PagePanel`
2. `SurfaceCard`
3. Direct HeroUI `Card` with handwritten shell classes

Because page authors can choose any of these paths, the same visual layer is implemented repeatedly with small variations in border treatment, shadow depth, background tone, and density.

This is why the UI can feel inconsistent even though a theme exists.

## Goals

- Make one primary shell the default for all page-level top cards across the front-end.
- Preserve semantic hierarchy for nested sections, notices, alerts, and inset blocks.
- Reduce handwritten shell classes in page files.
- Make it harder to regress back into per-page shell drift.
- Keep HeroUI as the base component system and remain aligned with existing theme tokens.

## Non-Goals

- Do not make every rectangular container look identical.
- Do not flatten alerts, empty states, or inset metrics into the same visual weight as top-level cards.
- Do not restyle special brand surfaces like auth hero panels or stage sections into generic business cards.
- Do not redesign page layouts or information architecture as part of this standardization.

## Design Principles

### 1. One primary shell for one visual layer

Top-level business panels should always use the same shell:

- `border-small`
- `border-default-200`
- `bg-content1`
- `shadow-small`
- `rounded-large`

This is the current standard primary shell already represented by `PagePanel`.

### 2. Hierarchy must remain visible

Nested blocks should not use the same weight as primary panels by default.

Secondary surfaces should continue to look quieter:

- `bg-content2`
- reduced or no shadow
- same radius family
- lower visual dominance

### 3. Semantics belong inside the card before they belong on the outer shell

Alerts, warnings, and state emphasis should come from:

- chips
- icons
- inline notices
- row accents
- local semantic blocks

They should not normally mutate the outer page-level shell unless the entire panel is itself a semantic notice.

### 4. Page code should consume primitives, not recreate them

Page authors should choose from a small set of approved surface primitives instead of rebuilding shells with free-form class strings.

## Surface Hierarchy

### Primary panel

Use for:

- page-level summary cards
- page-level tables
- page-level charts
- page-level detail panels
- modal-level main content panels when they function as first-class content blocks

Component:

- `PagePanel tone="primary"`

Visual standard:

- standard main shell

### Secondary panel

Use for:

- nested sections inside a primary panel
- support metrics
- inset summaries
- lighter grouped content inside a larger panel

Component:

- `PagePanel tone="secondary"`
- or `SurfaceSection` where header/content framing is useful

Visual standard:

- quieter surface
- no competing shadow depth

### Semantic notice

Use for:

- warning banners
- danger notices
- success confirmations
- operational advisories

Component:

- `SurfaceNotice`

Visual standard:

- semantic tint
- no promotion into a full primary page shell unless the whole panel is intentionally a notice card

### Inset or utility block

Use for:

- mini stat groups
- code blocks
- metadata boxes
- filter summaries
- compact structured content inside a larger panel

Component:

- `SurfaceInset`
- `SurfaceCode`
- small layout containers built on the secondary surface recipe

### Special surfaces

Do not force into the primary panel standard:

- auth shells
- landing or stage panels
- onboarding hero sections
- intentionally branded spotlight panels

These remain opt-in special surfaces.

## Recommended Architecture

### Source of truth

The primary shell should have one source of truth, consumed by all page-level panel abstractions.

Recommended implementation:

1. Extract shared shell class recipes into a single panel/surface helper.
2. Make `PagePanel` consume that helper for `primary` and `secondary`.
3. Make `SurfaceCard` consume the same helper for overlapping tones where appropriate.
4. Stop duplicating primary-shell classes directly in page files.

This means the system becomes token-driven and primitive-driven at the same time.

### Recommended primitive roles

- `PagePanel`
  - official page-level panel primitive
- `SurfaceCard`
  - reusable semantic or utility card primitive
- `SurfaceNotice`
  - semantic alert/notice primitive
- `SurfaceSection`
  - nested grouped section inside larger panels

The important boundary is:

- page files should prefer `PagePanel` for top-level business cards
- page files should avoid raw HeroUI `Card` for top-level shell creation

## Migration Strategy

### Phase 1. Unify the primitive

Create a shared surface recipe helper so the main shell is defined in one place.

Deliverables:

- shared primary panel recipe
- shared secondary panel recipe
- `PagePanel` and `SurfaceCard` aligned to the same source

### Phase 2. Migrate page-level top cards

Convert top-level page cards in admin and tenant pages from raw `Card` to `PagePanel`.

Scope includes:

- dashboards
- reports
- tables
- page summary panels
- page detail sections

Scope excludes:

- nested mini-blocks
- banners
- inline notices
- special auth or hero surfaces

### Phase 3. Normalize nested surfaces

Review common nested blocks and map them to:

- `PagePanel tone="secondary"`
- `SurfaceSection`
- `SurfaceNotice`
- `SurfaceInset`

This phase is intentionally selective, not blanket replacement.

### Phase 4. Add a guardrail

Add a lightweight rule to prevent recurrence.

Recommended guardrail:

- lint or static search rule for page files
- flag raw page-level `Card` usage with the standard primary shell class string

This should not ban HeroUI `Card` globally.
It should only discourage recreating the primary page shell by hand in page-level files.

## How to Decide Which Primitive to Use

Use this decision order:

1. Is this a page-level top card visible as a first-class panel on the page?
   - Use `PagePanel tone="primary"`.
2. Is this a nested support section within a larger card?
   - Use `PagePanel tone="secondary"` or `SurfaceSection`.
3. Is this conveying state or operational severity?
   - Use `SurfaceNotice`.
4. Is this a compact inset utility block?
   - Use `SurfaceInset` or a secondary-surface helper.
5. Is this an intentionally branded or special shell?
   - Keep it as a documented exception.

## Examples

### Good

- Dashboard summary cards use the standard primary shell.
- An alerts panel uses the standard primary shell, while the alert rows or notice area inside it carry the warning or danger semantics.
- A chart card uses a primary shell, with smaller muted metric blocks inside.

### Not recommended

- A top-level table card handwrites `border-small border-default-200 bg-content1 shadow-small`.
- A warning row promotes the whole page card into a custom tinted shell when the warning can be represented inside the card.
- A nested metadata box uses the same visual weight as the page-level panel around it.

## Risks And Mitigations

### Risk: hierarchy gets flattened

Mitigation:

- keep secondary and semantic surfaces as first-class concepts
- do not standardize nested blocks into primary shells

### Risk: migration becomes noisy and repetitive

Mitigation:

- first align the primitive source of truth
- then migrate by page groups

### Risk: exceptions quietly multiply again

Mitigation:

- define approved exceptions explicitly
- add a lint or static guardrail for page-level handwritten main shell usage

## Testing And Verification

### Visual verification

- compare representative admin pages and tenant pages before and after migration
- confirm top-level shells match across dashboards, reports, and tables
- confirm nested blocks still read as secondary hierarchy

### Functional verification

- ensure component swaps do not change interaction behavior
- confirm spacing, overflow, and responsive behavior remain intact

### Regression checks

- run front-end build
- run existing page-level tests where relevant
- spot-check light and dark themes

## Recommendation

Adopt this as a front-end standard:

- one shared primary shell for all page-level top cards
- retain explicit secondary and semantic surface layers
- migrate pages toward `PagePanel`
- keep HeroUI as the base library, but stop using raw page-level `Card` shells as an authoring pattern

This gives the product a unified visual structure without sacrificing semantic clarity.

## Implementation Planning Notes

When implementation begins, the work should be split into three concrete tracks:

1. Primitive alignment
   - unify shell recipes across `PagePanel` and `SurfaceCard`
2. Page migration
   - convert top-level cards in admin pages and tenant pages
3. Guardrail
   - add a lightweight check that discourages handwritten page-level main shell recreation

The migration should favor low-risk, page-by-page replacement over a single sweeping refactor.
