import type { UsageDashboardMetrics } from '@/api/types'

export type TokenComponentKey = 'input' | 'cached' | 'output' | 'reasoning'
export type ModelDistributionMode = 'requests' | 'tokens'

export interface TokenComponentSelection {
  input: boolean
  cached: boolean
  output: boolean
  reasoning: boolean
}

export interface TokenTrendChartPoint {
  timestamp: number
  hourStart: number
  requestCount: number
  inputTokens: number
  cachedInputTokens: number
  outputTokens: number
  reasoningTokens: number
  totalTokens: number
}

export interface ModelDistributionPoint {
  model: string
  value: number
  requestCount: number
  totalTokens: number
}

export const DEFAULT_TOKEN_COMPONENT_SELECTION: TokenComponentSelection = {
  input: true,
  cached: true,
  output: true,
  reasoning: true,
}

export function loadTokenComponentSelection(storageKey: string): TokenComponentSelection {
  if (typeof window === 'undefined') {
    return DEFAULT_TOKEN_COMPONENT_SELECTION
  }

  try {
    const raw = window.localStorage.getItem(storageKey)
    if (!raw) {
      return DEFAULT_TOKEN_COMPONENT_SELECTION
    }
    const parsed = JSON.parse(raw) as Partial<TokenComponentSelection>
    return normalizeTokenComponentSelection(parsed)
  } catch {
    return DEFAULT_TOKEN_COMPONENT_SELECTION
  }
}

export function persistTokenComponentSelection(storageKey: string, selection: TokenComponentSelection) {
  if (typeof window === 'undefined') {
    return
  }
  window.localStorage.setItem(storageKey, JSON.stringify(selection))
}

export function toggleTokenComponent(
  selection: TokenComponentSelection,
  key: TokenComponentKey,
): TokenComponentSelection {
  const next = { ...selection, [key]: !selection[key] }
  const enabledCount = Object.values(next).filter(Boolean).length
  return enabledCount > 0 ? next : selection
}

export function computePerMinute(total: number, startTs: number, endTs: number): number {
  const totalMinutes = Math.max(1, Math.floor((endTs - startTs) / 60))
  return total / totalMinutes
}

export function buildTokenTrendChartPoints(metrics?: UsageDashboardMetrics): TokenTrendChartPoint[] {
  const items = metrics?.token_trends ?? []
  return [...items]
    .sort((left, right) => left.hour_start - right.hour_start)
    .map((item) => ({
      timestamp: item.hour_start * 1000,
      hourStart: item.hour_start,
      requestCount: item.request_count,
      inputTokens: item.input_tokens,
      cachedInputTokens: item.cached_input_tokens,
      outputTokens: item.output_tokens,
      reasoningTokens: item.reasoning_tokens,
      totalTokens: item.total_tokens,
    }))
}

export function buildModelDistributionPoints(
  metrics: UsageDashboardMetrics | undefined,
  mode: ModelDistributionMode,
  topN = 10,
): ModelDistributionPoint[] {
  const items = mode === 'tokens'
    ? metrics?.model_token_distribution ?? []
    : metrics?.model_request_distribution ?? []
  if (items.length === 0) {
    return []
  }

  const top = items.slice(0, topN).map((item) => ({
    model: item.model,
    value: mode === 'tokens' ? item.total_tokens : item.request_count,
    requestCount: item.request_count,
    totalTokens: item.total_tokens,
  }))

  if (items.length <= topN) {
    return top
  }

  const rest = items.slice(topN).reduce(
    (acc, item) => ({
      requestCount: acc.requestCount + item.request_count,
      totalTokens: acc.totalTokens + item.total_tokens,
    }),
    { requestCount: 0, totalTokens: 0 },
  )

  return [
    ...top,
    {
      model: 'other',
      value: mode === 'tokens' ? rest.totalTokens : rest.requestCount,
      requestCount: rest.requestCount,
      totalTokens: rest.totalTokens,
    },
  ]
}

export function extractSparklineData(
  metrics: UsageDashboardMetrics | undefined,
  field: 'requestCount' | 'totalTokens' | 'inputTokens' | 'outputTokens',
): number[] {
  const points = buildTokenTrendChartPoints(metrics)
  if (points.length === 0) return []
  return points.map((p) => p[field])
}

function normalizeTokenComponentSelection(
  value: Partial<TokenComponentSelection> | undefined,
): TokenComponentSelection {
  return {
    input: value?.input ?? DEFAULT_TOKEN_COMPONENT_SELECTION.input,
    cached: value?.cached ?? DEFAULT_TOKEN_COMPONENT_SELECTION.cached,
    output: value?.output ?? DEFAULT_TOKEN_COMPONENT_SELECTION.output,
    reasoning: value?.reasoning ?? DEFAULT_TOKEN_COMPONENT_SELECTION.reasoning,
  }
}
