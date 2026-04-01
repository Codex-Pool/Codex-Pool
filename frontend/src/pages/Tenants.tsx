import { useCallback, useEffect, useMemo, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { type ColumnDef } from '@tanstack/react-table'
import { Button } from '@heroui/react'
import { motion } from 'framer-motion'
import { Copy, Loader2, Plus, RefreshCw } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import {
  adminTenantsApi,
  type AdminImpersonateResponse,
  type AdminTenantItem,
} from '@/api/adminTenants'
import { apiClient } from '@/api/client'
import { localizeApiErrorDisplay } from '@/api/errorI18n'
import { apiKeysApi, type ApiKey, type CreateApiKeyResponse } from '@/api/settings'
import { systemApi, DEFAULT_SYSTEM_CAPABILITIES } from '@/api/system'
import type { UsageSummaryQueryResponse } from '@/api/types'
import {
  AntigravityDialogActions,
  AntigravityDialogBody,
  AntigravityDialogPanel,
  AntigravityDialogShell,
} from '@/components/layout/dialog-archetypes'
import { DockedPageIntro, PageContent, PagePanel, SectionHeader } from '@/components/layout/page-archetypes'
import { Badge } from '@/components/ui/badge'
import { AccessibleTabList } from '@/components/ui/accessible-tabs'
import {
  Dialog,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import {
  SurfaceCode,
  SurfaceDivider,
  SurfaceInset,
  SurfaceNotice,
} from '@/components/ui/surface'
import { DataTable } from '@/components/DataTable'
import { TenantUsageSection } from '@/features/tenants/tenant-usage-section'
import { notify } from '@/lib/notification'
import {
  LABEL_CLASS_NAME,
  USAGE_API_KEY_FILTER_ALL,
  copyText,
  createDateTimeFormatter,
  formatDateTimeValue,
  formatMicrocredits,
  maskToken,
  toIsoDatetime,
  toLocalDatetimeInput,
  type TenantProfileTab,
} from '@/features/tenants/utils'

const DEFAULT_TENANT_RECHARGE_REASON_CODE = 'admin_recharge'
const DEFAULT_TENANT_IMPERSONATION_REASON_CODE = 'support'

function normalizeEnumValue(value?: string | null): string {
  return (value ?? '').trim().toLowerCase()
}

function localizeTenantStatus(value: string | undefined, t: ReturnType<typeof useTranslation>['t']): string {
  const normalized = normalizeEnumValue(value)
  if (normalized === 'active') {
    return t('tenants.list.statusValues.active', { defaultValue: 'Active' })
  }
  if (normalized === 'inactive' || normalized === 'disabled' || normalized === 'revoked') {
    return t('tenants.list.statusValues.inactive', { defaultValue: 'Inactive' })
  }
  return t('tenants.list.statusValues.unknown', {
    defaultValue: 'Unknown ({{value}})',
    value: value?.trim() || '-',
  })
}

function localizeTenantPlan(value: string | undefined, t: ReturnType<typeof useTranslation>['t']): string {
  const normalized = normalizeEnumValue(value)
  if (normalized === 'credit') {
    return t('tenants.list.planValues.credit', { defaultValue: 'Credit' })
  }
  return t('tenants.list.planValues.unknown', {
    defaultValue: 'Custom ({{value}})',
    value: value?.trim() || '-',
  })
}

interface TenantPoolRow {
  tenant: AdminTenantItem
  isDefaultAdmin: boolean
  apiKeyCount: number
}

export default function Tenants() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const { data: capabilities = DEFAULT_SYSTEM_CAPABILITIES } = useQuery({
    queryKey: ['systemCapabilities'],
    queryFn: () => systemApi.getCapabilities(),
    staleTime: 5 * 60_000,
  })

  const [createForm, setCreateForm] = useState({
    name: '',
    status: 'active',
    plan: 'credit',
    expires_at: '',
  })

  const [profileTenant, setProfileTenant] = useState<AdminTenantItem | null>(null)
  const [profileTab, setProfileTab] = useState<TenantProfileTab>('profile')
  const [profileForm, setProfileForm] = useState({
    status: '',
    plan: '',
    expires_at: '',
  })
  const [rechargeForm, setRechargeForm] = useState({
    amount_microcredits: '100000000',
    reason: DEFAULT_TENANT_RECHARGE_REASON_CODE,
  })
  const [impersonationReason, setImpersonationReason] = useState(
    DEFAULT_TENANT_IMPERSONATION_REASON_CODE,
  )
  const [lastImpersonation, setLastImpersonation] = useState<AdminImpersonateResponse | null>(null)

  const [newKeyName, setNewKeyName] = useState('')
  const [createdKey, setCreatedKey] = useState<CreateApiKeyResponse | null>(null)
  const [pendingKeyId, setPendingKeyId] = useState<string | null>(null)
  const [usageApiKeyFilter, setUsageApiKeyFilter] = useState(USAGE_API_KEY_FILTER_ALL)
  const [usageApiKeyKeyword, setUsageApiKeyKeyword] = useState('')
  const [usageApiKeyPopoverOpen, setUsageApiKeyPopoverOpen] = useState(false)

  const dateTimeFormatter = useMemo(() => createDateTimeFormatter(), [])

  const formatDateTime = useCallback(
    (value?: string | null) => formatDateTimeValue(dateTimeFormatter, value),
    [dateTimeFormatter],
  )

  const notifySuccess = useCallback((title: string) => {
    notify({
      variant: 'success',
      title,
    })
  }, [])

  const notifyError = useCallback(
    (error: unknown, fallback: string) => {
      const description = localizeApiErrorDisplay(t, error, fallback).label
      notify({
        variant: 'error',
        title: fallback,
        description: description !== fallback ? description : undefined,
      })
    },
    [t],
  )

  const tenantsQuery = useQuery({
    queryKey: ['adminTenants'],
    queryFn: () => adminTenantsApi.listTenants(),
    staleTime: 60000,
  })

  const keysQuery = useQuery({
    queryKey: ['apiKeys'],
    queryFn: apiKeysApi.listKeys,
    staleTime: 15000,
    refetchInterval: 30000,
  })

  const createTenantMutation = useMutation({
    mutationFn: async () =>
      adminTenantsApi.createTenant({
        name: createForm.name,
        status: createForm.status || undefined,
        plan: createForm.plan || undefined,
        expires_at: toIsoDatetime(createForm.expires_at),
      }),
    onSuccess: (tenant) => {
      notifySuccess(
        t('tenants.messages.createSuccess', {
          defaultValue: 'Tenant created: {{name}} ({{id}})',
          name: tenant.name,
          id: tenant.id,
        }),
      )
      setCreateForm({ name: '', status: 'active', plan: 'credit', expires_at: '' })
      queryClient.invalidateQueries({ queryKey: ['adminTenants'] })
    },
    onError: (err) => {
      const fallback = t('tenants.messages.createFailed', { defaultValue: 'Failed to create tenant' })
      notifyError(err, fallback)
    },
  })

  const patchTenantMutation = useMutation({
    mutationFn: async () => {
      if (!profileTenant) {
        throw new Error('tenant not selected')
      }
      return adminTenantsApi.patchTenant(profileTenant.id, {
        status: profileForm.status.trim() || undefined,
        plan: profileForm.plan.trim() || undefined,
        expires_at: toIsoDatetime(profileForm.expires_at),
      })
    },
    onSuccess: (tenant) => {
      notifySuccess(
        t('tenants.messages.updateSuccess', {
          defaultValue: 'Tenant updated: {{name}}',
          name: tenant.name,
        }),
      )
      queryClient.invalidateQueries({ queryKey: ['adminTenants'] })
    },
    onError: (err) => {
      const fallback = t('tenants.messages.updateFailed', { defaultValue: 'Failed to update tenant' })
      notifyError(err, fallback)
    },
  })

  const rechargeMutation = useMutation({
    mutationFn: async () => {
      if (!profileTenant) {
        throw new Error('tenant not selected')
      }
      return adminTenantsApi.rechargeTenant(profileTenant.id, {
        amount_microcredits: Number(rechargeForm.amount_microcredits),
        reason: rechargeForm.reason.trim() || undefined,
      })
    },
    onSuccess: (response) => {
      notifySuccess(
        t('tenants.messages.rechargeSuccess', {
          defaultValue: 'Recharge successful: +{{amount}}, current balance {{balance}}',
          amount: formatMicrocredits(response.amount_microcredits),
          balance: formatMicrocredits(response.balance_microcredits),
        }),
      )
    },
    onError: (err) => {
      const fallback = t('tenants.messages.rechargeFailed', { defaultValue: 'Failed to recharge tenant' })
      notifyError(err, fallback)
    },
  })

  const impersonationMutation = useMutation({
    mutationFn: async () => {
      if (!profileTenant) {
        throw new Error('tenant not selected')
      }
      return adminTenantsApi.createImpersonation({
        tenant_id: profileTenant.id,
        reason: impersonationReason,
      })
    },
    onSuccess: (response) => {
      setLastImpersonation(response)
      notifySuccess(
        t('tenants.messages.impersonationCreated', {
          defaultValue: 'Impersonation session created (token returned)',
        }),
      )
    },
    onError: (err) => {
      const fallback = t('tenants.messages.impersonationCreateFailed', {
        defaultValue: 'Failed to create impersonation',
      })
      notifyError(err, fallback)
    },
  })

  const revokeImpersonationMutation = useMutation({
    mutationFn: async (sessionId: string) => adminTenantsApi.deleteImpersonation(sessionId),
    onSuccess: () => {
      setLastImpersonation(null)
      notifySuccess(
        t('tenants.messages.impersonationRevoked', {
          defaultValue: 'Impersonation session revoked',
        }),
      )
    },
    onError: (err) => {
      const fallback = t('tenants.messages.impersonationRevokeFailed', {
        defaultValue: 'Failed to revoke impersonation',
      })
      notifyError(err, fallback)
    },
  })

  const createKeyMutation = useMutation({
    mutationFn: async () => {
      if (!profileTenant) {
        throw new Error('tenant not selected')
      }
      const name = newKeyName.trim()
      if (!name) {
        throw new Error(t('tenants.messages.apiKeyNameRequired', { defaultValue: 'Please enter an API key name' }))
      }
      return apiKeysApi.createKey(name, undefined, profileTenant.id)
    },
    onSuccess: (payload) => {
      setCreatedKey(payload)
      notifySuccess(
        t('tenants.messages.apiKeyCreateSuccess', {
          defaultValue: 'Created API key for tenant {{tenantName}}: {{keyName}}',
          tenantName: profileTenant?.name ?? '',
          keyName: payload.record.name,
        }),
      )
      setNewKeyName('')
      queryClient.invalidateQueries({ queryKey: ['apiKeys'] })
    },
    onError: (err) => {
      const fallback = t('tenants.messages.apiKeyCreateFailed', { defaultValue: 'Failed to create API key' })
      notifyError(err, fallback)
    },
  })

  const toggleKeyMutation = useMutation({
    mutationFn: async ({ keyId, enabled }: { keyId: string; enabled: boolean }) =>
      apiKeysApi.updateKeyEnabled(keyId, enabled),
    onMutate: ({ keyId }) => setPendingKeyId(keyId),
    onSettled: () => setPendingKeyId(null),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['apiKeys'] })
    },
    onError: (err) => {
      const fallback = t('tenants.messages.apiKeyToggleFailed', {
        defaultValue: 'Failed to update API key status',
      })
      notifyError(err, fallback)
    },
  })

  const tenantRows = useMemo(() => {
    const rows = [...(tenantsQuery.data ?? [])]
    rows.sort((a, b) => {
      const aIsAdmin = a.name.trim().toLowerCase() === 'admin'
      const bIsAdmin = b.name.trim().toLowerCase() === 'admin'
      if (aIsAdmin && !bIsAdmin) {
        return -1
      }
      if (!aIsAdmin && bIsAdmin) {
        return 1
      }
      return new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
    })
    return rows
  }, [tenantsQuery.data])

  const keyCountByTenant = useMemo(() => {
    const map = new Map<string, number>()
    for (const key of keysQuery.data ?? []) {
      if (!key.tenant_id) {
        continue
      }
      map.set(key.tenant_id, (map.get(key.tenant_id) ?? 0) + 1)
    }
    return map
  }, [keysQuery.data])

  const tenantPoolRows = useMemo<TenantPoolRow[]>(
    () =>
      tenantRows.map((tenant) => ({
        tenant,
        isDefaultAdmin: tenant.name.trim().toLowerCase() === 'admin',
        apiKeyCount: keyCountByTenant.get(tenant.id) ?? 0,
      })),
    [keyCountByTenant, tenantRows],
  )

  const keysForCurrentTenant = useMemo(() => {
    if (!profileTenant) {
      return []
    }
    return (keysQuery.data ?? []).filter((key) => key.tenant_id === profileTenant.id)
  }, [keysQuery.data, profileTenant])

  const filteredUsageApiKeys = useMemo(() => {
    const keyword = usageApiKeyKeyword.trim().toLowerCase()
    if (!keyword) {
      return keysForCurrentTenant
    }
    return keysForCurrentTenant.filter((key) => {
      const haystack = `${key.name} ${key.key_prefix} ${key.id}`.toLowerCase()
      return haystack.includes(keyword)
    })
  }, [keysForCurrentTenant, usageApiKeyKeyword])

  const effectiveUsageApiKeyFilter = useMemo(() => {
    if (usageApiKeyFilter === USAGE_API_KEY_FILTER_ALL) {
      return USAGE_API_KEY_FILTER_ALL
    }
    const exists = keysForCurrentTenant.some((key) => key.id === usageApiKeyFilter)
    return exists ? usageApiKeyFilter : USAGE_API_KEY_FILTER_ALL
  }, [keysForCurrentTenant, usageApiKeyFilter])

  const selectedUsageApiKey = useMemo(
    () => keysForCurrentTenant.find((key) => key.id === effectiveUsageApiKeyFilter) ?? null,
    [effectiveUsageApiKeyFilter, keysForCurrentTenant],
  )

  const usageSummaryQuery = useQuery({
    queryKey: ['tenantUsageSummary', profileTenant?.id, effectiveUsageApiKeyFilter],
    enabled: Boolean(profileTenant && profileTab === 'usage'),
    staleTime: 60000,
    queryFn: async () => {
      if (!profileTenant) {
        throw new Error('missing tenant')
      }
      const endTs = Math.floor(Date.now() / 1000)
      const startTs = endTs - 24 * 60 * 60
      const response = await apiClient.get<UsageSummaryQueryResponse>('/usage/summary', {
        params: {
          start_ts: startTs,
          end_ts: endTs,
          tenant_id: profileTenant.id,
          api_key_id:
            effectiveUsageApiKeyFilter === USAGE_API_KEY_FILTER_ALL
              ? undefined
              : effectiveUsageApiKeyFilter,
        },
      })
      return response.data
    },
  })

  const openTenantProfile = useCallback((tenant: AdminTenantItem) => {
    setProfileTenant(tenant)
    setProfileTab('profile')
    setProfileForm({
      status: tenant.status,
      plan: tenant.plan,
      expires_at: toLocalDatetimeInput(tenant.expires_at),
    })
    setRechargeForm({
      amount_microcredits: '100000000',
      reason: DEFAULT_TENANT_RECHARGE_REASON_CODE,
    })
    setImpersonationReason(DEFAULT_TENANT_IMPERSONATION_REASON_CODE)
    setNewKeyName(`${tenant.name}-key`)
    setCreatedKey(null)
    setLastImpersonation(null)
    setUsageApiKeyFilter(USAGE_API_KEY_FILTER_ALL)
    setUsageApiKeyKeyword('')
    setUsageApiKeyPopoverOpen(false)
  }, [])

  const tenantPoolColumns = useMemo<ColumnDef<TenantPoolRow>[]>(
    () => [
      {
        id: 'tenant',
        header: t('tenants.list.columns.tenant', { defaultValue: 'Tenant' }),
        accessorFn: (row) => row.tenant.name.toLowerCase(),
        cell: ({ row }) => (
          <div className="flex items-center gap-2">
            <span>{row.original.tenant.name}</span>
            {row.original.isDefaultAdmin ? (
              <Badge variant="info">{t('tenants.list.defaultBadge', { defaultValue: 'Default' })}</Badge>
            ) : null}
          </div>
        ),
      },
      {
        id: 'tenantId',
        header: t('tenants.list.columns.tenantId', { defaultValue: 'Tenant ID' }),
        accessorFn: (row) => row.tenant.id.toLowerCase(),
        cell: ({ row }) => <span className="font-mono text-xs">{row.original.tenant.id}</span>,
      },
      {
        id: 'status',
        header: t('tenants.list.columns.status', { defaultValue: 'Status' }),
        accessorFn: (row) => normalizeEnumValue(row.tenant.status),
        cell: ({ row }) => (
          <Badge variant={normalizeEnumValue(row.original.tenant.status) === 'active' ? 'success' : 'secondary'}>
            {localizeTenantStatus(row.original.tenant.status, t)}
          </Badge>
        ),
      },
      {
        id: 'plan',
        header: t('tenants.list.columns.plan', { defaultValue: 'Plan' }),
        accessorFn: (row) => normalizeEnumValue(row.tenant.plan),
        cell: ({ row }) => <span>{localizeTenantPlan(row.original.tenant.plan, t)}</span>,
      },
      {
        id: 'expiresAt',
        header: t('tenants.list.columns.expiresAt', { defaultValue: 'Expires At' }),
        accessorFn: (row) => row.tenant.expires_at ?? '',
        cell: ({ row }) => <span>{formatDateTime(row.original.tenant.expires_at)}</span>,
      },
      {
        id: 'apiKeys',
        header: t('tenants.list.columns.apiKeys', { defaultValue: 'API Keys' }),
        accessorFn: (row) => row.apiKeyCount,
        cell: ({ row }) => <span>{row.original.apiKeyCount}</span>,
      },
      {
        id: 'updatedAt',
        header: t('tenants.list.columns.updatedAt', { defaultValue: 'Updated At' }),
        accessorFn: (row) => row.tenant.updated_at ?? '',
        cell: ({ row }) => <span>{formatDateTime(row.original.tenant.updated_at)}</span>,
      },
      {
        id: 'actions',
        header: t('tenants.list.columns.actions', { defaultValue: 'Actions' }),
        cell: ({ row }) => (
          <Button size="sm" variant="flat" onClick={() => openTenantProfile(row.original.tenant)}>
            {t('tenants.list.openProfile', { defaultValue: 'Open Tenant Profile' })}
          </Button>
        ),
      },
    ],
    [formatDateTime, openTenantProfile, t],
  )

  const tenantKeyColumns = useMemo<ColumnDef<ApiKey>[]>(
    () => [
      {
        id: 'name',
        header: t('tenants.keys.list.columns.name', { defaultValue: 'Name' }),
        accessorFn: (row) => row.name.toLowerCase(),
        cell: ({ row }) => row.original.name,
      },
      {
        id: 'prefix',
        header: t('tenants.keys.list.columns.prefix', { defaultValue: 'Prefix' }),
        accessorFn: (row) => row.key_prefix.toLowerCase(),
        cell: ({ row }) => (
          <div className="flex items-center gap-2 font-mono text-xs">
            <span>{row.original.key_prefix}****************</span>
            <Button
              type="button"
              size="sm"
              variant="light"
              isIconOnly
              onClick={() => copyText(row.original.key_prefix)}
              aria-label={t('tenants.keys.list.copyPrefix', { defaultValue: 'Copy key prefix' })}
              title={t('tenants.keys.list.copyPrefix', { defaultValue: 'Copy key prefix' })}
            >
              <Copy className="h-3.5 w-3.5" />
            </Button>
          </div>
        ),
      },
      {
        id: 'status',
        header: t('tenants.keys.list.columns.status', { defaultValue: 'Status' }),
        accessorFn: (row) => (row.enabled ? 'active' : 'revoked'),
        cell: ({ row }) => (
          <Badge variant={row.original.enabled ? 'success' : 'secondary'}>
            {row.original.enabled
              ? t('tenants.keys.list.status.active', { defaultValue: 'Active' })
              : t('tenants.keys.list.status.revoked', { defaultValue: 'Revoked' })}
          </Badge>
        ),
      },
      {
        id: 'createdAt',
        header: t('tenants.keys.list.columns.createdAt', { defaultValue: 'Created At' }),
        accessorFn: (row) => row.created_at ?? '',
        cell: ({ row }) => formatDateTime(row.original.created_at),
      },
      {
        id: 'actions',
        header: t('tenants.keys.list.columns.actions', { defaultValue: 'Actions' }),
        cell: ({ row }) => {
          const isPending = pendingKeyId === row.original.id && toggleKeyMutation.isPending
          return (
            <Button
              size="sm"
              onClick={() =>
                toggleKeyMutation.mutate({
                  keyId: row.original.id,
                  enabled: !row.original.enabled,
                })
              }
              disabled={isPending}
            >
              {isPending ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
              {row.original.enabled
                ? t('tenants.keys.list.disable', { defaultValue: 'Disable' })
                : t('tenants.keys.list.enable', { defaultValue: 'Enable' })}
            </Button>
          )
        },
      },
    ],
    [formatDateTime, pendingKeyId, t, toggleKeyMutation],
  )

  useEffect(() => {
    if (!lastImpersonation) {
      return
    }
    const timeoutId = window.setTimeout(() => {
      setLastImpersonation(null)
    }, 60_000)
    return () => window.clearTimeout(timeoutId)
  }, [lastImpersonation])

  return (
    <PageContent className="h-full overflow-hidden">
      <motion.div
        initial={{ opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.3 }}
        className="flex h-full flex-col overflow-hidden"
      >
      <div className="space-y-6 overflow-y-auto pr-1">
        <DockedPageIntro
          archetype="workspace"
          title={t('nav.tenants')}
          description={t('tenants.subtitle', {
            defaultValue: 'Check tenant availability and manage profiles, API keys, and usage.',
          })}
          actions={(
            <Button
              size="sm"
              onClick={() => {
                queryClient.invalidateQueries({ queryKey: ['adminTenants'] })
                queryClient.invalidateQueries({ queryKey: ['apiKeys'] })
              }}
              disabled={tenantsQuery.isFetching}
            >
              <RefreshCw className={`mr-2 h-4 w-4 ${tenantsQuery.isFetching ? 'animate-spin' : ''}`} />
              {t('common.refresh')}
            </Button>
          )}
        />

      <PagePanel className="space-y-4">
        <SectionHeader
          title={t('tenants.create.title', { defaultValue: 'Create Tenant' })}
        />

        <form
          className="grid gap-3 lg:grid-cols-[minmax(0,1fr)_auto]"
          onSubmit={(event) => {
            event.preventDefault()
            createTenantMutation.mutate()
          }}
        >
          <div className="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-4">
            <div className="space-y-1.5">
              <label htmlFor="create-tenant-name" className={LABEL_CLASS_NAME}>
                {t('tenants.create.fields.name', { defaultValue: 'Tenant Name' })}
              </label>
              <Input
                id="create-tenant-name"
                name="name"
                value={createForm.name}
                autoComplete="off"
                spellCheck={false}
                onChange={(e) => setCreateForm((prev) => ({ ...prev, name: e.target.value }))}
              />
            </div>
            <div className="space-y-1.5">
              <label htmlFor="create-tenant-status" className={LABEL_CLASS_NAME}>
                {t('tenants.create.fields.status', { defaultValue: 'Status (active/inactive)' })}
              </label>
              <Input
                id="create-tenant-status"
                name="status"
                value={createForm.status}
                autoComplete="off"
                spellCheck={false}
                onChange={(e) => setCreateForm((prev) => ({ ...prev, status: e.target.value }))}
              />
            </div>
            <div className="space-y-1.5">
              <label htmlFor="create-tenant-plan" className={LABEL_CLASS_NAME}>
                {t('tenants.create.fields.plan', { defaultValue: 'Plan (credit)' })}
              </label>
              <Input
                id="create-tenant-plan"
                name="plan"
                value={createForm.plan}
                autoComplete="off"
                spellCheck={false}
                onChange={(e) => setCreateForm((prev) => ({ ...prev, plan: e.target.value }))}
              />
            </div>
            <div className="space-y-1.5">
              <label htmlFor="create-tenant-expire" className={LABEL_CLASS_NAME}>
                {t('tenants.create.fields.expiresAt', { defaultValue: 'Expires At' })}
              </label>
              <Input
                id="create-tenant-expire"
                name="expires_at"
                type="datetime-local"
                value={createForm.expires_at}
                autoComplete="off"
                onChange={(e) =>
                  setCreateForm((prev) => ({ ...prev, expires_at: e.target.value }))
                }
              />
            </div>
          </div>
          <div className="flex items-end lg:justify-end">
            <Button color="primary" type="submit" disabled={createTenantMutation.isPending} className="w-full lg:w-auto">
              {createTenantMutation.isPending ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <Plus className="mr-2 h-4 w-4" />
              )}
              {t('tenants.create.submit', { defaultValue: 'Create Tenant' })}
            </Button>
          </div>
        </form>
      </PagePanel>

      <PagePanel className="space-y-4">
        <SectionHeader title={t('tenants.list.title', { defaultValue: 'Tenant Pool' })} />
        <DataTable
          columns={tenantPoolColumns}
          data={tenantPoolRows}
          defaultPageSize={20}
          pageSizeOptions={[20, 50, 100]}
          density="compact"
          searchPlaceholder={t('tenants.list.searchPlaceholder', {
            defaultValue: 'Search tenant by name, ID, status or plan',
          })}
          emptyText={t('tenants.list.empty', { defaultValue: 'No tenant data' })}
        />
      </PagePanel>

      <Dialog
        open={Boolean(profileTenant)}
        onOpenChange={(open) => {
          if (!open) {
            setProfileTenant(null)
            setProfileTab('profile')
            setCreatedKey(null)
            setLastImpersonation(null)
            setUsageApiKeyFilter(USAGE_API_KEY_FILTER_ALL)
            setUsageApiKeyKeyword('')
            setUsageApiKeyPopoverOpen(false)
          }
        }}
      >
        <AntigravityDialogShell
          size="xl"
          title={
            profileTenant
              ? t('tenants.profile.dialogTitleWithName', {
                  defaultValue: 'Tenant Profile · {{name}}',
                  name: profileTenant.name,
                })
              : t('tenants.profile.dialogTitle', { defaultValue: 'Tenant Profile' })
          }
          description={t('tenants.profile.dialogDescription', {
            defaultValue: 'Manage profile, API keys, and usage in one dialog with tabs.',
          })}
          footer={(
            <AntigravityDialogActions>
              <Button variant="flat" onClick={() => setProfileTenant(null)}>
                {t('common.close')}
              </Button>
            </AntigravityDialogActions>
          )}
        >

          {profileTenant ? (
            <AntigravityDialogBody className="space-y-4">
              <AccessibleTabList
                idBase="tenants-profile"
                ariaLabel={t('tenants.profile.tabs.ariaLabel', { defaultValue: 'Tenant profile tabs' })}
                value={profileTab}
                onValueChange={setProfileTab}
                items={[
                  {
                    value: 'profile',
                    label: t('tenants.profile.tabs.profile', { defaultValue: 'Profile' }),
                  },
                  {
                    value: 'keys',
                    label: t('tenants.profile.tabs.keys', { defaultValue: 'API Keys' }),
                  },
                  {
                    value: 'usage',
                    label: t('tenants.profile.tabs.usage', { defaultValue: 'Usage' }),
                  },
                ]}
              />

              {profileTab === 'profile' ? (
                <section
                  id="tenants-profile-panel-profile"
                  role="tabpanel"
                  tabIndex={0}
                  aria-labelledby="tenants-profile-tab-profile"
                  className="grid gap-4 lg:grid-cols-2"
                >
                  <AntigravityDialogPanel as="section">
                    <h3 className="text-base font-medium">
                      {t('tenants.profile.section.title', { defaultValue: 'Tenant Profile' })}
                    </h3>
                    <div className="grid grid-cols-2 gap-2 text-sm">
                      <div className="text-muted-foreground">
                        {t('tenants.profile.meta.tenantId', { defaultValue: 'Tenant ID' })}
                      </div>
                      <div className="font-mono text-xs break-all">{profileTenant.id}</div>
                      <div className="text-muted-foreground">
                        {t('tenants.profile.meta.createdAt', { defaultValue: 'Created At' })}
                      </div>
                      <div>{formatDateTime(profileTenant.created_at)}</div>
                      <div className="text-muted-foreground">
                        {t('tenants.profile.meta.updatedAt', { defaultValue: 'Updated At' })}
                      </div>
                      <div>{formatDateTime(profileTenant.updated_at)}</div>
                    </div>

                    <div className="grid grid-cols-1 gap-3 md:grid-cols-3">
                      <div className="space-y-1.5">
                        <label htmlFor="profile-status" className={LABEL_CLASS_NAME}>
                          {t('tenants.profile.fields.status', { defaultValue: 'Status' })}
                        </label>
                        <Input
                          id="profile-status"
                          name="status"
                          value={profileForm.status}
                          onChange={(e) =>
                            setProfileForm((prev) => ({ ...prev, status: e.target.value }))
                          }
                        />
                      </div>
                      <div className="space-y-1.5">
                        <label htmlFor="profile-plan" className={LABEL_CLASS_NAME}>
                          {t('tenants.profile.fields.plan', { defaultValue: 'Plan' })}
                        </label>
                        <Input
                          id="profile-plan"
                          name="plan"
                          value={profileForm.plan}
                          onChange={(e) =>
                            setProfileForm((prev) => ({ ...prev, plan: e.target.value }))
                          }
                        />
                      </div>
                      <div className="space-y-1.5">
                        <label htmlFor="profile-expire" className={LABEL_CLASS_NAME}>
                          {t('tenants.profile.fields.expiresAt', { defaultValue: 'Expires At' })}
                        </label>
                        <Input
                          id="profile-expire"
                          name="expires_at"
                          type="datetime-local"
                          value={profileForm.expires_at}
                          onChange={(e) =>
                            setProfileForm((prev) => ({ ...prev, expires_at: e.target.value }))
                          }
                        />
                      </div>
                    </div>

                    <Button color="primary" onClick={() => patchTenantMutation.mutate()} disabled={patchTenantMutation.isPending}>
                      {patchTenantMutation.isPending ? (
                        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                      ) : null}
                      {t('tenants.profile.save', { defaultValue: 'Save Profile' })}
                    </Button>
                  </AntigravityDialogPanel>

                  <AntigravityDialogPanel as="section">
                    {capabilities.features.tenant_recharge ? (
                      <>
                        <h3 className="text-base font-medium">
                          {t('tenants.recharge.title', { defaultValue: 'Tenant Recharge' })}
                        </h3>
                        <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
                          <div className="space-y-1.5">
                            <label htmlFor="profile-recharge-amount" className={LABEL_CLASS_NAME}>
                              {t('tenants.recharge.fields.amount', { defaultValue: 'Microcredits (integer)' })}
                            </label>
                            <Input
                              id="profile-recharge-amount"
                              name="amount_microcredits"
                              type="number"
                              min={0}
                              value={rechargeForm.amount_microcredits}
                              onChange={(e) =>
                                setRechargeForm((prev) => ({
                                  ...prev,
                                  amount_microcredits: e.target.value,
                                }))
                              }
                            />
                          </div>
                          <div className="space-y-1.5">
                            <label htmlFor="profile-recharge-reason" className={LABEL_CLASS_NAME}>
                              {t('tenants.recharge.fields.reason', { defaultValue: 'Reason' })}
                            </label>
                            <Input
                              id="profile-recharge-reason"
                              name="reason"
                              value={rechargeForm.reason}
                              onChange={(e) =>
                                setRechargeForm((prev) => ({ ...prev, reason: e.target.value }))
                              }
                            />
                          </div>
                        </div>
                        <Button color="primary" onClick={() => rechargeMutation.mutate()} disabled={rechargeMutation.isPending}>
                          {rechargeMutation.isPending ? (
                            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                          ) : null}
                          {t('tenants.recharge.submit', { defaultValue: 'Apply Recharge' })}
                        </Button>

                        <SurfaceDivider />
                      </>
                    ) : null}

                    <h3 className="text-base font-medium">
                      {t('tenants.impersonation.title', { defaultValue: 'Admin Impersonation' })}
                    </h3>
                    <div className="space-y-1.5">
                      <label htmlFor="profile-impersonation-reason" className={LABEL_CLASS_NAME}>
                        {t('tenants.impersonation.fields.reason', { defaultValue: 'Reason (required)' })}
                      </label>
                      <Input
                        id="profile-impersonation-reason"
                        name="reason"
                        value={impersonationReason}
                        onChange={(e) => setImpersonationReason(e.target.value)}
                      />
                    </div>

                    <div className="flex flex-wrap items-center gap-2">
                      <Button
                        color="primary"
                        onClick={() => impersonationMutation.mutate()}
                        disabled={impersonationMutation.isPending}
                      >
                        {impersonationMutation.isPending ? (
                          <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                        ) : null}
                        {t('tenants.impersonation.create', { defaultValue: 'Create Impersonation' })}
                      </Button>

                      {lastImpersonation?.tenant_id === profileTenant.id ? (
                        <Button
                          onClick={() =>
                            revokeImpersonationMutation.mutate(lastImpersonation.session_id)
                          }
                          disabled={revokeImpersonationMutation.isPending}
                        >
                          {t('tenants.impersonation.revoke', { defaultValue: 'Revoke Session' })}
                        </Button>
                      ) : null}
                    </div>

                    {lastImpersonation?.tenant_id === profileTenant.id ? (
                      <SurfaceInset className="break-all space-y-1 text-xs">
                        <p>
                          {t('tenants.impersonation.sessionIdLabel', { defaultValue: 'Session ID:' })}{' '}
                          <span className="font-mono">{lastImpersonation.session_id}</span>
                        </p>
                        <p>
                          {t('tenants.impersonation.tokenLabel', { defaultValue: 'Token:' })}{' '}
                          <span className="font-mono">{maskToken(lastImpersonation.access_token)}</span>
                        </p>
                        <Button
                          type="button"
                          size="sm"
                          onClick={() => copyText(lastImpersonation.access_token)}
                        >
                          <Copy className="mr-2 h-4 w-4" />
                          {t('tenants.impersonation.copyToken', { defaultValue: 'Copy Token' })}
                        </Button>
                      </SurfaceInset>
                    ) : null}
                  </AntigravityDialogPanel>
                </section>
              ) : null}

              {profileTab === 'keys' ? (
                <section
                  id="tenants-profile-panel-keys"
                  role="tabpanel"
                  tabIndex={0}
                  aria-labelledby="tenants-profile-tab-keys"
                  className="space-y-4"
                >
                  <AntigravityDialogPanel as="section">
                    <h3 className="text-base font-medium">
                      {t('tenants.keys.create.title', { defaultValue: 'Create API Key' })}
                    </h3>
                    <div className="flex flex-wrap items-end gap-2">
                      <div className="min-w-[240px] flex-1 space-y-1.5">
                        <label htmlFor="profile-new-key-name" className={LABEL_CLASS_NAME}>
                          {t('tenants.keys.create.fields.name', { defaultValue: 'Key Name' })}
                        </label>
                        <Input
                          id="profile-new-key-name"
                          name="new_key_name"
                          value={newKeyName}
                          onChange={(e) => setNewKeyName(e.target.value)}
                          placeholder={t('tenants.keys.create.fields.namePlaceholder', {
                            defaultValue: 'e.g. admin-main-key',
                          })}
                        />
                      </div>
                      <Button
                        color="primary"
                        onClick={() => createKeyMutation.mutate()}
                        disabled={createKeyMutation.isPending}
                      >
                        {createKeyMutation.isPending ? (
                          <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                        ) : (
                          <Plus className="mr-2 h-4 w-4" />
                        )}
                        {t('tenants.keys.create.submit', { defaultValue: 'Create Key' })}
                      </Button>
                    </div>

                    {createdKey?.record.tenant_id === profileTenant.id ? (
                      <SurfaceNotice tone="warning" className="space-y-2">
                        <div className="text-sm font-medium">
                          {t('tenants.keys.created.notice', { defaultValue: 'The plaintext key is shown only once. Save it now.' })}
                        </div>
                        <SurfaceCode className="break-all">
                          {createdKey.plaintext_key}
                        </SurfaceCode>
                        <Button
                          size="sm"
                          onClick={() => copyText(createdKey.plaintext_key)}
                        >
                          <Copy className="mr-2 h-4 w-4" />
                          {t('tenants.keys.created.copyPlaintext', { defaultValue: 'Copy Plaintext Key' })}
                        </Button>
                      </SurfaceNotice>
                    ) : null}
                  </AntigravityDialogPanel>

                  <AntigravityDialogPanel as="section">
                    <h3 className="text-base font-medium">
                      {t('tenants.keys.list.title', { defaultValue: 'API Key List' })}
                    </h3>
                    <DataTable
                      columns={tenantKeyColumns}
                      data={keysForCurrentTenant}
                      density="compact"
                      enableSearch={false}
                      searchField="name"
                      showToolbar={false}
                      showPageControls={keysForCurrentTenant.length > 10}
                      defaultPageSize={10}
                      pageSizeOptions={[10, 20, 50]}
                      className="border-0 bg-transparent shadow-none"
                      emptyText={t('tenants.keys.list.empty', { defaultValue: 'No API keys for this tenant' })}
                      title={t('tenants.keys.list.caption', { defaultValue: 'Tenant API key list' })}
                    />
                  </AntigravityDialogPanel>
                </section>
              ) : null}

              {profileTab === 'usage' ? (
                <section
                  id="tenants-profile-panel-usage"
                  role="tabpanel"
                  tabIndex={0}
                  aria-labelledby="tenants-profile-tab-usage"
                >
                  <TenantUsageSection
                    tenantId={profileTenant.id}
                    labelClassName={LABEL_CLASS_NAME}
                    keysForCurrentTenant={keysForCurrentTenant}
                    filteredUsageApiKeys={filteredUsageApiKeys}
                    effectiveUsageApiKeyFilter={effectiveUsageApiKeyFilter}
                    selectedUsageApiKey={selectedUsageApiKey}
                    usageApiKeyFilterAllValue={USAGE_API_KEY_FILTER_ALL}
                    usageApiKeyPopoverOpen={usageApiKeyPopoverOpen}
                    setUsageApiKeyPopoverOpen={setUsageApiKeyPopoverOpen}
                    usageApiKeyKeyword={usageApiKeyKeyword}
                    setUsageApiKeyKeyword={setUsageApiKeyKeyword}
                    setUsageApiKeyFilter={setUsageApiKeyFilter}
                    usageSummaryQuery={usageSummaryQuery}
                  />
                </section>
              ) : null}
            </AntigravityDialogBody>
          ) : null}
        </AntigravityDialogShell>
      </Dialog>
      </div>
      </motion.div>
    </PageContent>
  )
}
