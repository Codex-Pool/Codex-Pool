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
import { Activity, DollarSign, Gauge, RefreshCcw, Search, TrendingUp, Zap } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import {
  Area,
  AreaChart,
  Bar,
  BarChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'

import { usageApi } from '@/api/usage'
import { Card, CardBody, CardHeader } from '@/components/ui/card'
import {
  DockedPageIntro,
  PageContent,
} from '@/components/layout/page-archetypes'
import { useChartTheme } from '@/lib/chart-theme'
import {
  buildDashboardKpis,
  buildModelDistribution,
  buildTopApiKeys,
  buildTokenTrend,
  groupTenantHourlyUsageByDay,
} from '@/features/usage/contracts'
import { formatDurationMs } from '@/lib/duration-format'
import { formatCompactNumber } from '@/lib/i18n-format'

const TABLE_PAGE_SIZE_OPTIONS = [5, 10, 20]

function normalizeSelection(selection: Selection) {
  if (selection === 'all') {
    return ''
  }

  const [first] = Array.from(selection)
  return first === undefined ? '' : String(first)
}

function formatCurrency(value: number): string {
  return `$${value.toFixed(2)}`
}

function matchTopKeySearch(
  item: { apiKeyId: string; tenantId: string; requests: number },
  keyword: string,
) {
  return `${item.apiKeyId} ${item.tenantId} ${item.requests}`.toLowerCase().includes(keyword)
}

export default function Usage() {
  const { t } = useTranslation()
  const { textColor: chartTextColor, gridColor: chartGridColor, tooltipStyle: chartTooltipStyle } = useChartTheme()

  const [{ startTs, endTs }] = useState(() => {
    const endTs = Math.floor(Date.now() / 1000)
    const startTs = endTs - 30 * 86400
    return { startTs, endTs }
  })
  const [searchValue, setSearchValue] = useState('')
  const [rowsPerPage, setRowsPerPage] = useState(10)
  const [currentPage, setCurrentPage] = useState(1)

  const { data: leaderboard, isLoading: isLoadingLeaderboard, isFetching: isFetchingLeaderboard, refetch: refetchLeaderboard } = useQuery({
    queryKey: ['usageLeaderboard', startTs, endTs],
    queryFn: () => usageApi.getLeaderboard({ start_ts: startTs, end_ts: endTs, limit: 25 }),
    refetchInterval: 60_000,
  })

  const { data: hourlyTrends, isLoading: isLoadingTrends, isFetching: isFetchingTrends, refetch: refetchTrends } = useQuery({
    queryKey: ['usageHourlyTenantTrends', startTs, endTs],
    queryFn: () =>
      usageApi.getHourlyTenantTrends({
        start_ts: startTs,
        end_ts: endTs,
        limit: 30 * 24,
      }),
    refetchInterval: 60_000,
  })

  const summaryData = leaderboard?.summary
  const summary = buildDashboardKpis(summaryData)
  const modelDistribution = useMemo(() => buildModelDistribution(summaryData), [summaryData])
  const tokenTrend = useMemo(() => buildTokenTrend(summaryData), [summaryData])
  const topApiKeys = useMemo(() => buildTopApiKeys(leaderboard), [leaderboard])
  const topApiKeyRequestMax = topApiKeys[0]?.requests ?? 0
  const dailyUsage = useMemo(
    () => groupTenantHourlyUsageByDay(hourlyTrends?.items ?? []),
    [hourlyTrends?.items],
  )
  const topModel = modelDistribution[0]
  const topApiKey = topApiKeys[0]
  const peakDay = dailyUsage.reduce<{ date: string; requests: number } | null>((current, item) => {
    if (!current || item.requests > current.requests) {
      return item
    }
    return current
  }, null)

  const filteredTopApiKeys = useMemo(() => {
    const keyword = searchValue.trim().toLowerCase()

    if (!keyword) {
      return topApiKeys
    }

    return topApiKeys.filter((item) => matchTopKeySearch(item, keyword))
  }, [searchValue, topApiKeys])

  const totalPages = Math.max(1, Math.ceil(filteredTopApiKeys.length / rowsPerPage))
  const resolvedPage = Math.min(currentPage, totalPages)
  const paginatedTopApiKeys = useMemo(() => {
    const start = (resolvedPage - 1) * rowsPerPage
    return filteredTopApiKeys.slice(start, start + rowsPerPage)
  }, [filteredTopApiKeys, resolvedPage, rowsPerPage])
  const visibleRangeStart = filteredTopApiKeys.length === 0 ? 0 : (resolvedPage - 1) * rowsPerPage + 1
  const visibleRangeEnd =
    filteredTopApiKeys.length === 0 ? 0 : Math.min(filteredTopApiKeys.length, resolvedPage * rowsPerPage)

  const isLoading = isLoadingLeaderboard || isLoadingTrends
  const isRefreshing = isFetchingLeaderboard || isFetchingTrends
  const overviewMetrics = [
    {
      title: t('dashboard.kpi.totalRequests'),
      value: formatCompactNumber(summary.totalRequests),
      description: t('usage.antigravity.requestsSummaryHint'),
      icon: <Activity className="h-4 w-4" />,
      toneClassName: 'bg-primary/10 text-primary',
    },
    {
      title: t('dashboard.kpi.totalTokens'),
      value: formatCompactNumber(summary.totalTokens),
      description: t('usage.antigravity.totalTokensHint'),
      icon: <Zap className="h-4 w-4" />,
      toneClassName: 'bg-secondary/10 text-secondary',
    },
    {
      title: t('usage.antigravity.estimatedCost'),
      value: formatCurrency(summary.estimatedCostUsd),
      description: t('usage.antigravity.estimatedCostHint'),
      icon: <DollarSign className="h-4 w-4" />,
      toneClassName: 'bg-success/10 text-success',
    },
    {
      title: t('usage.antigravity.avgLatency'),
      value: formatDurationMs(summary.avgFirstTokenMs),
      description: t('usage.antigravity.avgLatencyHint'),
      icon: <Gauge className="h-4 w-4" />,
      toneClassName: 'bg-warning/10 text-warning',
    },
  ]

  if (isLoading) {
    return (
      <div className="flex h-[calc(100vh-100px)] w-full items-center justify-center">
        <Spinner
          color="primary"
          label={t('usage.antigravity.loading')}
          size="lg"
        />
      </div>
    )
  }

  return (
    <PageContent className="space-y-6">
      <DockedPageIntro
        archetype="workspace"
        title={t('nav.usage')}
        description={t('usage.subtitle')}
        actions={(
          <Button
            color="primary"
            isLoading={isRefreshing}
            startContent={isRefreshing ? undefined : <RefreshCcw className="h-4 w-4" />}
            variant="flat"
            onPress={() => {
              void refetchLeaderboard()
              void refetchTrends()
            }}
          >
            {t('common.refresh')}
          </Button>
        )}
      />

      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        {overviewMetrics.map((metric) => (
          <Card key={metric.title}>
            <CardBody className="space-y-5 p-4">
              <div className={metric.toneClassName + ' flex h-10 w-10 items-center justify-center rounded-large'}>
                {metric.icon}
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

      <div className="grid gap-6 xl:grid-cols-[minmax(0,1.35fr)_minmax(0,0.95fr)]">
        <Card>
          <CardHeader className="px-5 pb-3 pt-5">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t('usage.chart.title')}
              </h2>
              <p className="text-sm leading-6 text-default-600">
                {t('usage.chart.subtitle')}
              </p>
            </div>
          </CardHeader>
          <CardBody className="px-5 pb-5 pt-1">
            {dailyUsage.length ? (
              <ResponsiveContainer height={280} width="100%">
                <BarChart data={dailyUsage}>
                  <CartesianGrid stroke={chartGridColor} strokeDasharray="3 3" />
                  <XAxis axisLine={false} dataKey="date" tick={{ fill: chartTextColor, fontSize: 11 }} tickLine={false} />
                  <YAxis axisLine={false} tick={{ fill: chartTextColor, fontSize: 11 }} tickFormatter={(value) => formatCompactNumber(Number(value))} tickLine={false} />
                  <Tooltip contentStyle={chartTooltipStyle} />
                  <Bar dataKey="requests" fill="hsl(var(--heroui-primary))" radius={[6, 6, 0, 0]} />
                </BarChart>
              </ResponsiveContainer>
            ) : (
              <div className="flex h-[280px] items-center justify-center rounded-large border border-dashed border-default-200 text-sm text-default-600">
                {t('usage.chart.empty')}
              </div>
            )}
          </CardBody>
        </Card>

        <Card>
          <CardHeader className="px-5 pb-3 pt-5">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t('usage.antigravity.signalsTitle')}
              </h2>
              <p className="text-sm leading-6 text-default-600">
                {t('usage.antigravity.signalsDescription')}
              </p>
            </div>
          </CardHeader>
          <CardBody className="grid gap-3 px-5 pb-5 pt-1">
            <div className="rounded-large border border-default-200 bg-content2/55 px-4 py-4">
              <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                {t('usage.antigravity.peakDay')}
              </div>
              <div className="mt-3 text-sm font-semibold text-foreground">
                {peakDay
                  ? t('usage.antigravity.peakDayValue', {
                    date: peakDay.date,
                    requests: formatCompactNumber(peakDay.requests),
                  })
                  : t('usage.chart.empty')}
              </div>
            </div>
            <div className="rounded-large border border-default-200 bg-content2/55 px-4 py-4">
              <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                {t('usage.antigravity.topModel')}
              </div>
              <div className="mt-3 text-sm font-semibold text-foreground">
                {topModel?.model ?? t('common.noData')}
              </div>
              <div className="mt-1 text-xs text-default-500">
                {topModel
                  ? t('usage.antigravity.topModelValue', { value: formatCompactNumber(topModel.requests) })
                  : t('usage.chart.empty')}
              </div>
            </div>
            <div className="rounded-large border border-default-200 bg-content2/55 px-4 py-4">
              <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                {t('usage.antigravity.topApiKey')}
              </div>
              <div className="mt-3 font-mono text-sm font-semibold text-foreground">
                {topApiKey?.apiKeyId ?? t('common.noData')}
              </div>
              <div className="mt-1 text-xs text-default-500">
                {topApiKey
                  ? t('usage.antigravity.topApiKeyValue', { value: formatCompactNumber(topApiKey.requests) })
                  : t('usage.topKeys.empty')}
              </div>
            </div>
            <div className="rounded-large border border-default-200 bg-content2/55 px-4 py-4">
              <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                {t('usage.antigravity.timeWindow')}
              </div>
              <div className="mt-3 text-sm font-semibold text-foreground">
                {t('usage.antigravity.last30Days')}
              </div>
            </div>
          </CardBody>
        </Card>
      </div>

      <div className="grid gap-6 xl:grid-cols-[minmax(0,1.35fr)_minmax(0,0.95fr)]">
        <Card>
          <CardHeader className="px-5 pb-3 pt-5">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t('usage.antigravity.tokenTrendTitle')}
              </h2>
              <p className="text-sm leading-6 text-default-600">
                {t('usage.antigravity.tokenTrendDescription')}
              </p>
            </div>
          </CardHeader>
          <CardBody className="px-5 pb-5 pt-1">
            {tokenTrend.length ? (
              <ResponsiveContainer height={280} width="100%">
                <AreaChart data={tokenTrend}>
                  <CartesianGrid stroke={chartGridColor} strokeDasharray="3 3" />
                  <XAxis axisLine={false} dataKey="hour" tick={{ fill: chartTextColor, fontSize: 11 }} tickLine={false} />
                  <YAxis axisLine={false} tick={{ fill: chartTextColor, fontSize: 11 }} tickFormatter={(value) => formatCompactNumber(Number(value))} tickLine={false} />
                  <Tooltip contentStyle={chartTooltipStyle} />
                  <Area dataKey="input" fill="hsl(var(--heroui-primary) / 0.18)" stroke="hsl(var(--heroui-primary))" strokeWidth={2} type="monotone" />
                  <Area dataKey="cached" fill="hsl(var(--heroui-secondary) / 0.15)" stroke="hsl(var(--heroui-secondary))" strokeWidth={2} type="monotone" />
                  <Area dataKey="output" fill="hsl(var(--heroui-success) / 0.14)" stroke="hsl(var(--heroui-success))" strokeWidth={2} type="monotone" />
                </AreaChart>
              </ResponsiveContainer>
            ) : (
              <div className="flex h-[280px] items-center justify-center rounded-large border border-dashed border-default-200 text-sm text-default-600">
                {t('usage.antigravity.tokenTrendEmpty')}
              </div>
            )}
          </CardBody>
        </Card>

        <Card>
          <CardHeader className="px-5 pb-3 pt-5">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t('usage.antigravity.modelMixTitle')}
              </h2>
              <p className="text-sm leading-6 text-default-600">
                {t('usage.antigravity.modelMixDescription')}
              </p>
            </div>
          </CardHeader>
          <CardBody className="gap-3 px-5 pb-5 pt-1">
            {modelDistribution.length ? modelDistribution.slice(0, 6).map((item) => {
              const share = summary.totalRequests > 0 ? Math.round((item.requests / summary.totalRequests) * 100) : 0
              return (
                <div
                  key={item.model}
                  className="rounded-large border border-default-200 bg-content2/55 px-4 py-4"
                >
                  <div className="flex items-center justify-between gap-3">
                    <div className="min-w-0">
                      <div className="truncate font-medium text-foreground">{item.model}</div>
                      <div className="text-xs text-default-500">
                        {t('usage.antigravity.requestsCount', { value: formatCompactNumber(item.requests) })}
                      </div>
                    </div>
                    <Chip color="primary" size="sm" variant="flat">
                      {t('usage.antigravity.shareValue', { value: share })}
                    </Chip>
                  </div>
                  <div className="mt-3 h-2 overflow-hidden rounded-full bg-default-200">
                    <div
                      className="h-full rounded-full bg-primary"
                      style={{ width: `${Math.max(share, item.requests > 0 ? 8 : 0)}%` }}
                    />
                  </div>
                </div>
              )
            }) : (
              <div className="rounded-large border border-dashed border-default-200 px-4 py-8 text-sm text-default-600">
                {t('usage.antigravity.modelMixEmpty')}
              </div>
            )}
          </CardBody>
        </Card>
      </div>

      <Card>
        <CardHeader className="flex flex-col items-start gap-4 px-5 pb-3 pt-5">
          <div className="space-y-1">
            <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
              {t('usage.topKeys.title')}
            </h2>
            <p className="text-sm leading-6 text-default-600">
              {t('usage.antigravity.topKeysHint')}
            </p>
          </div>

          <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
            <Input
              aria-label={t('usage.topKeys.searchPlaceholder')}
              className="sm:max-w-sm"
              placeholder={t('usage.topKeys.searchPlaceholder')}
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
            aria-label={t('usage.topKeys.title')}
            classNames={{
              base: 'min-h-[24rem]',
              wrapper: 'bg-transparent px-0 py-0 shadow-none',
              th: 'bg-default-100/60 text-xs font-semibold uppercase tracking-[0.12em] text-default-500',
              td: 'align-top py-4 text-sm text-foreground',
              tr: 'data-[hover=true]:bg-content2/35 transition-colors',
              emptyWrapper: 'h-44',
            }}
          >
            <TableHeader>
              <TableColumn>{t('usage.topKeys.columns.apiKey')}</TableColumn>
              <TableColumn>{t('usage.topKeys.columns.tenant')}</TableColumn>
              <TableColumn>{t('usage.topKeys.columns.requests')}</TableColumn>
              <TableColumn>{t('usage.topKeys.columns.share')}</TableColumn>
            </TableHeader>
            <TableBody
              emptyContent={(
                <div className="flex flex-col items-center gap-3 py-10 text-default-500">
                  <TrendingUp className="h-10 w-10 opacity-35" />
                  <div className="text-sm font-medium">{t('usage.topKeys.empty')}</div>
                </div>
              )}
              items={paginatedTopApiKeys}
            >
              {(item) => {
                const share = summary.totalRequests > 0 ? Math.round((item.requests / summary.totalRequests) * 100) : 0
                return (
                  <TableRow key={item.apiKeyId}>
                    <TableCell>
                      <div className="min-w-[220px] space-y-1">
                        <div className="font-mono text-sm font-semibold text-foreground">{item.apiKeyId}</div>
                        <div className="text-xs text-default-500">
                          {t('usage.antigravity.topApiKeyValue', {
                            value: formatCompactNumber(item.requests),
                          })}
                        </div>
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="min-w-[180px] text-sm text-default-600">{item.tenantId}</div>
                    </TableCell>
                    <TableCell>
                      <Chip color="primary" size="sm" variant="flat">
                        {formatCompactNumber(item.requests)}
                      </Chip>
                    </TableCell>
                    <TableCell>
                      <div className="min-w-[180px] space-y-2">
                        <div className="flex items-center justify-between gap-3 text-xs text-default-500">
                          <span>{t('usage.topKeys.columns.share')}</span>
                          <span>{t('usage.antigravity.shareValue', { value: share })}</span>
                        </div>
                        <div className="h-2 overflow-hidden rounded-full bg-default-200">
                          <div
                            className="h-full rounded-full bg-primary"
                            style={{
                              width: `${topApiKeyRequestMax > 0 ? Math.max(Math.round((item.requests / topApiKeyRequestMax) * 100), 8) : 0}%`,
                            }}
                          />
                        </div>
                      </div>
                    </TableCell>
                  </TableRow>
                )
              }}
            </TableBody>
          </Table>

          <div className="flex flex-col gap-3 border-t border-default-200 pt-3 text-xs text-default-500 sm:flex-row sm:items-center sm:justify-between">
            <div className="tabular-nums">
              {t('common.table.range', {
                start: visibleRangeStart,
                end: visibleRangeEnd,
                total: filteredTopApiKeys.length,
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
