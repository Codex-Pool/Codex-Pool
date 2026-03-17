/// <reference types="node" />

import assert from 'node:assert/strict'
import test from 'node:test'
import * as pageArchetypes from './page-archetypes.ts'

import {
  describeDashboardShellLayout,
  describePageRegions,
  resolvePageArchetype,
  type PageArchetype,
} from './page-archetypes.ts'

test('resolvePageArchetype gives auth a branded but non-effect-heavy structure', () => {
  const archetype = resolvePageArchetype('auth')

  assert.equal(archetype.name, 'auth')
  assert.equal(archetype.introStyle, 'stage')
  assert.equal(archetype.headerSurface, 'stage')
  assert.equal(archetype.stageMode, 'split')
  assert.equal(archetype.primaryZone, 'form')
  assert.equal(archetype.effectProfile, 'subtle')
  assert.equal(archetype.mobile.stageCompression, 'condense')
  assert.equal(archetype.mobile.primaryFirst, true)
})

test('resolvePageArchetype keeps workspace compact and task-first', () => {
  const archetype = resolvePageArchetype('workspace')

  assert.equal(archetype.name, 'workspace')
  assert.equal(archetype.introStyle, 'compact')
  assert.equal(archetype.headerSurface, 'panel')
  assert.equal(archetype.primaryZone, 'task')
  assert.equal(archetype.secondaryDensity, 'summary-first')
  assert.equal(archetype.stageMode, 'none')
  assert.equal(archetype.mobile.primaryFirst, true)
  assert.equal(archetype.mobile.stageCompression, 'hide')
})

test('resolvePageArchetype keeps dashboard in a context-panel rhythm', () => {
  const archetype = resolvePageArchetype('dashboard')

  assert.equal(archetype.name, 'dashboard')
  assert.equal(archetype.introStyle, 'compact')
  assert.equal(archetype.headerSurface, 'panel')
  assert.equal(archetype.stageMode, 'inline')
  assert.equal(archetype.secondaryDensity, 'balanced')
  assert.equal(archetype.effectProfile, 'minimal')
})

test('resolvePageArchetype falls back to settings for unknown variants', () => {
  const archetype = resolvePageArchetype('unknown' as PageArchetype)

  assert.equal(archetype.name, 'settings')
  assert.equal(archetype.introStyle, 'compact')
  assert.equal(archetype.headerSurface, 'plain')
  assert.equal(archetype.primaryZone, 'content')
  assert.equal(archetype.effectProfile, 'none')
  assert.equal(archetype.mobile.primaryFirst, true)
})

test('describePageRegions separates auth stage and keeps workspace summary after the main task', () => {
  assert.deepEqual(describePageRegions('auth'), {
    introAlignment: 'start',
    contentLayout: 'split',
    secondaryPlacement: 'after',
    stageEmphasis: 'high',
  })

  assert.deepEqual(describePageRegions('workspace'), {
    introAlignment: 'between',
    contentLayout: 'split',
    secondaryPlacement: 'aside',
    stageEmphasis: 'low',
  })
})

test('describeDashboardShellLayout keeps dashboard metrics ahead of the rail on mobile and avoids stretched headers on desktop', () => {
  assert.deepEqual(describeDashboardShellLayout(), {
    mobileRailPlacement: 'after-content',
    desktopAlignment: 'start',
  })
})

test('describeReportShellLayout keeps filters near the intro, shows the main trend first, and defers rail content until after the lead report on mobile', () => {
  const describeReportShellLayout = (
    pageArchetypes as typeof pageArchetypes & {
      describeReportShellLayout?: () => unknown
    }
  ).describeReportShellLayout

  assert.deepEqual(describeReportShellLayout?.(), {
    mobileToolbarPlacement: 'after-intro',
    mobileRailPlacement: 'after-content',
    desktopContentBalance: 'lead-first',
  })
})

test('describeBillingReportLayout keeps billing summaries ahead of the primary trend, pushes context tools into the rail, and keeps detailed tables behind them on mobile', () => {
  const describeBillingReportLayout = (
    pageArchetypes as typeof pageArchetypes & {
      describeBillingReportLayout?: () => unknown
    }
  ).describeBillingReportLayout

  assert.deepEqual(describeBillingReportLayout?.(), {
    leadSequence: 'summary-then-trend',
    mobileContextPlacement: 'after-lead',
    mobileDetailPlacement: 'after-context',
  })
})

test('describeLogsWorkbenchLayout keeps tab switching near the intro and keeps filters inside the active log panel', () => {
  const describeLogsWorkbenchLayout = (
    pageArchetypes as typeof pageArchetypes & {
      describeLogsWorkbenchLayout?: () => unknown
    }
  ).describeLogsWorkbenchLayout

  assert.deepEqual(describeLogsWorkbenchLayout?.(), {
    mobileToolbarPlacement: 'after-intro',
    desktopToolbarAlignment: 'between',
    filterPlacement: 'within-panel',
  })
})

test('describeAccountsWorkspaceLayout keeps primary actions ahead of filters and keeps batch actions attached to the workbench controls', () => {
  const describeAccountsWorkspaceLayout = (
    pageArchetypes as typeof pageArchetypes & {
      describeAccountsWorkspaceLayout?: () => unknown
    }
  ).describeAccountsWorkspaceLayout

  assert.deepEqual(describeAccountsWorkspaceLayout?.(), {
    mobileToolbarPlacement: 'after-intro',
    mobileFiltersPlacement: 'after-toolbar',
    batchActionsPlacement: 'with-filters',
  })
})
