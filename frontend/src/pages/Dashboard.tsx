import { Icon } from '@iconify/react'
import {
  Button,
  Chip,
  Dropdown,
  DropdownItem,
  DropdownMenu,
  DropdownTrigger,
  Progress,
  Spinner,
} from '@heroui/react'
import { useQuery } from '@tanstack/react-query'
import {
  AlertTriangle,
  Archive,
  Gauge,
  Inbox,
  RefreshCcw,
  ShieldCheck,
  Timer,
  TriangleAlert,
} from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router-dom'
import {
  Area,
  AreaChart,
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'

import { useSpringNumber } from '@/lib/use-spring-number'
import { accountPoolApi } from '@/api/accounts'
import { dashboardApi } from '@/api/dashboard'
import { adminApi } from '@/api/settings'
import { usageApi } from '@/api/usage'
import {
  DashboardMetricCard,
  DashboardMetricGrid,
  DockedPageIntro,
  PageContent,
} from '@/components/layout/page-archetypes'
import { Card, CardBody, CardHeader } from '@/components/ui/card'
import { CHART_SERIES_COLORS, useChartTheme } from '@/lib/chart-theme'
import {
  buildDashboardKpis,
  buildModelDistribution,
  buildTopApiKeys,
  buildTokenTrend,
  buildTrafficData,
} from '@/features/usage/contracts'
import { formatDurationMs } from '@/lib/duration-format'
import { cn } from '@/lib/utils'

const POOL_PROGRESS_COLORS = {
  brand: 'primary',
  success: 'success',
  warning: 'warning',
  danger: 'danger',
} as const

const POOL_ACCENT_CLASS_NAMES = {
  brand: 'bg-primary',
  success: 'bg-success',
  warning: 'bg-warning',
  danger: 'bg-danger',
} as const

/** HSL fill colors for the donut chart segments */
const POOL_PIE_COLORS = {
  brand: 'hsl(var(--heroui-primary))',
  success: 'hsl(var(--heroui-success))',
  warning: 'hsl(var(--heroui-warning))',
  danger: 'hsl(var(--heroui-danger))',
} as const

function formatNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`
  return n.toString()
}

/** 弹簧动画数字：从旧值平滑过渡到新值，刷新时有触感反馈 */
function SpringKpiValue({ rawValue, format }: { rawValue: number; format: (n: number) => string }) {
  const animated = useSpringNumber(rawValue, { stiffness: 100, damping: 18 })
  return <>{format(animated)}</>
}

export default function Dashboard() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { textColor: chartTextColor, gridColor: chartGridColor, tooltipStyle: chartTooltipStyle } = useChartTheme()
  const prefersReducedMotion =
    typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches
  const [{ startTs, endTs }] = useState(() => {
    const endTs = Math.floor(Date.now() / 1000)
    const startTs = endTs - 86400
    return { startTs, endTs }
  })

  const { data: systemState, isLoading: isLoadingSystem } = useQuery({
    queryKey: ['dashboardSystemState'],
    queryFn: adminApi.getSystemState,
    refetchInterval: 30_000,
  })

  const { data: summaryData, isLoading: isLoadingSummary } = useQuery({
    queryKey: ['dashboardUsageSummary', startTs, endTs],
    queryFn: () => dashboardApi.getUsageSummary({ start_ts: startTs, end_ts: endTs }),
    refetchInterval: 30_000,
  })

  const { data: hourlyTrends, isLoading: isLoadingTrends } = useQuery({
    queryKey: ['dashboardHourlyTrends', startTs, endTs],
    queryFn: () =>
      dashboardApi.getHourlyTrends({
        start_ts: startTs,
        end_ts: endTs,
        limit: 24,
      }),
    refetchInterval: 30_000,
  })

  const { data: leaderboard, isLoading: isLoadingLeaderboard } = useQuery({
    queryKey: ['dashboardLeaderboard', startTs, endTs],
    queryFn: () =>
      usageApi.getLeaderboard({
        start_ts: startTs,
        end_ts: endTs,
        limit: 5,
      }),
    refetchInterval: 60_000,
  })

  const { data: accountPoolSummary } = useQuery({
    queryKey: ['dashboardAccountPoolSummary'],
    queryFn: accountPoolApi.getSummary,
    refetchInterval: 60_000,
  })

  const kpis = buildDashboardKpis(summaryData, systemState?.counts)
  const alerts = useMemo(() => {
    const next: Array<{
      id: string
      severity: 'critical' | 'warning'
      status: 'open' | 'resolved'
      message: string
    }> = []

    if (systemState?.data_plane_error) {
      next.push({
        id: 'data-plane-error',
        severity: 'critical',
        status: 'open',
        message: t('dashboard.alerts.dataPlaneDisconnected', {
          defaultValue: 'Data plane is disconnected',
        }),
      })
    }
    if (systemState && !systemState.usage_repo_available) {
      next.push({
        id: 'usage-repo-unavailable',
        severity: 'warning',
        status: 'open',
        message: t('dashboard.alerts.usageRepoUnavailable', {
          defaultValue: 'Usage analytics storage is unavailable',
        }),
      })
    }

    return next
  }, [systemState, t])

  const trafficData = useMemo(() => buildTrafficData(hourlyTrends), [hourlyTrends])
  const tokenTrend = useMemo(() => buildTokenTrend(summaryData), [summaryData])
  const topApiKeys = useMemo(() => buildTopApiKeys(leaderboard), [leaderboard])
  const topApiKeysMax = topApiKeys[0]?.requests ?? 0
  const modelDistribution = useMemo(() => buildModelDistribution(summaryData), [summaryData])

  // 图表版本号：数据更新时递增，用作 key 强制重新运行入场动画
  const chartAnimKey = useMemo(() => ({
    traffic: `traffic-${hourlyTrends?.account_totals?.length ?? 0}-${hourlyTrends?.account_totals?.[0]?.hour_start ?? 0}`,
    token: `token-${summaryData?.dashboard_metrics?.token_trends?.length ?? 0}`,
    model: `model-${modelDistribution.length}-${modelDistribution[0]?.requests ?? 0}`,
  }), [hourlyTrends, summaryData, modelDistribution])

  const requestSparkline = useMemo(() => trafficData.map((p) => p.accounts), [trafficData])
  const tokenSparkline = useMemo(
    () => tokenTrend.map((p) => p.input + p.cached + p.output + p.reasoning),
    [tokenTrend],
  )

  const primaryMetrics = useMemo(
    () => [
      {
        id: 'total_requests',
        title: t('dashboard.kpi.totalRequests'),
        rawValue: kpis.totalRequests,
        format: formatNumber,
        description: t('dashboard.antigravity.last24Hours', { defaultValue: 'Last 24 hours' }),
        sparklineData: requestSparkline,
        trendType: 'up' as const,
        changeType: 'positive' as const,
      },
      {
        id: 'total_tokens',
        title: t('dashboard.kpi.totalTokens'),
        rawValue: kpis.totalTokens,
        format: formatNumber,
        description: t('dashboard.kpi.totalTokensDesc'),
        sparklineData: tokenSparkline,
        trendType: 'up' as const,
        changeType: 'neutral' as const,
      },
      {
        id: 'rpm',
        title: t('dashboard.kpi.rpm'),
        rawValue: kpis.rpm,
        format: (n: number) => n.toString(),
        description: t('dashboard.kpi.rpmDesc'),
        sparklineData: requestSparkline,
        trendType: 'up' as const,
        changeType: 'positive' as const,
      },
      {
        id: 'avg_ttft',
        title: t('dashboard.kpi.avgFirstTokenSpeed'),
        rawValue: kpis.avgFirstTokenMs,
        format: formatDurationMs,
        description: t('dashboard.kpi.avgFirstTokenSpeedDesc'),
      },
    ],
    [kpis, t, requestSparkline, tokenSparkline],
  )

  const secondaryMetrics = useMemo(
    () => [
      {
        id: 'tenant_count',
        title: t('dashboard.kpi.tenants'),
        rawValue: kpis.tenantCount,
        format: (n: number) => n.toString(),
        description: t('dashboard.kpi.tenantsDesc'),
      },
      {
        id: 'account_count',
        title: t('dashboard.kpi.accounts'),
        rawValue: kpis.accountCount,
        format: (n: number) => n.toString(),
        description: t('dashboard.antigravity.activeAccounts', {
          count: kpis.activeAccounts,
          defaultValue: '{{count}} active',
        }),
      },
      {
        id: 'api_key_count',
        title: t('dashboard.kpi.apiKeys'),
        rawValue: kpis.apiKeyCount,
        format: (n: number) => n.toString(),
        description: t('dashboard.kpi.apiKeysDesc'),
      },
      {
        id: 'tpm',
        title: t('dashboard.kpi.tpm'),
        rawValue: kpis.tpm,
        format: formatNumber,
        description: t('dashboard.kpi.tpmDesc'),
      },
    ],
    [kpis, t],
  )

  const poolOverviewMetrics = useMemo(
    () => [
      {
        title: t('dashboard.poolOverview.inventory'),
        value: accountPoolSummary?.inventory ?? 0,
        description: t('dashboard.poolOverview.inventoryDesc'),
        tone: 'brand' as const,
        icon: <Archive aria-hidden="true" className="h-5 w-5" />,
      },
      {
        title: t('dashboard.poolOverview.routable'),
        value: accountPoolSummary?.routable ?? 0,
        description: t('dashboard.poolOverview.routableDesc'),
        tone: 'success' as const,
        icon: <ShieldCheck aria-hidden="true" className="h-5 w-5" />,
      },
      {
        title: t('dashboard.poolOverview.cooling'),
        value: accountPoolSummary?.cooling ?? 0,
        description: t('dashboard.poolOverview.coolingDesc'),
        tone: 'warning' as const,
        icon: <Gauge aria-hidden="true" className="h-5 w-5" />,
      },
      {
        title: t('dashboard.poolOverview.pendingDelete'),
        value: accountPoolSummary?.pending_delete ?? 0,
        description: t('dashboard.poolOverview.pendingDeleteDesc'),
        tone: 'danger' as const,
        icon: <TriangleAlert aria-hidden="true" className="h-5 w-5" />,
      },
    ],
    [accountPoolSummary, t],
  )

  const totalManagedPool = useMemo(
    () => poolOverviewMetrics.reduce((sum, metric) => sum + metric.value, 0),
    [poolOverviewMetrics],
  )

  const poolOverviewSummaryMetrics = useMemo(
    () =>
      poolOverviewMetrics.map((metric) => ({
        ...metric,
        ratio: totalManagedPool > 0
          ? Math.round((metric.value / totalManagedPool) * 100)
          : 0,
      })),
    [poolOverviewMetrics, totalManagedPool],
  )

  const healthSignalMetrics = useMemo(() => {
    const signals = [
      {
        title: t('dashboard.healthSignals.healthy'),
        value: accountPoolSummary?.healthy ?? 0,
        icon: <ShieldCheck aria-hidden="true" className="h-3.5 w-3.5" />,
        color: 'success' as const,
      },
      {
        title: t('dashboard.healthSignals.quota'),
        value: accountPoolSummary?.quota ?? 0,
        icon: <Timer aria-hidden="true" className="h-3.5 w-3.5" />,
        color: 'warning' as const,
      },
      {
        title: t('dashboard.healthSignals.fatal'),
        value: accountPoolSummary?.fatal ?? 0,
        icon: <TriangleAlert aria-hidden="true" className="h-3.5 w-3.5" />,
        color: 'danger' as const,
      },
      {
        title: t('dashboard.healthSignals.transient'),
        value: accountPoolSummary?.transient ?? 0,
        icon: <RefreshCcw aria-hidden="true" className="h-3.5 w-3.5" />,
        color: 'secondary' as const,
      },
      {
        title: t('dashboard.healthSignals.admin'),
        value: accountPoolSummary?.admin ?? 0,
        icon: <Archive aria-hidden="true" className="h-3.5 w-3.5" />,
        color: 'primary' as const,
      },
    ]
    const total = signals.reduce((s, m) => s + m.value, 0)
    return signals.map((s) => ({ ...s, ratio: total > 0 ? Math.round((s.value / total) * 100) : 0 }))
  }, [accountPoolSummary, t])

  const isLoading = isLoadingSystem || isLoadingSummary || isLoadingTrends || isLoadingLeaderboard

  const handlePoolOverviewAction = (actionKey: string | number) => {
    switch (String(actionKey)) {
      case 'accounts':
        navigate('/accounts')
        break
      case 'logs':
        navigate('/logs')
        break
      case 'imports':
        navigate('/imports')
        break
      default:
        break
    }
  }

  if (isLoading) {
    return (
      <div className="flex h-[calc(100vh-100px)] w-full items-center justify-center">
        <Spinner
          size="lg"
          color="primary"
          label={t('dashboard.antigravity.loading', { defaultValue: 'Loading dashboard data…' })}
        />
      </div>
    )
  }

  return (
    <PageContent className="space-y-10">
      <DockedPageIntro
        archetype="workspace"
        title={t('nav.dashboard')}
        description={t('dashboard.subtitle')}
      />

      {/* ── Pool 数据概览（紧凑分组） ── */}
      <div className="space-y-5">
        {/* Primary KPIs — with sparkline */}
        <DashboardMetricGrid variant="primary" className="xl:grid-cols-4">
          {primaryMetrics.map((metric) => (
            <DashboardMetricCard
              key={metric.id}
              variant="primary"
              title={metric.title}
              value={<SpringKpiValue rawValue={metric.rawValue} format={metric.format} />}
              description={metric.description}
              sparklineData={'sparklineData' in metric ? metric.sparklineData : undefined}
              trendType={'trendType' in metric ? metric.trendType : undefined}
              changeType={'changeType' in metric ? metric.changeType : undefined}
            />
          ))}
        </DashboardMetricGrid>

        {/* Secondary KPIs — compact */}
        <DashboardMetricGrid variant="secondary" className="mt-3 xl:grid-cols-4">
          {secondaryMetrics.map((metric) => (
            <DashboardMetricCard
              key={metric.id}
              variant="secondary"
              title={metric.title}
              value={<SpringKpiValue rawValue={metric.rawValue} format={metric.format} />}
              description={metric.description}
            />
          ))}
        </DashboardMetricGrid>

        {/* ── 账号池总览 — Donut (HeroUI Pro Circles 3 style) ── */}
        <Card>
          <CardHeader className="flex items-start justify-between gap-4 px-5 pb-0 pt-5">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t('dashboard.poolOverview.title')}
              </h2>
              <p className="text-sm leading-6 text-default-600">
                {t('dashboard.poolOverview.description')}
              </p>
            </div>
            <Dropdown placement="bottom-end">
              <DropdownTrigger>
                <Button
                  aria-label={t('dashboard.actions.openMenu', { defaultValue: 'Open actions menu' })}
                  radius="full"
                  size="sm"
                  variant="flat"
                >
                  <Icon icon="solar:menu-dots-bold" className="text-lg text-default-500" />
                </Button>
              </DropdownTrigger>
              <DropdownMenu aria-label={t('dashboard.poolOverview.title')} onAction={handlePoolOverviewAction}>
                <DropdownItem key="accounts">
                  {t('dashboard.actions.viewAccounts', { defaultValue: 'View accounts' })}
                </DropdownItem>
                <DropdownItem key="logs">
                  {t('dashboard.actions.viewLogs', { defaultValue: 'View request logs' })}
                </DropdownItem>
                <DropdownItem key="imports">
                  {t('dashboard.actions.viewImports', { defaultValue: 'View imports' })}
                </DropdownItem>
              </DropdownMenu>
            </Dropdown>
          </CardHeader>

          <CardBody className="flex flex-col items-center gap-6 px-5 pb-5 pt-3 sm:flex-row">
            {/* Donut chart */}
            <div className="relative h-[180px] w-[180px] shrink-0">
              <ResponsiveContainer width="100%" height="100%">
                <PieChart>
                  <Pie
                    data={poolOverviewSummaryMetrics.map((m) => ({ name: m.title, value: Math.max(m.value, 0) }))}
                    cx="50%"
                    cy="50%"
                    innerRadius={58}
                    outerRadius={80}
                    paddingAngle={2}
                    dataKey="value"
                    stroke="none"
                    isAnimationActive={!prefersReducedMotion}
                    animationDuration={800}
                  >
                    {poolOverviewSummaryMetrics.map((metric) => (
                      <Cell key={metric.title} fill={POOL_PIE_COLORS[metric.tone]} />
                    ))}
                  </Pie>
                </PieChart>
              </ResponsiveContainer>
              {/* Center label */}
              <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center">
                <span className="text-[11px] font-semibold uppercase tracking-[0.14em] text-default-500">
                  {t('dashboard.poolOverview.totalLabel', { defaultValue: '总计' })}
                </span>
                <span className="tabular-nums text-2xl font-semibold leading-tight tracking-[-0.04em] text-foreground">
                  {formatNumber(totalManagedPool)}
                </span>
              </div>
            </div>

            {/* Legend list */}
            <div className="grid flex-1 gap-3 sm:grid-cols-2">
              {poolOverviewSummaryMetrics.map((metric) => (
                <div
                  key={metric.title}
                  className="flex items-start gap-3 rounded-large border-small border-default-100 px-3 py-2.5"
                >
                  <span className={cn('mt-1.5 h-2.5 w-2.5 shrink-0 rounded-full', POOL_ACCENT_CLASS_NAMES[metric.tone])} />
                  <div className="min-w-0">
                    <div className="flex items-baseline gap-2">
                      <span className="text-xs font-medium text-default-500">{metric.title}</span>
                      <Chip color={POOL_PROGRESS_COLORS[metric.tone]} size="sm" variant="flat" classNames={{ content: 'text-[0.6rem] font-semibold' }}>
                        {metric.ratio}%
                      </Chip>
                    </div>
                    <div className="tabular-nums text-lg font-semibold leading-tight tracking-[-0.03em] text-foreground">
                      {formatNumber(metric.value)}
                    </div>
                    <p className="text-[11px] leading-5 text-default-400">{metric.description}</p>
                  </div>
                </div>
              ))}
            </div>
          </CardBody>
        </Card>
      </div>

      {/* ── 监控区域 ── */}
      <div className="grid gap-6 xl:grid-cols-[minmax(0,1.3fr)_minmax(0,0.9fr)]">
        <Card>
          <CardHeader className="px-5 pb-2 pt-5">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t('dashboard.healthSignals.title')}
              </h2>
              <p className="text-sm leading-6 text-default-600">
                {t('dashboard.healthSignals.description')}
              </p>
            </div>
          </CardHeader>
          <CardBody className="flex flex-col gap-2 px-5 pb-5 pt-1">
            {healthSignalMetrics.map((metric) => (
              <div key={metric.title} className="flex items-center gap-3">
                <span className={cn('flex h-6 w-6 shrink-0 items-center justify-center rounded-md', {
                  'bg-success/10 text-success': metric.color === 'success',
                  'bg-warning/10 text-warning': metric.color === 'warning',
                  'bg-danger/10 text-danger': metric.color === 'danger',
                  'bg-secondary/10 text-secondary': metric.color === 'secondary',
                  'bg-primary/10 text-primary': metric.color === 'primary',
                })}>
                  {metric.icon}
                </span>
                <span className="w-12 shrink-0 text-xs font-medium text-default-600">{metric.title}</span>
                <span className="w-10 shrink-0 text-right tabular-nums text-sm font-semibold text-foreground">
                  {formatNumber(metric.value)}
                </span>
                <Progress
                  aria-label={metric.title}
                  color={metric.color}
                  size="sm"
                  radius="full"
                  value={metric.ratio}
                  className="flex-1"
                />
                <span className="w-10 shrink-0 text-right tabular-nums text-xs text-default-500">
                  {metric.ratio}%
                </span>
              </div>
            ))}
          </CardBody>
        </Card>

        <Card className={cn(
          'border-small shadow-small',
          alerts.length > 0
            ? 'border-danger/20 bg-danger/[0.04] dark:bg-danger/[0.07]'
            : 'border-default-200 bg-content1',
        )}>
          <CardHeader className="flex items-start justify-between gap-4 px-5 pb-2 pt-5">
            <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
              {t('dashboard.alerts.title')}
            </h2>
            <Chip
              color={alerts.length > 0 ? 'danger' : 'success'}
              size="sm"
              variant="flat"
            >
              {alerts.length}
            </Chip>
          </CardHeader>
          <CardBody className="gap-2.5 px-5 pb-5 pt-1">
            {alerts.length > 0 ? (
              alerts.map((alert) => (
                <div key={alert.id} className="flex items-center gap-3 rounded-large bg-content1/85 px-3 py-2.5">
                  <div className={cn(
                    'flex h-7 w-7 shrink-0 items-center justify-center rounded-md',
                    alert.severity === 'warning' ? 'bg-warning/10 text-warning' : 'bg-danger/10 text-danger',
                  )}>
                    <AlertTriangle aria-hidden="true" className="h-3.5 w-3.5" />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="text-sm leading-5 text-foreground">{alert.message}</div>
                  </div>
                  <div className="flex shrink-0 items-center gap-1.5">
                    <Chip
                      color={alert.severity === 'warning' ? 'warning' : 'danger'}
                      size="sm"
                      variant="flat"
                    >
                      {t(`dashboard.alerts.severity.${alert.severity}`, { defaultValue: alert.severity })}
                    </Chip>
                    <Chip
                      color={alert.status === 'open' ? 'warning' : 'success'}
                      size="sm"
                      variant="flat"
                    >
                      {t(`dashboard.alerts.status.${alert.status}`, { defaultValue: alert.status })}
                    </Chip>
                  </div>
                </div>
              ))
            ) : (
              <div className="flex items-center gap-3 rounded-large bg-success/5 px-3 py-2.5">
                <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md bg-success/10 text-success">
                  <ShieldCheck aria-hidden="true" className="h-3.5 w-3.5" />
                </span>
                <span className="text-sm text-success-700 dark:text-success-400">
                  {t('dashboard.overview.stable')}
                </span>
              </div>
            )}
          </CardBody>
        </Card>
      </div>

      <div className="space-y-6">
      <div className="grid gap-6 lg:grid-cols-[minmax(0,1.25fr)_minmax(0,0.75fr)]">
        {/* ── 全天流量趋势 — Graph 1 style ── */}
        <Card>
          <CardHeader className="flex flex-col gap-4 px-5 pb-2 pt-5">
            <div className="flex items-start justify-between gap-3">
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t('dashboard.trafficChart.title')}
              </h2>
              <Chip size="sm" variant="flat" color="default">24h</Chip>
            </div>
            {/* Inline KPI summary row */}
            <div className="flex flex-wrap gap-3">
              {[
                { label: t('dashboard.kpi.totalRequests'), value: formatNumber(kpis.totalRequests), color: 'success' as const, active: true },
                { label: t('dashboard.antigravity.accountTraffic', { defaultValue: 'Account traffic' }), value: formatNumber(trafficData.reduce((s, p) => s + p.accounts, 0)), color: 'success' as const, active: false },
                { label: t('dashboard.antigravity.apiKeyTraffic', { defaultValue: 'API key traffic' }), value: formatNumber(trafficData.reduce((s, p) => s + p.apiKeys, 0)), color: 'danger' as const, active: false },
              ].map((item) => (
                <div key={item.label} className={cn(
                  'rounded-large border-small px-3 py-2',
                  item.active ? 'border-success/40 bg-success/5' : 'border-default-200 bg-transparent',
                )}>
                  <div className="text-[11px] font-medium text-default-500">{item.label}</div>
                  <div className="text-lg font-semibold tabular-nums leading-tight text-foreground">{item.value}</div>
                </div>
              ))}
            </div>
          </CardHeader>
          <CardBody className="px-5 pb-5 pt-1">
            <ResponsiveContainer width="100%" height={240}>
              <AreaChart key={chartAnimKey.traffic} data={trafficData}>
                <defs>
                  <linearGradient id="successGradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor="hsl(var(--heroui-success))" stopOpacity={0.25} />
                    <stop offset="100%" stopColor="hsl(var(--heroui-success))" stopOpacity={0} />
                  </linearGradient>
                  <linearGradient id="dangerGradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor="hsl(var(--heroui-danger))" stopOpacity={0.15} />
                    <stop offset="100%" stopColor="hsl(var(--heroui-danger))" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke={chartGridColor} />
                <XAxis dataKey="hour" tick={{ fill: chartTextColor, fontSize: 11 }} axisLine={false} tickLine={false} />
                <YAxis tick={{ fill: chartTextColor, fontSize: 11 }} axisLine={false} tickLine={false} />
                <Tooltip contentStyle={chartTooltipStyle} />
                <Area type="monotone" dataKey="accounts" stroke="hsl(var(--heroui-success))" fill="url(#successGradient)" strokeWidth={2} isAnimationActive={!prefersReducedMotion} animationDuration={1000} animationEasing="ease-out" animationBegin={0} />
                <Area type="monotone" dataKey="apiKeys" stroke="hsl(var(--heroui-danger))" fill="url(#dangerGradient)" strokeWidth={1.5} isAnimationActive={!prefersReducedMotion} animationDuration={1000} animationEasing="ease-out" animationBegin={120} />
              </AreaChart>
            </ResponsiveContainer>
          </CardBody>
        </Card>

        {/* ── Token 使用趋势 — Graph 2 style with inline KPI legend ── */}
        <Card>
          <CardHeader className="flex flex-col gap-4 px-5 pb-2 pt-5">
            <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
              {t('dashboard.tokenTrend.title')}
            </h2>
            {/* Token component KPI legend row */}
            <div className="flex flex-wrap gap-3">
              {([
                { key: 'input', label: t('dashboard.tokenComponent.input', { defaultValue: 'Input' }), value: tokenTrend.reduce((s, p) => s + p.input, 0), color: CHART_SERIES_COLORS.input },
                { key: 'cached', label: t('dashboard.tokenComponent.cached', { defaultValue: 'Cached' }), value: tokenTrend.reduce((s, p) => s + p.cached, 0), color: CHART_SERIES_COLORS.cached },
                { key: 'output', label: t('dashboard.tokenComponent.output', { defaultValue: 'Output' }), value: tokenTrend.reduce((s, p) => s + p.output, 0), color: CHART_SERIES_COLORS.output },
                { key: 'reasoning', label: t('dashboard.tokenComponent.reasoning', { defaultValue: 'Reasoning' }), value: tokenTrend.reduce((s, p) => s + p.reasoning, 0), color: CHART_SERIES_COLORS.reasoning },
              ] as const).map((item) => (
                <div key={item.key} className="rounded-large border-small border-default-200 px-3 py-2">
                  <div className="flex items-center gap-1.5">
                    <span className="h-2 w-2 rounded-full" style={{ backgroundColor: item.color }} />
                    <span className="text-[11px] font-medium text-default-500">{item.label}</span>
                  </div>
                  <div className="text-base font-semibold tabular-nums leading-tight text-foreground">{formatNumber(item.value)}</div>
                </div>
              ))}
            </div>
          </CardHeader>
          <CardBody className="px-5 pb-5 pt-1">
            <ResponsiveContainer width="100%" height={240}>
              <AreaChart key={chartAnimKey.token} data={tokenTrend}>
                <defs>
                  <linearGradient id="inputGradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor={CHART_SERIES_COLORS.input} stopOpacity={0.25} />
                    <stop offset="100%" stopColor={CHART_SERIES_COLORS.input} stopOpacity={0} />
                  </linearGradient>
                  <linearGradient id="outputGradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor={CHART_SERIES_COLORS.output} stopOpacity={0.25} />
                    <stop offset="100%" stopColor={CHART_SERIES_COLORS.output} stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke={chartGridColor} />
                <XAxis dataKey="hour" tick={{ fill: chartTextColor, fontSize: 11 }} axisLine={false} tickLine={false} />
                <YAxis tick={{ fill: chartTextColor, fontSize: 11 }} axisLine={false} tickLine={false} tickFormatter={(v) => formatNumber(v)} />
                <Tooltip
                  contentStyle={chartTooltipStyle}
                  formatter={(value: number | string | readonly (number | string)[] | undefined) =>
                    formatNumber(Number(Array.isArray(value) ? value[0] ?? 0 : value ?? 0))}
                />
                <Area type="monotone" dataKey="input" stroke={CHART_SERIES_COLORS.input} fill="url(#inputGradient)" strokeWidth={2} isAnimationActive={!prefersReducedMotion} animationDuration={1100} animationEasing="ease-out" animationBegin={0} />
                <Area type="monotone" dataKey="cached" stroke={CHART_SERIES_COLORS.cached} fill="none" strokeWidth={1.5} strokeDasharray="4 4" isAnimationActive={!prefersReducedMotion} animationDuration={1100} animationEasing="ease-out" animationBegin={100} />
                <Area type="monotone" dataKey="output" stroke={CHART_SERIES_COLORS.output} fill="url(#outputGradient)" strokeWidth={2} isAnimationActive={!prefersReducedMotion} animationDuration={1100} animationEasing="ease-out" animationBegin={200} />
                <Area type="monotone" dataKey="reasoning" stroke={CHART_SERIES_COLORS.reasoning} fill="none" strokeWidth={1.5} isAnimationActive={!prefersReducedMotion} animationDuration={1100} animationEasing="ease-out" animationBegin={300} />
              </AreaChart>
            </ResponsiveContainer>
          </CardBody>
        </Card>
      </div>

      <div className="grid gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader className="px-5 pb-3 pt-5">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t('dashboard.modelDistribution.title')}
              </h2>
              <p className="text-sm leading-6 text-default-600">
                {t('dashboard.modelDistribution.description')}
              </p>
            </div>
          </CardHeader>
          <CardBody className="px-5 pb-5 pt-1">
            {modelDistribution.length === 0 ? (
              <div className="flex h-[280px] items-center justify-center text-sm text-default-600">
                {t('dashboard.modelDistribution.empty')}
              </div>
            ) : (
              <ResponsiveContainer width="100%" height={280}>
                <BarChart key={chartAnimKey.model} data={modelDistribution} layout="vertical">
                  <CartesianGrid strokeDasharray="3 3" stroke={chartGridColor} horizontal={false} />
                  <XAxis type="number" tick={{ fill: chartTextColor, fontSize: 11 }} axisLine={false} tickLine={false} tickFormatter={(v) => formatNumber(v)} />
                  <YAxis type="category" dataKey="model" tick={{ fill: chartTextColor, fontSize: 11 }} axisLine={false} tickLine={false} width={120} />
                  <Tooltip
                    contentStyle={chartTooltipStyle}
                    formatter={(value: number | string | readonly (number | string)[] | undefined) =>
                      formatNumber(Number(Array.isArray(value) ? value[0] ?? 0 : value ?? 0))}
                  />
                  <Bar dataKey="requests" radius={[0, 6, 6, 0]} barSize={20} isAnimationActive={!prefersReducedMotion} animationDuration={900} animationEasing="ease-out" animationBegin={0}>
                    {modelDistribution.map((_, idx) => (
                      <Cell key={idx} fill={CHART_SERIES_COLORS[['input', 'cached', 'output', 'reasoning', 'input'][idx % 5] as keyof typeof CHART_SERIES_COLORS] ?? 'hsl(var(--heroui-primary))'} />
                    ))}
                  </Bar>
                </BarChart>
              </ResponsiveContainer>
            )}
          </CardBody>
        </Card>

        <Card>
          <CardHeader className="px-5 pb-3 pt-5">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t('dashboard.topApiKeys.title')}
              </h2>
              <p className="text-sm leading-6 text-default-600">
                {t('dashboard.topApiKeys.scopeDescription', {
                  scope: t('dashboard.scope.global'),
                })}
              </p>
            </div>
          </CardHeader>
          <CardBody className="flex flex-col px-5 pb-5 pt-1">
            {topApiKeys.length === 0 ? (
              <div className="flex flex-col items-center justify-center gap-2 py-10 text-center">
                <Inbox className="h-8 w-8 text-default-300" />
                <p className="text-sm text-default-500">
                  {t('dashboard.topApiKeys.empty')}
                </p>
                <p className="text-xs text-default-400">
                  {t('dashboard.topApiKeys.emptyHint', { defaultValue: 'API key usage will appear here once requests are made.' })}
                </p>
              </div>
            ) : (
              <div className="divide-y divide-default-100">
                {topApiKeys.map((key, index) => {
                  const progressValue = topApiKeysMax > 0 ? (key.requests / topApiKeysMax) * 100 : 0
                  return (
                    <div key={key.apiKeyId} className="flex items-center gap-3 py-2.5">
                      <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-md bg-default-100 text-[10px] font-bold text-default-500">
                        {index + 1}
                      </span>
                      <div className="min-w-0 flex-1">
                        <div className="truncate text-sm font-medium leading-tight text-foreground">
                          {key.apiKeyId}
                        </div>
                        <div className="text-[11px] text-default-400">{key.tenantId}</div>
                      </div>
                      <Progress
                        aria-label={key.apiKeyId}
                        color="primary"
                        radius="full"
                        size="sm"
                        value={progressValue}
                        className="w-20 shrink-0"
                      />
                      <span className="w-14 shrink-0 text-right tabular-nums text-xs font-semibold text-foreground">
                        {formatNumber(key.requests)}
                      </span>
                    </div>
                  )
                })}
              </div>
            )}
          </CardBody>
        </Card>
      </div>
      </div>
    </PageContent>
  )
}
