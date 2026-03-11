import { apiClient } from './client'
import type {
  CodexOAuthLoginSessionError,
  CodexOAuthLoginSessionStatus,
} from './oauthImport'

export interface CodexOAuthProbeSessionResult {
  base_url: string
  access_token_present: boolean
  refresh_token_present: boolean
  id_token_present: boolean
  access_token_preview?: string
  refresh_token_preview?: string
  token_type?: string
  scope?: string
  expires_at: string
  email?: string
  chatgpt_account_id?: string
  chatgpt_plan_type?: string
  raw_exchange_payload?: unknown
  id_token_claims_raw?: unknown
}

export interface CodexOAuthProbeSession {
  session_id: string
  status: CodexOAuthLoginSessionStatus
  authorize_url: string
  callback_url: string
  created_at: string
  updated_at: string
  expires_at: string
  error?: CodexOAuthLoginSessionError
  result?: CodexOAuthProbeSessionResult
}

export interface CreateCodexOAuthProbeSessionRequest {
  base_url?: string
}

export const oauthProbeApi = {
  createCodexProbeSession: (payload: CreateCodexOAuthProbeSessionRequest) =>
    apiClient.post<CodexOAuthProbeSession>(
      '/upstream-accounts/oauth/codex/probe-sessions',
      payload,
      { timeout: 30000 },
    ),

  getCodexProbeSession: (sessionId: string) =>
    apiClient.get<CodexOAuthProbeSession>(
      `/upstream-accounts/oauth/codex/probe-sessions/${sessionId}`,
      { timeout: 30000 },
    ),

  submitCodexProbeCallback: (sessionId: string, redirectUrl: string) =>
    apiClient.post<CodexOAuthProbeSession>(
      `/upstream-accounts/oauth/codex/probe-sessions/${sessionId}/callback`,
      { redirect_url: redirectUrl },
      { timeout: 30000 },
    ),
}
