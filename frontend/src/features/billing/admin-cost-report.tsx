import { useMemo, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import {
  Button,
  Chip,
  Input,
  Pagination,
  Select,
  SelectItem,
  Spinner,
  Table,
  TableBody,
  TableCell,
  TableColumn,
  TableHeader,
  TableRow,
  type Selection,
} from '@heroui/react'
import { subDays } from 'date-fns'
import { BarChart3, RefreshCcw, Search } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import {
  Area,
  AreaChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'

import { adminTenantsApi } from '@/api/adminTenants'
import { dashboardApi } from '@/api/dashboard'
import { localizeHttpStatusDisplay } from '@/api/errorI18n'
import { requestLogsApi, type RequestAuditLogItem } from '@/api/requestLogs'
import type { SystemCapabilitiesResponse } from '@/api/types'
import {
  DockedPageIntro,
  PageContent,
} from '@/components/layout/page-archetypes'
import { Card, CardBody, CardHeader } from '@/components/ui/card'
import { useChartTheme } from '@/lib/chart-theme'
import { formatMicrousd } from '@/lib/cost-format'
import { formatDateTime, formatNumber, resolveLocale } from '@/lib/i18n-format'

type BillingGranularity = 'day' | 'month'

interface AdminCostReportPageProps {
  capabilities: SystemCapabilitiesResponse
}

const TABLE_PAGE_SIZE_OPTIONS = [10, 20, 50]

function normalizeSelection(selection: Selection) {
  if (selection === 'all') {
    return ''
  }

  const [first] = Array.from(selection)
  return first === undefined ? '' : String(first)
}

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

function formatBucketLabel(
  timestamp: number,
  granularity: BillingGranularity,
  locale: string,
) {
  return new Intl.DateTimeFormat(
    resolveLocale(locale),
    granularity === 'month'
      ? {
        year: 'numeric',
        month: '2-digit',
      }
      : {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
      },
  ).format(new Date(timestamp))
}

function matchRequestLogSearch(
  row: RequestAuditLogItem,
  keyword: string,
  tenantNameById: Map<string, string>,
) {
  return [
    row.request_id,
    row.model,
    row.api_key_id,
    row.tenant_id ? tenantNameById.get(row.tenant_id) ?? row.tenant_id : '',
    String(row.status_code),
  ]
    .filter(Boolean)
    .join(' ')
    .toLowerCase()
    .includes(keyword)
}

export function AdminCostReportPage({ capabilities }: AdminCostReportPageProps) {
  const { t, i18n } = useTranslation()
  const locale = i18n.resolvedLanguage ?? i18n.language
  const { textColor: chartTextColor, gridColor: chartGridColor, tooltipStyle: chartTooltipStyle } = useChartTheme()

  const [granularity, setGranularity] = useState<BillingGranularity>('day')
  const [selectedTenantId, setSelectedTenantId] = useState<string>('all')
  const [searchValue, setSearchValue] = useState('')
  const [rowsPerPage, setRowsPerPage] = useState(10)
  const [currentPage, setCurrentPage] = useState(1)
  const { startTs, endTs } = useMemo(() => resolveDefaultRange(), [])
  const effectiveTenantId = selectedTenantId === 'all' ? undefined : selectedTenantId

  const { data: tenants = [], isFetching: isFetchingTenants } = useQuery({
    queryKey: ['adminTenants', 'costReports'],
    queryFn: () => adminTenantsApi.listTenants(),
    enabled: capabilities.features.multi_tenant,
    staleTime: 60_000,
  })

  const { data: summary, isFetching: isFetchingSummary, refetch: refetchSummary } = useQuery({
    queryKey: ['adminCostSummary', startTs, endTs, effectiveTenantId],
    queryFn: () =>
      dashboardApi.getUsageSummary({
        start_ts: startTs,
        end_ts: endTs,
        tenant_id: effectiveTenantId,
      }),
    staleTime: 30_000,
  })

  const { data: requestLogs, isFetching: isFetchingLogs, refetch: refetchLogs } = useQuery({
    queryKey: ['adminCostLogs', startTs, endTs, effectiveTenantId],
    queryFn: () =>
      requestLogsApi.adminList({
        start_ts: startTs,
        end_ts: endTs,
        limit: 200,
        tenant_id: effectiveTenantId,
      }),
    staleTime: 30_000,
  })

  const tenantNameById = useMemo(
    () => new Map(tenants.map((tenant) => [tenant.id, tenant.name])),
    [tenants],
  )

  const tenantOptions = useMemo(
    () => [
      { key: 'all', label: t('costReports.filters.allTenants') },
      ...tenants.map((tenant) => ({
        key: tenant.id,
        label: tenant.name,
      })),
    ],
    [t, tenants],
  )

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
        label: formatBucketLabel(timestamp, granularity, locale),
        cost,
      }))
  }, [granularity, locale, summary?.dashboard_metrics?.token_trends])

  const averageCostMicrousd = useMemo(() => {
    const totalCost = summary?.estimated_cost_microusd
    const totalRequests = summary?.account_total_requests ?? 0
    if (typeof totalCost !== 'number' || totalRequests <= 0) {
      return undefined
    }
    return Math.round(totalCost / totalRequests)
  }, [summary?.account_total_requests, summary?.estimated_cost_microusd])

  const logs = useMemo(() => requestLogs?.items ?? [], [requestLogs?.items])
  const filteredLogs = useMemo(() => {
    const keyword = searchValue.trim().toLowerCase()
    if (!keyword) {
      return logs
    }
    return logs.filter((row) => matchRequestLogSearch(row, keyword, tenantNameById))
  }, [logs, searchValue, tenantNameById])

  const totalPages = Math.max(1, Math.ceil(filteredLogs.length / rowsPerPage))
  const resolvedPage = Math.min(currentPage, totalPages)
  const paginatedLogs = useMemo(() => {
    const start = (resolvedPage - 1) * rowsPerPage
    return filteredLogs.slice(start, start + rowsPerPage)
  }, [filteredLogs, resolvedPage, rowsPerPage])
  const visibleRangeStart = filteredLogs.length === 0 ? 0 : (resolvedPage - 1) * rowsPerPage + 1
  const visibleRangeEnd =
    filteredLogs.length === 0 ? 0 : Math.min(filteredLogs.length, resolvedPage * rowsPerPage)

  const metricCards = [
    {
      title: t('costReports.summary.totalCost'),
      value: formatMicrousd(summary?.estimated_cost_microusd, { locale }),
      description: t('billing.antigravity.totalCostHint'),
      toneClassName: 'bg-success/10 text-success',
    },
    {
      title: t('costReports.summary.totalRequests'),
      value: formatNumber(summary?.account_total_requests, {
        locale,
        maximumFractionDigits: 0,
      }),
      description: t('billing.antigravity.requestCountHint'),
      toneClassName: 'bg-primary/10 text-primary',
    },
    {
      title: t('costReports.summary.avgCostPerRequest'),
      value: formatMicrousd(averageCostMicrousd, {
        locale,
        minimumFractionDigits: 4,
        maximumFractionDigits: 4,
      }),
      description: t('billing.antigravity.avgCostHint'),
      toneClassName: 'bg-warning/10 text-warning',
    },
    {
      title: t('billing.antigravity.logCoverage'),
      value: formatNumber(logs.length, { locale, maximumFractionDigits: 0 }),
      description: t('billing.antigravity.logCoverageHint'),
      toneClassName: 'bg-secondary/10 text-secondary',
    },
  ]
  const isMultiTenant = capabilities.features.multi_tenant

  return (
    <PageContent className="space-y-6">
      <DockedPageIntro
        archetype="workspace"
        title={t('costReports.admin.title')}
        description={t('costReports.admin.description')}
        actions={(
          <Button
            color="primary"
            isLoading={isFetchingSummary || isFetchingLogs || isFetchingTenants}
            startContent={
              isFetchingSummary || isFetchingLogs || isFetchingTenants
                ? undefined
                : <RefreshCcw className="h-4 w-4" />
            }
            variant="flat"
            onPress={() => {
              void refetchSummary()
              void refetchLogs()
            }}
          >
            {t('common.refresh')}
          </Button>
        )}
      />

      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        {metricCards.map((metric) => (
          <Card key={metric.title}>
            <CardBody className="space-y-4 p-4">
              <div className={metric.toneClassName + ' flex h-10 w-10 items-center justify-center rounded-large'}>
                <BarChart3 className="h-4 w-4" />
              </div>
              <div className="space-y-2">
                <p className="text-xs font-semibold uppercase tracking-[0.16em] text-default-500">
                  {metric.title}
                </p>
                <p className="text-[clamp(1.55rem,3vw,2.15rem)] font-semibold leading-none tracking-[-0.045em] text-foreground">
                  {metric.value}
                </p>
                <p className="text-sm leading-6 text-default-600">
                  {metric.description}
                </p>
              </div>
            </CardBody>
          </Card>
        ))}
      </div>

      <div className="grid gap-6 xl:grid-cols-[minmax(0,1.3fr)_minmax(0,0.95fr)]">
        <Card>
          <CardHeader className="px-5 pb-3 pt-5">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t('costReports.chart.title')}
              </h2>
              <p className="text-sm leading-6 text-default-600">
                {t('costReports.chart.description')}
              </p>
            </div>
          </CardHeader>
          <CardBody className="px-5 pb-5 pt-1">
            {chartData.length > 0 ? (
              <ResponsiveContainer height={320} width="100%">
                <AreaChart data={chartData}>
                  <defs>
                    <linearGradient id="costReportGradient" x1="0" x2="0" y1="0" y2="1">
                      <stop offset="0%" stopColor="hsl(var(--heroui-success))" stopOpacity={0.28} />
                      <stop offset="100%" stopColor="hsl(var(--heroui-success))" stopOpacity={0} />
                    </linearGradient>
                  </defs>
                  <CartesianGrid stroke={chartGridColor} strokeDasharray="3 3" />
                  <XAxis axisLine={false} dataKey="label" tick={{ fill: chartTextColor, fontSize: 11 }} tickLine={false} />
                  <YAxis axisLine={false} tick={{ fill: chartTextColor, fontSize: 11 }} tickFormatter={(value) => formatMicrousd(value, { locale })} tickLine={false} />
                  <Tooltip contentStyle={chartTooltipStyle} formatter={(value) => formatMicrousd(Number(value), { locale })} />
                  <Area dataKey="cost" fill="url(#costReportGradient)" stroke="hsl(var(--heroui-success))" strokeWidth={2} type="monotone" />
                </AreaChart>
              </ResponsiveContainer>
            ) : (
              <div className="flex h-[320px] items-center justify-center rounded-large border border-dashed border-default-200 text-sm text-default-600">
                {t('costReports.chart.empty')}
              </div>
            )}
          </CardBody>
        </Card>

        <Card>
          <CardHeader className="px-5 pb-3 pt-5">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t('billing.antigravity.scopePanelTitle')}
              </h2>
              <p className="text-sm leading-6 text-default-600">
                {t('billing.antigravity.scopePanelDescription')}
              </p>
            </div>
          </CardHeader>
          <CardBody className="gap-4 px-5 pb-5 pt-1">
            {capabilities.features.multi_tenant ? (
              <Select
                aria-label={t('costReports.filters.tenant')}
                items={tenantOptions}
                selectedKeys={[selectedTenantId]}
                size="sm"
                onSelectionChange={(selection) => {
                  const nextValue = normalizeSelection(selection)
                  if (!nextValue) {
                    return
                  }
                  setCurrentPage(1)
                  setSelectedTenantId(nextValue)
                }}
              >
                {(item) => <SelectItem key={item.key}>{item.label}</SelectItem>}
              </Select>
            ) : null}

            <Select
              aria-label={t('billing.filters.granularityAriaLabel')}
              selectedKeys={[granularity]}
              size="sm"
              onSelectionChange={(selection) => {
                const nextValue = normalizeSelection(selection)
                if (!nextValue) {
                  return
                }
                setGranularity(nextValue as BillingGranularity)
              }}
            >
              <SelectItem key="day">{t('costReports.filters.day')}</SelectItem>
              <SelectItem key="month">{t('costReports.filters.month')}</SelectItem>
            </Select>

            <div className="grid gap-3 sm:grid-cols-2">
              <div className="rounded-large border border-default-200 bg-content2/55 px-4 py-4">
                <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                  {t('billing.antigravity.activeScope')}
                </div>
                <div className="mt-2 text-sm font-semibold text-foreground">
                  {effectiveTenantId
                    ? tenantNameById.get(effectiveTenantId) ?? effectiveTenantId
                    : t('costReports.filters.allTenants')}
                </div>
              </div>
              <div className="rounded-large border border-default-200 bg-content2/55 px-4 py-4">
                <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                  {t('billing.antigravity.activeGranularity')}
                </div>
                <div className="mt-2 text-sm font-semibold text-foreground">
                  {granularity === 'day' ? t('costReports.filters.day') : t('costReports.filters.month')}
                </div>
              </div>
            </div>
          </CardBody>
        </Card>
      </div>

      <Card>
        <CardHeader className="flex flex-col items-start gap-4 px-5 pb-3 pt-5">
          <div className="space-y-1">
            <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
              {t('costReports.logs.title')}
            </h2>
            <p className="text-sm leading-6 text-default-600">
              {t('costReports.admin.description')}
            </p>
          </div>

          <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
            <Input
              aria-label={t('costReports.logs.searchPlaceholder')}
              className="sm:max-w-sm"
              placeholder={t('costReports.logs.searchPlaceholder')}
              size="sm"
              startContent={<Search className="h-4 w-4 text-default-400" />}
              value={searchValue}
              onValueChange={(value) => {
                setCurrentPage(1)
                setSearchValue(value)
              }}
            />

            <div className="flex items-center gap-2 text-xs text-default-500">
              <span>{t('common.table.rowsPerPage')}</span>
              <Select
                aria-label={t('common.table.rowsPerPage')}
                className="w-[106px]"
                selectedKeys={[String(rowsPerPage)]}
                size="sm"
                onSelectionChange={(selection) => {
                  const nextValue = normalizeSelection(selection)
                  if (!nextValue) {
                    return
                  }
                  setCurrentPage(1)
                  setRowsPerPage(Number(nextValue))
                }}
              >
                {TABLE_PAGE_SIZE_OPTIONS.map((size) => (
                  <SelectItem key={String(size)}>{size}</SelectItem>
                ))}
              </Select>
            </div>
          </div>
        </CardHeader>

        <CardBody className="gap-4 px-5 pb-5 pt-0">
          <Table
            isHeaderSticky
            aria-label={t('costReports.logs.title')}
            classNames={{
              base: 'min-h-[26rem]',
              wrapper: 'bg-transparent px-0 py-0 shadow-none',
              th: 'bg-default-100/60 text-xs font-semibold uppercase tracking-[0.12em] text-default-500',
              td: 'align-top py-4 text-sm text-foreground',
              tr: 'data-[hover=true]:bg-content2/35 transition-colors',
              emptyWrapper: 'h-48',
            }}
          >
            {isMultiTenant ? (
              <TableHeader>
                <TableColumn>{t('costReports.logs.columns.time')}</TableColumn>
                <TableColumn>{t('costReports.logs.columns.tenant')}</TableColumn>
                <TableColumn>{t('costReports.logs.columns.requestId')}</TableColumn>
                <TableColumn>{t('costReports.logs.columns.model')}</TableColumn>
                <TableColumn>{t('costReports.logs.columns.status')}</TableColumn>
                <TableColumn>{t('costReports.logs.columns.cost')}</TableColumn>
              </TableHeader>
            ) : (
              <TableHeader>
                <TableColumn>{t('costReports.logs.columns.time')}</TableColumn>
                <TableColumn>{t('costReports.logs.columns.requestId')}</TableColumn>
                <TableColumn>{t('costReports.logs.columns.model')}</TableColumn>
                <TableColumn>{t('costReports.logs.columns.status')}</TableColumn>
                <TableColumn>{t('costReports.logs.columns.cost')}</TableColumn>
              </TableHeader>
            )}
            <TableBody
              emptyContent={(
                <div className="flex flex-col items-center gap-3 py-10 text-default-500">
                  <BarChart3 className="h-10 w-10 opacity-35" />
                  <div className="text-sm font-medium">{t('costReports.logs.empty')}</div>
                </div>
              )}
              isLoading={isFetchingLogs}
              items={paginatedLogs}
              loadingContent={<Spinner label={t('common.loading')} />}
            >
              {(row) => (
                isMultiTenant ? (
                  <TableRow key={row.id}>
                    <TableCell>
                      <div className="min-w-[160px] font-mono text-xs text-default-500">
                        {formatDateTime(row.created_at, {
                          locale,
                          preset: 'datetime',
                          fallback: '-',
                        })}
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="min-w-[180px] text-sm text-default-600">
                        {row.tenant_id
                          ? tenantNameById.get(row.tenant_id) ?? row.tenant_id
                          : t('costReports.filters.allTenants')}
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="min-w-[180px] font-mono text-xs text-default-500">
                        {row.request_id ?? '-'}
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="min-w-[180px] text-sm text-default-600">{row.model ?? '-'}</div>
                    </TableCell>
                    <TableCell>
                      <Chip
                        color={row.status_code >= 400 ? 'danger' : 'success'}
                        size="sm"
                        variant="flat"
                      >
                        {localizeHttpStatusDisplay(
                          t,
                          row.status_code,
                          t('errors.common.failed'),
                        ).label}
                      </Chip>
                    </TableCell>
                    <TableCell>
                      <div className="min-w-[120px] font-mono text-xs text-default-500">
                        {formatMicrousd(row.estimated_cost_microusd, { locale })}
                      </div>
                    </TableCell>
                  </TableRow>
                ) : (
                  <TableRow key={row.id}>
                    <TableCell>
                      <div className="min-w-[160px] font-mono text-xs text-default-500">
                        {formatDateTime(row.created_at, {
                          locale,
                          preset: 'datetime',
                          fallback: '-',
                        })}
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="min-w-[180px] font-mono text-xs text-default-500">
                        {row.request_id ?? '-'}
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="min-w-[180px] text-sm text-default-600">{row.model ?? '-'}</div>
                    </TableCell>
                    <TableCell>
                      <Chip
                        color={row.status_code >= 400 ? 'danger' : 'success'}
                        size="sm"
                        variant="flat"
                      >
                        {localizeHttpStatusDisplay(
                          t,
                          row.status_code,
                          t('errors.common.failed'),
                        ).label}
                      </Chip>
                    </TableCell>
                    <TableCell>
                      <div className="min-w-[120px] font-mono text-xs text-default-500">
                        {formatMicrousd(row.estimated_cost_microusd, { locale })}
                      </div>
                    </TableCell>
                  </TableRow>
                )
              )}
            </TableBody>
          </Table>

          <div className="flex flex-col gap-3 border-t border-default-200 pt-3 text-xs text-default-500 sm:flex-row sm:items-center sm:justify-between">
            <div className="tabular-nums">
              {t('common.table.range', {
                start: visibleRangeStart,
                end: visibleRangeEnd,
                total: filteredLogs.length,
              })}
            </div>
            <Pagination
              color="primary"
              isCompact
              page={resolvedPage}
              total={totalPages}
              onChange={setCurrentPage}
            />
          </div>
        </CardBody>
      </Card>
    </PageContent>
  )
}
