import type { TFunction } from 'i18next'

import type {
  OAuthAccountStatusResponse,
  OAuthRateLimitRefreshJobSummary,
  OAuthRateLimitWindow,
  UpstreamAccount,
} from '@/api/accounts'
import { formatRelativeTime } from '@/lib/time'

import {
  MAX_RECENT_IMPORT_JOBS,
  PLAN_UNKNOWN_VALUE,
  RATE_LIMIT_BUCKET_ORDER,
  RECENT_IMPORT_JOBS_STORAGE_KEY,
  SESSION_MODES,
  type CredentialKindShort,
  type RateLimitBucket,
  type RateLimitDisplay,
} from './types'

export function isSessionMode(mode: string) {
  return SESSION_MODES.has(mode)
}

export function clampPercent(value: number | undefined) {
  if (typeof value !== 'number' || Number.isNaN(value)) {
    return 0
  }
  return Math.min(100, Math.max(0, value))
}

function toRemainingPercent(usedPercent: number | undefined) {
  return clampPercent(100 - clampPercent(usedPercent))
}

function pad2(value: number) {
  return String(value).padStart(2, '0')
}

export function formatAbsoluteDateTime(value: string | Date) {
  const date = value instanceof Date ? value : new Date(value)
  if (Number.isNaN(date.getTime())) {
    return '-'
  }
  return `${date.getFullYear()}-${pad2(date.getMonth() + 1)}-${pad2(date.getDate())} ${pad2(date.getHours())}:${pad2(date.getMinutes())}`
}

function normalizeRelativeForReset(relative: string, locale: string) {
  const lowered = locale.toLowerCase()
  if (lowered.startsWith('zh-tw') || lowered.startsWith('zh-hk') || lowered.startsWith('zh-hant')) {
    return relative.replace(/\s+/g, '').replace(/內$/, '後')
  }
  if (lowered.startsWith('zh')) {
    return relative.replace(/\s+/g, '').replace(/内$/, '后')
  }
  return relative
}

export function resolveCredentialKindShort(
  kind?: OAuthAccountStatusResponse['credential_kind'],
): CredentialKindShort {
  if (kind === 'refresh_rotatable') {
    return 'rt'
  }
  if (kind === 'one_time_access_token') {
    return 'at'
  }
  return 'unknown'
}

export function normalizePlanValue(raw?: string) {
  const value = raw?.trim().toLowerCase()
  if (!value) {
    return PLAN_UNKNOWN_VALUE
  }
  return value
}

export function isRateLimitRefreshJobTerminal(status?: OAuthRateLimitRefreshJobSummary['status']) {
  return status === 'completed' || status === 'failed' || status === 'cancelled'
}

export function extractRateLimitDisplays(status?: OAuthAccountStatusResponse): RateLimitDisplay[] {
  const snapshots = status?.rate_limits ?? []
  if (snapshots.length === 0) {
    return []
  }

  let fiveHours: OAuthRateLimitWindow | undefined
  let oneWeek: OAuthRateLimitWindow | undefined
  let github: OAuthRateLimitWindow | undefined

  const resolveBucket = (
    window: OAuthRateLimitWindow,
    fallback: Exclude<RateLimitBucket, 'github'>,
  ): Exclude<RateLimitBucket, 'github'> => {
    if (typeof window.window_minutes === 'number') {
      if (window.window_minutes >= 6 * 24 * 60) {
        return 'one_week'
      }
      if (window.window_minutes <= 12 * 60) {
        return 'five_hours'
      }
    }
    return fallback
  }

  const assignWindow = (
    window: OAuthRateLimitWindow,
    fallback: Exclude<RateLimitBucket, 'github'>,
  ) => {
    const bucket = resolveBucket(window, fallback)
    if (bucket === 'five_hours' && !fiveHours) {
      fiveHours = window
      return
    }
    if (bucket === 'one_week' && !oneWeek) {
      oneWeek = window
    }
  }

  for (const snapshot of snapshots) {
    const marker = `${snapshot.limit_id ?? ''} ${snapshot.limit_name ?? ''}`.toLowerCase()
    const isGithubLimit = marker.includes('github')

    if (isGithubLimit && !github) {
      github = snapshot.primary ?? snapshot.secondary
      continue
    }

    if (snapshot.primary) {
      assignWindow(snapshot.primary, 'five_hours')
    }

    if (snapshot.secondary) {
      assignWindow(snapshot.secondary, 'one_week')
    }
  }

  const items: RateLimitDisplay[] = []
  if (fiveHours) {
    items.push({
      bucket: 'five_hours',
      remainingPercent: toRemainingPercent(fiveHours.used_percent),
      resetsAt: fiveHours.resets_at,
    })
  }
  if (oneWeek) {
    items.push({
      bucket: 'one_week',
      remainingPercent: toRemainingPercent(oneWeek.used_percent),
      resetsAt: oneWeek.resets_at,
    })
  }
  if (github) {
    items.push({
      bucket: 'github',
      remainingPercent: toRemainingPercent(github.used_percent),
      resetsAt: github.resets_at,
    })
  }

  return items
}

export function rateLimitSortValue(status?: OAuthAccountStatusResponse) {
  const displays = extractRateLimitDisplays(status)
  if (displays.length === 0) {
    return -1
  }
  return displays.reduce((min, item) => Math.min(min, item.remainingPercent), 100)
}

export function sortRateLimitDisplays(items: RateLimitDisplay[]) {
  const orderMap = new Map(RATE_LIMIT_BUCKET_ORDER.map((bucket, index) => [bucket, index]))
  return [...items].sort((left, right) => {
    if (left.remainingPercent !== right.remainingPercent) {
      return left.remainingPercent - right.remainingPercent
    }
    return (orderMap.get(left.bucket) ?? 99) - (orderMap.get(right.bucket) ?? 99)
  })
}

export function statusSortValue(status?: OAuthAccountStatusResponse) {
  if (!status) {
    return 0
  }
  if (status.last_refresh_status === 'failed') {
    return 1
  }
  if (status.last_refresh_status === 'never') {
    return 2
  }
  return 3
}

export function addRecentImportJobId(jobId: string) {
  const normalized = jobId.trim()
  if (!normalized) {
    return
  }
  try {
    const raw = localStorage.getItem(RECENT_IMPORT_JOBS_STORAGE_KEY)
    const parsed = raw ? JSON.parse(raw) : []
    const list = Array.isArray(parsed) ? parsed.filter((item) => typeof item === 'string') : []
    const next = [normalized, ...list.filter((item) => item !== normalized)].slice(0, MAX_RECENT_IMPORT_JOBS)
    localStorage.setItem(RECENT_IMPORT_JOBS_STORAGE_KEY, JSON.stringify(next))
  } catch {
    // ignore storage failures
  }
}

export function matchesAccountSearch(
  account: UpstreamAccount,
  keyword: string,
  oauthStatusMap: Map<string, OAuthAccountStatusResponse>,
) {
  const status = oauthStatusMap.get(account.id)
  const values = [
    status?.email,
    account.label,
    account.id,
    account.base_url,
    account.chatgpt_account_id,
    status?.chatgpt_plan_type,
    status?.source_type,
    status?.last_refresh_error,
    status?.last_refresh_error_code,
  ]

  return values.some((item) => item?.toLowerCase().includes(keyword))
}

export function bucketLabel(bucket: RateLimitBucket, t: TFunction) {
  if (bucket === 'five_hours') {
    return t('accounts.rateLimits.labels.fiveHours')
  }
  if (bucket === 'one_week') {
    return t('accounts.rateLimits.labels.oneWeek')
  }
  return t('accounts.rateLimits.labels.github')
}

export function bucketBarClass(bucket: RateLimitBucket) {
  void bucket
  return 'bg-success'
}

export function formatRateLimitResetText({
  resetsAt,
  locale,
  t,
}: {
  resetsAt?: string
  locale: string
  t: TFunction
}) {
  if (!resetsAt) {
    return t('accounts.rateLimits.noReset')
  }
  const date = new Date(resetsAt)
  if (Number.isNaN(date.getTime())) {
    return t('accounts.rateLimits.noReset')
  }
  const absolute = formatAbsoluteDateTime(date)
  const relativeRaw = formatRelativeTime(date, locale, true)
  const relative = normalizeRelativeForReset(relativeRaw, locale)
  return t('accounts.rateLimits.resetAt', {
    absolute,
    relative,
    defaultValue: `${absolute} (${relative}) reset`,
  })
}

export function getModeLabel(mode: string, t: TFunction) {
  if (mode === 'chat_gpt_session') return t('accounts.mode.chatgptSession')
  if (mode === 'codex_oauth') return t('accounts.mode.codexOauth')
  if (mode === 'open_ai_api_key') return t('accounts.mode.apiKey')
  return t('accounts.mode.unknown')
}

export function getCredentialKindShortLabel(
  kind: OAuthAccountStatusResponse['credential_kind'] | undefined,
  t: TFunction,
) {
  const short = resolveCredentialKindShort(kind)
  if (short === 'rt') return t('accounts.oauth.kindShort.refreshRotatable', { defaultValue: 'RT' })
  if (short === 'at') return t('accounts.oauth.kindShort.oneTime', { defaultValue: 'AT' })
  return t('accounts.oauth.kindShort.unknown', { defaultValue: 'Unknown' })
}

export function getCredentialKindLabel(
  kind: OAuthAccountStatusResponse['credential_kind'] | undefined,
  t: TFunction,
) {
  if (kind === 'refresh_rotatable') {
    return t('accounts.oauth.kind.refreshRotatable', { defaultValue: 'Refresh-token account' })
  }
  if (kind === 'one_time_access_token') {
    return t('accounts.oauth.kind.oneTime', { defaultValue: 'One-time access-token account' })
  }
  return t('accounts.oauth.kind.unknown', { defaultValue: 'Unknown credential type' })
}

export function getPlanLabel(plan: string | undefined, t: TFunction) {
  const value = normalizePlanValue(plan)
  if (value === PLAN_UNKNOWN_VALUE) {
    return t('accounts.filters.planUnknown', { defaultValue: 'Not Reported' })
  }
  return value
}

export function getRefreshStatusLabel(
  status: OAuthAccountStatusResponse['last_refresh_status'],
  t: TFunction,
) {
  if (status === 'ok') return t('accounts.oauth.status.ok')
  if (status === 'failed') return t('accounts.oauth.status.failed')
  return t('accounts.oauth.status.never')
}

export function getAuthProviderLabel(
  provider: OAuthAccountStatusResponse['auth_provider'],
  t: TFunction,
) {
  if (provider === 'oauth_refresh_token') {
    return t('accounts.oauth.provider.refreshToken', { defaultValue: 'Refresh token' })
  }
  return t('accounts.oauth.provider.legacyBearer', { defaultValue: 'Legacy bearer token' })
}

export function getSourceTypeLabel(sourceType: string | undefined, t: TFunction) {
  const normalized = sourceType?.trim().toLowerCase()
  if (!normalized) {
    return null
  }
  if (normalized === 'codex') {
    return t('accounts.oauth.sourceType.codex', { defaultValue: 'Codex' })
  }
  return t('accounts.oauth.sourceType.unknown', { defaultValue: 'Unknown source' })
}
