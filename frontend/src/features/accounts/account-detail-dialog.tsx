import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'

import type { OAuthAccountStatusResponse, UpstreamAccount } from '@/api/accounts'
import { localizeOAuthErrorCodeDisplay } from '@/api/errorI18n'
import { AccessibleTabList } from '@/components/ui/accessible-tabs'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { cn } from '@/lib/utils'

import type { AccountDetailTab, RateLimitDisplay } from './types'
import {
  bucketBarClass,
  bucketLabel,
  clampPercent,
  formatAbsoluteDateTime,
  formatRateLimitResetText,
  getAuthProviderLabel,
  getCredentialKindLabel,
  getModeLabel,
  getPoolStateBadgeVariant,
  getPoolStateLabel,
  getPlanLabel,
  getRefreshCredentialStateLabel,
  getRefreshStatusLabel,
  getSourceTypeLabel,
} from './utils'

type AccountDetailDialogProps = {
  account: UpstreamAccount | null
  detailTab: AccountDetailTab
  onDetailTabChange: (tab: AccountDetailTab) => void
  onOpenChange: (open: boolean) => void
  isSessionAccount: boolean
  oauthStatus?: OAuthAccountStatusResponse
  oauthStatusLoading: boolean
  rateLimitDisplays: RateLimitDisplay[]
  locale: string
}

type DetailSectionProps = {
  title: string
  children: ReactNode
  className?: string
}

type DetailFieldProps = {
  label: string
  children?: ReactNode
  mono?: boolean
  scrollable?: boolean
  title?: string
  containerClassName?: string
  className?: string
}

function DetailSection({ title, children, className }: DetailSectionProps) {
  return (
    <Card className={cn('gap-0 overflow-hidden border-border/70 py-0 shadow-none', className)}>
      <CardHeader className="border-b bg-muted/20 px-4 py-3">
        <CardTitle className="text-sm font-semibold">{title}</CardTitle>
      </CardHeader>
      <CardContent className="px-4 py-4">{children}</CardContent>
    </Card>
  )
}

function DetailField({
  label,
  children,
  mono = false,
  scrollable = false,
  title,
  containerClassName,
  className,
}: DetailFieldProps) {
  const isEmpty = children === null || children === undefined || children === ''

  return (
    <div className={cn('space-y-1.5', containerClassName)}>
      <div className="text-xs font-medium text-muted-foreground">{label}</div>
      <div
        className={cn(
          'rounded-lg border bg-background px-3 py-2 text-sm',
          mono ? 'font-mono text-xs break-all' : 'break-words',
          scrollable ? 'max-h-32 overflow-auto' : '',
          className,
        )}
        title={title}
      >
        {isEmpty ? <span className="text-muted-foreground">-</span> : children}
      </div>
    </div>
  )
}

function formatOptionalDateTime(value?: string) {
  if (!value) {
    return '-'
  }
  return formatAbsoluteDateTime(value)
}

function renderRefreshStatusBadge(
  status: OAuthAccountStatusResponse['last_refresh_status'],
  label: string,
) {
  if (status === 'failed') {
    return <Badge variant="destructive">{label}</Badge>
  }
  if (status === 'ok') {
    return <Badge variant="success">{label}</Badge>
  }
  return <Badge variant="secondary">{label}</Badge>
}

export function AccountDetailDialog({
  account,
  detailTab,
  onDetailTabChange,
  onOpenChange,
  isSessionAccount,
  oauthStatus,
  oauthStatusLoading,
  rateLimitDisplays,
  locale,
}: AccountDetailDialogProps) {
  const { t } = useTranslation()
  const fieldLabel = (key: string, defaultValue: string) =>
    t(`accounts.details.fields.${key}`, { defaultValue })

  const primaryIdentity = oauthStatus?.email?.trim() || account?.label || '-'
  const refreshErrorDisplay = localizeOAuthErrorCodeDisplay(t, oauthStatus?.last_refresh_error_code)
  const rateLimitErrorDisplay = localizeOAuthErrorCodeDisplay(
    t,
    oauthStatus?.rate_limits_last_error_code,
  )
  const sourceTypeLabel = getSourceTypeLabel(oauthStatus?.source_type, t)
  const supportedModels = oauthStatus?.supported_models ?? []

  return (
    <Dialog open={Boolean(account)} onOpenChange={onOpenChange}>
      <DialogContent className="flex max-h-[90vh] flex-col gap-0 overflow-hidden p-0 sm:max-w-6xl">
        <DialogHeader className="shrink-0 border-b px-6 py-5 pr-14">
          <DialogTitle>
            {account
              ? `${t('accounts.actions.viewDetails', { defaultValue: 'View Details' })} · ${primaryIdentity}`
              : t('accounts.actions.viewDetails', { defaultValue: 'View Details' })}
          </DialogTitle>
          <DialogDescription>
            {t('accounts.details.description', {
              defaultValue: 'View account profile, OAuth status, limits, and raw payloads.',
            })}
          </DialogDescription>
        </DialogHeader>

        {account ? (
          <div className="flex min-h-0 flex-1 flex-col">
            <div className="shrink-0 border-b px-6 py-4">
              <AccessibleTabList
                idBase="account-detail"
                ariaLabel={t('accounts.details.tabAria', { defaultValue: 'Account detail tabs' })}
                value={detailTab}
                onValueChange={onDetailTabChange}
                items={[
                  {
                    value: 'profile',
                    label: t('accounts.details.tabs.profile', { defaultValue: 'Profile' }),
                  },
                  {
                    value: 'oauth',
                    label: t('accounts.details.tabs.oauth', { defaultValue: 'OAuth' }),
                  },
                  {
                    value: 'limits',
                    label: t('accounts.details.tabs.limits', { defaultValue: 'Limits' }),
                  },
                  {
                    value: 'raw',
                    label: t('accounts.details.tabs.raw', { defaultValue: 'Raw' }),
                  },
                ]}
              />
            </div>

            <div className="min-h-0 flex-1 overflow-y-auto px-6 py-5">
              {detailTab === 'profile' ? (
                <section
                  id="account-detail-panel-profile"
                  role="tabpanel"
                  tabIndex={0}
                  aria-labelledby="account-detail-tab-profile"
                  className="space-y-4"
                >
                  <h3 className="text-base font-semibold">
                    {t('accounts.details.profileTitle', { defaultValue: 'Account Profile' })}
                  </h3>

                  <div className="grid grid-cols-1 gap-4 xl:grid-cols-[minmax(0,1.6fr)_minmax(0,1fr)]">
                    <DetailSection
                      title={t('accounts.details.sections.identity', {
                        defaultValue: 'Identity',
                      })}
                    >
                      <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
                        <DetailField label={fieldLabel('email', 'Email')}>
                          {oauthStatus?.email ?? '-'}
                        </DetailField>
                        <DetailField label={fieldLabel('label', 'Label')}>
                          {account.label}
                        </DetailField>
                        <DetailField label={fieldLabel('mode', 'Mode')}>
                          {getModeLabel(account.mode, t)}
                        </DetailField>
                        <DetailField label={fieldLabel('enabled', 'Enabled')}>
                          <Badge variant={account.enabled ? 'success' : 'warning'}>
                            {account.enabled
                              ? t('accounts.status.active')
                              : t('accounts.status.disabled')}
                          </Badge>
                        </DetailField>
                        <DetailField label={fieldLabel('createdAt', 'Created At')}>
                          {formatAbsoluteDateTime(account.created_at)}
                        </DetailField>
                      </div>
                    </DetailSection>

                    <DetailSection
                      title={t('accounts.details.sections.connection', {
                        defaultValue: 'Connection',
                      })}
                    >
                      <div className="grid grid-cols-1 gap-3">
                        <DetailField
                          label={fieldLabel('baseUrl', 'Base URL')}
                          mono
                          title={account.base_url}
                        >
                          {account.base_url}
                        </DetailField>
                        <DetailField label={fieldLabel('priority', 'Priority')}>
                          {String(account.priority)}
                        </DetailField>
                      </div>
                    </DetailSection>

                    <DetailSection
                      title={t('accounts.details.sections.credentials', {
                        defaultValue: 'Credentials',
                      })}
                      className="xl:col-span-2"
                    >
                      <DetailField
                        label={fieldLabel('bearerToken', 'Bearer Token')}
                        mono
                        scrollable
                        title={account.bearer_token}
                        className="max-h-40"
                      >
                        {account.bearer_token}
                      </DetailField>
                    </DetailSection>

                    <DetailSection
                      title={t('accounts.details.sections.supportedModels', {
                        defaultValue: 'Available Models',
                      })}
                      className="xl:col-span-2"
                    >
                      {!isSessionAccount ? (
                        <p className="text-sm text-muted-foreground">
                          {t('accounts.details.oauthNotApplicable', {
                            defaultValue: 'OAuth details are not available for this account type.',
                          })}
                        </p>
                      ) : oauthStatusLoading && !oauthStatus ? (
                        <p className="text-sm text-muted-foreground">
                          {t('accounts.oauth.loading')}
                        </p>
                      ) : supportedModels.length === 0 ? (
                        <p className="text-sm text-muted-foreground">
                          {t('accounts.details.noSupportedModels', {
                            defaultValue: 'No available model list has been captured for this account yet.',
                          })}
                        </p>
                      ) : (
                        <div className="space-y-3">
                          <p className="text-xs text-muted-foreground">
                            {t('accounts.details.supportedModelsCount', {
                              defaultValue: '{{count}} models',
                              count: supportedModels.length,
                            })}
                          </p>
                          <div className="flex flex-wrap gap-2">
                            {supportedModels.map((model) => (
                              <Badge
                                key={model}
                                variant="outline"
                                className="font-mono text-[11px]"
                                title={model}
                              >
                                {model}
                              </Badge>
                            ))}
                          </div>
                        </div>
                      )}
                    </DetailSection>
                  </div>
                </section>
              ) : null}

              {detailTab === 'oauth' ? (
                <section
                  id="account-detail-panel-oauth"
                  role="tabpanel"
                  tabIndex={0}
                  aria-labelledby="account-detail-tab-oauth"
                  className="space-y-4"
                >
                  <h3 className="text-base font-semibold">
                    {t('accounts.details.oauthTitle', { defaultValue: 'OAuth Status' })}
                  </h3>
                  {!isSessionAccount ? (
                    <p className="text-sm text-muted-foreground">
                      {t('accounts.details.oauthNotApplicable', {
                        defaultValue: 'OAuth details are not available for this account type.',
                      })}
                    </p>
                  ) : oauthStatusLoading && !oauthStatus ? (
                    <p className="text-sm text-muted-foreground">{t('accounts.oauth.loading')}</p>
                  ) : oauthStatus ? (
                    <div className="grid grid-cols-1 gap-4 xl:grid-cols-2">
                      <DetailSection
                        title={t('accounts.details.sections.subscription', {
                          defaultValue: 'Subscription',
                        })}
                      >
                        <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
                          <DetailField label={fieldLabel('chatgptPlanType', 'ChatGPT Plan Type')}>
                            <Badge variant="outline" title={oauthStatus.chatgpt_plan_type ?? undefined}>
                              {getPlanLabel(oauthStatus.chatgpt_plan_type, t)}
                            </Badge>
                          </DetailField>
                          <DetailField
                            label={fieldLabel('sourceType', 'Source Type')}
                            title={oauthStatus.source_type ?? undefined}
                          >
                            {sourceTypeLabel ?? '-'}
                          </DetailField>
                          <DetailField label={fieldLabel('tokenExpiresAt', 'Token Expires At')}>
                            {formatOptionalDateTime(oauthStatus.token_expires_at)}
                          </DetailField>
                          <DetailField label={fieldLabel('effectiveEnabled', 'Effective Enabled')}>
                            <Badge variant={oauthStatus.effective_enabled ? 'success' : 'warning'}>
                              {oauthStatus.effective_enabled ? t('common.yes') : t('common.no')}
                            </Badge>
                          </DetailField>
                        </div>
                      </DetailSection>

                      <DetailSection
                        title={t('accounts.details.sections.runtimeHealth', {
                          defaultValue: 'Runtime Health',
                        })}
                      >
                        <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
                          <DetailField label={fieldLabel('poolState', 'Runtime Pool')}>
                            <Badge variant={getPoolStateBadgeVariant(oauthStatus.pool_state)}>
                              {getPoolStateLabel(oauthStatus.pool_state, t)}
                            </Badge>
                          </DetailField>
                          <DetailField
                            label={fieldLabel('refreshCredentialState', 'Refresh Credential State')}
                          >
                            <Badge variant={oauthStatus.has_refresh_credential ? 'success' : 'secondary'}>
                              {getRefreshCredentialStateLabel(
                                oauthStatus.refresh_credential_state,
                                t,
                              )}
                            </Badge>
                          </DetailField>
                          <DetailField label={fieldLabel('quarantineReason', 'Quarantine Reason')}>
                            {oauthStatus.quarantine_reason
                              ? localizeOAuthErrorCodeDisplay(t, oauthStatus.quarantine_reason).label
                              : '-'}
                          </DetailField>
                          <DetailField label={fieldLabel('quarantineUntil', 'Quarantine Until')}>
                            {formatOptionalDateTime(oauthStatus.quarantine_until)}
                          </DetailField>
                          <DetailField label={fieldLabel('pendingPurgeReason', 'Pending Purge Reason')}>
                            {oauthStatus.pending_purge_reason
                              ? localizeOAuthErrorCodeDisplay(t, oauthStatus.pending_purge_reason).label
                              : '-'}
                          </DetailField>
                          <DetailField label={fieldLabel('pendingPurgeAt', 'Pending Purge At')}>
                            {formatOptionalDateTime(oauthStatus.pending_purge_at)}
                          </DetailField>
                          <DetailField label={fieldLabel('hasRefreshCredential', 'Has Refresh Credential')}>
                            <Badge variant={oauthStatus.has_refresh_credential ? 'success' : 'secondary'}>
                              {oauthStatus.has_refresh_credential ? t('common.yes') : t('common.no')}
                            </Badge>
                          </DetailField>
                          <DetailField label={fieldLabel('hasAccessTokenFallback', 'Has Access Token Fallback')}>
                            <Badge variant={oauthStatus.has_access_token_fallback ? 'info' : 'secondary'}>
                              {oauthStatus.has_access_token_fallback ? t('common.yes') : t('common.no')}
                            </Badge>
                          </DetailField>
                        </div>
                      </DetailSection>

                      <DetailSection
                        title={t('accounts.details.sections.refresh', {
                          defaultValue: 'Refresh State',
                        })}
                      >
                        <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
                          <DetailField
                            label={fieldLabel('authProvider', 'Auth Provider')}
                            title={oauthStatus.auth_provider}
                          >
                            {getAuthProviderLabel(oauthStatus.auth_provider, t)}
                          </DetailField>
                          <DetailField
                            label={fieldLabel('credentialKind', 'Credential Kind')}
                            title={oauthStatus.credential_kind ?? undefined}
                          >
                            {getCredentialKindLabel(oauthStatus.credential_kind, t)}
                          </DetailField>
                          <DetailField label={fieldLabel('lastRefreshStatus', 'Last Refresh Status')}>
                            {renderRefreshStatusBadge(
                              oauthStatus.last_refresh_status,
                              getRefreshStatusLabel(oauthStatus.last_refresh_status, t),
                            )}
                          </DetailField>
                          <DetailField label={fieldLabel('lastRefreshAt', 'Last Refresh At')}>
                            {formatOptionalDateTime(oauthStatus.last_refresh_at)}
                          </DetailField>
                          <DetailField
                            label={fieldLabel('refreshReusedDetected', 'Refresh Reused Detected')}
                          >
                            <Badge
                              variant={oauthStatus.refresh_reused_detected ? 'warning' : 'secondary'}
                            >
                              {oauthStatus.refresh_reused_detected
                                ? t('common.yes')
                                : t('common.no')}
                            </Badge>
                          </DetailField>
                          <DetailField
                            label={fieldLabel('tokenVersion', 'Token Version')}
                            title={
                              typeof oauthStatus.token_version === 'number'
                                ? String(oauthStatus.token_version)
                                : undefined
                            }
                          >
                            {oauthStatus.token_version?.toString() ?? '-'}
                          </DetailField>
                          <DetailField
                            label={fieldLabel('tokenFamilyId', 'Token Family ID')}
                            mono
                            title={oauthStatus.token_family_id ?? undefined}
                            containerClassName="md:col-span-2"
                          >
                            {oauthStatus.token_family_id ?? '-'}
                          </DetailField>
                          <DetailField
                            label={fieldLabel('lastRefreshErrorCode', 'Last Refresh Error Code')}
                            title={refreshErrorDisplay.tooltip}
                          >
                            {oauthStatus.last_refresh_error_code ? refreshErrorDisplay.label : '-'}
                          </DetailField>
                          <DetailField
                            label={fieldLabel('lastRefreshError', 'Last Refresh Error')}
                            scrollable
                            title={
                              import.meta.env.DEV
                                ? oauthStatus.last_refresh_error ?? undefined
                                : undefined
                            }
                            containerClassName="md:col-span-2"
                            className="max-h-28"
                          >
                            {oauthStatus.last_refresh_error ? refreshErrorDisplay.label : '-'}
                          </DetailField>
                        </div>
                      </DetailSection>

                      <DetailSection
                        title={t('accounts.details.sections.cache', {
                          defaultValue: 'Rate Limit Cache',
                        })}
                        className="xl:col-span-2"
                      >
                        <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
                          <DetailField label={fieldLabel('rateLimitsFetchedAt', 'Rate Limits Fetched At')}>
                            {formatOptionalDateTime(oauthStatus.rate_limits_fetched_at)}
                          </DetailField>
                          <DetailField label={fieldLabel('rateLimitsExpiresAt', 'Rate Limits Expires At')}>
                            {formatOptionalDateTime(oauthStatus.rate_limits_expires_at)}
                          </DetailField>
                          <DetailField
                            label={fieldLabel('rateLimitsLastErrorCode', 'Rate Limits Last Error Code')}
                            title={rateLimitErrorDisplay.tooltip}
                          >
                            {oauthStatus.rate_limits_last_error_code ? rateLimitErrorDisplay.label : '-'}
                          </DetailField>
                          <DetailField
                            label={fieldLabel('rateLimitsLastError', 'Rate Limits Last Error')}
                            scrollable
                            title={
                              import.meta.env.DEV
                                ? oauthStatus.rate_limits_last_error ?? undefined
                                : undefined
                            }
                            containerClassName="md:col-span-2"
                            className="max-h-28"
                          >
                            {oauthStatus.rate_limits_last_error ? rateLimitErrorDisplay.label : '-'}
                          </DetailField>
                        </div>
                      </DetailSection>
                    </div>
                  ) : (
                    <p className="text-sm text-muted-foreground">
                      {t('accounts.details.noOauthStatus', {
                        defaultValue: 'No OAuth status data yet.',
                      })}
                    </p>
                  )}
                </section>
              ) : null}

              {detailTab === 'limits' ? (
                <section
                  id="account-detail-panel-limits"
                  role="tabpanel"
                  tabIndex={0}
                  aria-labelledby="account-detail-tab-limits"
                  className="space-y-4"
                >
                  <h3 className="text-base font-semibold">
                    {t('accounts.details.limitsTitle', { defaultValue: 'Rate Limits' })}
                  </h3>
                  {!isSessionAccount ? (
                    <p className="text-sm text-muted-foreground">
                      {t('accounts.details.oauthNotApplicable', {
                        defaultValue: 'OAuth details are not available for this account type.',
                      })}
                    </p>
                  ) : rateLimitDisplays.length === 0 ? (
                    <p className="text-sm text-muted-foreground">
                      {t('accounts.rateLimits.unavailable')}
                    </p>
                  ) : (
                    <DetailSection
                      title={t('accounts.details.limitsTitle', { defaultValue: 'Rate Limits' })}
                    >
                      <div className="space-y-3">
                        {rateLimitDisplays.map((item) => {
                          const remaining = clampPercent(item.remainingPercent)
                          return (
                            <div
                              key={item.bucket}
                              className="rounded-lg border border-border/60 bg-muted/20 p-3"
                            >
                              <div className="flex items-center justify-between gap-2 text-sm">
                                <span className="font-medium">{bucketLabel(item.bucket, t)}</span>
                                <span className="tabular-nums text-muted-foreground">
                                  {t('accounts.rateLimits.remainingPrefix', {
                                    defaultValue: 'Remaining',
                                  })}{' '}
                                  {remaining.toFixed(1)}%
                                </span>
                              </div>
                              <div className="mt-2 h-2 overflow-hidden rounded-full bg-muted-foreground/20">
                                <div
                                  className={cn(
                                    'h-full transition-[width] duration-300',
                                    bucketBarClass(item.bucket),
                                  )}
                                  style={{ width: `${remaining}%` }}
                                />
                              </div>
                              <p className="mt-2 text-xs text-muted-foreground">
                                {formatRateLimitResetText({
                                  resetsAt: item.resetsAt,
                                  locale,
                                  t,
                                })}
                              </p>
                            </div>
                          )
                        })}
                      </div>
                    </DetailSection>
                  )}
                </section>
              ) : null}

              {detailTab === 'raw' ? (
                <section
                  id="account-detail-panel-raw"
                  role="tabpanel"
                  tabIndex={0}
                  aria-labelledby="account-detail-tab-raw"
                  className="space-y-4"
                >
                  <h3 className="text-base font-semibold">
                    {t('accounts.details.rawTitle', { defaultValue: 'Raw Payload' })}
                  </h3>
                  <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
                    <DetailSection title={fieldLabel('rawAccount', 'Account Payload')}>
                      <pre className="max-h-[28rem] overflow-auto rounded-lg border bg-muted/20 p-3 text-xs leading-relaxed">
                        {JSON.stringify(account, null, 2)}
                      </pre>
                    </DetailSection>
                    <DetailSection title={fieldLabel('rawOauthStatus', 'OAuth Status Payload')}>
                      <pre className="max-h-[28rem] overflow-auto rounded-lg border bg-muted/20 p-3 text-xs leading-relaxed">
                        {JSON.stringify(oauthStatus ?? null, null, 2)}
                      </pre>
                    </DetailSection>
                  </div>
                </section>
              ) : null}
            </div>
          </div>
        ) : null}
      </DialogContent>
    </Dialog>
  )
}
