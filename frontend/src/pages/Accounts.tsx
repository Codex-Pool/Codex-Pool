import { useCallback, useEffect, useMemo, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  Button,
  Card,
  CardBody,
  CardHeader,
  Chip,
  Divider,
  Dropdown,
  DropdownItem,
  DropdownMenu,
  DropdownTrigger,
  Input,
  Modal,
  ModalBody,
  ModalContent,
  ModalFooter,
  ModalHeader,
  Pagination,
  Progress,
  Select,
  SelectItem,
  Spinner,
  Table,
  TableBody,
  TableCell,
  TableColumn,
  TableHeader,
  TableRow,
  Textarea,
  type Selection,
  type SortDescriptor,
} from '@heroui/react'
import {
  Archive,
  Eye,
  MoreHorizontal,
  RefreshCcw,
  RotateCcw,
  Search,
  Trash2,
  X,
} from 'lucide-react'
import { useTranslation } from 'react-i18next'

import {
  accountsApi,
  accountPoolApi,
  type AccountPoolAction,
  type AccountPoolOperatorState,
  type AccountPoolReasonClass,
  type AccountPoolRecord,
  type AccountPoolRecordScope,
  type AccountPoolResponsesTestResponse,
} from '@/api/accounts'
import {
  extractApiErrorCodeFrom,
  extractApiErrorMessageFrom,
  extractApiErrorStatusFrom,
} from '@/api/client'
import { localizeApiErrorDisplay, localizeOAuthErrorCodeDisplay } from '@/api/errorI18n'
import { modelsApi } from '@/api/models'
import { SignalHeatmapCanvas, SignalHeatmapMini } from '@/features/accounts/signal-heatmap-canvas'
import {
  sortAccountPoolRecords,
  type AccountPoolSortColumn,
  type AccountPoolSortDescriptor,
} from '@/features/accounts/account-pool-sorting'
import {
  DockedPageIntro,
  PageContent,
} from '@/components/layout/page-archetypes'
import {
  bucketLabel,
  extractRateLimitDisplaysFromSnapshots,
  formatAbsoluteDateTime,
  formatRateLimitResetText,
  getPlanLabel,
  normalizePlanValue,
} from '@/features/accounts/utils'
import { PLAN_UNKNOWN_VALUE } from '@/features/accounts/types'
import { Checkbox } from '@/components/ui/checkbox'
import { formatRelativeTime } from '@/lib/time'
import { notify } from '@/lib/notification'
import { cn } from '@/lib/utils'

type StateFilter = 'all' | AccountPoolOperatorState
type ScopeFilter = 'all' | AccountPoolRecordScope
type ReasonClassFilter = 'all' | AccountPoolReasonClass
type PlanFilter = 'all' | string
type ResponsesTestMessageRole = 'user' | 'assistant'

interface AccountPoolResponsesTestMessage {
  id: string
  role: ResponsesTestMessageRole
  text: string
  meta?: AccountPoolResponsesTestResponse
}

interface AccountPoolResponsesTestErrorState {
  label: string
  rawMessage: string | null
  code: string | null
  status: number | null
}

const TABLE_PAGE_SIZE_OPTIONS = [10, 20, 50]
const ACCOUNT_POOL_SORTABLE_COLUMNS = new Set<AccountPoolSortColumn>([
  'account',
  'operationalStatus',
  'quota',
  'recentSignal',
])

function normalizeSelection(selection: Selection) {
  if (selection === 'all') {
    return ''
  }

  const [first] = Array.from(selection)
  return first === undefined ? '' : String(first)
}

function normalizeAccountPoolSortDescriptor(
  descriptor: SortDescriptor | null,
): AccountPoolSortDescriptor | null {
  if (!descriptor) {
    return null
  }

  const column = String(descriptor.column) as AccountPoolSortColumn
  if (!ACCOUNT_POOL_SORTABLE_COLUMNS.has(column)) {
    return null
  }

  return {
    column,
    direction: descriptor.direction === 'descending' ? 'descending' : 'ascending',
  }
}

function formatDateTime(value?: string) {
  if (!value) {
    return '-'
  }
  return formatAbsoluteDateTime(value)
}

function resolveResponsesTestUsageLines(
  response: AccountPoolResponsesTestResponse | undefined,
  t: ReturnType<typeof useTranslation>['t'],
) {
  if (!response?.usage) {
    return []
  }

  const items = [
    {
      key: 'input_tokens',
      label: t('accountPool.detail.test.inputTokens'),
      value: response.usage.input_tokens,
    },
    {
      key: 'cached_input_tokens',
      label: t('accountPool.detail.test.cachedInputTokens'),
      value: response.usage.cached_input_tokens,
    },
    {
      key: 'output_tokens',
      label: t('accountPool.detail.test.outputTokens'),
      value: response.usage.output_tokens,
    },
    {
      key: 'reasoning_tokens',
      label: t('accountPool.detail.test.reasoningTokens'),
      value: response.usage.reasoning_tokens,
    },
  ]

  return items.filter((item) => typeof item.value === 'number')
}

function buildResponsesTestErrorState(
  error: unknown,
  t: ReturnType<typeof useTranslation>['t'],
): AccountPoolResponsesTestErrorState {
  const display = localizeApiErrorDisplay(t, error, t('accountPool.messages.actionFailed'))
  const rawMessage = extractApiErrorMessageFrom(error)?.trim() ?? null
  const code = extractApiErrorCodeFrom(error)?.trim() ?? null
  const status = extractApiErrorStatusFrom(error)

  return {
    label: display.label,
    rawMessage: rawMessage || null,
    code: code || null,
    status,
  }
}

function getStateColor(state: AccountPoolOperatorState) {
  switch (state) {
    case 'routable':
      return 'success' as const
    case 'cooling':
      return 'warning' as const
    case 'pending_delete':
      return 'danger' as const
    case 'inventory':
    default:
      return 'default' as const
  }
}

function getStateLabel(state: AccountPoolOperatorState, t: ReturnType<typeof useTranslation>['t']) {
  switch (state) {
    case 'inventory':
      return t('accountPool.state.inventory')
    case 'routable':
      return t('accountPool.state.routable')
    case 'cooling':
      return t('accountPool.state.cooling')
    case 'pending_delete':
      return t('accountPool.state.pendingDelete')
    default:
      return t('accountPool.state.unknown')
  }
}

function getPlanChipColor(plan: string | undefined) {
  switch (normalizePlanValue(plan)) {
    case 'enterprise':
    case 'business':
      return 'success' as const
    case 'pro':
      return 'warning' as const
    case 'plus':
      return 'primary' as const
    case 'team':
      return 'secondary' as const
    case 'free':
    case PLAN_UNKNOWN_VALUE:
    default:
      return 'default' as const
  }
}

function getReasonColor(reasonClass: AccountPoolReasonClass) {
  switch (reasonClass) {
    case 'healthy':
      return 'success' as const
    case 'quota':
      return 'warning' as const
    case 'fatal':
      return 'danger' as const
    case 'transient':
      return 'primary' as const
    case 'admin':
    default:
      return 'default' as const
  }
}

function getReasonLabel(
  reasonClass: AccountPoolReasonClass,
  t: ReturnType<typeof useTranslation>['t'],
) {
  switch (reasonClass) {
    case 'healthy':
      return t('accountPool.reasonClass.healthy')
    case 'quota':
      return t('accountPool.reasonClass.quota')
    case 'fatal':
      return t('accountPool.reasonClass.fatal')
    case 'transient':
      return t('accountPool.reasonClass.transient')
    case 'admin':
      return t('accountPool.reasonClass.admin')
    default:
      return t('accountPool.reasonClass.unknown')
  }
}

function getScopeLabel(
  scope: AccountPoolRecordScope,
  t: ReturnType<typeof useTranslation>['t'],
) {
  return scope === 'runtime'
    ? t('accountPool.scope.runtime')
    : t('accountPool.scope.inventory')
}

function getModeLabel(mode: AccountPoolRecord['mode'], t: ReturnType<typeof useTranslation>['t']) {
  switch (mode) {
    case 'chat_gpt_session':
      return t('accounts.mode.chatgptSession')
    case 'codex_oauth':
      return t('accounts.mode.codexOauth')
    case 'open_ai_api_key':
      return t('accounts.mode.apiKey')
    default:
      return t('accounts.mode.unknown')
  }
}

function getAuthProviderLabel(
  provider: AccountPoolRecord['auth_provider'],
  t: ReturnType<typeof useTranslation>['t'],
) {
  switch (provider) {
    case 'legacy_bearer':
      return t('accounts.oauth.provider.legacyBearer')
    case 'oauth_refresh_token':
      return t('accounts.oauth.provider.refreshToken')
    default:
      return t('accounts.oauth.provider.unknown')
  }
}

function getCredentialKindLabel(
  credentialKind: AccountPoolRecord['credential_kind'],
  t: ReturnType<typeof useTranslation>['t'],
) {
  switch (credentialKind) {
    case 'refresh_rotatable':
      return t('accounts.oauth.kind.refreshRotatable')
    case 'one_time_access_token':
      return t('accounts.oauth.kind.oneTime')
    default:
      return t('accounts.oauth.kind.unknown')
  }
}

function getRefreshStateLabel(
  refreshState: AccountPoolRecord['refresh_credential_state'],
  t: ReturnType<typeof useTranslation>['t'],
) {
  switch (refreshState) {
    case 'healthy':
      return t('accountPool.refreshState.healthy')
    case 'degraded':
      return t('accountPool.refreshState.degraded')
    case 'missing':
      return t('accountPool.refreshState.missing')
    case 'invalid':
      return t('accountPool.refreshState.invalid')
    default:
      return t('accountPool.refreshState.unknown')
  }
}

function getHealthFreshnessLabel(
  freshness: AccountPoolRecord['health_freshness'],
  t: ReturnType<typeof useTranslation>['t'],
) {
  switch (freshness) {
    case 'fresh':
      return t('accountPool.healthFreshness.fresh')
    case 'stale':
      return t('accountPool.healthFreshness.stale')
    default:
      return t('accountPool.healthFreshness.unknown')
  }
}

function getProbeOutcomeLabel(
  outcome: AccountPoolRecord['last_probe_outcome'],
  t: ReturnType<typeof useTranslation>['t'],
) {
  switch (outcome) {
    case 'ok':
      return t('accountPool.probeOutcome.ok')
    case 'quota':
      return t('accountPool.probeOutcome.quota')
    case 'transient':
      return t('accountPool.probeOutcome.transient')
    case 'fatal':
      return t('accountPool.probeOutcome.fatal')
    default:
      return t('accountPool.probeOutcome.unknown')
  }
}

function getSignalSourceLabel(
  source: AccountPoolRecord['last_signal_source'],
  t: ReturnType<typeof useTranslation>['t'],
) {
  switch (source) {
    case 'active':
      return t('accountPool.signalSource.active')
    case 'passive':
      return t('accountPool.signalSource.passive')
    default:
      return t('accountPool.signalSource.unknown')
  }
}

function getSourceTypeLabel(
  sourceType: AccountPoolRecord['source_type'],
  t: ReturnType<typeof useTranslation>['t'],
) {
  switch (sourceType) {
    case 'codex':
      return t('accounts.oauth.sourceType.codex')
    default:
      return t('accounts.oauth.sourceType.unknown')
  }
}

function getReasonCodeLabel(
  reasonCode: string | undefined,
  t: ReturnType<typeof useTranslation>['t'],
) {
  const normalized = (reasonCode ?? '').trim().toLowerCase()
  if (!normalized) {
    return t('accountPool.reasonCode.none')
  }

  switch (normalized) {
    case 'token_invalidated':
      return t('accountPool.reasonCode.tokenInvalidated')
    case 'account_deactivated':
      return t('accountPool.reasonCode.accountDeactivated')
    case 'invalid_refresh_token':
      return t('accountPool.reasonCode.invalidRefreshToken')
    case 'refresh_token_revoked':
      return t('accountPool.reasonCode.refreshTokenRevoked')
    case 'refresh_token_reused':
      return t('accountPool.reasonCode.refreshTokenReused')
    case 'rate_limited':
      return t('accountPool.reasonCode.rateLimited')
    case 'quota_exhausted':
      return t('accountPool.reasonCode.quotaExhausted')
    case 'upstream_unavailable':
      return t('accountPool.reasonCode.upstreamUnavailable')
    case 'transport_error':
      return t('accountPool.reasonCode.transportError')
    case 'overloaded':
      return t('accountPool.reasonCode.overloaded')
    case 'operator_retired_invalid_refresh_token':
      return t('accountPool.reasonCode.operatorRetiredInvalidRefreshToken')
    default: {
      const display = localizeOAuthErrorCodeDisplay(t, normalized)
      if (display.label && display.label !== t('errors.common.failed')) {
        return display.label
      }
      return t('accountPool.reasonCode.unknown')
    }
  }
}

function hasReasonCode(reasonCode: string | undefined) {
  return Boolean((reasonCode ?? '').trim())
}

function buildUsageRows(
  record: AccountPoolRecord,
  t: ReturnType<typeof useTranslation>['t'],
) {
  return extractRateLimitDisplaysFromSnapshots(record.rate_limits).map((item) => {
    const remainingPercent = Math.max(0, Math.min(100, item.remainingPercent))

    return {
      key: item.bucket,
      bucketLabel: bucketLabel(item.bucket, t),
      compactLabel:
        item.bucket === 'five_hours'
          ? t('accountPool.rateLimits.fiveHoursShort')
          : item.bucket === 'one_week'
            ? t('accountPool.rateLimits.oneWeekShort')
            : t('accountPool.rateLimits.githubShort'),
      remainingPercent,
      remainingText: `${Math.round(remainingPercent)}%`,
      resetText: formatRateLimitResetText({
        resetsAt: item.resetsAt,
        t,
      }),
    }
  })
}

function getUsageProgressColor(value: number): 'success' | 'warning' | 'danger' {
  if (value <= 20) {
    return 'danger'
  }
  if (value <= 50) {
    return 'warning'
  }
  return 'success'
}

function getRecordLabel(record: AccountPoolRecord) {
  return record.email?.trim() || record.label
}

function getRecordSecondaryLabel(record: AccountPoolRecord) {
  const primary = getRecordLabel(record)
  const email = record.email?.trim()

  if (email && email !== primary) {
    return email
  }

  return null
}

function getRecentSignalDisplay(
  record: AccountPoolRecord,
  t: ReturnType<typeof useTranslation>['t'],
) {
  if (record.last_signal_at) {
    return {
      label: t('accountPool.fields.lastSignalAt'),
      timestamp: formatDateTime(record.last_signal_at),
      detail: getSignalSourceLabel(record.last_signal_source, t),
    }
  }

  if (record.last_probe_at) {
    return {
      label: t('accountPool.fields.lastProbeAt'),
      timestamp: formatDateTime(record.last_probe_at),
      detail: getProbeOutcomeLabel(record.last_probe_outcome, t),
    }
  }

  return {
    label: t('accountPool.fields.updatedAt'),
    timestamp: formatDateTime(record.updated_at),
    detail: t('accountPool.recentSignal.updatedFallback'),
  }
}

function getRecentSignalSummaryText(
  record: AccountPoolRecord,
  language: string,
  t: ReturnType<typeof useTranslation>['t'],
) {
  if (record.last_signal_at) {
    return t('accountPool.recentSignal.summaryWithDetail', {
      relative: formatRelativeTime(record.last_signal_at, language, true),
      detail: getSignalSourceLabel(record.last_signal_source, t),
    })
  }

  if (record.last_probe_at) {
    return t('accountPool.recentSignal.summaryWithDetail', {
      relative: formatRelativeTime(record.last_probe_at, language, true),
      detail: getProbeOutcomeLabel(record.last_probe_outcome, t),
    })
  }

  return t('accountPool.recentSignal.summaryWithDetail', {
    relative: formatRelativeTime(record.updated_at, language, true),
    detail: t('accountPool.recentSignal.updatedFallback'),
  })
}

function getRecentSignalInlineSummaryText(
  record: AccountPoolRecord,
  language: string,
  t: ReturnType<typeof useTranslation>['t'],
) {
  return getRecentSignalSummaryText(record, language, t).replace(/\s*·\s*/g, '·')
}

function getHeatmapActivityLabel(
  heatmap: Pick<NonNullable<AccountPoolRecord['recent_signal_heatmap']>, 'intensity_levels'> | null | undefined,
  t: ReturnType<typeof useTranslation>['t'],
) {
  if (!heatmap || heatmap.intensity_levels.length === 0) {
    return t('accountPool.recentSignal.noHeatmap')
  }

  const maxIntensity = Math.max(...heatmap.intensity_levels, 0)
  const nonZeroBuckets = heatmap.intensity_levels.filter((value) => value > 0).length
  if (maxIntensity === 0) {
    return t('accountPool.recentSignal.silent')
  }
  if (maxIntensity >= 3 || nonZeroBuckets >= 24) {
    return t('accountPool.recentSignal.busy')
  }
  if (nonZeroBuckets <= 8) {
    return t('accountPool.recentSignal.sparse')
  }
  return t('accountPool.recentSignal.active')
}

function matchesAccountPoolSearch(record: AccountPoolRecord, keyword: string) {
  const values = [
    record.email,
    record.label,
    record.chatgpt_account_id,
    record.chatgpt_plan_type,
    record.source_type,
    record.reason_code,
    record.mode,
    record.auth_provider,
    record.record_scope,
    record.operator_state,
  ]

  return values.some((value) => value?.toLowerCase().includes(keyword))
}

function canRunAccountPoolAction(record: AccountPoolRecord, action: AccountPoolAction) {
  switch (action) {
    case 'delete':
      return true
    case 'restore':
      return record.record_scope === 'inventory'
    case 'reprobe':
      return true // 测活不依赖认证方式，所有账号均可触发
  }
}

/** 冷却账号解冻倒计时：每分钟更新一次，显示"Xh Ym 后解冻" */
function CoolingCountdown({
  nextActionAt,
  t,
}: {
  nextActionAt?: string
  t: ReturnType<typeof useTranslation>['t']
}) {
  const [nowMs, setNowMs] = useState(() => Date.now())

  useEffect(() => {
    const id = window.setInterval(() => setNowMs(Date.now()), 60_000)
    return () => window.clearInterval(id)
  }, [])

  if (!nextActionAt) return null
  const ms = new Date(nextActionAt).getTime() - nowMs
  if (ms <= 0) {
    return (
      <span className="text-xs text-warning-600 tabular-nums">
        {t('accountPool.cooling.imminent', { defaultValue: '即将恢复' })}
      </span>
    )
  }

  const hours = Math.floor(ms / 3_600_000)
  const mins = Math.floor((ms % 3_600_000) / 60_000)
  const label = t('accountPool.cooling.thawIn', {
    defaultValue: hours > 0 ? '{{hours}}h {{minutes}}m 后解冻' : '{{minutes}}m 后解冻',
    hours,
    minutes: mins,
  })
  return <span className="text-xs tabular-nums text-warning-600">{label}</span>
}

export default function Accounts() {
  const { t, i18n } = useTranslation()
  const queryClient = useQueryClient()
  const [stateFilter, setStateFilter] = useState<StateFilter>('all')
  const [scopeFilter, setScopeFilter] = useState<ScopeFilter>('all')
  const [reasonClassFilter, setReasonClassFilter] = useState<ReasonClassFilter>('all')
  const [planFilter, setPlanFilter] = useState<PlanFilter>('all')
  const [searchValue, setSearchValue] = useState('')
  const [rowsPerPage, setRowsPerPage] = useState(20)
  const [currentPage, setCurrentPage] = useState(1)
  const [sortDescriptor, setSortDescriptor] = useState<SortDescriptor | null>(null)
  const [selectedRecordId, setSelectedRecordId] = useState<string | null>(null)
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set())
  const [batchDeletePending, setBatchDeletePending] = useState(false)
  const [testPrompt, setTestPrompt] = useState('')
  const [selectedTestModel, setSelectedTestModel] = useState('')
  const [testMessages, setTestMessages] = useState<AccountPoolResponsesTestMessage[]>([])
  const [previousResponseId, setPreviousResponseId] = useState<string | null>(null)
  const [testError, setTestError] = useState<AccountPoolResponsesTestErrorState | null>(null)
  /** 乐观反馈：记录 ID → 当前动效状态 */
  const [rowFeedback, setRowFeedback] = useState<Map<string, 'pending' | 'success' | 'error'>>(new Map())

  const setFeedback = useCallback((id: string, state: 'pending' | 'success' | 'error' | null) => {
    setRowFeedback((prev) => {
      const next = new Map(prev)
      if (state === null) next.delete(id)
      else next.set(id, state)
      return next
    })
  }, [])

  const resetResponsesTestState = useCallback(() => {
    setTestPrompt('')
    setSelectedTestModel('')
    setTestMessages([])
    setPreviousResponseId(null)
    setTestError(null)
  }, [])

  const openRecord = useCallback((id: string) => setSelectedRecordId(id), [])
  const closeRecord = useCallback(() => {
    resetResponsesTestState()
    setSelectedRecordId(null)
  }, [resetResponsesTestState])
  const [deleteTarget, setDeleteTarget] = useState<AccountPoolRecord | null>(null)
  const locale = i18n.resolvedLanguage ?? i18n.language

  const { data: summary, isLoading: isSummaryLoading, refetch: refetchSummary } = useQuery({
    queryKey: ['accountPoolSummary'],
    queryFn: accountPoolApi.getSummary,
    staleTime: 60_000,
    refetchInterval: 60_000,
    refetchOnWindowFocus: 'always',
  })

  const { data: records = [], isLoading: isRecordsLoading, refetch: refetchRecords } = useQuery({
    queryKey: ['accountPoolRecords'],
    queryFn: accountPoolApi.listRecords,
    staleTime: 60_000,
    refetchInterval: 60_000,
    refetchOnWindowFocus: 'always',
  })

  const { data: selectedRecord, isFetching: isFetchingSelectedRecord } = useQuery({
    queryKey: ['accountPoolRecord', selectedRecordId],
    queryFn: () => accountPoolApi.getRecord(selectedRecordId!),
    enabled: Boolean(selectedRecordId),
    staleTime: 15_000,
    refetchOnWindowFocus: 'always',
  })

  const { data: selectedSignalHeatmap, isFetching: isFetchingSelectedSignalHeatmap } = useQuery({
    queryKey: ['accountPoolRecordSignalHeatmap', selectedRecordId],
    queryFn: () => accountPoolApi.getSignalHeatmap(selectedRecordId!),
    enabled: Boolean(selectedRecordId),
    staleTime: 15_000,
    refetchOnWindowFocus: 'always',
  })

  const isRuntimeRecord = selectedRecord?.record_scope === 'runtime'

  const { data: selectedRuntimeOAuthStatus } = useQuery({
    queryKey: ['accountPoolRecordOAuthStatus', selectedRecordId],
    queryFn: () => accountsApi.getOAuthStatus(selectedRecordId!),
    enabled: isRuntimeRecord,
    staleTime: 60_000,
    refetchOnWindowFocus: 'always',
  })

  const { data: availableModels } = useQuery({
    queryKey: ['accountPoolResponsesTestModels'],
    queryFn: modelsApi.listModels,
    enabled: isRuntimeRecord,
    staleTime: 60_000,
    refetchOnWindowFocus: 'always',
  })

  const availablePlanFilters = useMemo(() => {
    const values = Array.from(new Set(records.map((record) => normalizePlanValue(record.chatgpt_plan_type))))
    const knownValues = values.filter((value) => value !== PLAN_UNKNOWN_VALUE)
    knownValues.sort((left, right) => getPlanLabel(left, t).localeCompare(getPlanLabel(right, t), locale))
    return values.includes(PLAN_UNKNOWN_VALUE)
      ? [...knownValues, PLAN_UNKNOWN_VALUE]
      : knownValues
  }, [locale, records, t])

  const filteredRecords = useMemo(() => {
    const keyword = searchValue.trim().toLowerCase()

    return records.filter((record) => {
      if (stateFilter !== 'all' && record.operator_state !== stateFilter) {
        return false
      }
      if (scopeFilter !== 'all' && record.record_scope !== scopeFilter) {
        return false
      }
      if (reasonClassFilter !== 'all' && record.reason_class !== reasonClassFilter) {
        return false
      }
      if (planFilter !== 'all' && normalizePlanValue(record.chatgpt_plan_type) !== planFilter) {
        return false
      }
      if (keyword && !matchesAccountPoolSearch(record, keyword)) {
        return false
      }
      return true
    })
  }, [planFilter, reasonClassFilter, records, scopeFilter, searchValue, stateFilter])

  const sortedRecords = useMemo(
    () => sortAccountPoolRecords(filteredRecords, normalizeAccountPoolSortDescriptor(sortDescriptor)),
    [filteredRecords, sortDescriptor],
  )

  const totalPages = Math.max(1, Math.ceil(sortedRecords.length / rowsPerPage))
  const resolvedPage = Math.min(currentPage, totalPages)
  const paginatedRecords = useMemo(() => {
    const start = (resolvedPage - 1) * rowsPerPage
    return sortedRecords.slice(start, start + rowsPerPage)
  }, [resolvedPage, rowsPerPage, sortedRecords])
  const currentPageRecordIds = useMemo(() => paginatedRecords.map((record) => record.id), [paginatedRecords])
  const visibleRangeStart = sortedRecords.length === 0 ? 0 : (resolvedPage - 1) * rowsPerPage + 1
  const visibleRangeEnd =
    sortedRecords.length === 0 ? 0 : Math.min(sortedRecords.length, resolvedPage * rowsPerPage)

  // ── Selection: 过滤结果的 ID 集合，用于全选和清理无效选中 ──
  const filteredRecordIds = useMemo(() => sortedRecords.map((r) => r.id), [sortedRecords])
  const selectedRecords = useMemo(
    () => sortedRecords.filter((record) => selectedIds.has(record.id)),
    [selectedIds, sortedRecords],
  )
  const allCurrentPageSelected =
    currentPageRecordIds.length > 0
    && currentPageRecordIds.every((id) => selectedIds.has(id))
  const someCurrentPageSelected =
    !allCurrentPageSelected
    && currentPageRecordIds.some((id) => selectedIds.has(id))
  const allFilteredSelected =
    filteredRecordIds.length > 0
    && filteredRecordIds.every((id) => selectedIds.has(id))
  const canSelectAcrossPages = filteredRecordIds.length > currentPageRecordIds.length
  const canBatchRestore =
    selectedRecords.length > 0
    && selectedRecords.every((record) => canRunAccountPoolAction(record, 'restore'))
  useEffect(() => {
    const validSet = new Set(filteredRecordIds)
    setSelectedIds((prev) => {
      const cleaned = new Set([...prev].filter((id) => validSet.has(id)))
      return cleaned.size === prev.size ? prev : cleaned
    })
  }, [filteredRecordIds])

  const invalidateQueries = useCallback(async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ['accountPoolSummary'] }),
      queryClient.invalidateQueries({ queryKey: ['accountPoolRecords'] }),
      queryClient.invalidateQueries({ queryKey: ['accountPoolRecord'] }),
      queryClient.invalidateQueries({ queryKey: ['accountPoolRecordSignalHeatmap'] }),
    ])
  }, [queryClient])

  const actionMutation = useMutation({
    mutationFn: ({ action, record }: { action: AccountPoolAction; record: AccountPoolRecord }) =>
      accountPoolApi.runAction(action, [record.id]),
    onSuccess: async (response, variables) => {
      if (variables.action === 'delete') {
        setDeleteTarget(null)
        if (selectedRecordId === variables.record.id) {
          closeRecord()
        }
      }

      await invalidateQueries()

      const failed = response.items.find((item) => !item.ok)
      if (failed?.error) {
        setFeedback(variables.record.id, 'error')
        window.setTimeout(() => setFeedback(variables.record.id, null), 800)
        const errorDisplay = localizeOAuthErrorCodeDisplay(t, failed.error.code)
        notify({
          variant: 'warning',
          title: t('accountPool.messages.actionPartialTitle', {
            action: t(`accountPool.actions.${variables.action}`),
          }),
          description:
            errorDisplay.label !== '-'
              ? errorDisplay.label
              : t('accountPool.messages.actionFailed'),
        })
        return
      }

      setFeedback(variables.record.id, 'success')
      window.setTimeout(() => setFeedback(variables.record.id, null), 900)
      notify({
        variant: 'success',
        title: t('accountPool.messages.actionSuccessTitle', {
          action: t(`accountPool.actions.${variables.action}`),
        }),
        description: t('accountPool.messages.actionSuccessDescription', {
          label: getRecordLabel(variables.record),
        }),
      })
    },
    onError: (error, variables) => {
      setFeedback(variables.record.id, 'error')
      window.setTimeout(() => setFeedback(variables.record.id, null), 800)
      const fallback = t('accountPool.messages.actionFailed')
      notify({
        variant: 'error',
        title: t('accountPool.messages.actionFailedTitle', {
          action: t(`accountPool.actions.${variables.action}`),
        }),
        description: localizeApiErrorDisplay(t, error, fallback).label,
      })
    },
  })

  const batchActionMutation = useMutation({
    mutationFn: ({ action, ids }: { action: AccountPoolAction; ids: string[] }) =>
      accountPoolApi.runAction(action, ids),
    onSuccess: async (response, variables) => {
      setSelectedIds(new Set())
      setBatchDeletePending(false)
      await invalidateQueries()
      const successCount = response.items.filter((item) => item.ok).length
      const failedCount = response.items.filter((item) => !item.ok).length
      if (failedCount === 0) {
        notify({
          variant: 'success',
          title: t('accountPool.messages.batchSuccessTitle', {
            defaultValue: 'Batch {{action}} completed',
            action: t(`accountPool.actions.${variables.action}`),
          }),
          description: t('accountPool.messages.batchSuccessDescription', {
            defaultValue: '{{count}} records processed successfully',
            count: successCount,
          }),
        })
      } else {
        notify({
          variant: 'warning',
          title: t('accountPool.messages.batchPartialTitle', {
            defaultValue: 'Batch {{action}} partially completed',
            action: t(`accountPool.actions.${variables.action}`),
          }),
          description: t('accountPool.messages.batchPartialDescription', {
            defaultValue: '{{success}} succeeded, {{failed}} failed',
            success: successCount,
            failed: failedCount,
          }),
        })
      }
    },
    onError: (_error, variables) => {
      notify({
        variant: 'error',
        title: t('accountPool.messages.batchFailedTitle', {
          defaultValue: 'Batch {{action}} failed',
          action: t(`accountPool.actions.${variables.action}`),
        }),
      })
    },
  })

  const runAccountAction = useCallback(
    (record: AccountPoolRecord, action: AccountPoolAction) => {
      if (action === 'delete') {
        setDeleteTarget(record)
        return
      }

      setFeedback(record.id, 'pending')
      actionMutation.mutate({ action, record })
    },
    [actionMutation, setFeedback],
  )

  const summaryCards = [
    {
      key: 'inventory' as const,
      title: t('accountPool.metrics.inventory'),
      value: summary?.inventory ?? 0,
      description: t('accountPool.metrics.inventoryDesc'),
    },
    {
      key: 'routable' as const,
      title: t('accountPool.metrics.routable'),
      value: summary?.routable ?? 0,
      description: t('accountPool.metrics.routableDesc'),
    },
    {
      key: 'cooling' as const,
      title: t('accountPool.metrics.cooling'),
      value: summary?.cooling ?? 0,
      description: t('accountPool.metrics.coolingDesc'),
    },
    {
      key: 'pending_delete' as const,
      title: t('accountPool.metrics.pendingDelete'),
      value: summary?.pending_delete ?? 0,
      description: t('accountPool.metrics.pendingDeleteDesc'),
    },
  ]

  const reasonCards = [
    { key: 'healthy' as const, title: t('accountPool.reasonClass.healthy'), value: summary?.healthy ?? 0, color: 'success' as const },
    { key: 'quota' as const, title: t('accountPool.reasonClass.quota'), value: summary?.quota ?? 0, color: 'warning' as const },
    { key: 'fatal' as const, title: t('accountPool.reasonClass.fatal'), value: summary?.fatal ?? 0, color: 'danger' as const },
    { key: 'transient' as const, title: t('accountPool.reasonClass.transient'), value: summary?.transient ?? 0, color: 'primary' as const },
    { key: 'admin' as const, title: t('accountPool.reasonClass.admin'), value: summary?.admin ?? 0, color: 'default' as const },
  ]

  const selectedLabel = selectedRecord ? getRecordLabel(selectedRecord) : null
  const testModelOptions = useMemo(() => {
    const seen = new Set<string>()
    const options: string[] = []

    ;(selectedRuntimeOAuthStatus?.supported_models ?? []).forEach((model) => {
      const normalized = model.trim()
      if (!normalized || seen.has(normalized)) {
        return
      }
      seen.add(normalized)
      options.push(normalized)
    })

    ;(availableModels?.data ?? []).forEach((model) => {
      const normalized = model.id.trim()
      if (!normalized || seen.has(normalized) || model.availability_status === 'unavailable') {
        return
      }
      seen.add(normalized)
      options.push(normalized)
    })

    return options
  }, [availableModels?.data, selectedRuntimeOAuthStatus?.supported_models])

  useEffect(() => {
    resetResponsesTestState()
  }, [resetResponsesTestState, selectedRecordId])

  useEffect(() => {
    if (selectedRecord?.record_scope !== 'runtime') {
      setSelectedTestModel('')
      return
    }

    setSelectedTestModel((current) => {
      if (current && testModelOptions.includes(current)) {
        return current
      }
      return testModelOptions[0] ?? ''
    })
  }, [selectedRecord?.record_scope, testModelOptions])

  const testMutation = useMutation({
    mutationFn: (payload: { model: string; input: string; previous_response_id?: string }) => {
      if (!selectedRecordId) {
        throw new Error('missing_selected_record')
      }
      return accountPoolApi.testResponses(selectedRecordId, payload)
    },
    onSuccess: (response, variables) => {
      const assistantText =
        response.assistant_text.trim() || t('accountPool.detail.test.emptyAssistant')

      setTestMessages((current) => [
        ...current,
        {
          id: `user-${crypto.randomUUID()}`,
          role: 'user',
          text: variables.input,
        },
        {
          id: `assistant-${response.response_id}`,
          role: 'assistant',
          text: assistantText,
          meta: response,
        },
      ])
      setPreviousResponseId(response.response_id)
      setTestPrompt('')
      setTestError(null)
    },
    onError: (error) => {
      setTestError(buildResponsesTestErrorState(error, t))
    },
  })

  const clearResponsesConversation = useCallback(() => {
    setTestMessages([])
    setPreviousResponseId(null)
    setTestError(null)
  }, [])

  const togglePageSelection = useCallback((value: boolean) => {
    setSelectedIds((prev) => {
      const next = new Set(prev)
      for (const id of currentPageRecordIds) {
        value ? next.add(id) : next.delete(id)
      }
      return next
    })
  }, [currentPageRecordIds])

  const toggleFilteredSelection = useCallback((value: boolean) => {
    setSelectedIds((prev) => {
      const next = new Set(prev)
      for (const id of filteredRecordIds) {
        value ? next.add(id) : next.delete(id)
      }
      return next
    })
  }, [filteredRecordIds])

  const handleSendResponsesTest = useCallback(() => {
    if (!selectedRecordId || selectedRecord?.record_scope !== 'runtime') {
      return
    }

    const input = testPrompt.trim()
    if (!input || !selectedTestModel) {
      return
    }

    setTestError(null)
    testMutation.mutate({
      model: selectedTestModel,
      input,
      previous_response_id: previousResponseId ?? undefined,
    })
  }, [
    previousResponseId,
    selectedRecord?.record_scope,
    selectedRecordId,
    selectedTestModel,
    testMutation,
    testPrompt,
  ])

  return (
    <PageContent className="space-y-6">
      <DockedPageIntro
        archetype="workspace"
        title={t('nav.accounts')}
        description={t('accountPool.subtitle')}
        actions={(
          <Button
            color="primary"
            isLoading={isSummaryLoading || isRecordsLoading}
            startContent={isSummaryLoading || isRecordsLoading ? undefined : <RefreshCcw className="h-4 w-4" />}
            variant="flat"
            onPress={() => {
              void refetchSummary()
              void refetchRecords()
            }}
          >
            {t('common.refresh')}
          </Button>
        )}
      />

      <div className="grid gap-6 xl:grid-cols-[minmax(0,1.35fr)_minmax(0,0.95fr)]">
        <Card className="border-small border-default-200 bg-content1 shadow-small">
          <CardHeader className="px-5 pb-3 pt-5">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t('accountPool.sections.stateOverviewTitle')}
              </h2>
              <p className="text-sm leading-6 text-default-600">
                {t('accountPool.sections.stateOverviewDescription')}
              </p>
            </div>
          </CardHeader>
          <CardBody className="grid gap-3 px-5 pb-5 pt-1 sm:grid-cols-2">
            {summaryCards.map((card) => (
              <Card
                key={card.title}
                isPressable
                aria-pressed={stateFilter === card.key}
                className={cn(
                  'w-full border px-4 py-4 text-left transition-colors',
                  stateFilter === card.key
                    ? 'border-primary/50 bg-primary/5'
                    : 'border-default-200 bg-content2/55 hover:bg-content2',
                )}
                onPress={() => {
                  setCurrentPage(1)
                  setStateFilter((current) => (current === card.key ? 'all' : card.key))
                }}
              >
                <div className="flex items-center justify-between gap-3">
                  <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                    {card.title}
                  </div>
                  <Chip color={stateFilter === card.key ? 'primary' : 'default'} size="sm" variant="flat">
                    {t('accountPool.metrics.records', { count: card.value })}
                  </Chip>
                </div>
                <div className="mt-3 text-3xl font-semibold tracking-[-0.04em] text-foreground">
                  {card.value}
                </div>
                <p className="mt-1.5 text-xs leading-5 text-default-500">
                  {card.description}
                </p>
              </Card>
            ))}
          </CardBody>
        </Card>

        <Card className="border-small border-default-200 bg-content1 shadow-small">
          <CardHeader className="px-5 pb-3 pt-5">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t('accountPool.sections.reasonOverviewTitle')}
              </h2>
              <p className="text-sm leading-6 text-default-600">
                {t('accountPool.sections.reasonOverviewDescription')}
              </p>
            </div>
          </CardHeader>
          <CardBody className="gap-3 px-5 pb-5 pt-1">
            <div className="grid gap-2 sm:grid-cols-2">
              {reasonCards.map((card) => (
                <Card
                  key={card.title}
                  isPressable
                  aria-pressed={reasonClassFilter === card.key}
                  className={cn(
                    'w-full border px-4 py-2.5 text-left transition-colors',
                    reasonClassFilter === card.key
                      ? 'border-primary/50 bg-primary/5'
                      : 'border-default-200 bg-content2/55 hover:bg-content2',
                  )}
                  onPress={() => {
                    setCurrentPage(1)
                    setReasonClassFilter((current) => (current === card.key ? 'all' : card.key))
                  }}
                >
                  <div className="flex items-center justify-between gap-3">
                    <div className="text-sm font-medium text-foreground">{card.title}</div>
                    <Chip color={reasonClassFilter === card.key ? 'primary' : card.color} size="sm" variant="flat">
                      {card.value}
                    </Chip>
                  </div>
                </Card>
              ))}
            </div>
            <Divider />
            <div className="grid gap-3 sm:grid-cols-2">
              <div className="rounded-large border border-default-200 bg-content2/55 px-4 py-3">
                <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                  {t('accountPool.metrics.totalRecords')}
                </div>
                <div className="mt-2 text-2xl font-semibold tracking-[-0.04em] text-foreground">
                  {summary?.total ?? records.length}
                </div>
              </div>
              <div className="rounded-large border border-default-200 bg-content2/55 px-4 py-3">
                <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                  {t('accountPool.metrics.filteredRecords')}
                </div>
                <div className="mt-2 text-2xl font-semibold tracking-[-0.04em] text-foreground">
                  {filteredRecords.length}
                </div>
              </div>
            </div>
          </CardBody>
        </Card>
      </div>

      <Card className="border-small border-default-200 bg-content1 shadow-small">
        <CardHeader className="flex flex-col items-start gap-4 px-5 pb-3 pt-5">
          <div className="space-y-1">
            <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
              {t('accountPool.sections.recordsTitle')}
            </h2>
            <p className="text-sm leading-6 text-default-600">
              {t('accountPool.sections.recordsDescription')}
            </p>
          </div>

          <div className="flex flex-col gap-4 xl:flex-row xl:items-end xl:justify-between">
            <div className="grid flex-1 gap-3 md:grid-cols-2 xl:grid-cols-5">
              <Input
                aria-label={t('accountPool.searchPlaceholder')}
                className="xl:col-span-1"
                placeholder={t('accountPool.searchPlaceholder')}
                size="sm"
                startContent={<Search className="h-4 w-4 text-default-400" />}
                value={searchValue}
                onValueChange={(value) => {
                  setCurrentPage(1)
                  setSearchValue(value)
                }}
              />

              <Select
                aria-label={t('accountPool.filters.state')}
                selectedKeys={new Set([stateFilter])}
                size="sm"
                onSelectionChange={(selection) => {
                  const nextValue = normalizeSelection(selection)
                  if (!nextValue) {
                    return
                  }
                  setCurrentPage(1)
                  setStateFilter(nextValue as StateFilter)
                }}
              >
                <SelectItem key="all">{t('accountPool.filters.allStates')}</SelectItem>
                <SelectItem key="inventory">{getStateLabel('inventory', t)}</SelectItem>
                <SelectItem key="routable">{getStateLabel('routable', t)}</SelectItem>
                <SelectItem key="cooling">{getStateLabel('cooling', t)}</SelectItem>
                <SelectItem key="pending_delete">{getStateLabel('pending_delete', t)}</SelectItem>
              </Select>

              <Select
                aria-label={t('accountPool.filters.scope')}
                selectedKeys={new Set([scopeFilter])}
                size="sm"
                onSelectionChange={(selection) => {
                  const nextValue = normalizeSelection(selection)
                  if (!nextValue) {
                    return
                  }
                  setCurrentPage(1)
                  setScopeFilter(nextValue as ScopeFilter)
                }}
              >
                <SelectItem key="all">{t('accountPool.filters.allScopes')}</SelectItem>
                <SelectItem key="runtime">{getScopeLabel('runtime', t)}</SelectItem>
                <SelectItem key="inventory">{getScopeLabel('inventory', t)}</SelectItem>
              </Select>

              <Select
                aria-label={t('accountPool.filters.reasonClass')}
                selectedKeys={new Set([reasonClassFilter])}
                size="sm"
                onSelectionChange={(selection) => {
                  const nextValue = normalizeSelection(selection)
                  if (!nextValue) {
                    return
                  }
                  setCurrentPage(1)
                  setReasonClassFilter(nextValue as ReasonClassFilter)
                }}
              >
                <SelectItem key="all">{t('accountPool.filters.allReasons')}</SelectItem>
                <SelectItem key="healthy">{getReasonLabel('healthy', t)}</SelectItem>
                <SelectItem key="quota">{getReasonLabel('quota', t)}</SelectItem>
                <SelectItem key="fatal">{getReasonLabel('fatal', t)}</SelectItem>
                <SelectItem key="transient">{getReasonLabel('transient', t)}</SelectItem>
                <SelectItem key="admin">{getReasonLabel('admin', t)}</SelectItem>
              </Select>

              <Select
                aria-label={t('accounts.filters.plan')}
                selectedKeys={new Set([planFilter])}
                size="sm"
                onSelectionChange={(selection) => {
                  const nextValue = normalizeSelection(selection)
                  if (!nextValue) {
                    return
                  }
                  setCurrentPage(1)
                  setPlanFilter(nextValue as PlanFilter)
                }}
              >
                <>
                  <SelectItem key="all">{t('accounts.filters.planAll')}</SelectItem>
                  {availablePlanFilters.map((planValue) => (
                    <SelectItem key={planValue}>
                      {getPlanLabel(planValue === PLAN_UNKNOWN_VALUE ? undefined : planValue, t)}
                    </SelectItem>
                  ))}
                </>
              </Select>
            </div>

            <div className="flex items-center gap-2 text-xs text-default-500">
              <span>{t('common.table.rowsPerPage')}</span>
              <Select
                aria-label={t('common.table.rowsPerPage')}
                className="w-[106px]"
                selectedKeys={new Set([String(rowsPerPage)])}
                size="sm"
                onSelectionChange={(selection) => {
                  const nextValue = normalizeSelection(selection)
                  if (!nextValue) {
                    return
                  }
                  setRowsPerPage(Number(nextValue))
                }}
              >
                {TABLE_PAGE_SIZE_OPTIONS.map((size) => (
                  <SelectItem key={String(size)} textValue={String(size)}>
                    {size}
                  </SelectItem>
                ))}
              </Select>
            </div>
          </div>

          {/* ── Batch action bar ── */}
          <div className={cn(
            'w-full overflow-hidden transition-all duration-200',
            selectedIds.size > 0 ? 'max-h-14 opacity-100' : 'max-h-0 opacity-0 pointer-events-none',
          )}>
            <div className="w-full rounded-large bg-default-100/80 px-3.5 py-2">
              <div className="flex w-full flex-wrap items-center gap-2.5">
                {canSelectAcrossPages ? (
                  <>
                    <Checkbox
                      checked={allFilteredSelected}
                      isDisabled={batchActionMutation.isPending}
                      onCheckedChange={toggleFilteredSelection}
                      aria-label={t('common.table.selectFiltered', {
                        defaultValue: 'Select all filtered results ({{count}})',
                        count: filteredRecordIds.length,
                      })}
                    >
                      {t('common.table.selectFiltered', {
                        defaultValue: 'Select all filtered results ({{count}})',
                        count: filteredRecordIds.length,
                      })}
                    </Checkbox>
                    <div className="h-4 w-px bg-default-300" />
                  </>
                ) : null}
                <span className="text-sm font-semibold tabular-nums text-foreground">
                {t('common.table.selectedCount', { defaultValue: '{{count}} selected', count: selectedIds.size })}
                </span>
                <Button
                  size="sm"
                  variant="flat"
                  isDisabled={batchActionMutation.isPending}
                  onPress={() => batchActionMutation.mutate({ action: 'reprobe', ids: [...selectedIds] })}
                >
                  {t('accountPool.actions.reprobe')}
                </Button>
                <Button
                  size="sm"
                  variant="flat"
                  isDisabled={batchActionMutation.isPending || !canBatchRestore}
                  onPress={() => batchActionMutation.mutate({ action: 'restore', ids: [...selectedIds] })}
                >
                  {t('accountPool.actions.restore')}
                </Button>
                <Button
                  size="sm"
                  variant="flat"
                  color="danger"
                  isDisabled={batchActionMutation.isPending}
                  onPress={() => setBatchDeletePending(true)}
                >
                  {t('accountPool.actions.delete')}
                </Button>
                <div className="ml-auto">
                  <Button
                    isIconOnly
                    size="sm"
                    variant="light"
                    aria-label={t('common.table.clearSelection', { defaultValue: 'Clear selection' })}
                    onPress={() => setSelectedIds(new Set())}
                  >
                    <X className="h-3.5 w-3.5" />
                  </Button>
                </div>
              </div>
            </div>
          </div>
        </CardHeader>

        <CardBody className="gap-4 px-5 pb-5 pt-0">
          <Table
            isHeaderSticky
            aria-label={t('accountPool.sections.recordsTitle')}
            sortDescriptor={sortDescriptor ?? undefined}
            classNames={{
              base: 'min-h-[30rem]',
              wrapper: 'bg-transparent px-0 py-0 shadow-none',
              th: 'whitespace-nowrap bg-default-100/60 text-xs font-semibold uppercase tracking-[0.12em] text-default-500',
              td: 'whitespace-nowrap align-top py-4 text-sm text-foreground',
              tr: 'data-[hover=true]:bg-content2/35 transition-colors',
              emptyWrapper: 'h-56',
            }}
            onSortChange={(descriptor) => {
              setCurrentPage(1)
              setSortDescriptor(descriptor)
            }}
          >
            <TableHeader>
              <TableColumn key="__select__" width={40}>
                <Checkbox
                  checked={allCurrentPageSelected ? true : someCurrentPageSelected ? 'indeterminate' : false}
                  onCheckedChange={togglePageSelection}
                  aria-label={t('common.table.selectAll', { defaultValue: 'Select page' })}
                />
              </TableColumn>
              <TableColumn key="account" allowsSorting>{t('accountPool.columns.account')}</TableColumn>
              <TableColumn key="operationalStatus" allowsSorting>{t('accountPool.columns.operationalStatus')}</TableColumn>
              <TableColumn key="quota" allowsSorting width={276}>{t('accountPool.columns.quota')}</TableColumn>
              <TableColumn key="recentSignal" allowsSorting width={216}>{t('accountPool.columns.recentSignal')}</TableColumn>
              <TableColumn key="actions">{t('accountPool.columns.actions')}</TableColumn>
            </TableHeader>
            <TableBody
              emptyContent={(
                <div className="flex flex-col items-center gap-3 py-12 text-default-500">
                  <Archive className="h-10 w-10 opacity-35" />
                  <div className="text-sm font-medium">{t('accountPool.empty')}</div>
                </div>
              )}
              isLoading={isRecordsLoading}
              loadingContent={<Spinner label={t('common.loading')} />}
            >
              {paginatedRecords.map((record) => {
                const isRecordSelected = selectedIds.has(record.id)

                return (
                  <TableRow
                    key={record.id}
                    className={cn(
                      rowFeedback.get(record.id) === 'success' && 'row-action-success',
                      rowFeedback.get(record.id) === 'error' && 'row-action-error',
                      rowFeedback.get(record.id) === 'pending' && 'opacity-60 transition-opacity',
                      isRecordSelected && 'bg-primary/5',
                    )}
                  >
                    <TableCell>
                      <Checkbox
                        checked={isRecordSelected}
                        onCheckedChange={(value) => {
                          setSelectedIds((prev) => {
                            const next = new Set(prev)
                            value ? next.add(record.id) : next.delete(record.id)
                            return next
                          })
                        }}
                        aria-label={t('common.table.selectRow', { defaultValue: 'Select row' })}
                      />
                    </TableCell>
                    <TableCell>
                      <div className="min-w-[240px] space-y-2 overflow-hidden">
                        <div className="truncate font-medium text-foreground">
                          {getRecordLabel(record)}
                        </div>
                        <div className="flex min-w-0 items-center gap-2 overflow-hidden text-xs leading-5 text-default-500">
                          <Chip className="shrink-0" color={getPlanChipColor(record.chatgpt_plan_type)} size="sm" variant="flat">
                            {getPlanLabel(record.chatgpt_plan_type, t)}
                          </Chip>
                          <Chip className="shrink-0" size="sm" variant="flat">
                            {getScopeLabel(record.record_scope, t)}
                          </Chip>
                          {getRecordSecondaryLabel(record) ? (
                            <span className="min-w-0 truncate">{getRecordSecondaryLabel(record)}</span>
                          ) : null}
                        </div>
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="min-w-0 space-y-2 overflow-hidden">
                        <div className="flex items-center gap-2 overflow-hidden">
                          <Chip className="shrink-0" color={getStateColor(record.operator_state)} size="sm" variant="flat">
                            {getStateLabel(record.operator_state, t)}
                          </Chip>
                          <Chip className="shrink-0" color={getReasonColor(record.reason_class)} size="sm" variant="flat">
                            {getReasonLabel(record.reason_class, t)}
                          </Chip>
                        </div>
                        {hasReasonCode(record.reason_code) ? (
                          <div className="truncate text-xs leading-5 text-default-500">
                            {t('accountPool.fields.reasonCode')}: {getReasonCodeLabel(record.reason_code, t)}
                          </div>
                        ) : null}
                        {record.operator_state === 'cooling' ? (
                          <CoolingCountdown nextActionAt={record.next_action_at} t={t} />
                        ) : null}
                      </div>
                    </TableCell>
                    <TableCell className="w-[276px]">
                      <div className="min-w-[248px] space-y-3">
                        {(() => {
                          const usageRows = buildUsageRows(record, t)

                          if (usageRows.length === 0) {
                            return (
                              <div className="text-xs text-default-500">
                                {t('accounts.rateLimits.unavailable')}
                              </div>
                            )
                          }

                          return (
                            <div className="grid gap-2">
                              {usageRows.map((item) => (
                                <div key={item.key} className="rounded-large border border-default-200 bg-content1/70 px-3 py-2">
                                  <div className="flex items-center justify-between gap-3 text-xs text-default-500">
                                    <span>{item.compactLabel}</span>
                                    <span className="tabular-nums text-[11px]">{item.resetText}</span>
                                    <span className="tabular-nums">{item.remainingText}</span>
                                  </div>
                                  <Progress
                                    aria-label={`${item.bucketLabel} ${item.remainingText}`}
                                    className="mt-2"
                                    color={getUsageProgressColor(item.remainingPercent)}
                                    size="sm"
                                    value={item.remainingPercent}
                                  />
                                </div>
                              ))}
                            </div>
                          )
                        })()}
                      </div>
                    </TableCell>
                    <TableCell className="w-[216px]">
                      {(() => {
                        const signal = getRecentSignalDisplay(record, t)
                        const heatmap = record.recent_signal_heatmap

                        return (
                          <div className="min-w-[184px] space-y-2">
                            <div className="flex items-center justify-between gap-2">
                              <div className="text-xs font-semibold uppercase tracking-[0.12em] text-default-500">
                                {heatmap ? t('accountPool.recentSignal.window12h') : signal.label}
                              </div>
                              {heatmap ? (
                                <div className="min-w-0 truncate text-right text-xs text-default-500">
                                  {getRecentSignalInlineSummaryText(record, locale, t)}
                                </div>
                              ) : null}
                            </div>
                            {heatmap ? (
                              <>
                                <SignalHeatmapMini
                                  intensityLevels={heatmap.intensity_levels}
                                  activeCounts={heatmap.active_counts}
                                  passiveCounts={heatmap.passive_counts}
                                  successCounts={heatmap.success_counts}
                                  errorCounts={heatmap.error_counts}
                                  bucketMinutes={heatmap.bucket_minutes}
                                  windowStart={heatmap.window_start}
                                  visibleCount={12}
                                />
                              </>
                            ) : (
                              <>
                                <div className="truncate text-sm font-medium text-foreground">
                                  {signal.timestamp}
                                </div>
                                <div className="truncate text-xs leading-5 text-default-500">
                                  {signal.detail}
                                </div>
                              </>
                            )}
                          </div>
                        )
                      })()}
                    </TableCell>
                    <TableCell className="w-[132px]">
                      <div className="flex items-center justify-end gap-1">
                        <Button
                          isIconOnly
                          aria-label={t('accountPool.actions.inspect')}
                          size="sm"
                          variant="light"
                          onPress={() => openRecord(record.id)}
                        >
                          <Eye className="h-4 w-4" />
                        </Button>
                        <Dropdown>
                          <DropdownTrigger>
                            <Button
                              isIconOnly
                              aria-label={t('accountPool.actions.more')}
                              size="sm"
                              variant="light"
                            >
                              <MoreHorizontal className="h-4 w-4" />
                            </Button>
                          </DropdownTrigger>
                          <DropdownMenu
                            aria-label={t('accountPool.actions.more')}
                            onAction={(key) => runAccountAction(record, String(key) as AccountPoolAction)}
                          >
                            <DropdownItem
                              key="reprobe"
                              startContent={<RefreshCcw className="h-4 w-4" />}
                            >
                              {t('accountPool.actions.reprobe')}
                            </DropdownItem>
                            <DropdownItem
                              key="restore"
                              isDisabled={!canRunAccountPoolAction(record, 'restore')}
                              startContent={<RotateCcw className="h-4 w-4" />}
                            >
                              {t('accountPool.actions.restore')}
                            </DropdownItem>
                            <DropdownItem
                              key="delete"
                              className="text-danger"
                              color="danger"
                              startContent={<Trash2 className="h-4 w-4" />}
                            >
                              {t('accountPool.actions.delete')}
                            </DropdownItem>
                          </DropdownMenu>
                        </Dropdown>
                      </div>
                    </TableCell>
                  </TableRow>
                )
              })}
            </TableBody>
          </Table>
          <div className="flex flex-col gap-3 border-t border-default-200 px-0 pt-4 text-xs text-default-500 sm:flex-row sm:items-center sm:justify-between">
            <div className="tabular-nums">
              {t('common.table.range', {
                start: visibleRangeStart,
                end: visibleRangeEnd,
                total: sortedRecords.length,
              })}
            </div>
            <Pagination
              page={resolvedPage}
              size="sm"
              total={totalPages}
              onChange={setCurrentPage}
            />
          </div>
        </CardBody>
      </Card>

      <Modal
        backdrop="blur"
        classNames={{
          base: 'border-small border-default-200 bg-content1 shadow-large',
          body: 'pt-0',
          backdrop: 'bg-black/52 backdrop-blur-[2px]',
          wrapper: 'px-2 py-2 sm:px-6 sm:py-6',
        }}
        isOpen={Boolean(selectedRecordId)}
        placement="center"
        scrollBehavior="inside"
        size="5xl"
        onOpenChange={(open) => {
          if (!open) closeRecord()
        }}
      >
        <ModalContent>
          {() => (
            <>
              <ModalHeader className="flex flex-col gap-3">
                <div className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                  {selectedLabel ?? t('accountPool.detail.modalTitle')}
                </div>
                <div className="flex flex-wrap gap-2">
                  {selectedRecord ? (
                    <>
                      <Chip color={getStateColor(selectedRecord.operator_state)} size="sm" variant="flat">
                        {getStateLabel(selectedRecord.operator_state, t)}
                      </Chip>
                      <Chip color={getReasonColor(selectedRecord.reason_class)} size="sm" variant="flat">
                        {getReasonLabel(selectedRecord.reason_class, t)}
                      </Chip>
                      <Chip size="sm" variant="flat">
                        {getScopeLabel(selectedRecord.record_scope, t)}
                      </Chip>
                    </>
                  ) : null}
                </div>
                <div className="text-sm leading-6 text-default-600">
                  {t('accountPool.detail.description')}
                </div>
              </ModalHeader>
              <ModalBody className="pb-5">
                {selectedRecord ? (
                  <div className="space-y-5">
                    <div className="grid gap-4 lg:grid-cols-2">
                      <div className="space-y-3 rounded-large border border-default-200/70 px-4 py-4">
                        <h3 className="text-sm font-semibold text-foreground">
                          {t('accountPool.detail.sections.status')}
                        </h3>
                        <div className="space-y-2 text-sm text-default-600">
                          <div>{t('accountPool.fields.routeEligible')}: {selectedRecord.route_eligible ? t('common.yes') : t('common.no')}</div>
                          <div>{t('accountPool.fields.healthFreshness')}: {getHealthFreshnessLabel(selectedRecord.health_freshness, t)}</div>
                          <div>{t('accountPool.fields.nextAction')}: {formatDateTime(selectedRecord.next_action_at)}</div>
                          <div>{t('accountPool.fields.lastSignalAt')}: {formatDateTime(selectedRecord.last_signal_at)}</div>
                          <div>{t('accountPool.fields.lastSignalSource')}: {getSignalSourceLabel(selectedRecord.last_signal_source, t)}</div>
                          <div>{t('accountPool.fields.lastProbeAt')}: {formatDateTime(selectedRecord.last_probe_at)}</div>
                          <div>{t('accountPool.fields.lastProbeOutcome')}: {getProbeOutcomeLabel(selectedRecord.last_probe_outcome, t)}</div>
                          <div>{t('accountPool.fields.reasonCode')}: {getReasonCodeLabel(selectedRecord.reason_code, t)}</div>
                          <div>{t('accountPool.fields.updatedAt')}: {formatDateTime(selectedRecord.updated_at)}</div>
                          <div>{t('accountPool.fields.createdAt')}: {formatDateTime(selectedRecord.created_at)}</div>
                        </div>
                      </div>

                      <div className="space-y-3 rounded-large border border-default-200/70 px-4 py-4">
                        <h3 className="text-sm font-semibold text-foreground">
                          {t('accountPool.detail.sections.profile')}
                        </h3>
                        <div className="space-y-2 text-sm text-default-600">
                          <div>{t('accountPool.fields.email')}: {selectedRecord.email ?? '-'}</div>
                          <div>{t('accountPool.fields.chatgptAccountId')}: {selectedRecord.chatgpt_account_id ?? '-'}</div>
                          <div>{t('accountPool.fields.plan')}: {getPlanLabel(selectedRecord.chatgpt_plan_type, t)}</div>
                          <div>{t('accountPool.fields.sourceType')}: {selectedRecord.source_type ? getSourceTypeLabel(selectedRecord.source_type, t) : '-'}</div>
                          <div>{t('accountPool.fields.recordScope')}: {getScopeLabel(selectedRecord.record_scope, t)}</div>
                        </div>
                      </div>

                      <div className="space-y-3 rounded-large border border-default-200/70 px-4 py-4 lg:col-span-2">
                        <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
                          <div>
                            <h3 className="text-sm font-semibold text-foreground">
                              {t('accountPool.detail.sections.recentSignal')}
                            </h3>
                            <div className="mt-1 text-sm text-default-600">
                              {t('accountPool.recentSignal.window24h')}
                            </div>
                          </div>
                          <Chip size="sm" variant="flat">
                            {selectedSignalHeatmap
                              ? getHeatmapActivityLabel(
                                  {
                                    intensity_levels: selectedSignalHeatmap.buckets.map((bucket) => bucket.intensity),
                                  },
                                  t,
                                )
                              : t('accountPool.recentSignal.noHeatmap')}
                          </Chip>
                        </div>
                        {isFetchingSelectedSignalHeatmap ? (
                          <div className="flex items-center gap-2 py-4 text-sm text-default-600">
                            <Spinner size="sm" />
                            {t('common.loading')}
                          </div>
                        ) : selectedSignalHeatmap ? (
                          <div className="space-y-3">
                            <SignalHeatmapCanvas
                              buckets={selectedSignalHeatmap.buckets}
                              bucketMinutes={selectedSignalHeatmap.bucket_minutes}
                            />
                            <div className="text-sm leading-6 text-default-600">
                              {getRecentSignalSummaryText(selectedRecord, locale, t)}
                            </div>
                          </div>
                        ) : (
                          <div className="rounded-large border border-dashed border-default-200 bg-content1/65 px-4 py-4 text-sm text-default-600">
                            {t('accountPool.recentSignal.noHeatmap')}
                          </div>
                        )}
                      </div>

                      <div className="space-y-3 rounded-large border border-default-200/70 px-4 py-4">
                        <h3 className="text-sm font-semibold text-foreground">
                          {t('accountPool.detail.sections.credentials')}
                        </h3>
                        <div className="space-y-2 text-sm text-default-600">
                          <div>{t('accountPool.fields.mode')}: {getModeLabel(selectedRecord.mode, t)}</div>
                          <div>{t('accountPool.fields.authProvider')}: {getAuthProviderLabel(selectedRecord.auth_provider, t)}</div>
                          <div>{t('accountPool.fields.credentialKind')}: {getCredentialKindLabel(selectedRecord.credential_kind, t)}</div>
                          <div>{t('accountPool.fields.refreshState')}: {getRefreshStateLabel(selectedRecord.refresh_credential_state, t)}</div>
                          <div>{t('accountPool.fields.hasRefreshCredential')}: {selectedRecord.has_refresh_credential ? t('common.yes') : t('common.no')}</div>
                          <div>{t('accountPool.fields.accessTokenFallback')}: {selectedRecord.has_access_token_fallback ? t('common.yes') : t('common.no')}</div>
                        </div>
                      </div>

                      <div className="space-y-3 rounded-large border border-default-200/70 px-4 py-4">
                        <h3 className="text-sm font-semibold text-foreground">
                          {t('accountPool.detail.sections.quota')}
                        </h3>
                        {(() => {
                          const usageRows = buildUsageRows(selectedRecord, t)

                          if (usageRows.length === 0) {
                            return (
                              <div className="rounded-large border border-default-200 bg-content1/70 px-4 py-3 text-sm text-default-600">
                                {t('accounts.rateLimits.unavailable')}
                              </div>
                            )
                          }

                          return (
                            <div className="space-y-3">
                              {usageRows.map((item) => (
                                <div
                                  key={item.key}
                                  className="rounded-large border border-default-200 bg-content1/70 px-4 py-3"
                                >
                                  <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
                                    <div>
                                      <div className="text-xs font-semibold uppercase tracking-[0.12em] text-default-500">
                                        {item.bucketLabel}
                                      </div>
                                      <div className="mt-1 tabular-nums text-base font-semibold text-foreground">
                                        {t('accounts.rateLimits.remainingPrefix')} {item.remainingText}
                                      </div>
                                    </div>
                                    <div className="text-sm leading-5 text-default-500">
                                      {item.resetText}
                                    </div>
                                  </div>
                                  <Progress
                                    aria-label={`${item.bucketLabel} ${item.remainingText}`}
                                    className="mt-3"
                                    color={getUsageProgressColor(item.remainingPercent)}
                                    size="sm"
                                    value={item.remainingPercent}
                                  />
                                </div>
                              ))}
                            </div>
                          )
                        })()}
                        <div className="text-sm text-default-600">
                          {t('accountPool.fields.rateLimitsFetchedAt')}: {formatDateTime(selectedRecord.rate_limits_fetched_at)}
                        </div>
                      </div>

                      <Card
                        className="border border-default-200/70 bg-content1/70 shadow-none lg:col-span-2"
                        radius="lg"
                      >
                        <CardHeader className="flex flex-col items-start gap-2 px-4 pb-0 pt-4">
                          <div className="flex w-full flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
                            <div className="space-y-1">
                              <h3 className="text-sm font-semibold text-foreground">
                                {t('accountPool.detail.sections.realTest')}
                              </h3>
                              <p className="text-sm leading-6 text-default-600">
                                {t('accountPool.detail.test.description')}
                              </p>
                            </div>
                            <Chip size="sm" variant="flat">
                              {selectedRecord.record_scope === 'runtime'
                                ? t('accountPool.scope.runtime')
                                : t('accountPool.scope.inventory')}
                            </Chip>
                          </div>
                        </CardHeader>
                        <CardBody className="space-y-4 px-4 pb-4 pt-4">
                          {selectedRecord.record_scope === 'runtime' ? (
                            <>
                              <div className="grid gap-4 lg:grid-cols-[minmax(0,220px)_minmax(0,1fr)]">
                                <Select
                                  aria-label={t('accountPool.detail.test.modelLabel')}
                                  disallowEmptySelection
                                  isDisabled={testMutation.isPending || testModelOptions.length === 0}
                                  label={t('accountPool.detail.test.modelLabel')}
                                  placeholder={t('accountPool.detail.test.noModels')}
                                  selectedKeys={selectedTestModel ? [selectedTestModel] : []}
                                  size="sm"
                                  variant="bordered"
                                  onSelectionChange={(keys) => {
                                    setSelectedTestModel(normalizeSelection(keys))
                                  }}
                                >
                                  {testModelOptions.map((model) => (
                                    <SelectItem key={model}>
                                      {model}
                                    </SelectItem>
                                  ))}
                                </Select>
                                <Textarea
                                  aria-label={t('accountPool.detail.test.promptLabel')}
                                  isDisabled={testMutation.isPending || testModelOptions.length === 0}
                                  label={t('accountPool.detail.test.promptLabel')}
                                  minRows={4}
                                  placeholder={t('accountPool.detail.test.promptPlaceholder')}
                                  value={testPrompt}
                                  variant="bordered"
                                  onValueChange={setTestPrompt}
                                />
                              </div>

                              {testModelOptions.length === 0 ? (
                                <div className="rounded-large border border-dashed border-default-200 px-4 py-3 text-sm text-default-600">
                                  {t('accountPool.detail.test.noModels')}
                                </div>
                              ) : null}

                              {testError ? (
                                <div className="space-y-3 rounded-large border border-danger/40 px-4 py-3">
                                  <div className="text-sm text-danger-600">
                                    {testError.label}
                                  </div>
                                  {testError.rawMessage ? (
                                    <div className="space-y-2 rounded-large border border-default-200/70 bg-content1/70 px-3 py-3">
                                      <div className="text-[11px] font-semibold uppercase tracking-[0.08em] text-default-500">
                                        {t('accountPool.detail.test.rawError')}
                                      </div>
                                      <div className="whitespace-pre-wrap break-words font-mono text-xs leading-6 text-foreground">
                                        {testError.rawMessage}
                                      </div>
                                    </div>
                                  ) : null}
                                  {testError.code || typeof testError.status === 'number' ? (
                                    <div className="flex flex-wrap gap-2 text-xs text-default-500">
                                      {typeof testError.status === 'number' ? (
                                        <Chip size="sm" variant="flat">
                                          {t('accountPool.detail.test.statusCode')}: {testError.status}
                                        </Chip>
                                      ) : null}
                                      {testError.code ? (
                                        <Chip size="sm" variant="flat">
                                          {t('accountPool.detail.test.errorCode')}: {testError.code}
                                        </Chip>
                                      ) : null}
                                    </div>
                                  ) : null}
                                </div>
                              ) : null}

                              <div className="flex flex-wrap justify-end gap-2">
                                <Button
                                  isDisabled={
                                    testMutation.isPending
                                    || (testMessages.length === 0 && !previousResponseId)
                                  }
                                  size="sm"
                                  variant="light"
                                  onPress={clearResponsesConversation}
                                >
                                  {t('accountPool.detail.test.clearSession')}
                                </Button>
                                <Button
                                  color="primary"
                                  isDisabled={
                                    testMutation.isPending
                                    || !selectedTestModel
                                    || testPrompt.trim().length === 0
                                  }
                                  isLoading={testMutation.isPending}
                                  size="sm"
                                  onPress={handleSendResponsesTest}
                                >
                                  {t('accountPool.detail.test.send')}
                                </Button>
                              </div>

                              {testMessages.length > 0 ? (
                                <div className="space-y-3">
                                  {testMessages.map((message) => {
                                    const usageLines = resolveResponsesTestUsageLines(message.meta, t)
                                    return (
                                      <Card
                                        key={message.id}
                                        className="border border-default-200/70 bg-content1 shadow-none"
                                        radius="lg"
                                      >
                                        <CardBody className="space-y-3 px-4 py-4">
                                          <div className="flex items-center justify-between gap-3">
                                            <Chip
                                              color={message.role === 'assistant' ? 'primary' : 'default'}
                                              size="sm"
                                              variant="flat"
                                            >
                                              {message.role === 'assistant'
                                                ? t('accountPool.detail.test.assistantLabel')
                                                : t('accountPool.detail.test.userLabel')}
                                            </Chip>
                                            {message.meta ? (
                                              <Chip size="sm" variant="flat">
                                                {t('accountPool.detail.test.latency')}: {message.meta.latency_ms}ms
                                              </Chip>
                                            ) : null}
                                          </div>
                                          <div className="whitespace-pre-wrap break-words text-sm leading-6 text-foreground">
                                            {message.text}
                                          </div>
                                          {message.meta ? (
                                            <div className="flex flex-wrap gap-2 text-xs text-default-500">
                                              <Chip size="sm" variant="flat">
                                                {t('accountPool.detail.test.requestId')}: {message.meta.request_id}
                                              </Chip>
                                              <Chip size="sm" variant="flat">
                                                {t('accountPool.detail.test.responseId')}: {message.meta.response_id}
                                              </Chip>
                                              <Chip size="sm" variant="flat">
                                                {t('accountPool.detail.test.modelLabel')}: {message.meta.model}
                                              </Chip>
                                              {usageLines.map((item) => (
                                                <Chip key={item.key} size="sm" variant="flat">
                                                  {item.label}: {item.value}
                                                </Chip>
                                              ))}
                                            </div>
                                          ) : null}
                                        </CardBody>
                                      </Card>
                                    )
                                  })}
                                </div>
                              ) : null}
                            </>
                          ) : (
                            <div className="rounded-large border border-dashed border-default-200 px-4 py-4 text-sm leading-6 text-default-600">
                              {t('accountPool.detail.test.runtimeOnly')}
                            </div>
                          )}
                        </CardBody>
                      </Card>
                    </div>
                  </div>
                ) : (
                  <div className="flex items-center gap-2 py-12 text-sm text-default-600">
                    <Spinner size="sm" />
                    {isFetchingSelectedRecord ? t('common.loading') : t('accountPool.detail.empty')}
                  </div>
                )}
              </ModalBody>
              {selectedRecord ? (
                <ModalFooter className="flex flex-col gap-3 sm:flex-row sm:justify-between">
                  <div className="flex flex-wrap gap-2">
                    <Button
                      isDisabled={!canRunAccountPoolAction(selectedRecord, 'reprobe') || actionMutation.isPending}
                      size="sm"
                      startContent={<RefreshCcw className="h-4 w-4" />}
                      variant="flat"
                      onPress={() => runAccountAction(selectedRecord, 'reprobe')}
                    >
                      {t('accountPool.actions.reprobe')}
                    </Button>
                    <Button
                      isDisabled={!canRunAccountPoolAction(selectedRecord, 'restore') || actionMutation.isPending}
                      size="sm"
                      startContent={<RotateCcw className="h-4 w-4" />}
                      variant="flat"
                      onPress={() => runAccountAction(selectedRecord, 'restore')}
                    >
                      {t('accountPool.actions.restore')}
                    </Button>
                    <Button
                      color="danger"
                      isDisabled={actionMutation.isPending}
                      size="sm"
                      startContent={<Trash2 className="h-4 w-4" />}
                      variant="light"
                      onPress={() => runAccountAction(selectedRecord, 'delete')}
                    >
                      {t('accountPool.actions.delete')}
                    </Button>
                  </div>
                  <Button size="sm" variant="light" onPress={closeRecord}>
                    {t('common.close')}
                  </Button>
                </ModalFooter>
              ) : null}
            </>
          )}
        </ModalContent>
      </Modal>

      <Modal
        backdrop="blur"
        classNames={{
          base: 'border-small border-default-200 bg-content1 shadow-large',
          backdrop: 'bg-black/48 backdrop-blur-[2px]',
        }}
        isOpen={Boolean(deleteTarget)}
        size="md"
        onOpenChange={(open) => {
          if (!open) {
            setDeleteTarget(null)
          }
        }}
      >
        <ModalContent>
          {() => (
            <>
              <ModalHeader className="flex flex-col gap-1">
                <div className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                  {t('accountPool.messages.confirmDeleteTitle', {
                    label: deleteTarget ? getRecordLabel(deleteTarget) : '',
                  })}
                </div>
                <div className="text-sm leading-6 text-default-600">
                  {t('accountPool.messages.confirmDeleteDescription')}
                </div>
              </ModalHeader>
              <ModalFooter>
                <Button size="sm" variant="light" onPress={() => setDeleteTarget(null)}>
                  {t('common.cancel')}
                </Button>
                <Button
                  color="danger"
                  isLoading={actionMutation.isPending}
                  size="sm"
                  variant="flat"
                  onPress={() => {
                    if (!deleteTarget) {
                      return
                    }
                    actionMutation.mutate({ action: 'delete', record: deleteTarget })
                  }}
                >
                  {t('accountPool.actions.delete')}
                </Button>
              </ModalFooter>
            </>
          )}
        </ModalContent>
      </Modal>

      {/* ── Batch delete confirmation ── */}
      <Modal
        classNames={{
          base: 'border-small border-default-200 bg-content1 shadow-large',
          backdrop: 'bg-black/48 backdrop-blur-[2px]',
        }}
        isOpen={batchDeletePending}
        size="md"
        onOpenChange={(open) => {
          if (!open) setBatchDeletePending(false)
        }}
      >
        <ModalContent>
          {() => (
            <>
              <ModalHeader className="flex flex-col gap-1">
                <div className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                  {t('accountPool.messages.batchDeleteTitle', {
                    defaultValue: 'Delete {{count}} records?',
                    count: selectedIds.size,
                  })}
                </div>
                <div className="text-sm leading-6 text-default-600">
                  {t('accountPool.messages.batchDeleteDescription', {
                    defaultValue: 'This action will mark selected records for deletion. This cannot be undone easily.',
                  })}
                </div>
              </ModalHeader>
              <ModalFooter>
                <Button size="sm" variant="light" onPress={() => setBatchDeletePending(false)}>
                  {t('common.cancel')}
                </Button>
                <Button
                  color="danger"
                  isLoading={batchActionMutation.isPending}
                  size="sm"
                  variant="flat"
                  onPress={() => {
                    batchActionMutation.mutate({ action: 'delete', ids: [...selectedIds] })
                  }}
                >
                  {t('accountPool.actions.delete')}
                </Button>
              </ModalFooter>
            </>
          )}
        </ModalContent>
      </Modal>
    </PageContent>
  )
}
