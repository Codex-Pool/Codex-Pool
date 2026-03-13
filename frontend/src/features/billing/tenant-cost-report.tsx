import { useMemo, useState } from 'react'
import type { ColumnDef } from '@tanstack/react-table'
import { useQuery } from '@tanstack/react-query'
import { subDays } from 'date-fns'
import { useTranslation } from 'react-i18next'

import { requestLogsApi, type RequestAuditLogItem } from '@/api/requestLogs'
import { tenantKeysApi } from '@/api/tenantKeys'
import { tenantUsageApi } from '@/api/tenantUsage'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { StandardDataTable } from '@/components/ui/standard-data-table'
import { TrendChart } from '@/components/ui/trend-chart'
import { formatMicrousd } from '@/lib/cost-format'
import { formatDateTime, formatNumber } from '@/lib/i18n-format'
import { localizeHttpStatusDisplay } from '@/api/errorI18n'

type BillingGranularity = 'day' | 'month'

function resolveDefaultRange() {
  const endTs = Math.floor(Date.now() / 1000)
  const startTs = Math.floor(subDays(new Date(), 30).getTime() / 1000)
  return { startTs, endTs }
}

function bucketTimestamp(hourStart: number, granularity: BillingGranularity) {
  const date = new Date(hourStart * 1000)
  if (granularity === 'month') {
    return new Date(date.getFullYear(), date.getMonth(), 1).getTime()
  }
  return new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime()
}

export function TenantCostReportPage() {
  const { t, i18n } = useTranslation()
  const locale = i18n.resolvedLanguage ?? i18n.language
  const [granularity, setGranularity] = useState<BillingGranularity>('day')
  const [selectedApiKeyId, setSelectedApiKeyId] = useState<string>('all')
  const { startTs, endTs } = useMemo(() => resolveDefaultRange(), [])
  const effectiveApiKeyId = selectedApiKeyId === 'all' ? undefined : selectedApiKeyId

  const { data: keys = [] } = useQuery({
    queryKey: ['tenantKeys', 'costReports'],
    queryFn: () => tenantKeysApi.list(),
    staleTime: 60_000,
  })

  const { data: summary } = useQuery({
    queryKey: ['tenantCostSummary', startTs, endTs, effectiveApiKeyId],
    queryFn: () =>
      tenantUsageApi.summary({
        start_ts: startTs,
        end_ts: endTs,
        api_key_id: effectiveApiKeyId,
      }),
    staleTime: 30_000,
  })

  const { data: requestLogs } = useQuery({
    queryKey: ['tenantCostLogs', startTs, endTs, effectiveApiKeyId],
    queryFn: () =>
      requestLogsApi.tenantList({
        start_ts: startTs,
        end_ts: endTs,
        limit: 200,
        api_key_id: effectiveApiKeyId,
      }),
    staleTime: 30_000,
  })

  const chartData = useMemo(() => {
    const buckets = new Map<number, number>()
    for (const point of summary?.dashboard_metrics?.token_trends ?? []) {
      const cost = point.estimated_cost_microusd
      if (typeof cost !== 'number') {
        continue
      }
      const bucket = bucketTimestamp(point.hour_start, granularity)
      buckets.set(bucket, (buckets.get(bucket) ?? 0) + cost)
    }

    return Array.from(buckets.entries())
      .sort((left, right) => left[0] - right[0])
      .map(([timestamp, cost]) => ({
        timestamp,
        cost,
      }))
  }, [granularity, summary?.dashboard_metrics?.token_trends])

  const averageCostMicrousd = useMemo(() => {
    const totalCost = summary?.estimated_cost_microusd
    const totalRequests = summary?.tenant_api_key_total_requests ?? 0
    if (typeof totalCost !== 'number' || totalRequests <= 0) {
      return undefined
    }
    return Math.round(totalCost / totalRequests)
  }, [summary?.estimated_cost_microusd, summary?.tenant_api_key_total_requests])

  const columns = useMemo<ColumnDef<RequestAuditLogItem>[]>(
    () => [
      {
        accessorKey: 'created_at',
        header: t('costReports.logs.columns.time'),
        cell: ({ row }) =>
          formatDateTime(row.original.created_at, {
            locale,
            preset: 'datetime',
            fallback: '-',
          }),
      },
      {
        accessorKey: 'request_id',
        header: t('costReports.logs.columns.requestId'),
        cell: ({ row }) => row.original.request_id ?? '-',
      },
      {
        accessorKey: 'model',
        header: t('costReports.logs.columns.model'),
        cell: ({ row }) => row.original.model ?? '-',
      },
      {
        accessorKey: 'status_code',
        header: t('costReports.logs.columns.status'),
        cell: ({ row }) =>
          localizeHttpStatusDisplay(t, row.original.status_code, t('errors.common.failed')).label,
      },
      {
        accessorKey: 'estimated_cost_microusd',
        header: t('costReports.logs.columns.cost'),
        cell: ({ row }) => formatMicrousd(row.original.estimated_cost_microusd, { locale }),
      },
    ],
    [locale, t],
  )

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader className="space-y-2">
          <CardTitle>{t('costReports.tenant.title')}</CardTitle>
          <CardDescription>{t('costReports.tenant.description')}</CardDescription>
        </CardHeader>
        <CardContent className="grid gap-4 md:grid-cols-3">
          <div className="rounded-xl border border-border/60 bg-muted/20 p-4">
            <p className="text-sm text-muted-foreground">{t('costReports.summary.totalCost')}</p>
            <p className="mt-2 text-2xl font-semibold">
              {formatMicrousd(summary?.estimated_cost_microusd, { locale })}
            </p>
          </div>
          <div className="rounded-xl border border-border/60 bg-muted/20 p-4">
            <p className="text-sm text-muted-foreground">{t('costReports.summary.totalRequests')}</p>
            <p className="mt-2 text-2xl font-semibold">
              {formatNumber(summary?.tenant_api_key_total_requests, {
                locale,
                maximumFractionDigits: 0,
              })}
            </p>
          </div>
          <div className="rounded-xl border border-border/60 bg-muted/20 p-4">
            <p className="text-sm text-muted-foreground">
              {t('costReports.summary.avgCostPerRequest')}
            </p>
            <p className="mt-2 text-2xl font-semibold">
              {formatMicrousd(averageCostMicrousd, {
                locale,
                minimumFractionDigits: 4,
                maximumFractionDigits: 4,
              })}
            </p>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
          <div>
            <CardTitle>{t('costReports.chart.title')}</CardTitle>
            <CardDescription>{t('costReports.chart.description')}</CardDescription>
          </div>
          <div className="flex flex-col gap-3 sm:flex-row">
            <Select value={selectedApiKeyId} onValueChange={setSelectedApiKeyId}>
              <SelectTrigger className="w-full sm:w-[220px]">
                <SelectValue placeholder={t('costReports.filters.apiKey')} />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">{t('costReports.filters.allApiKeys')}</SelectItem>
                {keys.map((key) => (
                <SelectItem key={key.id} value={key.id}>
                    {key.name}
                </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <Select
              value={granularity}
              onValueChange={(value) => setGranularity(value as BillingGranularity)}
            >
              <SelectTrigger className="w-full sm:w-[180px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="day">{t('costReports.filters.day')}</SelectItem>
                <SelectItem value="month">{t('costReports.filters.month')}</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </CardHeader>
        <CardContent>
          {chartData.length > 0 ? (
            <TrendChart
              data={chartData}
              lines={[
                {
                  dataKey: 'cost',
                  stroke: '#1d4ed8',
                  name: t('costReports.chart.series.cost'),
                },
              ]}
              height={320}
              locale={locale}
              valueFormatter={(value) => formatMicrousd(value, { locale })}
            />
          ) : (
            <p className="text-sm text-muted-foreground">{t('costReports.chart.empty')}</p>
          )}
        </CardContent>
      </Card>

      <StandardDataTable
        columns={columns}
        data={requestLogs?.items ?? []}
        defaultPageSize={10}
        searchPlaceholder={t('costReports.logs.searchPlaceholder')}
        emptyText={t('costReports.logs.empty')}
        enableSearch
        searchFn={(row, keyword) =>
          [row.request_id, row.model, String(row.status_code)]
            .filter(Boolean)
            .join(' ')
            .toLowerCase()
            .includes(keyword)
        }
        filters={
          <div className="text-sm text-muted-foreground">
            {t('costReports.logs.title')}
          </div>
        }
      />
    </div>
  )
}
