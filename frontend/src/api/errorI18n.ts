import type { AxiosError } from 'axios'
import type { TFunction } from 'i18next'

import type { ApiErrorBody } from './httpClient'
import {
  extractApiErrorCodeFrom,
  extractApiErrorMessageFrom,
  extractApiErrorStatusFrom,
} from './httpClient'

export interface LocalizedErrorDisplay {
  label: string
  tooltip?: string
  code?: string
  status?: number
}

function normalizeCode(code: string | null | undefined): string | null {
  const value = (code ?? '').trim().toLowerCase()
  return value ? value : null
}

function truncate(value: string, maxLen: number): string {
  if (value.length <= maxLen) {
    return value
  }
  return `${value.slice(0, maxLen)}…`
}

function isAxiosError(error: unknown): error is AxiosError<ApiErrorBody> {
  return Boolean((error as AxiosError<ApiErrorBody> | undefined)?.isAxiosError)
}

function apiErrorCodeToLabel(t: TFunction, code: string | null): string | null {
  switch (code) {
    case 'unauthorized':
      return t('errors.api.unauthorized')
    case 'invalid_request':
      return t('errors.api.invalidRequest')
    case 'invalid_proxy_url':
      return t('errors.api.invalidProxyUrl')
    case 'not_found':
      return t('errors.api.notFound')
    case 'service_unavailable':
      return t('errors.api.serviceUnavailable')
    case 'internal_error':
      return t('errors.api.internalError')
    case 'oauth_provider_not_configured':
      return t('errors.api.oauthProviderNotConfigured')
    case 'oauth_callback_listener_unavailable':
      return t('errors.api.oauthCallbackListenerUnavailable')
    case 'invalid_refresh_token':
      return t('errors.api.invalidRefreshToken')
    case 'refresh_token_reused':
      return t('errors.api.refreshTokenReused')
    case 'refresh_token_revoked':
      return t('errors.api.refreshTokenRevoked')
    case 'missing_client_id':
      return t('errors.api.oauthMissingClientId')
    case 'unauthorized_client':
      return t('errors.api.oauthUnauthorizedClient')
    case 'rate_limited':
      return t('errors.http.rateLimited')
    case 'upstream_unavailable':
      return t('errors.api.upstreamUnavailable')
    case 'upstream_network_error':
      return t('errors.api.upstreamNetworkError')
    case 'oauth_exchange_failed':
      return t('errors.api.oauthExchangeFailed')
    default:
      return null
  }
}

function httpStatusToLabel(t: TFunction, status: number | null): string | null {
  switch (status) {
    case 400:
      return t('errors.http.badRequest')
    case 401:
      return t('errors.http.unauthorized')
    case 403:
      return t('errors.http.forbidden')
    case 404:
      return t('errors.http.notFound')
    case 409:
      return t('errors.http.conflict')
    case 413:
      return t('errors.http.payloadTooLarge')
    case 429:
      return t('errors.http.rateLimited')
    case 500:
      return t('errors.http.internalServerError')
    case 502:
      return t('errors.http.badGateway')
    case 503:
      return t('errors.http.serviceUnavailable')
    case 504:
      return t('errors.http.gatewayTimeout')
    default:
      return null
  }
}

export function localizeApiErrorDisplay(
  t: TFunction,
  error: unknown,
  fallbackLabel: string,
): LocalizedErrorDisplay {
  const status = extractApiErrorStatusFrom(error)
  const rawCode = extractApiErrorCodeFrom(error)
  const rawMessage = extractApiErrorMessageFrom(error)
  const code = normalizeCode(rawCode)

  const axiosCode = isAxiosError(error)
    ? typeof error.code === 'string'
      ? error.code
      : null
    : null

  const timeout = axiosCode === 'ECONNABORTED' || axiosCode === 'ETIMEDOUT'
  const network =
    axiosCode === 'ERR_NETWORK' || (rawMessage ? rawMessage.toLowerCase().includes('network') : false)

  const label =
    apiErrorCodeToLabel(t, code) ??
    httpStatusToLabel(t, typeof status === 'number' ? status : null) ??
    (timeout ? t('errors.common.timeout') : null) ??
    (network ? t('errors.common.network') : null) ??
    fallbackLabel

  const tooltipParts: string[] = []
  if (code) tooltipParts.push(`code=${code}`)
  if (typeof status === 'number') tooltipParts.push(`status=${status}`)
  if (import.meta.env.DEV && rawMessage) tooltipParts.push(`message=${truncate(rawMessage, 200)}`)

  return {
    label,
    tooltip: tooltipParts.length ? tooltipParts.join(' | ') : undefined,
    code: code ?? undefined,
    status: typeof status === 'number' ? status : undefined,
  }
}

export function localizeHttpStatusDisplay(
  t: TFunction,
  status: number | null | undefined,
  fallbackLabel: string,
): LocalizedErrorDisplay {
  const label =
    httpStatusToLabel(t, typeof status === 'number' ? status : null) ??
    fallbackLabel
  return {
    label,
    tooltip: typeof status === 'number' ? `status=${status}` : undefined,
    status: typeof status === 'number' ? status : undefined,
  }
}

export function localizeRequestLogErrorDisplay(
  t: TFunction,
  errorCode: string | null | undefined,
  statusCode: number | null | undefined,
): LocalizedErrorDisplay {
  const raw = (errorCode ?? '').trim()
  const numeric = raw ? Number(raw) : undefined
  const status = typeof numeric === 'number' && !Number.isNaN(numeric) ? numeric : statusCode
  const display = localizeHttpStatusDisplay(t, status, raw ? t('errors.common.failed') : '-')
  const tooltip = raw ? `error_code=${raw}` : display.tooltip
  return { ...display, tooltip }
}

export function localizeOAuthErrorCodeDisplay(
  t: TFunction,
  errorCode: string | null | undefined,
): LocalizedErrorDisplay {
  const normalized = normalizeCode(errorCode)
  const label =
    apiErrorCodeToLabel(t, normalized) ??
    (normalized ? t('errors.common.failed') : '-')
  return {
    label,
    tooltip: normalized ? `code=${normalized}` : undefined,
    code: normalized ?? undefined,
  }
}
