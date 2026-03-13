import { apiClient } from './client'
import { tenantApiClient } from './tenantClient'

export interface RequestLogQueryParams {
  start_ts?: number
  end_ts?: number
  limit?: number
  tenant_id?: string
  api_key_id?: string
  status_code?: number
  request_id?: string
  keyword?: string
}

export interface RequestAuditLogItem {
  id: string
  account_id: string
  tenant_id?: string
  api_key_id?: string
  request_id?: string
  path: string
  method: string
  model?: string
  service_tier?: string
  status_code: number
  latency_ms: number
  is_stream: boolean
  error_code?: string
  estimated_cost_microusd?: number
  event_version: number
  created_at: string
}

export interface RequestLogsResponse {
  items: RequestAuditLogItem[]
}

export const requestLogsApi = {
  adminList: (params: RequestLogQueryParams) =>
    apiClient.get<RequestLogsResponse>('/admin/request-logs', {
      params,
    }),
  tenantList: (params: Omit<RequestLogQueryParams, 'tenant_id'>) =>
    tenantApiClient.get<RequestLogsResponse>('/request-logs', {
      params,
    }),
}
