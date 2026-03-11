import { useCallback, useEffect, useMemo, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import type { TFunction } from 'i18next'
import { motion } from 'framer-motion'
import { ChevronDown, Download, Plus, RefreshCw } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router-dom'

import {
  accountsApi,
  type OAuthAccountStatusResponse,
  type OAuthRateLimitRefreshJobSummary,
  type UpstreamAccount,
} from '@/api/accounts'
import { extractApiErrorStatus } from '@/api/client'
import { localizeApiErrorDisplay, localizeOAuthErrorCodeDisplay } from '@/api/errorI18n'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { useConfirmDialog } from '@/components/ui/confirm-dialog'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { LoadingOverlay } from '@/components/ui/loading-overlay'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { StandardDataTable } from '@/components/ui/standard-data-table'
import { AccountDetailDialog } from '@/features/accounts/account-detail-dialog'
import { executeAccountBatch } from '@/features/accounts/batch-executor'
import {
  EMPTY_ACCOUNTS,
  EMPTY_OAUTH_STATUSES,
  PLAN_UNKNOWN_VALUE,
  type AccountBatchAction,
  type AccountDetailTab,
  type CredentialFilter,
  type ModeFilter,
  type PlanFilter,
  type StatusFilter,
  type ToggleAccountPayload,
} from '@/features/accounts/types'
import { useAccountsColumns } from '@/features/accounts/use-accounts-columns'
import {
  addRecentImportJobId,
  extractRateLimitDisplays,
  isRateLimitRefreshJobTerminal,
  isSessionMode,
  matchesAccountSearch,
  normalizePlanValue,
  resolveCredentialKindShort,
  sortRateLimitDisplays,
} from '@/features/accounts/utils'
import { notify } from '@/lib/notification'
import { cn } from '@/lib/utils'

const BATCH_CONCURRENCY: Record<AccountBatchAction, number> = {
  enable: 10,
  disable: 10,
  delete: 6,
  refreshLogin: 4,
  pauseFamily: 6,
  resumeFamily: 6,
}

function localizeRateLimitRefreshJobStatus(
  t: TFunction,
  status: string | null | undefined,
): string {
  const normalized = (status ?? '').trim().toLowerCase()
  switch (normalized) {
    case 'queued':
      return t('accounts.rateLimitRefreshJobStatus.queued', { defaultValue: 'Queued' })
    case 'running':
      return t('accounts.rateLimitRefreshJobStatus.running', { defaultValue: 'Running' })
    case 'completed':
      return t('accounts.rateLimitRefreshJobStatus.completed', { defaultValue: 'Completed' })
    case 'failed':
      return t('accounts.rateLimitRefreshJobStatus.failed', { defaultValue: 'Failed' })
    case 'cancelled':
      return t('accounts.rateLimitRefreshJobStatus.cancelled', { defaultValue: 'Cancelled' })
    default:
      return t('accounts.rateLimitRefreshJobStatus.unknown', { defaultValue: 'Unknown' })
  }
}

export default function Accounts() {
  const { t, i18n } = useTranslation()
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const { confirm, confirmDialog } = useConfirmDialog()

  const [statusFilter, setStatusFilter] = useState<StatusFilter>('all')
  const [modeFilter, setModeFilter] = useState<ModeFilter>('all')
  const [credentialFilter, setCredentialFilter] = useState<CredentialFilter>('all')
  const [planFilter, setPlanFilter] = useState<PlanFilter>('all')
  const [selectedAccountIds, setSelectedAccountIds] = useState<string[]>([])
  const [tableFilteredAccounts, setTableFilteredAccounts] = useState<UpstreamAccount[]>([])
  const [isBatchOperating, setIsBatchOperating] = useState(false)
  const [isManualRefreshing, setIsManualRefreshing] = useState(false)
  const [rateLimitRefreshJob, setRateLimitRefreshJob] =
    useState<OAuthRateLimitRefreshJobSummary | null>(null)
  const [detailAccount, setDetailAccount] = useState<UpstreamAccount | null>(null)
  const [detailTab, setDetailTab] = useState<AccountDetailTab>('profile')

  const {
    data: accountsRaw = EMPTY_ACCOUNTS,
    isLoading,
    refetch: refetchAccounts,
  } = useQuery({
    queryKey: ['upstreamAccounts'],
    queryFn: accountsApi.listAccounts,
    staleTime: 180000,
    refetchInterval: 180000,
    refetchOnWindowFocus: 'always',
  })

  const oauthAccountIds = useMemo(
    () => accountsRaw.filter((item) => isSessionMode(item.mode)).map((item) => item.id),
    [accountsRaw],
  )

  const {
    data: oauthStatusesRaw = EMPTY_OAUTH_STATUSES,
    isLoading: isOauthStatusesLoading,
    isFetching: isOauthStatusesFetching,
    refetch: refetchOAuthStatuses,
  } = useQuery({
    queryKey: ['oauthStatuses', oauthAccountIds],
    queryFn: () => accountsApi.listOAuthStatuses(oauthAccountIds),
    enabled: oauthAccountIds.length > 0,
    staleTime: 180000,
    refetchInterval: 180000,
    refetchOnWindowFocus: 'always',
    retry: false,
  })

  const isOAuthStatusRefreshing =
    oauthAccountIds.length > 0 && (isOauthStatusesLoading || isOauthStatusesFetching)

  const oauthStatusMap = useMemo(() => {
    const map = new Map<string, OAuthAccountStatusResponse>()
    oauthStatusesRaw.forEach((status) => {
      map.set(status.account_id, status)
    })
    return map
  }, [oauthStatusesRaw])

  const detailIsSessionAccount = Boolean(detailAccount && isSessionMode(detailAccount.mode))

  const detailAccountStatusFromMap = useMemo(() => {
    if (!detailAccount) {
      return undefined
    }
    return oauthStatusMap.get(detailAccount.id)
  }, [detailAccount, oauthStatusMap])

  const detailOAuthStatusQuery = useQuery({
    queryKey: ['oauthStatusDetail', detailAccount?.id],
    enabled: Boolean(detailAccount?.id && detailIsSessionAccount),
    queryFn: () => accountsApi.getOAuthStatus(detailAccount!.id),
    staleTime: 30000,
    retry: false,
  })

  const detailOAuthStatus = detailOAuthStatusQuery.data ?? detailAccountStatusFromMap

  const detailRateLimitDisplays = useMemo(
    () => sortRateLimitDisplays(extractRateLimitDisplays(detailOAuthStatus)),
    [detailOAuthStatus],
  )

  const accountById = useMemo(() => {
    const map = new Map<string, UpstreamAccount>()
    accountsRaw.forEach((account) => {
      map.set(account.id, account)
    })
    return map
  }, [accountsRaw])

  const invalidateAccountQueries = useCallback(() => {
    queryClient.invalidateQueries({ queryKey: ['oauthStatuses'] })
    queryClient.invalidateQueries({ queryKey: ['upstreamAccounts'] })
  }, [queryClient])

  const resolveErrorLabel = useCallback(
    (error: unknown, fallback: string) => localizeApiErrorDisplay(t, error, fallback).label,
    [t],
  )

  const handleRefreshAccounts = useCallback(async () => {
    if (isManualRefreshing) {
      return
    }

    setIsManualRefreshing(true)
    setRateLimitRefreshJob(null)
    try {
      const created = await accountsApi.createRateLimitRefreshJob()
      setRateLimitRefreshJob(created)

      let latest = created
      const maxPollCount = 1200
      let pollCount = 0
      while (!isRateLimitRefreshJobTerminal(latest.status)) {
        await new Promise((resolve) => setTimeout(resolve, 2000))
        latest = await accountsApi.getRateLimitRefreshJob(latest.job_id)
        setRateLimitRefreshJob(latest)
        pollCount += 1
        if (pollCount >= maxPollCount) {
          throw new Error(
            t('accounts.messages.rateLimitPollingTimeout', {
              defaultValue: 'Rate-limit refresh job polling timed out.',
            }),
          )
        }
      }

	      if (latest.status === 'failed' || latest.status === 'cancelled') {
	        const errorSummary = (latest.error_summary ?? [])
	          .slice(0, 3)
	          .map((item) => `${localizeOAuthErrorCodeDisplay(t, item.error_code).label}(${item.count})`)
	          .join(', ')
	        if (errorSummary) {
	          throw new Error(
	            t('accounts.messages.rateLimitRefreshFailedSummary', {
              defaultValue: 'Rate-limit refresh job failed: {{summary}}',
              summary: errorSummary,
            }),
          )
        }
        throw new Error(
          t('accounts.messages.rateLimitRefreshFailedStatus', {
            defaultValue: 'Rate-limit refresh job failed, status={{status}}',
            status: localizeRateLimitRefreshJobStatus(t, latest.status),
          }),
        )
      }

      await refetchAccounts({ throwOnError: true })
      if (oauthAccountIds.length > 0) {
        await refetchOAuthStatuses({ throwOnError: true })
      }
      notify({
        variant: 'success',
        title: t('accounts.messages.refreshListSuccess', { defaultValue: 'Refresh List Success' }),
        description: t('accounts.messages.refreshJobSummary', {
          defaultValue: 'Job ID: {{jobId}} · {{processed}}/{{total}}',
          jobId: latest.job_id,
          processed: latest.processed,
          total: latest.total,
        }),
      })
    } catch (error) {
      notify({
        variant: 'error',
        title: t('accounts.messages.refreshListFailed', { defaultValue: 'Refresh List Failed' }),
        description: resolveErrorLabel(
          error,
          t('accounts.messages.requestFailed', { defaultValue: 'Request failed. Please try again later.' }),
        ),
      })
    } finally {
      setIsManualRefreshing(false)
    }
	  }, [
	    isManualRefreshing,
	    oauthAccountIds.length,
	    refetchAccounts,
	    refetchOAuthStatuses,
	    resolveErrorLabel,
	    t,
	  ])

  const performSetEnabled = useCallback(
    async (accountId: string, enabled: boolean) => {
      try {
        return await accountsApi.setEnabled(accountId, enabled)
      } catch (error) {
        const statusCode = extractApiErrorStatus(error)
        const account = accountById.get(accountId)
        const oauthStatus = oauthStatusMap.get(accountId)
        const canFamilyFallback =
          statusCode === 404
          && account !== undefined
          && isSessionMode(account.mode)
          && oauthStatus?.auth_provider === 'oauth_refresh_token'

        if (canFamilyFallback) {
          return enabled
            ? accountsApi.enableFamily(accountId)
            : accountsApi.disableFamily(accountId)
        }

        if (statusCode === 404) {
          throw new Error(
            t('accounts.messages.toggleUnsupported', {
              defaultValue: 'Current backend does not support account enable/disable. Please upgrade control-plane.',
            }),
            { cause: error },
          )
        }

        throw error
      }
    },
    [accountById, oauthStatusMap, t],
  )

  const refreshMutation = useMutation({
    mutationFn: (accountId: string) => accountsApi.refreshOAuthJob(accountId),
    onSuccess: (job) => {
      addRecentImportJobId(job.job_id)
      invalidateAccountQueries()
      notify({
        variant: 'success',
        title: t('accounts.messages.refreshSuccess', { defaultValue: 'Login refreshed successfully' }),
        description: t('accounts.messages.refreshJobId', {
          defaultValue: 'Job ID: {{jobId}}',
          jobId: job.job_id,
        }),
      })
    },
    onError: (error) => {
      notify({
        variant: 'error',
        title: t('accounts.messages.refreshFailed', { defaultValue: 'Login refresh failed' }),
        description: resolveErrorLabel(
          error,
          t('accounts.messages.requestFailed', { defaultValue: 'Request failed. Please try again later.' }),
        ),
      })
    },
  })

  const toggleEnabledMutation = useMutation({
    mutationFn: ({ accountId, enabled }: ToggleAccountPayload) =>
      performSetEnabled(accountId, enabled),
    onSuccess: (_, variables) => {
      invalidateAccountQueries()
      notify({
        variant: 'success',
        title: variables.enabled
          ? t('accounts.messages.enableSuccess', { defaultValue: 'Account enabled' })
          : t('accounts.messages.disableSuccess', { defaultValue: 'Account disabled' }),
      })
    },
    onError: (error, variables) => {
      notify({
        variant: 'error',
        title: variables.enabled
          ? t('accounts.messages.enableFailed', { defaultValue: 'Failed to enable account' })
          : t('accounts.messages.disableFailed', { defaultValue: 'Failed to disable account' }),
        description: resolveErrorLabel(
          error,
          t('accounts.messages.requestFailed', { defaultValue: 'Request failed. Please try again later.' }),
        ),
      })
    },
  })

  const deleteAccountMutation = useMutation({
    mutationFn: (accountId: string) => accountsApi.deleteAccount(accountId),
    onSuccess: () => {
      invalidateAccountQueries()
      notify({
        variant: 'success',
        title: t('accounts.messages.deleteSuccess', { defaultValue: 'Account deleted' }),
      })
    },
    onError: (error) => {
      notify({
        variant: 'error',
        title: t('accounts.messages.deleteFailed', { defaultValue: 'Failed to delete account' }),
        description: resolveErrorLabel(
          error,
          t('accounts.messages.requestFailed', { defaultValue: 'Request failed. Please try again later.' }),
        ),
      })
    },
  })

  const disableFamilyMutation = useMutation({
    mutationFn: (accountId: string) => accountsApi.disableFamily(accountId),
    onSuccess: () => {
      invalidateAccountQueries()
      notify({
        variant: 'success',
        title: t('accounts.messages.pauseFamilySuccess', { defaultValue: 'Linked accounts paused' }),
      })
    },
    onError: (error) => {
      notify({
        variant: 'error',
        title: t('accounts.messages.pauseFamilyFailed', { defaultValue: 'Failed to pause linked accounts' }),
        description: resolveErrorLabel(
          error,
          t('accounts.messages.requestFailed', { defaultValue: 'Request failed. Please try again later.' }),
        ),
      })
    },
  })

  const enableFamilyMutation = useMutation({
    mutationFn: (accountId: string) => accountsApi.enableFamily(accountId),
    onSuccess: () => {
      invalidateAccountQueries()
      notify({
        variant: 'success',
        title: t('accounts.messages.resumeFamilySuccess', { defaultValue: 'Linked accounts resumed' }),
      })
    },
    onError: (error) => {
      notify({
        variant: 'error',
        title: t('accounts.messages.resumeFamilyFailed', { defaultValue: 'Failed to resume linked accounts' }),
        description: resolveErrorLabel(
          error,
          t('accounts.messages.requestFailed', { defaultValue: 'Request failed. Please try again later.' }),
        ),
      })
    },
  })

  const planOptions = useMemo(() => {
    const set = new Set<string>()
    accountsRaw.forEach((account) => {
      if (!isSessionMode(account.mode)) {
        return
      }
      const status = oauthStatusMap.get(account.id)
      set.add(normalizePlanValue(status?.chatgpt_plan_type))
    })
    return Array.from(set).sort((a, b) => {
      if (a === PLAN_UNKNOWN_VALUE) {
        return 1
      }
      if (b === PLAN_UNKNOWN_VALUE) {
        return -1
      }
      return a.localeCompare(b)
    })
  }, [accountsRaw, oauthStatusMap])

  const filteredAccounts = useMemo(() => {
    return accountsRaw.filter((account) => {
      const status = oauthStatusMap.get(account.id)
      const effectiveEnabled = status?.effective_enabled ?? account.enabled

      if (statusFilter === 'active' && !effectiveEnabled) {
        return false
      }
      if (statusFilter === 'disabled' && effectiveEnabled) {
        return false
      }

      if (modeFilter === 'oauth' && !isSessionMode(account.mode)) {
        return false
      }
      if (modeFilter === 'api_key' && isSessionMode(account.mode)) {
        return false
      }

      if (credentialFilter !== 'all') {
        const credentialKind = resolveCredentialKindShort(status?.credential_kind)
        if (credentialKind !== credentialFilter) {
          return false
        }
      }

      if (planFilter !== 'all') {
        const planValue = normalizePlanValue(status?.chatgpt_plan_type)
        if (planValue !== planFilter) {
          return false
        }
      }

      return true
    })
  }, [accountsRaw, credentialFilter, modeFilter, oauthStatusMap, planFilter, statusFilter])

  const tableFilteredAccountIds = useMemo(
    () => tableFilteredAccounts.map((account) => account.id),
    [tableFilteredAccounts],
  )

  const tableFilteredAccountIdSet = useMemo(
    () => new Set(tableFilteredAccountIds),
    [tableFilteredAccountIds],
  )

  useEffect(() => {
    setSelectedAccountIds((prev) => {
      const next = prev.filter((id) => tableFilteredAccountIdSet.has(id))
      return next.length === prev.length ? prev : next
    })
  }, [tableFilteredAccountIdSet])

  const selectedAccountIdSet = useMemo(() => new Set(selectedAccountIds), [selectedAccountIds])

  const selectedAccounts = useMemo(
    () => tableFilteredAccounts.filter((account) => selectedAccountIdSet.has(account.id)),
    [selectedAccountIdSet, tableFilteredAccounts],
  )

  const selectedCount = selectedAccounts.length

  const selectedRefreshableAccountIds = useMemo(() => {
    return selectedAccounts
      .filter((account) => {
        if (!isSessionMode(account.mode)) {
          return false
        }
        const status = oauthStatusMap.get(account.id)
        return status?.auth_provider === 'oauth_refresh_token'
      })
      .map((account) => account.id)
  }, [oauthStatusMap, selectedAccounts])

  const selectedFamilyActionAccountIds = useMemo(() => {
    return selectedAccounts
      .filter((account) => {
        if (!isSessionMode(account.mode)) {
          return false
        }
        const status = oauthStatusMap.get(account.id)
        return (
          status?.auth_provider === 'oauth_refresh_token'
          && status.credential_kind !== 'one_time_access_token'
        )
      })
      .map((account) => account.id)
  }, [oauthStatusMap, selectedAccounts])

  const accountSearchFn = useCallback(
    (row: UpstreamAccount, keyword: string) => matchesAccountSearch(row, keyword, oauthStatusMap),
    [oauthStatusMap],
  )

  const handleFilteredDataChange = useCallback((rows: UpstreamAccount[]) => {
    setTableFilteredAccounts((prev) => {
      if (prev.length === rows.length && prev.every((item, index) => item.id === rows[index]?.id)) {
        return prev
      }
      return rows
    })
  }, [])

  const toggleAccountSelection = useCallback((accountId: string, checked: boolean) => {
    setSelectedAccountIds((prev) => {
      if (checked) {
        if (prev.includes(accountId)) {
          return prev
        }
        return [...prev, accountId]
      }
      return prev.filter((id) => id !== accountId)
    })
  }, [])

  const toggleSelectAllFiltered = useCallback(
    (checked: boolean) => {
      if (!checked) {
        setSelectedAccountIds([])
        return
      }
      setSelectedAccountIds(tableFilteredAccountIds)
    },
    [tableFilteredAccountIds],
  )

  const handleOpenDetailAccount = useCallback((account: UpstreamAccount) => {
    setDetailAccount(account)
    setDetailTab('profile')
  }, [])

  const handleCloseDetailDialog = useCallback((open: boolean) => {
    if (!open) {
      setDetailAccount(null)
      setDetailTab('profile')
    }
  }, [])

  const handleDeleteAccount = useCallback(
    async (account: UpstreamAccount) => {
      const confirmed = await confirm({
        title: t('accounts.actions.delete', { defaultValue: 'Delete Account' }),
        description: t('accounts.actions.deleteConfirm', {
          label: account.label,
          defaultValue: 'Delete account {{label}}?',
        }),
        cancelText: t('common.cancel', { defaultValue: 'Cancel' }),
        confirmText: t('common.delete', { defaultValue: 'Delete' }),
        variant: 'destructive',
      })
      if (!confirmed) {
        return
      }
      deleteAccountMutation.mutate(account.id)
    },
    [confirm, deleteAccountMutation, t],
  )

  const runBatchMutation = useCallback(
    async (action: AccountBatchAction) => {
      if (isBatchOperating) {
        return
      }

      const targetIds = (() => {
        if (action === 'refreshLogin') {
          return selectedRefreshableAccountIds
        }
        if (action === 'pauseFamily' || action === 'resumeFamily') {
          return selectedFamilyActionAccountIds
        }
        return selectedAccounts.map((account) => account.id)
      })()

      if (targetIds.length === 0) {
        return
      }

      if (action === 'delete') {
        const confirmed = await confirm({
          title: t('accounts.actions.batchDelete', { defaultValue: 'Batch Delete' }),
          description: t('accounts.actions.batchDeleteConfirm', {
            count: targetIds.length,
            defaultValue: 'Delete {{count}} selected accounts?',
          }),
          cancelText: t('common.cancel', { defaultValue: 'Cancel' }),
          confirmText: t('common.delete', { defaultValue: 'Delete' }),
          variant: 'destructive',
        })
        if (!confirmed) {
          return
        }
      }

      setIsBatchOperating(true)
      try {
        const worker = (accountId: string): Promise<unknown> => {
          if (action === 'enable') {
            return performSetEnabled(accountId, true)
          }
          if (action === 'disable') {
            return performSetEnabled(accountId, false)
          }
          if (action === 'refreshLogin') {
            return accountsApi.refreshOAuthJob(accountId)
          }
          if (action === 'pauseFamily') {
            return accountsApi.disableFamily(accountId)
          }
          if (action === 'resumeFamily') {
            return accountsApi.enableFamily(accountId)
          }
          return accountsApi.deleteAccount(accountId)
        }

        const actionLabel = (() => {
          if (action === 'enable') {
            return t('accounts.actions.batchEnable', { defaultValue: 'Batch Enable' })
          }
          if (action === 'disable') {
            return t('accounts.actions.batchDisable', { defaultValue: 'Batch Disable' })
          }
          if (action === 'refreshLogin') {
            return t('accounts.actions.batchRefreshLogin', { defaultValue: 'Batch Refresh Login ({{count}})' })
          }
          if (action === 'pauseFamily') {
            return t('accounts.actions.batchPauseFamily', { defaultValue: 'Batch Pause Family ({{count}})' })
          }
          if (action === 'resumeFamily') {
            return t('accounts.actions.batchResumeFamily', { defaultValue: 'Batch Resume Family ({{count}})' })
          }
          return t('accounts.actions.batchDelete', { defaultValue: 'Batch Delete' })
        })()

        const succeededIds: string[] = []
        let failed = 0
        let firstErrorMessage: string | null = null

        const setFirstErrorMessage = (message: string | null | undefined) => {
          if (!firstErrorMessage && message) {
            firstErrorMessage = message
          }
        }

        try {
          const batchResponse = await accountsApi.batchOperate(action, targetIds)
	          batchResponse.items.forEach((item) => {
	            if (item.ok) {
	              succeededIds.push(item.account_id)
	              if (action === 'refreshLogin' && item.job_id) {
                addRecentImportJobId(item.job_id)
              }
              return
	            }
	            failed += 1
	            setFirstErrorMessage(
	              item.error
	                ? localizeOAuthErrorCodeDisplay(t, item.error.code).label
	                : t('accounts.messages.batchUnknownError', {
	                    defaultValue: 'Batch operation failed',
	                  }),
	            )
	          })
        } catch (error) {
          const status = extractApiErrorStatus(error)
          if (status !== 404 && status !== 405 && status !== 501) {
            throw error
          }
          const { successes, failures } = await executeAccountBatch(
            targetIds,
            worker,
            {
              concurrency: BATCH_CONCURRENCY[action],
              maxRetries: 2,
              retryBaseDelayMs: 300,
            },
          )
          successes.forEach((result) => {
            succeededIds.push(result.accountId)
            if (
              action === 'refreshLogin'
              && result.value
              && typeof result.value === 'object'
              && 'job_id' in result.value
              && typeof (result.value as { job_id?: unknown }).job_id === 'string'
            ) {
              addRecentImportJobId((result.value as { job_id: string }).job_id)
            }
          })
	          failures.forEach((result) => {
	            setFirstErrorMessage(
	              resolveErrorLabel(
	                result.error,
	                t('accounts.messages.batchUnknownError', { defaultValue: 'Batch operation failed' }),
	              ),
	            )
	          })
          failed = failures.length
        }

        if (succeededIds.length > 0) {
          if (action === 'delete') {
            const succeededSet = new Set(succeededIds)
            setSelectedAccountIds((prev) => prev.filter((id) => !succeededSet.has(id)))
          }
          invalidateAccountQueries()
        }

        if (failed === 0) {
          notify({
            variant: 'success',
            title: t('accounts.messages.batchAllSuccess', {
              action: actionLabel,
              defaultValue: '{{action}} completed',
            }),
            description: t('accounts.messages.batchSuccessCount', {
              count: succeededIds.length,
              defaultValue: '{{count}} succeeded',
            }),
          })
        } else if (succeededIds.length > 0) {
          notify({
            variant: 'warning',
            title: t('accounts.messages.batchPartialFailedTitle', {
              action: actionLabel,
              defaultValue: '{{action}} partially failed',
            }),
            description: t('accounts.messages.batchPartialFailed', {
              failed,
              error: firstErrorMessage ? `: ${firstErrorMessage}` : '',
              defaultValue: '{{failed}} operations failed{{error}}',
            }),
          })
        } else {
          notify({
            variant: 'error',
            title: t('accounts.messages.batchAllFailed', {
              action: actionLabel,
              defaultValue: '{{action}} failed',
            }),
            description:
              firstErrorMessage
              ?? t('accounts.messages.batchUnknownError', { defaultValue: 'Batch operation failed' }),
          })
        }
      } finally {
        setIsBatchOperating(false)
      }
    },
    [
      confirm,
      invalidateAccountQueries,
      isBatchOperating,
	      performSetEnabled,
	      resolveErrorLabel,
	      selectedAccounts,
	      selectedFamilyActionAccountIds,
	      selectedRefreshableAccountIds,
	      t,
    ],
  )

  const columns = useAccountsColumns({
    oauthStatusMap,
    isOAuthStatusRefreshing,
    tableFilteredAccountIds,
    selectedAccountIdSet,
    onToggleSelectAllFiltered: toggleSelectAllFiltered,
    onToggleAccountSelection: toggleAccountSelection,
    onOpenDetailAccount: handleOpenDetailAccount,
    onRefreshAccount: refreshMutation.mutate,
    onToggleAccountEnabled: toggleEnabledMutation.mutate,
    onDeleteAccount: handleDeleteAccount,
    onPauseFamily: disableFamilyMutation.mutate,
    onResumeFamily: enableFamilyMutation.mutate,
    isRefreshPending: refreshMutation.isPending,
    isTogglePending: toggleEnabledMutation.isPending,
    isDeletePending: deleteAccountMutation.isPending,
    isPauseFamilyPending: disableFamilyMutation.isPending,
    isResumeFamilyPending: enableFamilyMutation.isPending,
  })

  const filteredStatusCount = filteredAccounts.length
  const hasPendingAccountAction =
    isBatchOperating
    || toggleEnabledMutation.isPending
    || deleteAccountMutation.isPending
    || refreshMutation.isPending
    || disableFamilyMutation.isPending
    || enableFamilyMutation.isPending
    || isManualRefreshing

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3 }}
      className="flex h-full flex-col overflow-hidden p-8"
    >
      <div className="mb-6 flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h2 className="text-3xl font-bold tracking-tight">{t('accounts.title')}</h2>
          <p className="mt-1 text-muted-foreground">{t('accounts.subtitle')}</p>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Button onClick={() => navigate('/imports')}>
            <Plus className="mr-2 h-4 w-4" />
            {t('accounts.actions.add', { defaultValue: 'Add Account' })}
          </Button>

          <Button
            variant="outline"
            onClick={() => {
              const payload = filteredAccounts.map((account) => ({
                ...account,
                oauth_status: oauthStatusMap.get(account.id),
              }))
              const blob = new Blob([JSON.stringify(payload, null, 2)], {
                type: 'application/json',
              })
              const url = URL.createObjectURL(blob)
              const anchor = document.createElement('a')
              anchor.href = url
              anchor.download = `accounts-${Date.now()}.json`
              anchor.click()
              URL.revokeObjectURL(url)
              notify({
                variant: 'success',
                title: t('accounts.messages.exportSuccess', { defaultValue: 'Export successful' }),
              })
            }}
          >
            <Download className="mr-2 h-4 w-4" />
            {t('accounts.actions.export')}
          </Button>

          <Button onClick={handleRefreshAccounts} disabled={isManualRefreshing}>
            <RefreshCw
              className={cn('mr-2 h-4 w-4', isManualRefreshing ? 'animate-spin' : undefined)}
            />
            {isManualRefreshing
              ? rateLimitRefreshJob && rateLimitRefreshJob.total > 0
                ? t('accounts.actions.refreshingAccounts', {
                  defaultValue: 'Refreshing Accounts',
                }) + ` ${rateLimitRefreshJob.processed}/${rateLimitRefreshJob.total}`
                : t('accounts.actions.refreshingAccounts', { defaultValue: 'Refreshing Accounts' })
              : t('accounts.actions.refreshAccounts')}
          </Button>
        </div>
      </div>

      <div className="relative min-h-0 flex-1">
        <LoadingOverlay
          show={isLoading || isManualRefreshing}
          title={t('accounts.syncing')}
          description={t('common.loading')}
        />

        <StandardDataTable
          columns={columns}
          data={filteredAccounts}
          density="comfortable"
          rowClassName="h-[60px]"
          searchPlaceholder={t('accounts.searchPlaceholder')}
          searchFn={accountSearchFn}
          onFilteredDataChange={handleFilteredDataChange}
          actions={(
            <div className="flex flex-wrap items-center gap-2">
              <Badge variant={selectedCount > 0 ? 'info' : 'secondary'} className="font-normal">
                {t('accounts.actions.selectedCount', {
                  count: selectedCount,
                  defaultValue: '{{count}} selected',
                })}
              </Badge>
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    disabled={selectedCount === 0 || hasPendingAccountAction}
                  >
                    {t('accounts.actions.batchMenu', { defaultValue: 'Batch Actions' })}
                    <ChevronDown className="h-3.5 w-3.5" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="w-[260px]">
                  <DropdownMenuLabel>
                    {t('accounts.actions.selectedCount', {
                      count: selectedCount,
                      defaultValue: '{{count}} selected',
                    })}
                  </DropdownMenuLabel>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem
                    className="cursor-pointer"
                    disabled={selectedCount === 0 || hasPendingAccountAction}
                    onClick={() => runBatchMutation('enable')}
                  >
                    {t('accounts.actions.batchEnable', { defaultValue: 'Batch Enable' })}
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    className="cursor-pointer"
                    disabled={selectedCount === 0 || hasPendingAccountAction}
                    onClick={() => runBatchMutation('disable')}
                  >
                    {t('accounts.actions.batchDisable', { defaultValue: 'Batch Disable' })}
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    className="cursor-pointer"
                    disabled={selectedRefreshableAccountIds.length === 0 || hasPendingAccountAction}
                    onClick={() => runBatchMutation('refreshLogin')}
                  >
                    {t('accounts.actions.batchRefreshLogin', {
                      count: selectedRefreshableAccountIds.length,
                      defaultValue: 'Batch Refresh Login ({{count}})',
                    })}
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    className="cursor-pointer"
                    disabled={selectedFamilyActionAccountIds.length === 0 || hasPendingAccountAction}
                    onClick={() => runBatchMutation('pauseFamily')}
                  >
                    {t('accounts.actions.batchPauseFamily', {
                      count: selectedFamilyActionAccountIds.length,
                      defaultValue: 'Batch Pause Family ({{count}})',
                    })}
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    className="cursor-pointer"
                    disabled={selectedFamilyActionAccountIds.length === 0 || hasPendingAccountAction}
                    onClick={() => runBatchMutation('resumeFamily')}
                  >
                    {t('accounts.actions.batchResumeFamily', {
                      count: selectedFamilyActionAccountIds.length,
                      defaultValue: 'Batch Resume Family ({{count}})',
                    })}
                  </DropdownMenuItem>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem
                    className="cursor-pointer text-destructive focus:bg-destructive/10"
                    disabled={selectedCount === 0 || hasPendingAccountAction}
                    onClick={() => runBatchMutation('delete')}
                  >
                    {t('accounts.actions.batchDelete', { defaultValue: 'Batch Delete' })}
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
          )}
          filters={(
            <>
              <Select value={statusFilter} onValueChange={(value) => setStatusFilter(value as StatusFilter)}>
                <SelectTrigger className="w-[170px]" aria-label={t('accounts.actions.filter')}>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">{t('accounts.filters.all')}</SelectItem>
                  <SelectItem value="active">{t('accounts.filters.active')}</SelectItem>
                  <SelectItem value="disabled">{t('accounts.filters.disabled')}</SelectItem>
                </SelectContent>
              </Select>

              <Select value={modeFilter} onValueChange={(value) => setModeFilter(value as ModeFilter)}>
                <SelectTrigger className="w-[170px]" aria-label={t('accounts.filters.mode')}>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">{t('accounts.filters.modeAll')}</SelectItem>
                  <SelectItem value="oauth">{t('accounts.filters.modeOAuth')}</SelectItem>
                  <SelectItem value="api_key">{t('accounts.filters.modeApiKey')}</SelectItem>
                </SelectContent>
              </Select>

              <Select
                value={credentialFilter}
                onValueChange={(value) => setCredentialFilter(value as CredentialFilter)}
              >
                <SelectTrigger className="w-[170px]" aria-label={t('accounts.filters.credential')}>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">
                    {t('accounts.filters.credentialAll', { defaultValue: 'All Credentials' })}
                  </SelectItem>
                  <SelectItem value="rt">
                    {t('accounts.filters.credentialRt', { defaultValue: 'RT' })}
                  </SelectItem>
                  <SelectItem value="at">
                    {t('accounts.filters.credentialAt', { defaultValue: 'AT' })}
                  </SelectItem>
                  <SelectItem value="unknown">
                    {t('accounts.filters.credentialUnknown', { defaultValue: 'Unknown' })}
                  </SelectItem>
                </SelectContent>
              </Select>

              <Select value={planFilter} onValueChange={(value) => setPlanFilter(value as PlanFilter)}>
                <SelectTrigger className="w-[170px]" aria-label={t('accounts.filters.plan')}>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">
                    {t('accounts.filters.planAll', { defaultValue: 'All Plans' })}
                  </SelectItem>
                  {planOptions.map((plan) => (
                    <SelectItem key={plan} value={plan}>
                      {plan === PLAN_UNKNOWN_VALUE
                        ? t('accounts.filters.planUnknown', { defaultValue: 'Not Reported' })
                        : plan}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>

              <Badge variant="secondary" className="font-normal">
                {t('accounts.filters.total', { count: filteredStatusCount })}
              </Badge>
            </>
          )}
        />
      </div>

      <AccountDetailDialog
        account={detailAccount}
        detailTab={detailTab}
        onDetailTabChange={setDetailTab}
        onOpenChange={handleCloseDetailDialog}
        isSessionAccount={detailIsSessionAccount}
        oauthStatus={detailOAuthStatus}
        oauthStatusLoading={detailOAuthStatusQuery.isLoading}
        rateLimitDisplays={detailRateLimitDisplays}
        locale={i18n.resolvedLanguage ?? i18n.language}
      />

      {confirmDialog}
    </motion.div>
  )
}
