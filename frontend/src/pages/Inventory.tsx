import { type ReactNode, useMemo, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { type ColumnDef } from '@tanstack/react-table'
import { Archive, KeyRound, RefreshCcw, ShieldCheck, TimerReset, TriangleAlert } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import {
  accountsApi,
  type OAuthInventoryRecord,
  type OAuthInventoryStatus,
} from '@/api/accounts'
import { localizeOAuthErrorCodeDisplay } from '@/api/errorI18n'
import { PageIntro, PagePanel, SectionHeader } from '@/components/layout/page-archetypes'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { StandardDataTable } from '@/components/ui/standard-data-table'
import {
  bucketLabel,
  extractRateLimitDisplaysFromSnapshots,
  formatAbsoluteDateTime,
  formatRateLimitResetText,
  getInventoryStatusBadgeVariant,
  getInventoryStatusLabel,
  getSourceTypeLabel,
  normalizePlanValue,
} from '@/features/accounts/utils'
import { PLAN_UNKNOWN_VALUE } from '@/features/accounts/types'
import { formatExactCount } from '@/lib/count-number-format'

type InventoryFilter = 'all' | OAuthInventoryStatus

function formatOptionalDateTime(value?: string) {
  if (!value) {
    return '-'
  }
  return formatAbsoluteDateTime(value)
}

function buildQuotaSummary(record: OAuthInventoryRecord, locale: string, t: ReturnType<typeof useTranslation>['t']) {
  const displays = extractRateLimitDisplaysFromSnapshots(record.admission_rate_limits)
  if (displays.length === 0) {
    return '-'
  }

  return displays
    .map((item) => {
      const remaining = `${Math.max(0, Math.min(100, item.remainingPercent)).toFixed(0)}%`
      const reset = formatRateLimitResetText({ resetsAt: item.resetsAt, locale, t })
      return `${bucketLabel(item.bucket, t)} ${remaining} · ${reset}`
    })
    .join('\n')
}

function InventoryMetric({
  icon,
  title,
  value,
  tone = 'default',
}: {
  icon: ReactNode
  title: string
  value: number
  tone?: 'default' | 'success' | 'warning' | 'destructive' | 'info'
}) {
  const valueClassName =
    tone === 'success'
      ? 'text-emerald-700 dark:text-emerald-300'
      : tone === 'warning'
        ? 'text-amber-700 dark:text-amber-300'
        : tone === 'destructive'
          ? 'text-destructive'
          : tone === 'info'
            ? 'text-sky-700 dark:text-sky-300'
            : 'text-foreground'

  return (
    <div className="space-y-3 bg-background/84 px-4 py-4 dark:bg-card/84">
      <div className="flex items-start justify-between gap-3">
        <p className="text-[12px] font-semibold uppercase tracking-[0.1em] text-foreground/76">
          {title}
        </p>
        <div className="flex h-8 w-8 items-center justify-center rounded-[0.8rem] border border-border/70 bg-background/62 text-muted-foreground">
          {icon}
        </div>
      </div>
      <p className={`text-[clamp(1.65rem,2.4vw,2.1rem)] font-semibold leading-none tracking-[-0.04em] ${valueClassName}`}>
        {formatExactCount(value)}
      </p>
    </div>
  )
}

export default function Inventory() {
  const { t, i18n } = useTranslation()
  const [statusFilter, setStatusFilter] = useState<InventoryFilter>('all')

  const {
    data: summary,
    isLoading: isSummaryLoading,
    refetch: refetchSummary,
    isFetching: isSummaryFetching,
  } = useQuery({
    queryKey: ['oauthInventorySummary'],
    queryFn: accountsApi.getOAuthInventorySummary,
    staleTime: 60_000,
    refetchInterval: 60_000,
    refetchOnWindowFocus: 'always',
  })

  const {
    data: records = [],
    isLoading: isRecordsLoading,
    refetch: refetchRecords,
    isFetching: isRecordsFetching,
  } = useQuery({
    queryKey: ['oauthInventoryRecords'],
    queryFn: accountsApi.getOAuthInventoryRecords,
    staleTime: 60_000,
    refetchInterval: 60_000,
    refetchOnWindowFocus: 'always',
  })

  const filteredRecords = useMemo(() => {
    if (statusFilter === 'all') {
      return records
    }
    return records.filter((record) => record.vault_status === statusFilter)
  }, [records, statusFilter])

  const inventoryColumns = useMemo<ColumnDef<OAuthInventoryRecord>[]>(
    () => [
      {
        id: 'identity',
        accessorFn: (row) => `${row.email ?? ''} ${row.label}`.toLowerCase(),
        header: t('inventory.columns.account', { defaultValue: 'Account' }),
        cell: ({ row }) => {
          const planValue = normalizePlanValue(row.original.chatgpt_plan_type)
          return (
            <div className="min-w-[220px] space-y-1">
              <div className="font-medium text-foreground">
                {row.original.email?.trim() || row.original.label}
              </div>
              <div className="text-xs text-muted-foreground">{row.original.label}</div>
              <div className="flex flex-wrap gap-2">
                <Badge variant="outline" className="text-[11px] font-normal">
                  {planValue === PLAN_UNKNOWN_VALUE
                    ? t('accounts.filters.planUnknown', { defaultValue: 'Not Reported' })
                    : planValue}
                </Badge>
                {row.original.source_type ? (
                  <Badge variant="secondary" className="text-[11px] font-normal">
                    {getSourceTypeLabel(row.original.source_type, t) ?? row.original.source_type}
                  </Badge>
                ) : null}
              </div>
            </div>
          )
        },
      },
      {
        id: 'chatgptAccountId',
        accessorFn: (row) => row.chatgpt_account_id ?? '',
        header: t('inventory.columns.chatgptAccountId', { defaultValue: 'ChatGPT Account ID' }),
        cell: ({ row }) => (
          <span className="font-mono text-xs text-muted-foreground">
            {row.original.chatgpt_account_id ?? '-'}
          </span>
        ),
      },
      {
        id: 'vaultStatus',
        accessorFn: (row) => row.vault_status,
        header: t('inventory.columns.vaultStatus', { defaultValue: 'Vault Status' }),
        cell: ({ row }) => (
          <Badge variant={getInventoryStatusBadgeVariant(row.original.vault_status)}>
            {getInventoryStatusLabel(row.original.vault_status, t)}
          </Badge>
        ),
      },
      {
        id: 'credentials',
        accessorFn: (row) => Number(row.has_refresh_token) + Number(row.has_access_token_fallback),
        header: t('inventory.columns.credentials', { defaultValue: 'Credentials' }),
        cell: ({ row }) => (
          <div className="min-w-[140px] space-y-1 text-xs text-muted-foreground">
            <div className="flex items-center gap-2">
              <Badge variant={row.original.has_refresh_token ? 'success' : 'secondary'}>
                {row.original.has_refresh_token
                  ? t('inventory.credentials.hasRt', { defaultValue: 'RT ready' })
                  : t('inventory.credentials.noRt', { defaultValue: 'No RT' })}
              </Badge>
            </div>
            <div className="flex items-center gap-2">
              <Badge variant={row.original.has_access_token_fallback ? 'info' : 'secondary'}>
                {row.original.has_access_token_fallback
                  ? t('inventory.credentials.hasAk', { defaultValue: 'AK fallback' })
                  : t('inventory.credentials.noAk', { defaultValue: 'No AK' })}
              </Badge>
            </div>
          </div>
        ),
      },
      {
        id: 'quota',
        accessorFn: (row) =>
          row.admission_rate_limits?.map((item) => item.limit_id ?? item.limit_name ?? '').join(' ') ?? '',
        header: t('inventory.columns.quota', { defaultValue: 'Quota Summary' }),
        cell: ({ row }) => {
          const summaryText = buildQuotaSummary(
            row.original,
            i18n.resolvedLanguage ?? i18n.language,
            t,
          )
          return (
            <div className="min-w-[260px] whitespace-pre-line text-xs leading-5 text-muted-foreground">
              {summaryText}
            </div>
          )
        },
      },
      {
        id: 'timestamps',
        accessorFn: (row) => row.admission_checked_at ?? row.updated_at,
        header: t('inventory.columns.timeline', { defaultValue: 'Admission Timeline' }),
        cell: ({ row }) => (
          <div className="min-w-[180px] space-y-1 text-xs text-muted-foreground">
            <div>
              {t('inventory.fields.checkedAt', { defaultValue: 'Checked' })}: {formatOptionalDateTime(row.original.admission_checked_at)}
            </div>
            <div>
              {t('inventory.fields.retryAfter', { defaultValue: 'Retry after' })}: {formatOptionalDateTime(row.original.admission_retry_after)}
            </div>
          </div>
        ),
      },
      {
        id: 'reason',
        accessorFn: (row) =>
          `${row.admission_error_code ?? ''} ${row.admission_error_message ?? ''}`.toLowerCase(),
        header: t('inventory.columns.reason', { defaultValue: 'Reason' }),
        cell: ({ row }) => {
          const errorDisplay = localizeOAuthErrorCodeDisplay(t, row.original.admission_error_code)
          const reasonLabel = row.original.admission_error_code
            ? errorDisplay.label
            : row.original.admission_error_message ?? '-'
          const tooltip = row.original.admission_error_code
            ? errorDisplay.tooltip
            : row.original.admission_error_message
          return (
            <div className="min-w-[220px]">
              <div className="truncate text-sm text-foreground" title={tooltip}>
                {reasonLabel}
              </div>
              {row.original.admission_source ? (
                <div className="mt-1 text-xs text-muted-foreground">
                  {t('inventory.fields.source', { defaultValue: 'Source' })}: {row.original.admission_source}
                </div>
              ) : null}
            </div>
          )
        },
      },
    ],
    [i18n.language, i18n.resolvedLanguage, t],
  )

  const refreshInventory = async () => {
    await Promise.all([refetchSummary(), refetchRecords()])
  }

  return (
    <div className="flex-1 overflow-y-auto px-4 py-4 sm:px-6 lg:px-8">
      <div className="space-y-6 md:space-y-7">
        <PageIntro
          archetype="workspace"
          eyebrow={t('inventory.eyebrow', { defaultValue: 'Inventory' })}
          title={t('inventory.title', { defaultValue: 'OAuth Inventory' })}
          description={t('inventory.subtitle', {
            defaultValue:
              'Track vaulted OAuth inventory before activation so queued, ready, and no-quota records never get mixed into the runtime pool view.',
          })}
          meta={(
            <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
              <span>
                {t('inventory.meta.total', {
                  defaultValue: 'Total {{count}}',
                  count: summary?.total ?? records.length,
                })}
              </span>
              <span className="text-border">/</span>
              <span>
                {t('inventory.meta.filtered', {
                  defaultValue: 'Showing {{count}}',
                  count: filteredRecords.length,
                })}
              </span>
            </div>
          )}
          actions={(
            <div className="flex flex-wrap items-center gap-2">
              <Select value={statusFilter} onValueChange={(value) => setStatusFilter(value as InventoryFilter)}>
                <SelectTrigger className="min-w-[11rem]" aria-label={t('inventory.filters.status')}>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">{t('inventory.filters.all', { defaultValue: 'All inventory' })}</SelectItem>
                  <SelectItem value="queued">{getInventoryStatusLabel('queued', t)}</SelectItem>
                  <SelectItem value="ready">{getInventoryStatusLabel('ready', t)}</SelectItem>
                  <SelectItem value="needs_refresh">{getInventoryStatusLabel('needs_refresh', t)}</SelectItem>
                  <SelectItem value="no_quota">{getInventoryStatusLabel('no_quota', t)}</SelectItem>
                  <SelectItem value="failed">{getInventoryStatusLabel('failed', t)}</SelectItem>
                </SelectContent>
              </Select>
              <Button variant="outline" size="sm" onClick={() => void refreshInventory()} disabled={isSummaryFetching || isRecordsFetching}>
                <RefreshCcw className="mr-2 h-4 w-4" />
                {t('common.refresh')}
              </Button>
            </div>
          )}
        />

        <div className="grid gap-px overflow-hidden rounded-[1rem] border border-border/70 bg-border/70 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-6">
          <InventoryMetric
            icon={<Archive className="h-4 w-4" />}
            title={getInventoryStatusLabel('queued', t)}
            value={summary?.queued ?? 0}
          />
          <InventoryMetric
            icon={<ShieldCheck className="h-4 w-4" />}
            title={getInventoryStatusLabel('ready', t)}
            value={summary?.ready ?? 0}
            tone="success"
          />
          <InventoryMetric
            icon={<RefreshCcw className="h-4 w-4" />}
            title={getInventoryStatusLabel('needs_refresh', t)}
            value={summary?.needs_refresh ?? 0}
            tone="warning"
          />
          <InventoryMetric
            icon={<TimerReset className="h-4 w-4" />}
            title={getInventoryStatusLabel('no_quota', t)}
            value={summary?.no_quota ?? 0}
            tone="info"
          />
          <InventoryMetric
            icon={<TriangleAlert className="h-4 w-4" />}
            title={getInventoryStatusLabel('failed', t)}
            value={summary?.failed ?? 0}
            tone="destructive"
          />
          <InventoryMetric
            icon={<KeyRound className="h-4 w-4" />}
            title={t('inventory.metrics.total', { defaultValue: 'Total records' })}
            value={summary?.total ?? records.length}
          />
        </div>

        <PagePanel className="space-y-4">
          <SectionHeader
            eyebrow={t('inventory.table.eyebrow', { defaultValue: 'Vault view' })}
            title={t('inventory.table.title', { defaultValue: 'Admission inventory records' })}
            description={t('inventory.table.description', {
              defaultValue:
                'This table only covers vault inventory. Runtime activation and quarantine are still managed from Accounts.',
            })}
          />
          <StandardDataTable
            columns={inventoryColumns}
            data={filteredRecords}
            density="comfortable"
            className="min-h-[36rem] border border-border/60 bg-background/[0.5] shadow-none backdrop-blur-[2px]"
            searchPlaceholder={t('inventory.searchPlaceholder', {
              defaultValue: 'Search by email, label, account ID, or admission reason…',
            })}
            emptyText={
              isSummaryLoading || isRecordsLoading
                ? t('inventory.loading', { defaultValue: 'Loading inventory…' })
                : t('inventory.empty', { defaultValue: 'No inventory records match the current filter.' })
            }
          />
        </PagePanel>
      </div>
    </div>
  )
}
