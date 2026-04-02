/// <reference types="node" />

import assert from 'node:assert/strict'
import test from 'node:test'

import {
  buildTokenTrend,
  buildTrafficData,
} from './contracts.ts'
import type {
  UsageHourlyTrendsResponse,
  UsageSummaryQueryResponse,
} from '../../api/types.ts'

test('buildTrafficData 支持按天聚合小时流量点', () => {
  const hourlyTrends: UsageHourlyTrendsResponse = {
    start_ts: 0,
    end_ts: 0,
    account_totals: [
      { hour_start: 1_710_000_000, request_count: 10 },
      { hour_start: 1_710_003_600, request_count: 5 },
      { hour_start: 1_710_086_400, request_count: 7 },
    ],
    tenant_api_key_totals: [
      { hour_start: 1_710_000_000, request_count: 4 },
      { hour_start: 1_710_003_600, request_count: 6 },
      { hour_start: 1_710_086_400, request_count: 3 },
    ],
  }

  const aggregated = buildTrafficData(hourlyTrends, { granularity: 'day' })

  assert.equal(aggregated.length, 2)
  assert.deepEqual(aggregated.map((point) => point.accounts), [15, 7])
  assert.deepEqual(aggregated.map((point) => point.apiKeys), [10, 3])
})

test('buildTokenTrend 支持按周聚合 token 趋势点', () => {
  const summary: UsageSummaryQueryResponse = {
    start_ts: 0,
    end_ts: 0,
    account_total_requests: 0,
    tenant_api_key_total_requests: 0,
    unique_account_count: 0,
    unique_tenant_api_key_count: 0,
    dashboard_metrics: {
      total_requests: 0,
      token_breakdown: {
        input_tokens: 0,
        cached_input_tokens: 0,
        output_tokens: 0,
        reasoning_tokens: 0,
        total_tokens: 0,
      },
      avg_first_token_latency_ms: 0,
      model_request_distribution: [],
      model_token_distribution: [],
      token_trends: [
        {
          hour_start: 1_709_870_400,
          request_count: 1,
          input_tokens: 10,
          cached_input_tokens: 2,
          output_tokens: 5,
          reasoning_tokens: 1,
          total_tokens: 18,
        },
        {
          hour_start: 1_710_043_200,
          request_count: 1,
          input_tokens: 20,
          cached_input_tokens: 3,
          output_tokens: 7,
          reasoning_tokens: 2,
          total_tokens: 32,
        },
        {
          hour_start: 1_710_475_200,
          request_count: 1,
          input_tokens: 30,
          cached_input_tokens: 4,
          output_tokens: 9,
          reasoning_tokens: 3,
          total_tokens: 46,
        },
      ],
    },
  }

  const aggregated = buildTokenTrend(summary, { granularity: 'week' })

  assert.equal(aggregated.length, 2)
  assert.deepEqual(
    aggregated.map((point) => ({
      input: point.input,
      cached: point.cached,
      output: point.output,
      reasoning: point.reasoning,
    })),
    [
      { input: 30, cached: 5, output: 12, reasoning: 3 },
      { input: 30, cached: 4, output: 9, reasoning: 3 },
    ],
  )
})

test('buildTokenTrend 在长时间窗口下保留带日期的小时标签', () => {
  const summary: UsageSummaryQueryResponse = {
    start_ts: 0,
    end_ts: 0,
    account_total_requests: 0,
    tenant_api_key_total_requests: 0,
    unique_account_count: 0,
    unique_tenant_api_key_count: 0,
    dashboard_metrics: {
      total_requests: 0,
      token_breakdown: {
        input_tokens: 0,
        cached_input_tokens: 0,
        output_tokens: 0,
        reasoning_tokens: 0,
        total_tokens: 0,
      },
      avg_first_token_latency_ms: 0,
      model_request_distribution: [],
      model_token_distribution: [],
      token_trends: [
        {
          hour_start: 1_710_000_000,
          request_count: 1,
          input_tokens: 10,
          cached_input_tokens: 0,
          output_tokens: 0,
          reasoning_tokens: 0,
          total_tokens: 10,
        },
      ],
    },
  }

  const aggregated = buildTokenTrend(summary, {
    granularity: 'hour',
    rangeSeconds: 7 * 86400,
  })

  assert.equal(aggregated.length, 1)
  assert.match(aggregated[0]?.hour ?? '', /^\d{2}-\d{2} \d{2}:00$/)
})
