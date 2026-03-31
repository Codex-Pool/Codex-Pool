import { apiClient } from './client'
import type { OAuthImportJobSummary } from './importJobs'

const ACCOUNT_MUTATION_TIMEOUT_MS = 30_000
const ACCOUNT_BATCH_MUTATION_TIMEOUT_MS = 120_000

type AccountBatchActionInput =
  | 'enable'
  | 'disable'
  | 'delete'
  | 'refreshLogin'
  | 'pauseFamily'
  | 'resumeFamily'

type AccountBatchActionWire =
  | 'enable'
  | 'disable'
  | 'delete'
  | 'refresh_login'
  | 'pause_family'
  | 'resume_family'

const ACCOUNT_BATCH_ACTION_TO_WIRE: Record<AccountBatchActionInput, AccountBatchActionWire> = {
  enable: 'enable',
  disable: 'disable',
  delete: 'delete',
  refreshLogin: 'refresh_login',
  pauseFamily: 'pause_family',
  resumeFamily: 'resume_family',
}

export interface UpstreamAccount {
  id: string
  label: string
  mode: 'chat_gpt_session' | 'codex_oauth' | 'open_ai_api_key' | string
  base_url: string
  bearer_token: string
  chatgpt_account_id?: string
  enabled: boolean
  priority: number
  created_at: string
}

export type OAuthLiveResultSource = 'active' | 'passive'
export type AccountProbeOutcome = 'ok' | 'quota' | 'transient' | 'fatal'
export type AccountHealthFreshness = 'fresh' | 'stale' | 'unknown'
export type RefreshCredentialState = 'healthy' | 'degraded' | 'missing' | 'invalid'

export interface OAuthAccountStatusResponse {
  account_id: string
  auth_provider: 'legacy_bearer' | 'oauth_refresh_token'
  credential_kind?: 'refresh_rotatable' | 'one_time_access_token'
  operator_state?: AccountPoolOperatorState
  reason_class?: AccountPoolReasonClass
  route_eligible?: boolean
  pool_state?: 'active' | 'quarantine' | 'pending_purge'
  quarantine_reason?: string
  quarantine_until?: string
  pending_purge_at?: string
  pending_purge_reason?: string
  last_live_result_at?: string
  last_live_result_status?: 'ok' | 'failed'
  last_live_result_source?: OAuthLiveResultSource
  last_live_result_status_code?: number
  last_live_error_code?: string
  last_live_error_message_preview?: string
  email?: string
  oauth_subject?: string
  oauth_identity_provider?: string
  email_verified?: boolean
  chatgpt_plan_type?: string
  chatgpt_user_id?: string
  chatgpt_subscription_active_start?: string
  chatgpt_subscription_active_until?: string
  chatgpt_subscription_last_checked?: string
  chatgpt_account_user_id?: string
  chatgpt_compute_residency?: string
  workspace_name?: string
  organizations?: unknown[]
  groups?: unknown[]
  source_type?: string
  token_family_id?: string
  token_version?: number
  token_expires_at?: string
  last_refresh_at?: string
  last_refresh_status: 'never' | 'ok' | 'failed'
  refresh_reused_detected: boolean
  last_refresh_error_code?: string
  last_refresh_error?: string
  has_refresh_credential?: boolean
  has_access_token_fallback?: boolean
  refresh_credential_state?: RefreshCredentialState
  supported_models?: string[]
  rate_limits?: OAuthRateLimitSnapshot[]
  rate_limits_fetched_at?: string
  rate_limits_expires_at?: string
  rate_limits_last_error_code?: string
  rate_limits_last_error?: string
  next_refresh_at?: string
}

export interface OAuthRateLimitSnapshot {
  limit_id?: string
  limit_name?: string
  primary?: OAuthRateLimitWindow
  secondary?: OAuthRateLimitWindow
}

export interface OAuthRateLimitWindow {
  used_percent: number
  window_minutes?: number
  resets_at?: string
}

export interface OAuthFamilyActionResponse {
  account_id: string
  token_family_id?: string
  enabled: boolean
  affected_accounts: number
}

export interface UpstreamAccountBatchActionError {
  code: string
  message: string
}

export interface UpstreamAccountBatchActionItem {
  account_id: string
  ok: boolean
  job_id?: string
  error?: UpstreamAccountBatchActionError
}

export interface UpstreamAccountBatchActionResponse {
  action: AccountBatchActionWire
  total: number
  success_count: number
  failed_count: number
  items: UpstreamAccountBatchActionItem[]
}

export type OAuthRateLimitRefreshJobStatus =
  | 'queued'
  | 'running'
  | 'completed'
  | 'failed'
  | 'cancelled'

export interface OAuthRateLimitRefreshErrorSummary {
  error_code: string
  count: number
}

export interface OAuthRateLimitRefreshJobSummary {
  job_id: string
  status: OAuthRateLimitRefreshJobStatus
  total: number
  processed: number
  success_count: number
  failed_count: number
  started_at?: string
  finished_at?: string
  created_at: string
  throughput_per_min?: number
  error_summary?: OAuthRateLimitRefreshErrorSummary[]
}

export type OAuthInventoryStatus =
  | 'queued'
  | 'ready'
  | 'needs_refresh'
  | 'no_quota'
  | 'failed'

export type OAuthInventoryFailureStage =
  | 'admission_probe'
  | 'activation_refresh'
  | 'activation_rate_limits'
  | 'runtime_refresh'

export interface OAuthInventoryRecord {
  id: string
  label: string
  email?: string
  chatgpt_account_id?: string
  chatgpt_plan_type?: string
  source_type?: string
  vault_status: OAuthInventoryStatus
  has_refresh_token: boolean
  has_access_token_fallback: boolean
  admission_source?: string
  admission_checked_at?: string
  admission_retry_after?: string
  admission_error_code?: string
  admission_error_message?: string
  admission_rate_limits?: OAuthRateLimitSnapshot[]
  admission_rate_limits_expires_at?: string
  failure_stage?: OAuthInventoryFailureStage
  attempt_count: number
  transient_retry_count: number
  next_retry_at?: string
  retryable: boolean
  terminal_reason?: string
  created_at: string
  updated_at: string
}

export interface OAuthInventorySummaryResponse {
  total: number
  queued: number
  ready: number
  needs_refresh: number
  no_quota: number
  failed: number
}

export interface OAuthRuntimePoolSummaryResponse {
  total: number
  active: number
  quarantine: number
  pending_purge: number
  oauth_refresh_token: number
  legacy_bearer: number
  rate_limits_ready: number
}

export interface OAuthHealthSignalsSummaryResponse {
  total: number
  live_result_ok: number
  live_result_failed: number
  pending_purge_signals: number
  quarantine_signals: number
}

export type AccountPoolRecordScope = 'runtime' | 'inventory'
export type AccountPoolOperatorState = 'inventory' | 'routable' | 'cooling' | 'pending_delete'
export type AccountPoolReasonClass = 'healthy' | 'quota' | 'fatal' | 'transient' | 'admin'

export interface AccountSignalHeatmapSummary {
  bucket_minutes: number
  window_minutes: number
  window_start: string
  intensity_levels: number[]
  active_counts: number[]
  passive_counts: number[]
  success_counts: number[]
  error_counts: number[]
  latest_signal_at?: string
  latest_signal_source?: OAuthLiveResultSource
}

export interface AccountSignalHeatmapBucket {
  start_at: string
  signal_count: number
  intensity: number
  active_count: number
  passive_count: number
  success_count: number
  error_count: number
}

export interface AccountSignalHeatmapDetail {
  record_id: string
  bucket_minutes: number
  window_minutes: number
  window_start: string
  buckets: AccountSignalHeatmapBucket[]
  latest_signal_at?: string
  latest_signal_source?: OAuthLiveResultSource
}

export interface AccountPoolRecord {
  id: string
  record_scope: AccountPoolRecordScope
  operator_state: AccountPoolOperatorState
  health_freshness: AccountHealthFreshness
  reason_class: AccountPoolReasonClass
  reason_code?: string
  route_eligible: boolean
  next_action_at?: string
  last_signal_at?: string
  last_signal_source?: OAuthLiveResultSource
  recent_signal_heatmap?: AccountSignalHeatmapSummary
  last_probe_at?: string
  last_probe_outcome?: AccountProbeOutcome
  label: string
  email?: string
  chatgpt_account_id?: string
  chatgpt_plan_type?: string
  source_type?: string
  mode?: UpstreamAccount['mode']
  auth_provider?: OAuthAccountStatusResponse['auth_provider']
  credential_kind?: OAuthAccountStatusResponse['credential_kind']
  has_refresh_credential: boolean
  has_access_token_fallback: boolean
  refresh_credential_state?: RefreshCredentialState
  enabled?: boolean
  rate_limits: OAuthRateLimitSnapshot[]
  rate_limits_fetched_at?: string
  created_at: string
  updated_at: string
}

export interface AccountPoolSummaryResponse {
  total: number
  inventory: number
  routable: number
  cooling: number
  pending_delete: number
  healthy: number
  quota: number
  fatal: number
  transient: number
  admin: number
}

export type AccountPoolAction = 'reprobe' | 'restore' | 'delete'

export interface AccountPoolActionError {
  code: string
  message: string
}

export interface AccountPoolActionItem {
  record_id: string
  ok: boolean
  error?: AccountPoolActionError
}

export interface AccountPoolActionResponse {
  action: AccountPoolAction
  total: number
  success_count: number
  failed_count: number
  items: AccountPoolActionItem[]
}

export interface AccountPoolResponsesTestRequest {
  model: string
  input: string
  previous_response_id?: string
}

export interface AccountPoolResponsesTestUsage {
  input_tokens?: number
  cached_input_tokens?: number
  output_tokens?: number
  reasoning_tokens?: number
}

export interface AccountPoolResponsesTestResponse {
  response_id: string
  assistant_text: string
  request_id: string
  model: string
  status_code: number
  latency_ms: number
  usage?: AccountPoolResponsesTestUsage
  previous_response_id?: string
}

export const accountsApi = {
  listAccounts: async () => {
    const response = await apiClient.get<UpstreamAccount[]>('/upstream-accounts')
    return response.data
  },

  setEnabled: async (accountId: string, enabled: boolean) => {
    const response = await apiClient.patch<UpstreamAccount>(
      `/upstream-accounts/${accountId}`,
      { enabled },
      { timeout: ACCOUNT_MUTATION_TIMEOUT_MS },
    )
    return response.data
  },

  deleteAccount: (accountId: string) =>
    apiClient.delete<void>(`/upstream-accounts/${accountId}`, {
      timeout: ACCOUNT_MUTATION_TIMEOUT_MS,
    }),

  listOAuthStatuses: async (accountIds: string[]) => {
    if (accountIds.length === 0) {
      return [] as OAuthAccountStatusResponse[]
    }

    const response = await apiClient.post<{ items: OAuthAccountStatusResponse[] }>(
      '/upstream-accounts/oauth/statuses',
      { account_ids: accountIds },
      { timeout: 30_000 },
    )
    return response.data.items
  },

  getOAuthStatus: async (accountId: string) => {
    const response = await apiClient.get<OAuthAccountStatusResponse>(
      `/upstream-accounts/${accountId}/oauth/status`,
    )
    return response.data
  },

  getOAuthInventorySummary: async () => {
    const response = await apiClient.get<OAuthInventorySummaryResponse>(
      '/upstream-accounts/oauth/inventory/summary',
    )
    return response.data
  },

  getOAuthInventoryRecords: async () => {
    const response = await apiClient.get<OAuthInventoryRecord[]>(
      '/upstream-accounts/oauth/inventory/records',
    )
    return response.data
  },

  getOAuthRuntimePoolSummary: async () => {
    const response = await apiClient.get<OAuthRuntimePoolSummaryResponse>(
      '/upstream-accounts/runtime/summary',
    )
    return response.data
  },

  getOAuthHealthSignalsSummary: async () => {
    const response = await apiClient.get<OAuthHealthSignalsSummaryResponse>(
      '/upstream-accounts/health/signals/summary',
    )
    return response.data
  },

  refreshOAuth: async (accountId: string) => {
    const response = await apiClient.post<OAuthAccountStatusResponse>(
      `/upstream-accounts/${accountId}/oauth/refresh`,
      undefined,
      { timeout: ACCOUNT_MUTATION_TIMEOUT_MS },
    )
    return response.data
  },

  refreshOAuthJob: async (accountId: string) => {
    const response = await apiClient.post<OAuthImportJobSummary>(
      `/upstream-accounts/${accountId}/oauth/refresh-jobs`,
      undefined,
      { timeout: ACCOUNT_MUTATION_TIMEOUT_MS },
    )
    return response.data
  },

  createRateLimitRefreshJob: async () => {
    const response = await apiClient.post<OAuthRateLimitRefreshJobSummary>(
      '/upstream-accounts/oauth/rate-limits/refresh-jobs',
    )
    return response.data
  },

  getRateLimitRefreshJob: async (jobId: string) => {
    const response = await apiClient.get<OAuthRateLimitRefreshJobSummary>(
      `/upstream-accounts/oauth/rate-limits/refresh-jobs/${jobId}`,
    )
    return response.data
  },

  disableFamily: async (accountId: string) => {
    const response = await apiClient.post<OAuthFamilyActionResponse>(
      `/upstream-accounts/${accountId}/oauth/family/disable`,
      undefined,
      { timeout: ACCOUNT_MUTATION_TIMEOUT_MS },
    )
    return response.data
  },

  enableFamily: async (accountId: string) => {
    const response = await apiClient.post<OAuthFamilyActionResponse>(
      `/upstream-accounts/${accountId}/oauth/family/enable`,
      undefined,
      { timeout: ACCOUNT_MUTATION_TIMEOUT_MS },
    )
    return response.data
  },

  batchOperate: async (action: AccountBatchActionInput, accountIds: string[]) => {
    const response = await apiClient.post<UpstreamAccountBatchActionResponse>(
      '/upstream-accounts/batch-actions',
      {
        action: ACCOUNT_BATCH_ACTION_TO_WIRE[action],
        account_ids: accountIds,
      },
      { timeout: ACCOUNT_BATCH_MUTATION_TIMEOUT_MS },
    )
    return response.data
  },
}

export const accountPoolApi = {
  getSummary: async () => {
    const response = await apiClient.get<AccountPoolSummaryResponse>('/account-pool/summary')
    return response.data
  },

  listRecords: async () => {
    const response = await apiClient.get<AccountPoolRecord[]>('/account-pool/accounts')
    return response.data
  },

  getRecord: async (recordId: string) => {
    const response = await apiClient.get<AccountPoolRecord>(`/account-pool/accounts/${recordId}`)
    return response.data
  },

  getSignalHeatmap: async (recordId: string) => {
    const response = await apiClient.get<AccountSignalHeatmapDetail | null>(
      `/account-pool/accounts/${recordId}/signal-heatmap`,
    )
    return response.data
  },

  runAction: async (action: AccountPoolAction, recordIds: string[]) => {
    const response = await apiClient.post<AccountPoolActionResponse>(
      '/account-pool/actions',
      {
        action,
        record_ids: recordIds,
      },
      { timeout: ACCOUNT_BATCH_MUTATION_TIMEOUT_MS },
    )
    return response.data
  },

  testResponses: async (
    recordId: string,
    payload: AccountPoolResponsesTestRequest,
  ) => {
    const response = await apiClient.post<AccountPoolResponsesTestResponse>(
      `/account-pool/accounts/${recordId}/responses/test`,
      payload,
      { timeout: ACCOUNT_MUTATION_TIMEOUT_MS },
    )
    return response.data
  },
}
