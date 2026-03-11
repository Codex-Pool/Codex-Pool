import { apiClient } from './client'
import type { OAuthImportJobSummary } from './importJobs'

const ACCOUNT_MUTATION_TIMEOUT_MS = 30000
const ACCOUNT_BATCH_MUTATION_TIMEOUT_MS = 120000

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
    id: string;
    label: string;
    mode: 'chat_gpt_session' | 'codex_oauth' | 'open_ai_api_key' | string;
    base_url: string;
    bearer_token: string;
    chatgpt_account_id?: string;
    enabled: boolean;
    priority: number;
    created_at: string;
}

export interface OAuthAccountStatusResponse {
    account_id: string
    auth_provider: 'legacy_bearer' | 'oauth_refresh_token'
    credential_kind?: 'refresh_rotatable' | 'one_time_access_token'
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
    effective_enabled: boolean
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

export const accountsApi = {
    listAccounts: () =>
        apiClient.get<UpstreamAccount[]>('/upstream-accounts'),

    setEnabled: (accountId: string, enabled: boolean) =>
        apiClient.patch<UpstreamAccount>(
            `/upstream-accounts/${accountId}`,
            { enabled },
            { timeout: ACCOUNT_MUTATION_TIMEOUT_MS },
        ),

    deleteAccount: (accountId: string) =>
        apiClient.delete<void>(
            `/upstream-accounts/${accountId}`,
            { timeout: ACCOUNT_MUTATION_TIMEOUT_MS },
        ),

    listOAuthStatuses: async (accountIds: string[]) => {
        if (accountIds.length === 0) {
            return [] as OAuthAccountStatusResponse[]
        }
        const response = await apiClient.post<{ items: OAuthAccountStatusResponse[] }>(
            '/upstream-accounts/oauth/statuses',
            { account_ids: accountIds },
            // 批量状态仅读缓存，不走上游实时拉取。
            { timeout: 30000 },
        )
        return response.items
    },

    getOAuthStatus: (accountId: string) =>
        apiClient.get<OAuthAccountStatusResponse>(`/upstream-accounts/${accountId}/oauth/status`),

    refreshOAuth: (accountId: string) =>
        apiClient.post<OAuthAccountStatusResponse>(
            `/upstream-accounts/${accountId}/oauth/refresh`,
            undefined,
            { timeout: ACCOUNT_MUTATION_TIMEOUT_MS },
        ),

    refreshOAuthJob: (accountId: string) =>
        apiClient.post<OAuthImportJobSummary>(
            `/upstream-accounts/${accountId}/oauth/refresh-jobs`,
            undefined,
            { timeout: ACCOUNT_MUTATION_TIMEOUT_MS },
        ),

    createRateLimitRefreshJob: () =>
        apiClient.post<OAuthRateLimitRefreshJobSummary>(
            '/upstream-accounts/oauth/rate-limits/refresh-jobs',
        ),

    getRateLimitRefreshJob: (jobId: string) =>
        apiClient.get<OAuthRateLimitRefreshJobSummary>(
            `/upstream-accounts/oauth/rate-limits/refresh-jobs/${jobId}`,
        ),

    disableFamily: (accountId: string) =>
        apiClient.post<OAuthFamilyActionResponse>(
            `/upstream-accounts/${accountId}/oauth/family/disable`,
            undefined,
            { timeout: ACCOUNT_MUTATION_TIMEOUT_MS },
        ),

    enableFamily: (accountId: string) =>
        apiClient.post<OAuthFamilyActionResponse>(
            `/upstream-accounts/${accountId}/oauth/family/enable`,
            undefined,
            { timeout: ACCOUNT_MUTATION_TIMEOUT_MS },
        ),

    batchOperate: (action: AccountBatchActionInput, accountIds: string[]) =>
        apiClient.post<UpstreamAccountBatchActionResponse>(
            '/upstream-accounts/batch-actions',
            {
                action: ACCOUNT_BATCH_ACTION_TO_WIRE[action],
                account_ids: accountIds,
            },
            { timeout: ACCOUNT_BATCH_MUTATION_TIMEOUT_MS },
        ),
}
