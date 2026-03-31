import {
  createAuthApiClient,
  extractApiErrorCodeFrom as extractApiErrorCodeFromHttp,
  extractApiErrorMessageFrom as extractApiErrorMessageFromHttp,
  extractApiErrorStatusFrom as extractApiErrorStatusFromHttp,
} from './httpClient'
import { getAdminAccessToken } from '@/lib/admin-session'

export type { ApiErrorBody } from './httpClient'

export const AUTH_REQUIRED_EVENT = 'cp:auth-required'
export const SESSION_EXPIRED_REASON = 'session-expired'
export const LOGIN_FAILED_EVENT = 'cp:login-failed'

function isAuthEndpoint(url?: string): boolean {
  if (!url) {
    return false
  }
  return (
    url.includes('/admin/auth/login') ||
    url.includes('/admin/auth/me') ||
    url.includes('/admin/auth/logout')
  )
}

function shouldDispatchAdminAuthRequired(error: Parameters<NonNullable<Parameters<typeof createAuthApiClient>[0]['shouldDispatchAuthRequired']>>[0]) {
  const url = error?.config?.url
  if (!url) {
    return true
  }

  return !/\/account-pool\/accounts\/[^/]+\/responses\/test(?:\?.*)?$/.test(url)
}

export const apiClient = createAuthApiClient({
  baseURL: '/api/v1',
  timeout: 10_000,
  getAccessToken: getAdminAccessToken,
  isAuthEndpoint,
  isLoginEndpoint: (url) => Boolean(url?.includes('/admin/auth/login')),
  authRequiredEvent: AUTH_REQUIRED_EVENT,
  loginFailedEvent: LOGIN_FAILED_EVENT,
  authRequiredDetail: { reason: SESSION_EXPIRED_REASON },
  logDevErrors: true,
  unwrapResponseData: false,
  shouldDispatchAuthRequired: shouldDispatchAdminAuthRequired,
})

export function extractApiErrorMessage(error: unknown): string | null {
  return extractApiErrorMessageFromHttp(error)
}

export function extractApiErrorCode(error: unknown): string | null {
  return extractApiErrorCodeFromHttp(error)
}

export function extractApiErrorStatus(error: unknown): number | null {
  return extractApiErrorStatusFromHttp(error)
}

export const extractApiErrorMessageFrom = extractApiErrorMessage
export const extractApiErrorCodeFrom = extractApiErrorCode
export const extractApiErrorStatusFrom = extractApiErrorStatus
