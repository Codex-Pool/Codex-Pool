import type {
  AdminSystemCounts,
  HourlyTenantUsageTotalPoint,
  UsageHourlyTrendsResponse,
  UsageLeaderboardOverviewResponse,
  UsageSummaryQueryResponse,
} from '../../api/types.ts'

export interface DashboardKpis {
  totalRequests: number
  totalTokens: number
  rpm: number
  tpm: number
  avgFirstTokenMs: number
  tenantCount: number
  accountCount: number
  apiKeyCount: number
  activeAccounts: number
  estimatedCostUsd: number
}

export interface DashboardTrafficPoint {
  hour: string
  accounts: number
  apiKeys: number
}

export interface DashboardTokenTrendPoint {
  hour: string
  input: number
  cached: number
  output: number
  reasoning: number
}

export type DashboardTrendGranularity = 'hour' | 'day' | 'week'

interface DashboardTrendBuildOptions {
  granularity?: DashboardTrendGranularity
  rangeSeconds?: number
}

export interface DashboardTopApiKey {
  apiKeyId: string
  tenantId: string
  requests: number
}

export interface DashboardModelDistributionItem {
  model: string
  requests: number
}

function roundToTwo(value: number): number {
  return Math.round(value * 100) / 100
}

function safeArray<T>(value: T[] | null | undefined): T[] {
  return Array.isArray(value) ? value : []
}

function resolveBucketStart(
  hourStart: number,
  granularity: DashboardTrendGranularity,
): number {
  const bucket = new Date(hourStart * 1000)

  if (granularity === 'week') {
    const day = bucket.getDay()
    const diffToMonday = day === 0 ? 6 : day - 1
    bucket.setDate(bucket.getDate() - diffToMonday)
    bucket.setHours(0, 0, 0, 0)
    return Math.floor(bucket.getTime() / 1000)
  }

  if (granularity === 'day') {
    bucket.setHours(0, 0, 0, 0)
    return Math.floor(bucket.getTime() / 1000)
  }

  bucket.setMinutes(0, 0, 0)
  return Math.floor(bucket.getTime() / 1000)
}

function formatBucketLabel(
  bucketStart: number,
  granularity: DashboardTrendGranularity,
  rangeSeconds?: number,
): string {
  const bucket = new Date(bucketStart * 1000)
  const month = String(bucket.getMonth() + 1).padStart(2, '0')
  const day = String(bucket.getDate()).padStart(2, '0')
  const hour = String(bucket.getHours()).padStart(2, '0')

  if (granularity === 'week') {
    return `${month}-${day}`
  }

  if (granularity === 'day') {
    return `${month}-${day}`
  }

  if ((rangeSeconds ?? 0) > 86400) {
    return `${month}-${day} ${hour}:00`
  }

  return `${hour}:00`
}

export function buildDashboardKpis(
  summary: UsageSummaryQueryResponse | undefined,
  counts?: AdminSystemCounts,
): DashboardKpis {
  const durationMinutes = summary
    ? Math.max((summary.end_ts - summary.start_ts) / 60, 1)
    : 1
  const totalRequests =
    summary?.dashboard_metrics?.total_requests ??
    ((summary?.account_total_requests ?? 0) + (summary?.tenant_api_key_total_requests ?? 0))
  const totalTokens = summary?.dashboard_metrics?.token_breakdown.total_tokens ?? 0

  return {
    totalRequests,
    totalTokens,
    rpm: roundToTwo(totalRequests / durationMinutes),
    tpm: roundToTwo(totalTokens / durationMinutes),
    avgFirstTokenMs: summary?.dashboard_metrics?.avg_first_token_latency_ms ?? 0,
    tenantCount: counts?.tenants ?? 0,
    accountCount: counts?.total_accounts ?? 0,
    apiKeyCount: counts?.api_keys ?? 0,
    activeAccounts: counts?.enabled_accounts ?? 0,
    estimatedCostUsd: roundToTwo((summary?.estimated_cost_microusd ?? 0) / 1_000_000),
  }
}

export function groupTenantHourlyUsageByDay(
  items: HourlyTenantUsageTotalPoint[],
): Array<{ date: string; requests: number }> {
  const grouped = new Map<string, number>()

  items.forEach((item) => {
    const date = new Date(item.hour_start * 1000).toISOString().slice(0, 10)
    grouped.set(date, (grouped.get(date) ?? 0) + item.request_count)
  })

  return [...grouped.entries()]
    .map(([date, requests]) => ({ date, requests }))
    .sort((left, right) => left.date.localeCompare(right.date))
}

export function buildTrafficData(
  hourlyTrends: UsageHourlyTrendsResponse | undefined,
  options: DashboardTrendBuildOptions = {},
): DashboardTrafficPoint[] {
  const granularity = options.granularity ?? 'hour'
  const hourlyMap = new Map<number, DashboardTrafficPoint>()

  safeArray(hourlyTrends?.account_totals).forEach((point) => {
    const bucketStart = resolveBucketStart(point.hour_start, granularity)
    hourlyMap.set(bucketStart, {
      hour: formatBucketLabel(bucketStart, granularity, options.rangeSeconds),
      accounts: point.request_count + (hourlyMap.get(bucketStart)?.accounts ?? 0),
      apiKeys: hourlyMap.get(bucketStart)?.apiKeys ?? 0,
    })
  })

  safeArray(hourlyTrends?.tenant_api_key_totals).forEach((point) => {
    const bucketStart = resolveBucketStart(point.hour_start, granularity)
    hourlyMap.set(bucketStart, {
      hour: formatBucketLabel(bucketStart, granularity, options.rangeSeconds),
      accounts: hourlyMap.get(bucketStart)?.accounts ?? 0,
      apiKeys: point.request_count + (hourlyMap.get(bucketStart)?.apiKeys ?? 0),
    })
  })

  return [...hourlyMap.entries()]
    .sort((left, right) => left[0] - right[0])
    .map(([, value]) => value)
}

export function buildTokenTrend(
  summary: UsageSummaryQueryResponse | undefined,
  options: DashboardTrendBuildOptions = {},
): DashboardTokenTrendPoint[] {
  const granularity = options.granularity ?? 'hour'
  const trendMap = new Map<number, DashboardTokenTrendPoint>()

  safeArray(summary?.dashboard_metrics?.token_trends).forEach((point) => {
    const bucketStart = resolveBucketStart(point.hour_start, granularity)
    trendMap.set(bucketStart, {
      hour: formatBucketLabel(bucketStart, granularity, options.rangeSeconds),
      input: point.input_tokens + (trendMap.get(bucketStart)?.input ?? 0),
      cached: point.cached_input_tokens + (trendMap.get(bucketStart)?.cached ?? 0),
      output: point.output_tokens + (trendMap.get(bucketStart)?.output ?? 0),
      reasoning: point.reasoning_tokens + (trendMap.get(bucketStart)?.reasoning ?? 0),
    })
  })

  return [...trendMap.entries()]
    .sort((left, right) => left[0] - right[0])
    .map(([, value]) => value)
}

export function buildTopApiKeys(
  leaderboard: UsageLeaderboardOverviewResponse | undefined,
): DashboardTopApiKey[] {
  return safeArray(leaderboard?.api_keys).map((item) => ({
    apiKeyId: item.api_key_id,
    tenantId: item.tenant_id,
    requests: item.total_requests,
  }))
}

export function buildModelDistribution(
  summary: UsageSummaryQueryResponse | undefined,
): DashboardModelDistributionItem[] {
  return safeArray(summary?.dashboard_metrics?.model_request_distribution).map((item) => ({
    model: item.model,
    requests: item.request_count,
  }))
}
