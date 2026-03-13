import { useMemo, useState } from 'react'
import { type ColumnDef } from '@tanstack/react-table'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Coins, Download, Info } from 'lucide-react'
import { useSearchParams } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import type { TFunction } from 'i18next'

import {
  adminTenantsApi,
  type AdminTenantCreditLedgerItem,
} from '@/api/adminTenants'
import { localizeApiErrorDisplay, localizeHttpStatusDisplay } from '@/api/errorI18n'
import { systemApi, DEFAULT_SYSTEM_CAPABILITIES } from '@/api/system'
import {
  getServiceTierBadgeTone,
  getServiceTierDefaultLabel,
  normalizeServiceTierForDisplay,
  shouldHighlightServiceTier,
} from '@/features/billing/service-tier'
import { formatDateTime as formatI18nDateTime, formatNumber } from '@/lib/i18n-format'
import { notify } from '@/lib/notification'
import { formatTokenCount } from '@/lib/token-format'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { StandardDataTable } from '@/components/ui/standard-data-table'
import { TrendChart } from '@/components/ui/trend-chart'
import { AdminCostReportPage } from '@/features/billing/admin-cost-report'

type BillingGranularity = 'day' | 'month'

interface BillingSeriesItem {
  timestamp: string
  consumed: number
  [key: string]: string | number
}

interface BillingSnapshotRow {
  period: string
  consumed_microcredits: number
  event_count: number
}

function formatMicrocredits(value: number | undefined, locale?: string) {
  if (typeof value !== 'number' || Number.isNaN(value)) return '-'
  return formatNumber(value / 1_000_000, {
    locale,
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
    useGrouping: false,
  })
}

function formatMicrocreditsPrecise(value: number | undefined, locale?: string) {
  if (typeof value !== 'number' || Number.isNaN(value)) return '-'
  const credits = Math.abs(value) / 1_000_000
  if (credits === 0) {
    return formatNumber(credits, {
      locale,
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
      useGrouping: false,
    })
  }
  if (credits < 0.0001) return '<0.0001'
  if (credits < 1) {
    return formatNumber(credits, {
      locale,
      minimumFractionDigits: 0,
      maximumFractionDigits: 6,
      useGrouping: false,
    })
  }
  return formatNumber(credits, {
    locale,
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
    useGrouping: false,
  })
}

interface LedgerBillingMeta {
  authorization_id?: string
  phase?: string
  is_stream?: boolean
  service_tier?: string
  pricing_source?: string
  input_price_microcredits?: number
  cached_input_price_microcredits?: number
  output_price_microcredits?: number
  billable_input_tokens?: number
  cached_input_tokens?: number
  billable_output_tokens?: number
  charged_microcredits?: number
  extra_charge_microcredits?: number
  delta_microcredits?: number
  release_reason?: string
  upstream_status_code?: number
  upstream_error_code?: string
  failover_action?: string
  failover_reason_class?: string
  recovery_action?: string
  recovery_outcome?: string
  cross_account_failover_attempted?: boolean
}

type TokenSegmentKind = 'input' | 'cached' | 'output'

interface TokenPriceSegment {
  kind: TokenSegmentKind
  label: string
  tokens: number
  priceMicrocredits?: number
}

const DEFAULT_BILLING_RECHARGE_REASON_CODE = 'admin_recharge'

function tokenSegmentTone(kind: TokenSegmentKind): string {
  switch (kind) {
    case 'input':
      return 'border-info/30 bg-info-muted text-info-foreground'
    case 'cached':
      return 'border-warning/30 bg-warning-muted text-warning-foreground'
    case 'output':
      return 'border-primary/30 bg-primary/10 text-primary'
    default:
      return 'border-muted bg-muted text-foreground'
  }
}

function asNumber(value: unknown): number | undefined {
  if (typeof value !== 'number' || Number.isNaN(value)) {
    return undefined
  }
  return value
}

function asString(value: unknown): string | undefined {
  if (typeof value !== 'string' || !value.trim()) {
    return undefined
  }
  return value.trim()
}

function asBoolean(value: unknown): boolean | undefined {
  if (typeof value !== 'boolean') {
    return undefined
  }
  return value
}

function mapCodeToLabel(code: string | undefined, t: TFunction): string | undefined {
  if (!code) return undefined
  const normalized = code.trim()
  if (!normalized) return undefined
  switch (normalized.toLowerCase()) {
    case 'token_invalidated':
      return t('billing.ledger.codeLabels.tokenInvalidated', { defaultValue: 'Token invalidated' })
    case 'account_deactivated':
      return t('billing.ledger.codeLabels.accountDeactivated', { defaultValue: 'Account deactivated' })
    case 'transport_error':
      return t('billing.ledger.codeLabels.transportError', { defaultValue: 'Upstream network error' })
    case 'stream_prelude_error':
      return t('billing.ledger.codeLabels.streamPreludeError', { defaultValue: 'Stream prelude error' })
    case 'billing_usage_missing':
      return t('billing.ledger.codeLabels.billingUsageMissing', {
        defaultValue: 'Missing usage settlement fields',
      })
    case 'upstream_request_failed':
      return t('billing.ledger.codeLabels.upstreamRequestFailed', { defaultValue: 'Upstream request failed' })
    case 'no_upstream_account':
      return t('billing.ledger.codeLabels.noUpstreamAccount', { defaultValue: 'No upstream account available' })
    case 'failover_exhausted':
      return t('billing.ledger.codeLabels.failoverExhausted', { defaultValue: 'Retry/failover exhausted' })
    default:
      return t('billing.ledger.codeLabels.unknown', {
        defaultValue: 'Unknown',
      })
  }
}

function mapReleaseReasonLabel(reason: string | undefined, t: TFunction): string | undefined {
  if (!reason) return undefined
  const normalized = reason.trim()
  if (!normalized) return undefined
  switch (normalized.toLowerCase()) {
    case 'failover_exhausted':
      return t('billing.ledger.releaseReasons.failoverExhausted', {
        defaultValue: 'Retry/failover exhausted',
      })
    case 'no_upstream_account':
      return t('billing.ledger.releaseReasons.noUpstreamAccount', {
        defaultValue: 'No upstream account available',
      })
    case 'invalid_upstream_url':
      return t('billing.ledger.releaseReasons.invalidUpstreamUrl', {
        defaultValue: 'Invalid upstream URL configuration',
      })
    case 'transport_error':
      return t('billing.ledger.releaseReasons.transportError', { defaultValue: 'Upstream network error' })
    case 'stream_prelude_error':
      return t('billing.ledger.releaseReasons.streamPreludeError', { defaultValue: 'Stream prelude error' })
    case 'billing_settle_failed':
      return t('billing.ledger.releaseReasons.billingSettleFailed', { defaultValue: 'Billing settlement failed' })
    case 'upstream_request_failed':
      return t('billing.ledger.releaseReasons.upstreamRequestFailed', { defaultValue: 'Upstream request failed' })
    case 'stream_usage_missing':
      return t('billing.ledger.releaseReasons.streamUsageMissing', { defaultValue: 'Stream usage missing' })
    default:
      return t('billing.ledger.releaseReasons.unknown', {
        defaultValue: 'Unknown',
      })
  }
}

function mapFailoverActionLabel(action: string | undefined, t: TFunction): string | undefined {
  if (!action) return undefined
  const normalized = action.trim()
  if (!normalized) return undefined
  switch (normalized.toLowerCase()) {
    case 'cross_account_failover':
      return t('billing.ledger.failoverActions.crossAccountFailover', {
        defaultValue: 'Cross-account failover',
      })
    case 'return_failure':
      return t('billing.ledger.failoverActions.returnFailure', { defaultValue: 'Return failure' })
    case 'retry_same_account':
      return t('billing.ledger.failoverActions.retrySameAccount', { defaultValue: 'Retry same account' })
    default:
      return t('billing.ledger.failoverActions.unknown', {
        defaultValue: 'Unknown',
      })
  }
}

function localizeAdminBillingServiceTierLabel(value: string | undefined, t: TFunction): string {
  const defaultLabel = getServiceTierDefaultLabel(value)
  switch (normalizeServiceTierForDisplay(value)) {
    case 'priority':
      return t('serviceTier.priority', { defaultValue: defaultLabel })
    case 'flex':
      return t('serviceTier.flex', { defaultValue: defaultLabel })
    default:
      return t('serviceTier.default', { defaultValue: defaultLabel })
  }
}

function parseLedgerBillingMeta(item: AdminTenantCreditLedgerItem): LedgerBillingMeta | undefined {
  const payload = item.meta_json
  if (!payload || typeof payload !== 'object' || Array.isArray(payload)) {
    return undefined
  }
  const map = payload as Record<string, unknown>
  return {
    authorization_id: asString(map.authorization_id),
    phase: asString(map.phase),
    is_stream: asBoolean(map.is_stream),
    service_tier: asString(map.service_tier),
    pricing_source: asString(map.pricing_source),
    input_price_microcredits: asNumber(map.input_price_microcredits),
    cached_input_price_microcredits: asNumber(map.cached_input_price_microcredits),
    output_price_microcredits: asNumber(map.output_price_microcredits),
    billable_input_tokens: asNumber(map.billable_input_tokens),
    cached_input_tokens: asNumber(map.cached_input_tokens),
    billable_output_tokens: asNumber(map.billable_output_tokens),
    charged_microcredits: asNumber(map.charged_microcredits),
    extra_charge_microcredits: asNumber(map.extra_charge_microcredits),
    delta_microcredits: asNumber(map.delta_microcredits),
    release_reason: asString(map.release_reason),
    upstream_status_code: asNumber(map.upstream_status_code),
    upstream_error_code: asString(map.upstream_error_code),
    failover_action: asString(map.failover_action),
    failover_reason_class: asString(map.failover_reason_class),
    recovery_action: asString(map.recovery_action),
    recovery_outcome: asString(map.recovery_outcome),
    cross_account_failover_attempted: asBoolean(map.cross_account_failover_attempted),
  }
}

function buildLedgerFailureTooltip(meta: LedgerBillingMeta | undefined): string | undefined {
  if (!meta) return undefined
  const parts: string[] = []
  if (meta.release_reason) parts.push(`release_reason=${meta.release_reason}`)
  if (typeof meta.upstream_status_code === 'number') parts.push(`upstream_status_code=${meta.upstream_status_code}`)
  if (meta.upstream_error_code) parts.push(`upstream_error_code=${meta.upstream_error_code}`)
  if (meta.failover_action) parts.push(`failover_action=${meta.failover_action}`)
  if (meta.failover_reason_class) parts.push(`failover_reason_class=${meta.failover_reason_class}`)
  if (meta.recovery_action) parts.push(`recovery_action=${meta.recovery_action}`)
  if (meta.recovery_outcome) parts.push(`recovery_outcome=${meta.recovery_outcome}`)
  return parts.length ? parts.join(' | ') : undefined
}

function detectLedgerStreamFlag(meta: LedgerBillingMeta | undefined): boolean | undefined {
  if (!meta) return undefined
  if (typeof meta.is_stream === 'boolean') return meta.is_stream
  const streamHintValues = [meta.release_reason, meta.upstream_error_code, meta.failover_reason_class]
  const hasStreamHint = streamHintValues.some((value) =>
    typeof value === 'string' ? value.trim().toLowerCase().startsWith('stream_') : false,
  )
  if (hasStreamHint) return true
  return undefined
}

function ledgerStreamTypeLabel(item: AdminTenantCreditLedgerItem, t: TFunction): string {
  const streamFlag = detectLedgerStreamFlag(parseLedgerBillingMeta(item))
  if (streamFlag === true) {
    return t('billing.ledger.requestTypes.stream', { defaultValue: 'Stream' })
  }
  if (streamFlag === false) {
    return t('billing.ledger.requestTypes.nonStream', { defaultValue: 'Non-stream' })
  }
  return t('billing.ledger.requestTypes.unknown', { defaultValue: '-' })
}

function buildTokenPriceSegments(
  meta: LedgerBillingMeta | undefined,
  item: AdminTenantCreditLedgerItem,
  t: TFunction,
): TokenPriceSegment[] {
  if (!meta) return []
  const segments: TokenPriceSegment[] = []

  const inputTokens = meta.billable_input_tokens ?? item.input_tokens
  if (typeof inputTokens === 'number' || typeof meta.input_price_microcredits === 'number') {
    segments.push({
      kind: 'input',
      label: t('billing.ledger.tokenSegments.input', { defaultValue: 'Input' }),
      tokens: typeof inputTokens === 'number' ? inputTokens : 0,
      priceMicrocredits: meta.input_price_microcredits,
    })
  }

  if (typeof meta.cached_input_tokens === 'number' || typeof meta.cached_input_price_microcredits === 'number') {
    segments.push({
      kind: 'cached',
      label: t('billing.ledger.tokenSegments.cached', { defaultValue: 'Cached' }),
      tokens: typeof meta.cached_input_tokens === 'number' ? meta.cached_input_tokens : 0,
      priceMicrocredits: meta.cached_input_price_microcredits,
    })
  }

  const outputTokens = meta.billable_output_tokens ?? item.output_tokens
  if (typeof outputTokens === 'number' || typeof meta.output_price_microcredits === 'number') {
    segments.push({
      kind: 'output',
      label: t('billing.ledger.tokenSegments.output', { defaultValue: 'Output' }),
      tokens: typeof outputTokens === 'number' ? outputTokens : 0,
      priceMicrocredits: meta.output_price_microcredits,
    })
  }

  return segments
}

function buildLedgerBillingDetailLines(
  item: AdminTenantCreditLedgerItem,
  showRawEvents: boolean,
  locale?: string,
  t?: TFunction,
): string[] {
  const meta = parseLedgerBillingMeta(item)
  if (!meta) {
    return ['-']
  }
  if (!t) {
    return ['-']
  }

  const tokenLineParts: string[] = []
  const billableInputTokens = meta.billable_input_tokens ?? item.input_tokens
  const cachedInputTokens = meta.cached_input_tokens
  const billableOutputTokens = meta.billable_output_tokens ?? item.output_tokens
  if (
    typeof billableInputTokens === 'number' ||
    typeof cachedInputTokens === 'number' ||
    typeof billableOutputTokens === 'number'
  ) {
    tokenLineParts.push(
      t('billing.ledger.details.tokenSettlement', {
        defaultValue: 'Token settlement: input {{input}} + cached {{cached}} + output {{output}}',
        input: formatTokenCount(billableInputTokens ?? 0),
        cached: formatTokenCount(cachedInputTokens ?? 0),
        output: formatTokenCount(billableOutputTokens ?? 0),
      }),
    )
  }

  const priceParts: string[] = []
  if (typeof meta.input_price_microcredits === 'number') {
    priceParts.push(
      t('billing.ledger.tokenSegments.input', { defaultValue: 'Input' })
      + ` ${formatMicrocredits(meta.input_price_microcredits, locale)}`,
    )
  }
  if (typeof meta.cached_input_price_microcredits === 'number') {
    priceParts.push(
      t('billing.ledger.tokenSegments.cached', { defaultValue: 'Cached' })
      + ` ${formatMicrocredits(meta.cached_input_price_microcredits, locale)}`,
    )
  }
  if (typeof meta.output_price_microcredits === 'number') {
    priceParts.push(
      t('billing.ledger.tokenSegments.output', { defaultValue: 'Output' })
      + ` ${formatMicrocredits(meta.output_price_microcredits, locale)}`,
    )
  }
  if (priceParts.length > 0) {
    tokenLineParts.push(
      t('billing.ledger.details.unitPrice', {
        defaultValue: 'Unit price: {{prices}} credits/1M tokens',
        prices: priceParts.join(' / '),
      }),
    )
  }
  if (showRawEvents && meta.pricing_source && meta.pricing_source !== 'exact') {
    tokenLineParts.push(
      t('billing.ledger.details.source', {
        defaultValue: 'Source: {{source}}',
        source: meta.pricing_source,
      }),
    )
  }

  const settleLineParts: string[] = []
  const charged =
    typeof meta.charged_microcredits === 'number'
      ? meta.charged_microcredits
      : item.delta_microcredits < 0
        ? Math.abs(item.delta_microcredits)
        : 0
  const extra =
    typeof meta.extra_charge_microcredits === 'number'
      ? meta.extra_charge_microcredits
      : 0
  settleLineParts.push(
    t('billing.ledger.details.accrued', {
      defaultValue: 'Accrued: {{value}} credits',
      value: formatMicrocreditsPrecise(charged, locale),
    }),
  )
  settleLineParts.push(
    t('billing.ledger.details.extraCharge', {
      defaultValue: 'Extra charge: {{value}} credits',
      value: formatMicrocreditsPrecise(extra, locale),
    }),
  )
  if (meta.phase === 'reconcile_adjust' && typeof meta.delta_microcredits === 'number') {
    const symbol = meta.delta_microcredits >= 0 ? '+' : '-'
    settleLineParts.push(
      t('billing.ledger.details.adjustment', {
        defaultValue: 'Adjustment: {{value}}',
        value: `${symbol}${formatMicrocreditsPrecise(Math.abs(meta.delta_microcredits), locale)}`,
      }),
    )
  }

  const releaseReasonLabel = mapReleaseReasonLabel(meta.release_reason, t)
  const upstreamErrorLabel = mapCodeToLabel(meta.upstream_error_code, t)
  let failureSummary: string | undefined
  if (releaseReasonLabel) {
    failureSummary = releaseReasonLabel
  }
  if (typeof meta.upstream_status_code === 'number' || upstreamErrorLabel) {
    const statusPart =
      typeof meta.upstream_status_code === 'number'
        ? t('billing.ledger.details.upstreamStatus', {
            defaultValue: 'Upstream {{status}}',
            status: localizeHttpStatusDisplay(
              t,
              meta.upstream_status_code,
              t('errors.common.failed'),
            ).label,
          })
        : ''
    const errorPart = upstreamErrorLabel ?? ''
    const reasonPart = [statusPart, errorPart].filter(Boolean).join(' ')
    failureSummary = failureSummary ? `${failureSummary}（${reasonPart}）` : reasonPart
  }
  if (failureSummary) {
    settleLineParts.push(
      t('billing.ledger.details.failure', {
        defaultValue: 'Failure: {{summary}}',
        summary: failureSummary,
      }),
    )
  } else if (meta.cross_account_failover_attempted) {
    const failoverActionLabel = mapFailoverActionLabel(meta.failover_action, t)
    if (failoverActionLabel) {
      settleLineParts.push(
        t('billing.ledger.details.failoverAction', {
          defaultValue: 'Action: {{action}}',
          action: failoverActionLabel,
        }),
      )
    }
  }

  const tokenLine = tokenLineParts.join(' | ')
  const settleLine = settleLineParts.join(' | ')
  if (!tokenLine && !settleLine) {
    return ['-']
  }
  if (!settleLine) {
    return [tokenLine]
  }
  if (!tokenLine) {
    return [settleLine]
  }
  return [tokenLine, settleLine]
}

function filterLedgerRowsForDisplay(
  items: AdminTenantCreditLedgerItem[],
  showRawEvents: boolean,
): AdminTenantCreditLedgerItem[] {
  if (showRawEvents) {
    return items
  }
  return items.filter((item) => {
    const eventType = item.event_type.toLowerCase()
    if (eventType === 'authorize_hold' || eventType === 'release') {
      return false
    }
    return true
  })
}

function consumedMicrocreditsForLedgerItem(item: AdminTenantCreditLedgerItem): number {
  const eventType = item.event_type.toLowerCase()
  if (eventType === 'capture') {
    const charged = parseLedgerBillingMeta(item)?.charged_microcredits
    if (typeof charged === 'number' && charged > 0) {
      return charged
    }
    return item.delta_microcredits < 0 ? Math.abs(item.delta_microcredits) : 0
  }
  if (eventType === 'adjust' || eventType === 'consume') {
    return item.delta_microcredits < 0 ? Math.abs(item.delta_microcredits) : 0
  }
  return 0
}

function displayDeltaMicrocreditsForLedgerItem(
  item: AdminTenantCreditLedgerItem,
  showRawEvents: boolean,
): number {
  if (showRawEvents) {
    return item.delta_microcredits
  }
  const eventType = item.event_type.toLowerCase()
  if (eventType === 'capture') {
    const charged = parseLedgerBillingMeta(item)?.charged_microcredits
    if (typeof charged === 'number' && charged > 0) {
      return -charged
    }
  }
  return item.delta_microcredits
}

function formatMicrocreditsAdaptive(value: number | undefined, locale?: string) {
  return formatMicrocreditsPrecise(value, locale)
}

function bucketKey(date: Date, granularity: BillingGranularity) {
  if (granularity === 'day') {
    return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(
      date.getDate(),
    ).padStart(2, '0')}`
  }
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}`
}

function parseGranularity(raw: string | null): BillingGranularity {
  return raw === 'month' ? 'month' : 'day'
}

function escapeCsvField(value: string) {
  if (value.includes('"') || value.includes(',') || value.includes('\n')) {
    return `"${value.replaceAll('"', '""')}"`
  }
  return value
}

export default function Billing() {
  const { data: capabilities = DEFAULT_SYSTEM_CAPABILITIES } = useQuery({
    queryKey: ['systemCapabilities'],
    queryFn: () => systemApi.getCapabilities(),
    staleTime: 5 * 60_000,
  })

  if (!capabilities.features.credit_billing && capabilities.features.cost_reports) {
    return <AdminCostReportPage capabilities={capabilities} />
  }

  return <BusinessBillingPage />
}

function BusinessBillingPage() {
  const { i18n, t } = useTranslation()
  const queryClient = useQueryClient()
  const locale = i18n.resolvedLanguage ?? i18n.language
  const [searchParams] = useSearchParams()
  const [granularity, setGranularity] = useState<BillingGranularity>(() =>
    parseGranularity(searchParams.get('granularity')),
  )
  const [selectedTenantId, setSelectedTenantId] = useState(
    () => searchParams.get('tenant_id') || '',
  )
  const [showRawLedgerEvents, setShowRawLedgerEvents] = useState(false)
  const [rechargeAmount, setRechargeAmount] = useState('100000000')
  const [rechargeReason, setRechargeReason] = useState(DEFAULT_BILLING_RECHARGE_REASON_CODE)

  const { data: tenants = [] } = useQuery({
    queryKey: ['adminTenants', 'billing'],
    queryFn: () => adminTenantsApi.listTenants(),
    staleTime: 60_000,
  })

  const effectiveTenantId = useMemo(
    () => selectedTenantId || tenants[0]?.id || '',
    [selectedTenantId, tenants],
  )

  const { data: balance } = useQuery({
    queryKey: ['adminTenantBalance', effectiveTenantId],
    queryFn: () => adminTenantsApi.getTenantCreditBalance(effectiveTenantId),
    enabled: Boolean(effectiveTenantId),
    staleTime: 60_000,
  })

  const { data: ledgerResponse } = useQuery({
    queryKey: ['adminTenantLedger', effectiveTenantId],
    queryFn: () => adminTenantsApi.getTenantCreditLedger(effectiveTenantId, 200),
    enabled: Boolean(effectiveTenantId),
    staleTime: 60_000,
  })

  const { data: summary } = useQuery({
    queryKey: ['adminTenantSummary', effectiveTenantId],
    queryFn: () => adminTenantsApi.getTenantCreditSummary(effectiveTenantId),
    enabled: Boolean(effectiveTenantId),
    staleTime: 60_000,
  })

  const rechargeMutation = useMutation({
    mutationFn: () =>
      adminTenantsApi.rechargeTenant(effectiveTenantId, {
        amount_microcredits: Number(rechargeAmount),
        reason: rechargeReason || undefined,
      }),
    onSuccess: (response) => {
      notify({
        variant: 'success',
        title: t('billing.messages.rechargeSuccessTitle', { defaultValue: 'Recharge successful' }),
        description: t('billing.messages.rechargeSuccessDetail', {
          defaultValue: '+{{amount}}, balance {{balance}}',
          amount: formatMicrocredits(response.amount_microcredits, locale),
          balance: formatMicrocredits(response.balance_microcredits, locale),
        }),
      })
      queryClient.invalidateQueries({ queryKey: ['adminTenantBalance', effectiveTenantId] })
      queryClient.invalidateQueries({ queryKey: ['adminTenantSummary', effectiveTenantId] })
      queryClient.invalidateQueries({ queryKey: ['adminTenantLedger', effectiveTenantId] })
    },
    onError: (error) => {
      notify({
        variant: 'error',
        title: t('billing.messages.rechargeFailedTitle', { defaultValue: 'Recharge failed' }),
        description: localizeApiErrorDisplay(
          t,
          error,
          t('billing.messages.retryLater', { defaultValue: 'Please try again later' }),
        ).label,
      })
    },
  })

  const allRows = useMemo(() => ledgerResponse?.items ?? [], [ledgerResponse?.items])
  const rows = useMemo(
    () => filterLedgerRowsForDisplay(allRows, showRawLedgerEvents),
    [allRows, showRawLedgerEvents],
  )

  const fallbackTodayConsumed = useMemo(() => {
    const today = bucketKey(new Date(), 'day')
    return rows.reduce((sum, item) => {
      if (bucketKey(new Date(item.created_at), 'day') !== today) return sum
      return sum + consumedMicrocreditsForLedgerItem(item)
    }, 0)
  }, [rows])

  const fallbackMonthConsumed = useMemo(() => {
    const currentMonth = bucketKey(new Date(), 'month')
    return rows.reduce((sum, item) => {
      if (bucketKey(new Date(item.created_at), 'month') !== currentMonth) return sum
      return sum + consumedMicrocreditsForLedgerItem(item)
    }, 0)
  }, [rows])

  const todayConsumed = summary?.today_consumed_microcredits ?? fallbackTodayConsumed
  const monthConsumed = summary?.month_consumed_microcredits ?? fallbackMonthConsumed

  const chartData = useMemo<BillingSeriesItem[]>(() => {
    const buckets = new Map<string, number>()
    rows.forEach((item) => {
      const consumed = consumedMicrocreditsForLedgerItem(item)
      if (consumed <= 0) return
      const key = bucketKey(new Date(item.created_at), granularity)
      buckets.set(key, (buckets.get(key) ?? 0) + consumed)
    })
    return Array.from(buckets.entries())
      .map(([label, consumed]) => ({ timestamp: label, consumed }))
      .sort((a, b) => a.timestamp.localeCompare(b.timestamp))
  }, [granularity, rows])

  const snapshotRows = useMemo<BillingSnapshotRow[]>(() => {
    const buckets = new Map<string, BillingSnapshotRow>()
    for (const item of rows) {
      const consumed = consumedMicrocreditsForLedgerItem(item)
      if (consumed <= 0) continue
      const key = bucketKey(new Date(item.created_at), granularity)
      const current = buckets.get(key) ?? {
        period: key,
        consumed_microcredits: 0,
        event_count: 0,
      }
      current.consumed_microcredits += consumed
      current.event_count += 1
      buckets.set(key, current)
    }
    return Array.from(buckets.values()).sort((left, right) => left.period.localeCompare(right.period))
  }, [granularity, rows])

  const columns = useMemo<ColumnDef<AdminTenantCreditLedgerItem>[]>(() => {
    const resolvedColumns: ColumnDef<AdminTenantCreditLedgerItem>[] = [
      {
        id: 'createdAt',
        header: t('billing.columns.timestamp', { defaultValue: 'Time' }),
        accessorFn: (row) => new Date(row.created_at).getTime(),
        cell: ({ row }) => (
          <span className="font-mono text-xs">
            {formatI18nDateTime(row.original.created_at, { locale, preset: 'datetime', fallback: '-' })}
          </span>
        ),
      },
    ]

    if (showRawLedgerEvents) {
      resolvedColumns.push({
        id: 'eventType',
        header: t('billing.columns.eventType', { defaultValue: 'Event' }),
        accessorFn: (row) => row.event_type.toLowerCase(),
        cell: ({ row }) => row.original.event_type,
      })
    }

    resolvedColumns.push({
      id: 'requestType',
      header: t('billing.columns.requestType', { defaultValue: 'Request Type' }),
      accessorFn: (row) => ledgerStreamTypeLabel(row, t).toLowerCase(),
      cell: ({ row }) => {
        const streamFlag = detectLedgerStreamFlag(parseLedgerBillingMeta(row.original))
        if (streamFlag === true) {
          return (
            <Badge variant="info" className="px-2 py-0.5 font-medium">
              {ledgerStreamTypeLabel(row.original, t)}
            </Badge>
          )
        }
        if (streamFlag === false) {
          return (
            <Badge variant="secondary" className="px-2 py-0.5 font-medium">
              {ledgerStreamTypeLabel(row.original, t)}
            </Badge>
          )
        }
        return <span className="text-xs text-muted-foreground">{ledgerStreamTypeLabel(row.original, t)}</span>
      },
    })

    resolvedColumns.push(
      {
        id: 'delta',
        header: t('billing.columns.delta', { defaultValue: 'Delta Credits' }),
        accessorFn: (row) => displayDeltaMicrocreditsForLedgerItem(row, showRawLedgerEvents),
        cell: ({ row }) => {
          const value = displayDeltaMicrocreditsForLedgerItem(row.original, showRawLedgerEvents)
          return (
            <span
              className={`inline-block min-w-[106px] text-right font-mono tabular-nums ${
                value < 0 ? 'text-destructive' : 'text-success-foreground'
              }`}
            >
              {value < 0 ? '-' : '+'}
              {formatMicrocreditsAdaptive(value, locale)}
            </span>
          )
        },
      },
      {
        id: 'balanceAfter',
        header: t('billing.columns.balanceAfter', { defaultValue: 'Balance After Change' }),
        accessorFn: (row) => row.balance_after_microcredits,
        cell: ({ row }) => <span className="font-mono">{formatMicrocredits(row.original.balance_after_microcredits, locale)}</span>,
      },
      {
        id: 'model',
        header: t('billing.columns.model', { defaultValue: 'Model' }),
        accessorFn: (row) => (row.model ?? '').toLowerCase(),
        cell: ({ row }) => row.original.model ?? '-',
      },
      {
        id: 'billingDetail',
        header: t('billing.columns.billingDetail', { defaultValue: 'Billing Details' }),
        accessorFn: (row) => {
          const meta = parseLedgerBillingMeta(row)
          const serviceTier = meta?.service_tier ? normalizeServiceTierForDisplay(meta.service_tier) : ''
          return [serviceTier, ...buildLedgerBillingDetailLines(row, showRawLedgerEvents, locale, t)]
            .join(' | ')
            .toLowerCase()
        },
        cell: ({ row }) => {
          const meta = parseLedgerBillingMeta(row.original)
          const segments = buildTokenPriceSegments(meta, row.original, t)
          const lines = buildLedgerBillingDetailLines(row.original, showRawLedgerEvents, locale, t)
          const showServiceTierBadge = shouldHighlightServiceTier(meta?.service_tier)
          const serviceTierLabel = localizeAdminBillingServiceTierLabel(meta?.service_tier, t)
          const serviceTierLine = showServiceTierBadge
            ? t('billing.ledger.details.serviceTier', {
                defaultValue: 'Service Tier: {{tier}}',
                tier: serviceTierLabel,
              })
            : undefined
          if (lines.length === 1 && lines[0] === '-') {
            if (!showServiceTierBadge) {
              return <span className="text-xs text-muted-foreground">-</span>
            }
          }
          const primaryLine = lines[0]
          const secondaryLine = lines[1]
          const showPrimaryLine = !(lines.length === 1 && lines[0] === '-')
          const failurePrefix = t('billing.ledger.details.failurePrefix', { defaultValue: 'Failure:' })
          const failureTooltip = buildLedgerFailureTooltip(meta)
          const secondaryTone = secondaryLine?.includes(failurePrefix)
            ? 'text-warning-foreground'
            : 'text-muted-foreground'
          const showSource =
            showRawLedgerEvents &&
            meta?.pricing_source &&
            meta.pricing_source.trim() &&
            meta.pricing_source !== 'exact'
          return (
            <div className="max-w-[620px] space-y-1 text-xs leading-relaxed">
              <div className="space-y-1">
                {showServiceTierBadge ? (
                  <div className="flex flex-wrap items-center gap-1.5">
                    <Badge
                      variant={getServiceTierBadgeTone(meta?.service_tier)}
                      className="px-2 py-0.5 font-medium"
                    >
                      {serviceTierLabel}
                    </Badge>
                    {segments.map((segment) => (
                      <span
                        key={`${row.original.id}-${segment.kind}`}
                        className={`inline-flex items-center gap-1 rounded-md border px-2 py-0.5 ${tokenSegmentTone(segment.kind)}`}
                      >
                        <span>{segment.label}</span>
                        <span className="font-mono tabular-nums">{formatTokenCount(segment.tokens)}</span>
                        {typeof segment.priceMicrocredits === 'number' ? (
                          <span className="font-mono tabular-nums opacity-80">
                            @{formatMicrocredits(segment.priceMicrocredits, locale)}/1M
                          </span>
                        ) : null}
                      </span>
                    ))}
                  </div>
                ) : segments.length > 0 ? (
                  <div className="flex flex-wrap items-center gap-1.5">
                    {segments.map((segment) => (
                      <span
                        key={`${row.original.id}-${segment.kind}`}
                        className={`inline-flex items-center gap-1 rounded-md border px-2 py-0.5 ${tokenSegmentTone(segment.kind)}`}
                      >
                        <span>{segment.label}</span>
                        <span className="font-mono tabular-nums">{formatTokenCount(segment.tokens)}</span>
                        {typeof segment.priceMicrocredits === 'number' ? (
                          <span className="font-mono tabular-nums opacity-80">
                            @{formatMicrocredits(segment.priceMicrocredits, locale)}/1M
                          </span>
                        ) : null}
                      </span>
                    ))}
                  </div>
                ) : showPrimaryLine ? (
                  <div className="flex items-start gap-1.5 text-muted-foreground">
                    <Info className="mt-0.5 h-3.5 w-3.5 shrink-0" />
                    <span>{primaryLine}</span>
                  </div>
                ) : null}
                {serviceTierLine ? (
                  <div className="flex items-start gap-1.5 text-muted-foreground">
                    <Info className="mt-0.5 h-3.5 w-3.5 shrink-0" />
                    <span>{serviceTierLine}</span>
                  </div>
                ) : null}
                {showSource ? (
                  <div className="flex items-start gap-1.5 text-muted-foreground">
                    <Info className="mt-0.5 h-3.5 w-3.5 shrink-0" />
                    <span>
                      {t('billing.columns.source', { defaultValue: 'Source' })}: {meta?.pricing_source}
                    </span>
                  </div>
                ) : null}
              </div>
              {secondaryLine ? (
                <div className={`flex items-start gap-1.5 ${secondaryTone}`}>
                  <Coins className="mt-0.5 h-3.5 w-3.5 shrink-0 text-success-foreground" />
                  <span title={failureTooltip}>{secondaryLine}</span>
                </div>
              ) : null}
            </div>
          )
        },
      },
    )

    return resolvedColumns
  }, [locale, showRawLedgerEvents, t])

  const snapshotColumns = useMemo<ColumnDef<BillingSnapshotRow>[]>(
    () => [
      {
        id: 'period',
        header:
          granularity === 'day'
            ? t('billing.columns.periodDay', { defaultValue: 'Date' })
            : t('billing.columns.periodMonth', { defaultValue: 'Month' }),
        accessorKey: 'period',
      },
      {
        id: 'consumed',
        header: t('billing.columns.deductedCredits', { defaultValue: 'Deducted Credits' }),
        accessorFn: (row) => row.consumed_microcredits,
        cell: ({ row }) => (
          <span className="font-mono text-destructive">
            -{formatMicrocredits(row.original.consumed_microcredits, locale)}
          </span>
        ),
      },
      {
        id: 'eventCount',
        header: t('billing.columns.deductionEvents', { defaultValue: 'Deduction Events' }),
        accessorFn: (row) => row.event_count,
        cell: ({ row }) => <span className="font-mono">{row.original.event_count}</span>,
      },
    ],
    [granularity, locale, t],
  )

  const handleExportLedgerCsv = () => {
    const lines = [
      [
        'created_at',
        'event_type',
        'request_type',
        'service_tier',
        'delta_microcredits',
        'balance_after_microcredits',
        'api_key_id',
        'request_id',
        'model',
        'billing_detail',
      ],
      ...rows.map((item) => [
        item.created_at,
        item.event_type,
        ledgerStreamTypeLabel(item, t),
        (() => {
          const meta = parseLedgerBillingMeta(item)
          return meta?.service_tier ? normalizeServiceTierForDisplay(meta.service_tier) : ''
        })(),
        String(displayDeltaMicrocreditsForLedgerItem(item, showRawLedgerEvents)),
        String(item.balance_after_microcredits),
        item.api_key_id ?? '',
        item.request_id ?? '',
        item.model ?? '',
        buildLedgerBillingDetailLines(item, showRawLedgerEvents, locale, t).join(' | '),
      ]),
    ]
    const csvContent = `${lines
      .map((line) => line.map((value) => escapeCsvField(value)).join(','))
      .join('\n')}\n`
    const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' })
    const url = URL.createObjectURL(blob)
    const anchor = document.createElement('a')
    anchor.href = url
    anchor.download = `admin-billing-ledger-${effectiveTenantId || 'tenant'}-${Date.now()}.csv`
    anchor.click()
    URL.revokeObjectURL(url)
  }

  const tenantSelectValue = effectiveTenantId || '__none__'

  return (
    <div className="flex-1 p-4 sm:p-6 lg:p-8 space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-3xl font-semibold tracking-tight">
            {t('billing.title', { defaultValue: 'Billing Center' })}
          </h2>
          <p className="text-sm text-muted-foreground mt-1">
            {t('billing.subtitle', {
              defaultValue: 'Primary view: credit ledger (actual charges), with tenant-level admin filtering.',
            })}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Select
            value={tenantSelectValue}
            onValueChange={(value) => setSelectedTenantId(value === '__none__' ? '' : value)}
          >
            <SelectTrigger className="min-w-[280px]" aria-label={t('billing.filters.tenantAriaLabel', { defaultValue: 'Tenant' })}>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="__none__">
                {t('billing.filters.tenantPlaceholder', { defaultValue: 'Select tenant' })}
              </SelectItem>
              {tenants.map((tenant) => (
                <SelectItem key={tenant.id} value={tenant.id}>
                  {tenant.name} ({tenant.id})
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Select value={granularity} onValueChange={(value) => setGranularity(value as BillingGranularity)}>
            <SelectTrigger className="w-[140px]" aria-label={t('billing.filters.granularityAriaLabel', { defaultValue: 'Granularity' })}>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="day">{t('billing.granularity.day', { defaultValue: 'Daily' })}</SelectItem>
              <SelectItem value="month">{t('billing.granularity.month', { defaultValue: 'Monthly' })}</SelectItem>
            </SelectContent>
          </Select>
          <Button variant="outline" onClick={handleExportLedgerCsv} disabled={!rows.length}>
            <Download className="mr-2 h-4 w-4" />
            {t('billing.exportCsv', { defaultValue: 'Export CSV' })}
          </Button>
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>{t('billing.recharge.title', { defaultValue: 'Admin Recharge' })}</CardTitle>
          <CardDescription>
            {t('billing.recharge.subtitle', { defaultValue: 'Recharge the currently selected tenant.' })}
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-wrap items-center gap-2">
          <Input
            value={rechargeAmount}
            onChange={(event) => setRechargeAmount(event.target.value)}
            type="number"
            inputMode="numeric"
            min={0}
            aria-label={t('billing.recharge.amountAriaLabel', { defaultValue: 'Recharge amount in microcredits' })}
            placeholder={t('billing.recharge.amountPlaceholder', {
              defaultValue: 'Recharge credits (microcredits)',
            })}
          />
          <Input
            className="min-w-[280px]"
            value={rechargeReason}
            onChange={(event) => setRechargeReason(event.target.value)}
            aria-label={t('billing.recharge.reasonAriaLabel', { defaultValue: 'Recharge reason' })}
            placeholder={t('billing.recharge.reasonPlaceholder', { defaultValue: 'Recharge reason' })}
          />
          <Button
            onClick={() => rechargeMutation.mutate()}
            disabled={rechargeMutation.isPending || !effectiveTenantId}
          >
            {t('billing.recharge.submit', { defaultValue: 'Execute Recharge' })}
          </Button>
        </CardContent>
      </Card>

      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>{t('billing.summary.currentBalance', { defaultValue: 'Current Balance' })}</CardDescription>
            <CardTitle className="text-2xl font-bold">{formatMicrocredits(balance?.balance_microcredits, locale)}</CardTitle>
          </CardHeader>
          <CardContent className="text-xs text-muted-foreground">
            {t('billing.summary.unitCredits', { defaultValue: 'Unit: credits' })}
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>{t('billing.summary.todayConsumed', { defaultValue: "Today's Consumption" })}</CardDescription>
            <CardTitle className="text-2xl font-bold">{formatMicrocredits(todayConsumed, locale)}</CardTitle>
          </CardHeader>
          <CardContent className="text-xs text-muted-foreground">
            {t('billing.summary.deductionHint', { defaultValue: 'Only negative ledger deduction events are counted.' })}
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>{t('billing.summary.monthConsumed', { defaultValue: 'This Month Consumption' })}</CardDescription>
            <CardTitle className="text-2xl font-bold">{formatMicrocredits(monthConsumed, locale)}</CardTitle>
          </CardHeader>
          <CardContent className="text-xs text-muted-foreground">
            {t('billing.summary.deductionHint', { defaultValue: 'Only negative ledger deduction events are counted.' })}
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>{t('billing.trend.title', { defaultValue: 'Consumption Trend' })}</CardTitle>
          <CardDescription>
            {t('billing.trend.subtitle', {
              defaultValue: 'Show ledger deductions aggregated by {{granularity}}.',
              granularity:
                granularity === 'day'
                  ? t('billing.granularity.day', { defaultValue: 'Daily' })
                  : t('billing.granularity.month', { defaultValue: 'Monthly' }),
            })}
          </CardDescription>
        </CardHeader>
        <CardContent>
          {chartData.length === 0 ? (
            <div className="h-[260px] rounded-md border border-dashed flex items-center justify-center text-sm text-muted-foreground">
              {t('billing.trend.noData', { defaultValue: 'No trend data yet.' })}
            </div>
          ) : (
            <TrendChart
              data={chartData}
              lines={[
                {
                  dataKey: 'consumed',
                  name: t('billing.trend.seriesConsumed', { defaultValue: 'Consumed Credits' }),
                  stroke: 'var(--chart-5)',
                },
              ]}
              height={260}
              locale={locale}
              xAxisFormatter={(value) => String(value)}
            />
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t('billing.snapshot.title', { defaultValue: 'Settlement Snapshot' })}</CardTitle>
          <CardDescription>
            {t('billing.snapshot.subtitle', {
              defaultValue: 'Aggregate deduction events by {{granularity}} for month-end settlement and reconciliation.',
              granularity:
                granularity === 'day'
                  ? t('billing.granularity.day', { defaultValue: 'Daily' })
                  : t('billing.granularity.month', { defaultValue: 'Monthly' }),
            })}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <StandardDataTable
            columns={snapshotColumns}
            data={snapshotRows}
            defaultPageSize={20}
            pageSizeOptions={[20, 50, 100]}
            density="compact"
            emptyText={t('billing.snapshot.empty', { defaultValue: 'No settlement snapshots yet.' })}
          />
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
          <div>
            <CardTitle>{t('billing.ledger.title', { defaultValue: 'Ledger Entries' })}</CardTitle>
            <CardDescription>{t('billing.ledger.subtitle', { defaultValue: 'Filtered by current tenant.' })}</CardDescription>
          </div>
          <label className="inline-flex items-center gap-2 text-xs text-muted-foreground">
            <Checkbox
              checked={showRawLedgerEvents}
              onCheckedChange={(checked) => setShowRawLedgerEvents(Boolean(checked))}
              aria-label={t('billing.ledger.showRaw', { defaultValue: 'Show raw entries' })}
            />
            {t('billing.ledger.showRaw', { defaultValue: 'Show raw entries' })}
          </label>
        </CardHeader>
        <CardContent>
          <StandardDataTable
            columns={columns}
            data={rows}
            defaultPageSize={20}
            pageSizeOptions={[20, 50, 100]}
            density="compact"
            emptyText={t('billing.ledger.empty', { defaultValue: 'No ledger entries yet.' })}
          />
        </CardContent>
      </Card>
    </div>
  )
}
