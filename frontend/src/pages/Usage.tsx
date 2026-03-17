import { useMemo, useState } from 'react'
import { type ColumnDef } from '@tanstack/react-table'
import { useQuery } from '@tanstack/react-query'
import { format, subDays } from 'date-fns'
import { Download } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { usageApi } from '@/api/usage'
import type { HourlyTenantUsageTotalPoint } from '@/api/types'
import {
  PageIntro,
  PagePanel,
  ReportShell,
  SectionHeader,
} from '@/components/layout/page-archetypes'
import { Button } from '@/components/ui/button'
import { StandardDataTable } from '@/components/ui/standard-data-table'
import { TrendChart } from '@/components/ui/trend-chart'
import { formatExactCount } from '@/lib/count-number-format'
import { formatPercent, resolveLocale } from '@/lib/i18n-format'
import { cn } from '@/lib/utils'

interface TopKeyRow {
  name: string
  tenantId: string
  apiKeyId: string
  requests: number
  share: number
}

function shareBarClass(share: number) {
  if (share >= 60) {
    return 'bg-rose-500'
  }
  if (share >= 30) {
    return 'bg-warning'
  }
  return 'bg-success'
}

const tableSurfaceClassName = 'h-[420px] border-0 bg-transparent shadow-none'
const chartEmptyStateClassName =
  'flex h-[360px] items-center justify-center rounded-[1.2rem] border border-dashed border-border/60 bg-muted/20 text-sm text-muted-foreground'

export default function Usage() {
  const { t, i18n } = useTranslation()
  const locale = resolveLocale(i18n.resolvedLanguage ?? i18n.language)
  const [{ startTs, endTs }] = useState(() => {
    const endTs = Math.floor(Date.now() / 1000)
    const startTs = Math.floor(subDays(new Date(), 30).getTime() / 1000)
    return { startTs, endTs }
  })
  const xAxisDateFormatter = useMemo(
    () =>
      new Intl.DateTimeFormat(locale, {
        month: 'short',
        day: 'numeric',
      }),
    [locale],
  )

  const { data: trendData, isLoading: isLoadingTrends } = useQuery({
    queryKey: ['tenantHourlyTrends', startTs, endTs],
    queryFn: () =>
      usageApi.getHourlyTenantTrends({
        start_ts: startTs,
        end_ts: endTs,
        limit: 30 * 24,
      }),
    refetchInterval: 60_000,
  })

  const { data: leaderboard, isLoading: isLoadingLeaderboard } = useQuery({
    queryKey: ['usageLeaderboard', startTs, endTs],
    queryFn: () => usageApi.getLeaderboard({ start_ts: startTs, end_ts: endTs }),
    refetchInterval: 60_000,
  })

  const dailyVolume =
    trendData?.items.reduce((acc: Record<string, number>, curr: HourlyTenantUsageTotalPoint) => {
      const dayStr = format(new Date(curr.hour_start * 1000), 'yyyy-MM-dd')
      acc[dayStr] = (acc[dayStr] || 0) + curr.request_count
      return acc
    }, {}) ?? {}

  const chartData = Object.entries(dailyVolume)
    .map(([date, requests]) => ({
      timestamp: new Date(date).toISOString(),
      requests,
    }))
    .sort((left, right) => new Date(left.timestamp).getTime() - new Date(right.timestamp).getTime())

  const keysUsage = useMemo(() => leaderboard?.api_keys ?? [], [leaderboard?.api_keys])
  const totalRequests = useMemo(
    () => keysUsage.reduce((sum, item) => sum + item.total_requests, 0),
    [keysUsage],
  )

  const topKeyRows = useMemo<TopKeyRow[]>(
    () =>
      keysUsage
        .map((item) => {
          const tenantLabel = item.tenant_id?.trim()
          const share = totalRequests > 0 ? (item.total_requests / totalRequests) * 100 : 0
          return {
            name: tenantLabel || t('usage.topKeys.keyFallback', { keyId: item.api_key_id.split('-')[0] }),
            tenantId: item.tenant_id,
            apiKeyId: item.api_key_id,
            requests: item.total_requests,
            share,
          }
        })
        .sort((left, right) => right.requests - left.requests),
    [keysUsage, t, totalRequests],
  )

  const topKeyColumns = useMemo<ColumnDef<TopKeyRow>[]>(
    () => [
      {
        id: 'name',
        header: t('usage.topKeys.columns.name'),
        accessorFn: (row) => row.name.toLowerCase(),
        cell: ({ row }) => (
          <div className="min-w-[220px] space-y-1">
            <div className="font-medium">{row.original.name}</div>
            <div className="truncate text-xs font-mono text-muted-foreground">
              {t('usage.topKeys.columns.apiKey')}: {row.original.apiKeyId}
            </div>
          </div>
        ),
      },
      {
        id: 'requests',
        header: t('usage.topKeys.columns.requests'),
        accessorKey: 'requests',
        cell: ({ row }) => (
          <span className="font-mono tabular-nums">
            {formatExactCount(row.original.requests, locale)}
          </span>
        ),
      },
      {
        id: 'share',
        header: t('usage.topKeys.columns.share'),
        accessorKey: 'share',
        cell: ({ row }) => {
          const share = Math.max(0, Math.min(100, row.original.share))
          return (
            <div className="min-w-[140px] space-y-1">
              <div className="text-xs tabular-nums text-muted-foreground">
                {formatPercent(share, {
                  locale,
                  inputScale: 'percent',
                  minimumFractionDigits: 1,
                  maximumFractionDigits: 1,
                })}
              </div>
              <div className="h-1.5 overflow-hidden rounded-full bg-muted">
                <div
                  className={cn('h-full transition-[width] duration-300', shareBarClass(share))}
                  style={{ width: `${share}%` }}
                />
              </div>
            </div>
          )
        },
      },
    ],
    [locale, t],
  )

  const handleExportTopKeysCsv = () => {
    const rows = [
      [
        t('usage.topKeys.columns.name'),
        t('usage.topKeys.columns.requests'),
        t('usage.topKeys.columns.share'),
        t('usage.topKeys.columns.apiKey'),
        t('usage.topKeys.columns.tenant'),
      ],
      ...topKeyRows.map((row) => [
        row.name,
        String(row.requests),
        row.share.toFixed(4),
        row.apiKeyId,
        row.tenantId,
      ]),
    ]
    const escapeCsvField = (value: string) => {
      if (value.includes('"') || value.includes(',') || value.includes('\n')) {
        return `"${value.replaceAll('"', '""')}"`
      }
      return value
    }
    const csvContent = `${rows.map((line) => line.map(escapeCsvField).join(',')).join('\n')}\n`
    const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' })
    const url = URL.createObjectURL(blob)
    const anchor = document.createElement('a')
    anchor.href = url
    anchor.download = `usage-top-keys-${Date.now()}.csv`
    anchor.click()
    URL.revokeObjectURL(url)
  }

  return (
    <div className="flex-1 w-full overflow-y-auto p-4 sm:p-6 lg:p-8">
      <ReportShell
        intro={
          <PageIntro
            archetype="detail"
            title={t('usage.title')}
            description={t('usage.subtitle')}
            actions={
              <Button
                variant="outline"
                onClick={handleExportTopKeysCsv}
                disabled={topKeyRows.length === 0}
              >
                <Download className="mr-2 h-4 w-4" />
                {t('usage.actions.export')}
              </Button>
            }
          />
        }
        rail={
          <PagePanel tone="secondary" className="space-y-5">
            <SectionHeader
              title={t('usage.topKeys.title')}
              description={t('usage.topKeys.subtitle')}
            />
            <div className="min-h-0">
              {isLoadingLeaderboard ? (
                <div className="space-y-3">
                  {Array.from({ length: 6 }).map((_, index) => (
                    <div key={index} className="h-10 animate-pulse rounded-xl bg-slate-200/70 dark:bg-slate-800/70" />
                  ))}
                </div>
              ) : (
                <StandardDataTable
                  columns={topKeyColumns}
                  data={topKeyRows}
                  density="compact"
                  defaultPageSize={8}
                  pageSizeOptions={[8, 16, 32, 64]}
                  className={tableSurfaceClassName}
                  emptyText={t('usage.topKeys.empty')}
                  searchPlaceholder={t('usage.topKeys.searchPlaceholder')}
                  searchFn={(row, keyword) =>
                    `${row.name} ${row.tenantId} ${row.apiKeyId}`.toLowerCase().includes(keyword)
                  }
                />
              )}
            </div>
          </PagePanel>
        }
        lead={
          <PagePanel className="space-y-5">
            <SectionHeader
              title={t('usage.chart.title')}
              description={t('usage.chart.subtitle')}
            />
            {isLoadingTrends ? (
              <div className="h-[360px] animate-pulse rounded-[1.2rem] bg-slate-200/70 dark:bg-slate-800/70" />
            ) : chartData.length === 0 ? (
              <div className={chartEmptyStateClassName}>{t('usage.chart.empty')}</div>
            ) : (
              <TrendChart
                data={chartData}
                lines={[
                  {
                    dataKey: 'requests',
                    name: t('usage.chart.requests'),
                    stroke: 'var(--chart-1)',
                  },
                ]}
                height={360}
                locale={locale}
                valueFormatter={(value) => formatExactCount(value, locale)}
                xAxisFormatter={(value) => xAxisDateFormatter.format(new Date(value))}
              />
            )}
          </PagePanel>
        }
      />
    </div>
  )
}
