import { useMemo } from 'react'
import type { ColumnDef } from '@tanstack/react-table'
import {
  Eye,
  MoreHorizontal,
  Power,
  PowerOff,
  RotateCcw,
  Trash2,
} from 'lucide-react'
import { useTranslation } from 'react-i18next'

import type { OAuthAccountStatusResponse, UpstreamAccount } from '@/api/accounts'
import { localizeOAuthErrorCodeDisplay } from '@/api/errorI18n'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Skeleton } from '@/components/ui/skeleton'
import { cn } from '@/lib/utils'
import { formatRelativeTime } from '@/lib/time'

import { RateLimitCell } from './rate-limit-cell'
import type { ToggleAccountPayload } from './types'
import {
  getCredentialKindShortLabel,
  getModeLabel,
  getPlanLabel,
  getRefreshStatusLabel,
  isSessionMode,
  normalizePlanValue,
  rateLimitSortValue,
  resolveCredentialKindShort,
  statusSortValue,
} from './utils'

type UseAccountsColumnsParams = {
  oauthStatusMap: Map<string, OAuthAccountStatusResponse>
  isOAuthStatusRefreshing: boolean
  tableFilteredAccountIds: string[]
  selectedAccountIdSet: Set<string>
  onToggleSelectAllFiltered: (checked: boolean) => void
  onToggleAccountSelection: (accountId: string, checked: boolean) => void
  onOpenDetailAccount: (account: UpstreamAccount) => void
  onRefreshAccount: (accountId: string) => void
  onToggleAccountEnabled: (payload: ToggleAccountPayload) => void
  onDeleteAccount: (account: UpstreamAccount) => Promise<void> | void
  onPauseFamily: (accountId: string) => void
  onResumeFamily: (accountId: string) => void
  isRefreshPending: boolean
  isTogglePending: boolean
  isDeletePending: boolean
  isPauseFamilyPending: boolean
  isResumeFamilyPending: boolean
}

export function useAccountsColumns({
  oauthStatusMap,
  isOAuthStatusRefreshing,
  tableFilteredAccountIds,
  selectedAccountIdSet,
  onToggleSelectAllFiltered,
  onToggleAccountSelection,
  onOpenDetailAccount,
  onRefreshAccount,
  onToggleAccountEnabled,
  onDeleteAccount,
  onPauseFamily,
  onResumeFamily,
  isRefreshPending,
  isTogglePending,
  isDeletePending,
  isPauseFamilyPending,
  isResumeFamilyPending,
}: UseAccountsColumnsParams): ColumnDef<UpstreamAccount>[] {
  const { t, i18n } = useTranslation()
  const locale = i18n.resolvedLanguage ?? i18n.language

  return useMemo<ColumnDef<UpstreamAccount>[]>(() => {
    return [
      {
        id: 'select',
        header: () => {
          const allSelected =
            tableFilteredAccountIds.length > 0
            && tableFilteredAccountIds.every((id) => selectedAccountIdSet.has(id))
          const hasSomeSelected =
            !allSelected && tableFilteredAccountIds.some((id) => selectedAccountIdSet.has(id))

          return (
            <Checkbox
              checked={allSelected ? true : hasSomeSelected ? 'indeterminate' : false}
              onCheckedChange={(value) => onToggleSelectAllFiltered(Boolean(value))}
              aria-label={t('accounts.actions.selectAll', {
                defaultValue: 'Select all filtered results',
              })}
            />
          )
        },
        enableSorting: false,
        cell: ({ row }) => (
          <Checkbox
            checked={selectedAccountIdSet.has(row.original.id)}
            onCheckedChange={(value) => onToggleAccountSelection(row.original.id, Boolean(value))}
            aria-label={t('accounts.actions.selectOne', {
              label: row.original.label,
              defaultValue: 'Select account {{label}}',
            })}
          />
        ),
      },
      {
        id: 'identity',
        accessorFn: (row) => {
          const status = oauthStatusMap.get(row.id)
          return (status?.email ?? row.label).toLowerCase()
        },
        header: t('accounts.columns.account', { defaultValue: 'Account' }),
        cell: ({ row }) => {
          const status = oauthStatusMap.get(row.original.id)
          const primaryIdentity = status?.email?.trim() || row.original.label
          const secondaryIdentity =
            status?.email?.trim() && row.original.label.trim() !== primaryIdentity
              ? row.original.label
              : null

          return (
            <div className="min-w-[220px]">
              <div className="font-medium text-foreground truncate" title={primaryIdentity}>
                {primaryIdentity}
              </div>
              {secondaryIdentity ? (
                <div
                  className="mt-0.5 truncate text-xs text-muted-foreground"
                  title={secondaryIdentity}
                >
                  {secondaryIdentity}
                </div>
              ) : null}
            </div>
          )
        },
      },
      {
        accessorKey: 'mode',
        header: t('accounts.columns.provider'),
        cell: ({ row }) => {
          const mode = row.original.mode
          const modeLabel = getModeLabel(mode, t)
          const isOpenAiFamily =
            mode.includes('openai') || mode.includes('chat_gpt') || mode.includes('codex')
          return (
            <div className="flex min-w-[112px] max-w-[148px] items-center gap-2">
              <div
                className={cn(
                  'h-2 w-2 rounded-full',
                  isOpenAiFamily ? 'bg-success' : 'bg-purple-500',
                )}
              />
              <span className="min-w-0 flex-1 truncate" title={modeLabel}>
                {modeLabel}
              </span>
            </div>
          )
        },
      },
      {
        id: 'oauthStatus',
        accessorFn: (row) => statusSortValue(oauthStatusMap.get(row.id)),
        header: t('accounts.columns.loginStatus'),
        cell: ({ row }) => {
          const cellClass = 'min-w-[98px] max-w-[114px] h-[40px]'
          const isSession = isSessionMode(row.original.mode)
          if (!isSession) {
            return (
              <div className={cn(cellClass, 'flex items-center')}>
                <span className="text-xs text-muted-foreground">
                  {t('accounts.oauth.notApplicable')}
                </span>
              </div>
            )
          }
          const status = oauthStatusMap.get(row.original.id)
          if (isOAuthStatusRefreshing) {
            return (
              <div className={cn(cellClass, 'flex items-center')}>
                <Skeleton className="h-5 w-14" />
              </div>
            )
          }
          if (!status) {
            return (
              <div className={cn(cellClass, 'flex items-center')}>
                <span className="text-xs text-muted-foreground">{t('accounts.oauth.loading')}</span>
              </div>
            )
          }

          if (status.last_refresh_status === 'failed') {
            const statusLabel = getRefreshStatusLabel(status.last_refresh_status, t)
            const errorDisplay = localizeOAuthErrorCodeDisplay(t, status.last_refresh_error_code)
            return (
              <div className={cn(cellClass, 'flex flex-col items-start justify-center gap-0.5')}>
                <Badge variant="destructive" className="w-fit max-w-full truncate" title={statusLabel}>
                  {statusLabel}
                </Badge>
                <div className="max-w-full truncate text-[11px] text-muted-foreground" title={errorDisplay.tooltip}>
                  {errorDisplay.label}
                </div>
              </div>
            )
          }

          const statusLabel = getRefreshStatusLabel(status.last_refresh_status, t)
          return (
            <div className={cn(cellClass, 'flex items-center')}>
              <Badge variant="success" className="w-fit max-w-full truncate" title={statusLabel}>
                {statusLabel}
              </Badge>
            </div>
          )
        },
      },
      {
        id: 'credentialKind',
        accessorFn: (row) => {
          const status = oauthStatusMap.get(row.id)
          const kind = resolveCredentialKindShort(status?.credential_kind)
          if (kind === 'rt') return 3
          if (kind === 'at') return 2
          return 1
        },
        header: t('accounts.columns.credentialType', { defaultValue: 'Credential Type' }),
        cell: ({ row }) => {
          const isSession = isSessionMode(row.original.mode)
          if (!isSession) {
            return <span className="text-xs text-muted-foreground">{t('accounts.oauth.notApplicable')}</span>
          }
          if (isOAuthStatusRefreshing) {
            return <Skeleton className="h-5 w-14" />
          }
          const status = oauthStatusMap.get(row.original.id)
          if (!status) {
            return <span className="text-xs text-muted-foreground">{t('accounts.oauth.loading')}</span>
          }
          const isRefreshToken = status.credential_kind === 'refresh_rotatable'
          return (
            <Badge variant={isRefreshToken ? 'success' : 'secondary'}>
              {getCredentialKindShortLabel(status.credential_kind, t)}
            </Badge>
          )
        },
      },
      {
        id: 'planType',
        accessorFn: (row) => {
          const status = oauthStatusMap.get(row.id)
          const value = normalizePlanValue(status?.chatgpt_plan_type)
          if (value === '__unknown__') {
            return ''
          }
          return value
        },
        header: t('accounts.columns.plan', { defaultValue: 'Plan' }),
        cell: ({ row }) => {
          const isSession = isSessionMode(row.original.mode)
          if (!isSession) {
            return <span className="text-xs text-muted-foreground">{t('accounts.oauth.notApplicable')}</span>
          }
          if (isOAuthStatusRefreshing) {
            return <Skeleton className="h-5 w-20" />
          }
          const status = oauthStatusMap.get(row.original.id)
          if (!status) {
            return <span className="text-xs text-muted-foreground">{t('accounts.oauth.loading')}</span>
          }
          return <Badge variant="outline">{getPlanLabel(status.chatgpt_plan_type, t)}</Badge>
        },
      },
      {
        id: 'health',
        accessorFn: (row) => (oauthStatusMap.get(row.id)?.effective_enabled ?? row.enabled ? 1 : 0),
        header: t('accounts.columns.health'),
        cell: ({ row }) => {
          if (isSessionMode(row.original.mode) && isOAuthStatusRefreshing) {
            return <Skeleton className="h-5 w-16" />
          }
          const status = oauthStatusMap.get(row.original.id)
          const enabled = status?.effective_enabled ?? row.original.enabled
          return (
            <Badge variant={enabled ? 'success' : 'warning'}>
              {enabled ? t('accounts.status.active') : t('accounts.status.disabled')}
            </Badge>
          )
        },
      },
      {
        id: 'rateLimit',
        accessorFn: (row) => rateLimitSortValue(oauthStatusMap.get(row.id)),
        header: t('accounts.columns.rateLimit'),
        cell: ({ row }) => (
          <RateLimitCell
            status={oauthStatusMap.get(row.original.id)}
            locale={locale}
            refreshing={isSessionMode(row.original.mode) && isOAuthStatusRefreshing}
          />
        ),
      },
      {
        accessorKey: 'created_at',
        header: t('accounts.columns.added'),
        cell: ({ row }) => (
          <span className="text-sm text-muted-foreground">
            {formatRelativeTime(row.original.created_at, locale, true)}
          </span>
        ),
      },
      {
        id: 'actions',
        enableSorting: false,
        cell: ({ row }) => {
          const account = row.original
          const status = oauthStatusMap.get(account.id)
          const isSession = isSessionMode(account.mode)
          const canRefresh = isSession && status?.auth_provider === 'oauth_refresh_token'
          const canFamilyAction =
            canRefresh && status?.credential_kind !== 'one_time_access_token'
          const accountEnabled = account.enabled

          return (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" className="h-8 w-8 p-0" aria-label={t('common.openMenu')}>
                  <MoreHorizontal className="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-[220px]">
                <DropdownMenuLabel>{t('accounts.columns.actions')}</DropdownMenuLabel>
                <DropdownMenuItem
                  className="cursor-pointer"
                  onClick={() => onOpenDetailAccount(account)}
                >
                  <Eye className="mr-2 h-4 w-4 text-muted-foreground" />
                  {t('accounts.actions.viewDetails', { defaultValue: 'View Details' })}
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                {canRefresh ? (
                  <DropdownMenuItem
                    className="cursor-pointer"
                    onClick={() => onRefreshAccount(account.id)}
                    disabled={isRefreshPending}
                  >
                    <RotateCcw className="mr-2 h-4 w-4 text-muted-foreground" />
                    {t('accounts.actions.refreshLogin')}
                  </DropdownMenuItem>
                ) : null}
                <DropdownMenuItem
                  className="cursor-pointer"
                  onClick={() =>
                    onToggleAccountEnabled({
                      accountId: account.id,
                      enabled: !accountEnabled,
                    })
                  }
                  disabled={isTogglePending}
                >
                  {accountEnabled ? (
                    <PowerOff className="mr-2 h-4 w-4 text-muted-foreground" />
                  ) : (
                    <Power className="mr-2 h-4 w-4 text-muted-foreground" />
                  )}
                  {accountEnabled
                    ? t('accounts.actions.disableAccount', { defaultValue: 'Disable Account' })
                    : t('accounts.actions.enableAccount', { defaultValue: 'Enable Account' })}
                </DropdownMenuItem>
                <DropdownMenuItem
                  className="cursor-pointer text-destructive focus:bg-destructive/10"
                  onClick={() => void onDeleteAccount(account)}
                  disabled={isDeletePending}
                >
                  <Trash2 className="mr-2 h-4 w-4 text-destructive" />
                  {t('accounts.actions.delete', { defaultValue: 'Delete Account' })}
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                {canFamilyAction ? (
                  <>
                    <DropdownMenuItem
                      className="cursor-pointer text-warning-foreground focus:bg-warning-muted"
                      onClick={() => onPauseFamily(account.id)}
                      disabled={isPauseFamilyPending}
                    >
                      <PowerOff className="mr-2 h-4 w-4 text-warning-foreground" />
                      {t('accounts.actions.pauseGroup')}
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      className="cursor-pointer text-success-foreground focus:bg-success-muted"
                      onClick={() => onResumeFamily(account.id)}
                      disabled={isResumeFamilyPending}
                    >
                      <Power className="mr-2 h-4 w-4 text-success-foreground" />
                      {t('accounts.actions.resumeGroup')}
                    </DropdownMenuItem>
                  </>
                ) : (
                  <DropdownMenuItem className="cursor-pointer" disabled>
                    {isSession
                      ? t('accounts.actions.oneTimeNoGroupAction')
                      : t('accounts.actions.apiKeyNoGroupAction')}
                  </DropdownMenuItem>
                )}
              </DropdownMenuContent>
            </DropdownMenu>
          )
        },
      },
    ]
  }, [
    isDeletePending,
    isOAuthStatusRefreshing,
    isPauseFamilyPending,
    isRefreshPending,
    isResumeFamilyPending,
    isTogglePending,
    locale,
    oauthStatusMap,
    onDeleteAccount,
    onOpenDetailAccount,
    onPauseFamily,
    onRefreshAccount,
    onResumeFamily,
    onToggleAccountEnabled,
    onToggleAccountSelection,
    onToggleSelectAllFiltered,
    selectedAccountIdSet,
    tableFilteredAccountIds,
    t,
  ])
}
