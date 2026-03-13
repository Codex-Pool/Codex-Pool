import { useMemo, useState } from 'react'
import { type ColumnDef } from '@tanstack/react-table'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import type { TFunction } from 'i18next'
import { Coins, Download, Info } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router-dom'

import { billingApi } from '@/api/billing'
import { localizeApiErrorDisplay, localizeHttpStatusDisplay } from '@/api/errorI18n'
import {
  getServiceTierBadgeTone,
  getServiceTierDefaultLabel,
  normalizeServiceTierForDisplay,
  shouldHighlightServiceTier,
} from '@/features/billing/service-tier'
import { groupsApi } from '@/api/groups'
import { systemApi, DEFAULT_SYSTEM_CAPABILITIES } from '@/api/system'
import { tenantCreditsApi, type TenantCreditLedgerItem } from '@/api/tenantCredits'
import { tenantKeysApi } from '@/api/tenantKeys'
import { notify } from '@/lib/notification'
import { formatTokenCount } from '@/lib/token-format'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { StandardDataTable } from '@/components/ui/standard-data-table'
import { TrendChart } from '@/components/ui/trend-chart'
import { formatDateTime, formatMicrocredits } from '@/tenant/lib/format'
import { TenantCostReportPage } from '@/features/billing/tenant-cost-report'

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

function formatMicrocreditsPrecise(value: number | undefined) {
  if (typeof value !== 'number' || Number.isNaN(value)) return '-'
  const credits = Math.abs(value) / 1_000_000
  if (credits === 0) return '0.00'
  if (credits < 0.0001) return '<0.0001'
  if (credits < 1) {
    return credits
      .toFixed(6)
      .replace(/0+$/, '')
      .replace(/\.$/, '')
  }
  return credits.toFixed(2)
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
      return t('tenantBilling.failureReason.tokenInvalidated', { defaultValue: 'Token Invalidated' })
    case 'account_deactivated':
      return t('tenantBilling.failureReason.accountDeactivated', { defaultValue: 'Account Deactivated' })
    case 'transport_error':
      return t('tenantBilling.failureReason.transportError', { defaultValue: 'Transport Error' })
    case 'stream_prelude_error':
      return t('tenantBilling.failureReason.streamPreludeError', { defaultValue: 'Stream Prelude Error' })
    case 'billing_usage_missing':
      return t('tenantBilling.failureReason.billingUsageMissing', { defaultValue: 'Billing Usage Missing' })
    case 'upstream_request_failed':
      return t('tenantBilling.failureReason.upstreamRequestFailed', { defaultValue: 'Upstream Request Failed' })
    case 'no_upstream_account':
      return t('tenantBilling.failureReason.noUpstreamAccount', { defaultValue: 'No Upstream Account' })
    case 'failover_exhausted':
      return t('tenantBilling.failureReason.failoverExhausted', { defaultValue: 'Failover Exhausted' })
    default:
      return t('tenantBilling.failureReason.unknown', {
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
      return t('tenantBilling.releaseReason.failoverExhausted', { defaultValue: 'Failover Exhausted' })
    case 'no_upstream_account':
      return t('tenantBilling.releaseReason.noUpstreamAccount', { defaultValue: 'No Upstream Account' })
    case 'invalid_upstream_url':
      return t('tenantBilling.releaseReason.invalidUpstreamUrl', { defaultValue: 'Invalid Upstream Url' })
    case 'transport_error':
      return t('tenantBilling.releaseReason.transportError', { defaultValue: 'Transport Error' })
    case 'stream_prelude_error':
      return t('tenantBilling.releaseReason.streamPreludeError', { defaultValue: 'Stream Prelude Error' })
    case 'billing_settle_failed':
      return t('tenantBilling.releaseReason.billingSettleFailed', { defaultValue: 'Billing Settle Failed' })
    case 'upstream_request_failed':
      return t('tenantBilling.releaseReason.upstreamRequestFailed', { defaultValue: 'Upstream Request Failed' })
    case 'stream_usage_missing':
      return t('tenantBilling.releaseReason.streamUsageMissing', { defaultValue: 'Stream Usage Missing' })
    default:
      return t('tenantBilling.releaseReason.unknown', {
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
      return t('tenantBilling.failoverAction.crossAccountFailover', { defaultValue: 'Cross Account Failover' })
    case 'return_failure':
      return t('tenantBilling.failoverAction.returnFailure', { defaultValue: 'Return Failure' })
    case 'retry_same_account':
      return t('tenantBilling.failoverAction.retrySameAccount', { defaultValue: 'Retry Same Account' })
    default:
      return t('tenantBilling.failoverAction.unknown', {
        defaultValue: 'Unknown',
      })
  }
}

function localizeTenantBillingServiceTierLabel(value: string | undefined, t: TFunction): string {
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

function parseLedgerBillingMeta(item: TenantCreditLedgerItem): LedgerBillingMeta | undefined {
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

function ledgerStreamTypeLabel(item: TenantCreditLedgerItem, t: TFunction): string {
  const streamFlag = detectLedgerStreamFlag(parseLedgerBillingMeta(item))
  if (streamFlag === true) {
    return t('tenantBilling.ledger.requestTypes.stream', { defaultValue: 'Stream' })
  }
  if (streamFlag === false) {
    return t('tenantBilling.ledger.requestTypes.nonStream', { defaultValue: 'Non-stream' })
  }
  return t('tenantBilling.ledger.requestTypes.unknown', { defaultValue: '-' })
}

function buildTokenPriceSegments(
  meta: LedgerBillingMeta | undefined,
  item: TenantCreditLedgerItem,
  t: TFunction,
): TokenPriceSegment[] {
  if (!meta) return []
  const segments: TokenPriceSegment[] = []

  const inputTokens = meta.billable_input_tokens ?? item.input_tokens
  if (typeof inputTokens === 'number' || typeof meta.input_price_microcredits === 'number') {
    segments.push({
      kind: 'input',
      label: t('tenantBilling.tokenSegment.input', { defaultValue: 'Input' }),
      tokens: typeof inputTokens === 'number' ? inputTokens : 0,
      priceMicrocredits: meta.input_price_microcredits,
    })
  }

  if (typeof meta.cached_input_tokens === 'number' || typeof meta.cached_input_price_microcredits === 'number') {
    segments.push({
      kind: 'cached',
      label: t('tenantBilling.tokenSegment.cached', { defaultValue: 'Cached' }),
      tokens: typeof meta.cached_input_tokens === 'number' ? meta.cached_input_tokens : 0,
      priceMicrocredits: meta.cached_input_price_microcredits,
    })
  }

  const outputTokens = meta.billable_output_tokens ?? item.output_tokens
  if (typeof outputTokens === 'number' || typeof meta.output_price_microcredits === 'number') {
    segments.push({
      kind: 'output',
      label: t('tenantBilling.tokenSegment.output', { defaultValue: 'Output' }),
      tokens: typeof outputTokens === 'number' ? outputTokens : 0,
      priceMicrocredits: meta.output_price_microcredits,
    })
  }

  return segments
}

function buildLedgerBillingDetailLines(
  item: TenantCreditLedgerItem,
  showRawEvents: boolean,
  t: TFunction,
): string[] {
  const meta = parseLedgerBillingMeta(item)
  if (!meta) {
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
    tokenLineParts.push(t('tenantBilling.ledger.detail.tokenSettle', {
      defaultValue: 'Token Settle',
      input: formatTokenCount(billableInputTokens ?? 0),
      cached: formatTokenCount(cachedInputTokens ?? 0),
      output: formatTokenCount(billableOutputTokens ?? 0),
    }))
  }

  const priceParts: string[] = []
  if (typeof meta.input_price_microcredits === 'number') {
    priceParts.push(t('tenantBilling.ledger.detail.unitPrice.input', {
      defaultValue: 'Input',
      price: formatMicrocredits(meta.input_price_microcredits),
    }))
  }
  if (typeof meta.cached_input_price_microcredits === 'number') {
    priceParts.push(t('tenantBilling.ledger.detail.unitPrice.cached', {
      defaultValue: 'Cached',
      price: formatMicrocredits(meta.cached_input_price_microcredits),
    }))
  }
  if (typeof meta.output_price_microcredits === 'number') {
    priceParts.push(t('tenantBilling.ledger.detail.unitPrice.output', {
      defaultValue: 'Output',
      price: formatMicrocredits(meta.output_price_microcredits),
    }))
  }
  if (priceParts.length > 0) {
    tokenLineParts.push(t('tenantBilling.ledger.detail.unitPrice.summary', {
      defaultValue: 'Summary',
      prices: priceParts.join(' / '),
    }))
  }
  if (showRawEvents && meta.pricing_source && meta.pricing_source !== 'exact') {
    tokenLineParts.push(t('tenantBilling.ledger.detail.source', {
      defaultValue: 'Source',
      source: meta.pricing_source,
    }))
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
  settleLineParts.push(t('tenantBilling.ledger.detail.charged', {
    defaultValue: 'Charged',
    value: formatMicrocreditsPrecise(charged),
  }))
  settleLineParts.push(t('tenantBilling.ledger.detail.extraCharge', {
    defaultValue: 'Extra Charge',
    value: formatMicrocreditsPrecise(extra),
  }))
  if (meta.phase === 'reconcile_adjust' && typeof meta.delta_microcredits === 'number') {
    const symbol = meta.delta_microcredits >= 0 ? '+' : '-'
    settleLineParts.push(t('tenantBilling.ledger.detail.reconcileAdjust', {
      defaultValue: 'Reconcile Adjust',
      value: `${symbol}${formatMicrocreditsPrecise(Math.abs(meta.delta_microcredits))}`,
    }))
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
        ? t('tenantBilling.ledger.detail.upstreamStatus', {
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
    failureSummary = failureSummary
      ? t('tenantBilling.ledger.detail.failureSummary', {
          defaultValue: '{{failure}}（{{reason}}）',
          failure: failureSummary,
          reason: reasonPart,
        })
      : reasonPart
  }
  if (failureSummary) {
    settleLineParts.push(t('tenantBilling.ledger.detail.failure', {
      defaultValue: 'Failure',
      summary: failureSummary,
    }))
  } else if (meta.cross_account_failover_attempted) {
    const failoverActionLabel = mapFailoverActionLabel(meta.failover_action, t)
    if (failoverActionLabel) {
      settleLineParts.push(t('tenantBilling.ledger.detail.failoverAction', {
        defaultValue: 'Failover Action',
        action: failoverActionLabel,
      }))
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
  items: TenantCreditLedgerItem[],
  showRawEvents: boolean,
): TenantCreditLedgerItem[] {
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

function consumedMicrocreditsForLedgerItem(item: TenantCreditLedgerItem): number {
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
  item: TenantCreditLedgerItem,
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

function formatMicrocreditsAdaptive(value: number | undefined) {
  return formatMicrocreditsPrecise(value)
}

export function TenantBillingPage() {
  const { data: capabilities = DEFAULT_SYSTEM_CAPABILITIES } = useQuery({
    queryKey: ['systemCapabilities'],
    queryFn: () => systemApi.getCapabilities(),
    staleTime: 5 * 60_000,
  })

  if (!capabilities.features.credit_billing && capabilities.features.cost_reports) {
    return <TenantCostReportPage />
  }

  return <TenantBusinessBillingPage />
}

function TenantBusinessBillingPage() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [searchParams] = useSearchParams()
  const [granularity, setGranularity] = useState<BillingGranularity>(() =>
    parseGranularity(searchParams.get('granularity')),
  )
  const [billingApiKeyId, setBillingApiKeyId] = useState<string>('all')
  const [showRawLedgerEvents, setShowRawLedgerEvents] = useState(false)

  const { data: summary } = useQuery({
    queryKey: ['tenantBillingSummary'],
    queryFn: () => billingApi.tenantSummary(),
    staleTime: 60_000,
  })

  const { data: ledgerResponse } = useQuery({
    queryKey: ['tenantBillingLedger'],
    queryFn: () => billingApi.tenantLedger(200),
    staleTime: 60_000,
  })

  const { data: keys = [] } = useQuery({
    queryKey: ['tenantKeys', 'billing'],
    queryFn: () => tenantKeysApi.list(),
    staleTime: 60_000,
  })

  const { data: groups = [] } = useQuery({
    queryKey: ['tenantApiKeyGroups', 'billing'],
    queryFn: () => groupsApi.tenantList(),
    staleTime: 60_000,
  })

  const checkinMutation = useMutation({
    mutationFn: () => tenantCreditsApi.checkin(),
    onSuccess: (response) => {
      notify({
        variant: 'success',
        title: t('tenantBilling.messages.checkinSuccess', { defaultValue: 'Checkin Success' }),
        description: t('tenantBilling.messages.checkinReward', {
          defaultValue: 'Checkin Reward',
          reward: formatMicrocredits(response.reward_microcredits),
          balance: formatMicrocredits(response.balance_microcredits),
        }),
      })
      queryClient.invalidateQueries({ queryKey: ['tenantCreditsBalance'] })
      queryClient.invalidateQueries({ queryKey: ['tenantCreditsLedger'] })
      queryClient.invalidateQueries({ queryKey: ['tenantBillingSummary'] })
      queryClient.invalidateQueries({ queryKey: ['tenantBillingLedger'] })
    },
    onError: (error) => {
      notify({
        variant: 'error',
        title: t('tenantBilling.messages.checkinFailed', { defaultValue: 'Checkin Failed' }),
        description: localizeApiErrorDisplay(
          t,
          error,
          t('tenantBilling.messages.retryLater', { defaultValue: 'Retry Later' }),
        ).label,
      })
    },
  })

  const allRows = useMemo(() => ledgerResponse?.items ?? [], [ledgerResponse?.items])
  const rows = useMemo(
    () => filterLedgerRowsForDisplay(allRows, showRawLedgerEvents),
    [allRows, showRawLedgerEvents],
  )

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

  const selectedBillingApiKey = useMemo(
    () => (billingApiKeyId === 'all' ? null : keys.find((item) => item.id === billingApiKeyId) ?? null),
    [billingApiKeyId, keys],
  )

  const selectedBillingGroup = useMemo(
    () => (selectedBillingApiKey ? groups.find((item) => item.id === selectedBillingApiKey.group_id) ?? null : null),
    [groups, selectedBillingApiKey],
  )

  const columns = useMemo<ColumnDef<TenantCreditLedgerItem>[]>(() => {
    const resolvedColumns: ColumnDef<TenantCreditLedgerItem>[] = [
      {
        id: 'createdAt',
        header: t('tenantBilling.ledger.columns.time', { defaultValue: 'Time' }),
        accessorFn: (row) => new Date(row.created_at).getTime(),
        cell: ({ row }) => <span className="font-mono text-xs">{formatDateTime(row.original.created_at)}</span>,
      },
    ]

    if (showRawLedgerEvents) {
      resolvedColumns.push({
        id: 'eventType',
        header: t('tenantBilling.ledger.columns.event', { defaultValue: 'Event' }),
        accessorFn: (row) => row.event_type.toLowerCase(),
        cell: ({ row }) => row.original.event_type,
      })
    }

    resolvedColumns.push({
      id: 'requestType',
      header: t('tenantBilling.ledger.columns.requestType', { defaultValue: 'Request Type' }),
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
        header: t('tenantBilling.ledger.columns.delta', { defaultValue: 'Delta' }),
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
              {formatMicrocreditsAdaptive(value)}
            </span>
          )
        },
      },
      {
        id: 'balanceAfter',
        header: t('tenantBilling.ledger.columns.balanceAfter', { defaultValue: 'Balance After' }),
        accessorFn: (row) => row.balance_after_microcredits,
        cell: ({ row }) => <span className="font-mono">{formatMicrocredits(row.original.balance_after_microcredits)}</span>,
      },
      {
        id: 'model',
        header: t('tenantBilling.ledger.columns.model', { defaultValue: 'Model' }),
        accessorFn: (row) => (row.model ?? '').toLowerCase(),
        cell: ({ row }) => row.original.model ?? '-',
      },
      {
        id: 'billingDetail',
        header: t('tenantBilling.ledger.columns.detail', { defaultValue: 'Detail' }),
        accessorFn: (row) => {
          const meta = parseLedgerBillingMeta(row)
          const serviceTier = meta?.service_tier ? normalizeServiceTierForDisplay(meta.service_tier) : ''
          return [serviceTier, ...buildLedgerBillingDetailLines(row, showRawLedgerEvents, t)]
            .join(' | ')
            .toLowerCase()
        },
        cell: ({ row }) => {
          const meta = parseLedgerBillingMeta(row.original)
          const segments = buildTokenPriceSegments(meta, row.original, t)
          const lines = buildLedgerBillingDetailLines(row.original, showRawLedgerEvents, t)
          const showServiceTierBadge = shouldHighlightServiceTier(meta?.service_tier)
          const serviceTierLabel = localizeTenantBillingServiceTierLabel(meta?.service_tier, t)
          const serviceTierLine = showServiceTierBadge
            ? t('tenantBilling.ledger.detail.serviceTier', {
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
          const failureTooltip = buildLedgerFailureTooltip(meta)
          const secondaryTone = secondaryLine?.includes(t('tenantBilling.ledger.detail.failureKeyword', { defaultValue: 'Failure Keyword' }))
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
                            @{formatMicrocredits(segment.priceMicrocredits)}/1M
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
                            @{formatMicrocredits(segment.priceMicrocredits)}/1M
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
                      {t('tenantBilling.ledger.detail.source', {
                        defaultValue: 'Source',
                        source: meta?.pricing_source ?? '',
                      })}
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
  }, [showRawLedgerEvents, t])

  const snapshotColumns = useMemo<ColumnDef<BillingSnapshotRow>[]>(
    () => [
      {
        id: 'period',
        header:
          granularity === 'day'
            ? t('tenantBilling.snapshot.columns.date', { defaultValue: 'Date' })
            : t('tenantBilling.snapshot.columns.month', { defaultValue: 'Month' }),
        accessorKey: 'period',
      },
      {
        id: 'consumed',
        header: t('tenantBilling.snapshot.columns.consumed', { defaultValue: 'Consumed' }),
        accessorFn: (row) => row.consumed_microcredits,
        cell: ({ row }) => (
          <span className="font-mono text-destructive">
            -{formatMicrocredits(row.original.consumed_microcredits)}
          </span>
        ),
      },
      {
        id: 'eventCount',
        header: t('tenantBilling.snapshot.columns.eventCount', { defaultValue: 'Event Count' }),
        accessorFn: (row) => row.event_count,
        cell: ({ row }) => <span className="font-mono">{row.original.event_count}</span>,
      },
    ],
    [granularity, t],
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
        buildLedgerBillingDetailLines(item, showRawLedgerEvents, t).join(' | '),
      ]),
    ]
    const csvContent = `${lines
      .map((line) => line.map((value) => escapeCsvField(value)).join(','))
      .join('\n')}\n`
    const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' })
    const url = URL.createObjectURL(blob)
    const anchor = document.createElement('a')
    anchor.href = url
    anchor.download = `tenant-billing-ledger-${Date.now()}.csv`
    anchor.click()
    URL.revokeObjectURL(url)
  }

  return (
    <div className="flex-1 p-4 sm:p-6 lg:p-8 space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-3xl font-semibold tracking-tight">
            {t('tenantBilling.title', { defaultValue: 'Title' })}
          </h2>
          <p className="text-sm text-muted-foreground mt-1">
            {t('tenantBilling.subtitle', { defaultValue: 'Subtitle' })}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Select value={granularity} onValueChange={(value) => setGranularity(value as BillingGranularity)}>
            <SelectTrigger className="w-[140px]" aria-label={t('tenantBilling.filters.granularityAriaLabel', { defaultValue: 'Granularity' })}>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="day">{t('tenantBilling.filters.day', { defaultValue: 'Day' })}</SelectItem>
              <SelectItem value="month">{t('tenantBilling.filters.month', { defaultValue: 'Month' })}</SelectItem>
            </SelectContent>
          </Select>
          <Button variant="outline" onClick={() => checkinMutation.mutate()} disabled={checkinMutation.isPending}>
            {t('tenantBilling.actions.dailyCheckin', { defaultValue: 'Daily Checkin' })}
          </Button>
          <Button variant="outline" onClick={handleExportLedgerCsv} disabled={!rows.length}>
            <Download className="mr-2 h-4 w-4" />
            {t('tenantBilling.actions.exportCsv', { defaultValue: 'Export Csv' })}
          </Button>
        </div>
      </div>

      <div className="grid gap-4 md:grid-cols-3">
          <Card>
            <CardHeader className="pb-2">
            <CardDescription>{t('tenantBilling.summary.balance', { defaultValue: 'Balance' })}</CardDescription>
            <CardTitle className="text-2xl font-bold">{formatMicrocredits(summary?.balance_microcredits)}</CardTitle>
          </CardHeader>
          <CardContent className="text-xs text-muted-foreground">
            {t('tenantBilling.summary.unitCredits', { defaultValue: 'Unit Credits' })}
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>{t('tenantBilling.summary.todayConsumed', { defaultValue: 'Today Consumed' })}</CardDescription>
            <CardTitle className="text-2xl font-bold">{formatMicrocredits(summary?.today_consumed_microcredits)}</CardTitle>
          </CardHeader>
          <CardContent className="text-xs text-muted-foreground">
            {t('tenantBilling.summary.negativeOnly', { defaultValue: 'Negative Only' })}
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardDescription>{t('tenantBilling.summary.monthConsumed', { defaultValue: 'Month Consumed' })}</CardDescription>
            <CardTitle className="text-2xl font-bold">{formatMicrocredits(summary?.month_consumed_microcredits)}</CardTitle>
          </CardHeader>
          <CardContent className="text-xs text-muted-foreground">
            {t('tenantBilling.summary.negativeOnly', { defaultValue: 'Negative Only' })}
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader className="space-y-2">
          <CardTitle>{t('tenantBilling.groupPricing.title', { defaultValue: 'API key group pricing' })}</CardTitle>
          <CardDescription>
            {t('tenantBilling.groupPricing.description', { defaultValue: 'Review which pricing group each API key uses, and inspect effective model prices for a selected key.' })}
          </CardDescription>
          <div className="max-w-[260px]">
            <Select value={billingApiKeyId} onValueChange={setBillingApiKeyId}>
              <SelectTrigger aria-label={t('tenantBilling.groupPricing.apiKeyAriaLabel', { defaultValue: 'API key selector' })}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">{t('tenantBilling.groupPricing.allKeys', { defaultValue: 'All API keys' })}</SelectItem>
                {keys.map((item) => (
                  <SelectItem key={item.id} value={item.id}>{`${item.name} (${item.key_prefix})`}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          {selectedBillingApiKey && selectedBillingGroup ? (
            <div className="space-y-3">
              <div className="rounded-md border px-4 py-3 text-sm">
                <div className="font-medium">{selectedBillingApiKey.name} · {selectedBillingGroup.name}</div>
                <div className="mt-1 text-muted-foreground">
                  {selectedBillingApiKey.group.deleted
                    ? t('tenantBilling.groupPricing.invalidGroup', { defaultValue: 'This API key is bound to a deleted group. Requests will fail until you change the group.' })
                    : t('tenantBilling.groupPricing.groupSummary', {
                        defaultValue: 'Configured models: {{count}} · allow-all: {{allowAll}}',
                        count: selectedBillingGroup.model_count,
                        allowAll: selectedBillingGroup.allow_all_models ? t('common.yes', { defaultValue: 'Yes' }) : t('common.no', { defaultValue: 'No' }),
                      })}
                </div>
              </div>
              <div className="max-h-[280px] overflow-auto rounded-md border">
                <table className="w-full text-sm">
                  <thead className="bg-muted/40 text-left text-xs text-muted-foreground">
                    <tr>
                      <th className="px-3 py-2">{t('tenantBilling.groupPricing.columns.model', { defaultValue: 'Model' })}</th>
                      <th className="px-3 py-2">{t('tenantBilling.groupPricing.columns.finalPrice', { defaultValue: 'Final price' })}</th>
                      <th className="px-3 py-2">{t('tenantBilling.groupPricing.columns.formulaPrice', { defaultValue: 'Formula price' })}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {selectedBillingGroup.models.slice(0, 16).map((item) => (
                      <tr key={item.model} className="border-t align-top">
                        <td className="px-3 py-2 font-mono text-xs">{item.model}</td>
                        <td className="px-3 py-2 text-xs">{`in ${formatMicrocredits(item.final_input_price_microcredits ?? undefined)} · cached ${formatMicrocredits(item.final_cached_input_price_microcredits ?? undefined)} · out ${formatMicrocredits(item.final_output_price_microcredits ?? undefined)}`}</td>
                        <td className="px-3 py-2 text-xs text-muted-foreground">
                          {item.uses_absolute_pricing ? (
                            <span className="line-through">{`in ${formatMicrocredits(item.formula_input_price_microcredits ?? undefined)} · cached ${formatMicrocredits(item.formula_cached_input_price_microcredits ?? undefined)} · out ${formatMicrocredits(item.formula_output_price_microcredits ?? undefined)}`}</span>
                          ) : (`in ${formatMicrocredits(item.formula_input_price_microcredits ?? undefined)} · cached ${formatMicrocredits(item.formula_cached_input_price_microcredits ?? undefined)} · out ${formatMicrocredits(item.formula_output_price_microcredits ?? undefined)}`)}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          ) : (
            <div className="max-h-[280px] overflow-auto rounded-md border">
              <table className="w-full text-sm">
                <thead className="bg-muted/40 text-left text-xs text-muted-foreground">
                  <tr>
                    <th className="px-3 py-2">{t('tenantBilling.groupPricing.columns.apiKey', { defaultValue: 'API key' })}</th>
                    <th className="px-3 py-2">{t('tenantBilling.groupPricing.columns.group', { defaultValue: 'Group' })}</th>
                    <th className="px-3 py-2">{t('tenantBilling.groupPricing.columns.state', { defaultValue: 'State' })}</th>
                  </tr>
                </thead>
                <tbody>
                  {keys.map((item) => (
                    <tr key={item.id} className="border-t">
                      <td className="px-3 py-2">{item.name}</td>
                      <td className="px-3 py-2">{item.group.name}</td>
                      <td className="px-3 py-2 text-xs text-muted-foreground">
                        {item.group.deleted
                          ? t('tenantBilling.groupPricing.state.invalid', { defaultValue: 'Invalid (deleted group)' })
                          : t('tenantBilling.groupPricing.state.active', { defaultValue: 'Active' })}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t('tenantBilling.trend.title', { defaultValue: 'Title' })}</CardTitle>
          <CardDescription>
            {t('tenantBilling.trend.description', {
              defaultValue: 'Description',
              granularity:
                granularity === 'day'
                  ? t('tenantBilling.filters.dayShort', { defaultValue: 'Day Short' })
                  : t('tenantBilling.filters.monthShort', { defaultValue: 'Month Short' }),
            })}
          </CardDescription>
        </CardHeader>
        <CardContent>
          {chartData.length === 0 ? (
            <div className="h-[260px] rounded-md border border-dashed flex items-center justify-center text-sm text-muted-foreground">
              {t('tenantBilling.trend.empty', { defaultValue: 'Empty' })}
            </div>
          ) : (
            <TrendChart
              data={chartData}
              lines={[
                {
                  dataKey: 'consumed',
                  name: t('tenantBilling.trend.series.consumed', { defaultValue: 'Consumed' }),
                  stroke: 'var(--chart-5)',
                },
              ]}
              height={260}
              xAxisFormatter={(value) => String(value)}
            />
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t('tenantBilling.snapshot.title', { defaultValue: 'Title' })}</CardTitle>
          <CardDescription>
            {t('tenantBilling.snapshot.description', {
              defaultValue: 'Description',
              granularity:
                granularity === 'day'
                  ? t('tenantBilling.filters.dayShort', { defaultValue: 'Day Short' })
                  : t('tenantBilling.filters.monthShort', { defaultValue: 'Month Short' }),
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
            emptyText={t('tenantBilling.snapshot.empty', { defaultValue: 'Empty' })}
          />
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
          <div>
            <CardTitle>{t('tenantBilling.ledger.title', { defaultValue: 'Title' })}</CardTitle>
            <CardDescription>
              {t('tenantBilling.ledger.description', { defaultValue: 'Description' })}
            </CardDescription>
          </div>
          <label className="inline-flex items-center gap-2 text-xs text-muted-foreground">
            <Checkbox
              checked={showRawLedgerEvents}
              onCheckedChange={(checked) => setShowRawLedgerEvents(Boolean(checked))}
              aria-label={t('tenantBilling.ledger.showRaw', { defaultValue: 'Show Raw' })}
            />
            {t('tenantBilling.ledger.showRaw', { defaultValue: 'Show Raw' })}
          </label>
        </CardHeader>
        <CardContent>
          <StandardDataTable
            columns={columns}
            data={rows}
            defaultPageSize={20}
            pageSizeOptions={[20, 50, 100]}
            density="compact"
            emptyText={t('tenantBilling.ledger.empty', { defaultValue: 'Empty' })}
          />
        </CardContent>
      </Card>
    </div>
  )
}
