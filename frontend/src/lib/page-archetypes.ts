export type PageArchetype = 'auth' | 'dashboard' | 'workspace' | 'detail' | 'settings'

export interface PageArchetypeConfig {
  name: PageArchetype
  introStyle: 'stage' | 'compact'
  stageMode: 'split' | 'inline' | 'none'
  headerSurface: 'stage' | 'panel' | 'plain'
  primaryZone: 'form' | 'task' | 'content'
  secondaryDensity: 'narrative' | 'balanced' | 'summary-first'
  surfaceTone: 'refined' | 'neutral' | 'quiet'
  effectProfile: 'subtle' | 'minimal' | 'none'
  mobile: {
    stageCompression: 'condense' | 'hide' | 'keep'
    primaryFirst: boolean
  }
}

export interface PageArchetypeRegions {
  introAlignment: 'start' | 'between'
  contentLayout: 'split' | 'stack'
  secondaryPlacement: 'after' | 'aside'
  stageEmphasis: 'high' | 'medium' | 'low'
}

export interface DashboardShellLayout {
  mobileRailPlacement: 'after-content' | 'after-intro'
  desktopAlignment: 'start' | 'stretch'
}

export interface ReportShellLayout {
  mobileToolbarPlacement: 'after-intro'
  mobileRailPlacement: 'after-content' | 'after-toolbar'
  desktopContentBalance: 'lead-first' | 'balanced'
}

export interface BillingReportLayout {
  leadSequence: 'summary-then-trend'
  mobileContextPlacement: 'after-lead'
  mobileDetailPlacement: 'after-context'
}

export interface LogsWorkbenchLayout {
  mobileToolbarPlacement: 'after-intro'
  desktopToolbarAlignment: 'between'
  filterPlacement: 'within-panel'
}

export interface AccountsWorkspaceLayout {
  mobileToolbarPlacement: 'after-intro'
  mobileFiltersPlacement: 'after-toolbar'
  batchActionsPlacement: 'with-filters'
}

export interface ModelsWorkspaceLayout {
  mobileContextPlacement: 'after-intro'
  actionPlacement: 'within-status-panel'
  feedbackPlacement: 'within-status-panel'
}

const ARCHETYPE_CONFIG: Record<PageArchetype, PageArchetypeConfig> = {
  auth: {
    name: 'auth',
    introStyle: 'stage',
    stageMode: 'split',
    headerSurface: 'stage',
    primaryZone: 'form',
    secondaryDensity: 'narrative',
    surfaceTone: 'refined',
    effectProfile: 'subtle',
    mobile: {
      stageCompression: 'condense',
      primaryFirst: true,
    },
  },
  dashboard: {
    name: 'dashboard',
    introStyle: 'compact',
    stageMode: 'inline',
    headerSurface: 'panel',
    primaryZone: 'content',
    secondaryDensity: 'balanced',
    surfaceTone: 'refined',
    effectProfile: 'minimal',
    mobile: {
      stageCompression: 'condense',
      primaryFirst: true,
    },
  },
  workspace: {
    name: 'workspace',
    introStyle: 'compact',
    stageMode: 'none',
    headerSurface: 'panel',
    primaryZone: 'task',
    secondaryDensity: 'summary-first',
    surfaceTone: 'neutral',
    effectProfile: 'none',
    mobile: {
      stageCompression: 'hide',
      primaryFirst: true,
    },
  },
  detail: {
    name: 'detail',
    introStyle: 'compact',
    stageMode: 'none',
    headerSurface: 'plain',
    primaryZone: 'content',
    secondaryDensity: 'balanced',
    surfaceTone: 'quiet',
    effectProfile: 'none',
    mobile: {
      stageCompression: 'hide',
      primaryFirst: true,
    },
  },
  settings: {
    name: 'settings',
    introStyle: 'compact',
    stageMode: 'none',
    headerSurface: 'plain',
    primaryZone: 'content',
    secondaryDensity: 'balanced',
    surfaceTone: 'quiet',
    effectProfile: 'none',
    mobile: {
      stageCompression: 'hide',
      primaryFirst: true,
    },
  },
}

export function resolvePageArchetype(name: PageArchetype | string | undefined): PageArchetypeConfig {
  if (!name) {
    return ARCHETYPE_CONFIG.settings
  }

  if (name in ARCHETYPE_CONFIG) {
    return ARCHETYPE_CONFIG[name as PageArchetype]
  }

  return ARCHETYPE_CONFIG.settings
}

export function describePageRegions(name: PageArchetype | string | undefined): PageArchetypeRegions {
  const archetype = resolvePageArchetype(name)

  switch (archetype.name) {
    case 'auth':
      return {
        introAlignment: 'start',
        contentLayout: 'split',
        secondaryPlacement: 'after',
        stageEmphasis: 'high',
      }
    case 'workspace':
      return {
        introAlignment: 'between',
        contentLayout: 'split',
        secondaryPlacement: 'aside',
        stageEmphasis: 'low',
      }
    case 'dashboard':
      return {
        introAlignment: 'between',
        contentLayout: 'stack',
        secondaryPlacement: 'after',
        stageEmphasis: 'medium',
      }
    case 'detail':
    case 'settings':
    default:
      return {
        introAlignment: 'start',
        contentLayout: 'stack',
        secondaryPlacement: 'after',
        stageEmphasis: 'low',
      }
  }
}

export function describeDashboardShellLayout(): DashboardShellLayout {
  return {
    mobileRailPlacement: 'after-content',
    desktopAlignment: 'start',
  }
}

export function describeReportShellLayout(): ReportShellLayout {
  return {
    mobileToolbarPlacement: 'after-intro',
    mobileRailPlacement: 'after-content',
    desktopContentBalance: 'lead-first',
  }
}

export function describeBillingReportLayout(): BillingReportLayout {
  return {
    leadSequence: 'summary-then-trend',
    mobileContextPlacement: 'after-lead',
    mobileDetailPlacement: 'after-context',
  }
}

export function describeLogsWorkbenchLayout(): LogsWorkbenchLayout {
  return {
    mobileToolbarPlacement: 'after-intro',
    desktopToolbarAlignment: 'between',
    filterPlacement: 'within-panel',
  }
}

export function describeAccountsWorkspaceLayout(): AccountsWorkspaceLayout {
  return {
    mobileToolbarPlacement: 'after-intro',
    mobileFiltersPlacement: 'after-toolbar',
    batchActionsPlacement: 'with-filters',
  }
}

export function describeModelsWorkspaceLayout(): ModelsWorkspaceLayout {
  return {
    mobileContextPlacement: 'after-intro',
    actionPlacement: 'within-status-panel',
    feedbackPlacement: 'within-status-panel',
  }
}
