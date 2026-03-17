import { useMemo, useState } from 'react'
import { type ColumnDef } from '@tanstack/react-table'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'

import { tenantKeysApi } from '@/api/tenantKeys'
import { tenantUsageApi } from '@/api/tenantUsage'
import {
  PageIntro,
  PagePanel,
  ReportShell,
  SectionHeader,
} from '@/components/layout/page-archetypes'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { StandardDataTable } from '@/components/ui/standard-data-table'
import { TrendChart } from '@/components/ui/trend-chart'
import { formatExactCount } from '@/lib/count-number-format'
import { resolveLocale } from '@/lib/i18n-format'
import { currentRangeByDays } from '@/tenant/lib/format'

type RangePreset = 1 | 7 | 30

interface KeyUsageRow {
  apiKeyId: string
  apiKeyLabel: string
  apiKeyMeta?: string
  requests: number
}

interface HourlyUsageRow {
  hourStart: number
  requests: number
}

const tableSurfaceClassName = 'h-[420px] border-0 bg-transparent shadow-none'
const chartEmptyStateClassName =
  'flex h-[340px] items-center justify-center rounded-[1.2rem] border border-dashed border-border/60 bg-muted/20 text-sm text-muted-foreground'

export function TenantUsagePage() {
  const { t, i18n } = useTranslation()
  const locale = resolveLocale(i18n.resolvedLanguage ?? i18n.language)
  const [rangePreset, setRangePreset] = useState<RangePreset>(7)
  const [apiKeyId, setApiKeyId] = useState<string>('all')

  const range = useMemo(() => currentRangeByDays(rangePreset), [rangePreset])
  const selectedApiKeyId = apiKeyId === 'all' ? undefined : apiKeyId
  const hourlyAxisFormatter = useMemo(
    () =>
      new Intl.DateTimeFormat(locale, {
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
        hour12: false,
      }),
    [locale],
  )

  const { data: keys = [] } = useQuery({
    queryKey: ['tenantKeys', 'usage'],
    queryFn: () => tenantKeysApi.list(),
    staleTime: 60_000,
  })

  const { data: trends } = useQuery({
    queryKey: ['tenantUsageTrends', range.start_ts, range.end_ts, selectedApiKeyId],
    queryFn: () =>
      tenantUsageApi.trendsHourly({
        ...range,
        api_key_id: selectedApiKeyId,
        limit: Math.max(24, rangePreset * 24),
      }),
    refetchInterval: 60_000,
  })

  const { data: apiKeyLeaderboard } = useQuery({
    queryKey: ['tenantApiKeyLeaderboard', range.start_ts, range.end_ts, selectedApiKeyId],
    queryFn: () =>
      tenantUsageApi.leaderboardApiKeys({
        ...range,
        limit: 50,
        api_key_id: selectedApiKeyId,
      }),
    refetchInterval: 60_000,
  })

  const keyMetaById = useMemo(
    () =>
      new Map(
        keys.map((item) => [
          item.id,
          {
            name: item.name,
            prefix: item.key_prefix,
          },
        ]),
      ),
    [keys],
  )

  const chartData = useMemo(
    () =>
      (trends?.tenant_api_key_totals ?? []).map((point) => ({
        timestamp: point.hour_start * 1000,
        requests: point.request_count,
      })),
    [trends?.tenant_api_key_totals],
  )

  const leaderboardRows = useMemo<KeyUsageRow[]>(
    () =>
      (apiKeyLeaderboard?.items ?? []).map((item) => {
        const matchedKey = keyMetaById.get(item.api_key_id)
        return {
          apiKeyId: item.api_key_id,
          apiKeyLabel: matchedKey?.name ?? item.api_key_id,
          apiKeyMeta: matchedKey?.prefix ?? (matchedKey ? item.api_key_id : undefined),
          requests: item.total_requests,
        }
      }),
    [apiKeyLeaderboard?.items, keyMetaById],
  )

  const hourlyRows = useMemo<HourlyUsageRow[]>(
    () =>
      (trends?.tenant_api_key_totals ?? []).map((item) => ({
        hourStart: item.hour_start,
        requests: item.request_count,
      })),
    [trends?.tenant_api_key_totals],
  )

  const keyColumns = useMemo<ColumnDef<KeyUsageRow>[]>(
    () => [
      {
        id: 'apiKey',
        header: t('tenantUsage.columns.apiKey'),
        accessorFn: (row) => `${row.apiKeyLabel} ${row.apiKeyId}`.toLowerCase(),
        cell: ({ row }) => (
          <div className="min-w-[220px] space-y-1">
            <div className="font-medium">{row.original.apiKeyLabel}</div>
            {row.original.apiKeyMeta ? (
              <div className="text-xs font-mono text-muted-foreground">{row.original.apiKeyMeta}</div>
            ) : null}
          </div>
        ),
      },
      {
        id: 'requests',
        header: t('tenantUsage.columns.requests'),
        accessorKey: 'requests',
        cell: ({ row }) => (
          <span className="font-mono tabular-nums">
            {formatExactCount(row.original.requests, locale)}
          </span>
        ),
      },
    ],
    [locale, t],
  )

  const hourlyColumns = useMemo<ColumnDef<HourlyUsageRow>[]>(
    () => [
      {
        id: 'hour',
        header: t('tenantUsage.columns.time'),
        accessorFn: (row) => row.hourStart,
        cell: ({ row }) => hourlyAxisFormatter.format(new Date(row.original.hourStart * 1000)),
      },
      {
        id: 'requests',
        header: t('tenantUsage.columns.requests'),
        accessorKey: 'requests',
        cell: ({ row }) => (
          <span className="font-mono tabular-nums">
            {formatExactCount(row.original.requests, locale)}
          </span>
        ),
      },
    ],
    [hourlyAxisFormatter, locale, t],
  )

  const rangeLabel = (days: RangePreset) => {
    if (days === 1) {
      return t('tenantUsage.filters.range.last24Hours')
    }
    if (days === 7) {
      return t('tenantUsage.filters.range.last7Days')
    }
    return t('tenantUsage.filters.range.last30Days')
  }

  return (
    <div className="flex-1 w-full overflow-y-auto p-4 sm:p-6 lg:p-8">
      <ReportShell
        intro={
          <PageIntro
            archetype="detail"
            title={t('tenantUsage.title')}
            description={t('tenantUsage.subtitle')}
            meta={
              <span className="inline-flex items-center rounded-full border border-slate-200/80 bg-white/80 px-3 py-1 text-xs font-medium text-slate-600 dark:border-slate-800 dark:bg-slate-950/60 dark:text-slate-300">
                {rangeLabel(rangePreset)}
              </span>
            }
          />
        }
        toolbar={
          <PagePanel tone="secondary">
            <div className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-2">
                <p className="text-[11px] font-semibold uppercase tracking-[0.14em] text-slate-500 dark:text-slate-400">
                  {t('tenantUsage.filters.rangeAriaLabel')}
                </p>
                <Select
                  value={String(rangePreset)}
                  onValueChange={(value) => setRangePreset(Number(value) as RangePreset)}
                >
                  <SelectTrigger className="w-full" aria-label={t('tenantUsage.filters.rangeAriaLabel')}>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="1">{t('tenantUsage.filters.range.last24Hours')}</SelectItem>
                    <SelectItem value="7">{t('tenantUsage.filters.range.last7Days')}</SelectItem>
                    <SelectItem value="30">{t('tenantUsage.filters.range.last30Days')}</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-2">
                <p className="text-[11px] font-semibold uppercase tracking-[0.14em] text-slate-500 dark:text-slate-400">
                  {t('tenantUsage.filters.apiKeyAriaLabel')}
                </p>
                <Select value={apiKeyId} onValueChange={setApiKeyId}>
                  <SelectTrigger className="w-full" aria-label={t('tenantUsage.filters.apiKeyAriaLabel')}>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">{t('tenantUsage.filters.apiKeyAll')}</SelectItem>
                    {keys.map((item) => (
                      <SelectItem key={item.id} value={item.id}>
                        {item.name} ({item.key_prefix})
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>
          </PagePanel>
        }
        rail={
          <PagePanel tone="secondary" className="space-y-5">
            <SectionHeader
              title={t('tenantUsage.leaderboard.title')}
              description={t('tenantUsage.leaderboard.description')}
            />
            <StandardDataTable
              columns={keyColumns}
              data={leaderboardRows}
              defaultPageSize={10}
              pageSizeOptions={[10, 20, 50]}
              density="compact"
              className={tableSurfaceClassName}
              emptyText={t('tenantUsage.leaderboard.empty')}
            />
          </PagePanel>
        }
        lead={
          <PagePanel className="space-y-5">
            <SectionHeader
              title={t('tenantUsage.trend.title')}
              description={t('tenantUsage.trend.description')}
            />
            {chartData.length === 0 ? (
              <div className={chartEmptyStateClassName}>{t('tenantUsage.trend.empty')}</div>
            ) : (
              <TrendChart
                data={chartData}
                lines={[
                  {
                    dataKey: 'requests',
                    name: t('tenantUsage.columns.requests'),
                    stroke: 'var(--chart-1)',
                  },
                ]}
                height={340}
                locale={locale}
                valueFormatter={(value) => formatExactCount(value, locale)}
                xAxisFormatter={(value) => hourlyAxisFormatter.format(new Date(value))}
              />
            )}
          </PagePanel>
        }
      >
        <PagePanel tone="secondary" className="space-y-5">
          <SectionHeader
            title={t('tenantUsage.hourly.title')}
            description={t('tenantUsage.hourly.description')}
          />
          <StandardDataTable
            columns={hourlyColumns}
            data={hourlyRows}
            defaultPageSize={10}
            pageSizeOptions={[10, 20, 50]}
            density="compact"
            className={tableSurfaceClassName}
            emptyText={t('tenantUsage.hourly.empty')}
            enableSearch={false}
          />
        </PagePanel>
      </ReportShell>
    </div>
  )
}
