import { useEffect, useMemo, useRef, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { RefreshCcw } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router-dom'
import {
  Button as NextButton,
  Chip,
  Select,
  SelectItem,
  Skeleton,
  Tab,
  Tabs,
} from '@heroui/react'
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

import { tenantKeysApi } from '@/api/tenantKeys'
import { tenantUsageApi } from '@/api/tenantUsage'
import {
  DashboardMetricCard,
  DashboardMetricGrid,
  DashboardShell,
  DockedPageIntro,
  PageContent,
  PagePanel,
  SectionHeader,
} from '@/components/layout/page-archetypes'
import {
  ChartAccessibility,
  type ChartAccessibilityColumn,
} from '@/components/ui/chart-accessibility'
import { SurfaceCard, SurfaceCardBody, SurfaceInset } from '@/components/ui/surface'
import {
  buildTokenTrendA11yRows,
  getVisibleTokenComponentKeys,
  summarizeModelDistribution,
  summarizeTokenTrendRows,
  type TokenTrendA11yRow,
} from '@/lib/dashboard-chart-a11y'
import {
  MODEL_DISTRIBUTION_BAR_COLOR,
  TOKEN_COMPONENT_CHART_COLORS,
} from '@/lib/dashboard-chart-theme'
import {
  buildModelDistributionPoints,
  buildTokenTrendChartPoints,
  computePerMinute,
  type ModelDistributionPoint,
  type ModelDistributionMode,
  loadTokenComponentSelection,
  persistTokenComponentSelection,
  toggleTokenComponent,
  type TokenComponentKey,
  type TokenComponentSelection,
} from '@/lib/dashboard-metrics'
import {
  formatDashboardCount,
  formatDashboardDurationSeconds,
  formatDashboardExactNumber,
  formatDashboardMetric,
  formatDashboardTokenCount,
  formatDashboardTokenRate,
  formatDashboardTrendTimestampLabel,
} from '@/lib/dashboard-number-format'
import { formatExactCount } from '@/lib/count-number-format'

type RangePreset = 1 | 7 | 30

const TOKEN_COMPONENT_STORAGE_KEY = 'cp:tenant-dashboard-token-components:v1'
const MODEL_MODE_STORAGE_KEY = 'cp:tenant-dashboard-model-mode:v1'

interface TokenBreakdownRow {
  key: TokenComponentKey
  label: string
  value: number
  color: string
}

function loadModelMode(): ModelDistributionMode {
  if (typeof window === 'undefined') {
    return 'requests'
  }
  const raw = window.localStorage.getItem(MODEL_MODE_STORAGE_KEY)
  return raw === 'tokens' ? 'tokens' : 'requests'
}

export function TenantDashboardPage() {
  const { t, i18n } = useTranslation()
  const navigate = useNavigate()
  const [rangePreset, setRangePreset] = useState<RangePreset>(1)
  const [apiKeyId, setApiKeyId] = useState<string>('all')
  const [rangeAnchorMs, setRangeAnchorMs] = useState<number>(() => Date.now())
  const [manualRefreshing, setManualRefreshing] = useState(false)
  const refreshIndicatorTimerRef = useRef<number | null>(null)
  const [tokenComponents, setTokenComponents] = useState<TokenComponentSelection>(() =>
    loadTokenComponentSelection(TOKEN_COMPONENT_STORAGE_KEY),
  )
  const [modelMode, setModelMode] = useState<ModelDistributionMode>(() => loadModelMode())

  useEffect(() => {
    persistTokenComponentSelection(TOKEN_COMPONENT_STORAGE_KEY, tokenComponents)
  }, [tokenComponents])

  useEffect(() => {
    if (typeof window === 'undefined') {
      return
    }
    window.localStorage.setItem(MODEL_MODE_STORAGE_KEY, modelMode)
  }, [modelMode])

  const range = useMemo(() => {
    const endTs = Math.floor(rangeAnchorMs / 1000)
    const startTs = endTs - rangePreset * 24 * 60 * 60
    return { start_ts: startTs, end_ts: endTs }
  }, [rangeAnchorMs, rangePreset])
  const selectedApiKeyId = apiKeyId === 'all' ? undefined : apiKeyId

  const { data: keys = [] } = useQuery({
    queryKey: ['tenantKeys', 'dashboard'],
    queryFn: () => tenantKeysApi.list(),
    staleTime: 60_000,
  })

  const {
    data: summary,
    isFetching: isFetchingSummary,
    refetch: refetchSummary,
  } = useQuery({
    queryKey: ['tenantDashboardSummary', range.start_ts, range.end_ts, selectedApiKeyId],
    queryFn: () => tenantUsageApi.summary({ ...range, api_key_id: selectedApiKeyId }),
    refetchInterval: 30_000,
  })

  const isRefreshing = manualRefreshing || isFetchingSummary

  const handleRefresh = () => {
    setRangeAnchorMs(Date.now())
    if (refreshIndicatorTimerRef.current !== null) {
      window.clearTimeout(refreshIndicatorTimerRef.current)
    }
    setManualRefreshing(true)
    refreshIndicatorTimerRef.current = window.setTimeout(() => {
      setManualRefreshing(false)
      refreshIndicatorTimerRef.current = null
    }, 500)
    refetchSummary()
  }

  useEffect(() => {
    return () => {
      if (refreshIndicatorTimerRef.current !== null) {
        window.clearTimeout(refreshIndicatorTimerRef.current)
      }
    }
  }, [])

  const logsSearch = useMemo(() => {
    const params = new URLSearchParams()
    params.set('range', String(rangePreset))
    if (selectedApiKeyId) {
      params.set('api_key_id', selectedApiKeyId)
    }
    return params.toString()
  }, [rangePreset, selectedApiKeyId])

  const billingSearch = useMemo(() => {
    const params = new URLSearchParams()
    params.set('granularity', rangePreset === 30 ? 'month' : 'day')
    return params.toString()
  }, [rangePreset])

  const rangeOptions: Array<{ key: string; label: string }> = [
    { key: '1', label: t('tenantDashboard.filters.range.last24Hours', { defaultValue: 'Last 24 hours' }) },
    { key: '7', label: t('tenantDashboard.filters.range.last7Days', { defaultValue: 'Last 7 days' }) },
    { key: '30', label: t('tenantDashboard.filters.range.last30Days', { defaultValue: 'Last 30 days' }) },
  ]

  const apiKeyOptions = useMemo(() => {
    const options = [
      {
        key: 'all',
        label: t('tenantDashboard.filters.apiKeyAll', { defaultValue: 'All API keys' }),
      },
    ]
    for (const item of keys) {
      options.push({
        key: item.id,
        label: `${item.name} (${item.key_prefix})`,
      })
    }
    return options
  }, [keys, t])

  const dashboardMetrics = summary?.dashboard_metrics
  const tokenBreakdown = dashboardMetrics?.token_breakdown ?? {
    input_tokens: 0,
    cached_input_tokens: 0,
    output_tokens: 0,
    reasoning_tokens: 0,
    total_tokens: 0,
  }
  const inputTokens = tokenBreakdown.input_tokens
  const cachedInputTokens = tokenBreakdown.cached_input_tokens
  const outputTokens = tokenBreakdown.output_tokens
  const reasoningTokens = tokenBreakdown.reasoning_tokens
  const totalRequests = dashboardMetrics?.total_requests ?? summary?.tenant_api_key_total_requests ?? 0
  const totalTokens = tokenBreakdown.total_tokens
  const rpm = computePerMinute(totalRequests, range.start_ts, range.end_ts)
  const tpm = computePerMinute(totalTokens, range.start_ts, range.end_ts)
  const avgFirstTokenMs = dashboardMetrics?.avg_first_token_latency_ms
  const avgFirstTokenSec = typeof avgFirstTokenMs === 'number' ? avgFirstTokenMs / 1000 : null

  const tokenTrendData = useMemo(
    () => buildTokenTrendChartPoints(dashboardMetrics),
    [dashboardMetrics],
  )
  const modelDistributionData = useMemo(
    () => buildModelDistributionPoints(dashboardMetrics, modelMode),
    [dashboardMetrics, modelMode],
  )

  const showTokenArea = tokenComponents.input
    || tokenComponents.cached
    || tokenComponents.output
    || tokenComponents.reasoning

  const tokenLabelByKey = useMemo<Record<TokenComponentKey, string>>(
    () => ({
      input: t('tenantDashboard.tokenComponents.input', { defaultValue: 'Input' }),
      cached: t('tenantDashboard.tokenComponents.cached', { defaultValue: 'Cached' }),
      output: t('tenantDashboard.tokenComponents.output', { defaultValue: 'Output' }),
      reasoning: t('tenantDashboard.tokenComponents.reasoning', { defaultValue: 'Reasoning' }),
    }),
    [t],
  )
  const tokenBreakdownRows: TokenBreakdownRow[] = [
    {
      key: 'input',
      label: tokenLabelByKey.input,
      value: inputTokens,
      color: TOKEN_COMPONENT_CHART_COLORS.input,
    },
    {
      key: 'cached',
      label: tokenLabelByKey.cached,
      value: cachedInputTokens,
      color: TOKEN_COMPONENT_CHART_COLORS.cached,
    },
    {
      key: 'output',
      label: tokenLabelByKey.output,
      value: outputTokens,
      color: TOKEN_COMPONENT_CHART_COLORS.output,
    },
    {
      key: 'reasoning',
      label: tokenLabelByKey.reasoning,
      value: reasoningTokens,
      color: TOKEN_COMPONENT_CHART_COLORS.reasoning,
    },
  ]
  const visibleTokenComponentKeys = useMemo(
    () => getVisibleTokenComponentKeys(tokenComponents),
    [tokenComponents],
  )
  const tokenTrendA11yRows = useMemo(
    () => buildTokenTrendA11yRows(tokenTrendData, tokenComponents),
    [tokenComponents, tokenTrendData],
  )
  const tokenTrendA11yColumns = useMemo<ChartAccessibilityColumn<TokenTrendA11yRow>[]>(
    () => [
      {
        key: 'timestamp',
        header: t('tenantDashboard.tokenTrend.a11y.timestamp', {
          defaultValue: 'Timestamp',
        }),
        render: (row: TokenTrendA11yRow) =>
          formatDashboardTrendTimestampLabel(row.timestamp, {
            locale: i18n.resolvedLanguage,
          }),
      },
      ...visibleTokenComponentKeys.map(
        (key): ChartAccessibilityColumn<TokenTrendA11yRow> => ({
          key,
          header: tokenLabelByKey[key],
          render: (row: TokenTrendA11yRow) =>
            formatDashboardTokenCount(
              row.values.find(
                (value: TokenTrendA11yRow['values'][number]) => value.key === key,
              )?.value ?? 0,
              i18n.resolvedLanguage,
            ),
        }),
      ),
    ],
    [i18n.resolvedLanguage, t, tokenLabelByKey, visibleTokenComponentKeys],
  )
  const tokenTrendA11ySummary = useMemo(() => {
    const summary = summarizeTokenTrendRows(tokenTrendA11yRows)

    if (summary.rowCount === 0 || summary.startTimestamp === null || summary.endTimestamp === null) {
      return t('tenantDashboard.tokenTrend.a11y.summaryEmpty', {
        defaultValue: 'No token trend data is available for the current selection.',
      })
    }

    return t('tenantDashboard.tokenTrend.a11y.summary', {
      defaultValue: 'Hourly token trend covering {{count}} time points from {{start}} to {{end}}. Accessible data table follows.',
      count: summary.rowCount,
      start: formatDashboardTrendTimestampLabel(summary.startTimestamp, {
        locale: i18n.resolvedLanguage,
      }),
      end: formatDashboardTrendTimestampLabel(summary.endTimestamp, {
        locale: i18n.resolvedLanguage,
      }),
    })
  }, [i18n.resolvedLanguage, t, tokenTrendA11yRows])
  const modelDistributionA11ySummary = useMemo(() => {
    const summary = summarizeModelDistribution(modelDistributionData)

    if (summary.rowCount === 0 || !summary.topLabel) {
      return t('tenantDashboard.modelDistribution.a11y.summaryEmpty', {
        defaultValue: 'No model distribution data is available for the current selection.',
      })
    }

    return t('tenantDashboard.modelDistribution.a11y.summary', {
      defaultValue: 'Model distribution includes {{count}} rows sorted by {{mode}}. Leading model: {{top}}. Accessible data table follows.',
      count: summary.rowCount,
      mode:
        modelMode === 'tokens'
          ? t('tenantDashboard.modelDistribution.modeTokens', { defaultValue: 'By tokens' })
          : t('tenantDashboard.modelDistribution.modeRequests', { defaultValue: 'By requests' }),
      top: summary.topLabel,
    })
  }, [modelDistributionData, modelMode, t])
  const modelDistributionA11yColumns = useMemo<ChartAccessibilityColumn<ModelDistributionPoint>[]>(
    () => [
      {
        key: 'model',
        header: t('tenantDashboard.modelDistribution.a11y.model', {
          defaultValue: 'Model',
        }),
        render: (row) =>
          row.model === 'other'
            ? t('tenantDashboard.modelDistribution.other', { defaultValue: 'Other' })
            : row.model,
      },
      {
        key: 'value',
        header:
          modelMode === 'tokens'
            ? t('tenantDashboard.modelDistribution.modeTokens', {
                defaultValue: 'By tokens',
              })
            : t('tenantDashboard.modelDistribution.modeRequests', {
                defaultValue: 'By requests',
              }),
        render: (row) =>
          modelMode === 'tokens'
            ? formatDashboardTokenCount(row.value, i18n.resolvedLanguage)
            : formatDashboardCount(row.value, i18n.resolvedLanguage),
      },
    ],
    [i18n.resolvedLanguage, modelMode, t],
  )

  const kpiCards = [
    {
      title: t('tenantDashboard.kpi.rpm', { defaultValue: 'RPM' }),
      value: formatDashboardMetric(rpm),
      exactValue: formatDashboardExactNumber(rpm),
      desc: t('tenantDashboard.kpi.rpmDesc', { defaultValue: 'Requests per minute' }),
    },
    {
      title: t('tenantDashboard.kpi.tpm', { defaultValue: 'TPM' }),
      value: formatDashboardTokenRate(tpm),
      exactValue: formatDashboardExactNumber(tpm),
      desc: t('tenantDashboard.kpi.tpmDesc', { defaultValue: 'Tokens per minute' }),
    },
    {
      title: t('tenantDashboard.kpi.totalTokens', { defaultValue: 'Token consumption total' }),
      value: formatDashboardTokenCount(totalTokens),
      exactValue: formatExactCount(totalTokens),
      desc: t('tenantDashboard.kpi.totalTokensDesc', { defaultValue: 'Input + cached + output + reasoning' }),
    },
    {
      title: t('tenantDashboard.kpi.totalRequests', { defaultValue: 'Total requests' }),
      value: formatDashboardCount(totalRequests),
      exactValue: formatExactCount(totalRequests),
      desc: t('tenantDashboard.kpi.totalRequestsDesc', { defaultValue: 'Selected time range' }),
    },
    {
      title: t('tenantDashboard.kpi.avgFirstTokenSpeed', { defaultValue: 'Average first-token speed' }),
      value: formatDashboardDurationSeconds(avgFirstTokenSec),
      exactValue: formatDashboardDurationSeconds(avgFirstTokenSec),
      desc: t('tenantDashboard.kpi.avgFirstTokenSpeedDesc', { defaultValue: 'TTFT (streaming exact / non-stream approximate)' }),
    },
  ]

  const groupOverviewItems = useMemo(() => {
    if (selectedApiKeyId) {
      const key = keys.find((item) => item.id === selectedApiKeyId)
      return key
        ? [
            {
              id: key.id,
              label: `${key.name} (${key.key_prefix})`,
              groupName: key.group.name,
              invalid: key.group.deleted,
            },
          ]
        : []
    }
    const buckets = new Map<string, { label: string; count: number; invalid: boolean }>()
    for (const key of keys) {
      const current = buckets.get(key.group_id) ?? {
        label: key.group.name,
        count: 0,
        invalid: key.group.deleted,
      }
      current.count += 1
      current.invalid = current.invalid || key.group.deleted
      buckets.set(key.group_id, current)
    }
    return Array.from(buckets.entries()).map(([id, item]) => ({
      id,
      label: item.label,
      groupName: t('tenantDashboard.groupOverview.keysBound', {
        defaultValue: '{{count}} API keys bound',
        count: item.count,
      }),
      invalid: item.invalid,
    }))
  }, [keys, selectedApiKeyId, t])

  return (
    <PageContent className="overflow-y-auto bg-muted/20">
      <DashboardShell
        intro={(
          <DockedPageIntro
            archetype="dashboard"
            eyebrow={t('tenantDashboard.hero.badge', { defaultValue: 'Tenant Workspace Overview' })}
            title={t('tenantDashboard.title', { defaultValue: 'Tenant Dashboard' })}
            description={t('tenantDashboard.subtitle.metricsFocus', {
              defaultValue: 'Focus metrics: TPM, RPM, total token consumption, total requests, and first-token speed.',
            })}
            actions={(
              <>
                <NextButton
                  variant="flat"
                  color="primary"
                  className="min-h-11"
                  onPress={() => navigate({ pathname: '/logs', search: `?${logsSearch}` })}
                >
                  {t('tenantDashboard.actions.viewRequestLogs', { defaultValue: 'View request logs' })}
                </NextButton>
                <NextButton
                  variant="flat"
                  color="default"
                  className="min-h-11"
                  onPress={() => navigate({ pathname: '/billing', search: `?${billingSearch}` })}
                >
                  {t('tenantDashboard.actions.viewBilling', { defaultValue: 'View billing' })}
                </NextButton>
                <NextButton
                  variant="flat"
                  color="secondary"
                  className="min-h-11"
                  onPress={() => navigate('/api-keys')}
                >
                  {t('tenantDashboard.actions.manageApiKeys', { defaultValue: 'Manage API keys' })}
                </NextButton>
                <NextButton
                  className="min-h-11"
                  onPress={handleRefresh}
                  isDisabled={isRefreshing}
                  startContent={<RefreshCcw className={`h-4 w-4 ${isRefreshing ? 'animate-spin' : ''}`} />}
                >
                  {t('tenantDashboard.actions.refresh', { defaultValue: 'Refresh' })}
                </NextButton>
              </>
            )}
          />
        )}
        rail={(
          <PagePanel tone="secondary" className="space-y-4">
            <SectionHeader
              eyebrow={t('dashboard.filters.eyebrow', { defaultValue: 'Context' })}
              title={t('dashboard.filters.title', { defaultValue: 'Scope and filters' })}
            />
            <div className="space-y-3">
              <Select
                aria-label={t('tenantDashboard.filters.rangeAriaLabel', { defaultValue: 'Time range' })}
                disallowEmptySelection
                selectedKeys={[String(rangePreset)]}
                onChange={(event) => setRangePreset(Number(event.target.value) as RangePreset)}
                size="sm"
              >
                {rangeOptions.map((option) => (
                  <SelectItem key={option.key}>{option.label}</SelectItem>
                ))}
              </Select>
              <Select
                aria-label={t('tenantDashboard.filters.apiKeyAriaLabel', { defaultValue: 'API key' })}
                disallowEmptySelection
                selectedKeys={[apiKeyId]}
                onChange={(event) => setApiKeyId(event.target.value)}
                size="sm"
                items={apiKeyOptions}
              >
                {(item) => <SelectItem key={item.key}>{item.label}</SelectItem>}
              </Select>
            </div>
          </PagePanel>
        )}
      >
        <DashboardMetricGrid gridClassName="xl:grid-cols-3">
          {kpiCards.map((item) => (
            <DashboardMetricCard
              key={item.title}
              title={item.title}
              value={item.value}
              valueTitle={item.exactValue}
              description={item.desc}
              loading={isFetchingSummary && !summary}
            />
          ))}
        </DashboardMetricGrid>

        <PagePanel tone="secondary">
          <SectionHeader title={t('tenantDashboard.groupOverview.title', { defaultValue: 'API key group overview' })} />
          <div className="mt-5 grid gap-3 md:grid-cols-2 xl:grid-cols-3">
            {groupOverviewItems.length === 0 ? (
              <SurfaceInset tone="muted" className="px-4 py-5 text-sm text-muted-foreground">
                {t('tenantDashboard.groupOverview.empty', { defaultValue: 'No API key groups to show yet.' })}
              </SurfaceInset>
            ) : (
              groupOverviewItems.map((item) => (
                <SurfaceCard key={item.id} tone="default" shadow="none">
                  <SurfaceCardBody className="px-4 py-3">
                    <div className="flex items-center justify-between gap-3">
                      <p className="font-medium text-foreground">{item.label}</p>
                      <Chip color={item.invalid ? 'danger' : 'success'} variant="flat">
                        {item.invalid
                          ? t('tenantDashboard.groupOverview.invalid', { defaultValue: 'Invalid' })
                          : t('tenantDashboard.groupOverview.valid', { defaultValue: 'Valid' })}
                      </Chip>
                    </div>
                    <p className="mt-2 text-sm text-muted-foreground">{item.groupName}</p>
                  </SurfaceCardBody>
                </SurfaceCard>
              ))
            )}
          </div>
        </PagePanel>

        <div className="grid gap-6 2xl:grid-cols-[minmax(0,1.55fr)_minmax(0,1fr)]">
          <PagePanel className="space-y-5">
            <SectionHeader
              title={t('tenantDashboard.tokenTrend.title', { defaultValue: 'Token usage trend' })}
              actions={(
                <div className="flex flex-wrap gap-2">
                  {tokenBreakdownRows.map((item) => (
                    <NextButton
                      key={item.key}
                      aria-pressed={tokenComponents[item.key]}
                      color={tokenComponents[item.key] ? 'primary' : 'default'}
                      radius="full"
                      size="sm"
                      startContent={(
                        <span
                          aria-hidden="true"
                          className="size-2 shrink-0 rounded-full border border-background/70"
                          style={{ backgroundColor: item.color }}
                        />
                      )}
                      variant={tokenComponents[item.key] ? 'flat' : 'bordered'}
                      title={formatExactCount(item.value)}
                      onPress={() => setTokenComponents((prev) => toggleTokenComponent(prev, item.key))}
                    >
                      <span>{item.label}: {formatDashboardTokenCount(item.value)}</span>
                    </NextButton>
                  ))}
                </div>
              )}
            />
            <ChartAccessibility
              summaryId="tenant-dashboard-token-trend-a11y"
              summary={tokenTrendA11ySummary}
              tableLabel={t('tenantDashboard.tokenTrend.a11y.tableLabel', {
                defaultValue: 'Token usage trend data table',
              })}
              columns={tokenTrendA11yColumns}
              rows={tokenTrendA11yRows}
            />
            {isFetchingSummary && tokenTrendData.length === 0 ? (
              <div className="space-y-3">
                <Skeleton className="h-8 w-48 rounded-xl" />
                <Skeleton className="h-[300px] w-full rounded-xl" />
              </div>
            ) : tokenTrendData.length === 0 ? (
              <SurfaceInset tone="muted" className="flex h-[320px] items-center justify-center text-sm text-muted-foreground">
                {t('tenantDashboard.tokenTrend.empty', { defaultValue: 'No token trend data yet' })}
              </SurfaceInset>
            ) : (
              <div aria-hidden="true" style={{ width: '100%', minHeight: 320 }}>
                <ResponsiveContainer width="100%" height={320}>
                  <AreaChart data={tokenTrendData} margin={{ top: 8, right: 12, left: 6, bottom: 8 }}>
                    <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="var(--border)" />
                    <XAxis
                      dataKey="timestamp"
                      tickFormatter={(value) =>
                        new Intl.DateTimeFormat(undefined, {
                          month: '2-digit',
                          day: '2-digit',
                          hour: '2-digit',
                          minute: '2-digit',
                          hour12: false,
                        }).format(new Date(value))
                      }
                      tick={{ fill: 'var(--muted-foreground)', fontSize: 12 }}
                      tickLine={false}
                      axisLine={false}
                    />
                    <YAxis
                      tick={{ fill: 'var(--muted-foreground)', fontSize: 12 }}
                      tickFormatter={(value) =>
                        typeof value === 'number'
                          ? formatDashboardTokenCount(value)
                          : String(value ?? '')
                      }
                      tickLine={false}
                      axisLine={false}
                    />
                    <Tooltip
                      formatter={(value) =>
                        typeof value === 'number'
                          ? formatDashboardTokenCount(value)
                          : String(value ?? '')
                      }
                      labelFormatter={(label) => formatDashboardTrendTimestampLabel(label)}
                    />
                    {showTokenArea && tokenComponents.input ? (
                      <Area type="monotone" dataKey="inputTokens" stackId="tokens" stroke={TOKEN_COMPONENT_CHART_COLORS.input} fill={TOKEN_COMPONENT_CHART_COLORS.input} fillOpacity={0.6} name={t('tenantDashboard.tokenComponents.input', { defaultValue: 'Input' })} />
                    ) : null}
                    {showTokenArea && tokenComponents.cached ? (
                      <Area type="monotone" dataKey="cachedInputTokens" stackId="tokens" stroke={TOKEN_COMPONENT_CHART_COLORS.cached} fill={TOKEN_COMPONENT_CHART_COLORS.cached} fillOpacity={0.6} name={t('tenantDashboard.tokenComponents.cached', { defaultValue: 'Cached' })} />
                    ) : null}
                    {showTokenArea && tokenComponents.output ? (
                      <Area type="monotone" dataKey="outputTokens" stackId="tokens" stroke={TOKEN_COMPONENT_CHART_COLORS.output} fill={TOKEN_COMPONENT_CHART_COLORS.output} fillOpacity={0.6} name={t('tenantDashboard.tokenComponents.output', { defaultValue: 'Output' })} />
                    ) : null}
                    {showTokenArea && tokenComponents.reasoning ? (
                      <Area type="monotone" dataKey="reasoningTokens" stackId="tokens" stroke={TOKEN_COMPONENT_CHART_COLORS.reasoning} fill={TOKEN_COMPONENT_CHART_COLORS.reasoning} fillOpacity={0.6} name={t('tenantDashboard.tokenComponents.reasoning', { defaultValue: 'Reasoning' })} />
                    ) : null}
                  </AreaChart>
                </ResponsiveContainer>
              </div>
            )}
          </PagePanel>

          <PagePanel className="space-y-5">
            <SectionHeader
              title={t('tenantDashboard.modelDistribution.title', { defaultValue: 'Model request distribution' })}
              actions={(
                <Tabs
                  aria-label={t('tenantDashboard.modelDistribution.title', { defaultValue: 'Model request distribution' })}
                  className="w-auto"
                  classNames={{
                    tabList: 'rounded-full bg-content2 p-1',
                    tab: 'h-9 px-3',
                    cursor: 'rounded-full bg-primary shadow-none',
                    panel: 'hidden',
                    tabContent:
                      'text-xs font-medium text-default-500 group-data-[selected=true]:text-primary-foreground',
                  }}
                  color="primary"
                  selectedKey={modelMode}
                  variant="solid"
                  onSelectionChange={(key) => setModelMode(String(key) as ModelDistributionMode)}
                >
                  <Tab
                    key="requests"
                    title={t('tenantDashboard.modelDistribution.modeRequests', { defaultValue: 'By requests' })}
                  />
                  <Tab
                    key="tokens"
                    title={t('tenantDashboard.modelDistribution.modeTokens', { defaultValue: 'By tokens' })}
                  />
                </Tabs>
              )}
            />
            <ChartAccessibility
              summaryId="tenant-dashboard-model-distribution-a11y"
              summary={modelDistributionA11ySummary}
              tableLabel={t('tenantDashboard.modelDistribution.a11y.tableLabel', {
                defaultValue: 'Model distribution data table',
              })}
              columns={modelDistributionA11yColumns}
              rows={modelDistributionData}
            />
            {isFetchingSummary && modelDistributionData.length === 0 ? (
              <div className="space-y-3">
                <Skeleton className="h-8 w-40 rounded-xl" />
                <Skeleton className="h-[300px] w-full rounded-xl" />
              </div>
            ) : modelDistributionData.length === 0 ? (
              <SurfaceInset tone="muted" className="flex h-[320px] items-center justify-center text-sm text-muted-foreground">
                {t('tenantDashboard.modelDistribution.empty', { defaultValue: 'No model distribution data yet' })}
              </SurfaceInset>
            ) : (
              <div aria-hidden="true" style={{ width: '100%', minHeight: 320 }}>
                <ResponsiveContainer width="100%" height={320}>
                  <BarChart
                    data={modelDistributionData}
                    layout="vertical"
                    margin={{ top: 8, right: 16, left: 8, bottom: 8 }}
                  >
                    <CartesianGrid strokeDasharray="3 3" horizontal={false} stroke="var(--border)" />
                    <XAxis
                      type="number"
                      tick={{ fill: 'var(--muted-foreground)', fontSize: 12 }}
                      tickFormatter={(value) =>
                        typeof value === 'number'
                          ? (modelMode === 'tokens'
                              ? formatDashboardTokenCount(value)
                              : formatDashboardCount(value))
                          : String(value ?? '')
                      }
                      tickLine={false}
                      axisLine={false}
                    />
                    <YAxis
                      dataKey="model"
                      type="category"
                      width={120}
                      tick={{ fill: 'var(--muted-foreground)', fontSize: 12 }}
                      tickLine={false}
                      axisLine={false}
                      tickFormatter={(value) =>
                        value === 'other'
                          ? t('tenantDashboard.modelDistribution.other', { defaultValue: 'Other' })
                          : String(value)
                      }
                    />
                    <Tooltip
                      formatter={(value) =>
                        typeof value === 'number'
                          ? (modelMode === 'tokens'
                              ? formatDashboardTokenCount(value)
                              : formatDashboardCount(value))
                          : String(value ?? '')
                      }
                      labelFormatter={(label) =>
                        label === 'other'
                          ? t('tenantDashboard.modelDistribution.other', { defaultValue: 'Other' })
                          : String(label)
                      }
                    />
                    <Bar dataKey="value" fill={MODEL_DISTRIBUTION_BAR_COLOR} radius={[0, 8, 8, 0]} />
                  </BarChart>
                </ResponsiveContainer>
              </div>
            )}
          </PagePanel>
        </div>

        <PagePanel tone="secondary">
          <SectionHeader
            title={t('tenantDashboard.tokenSummary.title', { defaultValue: 'Token component summary' })}
          />
          <div className="mt-5 grid gap-2 sm:grid-cols-2 xl:grid-cols-4">
            {tokenBreakdownRows.map((row) => (
              <SurfaceCard key={row.key} tone="muted" shadow="none">
                <SurfaceCardBody className="p-3">
                  <p className="text-xs text-muted-foreground">{row.label}</p>
                  <p className="text-lg font-semibold text-foreground">{formatDashboardTokenCount(row.value)}</p>
                </SurfaceCardBody>
              </SurfaceCard>
            ))}
          </div>
        </PagePanel>
      </DashboardShell>
    </PageContent>
  )
}
